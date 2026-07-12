//! D-Bus adapter + `MockBackend` for the BootControl daemon.
//!
//! Frontends (`crates/cli`, `crates/tui`, `crates/gui`) talk to this crate
//! and never touch `bootcontrold` directly — that boundary is enforced by
//! `audit.sh` and one of the active decisions in
//! `.claude/rules/decisions.md` (2026-05-03 "Frontendy nie omijają
//! client"). The crate exports three layers:
//!
//! - **Wire DTOs** ([`LoaderEntryDto`], [`EfiBootEntryDto`],
//!   [`SnapshotInfoDto`]) — owned by the client so the JSON-over-D-Bus
//!   shape is one place, serde-round-trippable, no `core` leakage.
//! - **[`BootBackend`] trait** — async, object-safe via `async_trait`.
//!   Concrete implementations live in this crate ([`DbusBackend`],
//!   [`MockBackend`]) so frontends only `Arc<dyn BootBackend>`.
//! - **Connection helpers** ([`connect_bus`], [`resolve_backend`],
//!   [`dbus_error_message`]) — the three free functions every frontend
//!   needs at startup.
//!
//! # Examples
//!
//! End-to-end client usage with [`MockBackend`] (no daemon, no bus —
//! demonstrates the wiring used by Demo Mode and every doctest below):
//!
//! ```
//! use bootcontrol_client::{BootBackend, MockBackend};
//!
//! # async fn run() -> zbus::Result<()> {
//! let backend = MockBackend;
//! let (config, etag) = backend.read_config().await?;
//! assert!(config.contains_key("GRUB_TIMEOUT"));
//! assert!(!etag.is_empty());
//! # Ok(())
//! # }
//! # tokio::runtime::Runtime::new().unwrap().block_on(run()).unwrap();
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use zbus::{proxy, Connection};

// ─────────────────────────────────────────────────────────────────────────────
// Shared DTO types
// ─────────────────────────────────────────────────────────────────────────────

/// A systemd-boot loader entry as returned by the daemon.
///
/// `id` is the filename stem (e.g. `"arch"` for `arch.conf`).
/// `etag` is the SHA-256 of that specific file.
///
/// # Examples
///
/// ```
/// use bootcontrol_client::LoaderEntryDto;
///
/// let entry = LoaderEntryDto {
///     id: "arch".to_string(),
///     title: Some("Arch Linux".to_string()),
///     linux: Some("/vmlinuz-linux".to_string()),
///     initrd: Some("/initramfs-linux.img".to_string()),
///     options: Some("root=/dev/sda1 rw".to_string()),
///     machine_id: None,
///     etag: "deadbeef".to_string(),
///     is_default: true,
/// };
///
/// // DTO is serde-round-trippable so it can cross the D-Bus boundary
/// // as a JSON-encoded `s` argument.
/// let json = serde_json::to_string(&entry).unwrap();
/// let back: LoaderEntryDto = serde_json::from_str(&json).unwrap();
/// assert_eq!(entry, back);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoaderEntryDto {
    /// Filename stem (e.g. `"arch"`).
    pub id: String,
    /// Entry title, if present.
    pub title: Option<String>,
    /// Kernel image path (e.g. `/vmlinuz-linux`).
    pub linux: Option<String>,
    /// Initramfs path.
    pub initrd: Option<String>,
    /// Kernel command-line options.
    pub options: Option<String>,
    /// Machine ID from `/etc/machine-id`.
    pub machine_id: Option<String>,
    /// Per-file SHA-256 ETag.
    pub etag: String,
    /// `true` when this entry is the current default.
    pub is_default: bool,
}

/// A snapshot row as returned by `ListSnapshots`.
///
/// Mirrors the on-disk manifest summary fields. The full manifest stays
/// in `/var/lib/bootcontrol/snapshots/<id>/manifest.json` and is read by
/// the daemon only when a snapshot is actually restored.
/// A parsed `Boot####` UEFI variable as returned by `ListEfiBootEntries`.
///
/// Mirrors `bootcontrol_core::uefi_vars::EfiBootEntry`. Kept as a separate
/// DTO so the wire format is owned by the client crate (the daemon
/// serialises a `Vec<EfiBootEntryDto>` to JSON and ships it as a single
/// string D-Bus method return; the client deserialises the same shape).
///
/// # Examples
///
/// ```
/// use bootcontrol_client::EfiBootEntryDto;
///
/// let entry = EfiBootEntryDto {
///     index: 0x0001,
///     active: true,
///     hidden: false,
///     description: "Windows Boot Manager".to_string(),
/// };
///
/// // Round-trip through JSON the way the daemon ships it on the wire.
/// let json = serde_json::to_string(&entry).unwrap();
/// let back: EfiBootEntryDto = serde_json::from_str(&json).unwrap();
/// assert_eq!(entry, back);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EfiBootEntryDto {
    /// Hex index from the variable name (e.g. `1` for `Boot0001`).
    pub index: u16,
    /// `LOAD_OPTION_ACTIVE` bit; inactive entries are skipped by firmware.
    pub active: bool,
    /// `LOAD_OPTION_HIDDEN` bit; hidden entries do not appear in firmware menus.
    pub hidden: bool,
    /// Human-readable description decoded from UTF-16. Examples:
    /// `"Windows Boot Manager"`, `"ubuntu"`, `"UEFI: SK hynix BC711..."`.
    pub description: String,
}

/// A snapshot row as returned by `ListSnapshots`.
///
/// Mirrors the on-disk manifest summary fields. The full manifest stays
/// in `/var/lib/bootcontrol/snapshots/<id>/manifest.json` and is read by
/// the daemon only when a snapshot is actually restored.
///
/// # Examples
///
/// ```
/// use bootcontrol_client::SnapshotInfoDto;
///
/// let info = SnapshotInfoDto {
///     id: "2026-04-30T130211Z-set_grub_value".to_string(),
///     op: "set_grub_value".to_string(),
///     ts: "2026-04-30T13:02:11Z".to_string(),
///     audit_job_id: "deadbeef-...".to_string(),
/// };
///
/// let json = serde_json::to_string(&info).unwrap();
/// let back: SnapshotInfoDto = serde_json::from_str(&json).unwrap();
/// assert_eq!(info, back);
///
/// // `audit_job_id` defaults to "" for snapshots created before the field
/// // was introduced — see `#[serde(default)]` on the field.
/// let legacy_json = r#"{"id":"x","op":"y","ts":"z"}"#;
/// let legacy: SnapshotInfoDto = serde_json::from_str(legacy_json).unwrap();
/// assert_eq!(legacy.audit_job_id, "");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnapshotInfoDto {
    /// Filesystem-safe snapshot id (e.g. `2026-04-30T130211Z-set_grub_value`).
    pub id: String,
    /// Operation tag the snapshot captured (e.g. `"set_grub_value"`).
    pub op: String,
    /// RFC 3339 timestamp the snapshot was taken.
    pub ts: String,
    /// Audit JOB_ID linking this snapshot to its journald audit row.
    /// Empty for snapshots created before the field was introduced.
    #[serde(default)]
    pub audit_job_id: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// D-Bus proxy
// ─────────────────────────────────────────────────────────────────────────────

/// D-Bus proxy for the `org.bootcontrol.Manager` interface exposed by `bootcontrold`.
#[proxy(
    interface = "org.bootcontrol.Manager",
    default_service = "org.bootcontrol.Manager",
    default_path = "/org/bootcontrol/Manager"
)]
pub trait Manager {
    // ── GRUB ──────────────────────────────────────────────────────────────────

    /// Read the GRUB configuration.
    async fn read_grub_config(&self) -> zbus::Result<(HashMap<String, String>, String)>;

    /// Set a GRUB value.
    async fn set_grub_value(&self, key: &str, value: &str, etag: &str) -> zbus::Result<()>;

    /// Get the current ETag of `/etc/default/grub`.
    async fn get_etag(&self) -> zbus::Result<String>;

    /// Get the name of the active bootloader backend.
    async fn get_active_backend(&self) -> zbus::Result<String>;

    /// Rebuild the GRUB config by running grub-mkconfig.
    async fn rebuild_grub_config(&self) -> zbus::Result<()>;

    // ── Secure Boot ───────────────────────────────────────────────────────────

    /// Back up EFI NVRAM variables to `target_dir` (empty = default path).
    /// Returns a JSON array of absolute paths of backed-up files.
    async fn backup_nvram(&self, target_dir: &str) -> zbus::Result<String>;

    /// Sign a UKI at `uki_path` and enroll the MOK certificate.
    async fn sign_and_enroll_uki(&self, uki_path: &str) -> zbus::Result<()>;

    // ── systemd-boot ──────────────────────────────────────────────────────────

    /// List all systemd-boot loader entries.
    /// Returns a JSON array of `LoaderEntryDto` objects.
    async fn list_loader_entries(&self) -> zbus::Result<String>;

    /// Read a single loader entry by ID.
    /// Returns `(json_entry, file_etag)`.
    async fn read_loader_entry(&self, id: &str) -> zbus::Result<(String, String)>;

    /// Set the default systemd-boot loader entry.
    async fn set_loader_default(&self, id: &str, etag: &str) -> zbus::Result<()>;

    /// Rename a systemd-boot loader entry (rewrites only the `title` line).
    async fn rename_loader_entry(&self, id: &str, new_title: &str, etag: &str) -> zbus::Result<()>;

    /// Get the ETag of `loader.conf`.
    async fn get_loader_conf_etag(&self) -> zbus::Result<String>;

    // ── UKI / kernel cmdline ──────────────────────────────────────────────────

    /// Read `/etc/kernel/cmdline`.  Returns `(params, etag)`.
    async fn read_kernel_cmdline(&self) -> zbus::Result<(Vec<String>, String)>;

    /// Add a kernel parameter to `/etc/kernel/cmdline`.
    async fn add_kernel_param(&self, param: &str, etag: &str) -> zbus::Result<()>;

    /// Remove a kernel parameter from `/etc/kernel/cmdline`.
    async fn remove_kernel_param(&self, param: &str, etag: &str) -> zbus::Result<()>;

    // ── Snapshots ─────────────────────────────────────────────────────────────

    /// List all snapshots, newest first.
    /// Returns a JSON array of `SnapshotInfoDto` objects.
    async fn list_snapshots(&self) -> zbus::Result<String>;

    /// Restore a snapshot by id. Overwrites all files captured in the manifest.
    async fn restore_snapshot(&self, id: &str) -> zbus::Result<()>;

    // ── EFI boot menu ─────────────────────────────────────────────────────────

    /// List every `Boot####` UEFI variable as a JSON array of
    /// `EfiBootEntryDto`.
    async fn list_efi_boot_entries(&self) -> zbus::Result<String>;

    /// Read the current `BootOrder` UEFI variable.
    async fn get_boot_order(&self) -> zbus::Result<Vec<u16>>;

    /// Replace `BootOrder` with `new_order`. Must be a permutation of the
    /// current set of indices — add/remove are out of scope here.
    async fn set_boot_order(&self, new_order: Vec<u16>) -> zbus::Result<()>;

    /// Read `BootNext`. Returns `-1` when the variable is absent (the
    /// next boot follows the normal `BootOrder`), `0..=65535` otherwise.
    async fn get_boot_next(&self) -> zbus::Result<i32>;

    /// Set `BootNext` to `index`, a one-shot override consumed by firmware
    /// on the next boot.
    async fn set_boot_next(&self, index: u16) -> zbus::Result<()>;

    /// Clear `BootNext` so the next boot falls back to `BootOrder`.
    /// Idempotent — succeeds when the variable was already absent.
    async fn clear_boot_next(&self) -> zbus::Result<()>;
}

// ─────────────────────────────────────────────────────────────────────────────
// BootBackend trait
// ─────────────────────────────────────────────────────────────────────────────

/// Abstract interface for BootControl operations.
///
/// Object-safe via [`async_trait`], so frontends carry an
/// `Arc<dyn BootBackend>` and never spell out the concrete type. The two
/// implementations shipped by this crate are [`DbusBackend`] (production —
/// proxies every call through `org.bootcontrol.Manager`) and
/// [`MockBackend`] (Demo Mode — returns plausible static data).
///
/// Frontends pick one via [`resolve_backend`] at startup; tests
/// instantiate [`MockBackend`] directly.
///
/// # Examples
///
/// ```
/// use bootcontrol_client::{BootBackend, MockBackend};
///
/// # async fn run() -> zbus::Result<()> {
/// let backend: &dyn BootBackend = &MockBackend;
/// let backend_name = backend.get_active_backend().await?;
/// // MockBackend identifies itself so frontends can render "Demo Mode" hints.
/// assert!(backend_name.contains("mock"));
/// # Ok(())
/// # }
/// # tokio::runtime::Runtime::new().unwrap().block_on(run()).unwrap();
/// ```
#[async_trait]
pub trait BootBackend: Send + Sync {
    // ── GRUB ──────────────────────────────────────────────────────────────────

    /// Read the GRUB boot configuration (key-values and ETag).
    async fn read_config(&self) -> zbus::Result<(HashMap<String, String>, String)>;

    /// Set a single GRUB configuration value.
    async fn set_value(&self, key: &str, value: &str, etag: &str) -> zbus::Result<()>;

    /// Return the name of the active backend (e.g., `"grub"`, `"systemd-boot"`).
    async fn get_active_backend(&self) -> zbus::Result<String>;

    /// Rebuild the GRUB config file (runs grub-mkconfig).
    async fn rebuild_grub_config(&self) -> zbus::Result<()>;

    // ── Secure Boot ───────────────────────────────────────────────────────────

    /// Back up EFI NVRAM variables. Returns JSON list of backed-up file paths.
    async fn backup_nvram(&self, target_dir: &str) -> zbus::Result<String>;

    /// Sign a UKI and enroll the MOK certificate.
    async fn sign_and_enroll_uki(&self, uki_path: &str) -> zbus::Result<()>;

    // ── systemd-boot ──────────────────────────────────────────────────────────

    /// List all systemd-boot loader entries.
    async fn list_loader_entries(&self) -> zbus::Result<Vec<LoaderEntryDto>>;

    /// Read a single loader entry by ID. Returns `(entry, file_etag)`.
    async fn read_loader_entry(&self, id: &str) -> zbus::Result<(LoaderEntryDto, String)>;

    /// Set the default systemd-boot entry.
    async fn set_loader_default(&self, id: &str, etag: &str) -> zbus::Result<()>;

    /// Rename a systemd-boot loader entry. Only the `title` line is
    /// rewritten; every other field stays byte-identical.
    async fn rename_loader_entry(&self, id: &str, new_title: &str, etag: &str) -> zbus::Result<()>;

    /// Get the ETag of `loader.conf`.
    async fn get_loader_conf_etag(&self) -> zbus::Result<String>;

    // ── UKI / kernel cmdline ──────────────────────────────────────────────────

    /// Read `/etc/kernel/cmdline`. Returns `(params, etag)`.
    async fn read_kernel_cmdline(&self) -> zbus::Result<(Vec<String>, String)>;

    /// Add a kernel parameter.
    async fn add_kernel_param(&self, param: &str, etag: &str) -> zbus::Result<()>;

    /// Remove a kernel parameter.
    async fn remove_kernel_param(&self, param: &str, etag: &str) -> zbus::Result<()>;

    // ── Snapshots ─────────────────────────────────────────────────────────────

    /// List all snapshots, newest first.
    async fn list_snapshots(&self) -> zbus::Result<Vec<SnapshotInfoDto>>;

    /// Restore a snapshot by id.
    async fn restore_snapshot(&self, id: &str) -> zbus::Result<()>;

    // ── EFI boot menu ─────────────────────────────────────────────────────────

    /// JSON array of `EfiBootEntryDto`.
    async fn list_efi_boot_entries(&self) -> zbus::Result<String>;

    /// Current `BootOrder` indices, in firmware-defined order.
    async fn get_boot_order(&self) -> zbus::Result<Vec<u16>>;

    /// Replace `BootOrder` with a permutation of the current entries.
    async fn set_boot_order(&self, new_order: Vec<u16>) -> zbus::Result<()>;

    /// `-1` when unset, `0..=65535` when set.
    async fn get_boot_next(&self) -> zbus::Result<i32>;

    /// One-shot boot override consumed by firmware on next boot.
    async fn set_boot_next(&self, index: u16) -> zbus::Result<()>;

    /// Idempotent — succeeds even when `BootNext` is already absent.
    async fn clear_boot_next(&self) -> zbus::Result<()>;
}

// ─────────────────────────────────────────────────────────────────────────────
// Real D-Bus backend
// ─────────────────────────────────────────────────────────────────────────────

/// Real D-Bus backend that talks to bootcontrold.
pub struct DbusBackend {
    conn: Connection,
}

impl DbusBackend {
    /// Create a new [`DbusBackend`] from a D-Bus connection.
    pub fn new(conn: Connection) -> Self {
        Self { conn }
    }
}

#[async_trait]
impl BootBackend for DbusBackend {
    async fn read_config(&self) -> zbus::Result<(HashMap<String, String>, String)> {
        let proxy = ManagerProxy::new(&self.conn).await?;
        proxy.read_grub_config().await
    }

    async fn set_value(&self, key: &str, value: &str, etag: &str) -> zbus::Result<()> {
        let proxy = ManagerProxy::new(&self.conn).await?;
        proxy.set_grub_value(key, value, etag).await
    }

    async fn get_active_backend(&self) -> zbus::Result<String> {
        let proxy = ManagerProxy::new(&self.conn).await?;
        proxy.get_active_backend().await
    }

    async fn rebuild_grub_config(&self) -> zbus::Result<()> {
        let proxy = ManagerProxy::new(&self.conn).await?;
        proxy.rebuild_grub_config().await
    }

    async fn backup_nvram(&self, target_dir: &str) -> zbus::Result<String> {
        let proxy = ManagerProxy::new(&self.conn).await?;
        proxy.backup_nvram(target_dir).await
    }

    async fn sign_and_enroll_uki(&self, uki_path: &str) -> zbus::Result<()> {
        let proxy = ManagerProxy::new(&self.conn).await?;
        proxy.sign_and_enroll_uki(uki_path).await
    }

    async fn list_loader_entries(&self) -> zbus::Result<Vec<LoaderEntryDto>> {
        let proxy = ManagerProxy::new(&self.conn).await?;
        let json = proxy.list_loader_entries().await?;
        serde_json::from_str(&json)
            .map_err(|e| zbus::Error::Failure(format!("failed to deserialize loader entries: {e}")))
    }

    async fn read_loader_entry(&self, id: &str) -> zbus::Result<(LoaderEntryDto, String)> {
        let proxy = ManagerProxy::new(&self.conn).await?;
        let (json, etag) = proxy.read_loader_entry(id).await?;
        let entry = serde_json::from_str(&json).map_err(|e| {
            zbus::Error::Failure(format!("failed to deserialize loader entry: {e}"))
        })?;
        Ok((entry, etag))
    }

    async fn set_loader_default(&self, id: &str, etag: &str) -> zbus::Result<()> {
        let proxy = ManagerProxy::new(&self.conn).await?;
        proxy.set_loader_default(id, etag).await
    }

    async fn rename_loader_entry(&self, id: &str, new_title: &str, etag: &str) -> zbus::Result<()> {
        let proxy = ManagerProxy::new(&self.conn).await?;
        proxy.rename_loader_entry(id, new_title, etag).await
    }

    async fn get_loader_conf_etag(&self) -> zbus::Result<String> {
        let proxy = ManagerProxy::new(&self.conn).await?;
        proxy.get_loader_conf_etag().await
    }

    async fn read_kernel_cmdline(&self) -> zbus::Result<(Vec<String>, String)> {
        let proxy = ManagerProxy::new(&self.conn).await?;
        proxy.read_kernel_cmdline().await
    }

    async fn add_kernel_param(&self, param: &str, etag: &str) -> zbus::Result<()> {
        let proxy = ManagerProxy::new(&self.conn).await?;
        proxy.add_kernel_param(param, etag).await
    }

    async fn remove_kernel_param(&self, param: &str, etag: &str) -> zbus::Result<()> {
        let proxy = ManagerProxy::new(&self.conn).await?;
        proxy.remove_kernel_param(param, etag).await
    }

    async fn list_snapshots(&self) -> zbus::Result<Vec<SnapshotInfoDto>> {
        let proxy = ManagerProxy::new(&self.conn).await?;
        let json = proxy.list_snapshots().await?;
        serde_json::from_str(&json)
            .map_err(|e| zbus::Error::Failure(format!("failed to deserialize snapshot list: {e}")))
    }

    async fn restore_snapshot(&self, id: &str) -> zbus::Result<()> {
        let proxy = ManagerProxy::new(&self.conn).await?;
        proxy.restore_snapshot(id).await
    }

    async fn list_efi_boot_entries(&self) -> zbus::Result<String> {
        let proxy = ManagerProxy::new(&self.conn).await?;
        proxy.list_efi_boot_entries().await
    }

    async fn get_boot_order(&self) -> zbus::Result<Vec<u16>> {
        let proxy = ManagerProxy::new(&self.conn).await?;
        proxy.get_boot_order().await
    }

    async fn set_boot_order(&self, new_order: Vec<u16>) -> zbus::Result<()> {
        let proxy = ManagerProxy::new(&self.conn).await?;
        proxy.set_boot_order(new_order).await
    }

    async fn get_boot_next(&self) -> zbus::Result<i32> {
        let proxy = ManagerProxy::new(&self.conn).await?;
        proxy.get_boot_next().await
    }

    async fn set_boot_next(&self, index: u16) -> zbus::Result<()> {
        let proxy = ManagerProxy::new(&self.conn).await?;
        proxy.set_boot_next(index).await
    }

    async fn clear_boot_next(&self) -> zbus::Result<()> {
        let proxy = ManagerProxy::new(&self.conn).await?;
        proxy.clear_boot_next().await
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Mock backend (Demo Mode)
// ─────────────────────────────────────────────────────────────────────────────

/// Mock backend for Demo Mode (works on Mac/any OS without a daemon).
pub struct MockBackend;

#[async_trait]
impl BootBackend for MockBackend {
    async fn read_config(&self) -> zbus::Result<(HashMap<String, String>, String)> {
        let mut mock = HashMap::new();
        mock.insert("GRUB_TIMEOUT".to_string(), "5".to_string());
        mock.insert("GRUB_DEFAULT".to_string(), "0".to_string());
        mock.insert("GRUB_DISTRIBUTOR".to_string(), "MockOS".to_string());
        mock.insert(
            "GRUB_CMDLINE_LINUX_DEFAULT".to_string(),
            "quiet splash".to_string(),
        );
        mock.insert("GRUB_DISABLE_OS_PROBER".to_string(), "false".to_string());
        Ok((mock, "mock-etag-12345".to_string()))
    }

    async fn set_value(&self, _key: &str, _value: &str, _etag: &str) -> zbus::Result<()> {
        Ok(())
    }

    async fn get_active_backend(&self) -> zbus::Result<String> {
        Ok("grub (mock)".to_string())
    }

    async fn rebuild_grub_config(&self) -> zbus::Result<()> {
        Ok(())
    }

    async fn backup_nvram(&self, _target_dir: &str) -> zbus::Result<String> {
        Ok(r#"["/var/lib/bootcontrol/certs/db-mock.efi","/var/lib/bootcontrol/certs/KEK-mock.efi","/var/lib/bootcontrol/certs/PK-mock.efi"]"#.to_string())
    }

    async fn sign_and_enroll_uki(&self, _uki_path: &str) -> zbus::Result<()> {
        Ok(())
    }

    async fn list_loader_entries(&self) -> zbus::Result<Vec<LoaderEntryDto>> {
        Ok(vec![
            LoaderEntryDto {
                id: "arch".to_string(),
                title: Some("Arch Linux".to_string()),
                linux: Some("/vmlinuz-linux".to_string()),
                initrd: Some("/initramfs-linux.img".to_string()),
                options: Some("root=/dev/sda1 rw quiet".to_string()),
                machine_id: None,
                etag: "mock-entry-etag-arch".to_string(),
                is_default: true,
            },
            LoaderEntryDto {
                id: "arch-fallback".to_string(),
                title: Some("Arch Linux (fallback)".to_string()),
                linux: Some("/vmlinuz-linux".to_string()),
                initrd: Some("/initramfs-linux-fallback.img".to_string()),
                options: Some("root=/dev/sda1 rw".to_string()),
                machine_id: None,
                etag: "mock-entry-etag-fallback".to_string(),
                is_default: false,
            },
        ])
    }

    async fn read_loader_entry(&self, id: &str) -> zbus::Result<(LoaderEntryDto, String)> {
        let entry = LoaderEntryDto {
            id: id.to_string(),
            title: Some(format!("Mock Entry: {id}")),
            linux: Some("/vmlinuz-linux".to_string()),
            initrd: Some("/initramfs-linux.img".to_string()),
            options: Some("root=/dev/sda1 rw quiet".to_string()),
            machine_id: None,
            etag: "mock-entry-etag-12345".to_string(),
            is_default: false,
        };
        Ok((entry, "mock-entry-etag-12345".to_string()))
    }

    async fn rename_loader_entry(
        &self,
        _id: &str,
        _new_title: &str,
        _etag: &str,
    ) -> zbus::Result<()> {
        Ok(())
    }

    async fn set_loader_default(&self, _id: &str, _etag: &str) -> zbus::Result<()> {
        Ok(())
    }

    async fn get_loader_conf_etag(&self) -> zbus::Result<String> {
        Ok("mock-loader-conf-etag-12345".to_string())
    }

    async fn read_kernel_cmdline(&self) -> zbus::Result<(Vec<String>, String)> {
        Ok((
            vec![
                "root=/dev/sda1".to_string(),
                "rw".to_string(),
                "quiet".to_string(),
                "splash".to_string(),
            ],
            "mock-cmdline-etag-12345".to_string(),
        ))
    }

    async fn add_kernel_param(&self, _param: &str, _etag: &str) -> zbus::Result<()> {
        Ok(())
    }

    async fn remove_kernel_param(&self, _param: &str, _etag: &str) -> zbus::Result<()> {
        Ok(())
    }

    async fn list_snapshots(&self) -> zbus::Result<Vec<SnapshotInfoDto>> {
        Ok(vec![
            SnapshotInfoDto {
                id: "2026-05-15T140312Z-set_grub_value".to_string(),
                op: "set_grub_value".to_string(),
                ts: "2026-05-15T14:03:12Z".to_string(),
                audit_job_id: "1a2b3c4d-mock-set-grub-value-001".to_string(),
            },
            SnapshotInfoDto {
                id: "2026-05-14T091805Z-set_grub_value".to_string(),
                op: "set_grub_value".to_string(),
                ts: "2026-05-14T09:18:05Z".to_string(),
                audit_job_id: "1a2b3c4d-mock-set-grub-value-002".to_string(),
            },
            SnapshotInfoDto {
                id: "2026-05-12T203044Z-rewrite_grub".to_string(),
                op: "rewrite_grub".to_string(),
                ts: "2026-05-12T20:30:44Z".to_string(),
                audit_job_id: "1a2b3c4d-mock-rewrite-grub-001".to_string(),
            },
        ])
    }

    async fn restore_snapshot(&self, _id: &str) -> zbus::Result<()> {
        Ok(())
    }

    async fn list_efi_boot_entries(&self) -> zbus::Result<String> {
        // Plausible-looking firmware list for Demo Mode: two real OSes
        // (Linux + Windows) and one inactive UEFI shell entry.
        let dtos = vec![
            EfiBootEntryDto {
                index: 0,
                active: true,
                hidden: false,
                description: "ubuntu".to_string(),
            },
            EfiBootEntryDto {
                index: 1,
                active: true,
                hidden: false,
                description: "Windows Boot Manager".to_string(),
            },
            EfiBootEntryDto {
                index: 2,
                active: false,
                hidden: true,
                description: "UEFI Shell".to_string(),
            },
        ];
        Ok(serde_json::to_string(&dtos).unwrap())
    }

    async fn get_boot_order(&self) -> zbus::Result<Vec<u16>> {
        Ok(vec![0u16, 1u16, 2u16])
    }

    async fn set_boot_order(&self, _new_order: Vec<u16>) -> zbus::Result<()> {
        Ok(())
    }

    async fn get_boot_next(&self) -> zbus::Result<i32> {
        // Default: BootNext not set.
        Ok(-1)
    }

    async fn set_boot_next(&self, _index: u16) -> zbus::Result<()> {
        Ok(())
    }

    async fn clear_boot_next(&self) -> zbus::Result<()> {
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Connection helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Connect to the appropriate D-Bus bus based on environment or platform.
///
/// Picks the **session bus** when `BOOTCONTROL_BUS=session` is set
/// (used by E2E tests and the `dbus-run-session` CI helper), otherwise
/// connects to the **system bus** (production layout where `bootcontrold`
/// listens on the system D-Bus).
///
/// # Errors
///
/// Surfaces the underlying [`zbus::Error`] from the bus connection attempt
/// — typically `Connection::system().await` failures (bus not running,
/// permission denied, missing address env var).
///
/// # Examples
///
/// ```no_run
/// # use bootcontrol_client::connect_bus;
/// # async fn run() -> zbus::Result<()> {
/// // Real usage requires a live D-Bus session; flagged `no_run` so doctests
/// // still compile and link inside `cargo test --doc` on hosts without one.
/// let conn = connect_bus().await?;
/// drop(conn);
/// # Ok(())
/// # }
/// ```
pub async fn connect_bus() -> zbus::Result<Connection> {
    match std::env::var("BOOTCONTROL_BUS").as_deref() {
        Ok("session") => Connection::session().await,
        _ => Connection::system().await,
    }
}

/// Resolve the correct [`BootBackend`] implementation for the current host.
///
/// Decision matrix:
///
/// | `BOOTCONTROL_DEMO` set | host OS  | result            |
/// |------------------------|----------|-------------------|
/// | yes                    | any      | [`MockBackend`]   |
/// | no                     | non-Linux| [`MockBackend`]   |
/// | no                     | Linux    | [`DbusBackend`] if bus connection succeeds, else [`MockBackend`] (graceful fallback for macOS dev / no-daemon environments) |
///
/// Never panics, never returns `Result` — the worst case is a silent
/// downgrade to `MockBackend` so frontends always have *something* to
/// render. The fallback path on `connect_bus` failure is intentional
/// (Demo Mode behaviour on Linux without the daemon installed).
///
/// # Examples
///
/// ```no_run
/// # use bootcontrol_client::resolve_backend;
/// # async fn run() {
/// // `BOOTCONTROL_DEMO=1` forces MockBackend regardless of platform.
/// std::env::set_var("BOOTCONTROL_DEMO", "1");
/// let backend = resolve_backend().await;
/// // Use `backend.read_config().await` like any other BootBackend.
/// let _ = backend;
/// # }
/// ```
pub async fn resolve_backend() -> std::sync::Arc<dyn BootBackend> {
    let is_demo = std::env::var("BOOTCONTROL_DEMO").is_ok();
    let target_os = std::env::consts::OS;

    if is_demo || target_os != "linux" {
        std::sync::Arc::new(MockBackend)
    } else {
        match connect_bus().await {
            Ok(conn) => std::sync::Arc::new(DbusBackend::new(conn)),
            Err(_) => std::sync::Arc::new(MockBackend),
        }
    }
}

/// Extract a human-readable string from a [`zbus::Error`].
///
/// For [`zbus::Error::MethodError`] this strips the
/// `org.bootcontrol.Error.` namespace prefix so the UI shows the short
/// variant name (`StateMismatch` instead of
/// `org.bootcontrol.Error.StateMismatch`) and appends the detail string
/// when it is non-empty. For any other variant of `zbus::Error` it falls
/// back to the default `Display` impl.
///
/// This is the **only** place frontends should turn a `zbus::Error` into a
/// user-facing string — the daemon contract is that the error name is the
/// machine-readable part and the detail is human-readable, never the other
/// way around (see `ARCHITECTURE.md` §II "D-Bus Error Convention").
///
/// # Examples
///
/// ```
/// use bootcontrol_client::dbus_error_message;
///
/// // Plain zbus errors round-trip through their Display impl.
/// let err = zbus::Error::Address("not a valid bus".into());
/// assert!(dbus_error_message(&err).contains("not a valid bus"));
/// ```
pub fn dbus_error_message(e: &zbus::Error) -> String {
    if let zbus::Error::MethodError(name, detail, _) = e {
        let short_name = name
            .strip_prefix("org.bootcontrol.Error.")
            .unwrap_or(name.as_str());
        return match detail {
            Some(d) if !d.is_empty() => format!("{short_name}: {d}"),
            _ => short_name.to_string(),
        };
    }
    e.to_string()
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── MockBackend::read_config ───────────────────────────────────────────────

    #[tokio::test]
    async fn mock_backend_read_config_returns_expected_entries() {
        let backend = MockBackend;
        let (map, _etag) = backend
            .read_config()
            .await
            .expect("read_config should succeed");
        assert_eq!(map.get("GRUB_TIMEOUT").map(String::as_str), Some("5"));
        assert_eq!(map.get("GRUB_DEFAULT").map(String::as_str), Some("0"));
        assert_eq!(
            map.get("GRUB_CMDLINE_LINUX_DEFAULT").map(String::as_str),
            Some("quiet splash")
        );
    }

    #[tokio::test]
    async fn mock_backend_read_config_etag_is_non_empty() {
        let backend = MockBackend;
        let (_map, etag) = backend
            .read_config()
            .await
            .expect("read_config should succeed");
        assert!(!etag.is_empty(), "ETag must not be empty");
    }

    // ── MockBackend::set_value ─────────────────────────────────────────────────

    #[tokio::test]
    async fn mock_backend_set_value_always_succeeds() {
        let backend = MockBackend;
        let result = backend
            .set_value("GRUB_TIMEOUT", "10", "mock-etag-12345")
            .await;
        assert!(
            result.is_ok(),
            "MockBackend::set_value must always return Ok"
        );
    }

    // ── MockBackend::get_active_backend ───────────────────────────────────────

    #[tokio::test]
    async fn mock_backend_active_backend_contains_grub() {
        let backend = MockBackend;
        let name = backend
            .get_active_backend()
            .await
            .expect("get_active_backend should succeed");
        assert!(
            name.contains("grub"),
            "active backend name should contain 'grub', got: {name}"
        );
    }

    // ── MockBackend::rebuild_grub_config ──────────────────────────────────────

    #[tokio::test]
    async fn mock_backend_rebuild_grub_config_succeeds() {
        let backend = MockBackend;
        assert!(backend.rebuild_grub_config().await.is_ok());
    }

    // ── MockBackend::backup_nvram ─────────────────────────────────────────────

    #[tokio::test]
    async fn mock_backend_backup_nvram_returns_json_array_string() {
        let backend = MockBackend;
        let json = backend
            .backup_nvram("")
            .await
            .expect("backup_nvram should succeed");
        let trimmed = json.trim();
        assert!(
            trimmed.starts_with('['),
            "must be a JSON array, got: {json}"
        );
        assert!(trimmed.ends_with(']'), "must be a JSON array, got: {json}");
        assert!(!json.is_empty(), "JSON string must not be empty");
    }

    // ── MockBackend::sign_and_enroll_uki ─────────────────────────────────────

    #[tokio::test]
    async fn mock_backend_sign_and_enroll_uki_succeeds() {
        let backend = MockBackend;
        assert!(backend
            .sign_and_enroll_uki("/boot/efi/EFI/linux.efi")
            .await
            .is_ok());
    }

    // ── MockBackend::list_loader_entries ──────────────────────────────────────

    #[tokio::test]
    async fn mock_backend_list_loader_entries_returns_two_entries() {
        let backend = MockBackend;
        let entries = backend
            .list_loader_entries()
            .await
            .expect("list_loader_entries should succeed");
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().any(|e| e.id == "arch" && e.is_default));
        assert!(entries
            .iter()
            .any(|e| e.id == "arch-fallback" && !e.is_default));
    }

    // ── MockBackend::read_kernel_cmdline ──────────────────────────────────────

    #[tokio::test]
    async fn mock_backend_read_kernel_cmdline_returns_params_and_etag() {
        let backend = MockBackend;
        let (params, etag) = backend
            .read_kernel_cmdline()
            .await
            .expect("read_kernel_cmdline should succeed");
        assert!(!params.is_empty(), "params must not be empty");
        assert!(!etag.is_empty(), "etag must not be empty");
        assert!(params.contains(&"quiet".to_string()));
    }

    // ── MockBackend::add/remove_kernel_param ──────────────────────────────────

    #[tokio::test]
    async fn mock_backend_add_kernel_param_succeeds() {
        let backend = MockBackend;
        assert!(backend
            .add_kernel_param("loglevel=3", "mock-etag")
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn mock_backend_remove_kernel_param_succeeds() {
        let backend = MockBackend;
        assert!(backend
            .remove_kernel_param("quiet", "mock-etag")
            .await
            .is_ok());
    }

    // ── MockBackend snapshot operations ───────────────────────────────────────

    #[tokio::test]
    async fn mock_backend_list_snapshots_returns_at_least_one_entry() {
        let backend = MockBackend;
        let snaps = backend
            .list_snapshots()
            .await
            .expect("list_snapshots should succeed");
        assert!(
            !snaps.is_empty(),
            "MockBackend::list_snapshots must return at least one row so Demo Mode shows data"
        );
    }

    #[tokio::test]
    async fn mock_backend_list_snapshots_dto_has_required_fields() {
        let backend = MockBackend;
        let snaps = backend
            .list_snapshots()
            .await
            .expect("list_snapshots should succeed");
        let first = &snaps[0];
        assert!(!first.id.is_empty(), "id must not be empty");
        assert!(!first.op.is_empty(), "op must not be empty");
        assert!(!first.ts.is_empty(), "ts must not be empty");
    }

    #[tokio::test]
    async fn mock_backend_restore_snapshot_succeeds_for_any_id() {
        let backend = MockBackend;
        assert!(backend.restore_snapshot("any-id").await.is_ok());
    }

    #[test]
    fn snapshot_info_dto_roundtrips_through_json() {
        // Daemon serializes Vec<SnapshotInfoDto>, client deserializes the same;
        // pin the JSON shape so a future field rename can't silently drift.
        let dto = SnapshotInfoDto {
            id: "2026-05-15T140312Z-set_grub_value".to_string(),
            op: "set_grub_value".to_string(),
            ts: "2026-05-15T14:03:12Z".to_string(),
            audit_job_id: "1a2b3c4d-job-id".to_string(),
        };
        let json = serde_json::to_string(std::slice::from_ref(&dto)).unwrap();
        assert!(json.contains("\"id\":"));
        assert!(json.contains("\"op\":"));
        assert!(json.contains("\"ts\":"));
        assert!(json.contains("\"audit_job_id\":"));
        let back: Vec<SnapshotInfoDto> = serde_json::from_str(&json).unwrap();
        assert_eq!(back, vec![dto]);
    }

    #[test]
    fn snapshot_info_dto_deserialises_without_audit_job_id() {
        // Snapshots created before the audit-link field was introduced must
        // still deserialise — the missing field defaults to empty.
        let json = r#"[{"id":"old-snap","op":"set_grub_value","ts":"2026-04-01T00:00:00Z"}]"#;
        let parsed: Vec<SnapshotInfoDto> = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].audit_job_id, "");
    }

    // ── dbus_error_message ────────────────────────────────────────────────────

    #[test]
    fn error_prefix_stripping_removes_org_bootcontrol_error_prefix() {
        let full_name = "org.bootcontrol.Error.StateMismatch";
        let stripped = full_name
            .strip_prefix("org.bootcontrol.Error.")
            .unwrap_or(full_name);
        assert_eq!(stripped, "StateMismatch");
    }

    #[test]
    fn error_prefix_stripping_handles_key_not_found() {
        let full_name = "org.bootcontrol.Error.KeyNotFound";
        let stripped = full_name
            .strip_prefix("org.bootcontrol.Error.")
            .unwrap_or(full_name);
        assert_eq!(stripped, "KeyNotFound");
    }

    #[test]
    fn error_prefix_stripping_does_not_strip_other_namespaces() {
        let full_name = "com.other.vendor.Error.Something";
        let stripped = full_name
            .strip_prefix("org.bootcontrol.Error.")
            .unwrap_or(full_name);
        assert_eq!(stripped, full_name);
    }

    #[test]
    fn error_prefix_stripping_handles_polkit_denied() {
        let full_name = "org.bootcontrol.Error.PolkitDenied";
        let stripped = full_name
            .strip_prefix("org.bootcontrol.Error.")
            .unwrap_or(full_name);
        assert_eq!(stripped, "PolkitDenied");
    }
}
