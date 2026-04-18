use std::collections::HashMap;
use zbus::Connection;

use crate::dbus::ManagerProxy;

/// View model bridging the D-Bus daemon and the Slint UI layer.
pub struct ViewModel {
    /// Active D-Bus connection.
    pub conn: Connection,
    /// Parsed GRUB key-value entries from the last successful load.
    pub entries: HashMap<String, String>,
    /// Current ETag of the GRUB config file on disk.
    pub etag: String,
    /// Name of the active bootloader backend reported by the daemon.
    pub active_backend: String,
}

impl ViewModel {
    /// Create a new [`ViewModel`] with an empty state.
    pub fn new(conn: Connection) -> Self {
        Self {
            conn,
            entries: HashMap::new(),
            etag: String::new(),
            active_backend: "grub".to_string(),
        }
    }

    /// Fetch the current GRUB config and backend name from the daemon.
    pub async fn load(&mut self) -> zbus::Result<()> {
        let manager = ManagerProxy::new(&self.conn).await?;
        let (config, etag) = manager.read_grub_config().await?;
        self.entries = config;
        self.etag = etag;
        self.active_backend = manager
            .get_active_backend()
            .await
            .unwrap_or_else(|_| "grub".to_string());
        Ok(())
    }

    /// Commit a single key-value edit to the daemon.
    pub async fn commit_edit(&mut self, key: &str, value: &str) -> zbus::Result<()> {
        let manager = ManagerProxy::new(&self.conn).await?;
        manager.set_grub_value(key, value, &self.etag).await?;
        // ETag usually needs to be re-fetched after commit, but load() handles that.
        Ok(())
    }
}
