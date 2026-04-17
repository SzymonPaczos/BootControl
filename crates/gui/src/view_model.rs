use std::collections::HashMap;
use zbus::Connection;

use crate::dbus::ManagerProxy;

pub struct ViewModel {
    pub conn: Connection,
    pub entries: HashMap<String, String>,
    pub etag: String,
}

impl ViewModel {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn,
            entries: HashMap::new(),
            etag: String::new(),
        }
    }

    pub async fn load(&mut self) -> zbus::Result<()> {
        let manager = ManagerProxy::new(&self.conn).await?;
        let (config, etag) = manager.read_grub_config().await?;
        self.entries = config;
        self.etag = etag;
        Ok(())
    }

    pub async fn commit_edit(&mut self, key: &str, value: &str) -> zbus::Result<()> {
        let manager = ManagerProxy::new(&self.conn).await?;
        manager.set_grub_value(key, value, &self.etag).await?;
        // ETag usually needs to be re-fetched after commit, but load() handles that.
        Ok(())
    }
}
