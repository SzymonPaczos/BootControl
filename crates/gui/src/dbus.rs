use std::collections::HashMap;
use zbus::{proxy, Connection};

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
}

/// Connect to the appropriate D-Bus bus based on the `BOOTCONTROL_BUS` environment variable.
/// Used to connect to the session bus during E2E/smoke testing.
pub async fn connect_bus() -> zbus::Result<Connection> {
    match std::env::var("BOOTCONTROL_BUS").as_deref() {
        Ok("session") => Connection::session().await,
        _ => Connection::system().await,
    }
}
