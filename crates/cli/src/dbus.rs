use std::collections::HashMap;
use zbus::proxy;

/// D-Bus proxy for the `org.bootcontrol.Manager` interface exposed by `bootcontrold`.
#[proxy(
    interface = "org.bootcontrol.Manager",
    default_service = "org.bootcontrol.Manager",
    default_path = "/org/bootcontrol/Manager"
)]
pub trait Manager {
    /// Read the GRUB configuration.
    ///
    /// Returns a tuple containing a dictionary of GRUB key-value pairs and the current ETag.
    async fn read_grub_config(&self) -> zbus::Result<(HashMap<String, String>, String)>;

    /// Set a GRUB value.
    ///
    /// The user must provide the `key`, the new `value`, and the latest `etag`.
    async fn set_grub_value(&self, key: &str, value: &str, etag: &str) -> zbus::Result<()>;

    /// Get the current ETag.
    async fn get_etag(&self) -> zbus::Result<String>;

    /// Get the name of the active bootloader backend.
    async fn get_active_backend(&self) -> zbus::Result<String>;
}
