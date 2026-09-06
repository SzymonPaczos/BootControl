use bootcontrol_client::{BootBackend, LoaderEntryDto, SnapshotInfoDto};
use std::collections::HashMap;
use std::sync::Arc;

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
    state_ready: bool,
}

impl ViewModel {
    /// Create a new [`ViewModel`] with the given backend.
    pub fn new(backend: Arc<dyn BootBackend>) -> Self {
        Self {
            backend,
            entries: HashMap::new(),
            etag: String::new(),
            active_backend: String::new(),
            loader_entries: Vec::new(),
            cmdline_params: Vec::new(),
            state_ready: false,
        }
    }

    fn invalidate_loaded_state(&mut self) {
        self.entries.clear();
        self.etag.clear();
        self.active_backend.clear();
        self.loader_entries.clear();
        self.cmdline_params.clear();
        self.state_ready = false;
    }

    fn require_loaded_state(&self) -> Result<(), zbus::Error> {
        if self.state_ready && !self.etag.is_empty() {
            Ok(())
        } else {
            Err(zbus::Error::Failure(
                "view model is not ready for writes; reload live state first".into(),
            ))
        }
    }

    /// Fetch the current boot configuration and backend name.
    ///
    /// Branches on `active_backend` to call the appropriate D-Bus method:
    /// - `"grub"` → `read_config()`
    /// - `"systemd-boot"` → `list_loader_entries()` + `get_loader_conf_etag()`
    /// - `"uki"` → `read_kernel_cmdline()`
    pub async fn load(&mut self) -> Result<(), zbus::Error> {
        self.invalidate_loaded_state();
        let active_backend = self.backend.get_active_backend().await?;
        let mut loader_entries = Vec::new();
        let mut cmdline_params = Vec::new();

        let (entries, etag) = if active_backend.contains("systemd-boot") {
            loader_entries = self.backend.list_loader_entries().await?;
            let etag = self.backend.get_loader_conf_etag().await?;
            let entries = loader_entries
                .iter()
                .map(|e| (e.id.clone(), e.title.clone().unwrap_or_default()))
                .collect();
            (entries, etag)
        } else if active_backend.contains("uki") {
            let (params, etag) = self.backend.read_kernel_cmdline().await?;
            let entries = params
                .iter()
                .cloned()
                .map(|param| (param, String::new()))
                .collect();
            cmdline_params = params;
            (entries, etag)
        } else {
            self.backend.read_config().await?
        };

        self.active_backend = active_backend;
        self.entries = entries;
        self.etag = etag;
        self.loader_entries = loader_entries;
        self.cmdline_params = cmdline_params;
        self.state_ready = true;
        Ok(())
    }

    /// Commit a single key-value edit to the backend.
    ///
    /// Semantics differ by backend:
    /// - GRUB: set `key=value` in `/etc/default/grub`
    /// - systemd-boot: not applicable via this method (use `set_default_entry`)
    /// - UKI: add `value` as a new kernel parameter
    pub async fn commit_edit(&mut self, key: &str, value: &str) -> Result<(), zbus::Error> {
        self.require_loaded_state()?;
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
        self.require_loaded_state()?;
        self.backend.set_loader_default(id, &self.etag).await
    }

    /// Remove a UKI kernel parameter (only meaningful for UKI).
    pub async fn remove_kernel_param(&mut self, param: &str) -> Result<(), zbus::Error> {
        self.require_loaded_state()?;
        self.backend.remove_kernel_param(param, &self.etag).await
    }

    /// Rebuild the GRUB config by running grub-mkconfig on the daemon.
    pub async fn rebuild_grub(&self) -> Result<(), zbus::Error> {
        self.backend.rebuild_grub_config().await
    }

    /// Read parsed GRUB menu entries and the generated configuration ETag.
    ///
    /// # Arguments
    ///
    /// This function takes no arguments.
    ///
    /// # Errors
    ///
    /// Returns the original D-Bus error or a client-side JSON decoding error.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(model: &bootcontrol_gui::view_model::ViewModel) -> zbus::Result<()> {
    /// let (entries, etag) = model.list_grub_entries().await?;
    /// assert!(!etag.is_empty() || entries.is_empty());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_grub_entries(
        &self,
    ) -> Result<(Vec<bootcontrol_client::GrubMenuEntryDto>, String), zbus::Error> {
        self.backend.list_grub_entries().await
    }

    /// Persist a versioned GRUB default-menu selection through the daemon.
    ///
    /// # Arguments
    ///
    /// * `selected_path` - Bootable path returned by the current menu list.
    /// * `menu_etag` - Version of generated `grub.cfg`.
    /// * `config_etag` - Version of `/etc/default/grub`.
    ///
    /// # Errors
    ///
    /// Returns the original structured D-Bus error from authorization,
    /// optimistic-lock validation, snapshotting, writing, or rebuilding.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(model: &bootcontrol_gui::view_model::ViewModel) -> zbus::Result<()> {
    /// model.set_grub_default("1>0", "menu-etag", "config-etag").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_grub_default(
        &self,
        selected_path: &str,
        menu_etag: &str,
        config_etag: &str,
    ) -> Result<(), zbus::Error> {
        self.backend
            .set_grub_default(selected_path, menu_etag, config_etag)
            .await
    }

    /// Back up EFI NVRAM variables. Returns JSON list of backed-up file paths.
    pub async fn backup_nvram(&self) -> Result<String, zbus::Error> {
        self.backend.backup_nvram("").await
    }

    /// Sign a UKI and enroll the MOK certificate.
    pub async fn enroll_mok(&self) -> Result<(), zbus::Error> {
        self.backend.sign_and_enroll_uki("").await
    }

    /// List all snapshots known to the daemon, newest first.
    pub async fn list_snapshots(&self) -> Result<Vec<SnapshotInfoDto>, zbus::Error> {
        self.backend.list_snapshots().await
    }

    /// Restore a snapshot using the primary-target ETag shown before confirmation.
    pub async fn restore_snapshot(&self, id: &str) -> Result<(), zbus::Error> {
        self.require_loaded_state()?;
        self.backend.restore_snapshot(id, &self.etag).await
    }
}
