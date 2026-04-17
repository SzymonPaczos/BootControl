//! D-Bus proxy for the `org.bootcontrol.Manager` interface.
//!
//! This module mirrors the proxy in `crates/cli/src/dbus.rs` verbatim. A
//! dedicated `crates/dbus-client` extraction is planned for a later phase; for
//! now duplication is intentional to keep each crate self-contained.
//!
//! # Usage
//!
//! ```no_run
//! use bootcontrol_tui::dbus::ManagerProxy;
//!
//! # async fn run() -> zbus::Result<()> {
//! let conn  = zbus::Connection::system().await?;
//! let proxy = ManagerProxy::new(&conn).await?;
//! let (cfg, etag) = proxy.read_grub_config().await?;
//! # Ok(())
//! # }
//! ```

use std::collections::HashMap;
use zbus::proxy;

/// Auto-generated async proxy for `org.bootcontrol.Manager`.
///
/// All methods are `async` and return [`zbus::Result`].  The generated struct
/// is named `ManagerProxy`; call [`ManagerProxy::new`] with an active
/// [`zbus::Connection`] to instantiate it.
#[proxy(
    interface = "org.bootcontrol.Manager",
    default_service = "org.bootcontrol.Manager",
    default_path = "/org/bootcontrol/Manager"
)]
pub trait Manager {
    /// Read the current GRUB configuration and its ETag.
    ///
    /// # Returns
    ///
    /// A tuple `(entries, etag)` where `entries` is a hash-map of
    /// `GRUB_KEY → raw_value` (surrounding double-quotes stripped by the
    /// daemon) and `etag` is the 64-character SHA-256 hex digest of the file.
    ///
    /// # Errors
    ///
    /// - `org.bootcontrol.Error.EspScanFailed` — the file could not be read.
    /// - `org.bootcontrol.Error.ComplexBashDetected` — the file contains Bash
    ///   constructs that the daemon cannot safely parse.
    async fn read_grub_config(&self) -> zbus::Result<(HashMap<String, String>, String)>;

    /// Set a single GRUB key-value pair.
    ///
    /// # Arguments
    ///
    /// * `key`   — GRUB variable name, e.g. `"GRUB_TIMEOUT"`.
    /// * `value` — New value without surrounding quotes, e.g. `"10"`.
    /// * `etag`  — The ETag returned by the most recent `read_grub_config` or
    ///             `get_etag` call.  Rejected if stale.
    ///
    /// # Errors
    ///
    /// - `org.bootcontrol.Error.PolkitDenied`
    /// - `org.bootcontrol.Error.SecurityPolicyViolation`
    /// - `org.bootcontrol.Error.StateMismatch`
    /// - `org.bootcontrol.Error.ConcurrentModification`
    /// - `org.bootcontrol.Error.EspScanFailed`
    async fn set_grub_value(&self, key: &str, value: &str, etag: &str) -> zbus::Result<()>;

    /// Return the SHA-256 ETag of the current on-disk GRUB configuration.
    ///
    /// # Errors
    ///
    /// - `org.bootcontrol.Error.EspScanFailed` — the file could not be read.
    async fn get_etag(&self) -> zbus::Result<String>;
}
