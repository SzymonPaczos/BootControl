use std::collections::HashMap;
use std::sync::Arc;
use bootcontrol_client::BootBackend;

/// View model bridging the boot backend and the Slint UI layer.
pub struct ViewModel {
    /// Active boot backend (D-Bus or Mock).
    pub backend: Arc<dyn BootBackend>,
    /// Parsed GRUB key-value entries from the last successful load.
    pub entries: HashMap<String, String>,
    /// Current ETag of the GRUB config file on disk.
    pub etag: String,
    /// Name of the active bootloader backend reported by the daemon.
    pub active_backend: String,
}

impl ViewModel {
    /// Create a new [`ViewModel`] with the given backend.
    pub fn new(backend: Arc<dyn BootBackend>) -> Self {
        Self {
            backend,
            entries: HashMap::new(),
            etag: String::new(),
            active_backend: "grub".to_string(),
        }
    }

    /// Fetch the current GRUB config and backend name from the backend.
    pub async fn load(&mut self) -> Result<(), zbus::Error> {
        let (config, etag) = self.backend.read_config().await?;
        self.entries = config;
        self.etag = etag;
        self.active_backend = self.backend
            .get_active_backend()
            .await
            .unwrap_or_else(|_| "grub".to_string());
        Ok(())
    }

    /// Commit a single key-value edit to the backend.
    pub async fn commit_edit(&mut self, key: &str, value: &str) -> Result<(), zbus::Error> {
        self.backend.set_value(key, value, &self.etag).await?;
        Ok(())
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
}
