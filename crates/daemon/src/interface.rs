//! D-Bus interface implementation for BootControl.
//!
//! This module exposes the `org.bootcontrol.Manager` interface at the D-Bus
//! object path `/org/bootcontrol/Manager`. All methods are async and delegate
//! to the pure logic in [`crate::grub_manager`] and [`crate::sanitize`].
//!
//! # Security layers per method
//!
//! ## `ReadGrubConfig`
//! Read-only — no authorization required.
//!
//! ## `GetEtag`
//! Read-only — no authorization required.
//!
//! ## `SetGrubValue`
//! Wielowarstwowa autoryzacja zapisu:
//! 1. Polkit check ([`crate::polkit::authorize_with_polkit`])
//! 2. Payload blacklist ([`crate::sanitize::check_payload`])
//! 3. ETag + flock + atomic write ([`crate::grub_manager::set_grub_value`])
//!    — weryfikacja ETag odbywa się **pod lockiem** (TOCTOU-safe)

use std::path::{Path, PathBuf};
use std::collections::HashMap;

use crate::{dbus_error::{to_daemon_error, DaemonError}, grub_manager, polkit::authorize_with_polkit, sanitize};
use tracing::{info, warn};
use zbus::interface;

/// The D-Bus object that implements `org.bootcontrol.Manager`.
///
/// `grub_path` is the filesystem path to `/etc/default/grub`. It is
/// injectable via [`GrubManager::new`] so that integration tests can point
/// the daemon at a `tempfile` without needing real root access.
///
/// `failsafe_cfg_path` is the path to the golden-parachute GRUB snippet
/// written after every successful `SetGrubValue`. It is injectable for tests
/// via [`GrubManager::with_failsafe_path`].
///
/// Both fields are **private** — external code accesses them only through the
/// corresponding accessor methods to prevent accidental mutation after
/// construction.
pub struct GrubManager {
    /// Path to the GRUB default configuration file.
    ///
    /// Private — use [`GrubManager::grub_path`] for read access.
    grub_path: PathBuf,
    /// Path to the failsafe GRUB snippet.
    ///
    /// Private — set during construction; not exposed directly.
    failsafe_cfg_path: PathBuf,
}

impl GrubManager {
    /// Create a new [`GrubManager`] pointing at the given `grub_path`.
    ///
    /// The failsafe snippet path defaults to `/etc/bootcontrol/failsafe.cfg`.
    /// Use [`GrubManager::with_failsafe_path`] to override it for tests.
    ///
    /// # Arguments
    ///
    /// * `grub_path` — Path to the GRUB configuration file. Production code
    ///   passes `/etc/default/grub`; tests pass a `NamedTempFile` path.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use bootcontrold::interface::GrubManager;
    ///
    /// let manager = GrubManager::new(PathBuf::from("/etc/default/grub"));
    /// assert_eq!(manager.grub_path(), std::path::Path::new("/etc/default/grub"));
    /// ```
    pub fn new(grub_path: PathBuf) -> Self {
        Self {
            grub_path,
            failsafe_cfg_path: PathBuf::from("/etc/bootcontrol/failsafe.cfg"),
        }
    }

    /// Create a [`GrubManager`] with a custom failsafe snippet path.
    ///
    /// Intended for integration tests that need to avoid writing to system
    /// paths (`/etc/bootcontrol`) during test runs.
    ///
    /// # Arguments
    ///
    /// * `grub_path`         — Path to the GRUB configuration file.
    /// * `failsafe_cfg_path` — Path where the failsafe GRUB snippet is written.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use bootcontrold::interface::GrubManager;
    ///
    /// let m = GrubManager::with_failsafe_path(
    ///     PathBuf::from("/etc/default/grub"),
    ///     PathBuf::from("/tmp/test-failsafe.cfg"),
    /// );
    /// assert_eq!(m.grub_path(), std::path::Path::new("/etc/default/grub"));
    /// ```
    pub fn with_failsafe_path(grub_path: PathBuf, failsafe_cfg_path: PathBuf) -> Self {
        Self {
            grub_path,
            failsafe_cfg_path,
        }
    }

    /// Return the path to the GRUB configuration file managed by this instance.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::{Path, PathBuf};
    /// use bootcontrold::interface::GrubManager;
    ///
    /// let m = GrubManager::new(PathBuf::from("/etc/default/grub"));
    /// assert_eq!(m.grub_path(), Path::new("/etc/default/grub"));
    /// ```
    pub fn grub_path(&self) -> &Path {
        &self.grub_path
    }
}

#[interface(name = "org.bootcontrol.Manager")]
impl GrubManager {
    /// Read the GRUB default configuration file and return all key-value pairs
    /// together with the file's ETag.
    ///
    /// The ETag must be passed back in every subsequent `SetGrubValue` call to
    /// ensure optimistic concurrency control: if the file has been modified
    /// externally since the caller last read it, the write will be rejected
    /// with `org.bootcontrol.Error.StateMismatch`.
    ///
    /// ## D-Bus signature
    ///
    /// ```text
    /// ReadGrubConfig() -> (a{ss}, s)
    /// ```
    ///
    /// ## Return value
    ///
    /// A tuple of:
    /// - `a{ss}` — Dictionary of GRUB key-value pairs with outer double-quotes
    ///   stripped from values.
    /// - `s` — 64-character lowercase hex SHA-256 ETag of the file.
    ///
    /// ## Errors
    ///
    /// - `org.bootcontrol.Error.EspScanFailed` — file could not be read.
    /// - `org.bootcontrol.Error.ComplexBashDetected` — file contains Bash
    ///   constructs that BootControl cannot safely parse.
    async fn read_grub_config(&self) -> Result<(HashMap<String, String>, String), DaemonError> {
        info!(path = ?self.grub_path, "D-Bus: ReadGrubConfig");
        grub_manager::read_grub_config(&self.grub_path).map_err(to_daemon_error)
    }

    /// Set a single key-value pair in the GRUB default configuration file.
    ///
    /// This method enforces the full security and concurrency pipeline:
    /// Polkit authorization → payload sanitization →
    /// flock (TOCTOU-safe) → ETag verification → atomic write.
    ///
    /// ## D-Bus signature
    ///
    /// ```text
    /// SetGrubValue(s, s, s) -> ()
    /// ```
    ///
    /// ## Arguments
    ///
    /// - `key`   — GRUB variable name (e.g. `"GRUB_TIMEOUT"`).
    /// - `value` — New value (e.g. `"10"`). Do **not** include surrounding
    ///             quotes; the daemon adds them when necessary.
    /// - `etag`  — The ETag returned by the most recent `ReadGrubConfig` or
    ///             `GetEtag` call.
    ///
    /// ## Errors
    ///
    /// - `org.bootcontrol.Error.PolkitDenied` — the caller is not authorized.
    /// - `org.bootcontrol.Error.SecurityPolicyViolation` — key or value
    ///   contains a blacklisted pattern.
    /// - `org.bootcontrol.Error.ConcurrentModification` — another process
    ///   holds an exclusive lock on the config file (checked first, before read).
    /// - `org.bootcontrol.Error.StateMismatch` — the ETag is stale (checked
    ///   after acquiring the lock — TOCTOU-safe).
    /// - `org.bootcontrol.Error.EspScanFailed` — I/O error during read/write.
    /// - `org.bootcontrol.Error.ComplexBashDetected` — the on-disk file
    ///   contains Bash constructs.
    async fn set_grub_value(
        &self,
        key: String,
        value: String,
        etag: String,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) -> Result<(), DaemonError> {
        let caller_serial = header.primary().serial_num();
        info!(
            caller_serial = %caller_serial,
            key = %key,
            "D-Bus: SetGrubValue"
        );

        // ── Step 1: Polkit authorization ────────────────────────────────────
        // TODO Phase 2: wyciągnij prawdziwy UID z D-Bus peer credentials.
        // Na razie mock zakończy się sukcesem dla wszystkich UID.
        let caller_uid: u32 = caller_serial.into();
        authorize_with_polkit(caller_uid).map_err(|e| {
            warn!(key = %key, "Polkit denied");
            to_daemon_error(e)
        })?;

        // ── Step 2: Payload sanitization ────────────────────────────────────
        sanitize::check_payload(&key, &value).map_err(|e| {
            warn!(key = %key, value = %value, "Security policy violation");
            to_daemon_error(e)
        })?;

        // ── Steps 3–7: flock → ETag verify → atomic write → failsafe refresh ──
        grub_manager::set_grub_value(&self.grub_path, &key, &value, &etag, &self.failsafe_cfg_path)
            .map_err(to_daemon_error)
    }

    /// Return the SHA-256 ETag of the current on-disk GRUB configuration.
    ///
    /// Used by clients to refresh their ETag after an external change without
    /// fetching the full key-value map.
    ///
    /// ## D-Bus signature
    ///
    /// ```text
    /// GetEtag() -> s
    /// ```
    ///
    /// ## Return value
    ///
    /// 64-character lowercase hex SHA-256 digest of `/etc/default/grub`.
    ///
    /// ## Errors
    ///
    /// - `org.bootcontrol.Error.EspScanFailed` — file could not be read.
    async fn get_etag(&self) -> Result<String, DaemonError> {
        info!(path = ?self.grub_path, "D-Bus: GetEtag");
        grub_manager::fetch_etag(&self.grub_path).map_err(to_daemon_error)
    }
}
