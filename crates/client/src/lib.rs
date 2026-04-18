use std::collections::HashMap;
use async_trait::async_trait;
use zbus::{Connection, proxy};

/// D-Bus proxy for the `org.bootcontrol.Manager` interface exposed by `bootcontrold`.
#[proxy(
    interface = "org.bootcontrol.Manager",
    default_service = "org.bootcontrol.Manager",
    default_path = "/org/bootcontrol/Manager"
)]
pub trait Manager {
    /// Read the GRUB configuration.
    async fn read_grub_config(&self) -> zbus::Result<(HashMap<String, String>, String)>;

    /// Set a GRUB value.
    async fn set_grub_value(&self, key: &str, value: &str, etag: &str) -> zbus::Result<()>;

    /// Get the current ETag.
    async fn get_etag(&self) -> zbus::Result<String>;

    /// Get the name of the active bootloader backend.
    async fn get_active_backend(&self) -> zbus::Result<String>;
}

/// Abstract interface for BootControl operations.
/// This allows us to swap between a real D-Bus connection and mock data (Demo Mode).
#[async_trait]
pub trait BootBackend: Send + Sync {
    /// Read the boot configuration (GRUB key-values and ETag).
    async fn read_config(&self) -> zbus::Result<(HashMap<String, String>, String)>;

    /// Set a single configuration value.
    async fn set_value(&self, key: &str, value: &str, etag: &str) -> zbus::Result<()>;

    /// Return the name of the active backend (e.g., "grub").
    async fn get_active_backend(&self) -> zbus::Result<String>;
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
}

/// Mock backend for Demo Mode (works on Mac/any OS without a daemon).
pub struct MockBackend;

#[async_trait]
impl BootBackend for MockBackend {
    async fn read_config(&self) -> zbus::Result<(HashMap<String, String>, String)> {
        let mut mock = HashMap::new();
        mock.insert("GRUB_TIMEOUT".to_string(), "5".to_string());
        mock.insert("GRUB_DEFAULT".to_string(), "0".to_string());
        mock.insert("GRUB_DISTRIBUTOR".to_string(), "MockOS".to_string());
        mock.insert("GRUB_CMDLINE_LINUX_DEFAULT".to_string(), "quiet splash".to_string());
        mock.insert("GRUB_DISABLE_OS_PROBER".to_string(), "false".to_string());
        
        Ok((mock, "mock-etag-12345".to_string()))
    }

    async fn set_value(&self, _key: &str, _value: &str, _etag: &str) -> zbus::Result<()> {
        Ok(())
    }

    async fn get_active_backend(&self) -> zbus::Result<String> {
        Ok("grub (mock)".to_string())
    }
}

/// Connect to the appropriate D-Bus bus based on environment or platform.
pub async fn connect_bus() -> zbus::Result<Connection> {
    match std::env::var("BOOTCONTROL_BUS").as_deref() {
        Ok("session") => Connection::session().await,
        _ => Connection::system().await,
    }
}

/// Resolve the correct backend based on environment variables and platform.
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
