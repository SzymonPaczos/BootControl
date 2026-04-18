use std::collections::HashMap;
use crate::dbus::ManagerProxy;
use zbus::Connection;

/// Result type for backend operations, using zbus::Error for compatibility.
pub type BackendResult<T> = Result<T, zbus::Error>;

/// Abstract interface for BootControl operations.
/// This allows us to swap between a real D-Bus connection and mock data (Demo Mode).
#[async_trait::async_trait]
pub trait BootBackend: Send + Sync {
    /// Read the boot configuration (GRUB key-values and ETag).
    async fn read_config(&self) -> BackendResult<(HashMap<String, String>, String)>;

    /// Set a single configuration value.
    async fn set_value(&self, key: &str, value: &str, etag: &str) -> BackendResult<()>;

    /// Return the name of the active backend (e.g., "grub").
    async fn get_active_backend(&self) -> BackendResult<String>;
}

/// Real D-Bus backend that talks to bootcontrold.
pub struct DbusBackend {
    conn: Connection,
}

impl DbusBackend {
    pub fn new(conn: Connection) -> Self {
        Self { conn }
    }
}

#[async_trait::async_trait]
impl BootBackend for DbusBackend {
    async fn read_config(&self) -> BackendResult<(HashMap<String, String>, String)> {
        let proxy = ManagerProxy::new(&self.conn).await?;
        proxy.read_grub_config().await
    }

    async fn set_value(&self, key: &str, value: &str, etag: &str) -> BackendResult<()> {
        let proxy = ManagerProxy::new(&self.conn).await?;
        proxy.set_grub_value(key, value, etag).await
    }

    async fn get_active_backend(&self) -> BackendResult<String> {
        let proxy = ManagerProxy::new(&self.conn).await?;
        proxy.get_active_backend().await
    }
}

/// Mock backend for Demo Mode (works on Mac/any OS without a daemon).
pub struct MockBackend;

#[async_trait::async_trait]
impl BootBackend for MockBackend {
    async fn read_config(&self) -> BackendResult<(HashMap<String, String>, String)> {
        let mut mock = HashMap::new();
        mock.insert("GRUB_TIMEOUT".to_string(), "5".to_string());
        mock.insert("GRUB_DEFAULT".to_string(), "0".to_string());
        mock.insert("GRUB_DISTRIBUTOR".to_string(), "MockOS".to_string());
        mock.insert("GRUB_CMDLINE_LINUX_DEFAULT".to_string(), "quiet splash".to_string());
        mock.insert("GRUB_DISABLE_OS_PROBER".to_string(), "false".to_string());
        
        Ok((mock, "mock-etag-12345".to_string()))
    }

    async fn set_value(&self, _key: &str, _value: &str, _etag: &str) -> BackendResult<()> {
        // Just simulate a successful write.
        Ok(())
    }

    async fn get_active_backend(&self) -> BackendResult<String> {
        Ok("grub (mock)".to_string())
    }
}
