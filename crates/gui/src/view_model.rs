use std::collections::HashMap;
use std::sync::Arc;
use bootcontrol_client::{BootBackend, LoaderEntryDto, SnapshotInfoDto};

/// View model bridging the boot backend and the Slint UI layer.
pub struct ViewModel {
    /// Active boot backend (D-Bus or Mock).
    pub backend: Arc<dyn BootBackend>,
    /// Parsed GRUB key-value entries from the last successful load.
    pub entries: HashMap<String, String>,
    /// Current ETag of the config file on disk.
    pub etag: String,
    /// Name of the active bootloader backend reported by the daemon.
    pub active_backend: String,
    /// systemd-boot loader entries (populated when backend = "systemd-boot").
    pub loader_entries: Vec<LoaderEntryDto>,
    /// UKI kernel cmdline parameters (populated when backend = "uki").
    pub cmdline_params: Vec<String>,
}

impl ViewModel {
    /// Create a new [`ViewModel`] with the given backend.
    pub fn new(backend: Arc<dyn BootBackend>) -> Self {
        Self {
            backend,
            entries: HashMap::new(),
            etag: String::new(),
            active_backend: "grub".to_string(),
            loader_entries: Vec::new(),
            cmdline_params: Vec::new(),
        }
    }

    /// Fetch the current boot configuration and backend name.
    ///
    /// Branches on `active_backend` to call the appropriate D-Bus method:
    /// - `"grub"` → `read_config()`
    /// - `"systemd-boot"` → `list_loader_entries()` + `get_loader_conf_etag()`
    /// - `"uki"` → `read_kernel_cmdline()`
    pub async fn load(&mut self) -> Result<(), zbus::Error> {
        self.active_backend = self.backend
            .get_active_backend()
            .await
            .unwrap_or_else(|_| "grub".to_string());

        if self.active_backend.contains("systemd-boot") {
            self.loader_entries = self.backend.list_loader_entries().await?;
            self.etag = self.backend.get_loader_conf_etag().await.unwrap_or_default();
            // Mirror entries as key-value pairs so existing GUI code works without changes.
            self.entries = self.loader_entries
                .iter()
                .map(|e| (e.id.clone(), e.title.clone().unwrap_or_default()))
                .collect();
        } else if self.active_backend.contains("uki") {
            let (params, etag) = self.backend.read_kernel_cmdline().await?;
            self.cmdline_params = params.clone();
            self.etag = etag;
            // Mirror as key-value (param → empty value) for existing GUI table.
            self.entries = params.into_iter().map(|p| (p, String::new())).collect();
        } else {
            // GRUB — existing path.
            let (config, etag) = self.backend.read_config().await?;
            self.entries = config;
            self.etag = etag;
        }

        Ok(())
    }

    /// Commit a single key-value edit to the backend.
    ///
    /// Semantics differ by backend:
    /// - GRUB: set `key=value` in `/etc/default/grub`
    /// - systemd-boot: not applicable via this method (use `set_default_entry`)
    /// - UKI: add `value` as a new kernel parameter
    pub async fn commit_edit(&mut self, key: &str, value: &str) -> Result<(), zbus::Error> {
        if self.active_backend.contains("uki") {
            // For UKI the "value" field isn't used; the key is the full parameter.
            self.backend.add_kernel_param(key, &self.etag).await?;
        } else {
            self.backend.set_value(key, value, &self.etag).await?;
        }
        Ok(())
    }

    /// Set the default systemd-boot entry (only meaningful for systemd-boot).
    pub async fn set_default_entry(&mut self, id: &str) -> Result<(), zbus::Error> {
        self.backend.set_loader_default(id, &self.etag).await
    }

    /// Remove a UKI kernel parameter (only meaningful for UKI).
    pub async fn remove_kernel_param(&mut self, param: &str) -> Result<(), zbus::Error> {
        self.backend.remove_kernel_param(param, &self.etag).await
    }

    /// Rebuild the GRUB config by running grub-mkconfig on the daemon.
    pub async fn rebuild_grub(&self) -> Result<(), zbus::Error> {
        self.backend.rebuild_grub_config().await
    }

    /// Back up EFI NVRAM variables. Returns JSON list of backed-up file paths.
    pub async fn backup_nvram(&self) -> Result<String, zbus::Error> {
        self.backend.backup_nvram("").await
    }

    /// Sign a UKI and enroll the MOK certificate.
    pub async fn enroll_mok(&self) -> Result<(), zbus::Error> {
        self.backend.sign_and_enroll_uki("").await
    }

    /// Generate a custom Secure Boot key set (PK/KEK/db).
    /// Returns JSON list of generated key file paths.
    pub async fn generate_paranoia(&self) -> Result<String, zbus::Error> {
        self.backend.generate_paranoia_keyset("").await
    }

    /// Merge custom db cert with Microsoft UEFI CA signatures.
    /// Returns path to the merged `.auth` file.
    pub async fn merge_paranoia(&self) -> Result<String, zbus::Error> {
        self.backend.merge_paranoia_with_microsoft("").await
    }

    /// List all snapshots known to the daemon, newest first.
    pub async fn list_snapshots(&self) -> Result<Vec<SnapshotInfoDto>, zbus::Error> {
        self.backend.list_snapshots().await
    }

    /// Restore a snapshot by id. Overwrites all files captured in its manifest.
    pub async fn restore_snapshot(&self, id: &str) -> Result<(), zbus::Error> {
        self.backend.restore_snapshot(id).await
    }
}
