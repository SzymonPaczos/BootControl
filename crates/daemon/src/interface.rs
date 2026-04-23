//! D-Bus interface implementation for BootControl.
//!
//! This module exposes the `org.bootcontrol.Manager` interface at the D-Bus
//! object path `/org/bootcontrol/Manager`. All methods are async and delegate
//! to the pure logic in [`crate::grub_manager`], [`crate::grub_rebuild`],
//! and [`crate::sanitize`].
//!
//! # Security layers per method
//!
//! ## `ReadGrubConfig`
//! Read-only — no authorization required.
//!
//! ## `GetEtag`
//! Read-only — no authorization required.
//!
//! ## `GetActiveBackend`
//! Read-only — no authorization required.
//!
//! ## `SetGrubValue`
//! Wielowarstwowa autoryzacja zapisu:
//! 1. Polkit check ([`crate::polkit::authorize_with_polkit`])
//! 2. Payload blacklist ([`crate::sanitize::check_payload`])
//! 3. ETag + flock + atomic write ([`crate::grub_manager::set_grub_value`])
//!    — weryfikacja ETag odbywa się **pod lockiem** (TOCTOU-safe)
//! 4. `grub-mkconfig` regeneration ([`crate::grub_rebuild::run_grub_mkconfig`])
//!
//! ## `RebuildGrubConfig`
//! 1. Polkit check ([`crate::polkit::authorize_with_polkit`])
//! 2. `grub-mkconfig` regeneration ([`crate::grub_rebuild::run_grub_mkconfig`])

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use bootcontrol_core::{boot_manager::BootManager, secureboot::MokSigner};
#[cfg(feature = "experimental_paranoia")]
use bootcontrol_core::secureboot::ParanoiaKeySet;

use crate::{
    dbus_error::{to_daemon_error, DaemonError},
    grub_manager, grub_rebuild,
    polkit::authorize_with_polkit,
    sanitize,
    secureboot::nvram::{backup_efi_variables, DEFAULT_BACKUP_DIR, DEFAULT_EFIVARS_DIR},
    secureboot::mok::{sign_with_default_keys, SbsignMokSigner},
    systemd_boot_manager, uki_manager,
};

#[cfg(feature = "experimental_paranoia")]
use crate::secureboot::paranoia::{generate_custom_keyset, merge_with_microsoft_signatures, DEFAULT_KEYSET_DIR};
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
/// `grub_cfg_path` is the destination for `grub-mkconfig -o <path>`. In
/// production this is `/boot/grub/grub.cfg`. Injectable for tests to avoid
/// writing to the live boot partition.
///
/// `backend` is the active [`BootManager`] backend. It is selected at startup
/// by the prober and injected here. This enables `GetActiveBackend()` and
/// future generic boot operations.
///
/// All fields are **private** — external code accesses them only through the
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
    /// Destination path for the regenerated `grub.cfg`.
    ///
    /// Private — production default is `/boot/grub/grub.cfg`.
    grub_cfg_path: PathBuf,
    /// Active bootloader backend, selected by the prober at startup.
    backend: Box<dyn BootManager>,
    /// Directory containing systemd-boot loader entry `.conf` files.
    ///
    /// Production default: `/boot/loader/entries`.
    loader_entries_dir: PathBuf,
    /// Path to `loader.conf` (systemd-boot global config).
    ///
    /// Production default: `/boot/loader/loader.conf`.
    loader_conf_path: PathBuf,
    /// Path to the UKI kernel command-line file.
    ///
    /// Production default: `/etc/kernel/cmdline`.
    kernel_cmdline_path: PathBuf,
}

impl GrubManager {
    /// Create a new [`GrubManager`] pointing at the given `grub_path`.
    ///
    /// The failsafe snippet path defaults to `/etc/bootcontrol/failsafe.cfg`
    /// and the grub.cfg output path defaults to `/boot/grub/grub.cfg`.
    /// Use [`GrubManager::with_failsafe_path`] to override both for tests.
    ///
    /// # Arguments
    ///
    /// * `grub_path` — Path to the GRUB configuration file. Production code
    ///   passes `/etc/default/grub`; tests pass a `NamedTempFile` path.
    /// * `backend`   — The active [`BootManager`] backend, selected by the prober.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use bootcontrold::interface::GrubManager;
    /// use bootcontrol_core::backends::grub::GrubBackend;
    ///
    /// let manager = GrubManager::new(PathBuf::from("/etc/default/grub"), Box::new(GrubBackend));
    /// assert_eq!(manager.grub_path(), std::path::Path::new("/etc/default/grub"));
    /// ```
    pub fn new(grub_path: PathBuf, backend: Box<dyn BootManager>) -> Self {
        Self {
            grub_path,
            failsafe_cfg_path: PathBuf::from("/etc/bootcontrol/failsafe.cfg"),
            grub_cfg_path: PathBuf::from("/boot/grub/grub.cfg"),
            backend,
            loader_entries_dir: PathBuf::from("/boot/loader/entries"),
            loader_conf_path: PathBuf::from("/boot/loader/loader.conf"),
            kernel_cmdline_path: PathBuf::from("/etc/kernel/cmdline"),
        }
    }

    /// Create a [`GrubManager`] with custom failsafe and grub.cfg output paths.
    ///
    /// Intended for integration tests that need to avoid writing to system
    /// paths (`/etc/bootcontrol`, `/boot/grub`) during test runs.
    ///
    /// # Arguments
    ///
    /// * `grub_path`         — Path to the GRUB configuration file.
    /// * `failsafe_cfg_path` — Path where the failsafe GRUB snippet is written.
    /// * `backend`           — The active [`BootManager`] backend.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use bootcontrold::interface::GrubManager;
    /// use bootcontrol_core::backends::grub::GrubBackend;
    ///
    /// let m = GrubManager::with_failsafe_path(
    ///     PathBuf::from("/etc/default/grub"),
    ///     PathBuf::from("/tmp/test-failsafe.cfg"),
    ///     Box::new(GrubBackend),
    /// );
    /// assert_eq!(m.grub_path(), std::path::Path::new("/etc/default/grub"));
    /// ```
    pub fn with_failsafe_path(
        grub_path: PathBuf,
        failsafe_cfg_path: PathBuf,
        backend: Box<dyn BootManager>,
    ) -> Self {
        Self {
            grub_path,
            failsafe_cfg_path,
            grub_cfg_path: PathBuf::from("/boot/grub/grub.cfg"),
            backend,
            loader_entries_dir: PathBuf::from("/boot/loader/entries"),
            loader_conf_path: PathBuf::from("/boot/loader/loader.conf"),
            kernel_cmdline_path: PathBuf::from("/etc/kernel/cmdline"),
        }
    }

    /// Create a [`GrubManager`] with fully injectable paths for testing.
    ///
    /// Overrides all filesystem paths so that tests never touch system
    /// directories (`/etc/bootcontrol`, `/boot/grub`).
    ///
    /// # Arguments
    ///
    /// * `grub_path`         — Path to the GRUB configuration file.
    /// * `failsafe_cfg_path` — Path where the failsafe GRUB snippet is written.
    /// * `grub_cfg_path`     — Destination for the regenerated `grub.cfg`.
    /// * `backend`           — The active [`BootManager`] backend.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use bootcontrold::interface::GrubManager;
    /// use bootcontrol_core::backends::grub::GrubBackend;
    ///
    /// let m = GrubManager::with_all_paths(
    ///     PathBuf::from("/tmp/grub"),
    ///     PathBuf::from("/tmp/failsafe.cfg"),
    ///     PathBuf::from("/tmp/grub.cfg"),
    ///     Box::new(GrubBackend),
    /// );
    /// assert_eq!(m.grub_path(), std::path::Path::new("/tmp/grub"));
    /// ```
    pub fn with_all_paths(
        grub_path: PathBuf,
        failsafe_cfg_path: PathBuf,
        grub_cfg_path: PathBuf,
        backend: Box<dyn BootManager>,
    ) -> Self {
        Self {
            grub_path,
            failsafe_cfg_path,
            grub_cfg_path,
            backend,
            loader_entries_dir: PathBuf::from("/boot/loader/entries"),
            loader_conf_path: PathBuf::from("/boot/loader/loader.conf"),
            kernel_cmdline_path: PathBuf::from("/etc/kernel/cmdline"),
        }
    }

    /// Create a [`GrubManager`] with fully injectable paths including systemd-boot and UKI.
    ///
    /// Used by integration tests that need to avoid writing to system directories.
    #[allow(clippy::too_many_arguments)]
    pub fn with_extended_paths(
        grub_path: PathBuf,
        failsafe_cfg_path: PathBuf,
        grub_cfg_path: PathBuf,
        loader_entries_dir: PathBuf,
        loader_conf_path: PathBuf,
        kernel_cmdline_path: PathBuf,
        backend: Box<dyn BootManager>,
    ) -> Self {
        Self {
            grub_path,
            failsafe_cfg_path,
            grub_cfg_path,
            backend,
            loader_entries_dir,
            loader_conf_path,
            kernel_cmdline_path,
        }
    }

    /// Return the path to the GRUB configuration file managed by this instance.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::{Path, PathBuf};
    /// use bootcontrold::interface::GrubManager;
    /// use bootcontrol_core::backends::grub::GrubBackend;
    ///
    /// let m = GrubManager::new(PathBuf::from("/etc/default/grub"), Box::new(GrubBackend));
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
    /// - `key`        — GRUB variable name (e.g. `"GRUB_TIMEOUT"`).
    /// - `value`      — New value (e.g. `"10"`). Do **not** include surrounding
    ///                  quotes; the daemon adds them when necessary.
    /// - `etag`       — The ETag returned by the most recent `ReadGrubConfig`
    ///                  or `GetEtag` call.
    /// - `connection` — Injected by zbus; used to resolve the caller's Unix UID
    ///                  via `org.freedesktop.DBus.GetConnectionUnixUser`.
    ///
    /// ## Errors
    ///
    /// - `org.bootcontrol.Error.PolkitDenied` — the caller UID could not be
    ///   resolved, or Polkit denies the action.
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
        #[zbus(connection)] connection: &zbus::Connection,
    ) -> Result<(), DaemonError> {
        info!(key = %key, "D-Bus: SetGrubValue");

        // ── Step 1: Resolve the caller's real UID via D-Bus ─────────────────
        // The D-Bus daemon tracks each connection's OS-level UID. We ask it
        // for the caller's unique bus name, then call
        // GetConnectionUnixUser(name) to retrieve the verified UID.
        // This is TOCTOU-safe: the D-Bus daemon maintains an immutable mapping
        // per connection that the caller cannot spoof.
        let caller_uid: u32 = {
            let sender = header
                .sender()
                .ok_or_else(|| {
                    warn!(key = %key, "D-Bus message has no sender field");
                    DaemonError::PolkitDenied("missing sender in D-Bus message".to_string())
                })?
                .clone();

            let dbus_proxy = zbus::fdo::DBusProxy::new(connection).await.map_err(|e| {
                warn!(key = %key, error = %e, "Failed to create DBus proxy");
                DaemonError::PolkitDenied(format!("failed to create D-Bus proxy: {e}"))
            })?;

            dbus_proxy
                .get_connection_unix_user(sender.into())
                .await
                .map_err(|e| {
                    warn!(key = %key, error = %e, "GetConnectionUnixUser failed");
                    DaemonError::PolkitDenied(format!("failed to resolve caller UID: {e}"))
                })?
        };

        info!(caller_uid = %caller_uid, key = %key, "Resolved caller UID for Polkit");

        // ── Step 2: Polkit authorization ────────────────────────────────────
        authorize_with_polkit(caller_uid).await.map_err(|e| {
            warn!(caller_uid = %caller_uid, key = %key, "Polkit denied");
            to_daemon_error(e)
        })?;

        // ── Step 3: Payload sanitization ────────────────────────────────────
        sanitize::check_payload(&key, &value).map_err(|e| {
            warn!(key = %key, value = %value, "Security policy violation");
            to_daemon_error(e)
        })?;

        // ── Steps 3–9: flock → ETag verify → atomic write → failsafe refresh → grub-mkconfig ──
        grub_manager::set_grub_value(
            &self.grub_path,
            &key,
            &value,
            &etag,
            &self.failsafe_cfg_path,
            &self.grub_cfg_path,
        )
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

    /// Return the name of the active bootloader backend.
    ///
    /// **Read-only — no Polkit authorization required.** This method only
    /// reports which backend was detected at daemon startup; it performs no
    /// writes and accesses no sensitive state.
    ///
    /// ## D-Bus signature
    ///
    /// ```text
    /// GetActiveBackend() -> s
    /// ```
    ///
    /// ## Return value
    ///
    /// A short ASCII string: `"grub"`, `"systemd-boot"`, or `"unknown"`.
    async fn get_active_backend(&self) -> String {
        info!("D-Bus: GetActiveBackend");
        self.backend.name().to_string()
    }

    /// Regenerate `/boot/grub/grub.cfg` by invoking `grub-mkconfig`.
    ///
    /// Use this method to apply pending changes to the GRUB configuration
    /// without writing any new key-value pair. `SetGrubValue` already calls
    /// this automatically; `RebuildGrubConfig` is provided as a standalone
    /// escape hatch for cases where an external tool modified
    /// `/etc/default/grub` directly and the caller wants to regenerate the
    /// boot config without going through the full ETag write pipeline.
    ///
    /// ## D-Bus signature
    ///
    /// ```text
    /// RebuildGrubConfig() -> ()
    /// ```
    ///
    /// ## Arguments
    ///
    /// None.
    ///
    /// ## Errors
    ///
    /// - `org.bootcontrol.Error.PolkitDenied` — the caller is not authorized.
    /// - `org.bootcontrol.Error.EspScanFailed` — `grub-mkconfig` (or
    ///   `grub2-mkconfig`) is not installed, or the command exited with a
    ///   non-zero status. The reason string includes the exit code and stderr.
    async fn rebuild_grub_config(
        &self,
        #[zbus(header)] header: zbus::message::Header<'_>,
        #[zbus(connection)] connection: &zbus::Connection,
    ) -> Result<(), DaemonError> {
        info!("D-Bus: RebuildGrubConfig");

        // ── Step 1: Resolve the caller's real UID via D-Bus ─────────────────
        let caller_uid: u32 = {
            let sender = header
                .sender()
                .ok_or_else(|| {
                    warn!("D-Bus message has no sender field (RebuildGrubConfig)");
                    DaemonError::PolkitDenied("missing sender in D-Bus message".to_string())
                })?
                .clone();

            let dbus_proxy = zbus::fdo::DBusProxy::new(connection).await.map_err(|e| {
                warn!(error = %e, "Failed to create DBus proxy (RebuildGrubConfig)");
                DaemonError::PolkitDenied(format!("failed to create D-Bus proxy: {e}"))
            })?;

            dbus_proxy
                .get_connection_unix_user(sender.into())
                .await
                .map_err(|e| {
                    warn!(error = %e, "GetConnectionUnixUser failed (RebuildGrubConfig)");
                    DaemonError::PolkitDenied(format!("failed to resolve caller UID: {e}"))
                })?
        };

        // ── Step 2: Polkit authorization ────────────────────────────────────
        authorize_with_polkit(caller_uid).await.map_err(|e| {
            warn!(caller_uid = %caller_uid, "Polkit denied for RebuildGrubConfig");
            to_daemon_error(e)
        })?;

        // ── Step 2: Run grub-mkconfig ───────────────────────────────────────
        grub_rebuild::run_grub_mkconfig(&self.grub_cfg_path).map_err(to_daemon_error)
    }

    /// Back up Secure Boot EFI NVRAM variables to a target directory.
    ///
    /// Reads all variables matching `db-*`, `KEK-*`, and `PK-*` from the
    /// Linux sysfs EFI variables interface and writes their raw bytes to the
    /// target directory. This operation must be performed **before** any key
    /// enrollment to preserve the original Microsoft certificates locally.
    ///
    /// Returns a JSON array of strings listing the absolute paths of all
    /// backed-up files.
    ///
    /// ## D-Bus signature
    ///
    /// ```text
    /// BackupNvram(s) -> s
    /// ```
    ///
    /// ## Arguments
    ///
    /// * `target_dir` — Target directory for backup files. Pass an empty
    ///   string to use the default (`/var/lib/bootcontrol/certs`).
    ///
    /// ## Errors
    ///
    /// - `org.bootcontrol.Error.PolkitDenied` — the caller is not authorized.
    /// - `org.bootcontrol.Error.NvramBackupFailed` — the sysfs efivars
    ///   directory is not mounted, no Secure Boot variables were found, or the
    ///   target directory could not be written.
    async fn backup_nvram(
        &self,
        target_dir: String,
        #[zbus(header)] header: zbus::message::Header<'_>,
        #[zbus(connection)] connection: &zbus::Connection,
    ) -> Result<String, DaemonError> {
        info!(target_dir = %target_dir, "D-Bus: BackupNvram");

        // ── Step 1: Resolve the caller's real UID via D-Bus ─────────────────
        let caller_uid: u32 = {
            let sender = header
                .sender()
                .ok_or_else(|| {
                    warn!("D-Bus message has no sender field (BackupNvram)");
                    DaemonError::PolkitDenied("missing sender in D-Bus message".to_string())
                })?
                .clone();

            let dbus_proxy = zbus::fdo::DBusProxy::new(connection).await.map_err(|e| {
                warn!(error = %e, "Failed to create DBus proxy (BackupNvram)");
                DaemonError::PolkitDenied(format!("failed to create D-Bus proxy: {e}"))
            })?;

            dbus_proxy
                .get_connection_unix_user(sender.into())
                .await
                .map_err(|e| {
                    warn!(error = %e, "GetConnectionUnixUser failed (BackupNvram)");
                    DaemonError::PolkitDenied(format!("failed to resolve caller UID: {e}"))
                })?
        };

        info!(caller_uid = %caller_uid, "Resolved caller UID for BackupNvram");

        // ── Step 2: Polkit authorization ────────────────────────────────────
        authorize_with_polkit(caller_uid).await.map_err(|e| {
            warn!(caller_uid = %caller_uid, "Polkit denied for BackupNvram");
            to_daemon_error(e)
        })?;

        // ── Step 3: Resolve target directory ────────────────────────────────
        let resolved_target = if target_dir.is_empty() {
            std::path::PathBuf::from(DEFAULT_BACKUP_DIR)
        } else {
            std::path::PathBuf::from(&target_dir)
        };

        // ── Step 4: Perform the backup ──────────────────────────────────────
        let backup =
            backup_efi_variables(std::path::Path::new(DEFAULT_EFIVARS_DIR), &resolved_target)
                .map_err(|e| {
                    warn!(error = %e, "NVRAM backup failed");
                    to_daemon_error(e)
                })?;

        info!(file_count = backup.files.len(), "NVRAM backup completed");

        // ── Step 5: Serialize file list as a JSON array ─────────────────────
        let json = format!(
            "[{}]",
            backup
                .files
                .iter()
                .map(|p| format!("\"{}\"", p.display()))
                .collect::<Vec<_>>()
                .join(",")
        );

        Ok(json)
    }

    /// Sign a UKI image and enroll the MOK certificate.
    ///
    /// Signs the UKI at `uki_path` using the MOK key and certificate stored
    /// at the default paths (`/var/lib/bootcontrol/keys/mok.key` and
    /// `/var/lib/bootcontrol/keys/mok.crt`), then generates a MokManager
    /// enrollment request so the key is trusted on the next reboot.
    ///
    /// Requires Polkit authorization (`org.bootcontrol.manage`).
    ///
    /// ## D-Bus signature
    ///
    /// ```text
    /// SignAndEnrollUki(s) -> ()
    /// ```
    ///
    /// ## Arguments
    ///
    /// * `uki_path` — Absolute path to the UKI `.efi` image to sign.
    ///
    /// ## Errors
    ///
    /// - `org.bootcontrol.Error.PolkitDenied` — the caller is not authorized.
    /// - `org.bootcontrol.Error.MokKeyNotFound` — the MOK key or certificate is absent.
    /// - `org.bootcontrol.Error.ToolNotFound` — `sbsign` or `mokutil` is not installed.
    /// - `org.bootcontrol.Error.SigningFailed` — signing or enrollment exited non-zero.
    async fn sign_and_enroll_uki(
        &self,
        uki_path: String,
        #[zbus(header)] header: zbus::message::Header<'_>,
        #[zbus(connection)] connection: &zbus::Connection,
    ) -> Result<(), DaemonError> {
        info!(uki_path = %uki_path, "D-Bus: SignAndEnrollUki");

        // ── Step 1: Resolve the caller's real UID via D-Bus ─────────────────
        let caller_uid: u32 = {
            let sender = header
                .sender()
                .ok_or_else(|| {
                    warn!(uki_path = %uki_path, "D-Bus message has no sender field");
                    DaemonError::PolkitDenied("missing sender in D-Bus message".to_string())
                })?
                .clone();

            let dbus_proxy = zbus::fdo::DBusProxy::new(connection).await.map_err(|e| {
                warn!(uki_path = %uki_path, error = %e, "Failed to create DBus proxy");
                DaemonError::PolkitDenied(format!("failed to create D-Bus proxy: {e}"))
            })?;

            dbus_proxy
                .get_connection_unix_user(sender.into())
                .await
                .map_err(|e| {
                    warn!(uki_path = %uki_path, error = %e, "GetConnectionUnixUser failed");
                    DaemonError::PolkitDenied(format!("failed to resolve caller UID: {e}"))
                })?
        };

        info!(caller_uid = %caller_uid, uki_path = %uki_path, "Resolved caller UID for Polkit");

        // ── Step 2: Polkit authorization ────────────────────────────────────
        authorize_with_polkit(caller_uid).await.map_err(|e| {
            warn!(caller_uid = %caller_uid, uki_path = %uki_path, "Polkit denied");
            to_daemon_error(e)
        })?;

        // ── Step 3: Instantiate the signer ──────────────────────────────────
        let signer = SbsignMokSigner {
            sbsign_override: None,
            mokutil_override: None,
        };

        // ── Step 4a: Pre-flight check — default keys exist ──────────────────
        sign_with_default_keys(&signer).map_err(|e| {
            warn!(uki_path = %uki_path, "MOK key pre-flight failed");
            to_daemon_error(e)
        })?;

        // ── Step 4b: Sign the UKI ────────────────────────────────────────────
        let uki = std::path::Path::new(&uki_path);
        let key = crate::secureboot::mok::get_mok_key_path();
        let cert = crate::secureboot::mok::get_mok_cert_path();

        signer.sign_uki(uki, &key, &cert).map_err(|e| {
            warn!(uki_path = %uki_path, "UKI signing failed");
            to_daemon_error(e)
        })?;

        // ── Step 4c: Generate enrollment request ────────────────────────────
        signer
            .generate_enrollment_request(&cert, std::path::Path::new(""))
            .map_err(|e| {
                warn!(uki_path = %uki_path, "MOK enrollment request failed");
                to_daemon_error(e)
            })?;

        info!(uki_path = %uki_path, "SignAndEnrollUki completed successfully");
        Ok(())
    }

    /// Generate a custom Secure Boot key set (PK, KEK, db) using openssl.
    ///
    /// Returns a JSON array of generated file paths.
    /// If `output_dir` is empty, defaults to `/var/lib/bootcontrol/paranoia-keys`.
    /// Requires Polkit authorization (`org.bootcontrol.manage`).
    #[cfg(feature = "experimental_paranoia")]
    async fn generate_paranoia_keyset(
        &self,
        output_dir: String,
        #[zbus(header)] header: zbus::message::Header<'_>,
        #[zbus(connection)] connection: &zbus::Connection,
    ) -> Result<String, DaemonError> {
        info!(output_dir = %output_dir, "D-Bus: GenerateParanoiaKeyset");

        // ── Step 1: Resolve the caller's real UID via D-Bus ─────────────────
        let caller_uid: u32 = {
            let sender = header
                .sender()
                .ok_or_else(|| {
                    warn!("D-Bus message has no sender field (GenerateParanoiaKeyset)");
                    DaemonError::PolkitDenied("missing sender in D-Bus message".to_string())
                })?
                .clone();

            let dbus_proxy = zbus::fdo::DBusProxy::new(connection).await.map_err(|e| {
                warn!(error = %e, "Failed to create DBus proxy (GenerateParanoiaKeyset)");
                DaemonError::PolkitDenied(format!("failed to create D-Bus proxy: {e}"))
            })?;

            dbus_proxy
                .get_connection_unix_user(sender.into())
                .await
                .map_err(|e| {
                    warn!(error = %e, "GetConnectionUnixUser failed (GenerateParanoiaKeyset)");
                    DaemonError::PolkitDenied(format!("failed to resolve caller UID: {e}"))
                })?
        };

        // ── Step 2: Polkit authorization ────────────────────────────────────
        authorize_with_polkit(caller_uid).await.map_err(|e| {
            warn!(caller_uid = %caller_uid, "Polkit denied for GenerateParanoiaKeyset");
            to_daemon_error(e)
        })?;

        // ── Step 3: Resolve target directory ────────────────────────────────
        let target = if output_dir.is_empty() {
            std::path::PathBuf::from(DEFAULT_KEYSET_DIR)
        } else {
            std::path::PathBuf::from(&output_dir)
        };

        // ── Step 4: Generate keys ───────────────────────────────────────────
        let keyset = generate_custom_keyset(&target, None).map_err(|e| {
            warn!(error = %e, "Key generation failed");
            to_daemon_error(e)
        })?;

        // ── Step 5: Serialize paths as JSON array ───────────────────────────
        let paths = vec![
            keyset.pk_cert,
            keyset.pk_key,
            keyset.kek_cert,
            keyset.kek_key,
            keyset.db_cert,
            keyset.db_key,
        ];

        let json = format!(
            "[{}]",
            paths
                .iter()
                .map(|p| format!("\"{}\"", p.display()))
                .collect::<Vec<_>>()
                .join(",")
        );

        info!(target = %target.display(), "Paranoia keyset generated successfully");
        Ok(json)
    }

    /// Merge custom db cert with Microsoft UEFI CA signatures.
    ///
    /// Returns path to the merged `.auth` file.
    /// Requires Polkit authorization (`org.bootcontrol.manage`).
    #[cfg(feature = "experimental_paranoia")]
    async fn merge_paranoia_with_microsoft(
        &self,
        output_dir: String,
        #[zbus(header)] header: zbus::message::Header<'_>,
        #[zbus(connection)] connection: &zbus::Connection,
    ) -> Result<String, DaemonError> {
        info!(output_dir = %output_dir, "D-Bus: MergeParanoiaWithMicrosoft");

        // ── Step 1: Resolve the caller's real UID via D-Bus ─────────────────
        let caller_uid: u32 = {
            let sender = header
                .sender()
                .ok_or_else(|| {
                    warn!("D-Bus message has no sender field (MergeParanoiaWithMicrosoft)");
                    DaemonError::PolkitDenied("missing sender in D-Bus message".to_string())
                })?
                .clone();

            let dbus_proxy = zbus::fdo::DBusProxy::new(connection).await.map_err(|e| {
                warn!(error = %e, "Failed to create DBus proxy (MergeParanoiaWithMicrosoft)");
                DaemonError::PolkitDenied(format!("failed to create D-Bus proxy: {e}"))
            })?;

            dbus_proxy
                .get_connection_unix_user(sender.into())
                .await
                .map_err(|e| {
                    warn!(error = %e, "GetConnectionUnixUser failed (MergeParanoiaWithMicrosoft)");
                    DaemonError::PolkitDenied(format!("failed to resolve caller UID: {e}"))
                })?
        };

        // ── Step 2: Polkit authorization ────────────────────────────────────
        authorize_with_polkit(caller_uid).await.map_err(|e| {
            warn!(caller_uid = %caller_uid, "Polkit denied for MergeParanoiaWithMicrosoft");
            to_daemon_error(e)
        })?;

        // ── Step 3: Resolve target directory ────────────────────────────────
        let target = if output_dir.is_empty() {
            std::path::PathBuf::from(DEFAULT_KEYSET_DIR)
        } else {
            std::path::PathBuf::from(&output_dir)
        };

        // ── Step 4: Construct ParanoiaKeySet from default locations ─────────
        // We expect the keys to follow the naming convention in DEFAULT_KEYSET_DIR
        let keys_base = std::path::PathBuf::from(DEFAULT_KEYSET_DIR);
        let keyset = ParanoiaKeySet {
            pk_cert: keys_base.join("PK.crt"),
            pk_key: keys_base.join("PK.key"),
            kek_cert: keys_base.join("KEK.crt"),
            kek_key: keys_base.join("KEK.key"),
            db_cert: keys_base.join("db.crt"),
            db_key: keys_base.join("db.key"),
        };

        // ── Step 5: Merge signatures ────────────────────────────────────────
        let auth_path = merge_with_microsoft_signatures(&keyset, &target, None).map_err(|e| {
            warn!(error = %e, "Merging signatures failed");
            to_daemon_error(e)
        })?;

        info!(auth_path = %auth_path.display(), "Signatures merged successfully");
        Ok(auth_path.display().to_string())
    }

    // ── systemd-boot methods ──────────────────────────────────────────────────

    /// List all systemd-boot loader entries.
    ///
    /// Reads every `.conf` file in `/boot/loader/entries/` and `loader.conf`
    /// for the current default.
    ///
    /// ## D-Bus signature
    ///
    /// ```text
    /// ListLoaderEntries() -> s
    /// ```
    ///
    /// ## Return value
    ///
    /// A JSON array of entry objects.  Each object has:
    /// `id`, `title`, `linux`, `initrd`, `options`, `machine_id`, `etag`, `is_default`.
    ///
    /// ## Errors
    ///
    /// - `org.bootcontrol.Error.EspScanFailed` — entries directory unreadable.
    async fn list_loader_entries(&self) -> Result<String, DaemonError> {
        info!(dir = ?self.loader_entries_dir, "D-Bus: ListLoaderEntries");
        let records = systemd_boot_manager::read_all_entries(
            &self.loader_entries_dir,
            &self.loader_conf_path,
        )
        .map_err(to_daemon_error)?;
        serde_json::to_string(&records).map_err(|e| {
            DaemonError::EspScanFailed(format!("serialization error: {e}"))
        })
    }

    /// Read a single systemd-boot loader entry by ID.
    ///
    /// ## D-Bus signature
    ///
    /// ```text
    /// ReadLoaderEntry(s) -> (s, s)
    /// ```
    ///
    /// ## Return value
    ///
    /// `(json_entry, file_etag)` where `json_entry` is the serialized
    /// [`systemd_boot_manager::EntryRecord`] and `file_etag` is the SHA-256
    /// of the `.conf` file.
    ///
    /// ## Errors
    ///
    /// - `org.bootcontrol.Error.EspScanFailed` — entry not found or unreadable.
    async fn read_loader_entry(&self, id: String) -> Result<(String, String), DaemonError> {
        info!(id = %id, "D-Bus: ReadLoaderEntry");
        let (entry, etag) =
            systemd_boot_manager::read_entry(&self.loader_entries_dir, &id)
                .map_err(to_daemon_error)?;
        let record = systemd_boot_manager::EntryRecord {
            id: id.clone(),
            title: entry.title,
            linux: entry.linux,
            initrd: entry.initrd,
            options: entry.options,
            machine_id: entry.machine_id,
            etag: etag.clone(),
            is_default: {
                let def = systemd_boot_manager::read_loader_conf_default(&self.loader_conf_path)
                    .unwrap_or_default();
                def.trim() == id || def.trim() == format!("{id}.conf")
            },
        };
        let json = serde_json::to_string(&record).map_err(|e| {
            DaemonError::EspScanFailed(format!("serialization error: {e}"))
        })?;
        Ok((json, etag))
    }

    /// Set the default systemd-boot loader entry.
    ///
    /// Writes the `default <id>` key to `loader.conf`.
    /// Requires Polkit authorization.
    ///
    /// ## D-Bus signature
    ///
    /// ```text
    /// SetLoaderDefault(s, s) -> ()
    /// ```
    ///
    /// ## Arguments
    ///
    /// - `id`   — Entry ID (filename stem, e.g. `"arch"`).
    /// - `etag` — Current `loader.conf` ETag.
    ///
    /// ## Errors
    ///
    /// - `org.bootcontrol.Error.PolkitDenied`
    /// - `org.bootcontrol.Error.StateMismatch` — stale ETag.
    /// - `org.bootcontrol.Error.ConcurrentModification`
    async fn set_loader_default(
        &self,
        id: String,
        etag: String,
        #[zbus(header)] header: zbus::message::Header<'_>,
        #[zbus(connection)] connection: &zbus::Connection,
    ) -> Result<(), DaemonError> {
        info!(id = %id, "D-Bus: SetLoaderDefault");
        let caller_uid = resolve_uid(&header, connection, "SetLoaderDefault").await?;
        authorize_with_polkit(caller_uid).await.map_err(to_daemon_error)?;
        systemd_boot_manager::set_loader_default(&self.loader_conf_path, &id, &etag)
            .map_err(to_daemon_error)
    }

    /// Get the ETag of `loader.conf`.
    ///
    /// ## D-Bus signature
    ///
    /// ```text
    /// GetLoaderConfEtag() -> s
    /// ```
    async fn get_loader_conf_etag(&self) -> Result<String, DaemonError> {
        info!("D-Bus: GetLoaderConfEtag");
        systemd_boot_manager::fetch_loader_conf_etag(&self.loader_conf_path)
            .map_err(to_daemon_error)
    }

    // ── UKI / kernel cmdline methods ──────────────────────────────────────────

    /// Read `/etc/kernel/cmdline` and return the current kernel parameters.
    ///
    /// ## D-Bus signature
    ///
    /// ```text
    /// ReadKernelCmdline() -> (as, s)
    /// ```
    ///
    /// ## Return value
    ///
    /// `(params, etag)` where `params` is an array of individual parameter
    /// tokens and `etag` is the SHA-256 of the file.
    ///
    /// ## Errors
    ///
    /// - `org.bootcontrol.Error.EspScanFailed` — file not found or unreadable.
    async fn read_kernel_cmdline(&self) -> Result<(Vec<String>, String), DaemonError> {
        info!(path = ?self.kernel_cmdline_path, "D-Bus: ReadKernelCmdline");
        uki_manager::read_kernel_cmdline(&self.kernel_cmdline_path).map_err(to_daemon_error)
    }

    /// Add a kernel parameter to `/etc/kernel/cmdline`.
    ///
    /// Idempotent: if the parameter is already present this is a no-op.
    /// Requires Polkit authorization.
    ///
    /// ## D-Bus signature
    ///
    /// ```text
    /// AddKernelParam(s, s) -> ()
    /// ```
    ///
    /// ## Arguments
    ///
    /// - `param` — Parameter token (e.g. `"quiet"`, `"root=/dev/sda1"`).
    /// - `etag`  — Current ETag of `/etc/kernel/cmdline`.
    ///
    /// ## Errors
    ///
    /// - `org.bootcontrol.Error.PolkitDenied`
    /// - `org.bootcontrol.Error.SecurityPolicyViolation` — blacklisted pattern.
    /// - `org.bootcontrol.Error.StateMismatch` — stale ETag.
    /// - `org.bootcontrol.Error.ConcurrentModification`
    async fn add_kernel_param(
        &self,
        param: String,
        etag: String,
        #[zbus(header)] header: zbus::message::Header<'_>,
        #[zbus(connection)] connection: &zbus::Connection,
    ) -> Result<(), DaemonError> {
        info!(param = %param, "D-Bus: AddKernelParam");
        let caller_uid = resolve_uid(&header, connection, "AddKernelParam").await?;
        authorize_with_polkit(caller_uid).await.map_err(to_daemon_error)?;
        uki_manager::add_kernel_param(&self.kernel_cmdline_path, &param, &etag)
            .map_err(to_daemon_error)
    }

    /// Remove a kernel parameter from `/etc/kernel/cmdline`.
    ///
    /// Requires Polkit authorization.
    ///
    /// ## D-Bus signature
    ///
    /// ```text
    /// RemoveKernelParam(s, s) -> ()
    /// ```
    ///
    /// ## Arguments
    ///
    /// - `param` — Parameter token to remove (e.g. `"quiet"`).
    /// - `etag`  — Current ETag of `/etc/kernel/cmdline`.
    ///
    /// ## Errors
    ///
    /// - `org.bootcontrol.Error.PolkitDenied`
    /// - `org.bootcontrol.Error.KeyNotFound` — param not present.
    /// - `org.bootcontrol.Error.StateMismatch` — stale ETag.
    async fn remove_kernel_param(
        &self,
        param: String,
        etag: String,
        #[zbus(header)] header: zbus::message::Header<'_>,
        #[zbus(connection)] connection: &zbus::Connection,
    ) -> Result<(), DaemonError> {
        info!(param = %param, "D-Bus: RemoveKernelParam");
        let caller_uid = resolve_uid(&header, connection, "RemoveKernelParam").await?;
        authorize_with_polkit(caller_uid).await.map_err(to_daemon_error)?;
        uki_manager::remove_kernel_param(&self.kernel_cmdline_path, &param, &etag)
            .map_err(to_daemon_error)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Private helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Resolve the caller's Unix UID from a D-Bus message header.
///
/// Shared by all write methods that need Polkit authorization.
async fn resolve_uid(
    header: &zbus::message::Header<'_>,
    connection: &zbus::Connection,
    method_name: &str,
) -> Result<u32, DaemonError> {
    let sender = header.sender().ok_or_else(|| {
        warn!(method = method_name, "D-Bus message has no sender field");
        DaemonError::PolkitDenied("missing sender in D-Bus message".to_string())
    })?.clone();

    let dbus_proxy = zbus::fdo::DBusProxy::new(connection).await.map_err(|e| {
        warn!(method = method_name, error = %e, "Failed to create DBus proxy");
        DaemonError::PolkitDenied(format!("failed to create D-Bus proxy: {e}"))
    })?;

    dbus_proxy
        .get_connection_unix_user(sender.into())
        .await
        .map_err(|e| {
            warn!(method = method_name, error = %e, "GetConnectionUnixUser failed");
            DaemonError::PolkitDenied(format!("failed to resolve caller UID: {e}"))
        })
}
