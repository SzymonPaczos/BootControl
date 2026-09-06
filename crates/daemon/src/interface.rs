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
//! ## `ListGrubEntries`
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
use std::time::SystemTime;

use bootcontrol_core::{boot_manager::BootManager, secureboot::MokSigner};

use crate::{
    audit::{self, message_ids, AuditEvent, Phase},
    dbus_error::{snapshot_to_daemon_error, to_daemon_error, DaemonError},
    grub_manager, grub_rebuild,
    immutable_distro::probe_immutable_distro,
    polkit::{actions, authorize_with_polkit},
    rpm_ostree, sanitize,
    secureboot::mok::{
        read_entry_token, sign_managed_uki, sign_with_default_keys, SbsignMokSigner,
        DEFAULT_ENTRY_TOKEN_PATHS, DEFAULT_MANAGED_UKI_DIRS,
    },
    secureboot::nvram::{backup_efi_variables, DEFAULT_BACKUP_DIR, DEFAULT_EFIVARS_DIR},
    snapshot, systemd_boot_manager, uki_manager,
};
use bootcontrol_core::immutable_distro::ImmutableDistro;

use serde::Serialize;

/// Wire-format DTO for a single snapshot row, serialized as JSON for the
/// `ListSnapshots` D-Bus method. Mirrors `snapshot::SnapshotInfo` minus the
/// internal `manifest_path` field, which is a daemon-side filesystem detail.
///
/// The client (`crates/client/src/lib.rs`) declares a matching
/// `SnapshotInfoDto` and deserializes identical JSON.
/// Daemon-side mirror of `bootcontrol_client::EfiBootEntryDto`. Serialised
/// as JSON inside the `ListEfiBootEntries` D-Bus method's string return.
#[derive(Debug, Clone, Serialize)]
struct EfiBootEntryDto {
    pub index: u16,
    pub active: bool,
    pub hidden: bool,
    pub description: String,
}

/// Daemon-side wire DTO for one GRUB boot-menu item, serialized as JSON
/// inside the `ListGrubEntries` D-Bus method's string return. Mirrors
/// [`bootcontrol_core::grub_cfg::GrubMenuEntry`] field-for-field (core has
/// no serde dependency); the client declares a matching DTO and
/// deserializes identical JSON.
#[derive(Debug, Clone, Serialize)]
struct GrubMenuEntryDto {
    pub title: String,
    pub id: Option<String>,
    /// `GRUB_DEFAULT`-compatible index path, e.g. `"1>0"`.
    pub path: String,
    pub depth: usize,
    pub is_submenu: bool,
}

#[derive(Debug, Clone, Serialize)]
struct SnapshotInfoDto {
    /// Filesystem-safe snapshot id (e.g. `2026-04-30T130211Z-set_grub_value`).
    pub id: String,
    /// Operation tag the snapshot captured (e.g. `"set_grub_value"`).
    pub op: String,
    /// RFC 3339 timestamp the snapshot was taken.
    pub ts: String,
    /// Audit JOB_ID linking this snapshot to its journald audit row.
    pub audit_job_id: String,
}

use tracing::{info, warn};
use zbus::interface;

/// Generate a unique job_id for an audit transaction.
///
/// Format: `<epoch_ns>-<pid>` — non-cryptographic uniqueness suffices since
/// audit job ids are strictly internal correlation keys, not security tokens.
/// A real UUID v4 dependency is deferred to a future commit; the field type
/// is `String` so the swap is non-breaking.
fn new_job_id() -> String {
    let ns = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{:x}-{}", ns, std::process::id())
}

/// The D-Bus object that implements `org.bootcontrol.Manager`.
///
/// `grub_path` is the filesystem path to `/etc/default/grub`. It is
/// injectable via [`GrubManager::new`] so that integration tests can point
/// the daemon at a `tempfile` without needing real root access.
///
/// `failsafe_cfg_path` is the path to the failsafe menu-entry GRUB snippet
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
    /// Root directory under which pre-write snapshots are written.
    ///
    /// Production default: `/var/lib/bootcontrol/snapshots/`. PR 5b integrates
    /// `snapshot::create` into the `set_grub_value` write-path. Other write
    /// paths (`rebuild_grub_config`, systemd-boot, UKI, secureboot) integrate
    /// in follow-up commits using this same field.
    snapshot_root: PathBuf,
    /// ESP `EFI/Linux` directories in which signing is permitted.
    managed_uki_dirs: Vec<PathBuf>,
    /// Candidate files identifying the running installation's UKI token.
    entry_token_paths: Vec<PathBuf>,
    /// Required owner UID for a UKI. Production always uses root (`0`).
    trusted_uki_uid: u32,
    #[cfg(test)]
    test_hooks: Option<std::sync::Arc<tests::TestHooks>>,
}

fn default_managed_uki_dirs() -> Vec<PathBuf> {
    DEFAULT_MANAGED_UKI_DIRS.iter().map(PathBuf::from).collect()
}

fn default_entry_token_paths() -> Vec<PathBuf> {
    DEFAULT_ENTRY_TOKEN_PATHS
        .iter()
        .map(PathBuf::from)
        .collect()
}

impl GrubManager {
    /// Authorize an operation; tests inject a per-instance decision and recorder.
    ///
    /// # Errors
    /// Returns `PolkitDenied` when Polkit rejects the caller or cannot be reached.
    async fn authorize(
        &self,
        sender: &str,
        action: &str,
    ) -> Result<(), bootcontrol_core::error::BootControlError> {
        #[cfg(test)]
        if let Some(hooks) = &self.test_hooks {
            return hooks.authorize(sender, action);
        }
        authorize_with_polkit(sender, action).await
    }

    fn probe_distro(&self) -> Option<ImmutableDistro> {
        #[cfg(test)]
        if let Some(hooks) = &self.test_hooks {
            hooks
                .probes
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            return hooks.distro.clone();
        }
        probe_immutable_distro()
    }

    /// Reject immutable hosts after authorization.
    ///
    /// # Errors
    /// Returns `ImmutableDistroDetected` for hosts without imperative writes.
    fn require_writable_host(&self) -> Result<(), bootcontrol_core::error::BootControlError> {
        match self.probe_distro() {
            Some(distro) => Err(
                bootcontrol_core::error::BootControlError::ImmutableDistroDetected {
                    distro: distro.to_string(),
                },
            ),
            None => Ok(()),
        }
    }

    fn efivars_reader(&self) -> crate::uefi_vars_linux::EfivarFsReader {
        #[cfg(test)]
        if let Some(hooks) = &self.test_hooks {
            return crate::uefi_vars_linux::EfivarFsReader::with_root(hooks.efivars.clone());
        }
        crate::uefi_vars_linux::EfivarFsReader::new()
    }

    fn nvram_source(&self) -> PathBuf {
        #[cfg(test)]
        if let Some(hooks) = &self.test_hooks {
            return hooks.efivars.clone();
        }
        PathBuf::from(DEFAULT_EFIVARS_DIR)
    }

    fn nvram_backup_root(&self) -> PathBuf {
        #[cfg(test)]
        if let Some(hooks) = &self.test_hooks {
            return hooks.efivars.with_file_name("backups");
        }
        PathBuf::from(DEFAULT_BACKUP_DIR)
    }

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
            snapshot_root: PathBuf::from("/var/lib/bootcontrol/snapshots"),
            managed_uki_dirs: default_managed_uki_dirs(),
            entry_token_paths: default_entry_token_paths(),
            trusted_uki_uid: 0,
            #[cfg(test)]
            test_hooks: None,
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
            snapshot_root: PathBuf::from("/var/lib/bootcontrol/snapshots"),
            managed_uki_dirs: default_managed_uki_dirs(),
            entry_token_paths: default_entry_token_paths(),
            trusted_uki_uid: 0,
            #[cfg(test)]
            test_hooks: None,
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
            snapshot_root: PathBuf::from("/var/lib/bootcontrol/snapshots"),
            managed_uki_dirs: default_managed_uki_dirs(),
            entry_token_paths: default_entry_token_paths(),
            trusted_uki_uid: 0,
            #[cfg(test)]
            test_hooks: None,
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
            snapshot_root: PathBuf::from("/var/lib/bootcontrol/snapshots"),
            managed_uki_dirs: default_managed_uki_dirs(),
            entry_token_paths: default_entry_token_paths(),
            trusted_uki_uid: 0,
            #[cfg(test)]
            test_hooks: None,
        }
    }

    /// Override the UKI containment policy for an isolated test daemon.
    ///
    /// Production constructors retain root ownership and the standard ESP
    /// directories. The daemon entry point uses this builder only for a
    /// session-bus process compiled with `polkit-mock`.
    ///
    /// # Arguments
    ///
    /// * `managed_dirs` - Test directories corresponding to `EFI/Linux`.
    /// * `entry_token_paths` - Test files containing the installation token.
    /// * `trusted_uid` - UID owning the test UKI.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::{Path, PathBuf};
    /// use bootcontrol_core::backends::grub::GrubBackend;
    /// use bootcontrold::interface::GrubManager;
    ///
    /// let manager = GrubManager::new(
    ///     PathBuf::from("/tmp/grub"),
    ///     Box::new(GrubBackend),
    /// )
    /// .with_uki_policy(
    ///     vec![PathBuf::from("/tmp/EFI/Linux")],
    ///     vec![PathBuf::from("/tmp/entry-token")],
    ///     1000,
    /// );
    /// assert_eq!(manager.grub_path(), Path::new("/tmp/grub"));
    /// ```
    pub fn with_uki_policy(
        mut self,
        managed_dirs: Vec<PathBuf>,
        entry_token_paths: Vec<PathBuf>,
        trusted_uid: u32,
    ) -> Self {
        self.managed_uki_dirs = managed_dirs;
        self.entry_token_paths = entry_token_paths;
        self.trusted_uki_uid = trusted_uid;
        self
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

    /// Override the snapshot root directory.
    ///
    /// Used by the daemon entry-point to honour `BOOTCONTROL_SNAPSHOT_ROOT`
    /// and by E2E tests that run as an unprivileged user (no permission to
    /// write `/var/lib/bootcontrol/snapshots/`). Returns `self` for builder-
    /// style chaining.
    ///
    /// # Arguments
    ///
    /// * `root` — Directory where snapshot subdirectories will be created.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use bootcontrold::interface::GrubManager;
    /// use bootcontrol_core::backends::grub::GrubBackend;
    ///
    /// let m = GrubManager::new(PathBuf::from("/etc/default/grub"), Box::new(GrubBackend))
    ///     .with_snapshot_root(PathBuf::from("/tmp/snapshots"));
    /// assert_eq!(m.snapshot_root(), std::path::Path::new("/tmp/snapshots"));
    /// ```
    pub fn with_snapshot_root(mut self, root: PathBuf) -> Self {
        self.snapshot_root = root;
        self
    }

    /// Return the snapshot root directory.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::{Path, PathBuf};
    /// use bootcontrold::interface::GrubManager;
    /// use bootcontrol_core::backends::grub::GrubBackend;
    ///
    /// let m = GrubManager::new(PathBuf::from("/etc/default/grub"), Box::new(GrubBackend));
    /// assert_eq!(m.snapshot_root(), Path::new("/var/lib/bootcontrol/snapshots"));
    /// ```
    pub fn snapshot_root(&self) -> &Path {
        &self.snapshot_root
    }

    /// The set of paths this daemon is allowed to write — every file it owns
    /// and every directory it writes into.
    ///
    /// Used as the containment policy for [`snapshot::restore`]: a snapshot
    /// manifest may only restore paths inside this set. It is derived from the
    /// manager's configured paths rather than hardcoded, so an integration
    /// test pointing the manager at a `TempDir` restores under that tempdir
    /// and nowhere else — and so production and tests cannot drift apart.
    ///
    /// Adding a new file the daemon writes means adding it here too, or
    /// restoring a snapshot of it will be refused.
    fn managed_paths(&self) -> Vec<PathBuf> {
        vec![
            self.grub_path.clone(),
            self.failsafe_cfg_path.clone(),
            self.grub_cfg_path.clone(),
            self.loader_entries_dir.clone(),
            self.loader_conf_path.clone(),
            self.kernel_cmdline_path.clone(),
        ]
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
        self.authorize(
            &resolve_bus_name(&header, "SetGrubValue")?,
            actions::REWRITE_GRUB,
        )
        .await
        .map_err(|e| {
            warn!(caller_uid = %caller_uid, key = %key, "Polkit denied");
            to_daemon_error(e)
        })?;

        self.require_writable_host().map_err(to_daemon_error)?;

        // ── Step 3: Payload sanitization ────────────────────────────────────
        sanitize::check_payload(&key, &value).map_err(|e| {
            warn!(key = %key, value = %value, "Security policy violation");
            to_daemon_error(e)
        })?;

        // ── Step 4: Snapshot + audit Started ────────────────────────────────
        // Per docs/GUI_V2_SPEC_v2.md §6 daemon contract: a snapshot is the
        // pre-write covenant; if snapshotting fails the operation must abort
        // before any disk mutation.
        let job_id = new_job_id();
        let target_paths = vec![self.grub_path.display().to_string()];
        audit::emit(&AuditEvent {
            message_id: message_ids::SET_GRUB_VALUE,
            operation: "set_grub_value",
            phase: Phase::Started,
            target_paths: target_paths.clone(),
            etag_before: Some(etag.clone()),
            etag_after: None,
            snapshot_id: None,
            exit_code: None,
            caller_uid,
            polkit_action: "org.bootcontrol.rewrite-grub",
            job_id: job_id.clone(),
            stderr_tail: String::new(),
        });

        // Capture the snapshot using the caller-supplied ETag as etag_before.
        // Race window: another process could mutate /etc/default/grub between
        // here and grub_manager's flock acquire — but the ETag check INSIDE
        // grub_manager::set_grub_value will reject the write with
        // StateMismatch in that case, leaving an orphan snapshot that
        // snapshot::reap eventually clears. Tightening this to atomic
        // snapshot+lock requires a refactor of grub_manager into a
        // post-flock callback, deferred to a follow-up.
        let snap_info = snapshot::create(snapshot::SnapshotRequest {
            root: &self.snapshot_root,
            op: "set_grub_value",
            polkit_action: "org.bootcontrol.rewrite-grub",
            caller_uid,
            etag_before: &etag,
            files: std::slice::from_ref(&self.grub_path),
            audit_job_id: &job_id,
        })
        .map_err(|e| {
            warn!(error = %e, "snapshot creation failed — aborting write");
            DaemonError::EspScanFailed(format!("snapshot failed: {}", e))
        })?;

        audit::emit(&AuditEvent {
            message_id: message_ids::SET_GRUB_VALUE,
            operation: "set_grub_value",
            phase: Phase::SnapshotTaken,
            target_paths: target_paths.clone(),
            etag_before: Some(etag.clone()),
            etag_after: None,
            snapshot_id: Some(snap_info.id.clone()),
            exit_code: None,
            caller_uid,
            polkit_action: "org.bootcontrol.rewrite-grub",
            job_id: job_id.clone(),
            stderr_tail: String::new(),
        });

        // ── Steps 5–9: flock → ETag verify → atomic write → failsafe refresh → grub-mkconfig ──
        let result = grub_manager::set_grub_value(
            &self.grub_path,
            &key,
            &value,
            &etag,
            &self.failsafe_cfg_path,
            &self.grub_cfg_path,
        )
        .map_err(to_daemon_error);

        // ── Step 10: Audit Completed ────────────────────────────────────────
        let exit_code = if result.is_ok() { 0 } else { 1 };
        let stderr_tail = result
            .as_ref()
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default();
        audit::emit(&AuditEvent {
            message_id: message_ids::SET_GRUB_VALUE,
            operation: "set_grub_value",
            phase: Phase::Completed,
            target_paths,
            etag_before: Some(etag),
            etag_after: None, // post-write ETag re-read deferred to follow-up
            snapshot_id: Some(snap_info.id),
            exit_code: Some(exit_code),
            caller_uid,
            polkit_action: "org.bootcontrol.rewrite-grub",
            job_id,
            stderr_tail,
        });

        result
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

    /// List the boot menu entries from the generated `grub.cfg`.
    ///
    /// Parses the `menuentry`/`submenu` structure that GRUB would show at
    /// boot, in file order (A1 chunk 2 — the read half of GRUB menu-entry
    /// management).
    ///
    /// **Read-only — no Polkit authorization required.** Consistent with
    /// `ListLoaderEntries`: the same information is readable from
    /// `/boot/grub/grub.cfg` anyway (see `docs/threat-model.md` on
    /// unauthenticated read-only listing).
    ///
    /// ## D-Bus signature
    ///
    /// ```text
    /// ListGrubEntries() -> (s, s)
    /// ```
    ///
    /// ## Return value
    ///
    /// A tuple of:
    /// - `s` — JSON array of menu items; each object has `title`, `id`
    ///   (nullable), `path` (`GRUB_DEFAULT`-compatible index path, e.g.
    ///   `"1>0"`), `depth`, `is_submenu`.
    /// - `s` — 64-character lowercase hex SHA-256 ETag of `grub.cfg`,
    ///   letting callers detect that the menu was regenerated since listing.
    ///
    /// ## Errors
    ///
    /// - `org.bootcontrol.Error.EspScanFailed` — `grub.cfg` missing or
    ///   unreadable.
    /// - `org.bootcontrol.Error.MalformedValue` — menu structure could not
    ///   be parsed (unterminated block or quote).
    async fn list_grub_entries(&self) -> Result<(String, String), DaemonError> {
        info!(path = ?self.grub_cfg_path, "D-Bus: ListGrubEntries");
        let (entries, etag) =
            grub_manager::list_menu_entries(&self.grub_cfg_path).map_err(to_daemon_error)?;
        let dtos: Vec<GrubMenuEntryDto> = entries
            .into_iter()
            .map(|e| GrubMenuEntryDto {
                title: e.title,
                id: e.id,
                path: e.path,
                depth: e.depth,
                is_submenu: e.is_submenu,
            })
            .collect();
        let json = serde_json::to_string(&dtos)
            .map_err(|e| DaemonError::EspScanFailed(format!("serialization error: {e}")))?;
        Ok((json, etag))
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
        self.authorize(
            &resolve_bus_name(&header, "RebuildGrubConfig")?,
            actions::REWRITE_GRUB,
        )
        .await
        .map_err(|e| {
            warn!(caller_uid = %caller_uid, "Polkit denied for RebuildGrubConfig");
            to_daemon_error(e)
        })?;

        self.require_writable_host().map_err(to_daemon_error)?;

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
    /// * `target_dir` — Absolute directory within `/var/lib/bootcontrol/certs`.
    ///   Pass an empty string to create a fresh timestamped backup subdirectory.
    ///   Parent traversal, symlinks and overwriting existing files are refused.
    ///
    /// ## Errors
    ///
    /// - `org.bootcontrol.Error.PolkitDenied` — the caller is not authorized.
    /// - `org.bootcontrol.Error.NvramBackupFailed` — the sysfs efivars
    ///   directory is not mounted, no Secure Boot variables were found, or the
    ///   target is outside the managed root, contains traversal/symlinks, already
    ///   contains a backup file, or cannot be written and synced.
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
        self.authorize(
            &resolve_bus_name(&header, "BackupNvram")?,
            actions::ENROLL_MOK,
        )
        .await
        .map_err(|e| {
            warn!(caller_uid = %caller_uid, "Polkit denied for BackupNvram");
            to_daemon_error(e)
        })?;

        self.require_writable_host().map_err(to_daemon_error)?;

        // ── Step 3: Resolve target directory ────────────────────────────────
        let root = self.nvram_backup_root();
        let resolved_target = if target_dir.is_empty() {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|error| DaemonError::NvramBackupFailed(error.to_string()))?;
            root.join(format!("backup-{}", timestamp.as_nanos()))
        } else {
            std::path::PathBuf::from(&target_dir)
        };
        if !resolved_target.starts_with(&root)
            || resolved_target
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(DaemonError::NvramBackupFailed(format!(
                "backup destination must be inside {} without '..'",
                root.display()
            )));
        }

        // ── Step 4: Perform the backup ──────────────────────────────────────
        let backup = backup_efi_variables(&self.nvram_source(), &resolved_target).map_err(|e| {
            warn!(error = %e, "NVRAM backup failed");
            to_daemon_error(e)
        })?;

        info!(file_count = backup.files.len(), "NVRAM backup completed");

        // ── Step 5: Serialize file list as a JSON array ─────────────────────
        let json = serde_json::to_string(&backup.files)
            .map_err(|error| DaemonError::NvramBackupFailed(error.to_string()))?;

        Ok(json)
    }

    /// Sign a UKI image and enroll the MOK certificate.
    ///
    /// Signs the UKI at `uki_path` using the MOK key and certificate stored
    /// at the default paths (`/var/lib/bootcontrol/keys/mok.key` and
    /// `/var/lib/bootcontrol/keys/mok.crt`), then generates a MokManager
    /// enrollment request so the key is trusted on the next reboot.
    ///
    /// Requires Polkit authorization (`org.bootcontrol.enroll-mok`).
    ///
    /// ## D-Bus signature
    ///
    /// ```text
    /// SignAndEnrollUki(s) -> ()
    /// ```
    ///
    /// ## Arguments
    ///
    /// * `uki_path` — Absolute path to a root-owned, installation-associated
    ///   UKI in a managed `EFI/Linux` directory.
    ///
    /// ## Errors
    ///
    /// - `org.bootcontrol.Error.PolkitDenied` — the caller is not authorized.
    /// - `org.bootcontrol.Error.ImmutableDistroDetected` — the host forbids direct boot mutation.
    /// - `org.bootcontrol.Error.SecurityPolicyViolation` — the UKI is outside
    ///   managed storage, has unsafe ownership or permissions, or belongs to
    ///   another installation.
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
        self.authorize(
            &resolve_bus_name(&header, "SignAndEnrollUki")?,
            actions::ENROLL_MOK,
        )
        .await
        .map_err(|e| {
            warn!(caller_uid = %caller_uid, uki_path = %uki_path, "Polkit denied");
            to_daemon_error(e)
        })?;

        // Secure Boot inherits the same authorized host preflight as config writes.
        self.require_writable_host().map_err(to_daemon_error)?;

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
        let entry_token = read_entry_token(&self.entry_token_paths).map_err(|e| {
            warn!(uki_path = %uki_path, error = %e, "UKI installation token validation failed");
            to_daemon_error(e)
        })?;

        sign_managed_uki(
            &signer,
            uki,
            &key,
            &cert,
            &self.managed_uki_dirs,
            &entry_token,
            self.trusted_uki_uid,
        )
        .map_err(|e| {
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
        serde_json::to_string(&records)
            .map_err(|e| DaemonError::EspScanFailed(format!("serialization error: {e}")))
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
    /// - `org.bootcontrol.Error.MalformedValue` — `id` contains traversal or
    ///   path separators instead of a loader-entry filename stem.
    /// - `org.bootcontrol.Error.EspScanFailed` — entry not found or unreadable.
    async fn read_loader_entry(&self, id: String) -> Result<(String, String), DaemonError> {
        info!(id = %id, "D-Bus: ReadLoaderEntry");
        let (entry, etag) = systemd_boot_manager::read_entry(&self.loader_entries_dir, &id)
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
        let json = serde_json::to_string(&record)
            .map_err(|e| DaemonError::EspScanFailed(format!("serialization error: {e}")))?;
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
        let _caller_uid = resolve_uid(&header, connection, "SetLoaderDefault").await?;
        self.authorize(
            &resolve_bus_name(&header, "SetLoaderDefault")?,
            actions::WRITE_BOOTLOADER,
        )
        .await
        .map_err(to_daemon_error)?;

        self.require_writable_host().map_err(to_daemon_error)?;
        systemd_boot_manager::set_loader_default(&self.loader_conf_path, &id, &etag)
            .map_err(to_daemon_error)
    }

    /// Rename a systemd-boot loader entry. Only the `title <…>` line of the
    /// underlying `.conf` is rewritten; every other field is preserved
    /// byte-for-byte.
    ///
    /// Polkit-gated; immutable-distro pre-flight refuses on
    /// ostree/SteamOS/NixOS/Vanilla OS.
    ///
    /// ## Errors
    ///
    /// - `org.bootcontrol.Error.PolkitDenied`
    /// - `org.bootcontrol.Error.ImmutableDistroDetected`
    /// - `org.bootcontrol.Error.MalformedValue` — empty title or title
    ///   containing newline / CR (would corrupt the loader entry format).
    /// - `org.bootcontrol.Error.StateMismatch` — stale ETag.
    /// - `org.bootcontrol.Error.EspScanFailed` — entry file unreadable or
    ///   unwritable.
    async fn rename_loader_entry(
        &self,
        id: String,
        new_title: String,
        etag: String,
        #[zbus(header)] header: zbus::message::Header<'_>,
        #[zbus(connection)] connection: &zbus::Connection,
    ) -> Result<(), DaemonError> {
        info!(id = %id, new_title = %new_title, "D-Bus: RenameLoaderEntry");
        let _caller_uid = resolve_uid(&header, connection, "RenameLoaderEntry").await?;
        self.authorize(
            &resolve_bus_name(&header, "RenameLoaderEntry")?,
            actions::WRITE_BOOTLOADER,
        )
        .await
        .map_err(to_daemon_error)?;

        self.require_writable_host().map_err(to_daemon_error)?;
        systemd_boot_manager::rename_loader_entry(&self.loader_entries_dir, &id, &new_title, &etag)
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
        // Phase 6 PR2: on rpm-ostree hosts the on-disk cmdline is owned by
        // ostree and overwritten on every upgrade. Read through
        // `rpm-ostree kargs` instead so the value matches what will actually
        // boot.
        if let Some(ImmutableDistro::RpmOstree) = probe_immutable_distro() {
            return rpm_ostree::kargs_read().map_err(to_daemon_error);
        }
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
        // Phase 6 PR2: dispatch on host class.
        //
        //   classic mutable host  → uki_manager (write `/etc/kernel/cmdline`)
        //   rpm-ostree host       → rpm-ostree kargs --append=<param>
        //   bare ostree host      → reject (no supported delegation target)
        let _caller_uid = resolve_uid(&header, connection, "AddKernelParam").await?;
        self.authorize(
            &resolve_bus_name(&header, "AddKernelParam")?,
            actions::REWRITE_GRUB,
        )
        .await
        .map_err(to_daemon_error)?;
        match self.probe_distro() {
            Some(ImmutableDistro::RpmOstree) => {
                return rpm_ostree::kargs_append(&param, &etag).map_err(to_daemon_error);
            }
            Some(other) => {
                // Ostree (bare), SteamOs, NixOs, VanillaOs — none of these
                // have a supported delegation target for kernel cmdline.
                // Reject with the distro tag carried through so the frontend
                // can render the right "use this instead" hint.
                return Err(to_daemon_error(
                    bootcontrol_core::error::BootControlError::ImmutableDistroDetected {
                        distro: other.to_string(),
                    },
                ));
            }
            None => {}
        }

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
        // Phase 6 PR2: dispatch on host class (see AddKernelParam).
        let _caller_uid = resolve_uid(&header, connection, "RemoveKernelParam").await?;
        self.authorize(
            &resolve_bus_name(&header, "RemoveKernelParam")?,
            actions::REWRITE_GRUB,
        )
        .await
        .map_err(to_daemon_error)?;
        match self.probe_distro() {
            Some(ImmutableDistro::RpmOstree) => {
                return rpm_ostree::kargs_delete(&param, &etag).map_err(to_daemon_error);
            }
            Some(other) => {
                // Ostree (bare), SteamOs, NixOs, VanillaOs — none of these
                // have a supported delegation target for kernel cmdline.
                // Reject with the distro tag carried through so the frontend
                // can render the right "use this instead" hint.
                return Err(to_daemon_error(
                    bootcontrol_core::error::BootControlError::ImmutableDistroDetected {
                        distro: other.to_string(),
                    },
                ));
            }
            None => {}
        }

        uki_manager::remove_kernel_param(&self.kernel_cmdline_path, &param, &etag)
            .map_err(to_daemon_error)
    }

    // ── Snapshot methods ──────────────────────────────────────────────────────

    /// List all snapshots under `/var/lib/bootcontrol/snapshots/`, newest first.
    ///
    /// **Read-only — no Polkit authorization required.** Returns a JSON array
    /// of `{id, op, ts}` objects mirroring the on-disk `manifest.json`
    /// summary fields. The full manifest stays on disk and is loaded only
    /// when a snapshot is actually restored.
    ///
    /// ## D-Bus signature
    ///
    /// ```text
    /// ListSnapshots() -> s
    /// ```
    ///
    /// ## Errors
    ///
    /// - `org.bootcontrol.Error.SnapshotCorrupt` — a manifest is malformed or
    ///   declares an unsupported `schema_version`.
    /// - `org.bootcontrol.Error.SnapshotFailed` — the snapshot directory is
    ///   present but unreadable (I/O error).
    async fn list_snapshots(&self) -> Result<String, DaemonError> {
        info!(root = ?self.snapshot_root, "D-Bus: ListSnapshots");
        let snaps = snapshot::list(&self.snapshot_root).map_err(snapshot_to_daemon_error)?;
        let dtos: Vec<SnapshotInfoDto> = snaps
            .into_iter()
            .map(|s| SnapshotInfoDto {
                id: s.id,
                op: s.op,
                ts: s.ts,
                audit_job_id: s.audit_job_id,
            })
            .collect();
        serde_json::to_string(&dtos)
            .map_err(|e| DaemonError::SnapshotFailed(format!("serialization error: {e}")))
    }

    /// Restore a previously captured snapshot by id.
    ///
    /// Every file recorded in the snapshot's manifest is rewritten with the
    /// captured bytes. The operation is idempotent against the snapshot
    /// itself (running it twice yields the same on-disk state) but **does
    /// not** rebuild downstream artefacts: after a successful restore of a
    /// GRUB snapshot the caller should invoke `RebuildGrubConfig` to
    /// regenerate `/boot/grub/grub.cfg`.
    ///
    /// ## Write-path
    ///
    /// 1. Polkit authorization (action `org.bootcontrol.restore-snapshot`,
    ///    checked via the shared `authorize_with_polkit` helper).
    /// 2. Audit `Started` event with `message_ids::RESTORE_SNAPSHOT`.
    /// 3. `snapshot::restore` — overwrites each captured path with the
    ///    bytes stored under `<snapshot_root>/<id>/`.
    /// 4. Audit `Completed` event with the exit code and stderr tail.
    ///
    /// ## D-Bus signature
    ///
    /// ```text
    /// RestoreSnapshot(s, s) -> ()
    /// ```
    ///
    /// ## Arguments
    ///
    /// - `id` — Snapshot id as returned by `ListSnapshots`.
    /// - `expected_etag` — Current ETag of the primary target, obtained before
    ///   opening the confirmation flow.
    ///
    /// ## Errors
    ///
    /// - `org.bootcontrol.Error.PolkitDenied` — caller not authorized.
    /// - `org.bootcontrol.Error.SnapshotNotFound` — no snapshot with that id.
    /// - `org.bootcontrol.Error.SnapshotCorrupt` — manifest is malformed.
    /// - `org.bootcontrol.Error.ConcurrentModification` — a target is locked.
    /// - `org.bootcontrol.Error.StateMismatch` — `expected_etag` is stale.
    /// - `org.bootcontrol.Error.SnapshotFailed` — I/O error during restore.
    async fn restore_snapshot(
        &self,
        id: String,
        expected_etag: String,
        #[zbus(header)] header: zbus::message::Header<'_>,
        #[zbus(connection)] connection: &zbus::Connection,
    ) -> Result<(), DaemonError> {
        info!(id = %id, root = ?self.snapshot_root, "D-Bus: RestoreSnapshot");

        let caller_uid = resolve_uid(&header, connection, "RestoreSnapshot").await?;
        self.authorize(
            &resolve_bus_name(&header, "RestoreSnapshot")?,
            actions::RESTORE_SNAPSHOT,
        )
        .await
        .map_err(|e| {
            warn!(caller_uid = %caller_uid, id = %id, "Polkit denied for RestoreSnapshot");
            to_daemon_error(e)
        })?;

        // Restore inherits the host policy of forward configuration writes.
        self.require_writable_host().map_err(to_daemon_error)?;

        let job_id = new_job_id();
        let target_paths = vec![self.snapshot_root.join(&id).display().to_string()];
        audit::emit(&AuditEvent {
            message_id: message_ids::RESTORE_SNAPSHOT,
            operation: "restore_snapshot",
            phase: Phase::Started,
            target_paths: target_paths.clone(),
            etag_before: Some(expected_etag.clone()),
            etag_after: None,
            snapshot_id: Some(id.clone()),
            exit_code: None,
            caller_uid,
            polkit_action: "org.bootcontrol.restore-snapshot",
            job_id: job_id.clone(),
            stderr_tail: String::new(),
        });

        let result = snapshot::restore(
            &self.snapshot_root,
            &id,
            &self.managed_paths(),
            &expected_etag,
        )
        .map_err(snapshot_to_daemon_error);

        let exit_code = if result.is_ok() { 0 } else { 1 };
        let stderr_tail = result
            .as_ref()
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default();
        audit::emit(&AuditEvent {
            message_id: message_ids::RESTORE_SNAPSHOT,
            operation: "restore_snapshot",
            phase: Phase::Completed,
            target_paths,
            etag_before: Some(expected_etag),
            etag_after: None,
            snapshot_id: Some(id),
            exit_code: Some(exit_code),
            caller_uid,
            polkit_action: "org.bootcontrol.restore-snapshot",
            job_id,
            stderr_tail,
        });

        result
    }

    // ── EFI boot menu (Phase 7 follow-up: BootOrder / BootNext / entries) ────

    /// List every `Boot####` UEFI variable currently present in the global
    /// EFI namespace. Read-only, no Polkit.
    ///
    /// Returns a JSON array of `EfiBootEntryDto` records (matching the
    /// client-side DTO).
    ///
    /// ## Errors
    ///
    /// - `org.bootcontrol.Error.EspScanFailed` — efivarfs unreachable.
    /// - `org.bootcontrol.Error.MalformedValue` — at least one `Boot####`
    ///   payload was shorter than the load-option header.
    async fn list_efi_boot_entries(&self) -> Result<String, DaemonError> {
        info!("D-Bus: ListEfiBootEntries");
        let reader = self.efivars_reader();
        let entries =
            crate::uefi_vars_linux::list_boot_entries(&reader).map_err(to_daemon_error)?;
        let dtos: Vec<EfiBootEntryDto> = entries
            .into_iter()
            .map(|e| EfiBootEntryDto {
                index: e.index,
                active: e.active,
                hidden: e.hidden,
                description: e.description,
            })
            .collect();
        serde_json::to_string(&dtos)
            .map_err(|e| DaemonError::EspScanFailed(format!("serialization error: {e}")))
    }

    /// Read the current `BootOrder` UEFI variable.
    ///
    /// ## Errors
    ///
    /// - `org.bootcontrol.Error.EspScanFailed` — variable absent or efivarfs
    ///   unreachable.
    /// - `org.bootcontrol.Error.MalformedValue` — payload not a multiple
    ///   of 2 bytes.
    async fn get_boot_order(&self) -> Result<Vec<u16>, DaemonError> {
        info!("D-Bus: GetBootOrder");
        let reader = self.efivars_reader();
        use bootcontrol_core::uefi_vars::UefiVarReader;
        let payload = reader.read_global("BootOrder").map_err(to_daemon_error)?;
        bootcontrol_core::uefi_vars::parse_boot_order(&payload).map_err(to_daemon_error)
    }

    /// Replace `BootOrder` with a permutation of the current entries.
    /// Polkit-gated; pre-flight refuses on immutable distros.
    ///
    /// ## Errors
    ///
    /// - `org.bootcontrol.Error.PolkitDenied`
    /// - `org.bootcontrol.Error.ImmutableDistroDetected`
    /// - `org.bootcontrol.Error.MalformedValue` — `new_order` has duplicates
    ///   or does not match the current set of indices (add/remove is a
    ///   separate operation, deliberately).
    async fn set_boot_order(
        &self,
        new_order: Vec<u16>,
        #[zbus(header)] header: zbus::message::Header<'_>,
        #[zbus(connection)] connection: &zbus::Connection,
    ) -> Result<(), DaemonError> {
        info!(new_order = ?new_order, "D-Bus: SetBootOrder");
        let _caller_uid = resolve_uid(&header, connection, "SetBootOrder").await?;
        self.authorize(
            &resolve_bus_name(&header, "SetBootOrder")?,
            actions::WRITE_BOOTLOADER,
        )
        .await
        .map_err(to_daemon_error)?;

        self.require_writable_host().map_err(to_daemon_error)?;
        let reader = self.efivars_reader();
        bootcontrol_core::uefi_vars::set_boot_order(&reader, &reader, &new_order)
            .map_err(to_daemon_error)
    }

    /// Read `BootNext`. Returns `-1` when the variable is absent (next
    /// boot follows `BootOrder`), `0..=65535` when set.
    ///
    /// Idempotent and read-only — no Polkit.
    async fn get_boot_next(&self) -> Result<i32, DaemonError> {
        info!("D-Bus: GetBootNext");
        let reader = self.efivars_reader();
        use bootcontrol_core::uefi_vars::UefiVarReader;
        match reader.read_global("BootNext") {
            Ok(payload) => bootcontrol_core::uefi_vars::parse_single_u16_var("BootNext", &payload)
                .map(|v| v as i32)
                .map_err(to_daemon_error),
            // Missing variable is not an error — it just means "no override".
            Err(bootcontrol_core::error::BootControlError::EspScanFailed { .. }) => Ok(-1),
            Err(e) => Err(to_daemon_error(e)),
        }
    }

    /// Set `BootNext` to `index`, a one-shot override consumed by the
    /// firmware on the next boot. Polkit-gated; pre-flight refuses on
    /// immutable distros.
    ///
    /// ## Errors
    ///
    /// - `org.bootcontrol.Error.PolkitDenied`
    /// - `org.bootcontrol.Error.ImmutableDistroDetected`
    /// - `org.bootcontrol.Error.KeyNotFound` — `index` does not appear in
    ///   the current `BootOrder`.
    async fn set_boot_next(
        &self,
        index: u16,
        #[zbus(header)] header: zbus::message::Header<'_>,
        #[zbus(connection)] connection: &zbus::Connection,
    ) -> Result<(), DaemonError> {
        info!(index = index, "D-Bus: SetBootNext");
        let _caller_uid = resolve_uid(&header, connection, "SetBootNext").await?;
        self.authorize(
            &resolve_bus_name(&header, "SetBootNext")?,
            actions::WRITE_BOOTLOADER,
        )
        .await
        .map_err(to_daemon_error)?;

        self.require_writable_host().map_err(to_daemon_error)?;
        let reader = self.efivars_reader();
        bootcontrol_core::uefi_vars::set_boot_next(&reader, &reader, index, true)
            .map_err(to_daemon_error)
    }

    /// Clear `BootNext` so the next boot falls back to `BootOrder`.
    /// Idempotent; Polkit-gated; pre-flight refuses on immutable distros.
    async fn clear_boot_next(
        &self,
        #[zbus(header)] header: zbus::message::Header<'_>,
        #[zbus(connection)] connection: &zbus::Connection,
    ) -> Result<(), DaemonError> {
        info!("D-Bus: ClearBootNext");
        let _caller_uid = resolve_uid(&header, connection, "ClearBootNext").await?;
        self.authorize(
            &resolve_bus_name(&header, "ClearBootNext")?,
            actions::WRITE_BOOTLOADER,
        )
        .await
        .map_err(to_daemon_error)?;

        self.require_writable_host().map_err(to_daemon_error)?;
        let reader = self.efivars_reader();
        bootcontrol_core::uefi_vars::clear_boot_next(&reader).map_err(to_daemon_error)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Private helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Return the unspoofable unique D-Bus sender name used as the Polkit subject.
fn resolve_bus_name(
    header: &zbus::message::Header<'_>,
    method_name: &str,
) -> Result<String, DaemonError> {
    header.sender().map(ToString::to_string).ok_or_else(|| {
        warn!(method = method_name, "D-Bus message has no sender field");
        DaemonError::PolkitDenied("missing sender in D-Bus message".to_string())
    })
}

/// Resolve the caller's Unix UID from a D-Bus message header.
///
/// Shared by all write methods that need Polkit authorization.
async fn resolve_uid(
    header: &zbus::message::Header<'_>,
    connection: &zbus::Connection,
    method_name: &str,
) -> Result<u32, DaemonError> {
    let sender = header
        .sender()
        .ok_or_else(|| {
            warn!(method = method_name, "D-Bus message has no sender field");
            DaemonError::PolkitDenied("missing sender in D-Bus message".to_string())
        })?
        .clone();

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

#[cfg(test)]
mod tests;
