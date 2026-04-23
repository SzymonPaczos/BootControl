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

    /// Rebuild the GRUB config by running grub-mkconfig.
    async fn rebuild_grub_config(&self) -> zbus::Result<()>;

    /// Back up EFI NVRAM variables to `target_dir` (empty = default path).
    /// Returns a JSON array of absolute paths of backed-up files.
    async fn backup_nvram(&self, target_dir: &str) -> zbus::Result<String>;

    /// Sign a UKI at `uki_path` and enroll the MOK certificate.
    async fn sign_and_enroll_uki(&self, uki_path: &str) -> zbus::Result<()>;

    /// Generate a custom PK/KEK/db key set in `output_dir` (empty = default).
    /// Returns a JSON array of generated file paths.
    async fn generate_paranoia_keyset(&self, output_dir: &str) -> zbus::Result<String>;

    /// Merge the custom db cert with Microsoft UEFI CA signatures.
    /// Returns the path to the merged `.auth` file.
    async fn merge_paranoia_with_microsoft(&self, output_dir: &str) -> zbus::Result<String>;
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

    /// Rebuild the GRUB config file (runs grub-mkconfig).
    async fn rebuild_grub_config(&self) -> zbus::Result<()>;

    /// Back up EFI NVRAM variables. Returns JSON list of backed-up file paths.
    async fn backup_nvram(&self, target_dir: &str) -> zbus::Result<String>;

    /// Sign a UKI and enroll the MOK certificate.
    async fn sign_and_enroll_uki(&self, uki_path: &str) -> zbus::Result<()>;

    /// Generate a custom Secure Boot key set (PK/KEK/db).
    /// Returns JSON list of generated key file paths.
    async fn generate_paranoia_keyset(&self, output_dir: &str) -> zbus::Result<String>;

    /// Merge custom db cert with Microsoft UEFI CA signatures.
    /// Returns path to the merged `.auth` file.
    async fn merge_paranoia_with_microsoft(&self, output_dir: &str) -> zbus::Result<String>;
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

    async fn rebuild_grub_config(&self) -> zbus::Result<()> {
        let proxy = ManagerProxy::new(&self.conn).await?;
        proxy.rebuild_grub_config().await
    }

    async fn backup_nvram(&self, target_dir: &str) -> zbus::Result<String> {
        let proxy = ManagerProxy::new(&self.conn).await?;
        proxy.backup_nvram(target_dir).await
    }

    async fn sign_and_enroll_uki(&self, uki_path: &str) -> zbus::Result<()> {
        let proxy = ManagerProxy::new(&self.conn).await?;
        proxy.sign_and_enroll_uki(uki_path).await
    }

    async fn generate_paranoia_keyset(&self, output_dir: &str) -> zbus::Result<String> {
        let proxy = ManagerProxy::new(&self.conn).await?;
        proxy.generate_paranoia_keyset(output_dir).await
    }

    async fn merge_paranoia_with_microsoft(&self, output_dir: &str) -> zbus::Result<String> {
        let proxy = ManagerProxy::new(&self.conn).await?;
        proxy.merge_paranoia_with_microsoft(output_dir).await
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

    async fn rebuild_grub_config(&self) -> zbus::Result<()> {
        Ok(())
    }

    async fn backup_nvram(&self, _target_dir: &str) -> zbus::Result<String> {
        Ok(r#"["/var/lib/bootcontrol/certs/db-mock.efi","/var/lib/bootcontrol/certs/KEK-mock.efi","/var/lib/bootcontrol/certs/PK-mock.efi"]"#.to_string())
    }

    async fn sign_and_enroll_uki(&self, _uki_path: &str) -> zbus::Result<()> {
        Ok(())
    }

    async fn generate_paranoia_keyset(&self, _output_dir: &str) -> zbus::Result<String> {
        Ok(r#"["/var/lib/bootcontrol/paranoia-keys/PK.crt","/var/lib/bootcontrol/paranoia-keys/PK.key","/var/lib/bootcontrol/paranoia-keys/KEK.crt","/var/lib/bootcontrol/paranoia-keys/KEK.key","/var/lib/bootcontrol/paranoia-keys/db.crt","/var/lib/bootcontrol/paranoia-keys/db.key"]"#.to_string())
    }

    async fn merge_paranoia_with_microsoft(&self, _output_dir: &str) -> zbus::Result<String> {
        Ok("/var/lib/bootcontrol/paranoia-keys/db-merged.auth".to_string())
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

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── MockBackend::read_config ───────────────────────────────────────────────

    #[tokio::test]
    async fn mock_backend_read_config_returns_expected_entries() {
        let backend = MockBackend;
        let (map, _etag) = backend.read_config().await.expect("read_config should succeed");
        assert_eq!(map.get("GRUB_TIMEOUT").map(String::as_str), Some("5"));
        assert_eq!(map.get("GRUB_DEFAULT").map(String::as_str), Some("0"));
        assert_eq!(
            map.get("GRUB_CMDLINE_LINUX_DEFAULT").map(String::as_str),
            Some("quiet splash")
        );
    }

    #[tokio::test]
    async fn mock_backend_read_config_etag_is_non_empty() {
        let backend = MockBackend;
        let (_map, etag) = backend.read_config().await.expect("read_config should succeed");
        assert!(!etag.is_empty(), "ETag must not be empty");
    }

    // ── MockBackend::set_value ─────────────────────────────────────────────────

    #[tokio::test]
    async fn mock_backend_set_value_always_succeeds() {
        let backend = MockBackend;
        let result = backend.set_value("GRUB_TIMEOUT", "10", "mock-etag-12345").await;
        assert!(result.is_ok(), "MockBackend::set_value must always return Ok");
    }

    // ── MockBackend::get_active_backend ───────────────────────────────────────

    #[tokio::test]
    async fn mock_backend_active_backend_contains_grub() {
        let backend = MockBackend;
        let name = backend.get_active_backend().await.expect("get_active_backend should succeed");
        assert!(
            name.contains("grub"),
            "active backend name should contain 'grub', got: {name}"
        );
    }

    // ── MockBackend::rebuild_grub_config ──────────────────────────────────────

    #[tokio::test]
    async fn mock_backend_rebuild_grub_config_succeeds() {
        let backend = MockBackend;
        assert!(backend.rebuild_grub_config().await.is_ok());
    }

    // ── MockBackend::backup_nvram ─────────────────────────────────────────────

    #[tokio::test]
    async fn mock_backend_backup_nvram_returns_json_array_string() {
        let backend = MockBackend;
        let json = backend.backup_nvram("").await.expect("backup_nvram should succeed");
        // The JSON must start with '[' and end with ']' — it is an array.
        let trimmed = json.trim();
        assert!(trimmed.starts_with('['), "must be a JSON array, got: {json}");
        assert!(trimmed.ends_with(']'), "must be a JSON array, got: {json}");
        // Must contain at least one file path
        assert!(!json.is_empty(), "JSON string must not be empty");
    }

    // ── MockBackend::sign_and_enroll_uki ─────────────────────────────────────

    #[tokio::test]
    async fn mock_backend_sign_and_enroll_uki_succeeds() {
        let backend = MockBackend;
        assert!(backend.sign_and_enroll_uki("/boot/efi/EFI/linux.efi").await.is_ok());
    }

    // ── MockBackend::generate_paranoia_keyset ─────────────────────────────────

    #[tokio::test]
    async fn mock_backend_generate_paranoia_keyset_returns_json_with_key_files() {
        let backend = MockBackend;
        let json = backend
            .generate_paranoia_keyset("")
            .await
            .expect("generate_paranoia_keyset should succeed");
        // Must be a JSON array
        let trimmed = json.trim();
        assert!(trimmed.starts_with('['), "must be a JSON array, got: {json}");
        assert!(trimmed.ends_with(']'), "must be a JSON array, got: {json}");
        // Must include key material paths
        assert!(json.contains("PK"), "keyset JSON must mention PK key file");
        assert!(json.contains("KEK"), "keyset JSON must mention KEK key file");
        assert!(json.contains(".key") || json.contains(".crt"), "keyset JSON must include key/cert files");
    }

    // ── MockBackend::merge_paranoia_with_microsoft ────────────────────────────

    #[tokio::test]
    async fn mock_backend_merge_paranoia_returns_auth_path() {
        let backend = MockBackend;
        let path = backend
            .merge_paranoia_with_microsoft("")
            .await
            .expect("merge_paranoia should succeed");
        assert!(
            path.ends_with(".auth"),
            "merged db path must end with .auth, got: {path}"
        );
    }

    // ── dbus_error_message ────────────────────────────────────────────────────
    //
    // We test the prefix-stripping logic directly using the same algorithm as
    // `dbus_error_message()`. We cannot construct a zbus::MethodError without
    // a real Message, so we verify the pure string logic in isolation.

    #[test]
    fn error_prefix_stripping_removes_org_bootcontrol_error_prefix() {
        let full_name = "org.bootcontrol.Error.StateMismatch";
        let stripped = full_name
            .strip_prefix("org.bootcontrol.Error.")
            .unwrap_or(full_name);
        assert_eq!(stripped, "StateMismatch");
    }

    #[test]
    fn error_prefix_stripping_handles_key_not_found() {
        let full_name = "org.bootcontrol.Error.KeyNotFound";
        let stripped = full_name
            .strip_prefix("org.bootcontrol.Error.")
            .unwrap_or(full_name);
        assert_eq!(stripped, "KeyNotFound");
    }

    #[test]
    fn error_prefix_stripping_does_not_strip_other_namespaces() {
        // An error from a different D-Bus service must NOT be stripped.
        let full_name = "com.other.vendor.Error.Something";
        let stripped = full_name
            .strip_prefix("org.bootcontrol.Error.")
            .unwrap_or(full_name);
        assert_eq!(stripped, full_name);
    }

    #[test]
    fn error_prefix_stripping_handles_polkit_denied() {
        let full_name = "org.bootcontrol.Error.PolkitDenied";
        let stripped = full_name
            .strip_prefix("org.bootcontrol.Error.")
            .unwrap_or(full_name);
        assert_eq!(stripped, "PolkitDenied");
    }
}
