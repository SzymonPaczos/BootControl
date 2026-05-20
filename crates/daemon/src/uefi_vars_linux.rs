//! Linux backend for the [`UefiVarReader`] trait — reads
//! `/sys/firmware/efi/efivars/`.
//!
//! Every variable in this directory is exposed as a file named
//! `<Name>-<vendor-guid>`. The file content is:
//!
//! ```text
//! offset  size   field
//! 0       4      Attributes (u32 LE)
//! 4       N      Raw value bytes
//! ```
//!
//! The 4-byte prefix is the same `EfiAttributes` bitfield exposed by the
//! pure layer; this module strips it before handing the payload to the
//! parser so callers don't need to know about the on-disk shape.
//!
//! # Override hook
//!
//! `BOOTCONTROL_EFIVARS_DIR` redirects the lookup to a fixture directory.
//! Used by unit tests below and is available for E2E suites that want to
//! exercise the boot-entry code path without a real EFI system.

#![deny(warnings)]
#![deny(missing_docs)]

use std::path::PathBuf;

use bootcontrol_core::{
    error::BootControlError,
    uefi_vars::{UefiVarReader, EFI_GLOBAL_VARIABLE_GUID},
};

/// Production efivarfs mount point.
const EFIVARS_DIR: &str = "/sys/firmware/efi/efivars";

/// The Linux-side `UefiVarReader`. Cheap to construct — holds no FDs or
/// kernel handles, every read is a fresh `read(2)`.
#[derive(Debug, Clone, Default)]
pub struct EfivarFsReader {
    /// Override path; `None` means use the production [`EFIVARS_DIR`] or
    /// honour `BOOTCONTROL_EFIVARS_DIR`.
    root: Option<PathBuf>,
}

impl EfivarFsReader {
    /// Construct a reader that uses the production efivarfs mount.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct a reader pinned to a specific directory. Intended for tests
    /// that pass `tempfile::TempDir::path()`.
    pub fn with_root(root: PathBuf) -> Self {
        Self { root: Some(root) }
    }

    /// Resolve the active efivars root, in priority order:
    ///   1. explicit `with_root` argument
    ///   2. `BOOTCONTROL_EFIVARS_DIR` env var
    ///   3. production `/sys/firmware/efi/efivars`
    fn root(&self) -> PathBuf {
        if let Some(ref p) = self.root {
            return p.clone();
        }
        if let Ok(env) = std::env::var("BOOTCONTROL_EFIVARS_DIR") {
            if !env.is_empty() {
                return PathBuf::from(env);
            }
        }
        PathBuf::from(EFIVARS_DIR)
    }
}

impl UefiVarReader for EfivarFsReader {
    fn read_global(&self, name: &str) -> Result<Vec<u8>, BootControlError> {
        let path = self
            .root()
            .join(format!("{name}-{EFI_GLOBAL_VARIABLE_GUID}"));
        let raw = std::fs::read(&path).map_err(|e| BootControlError::EspScanFailed {
            reason: format!("read {}: {e}", path.display()),
        })?;
        if raw.len() < 4 {
            return Err(BootControlError::EspScanFailed {
                reason: format!(
                    "{}: efivarfs entry shorter than 4-byte attribute header ({} bytes)",
                    path.display(),
                    raw.len()
                ),
            });
        }
        // Strip the 4-byte attribute prefix; the trait contract says callers
        // get the payload only.
        Ok(raw[4..].to_vec())
    }

    fn list_global_names(&self) -> Result<Vec<String>, BootControlError> {
        let suffix = format!("-{EFI_GLOBAL_VARIABLE_GUID}");
        let entries =
            std::fs::read_dir(self.root()).map_err(|e| BootControlError::EspScanFailed {
                reason: format!("list {}: {e}", self.root().display()),
            })?;

        let mut names = Vec::new();
        for entry in entries.flatten() {
            if let Some(file_name) = entry.file_name().to_str() {
                if let Some(name) = file_name.strip_suffix(&suffix) {
                    names.push(name.to_string());
                }
            }
        }
        Ok(names)
    }
}

/// Resolve all `Boot####` entries currently present in the global namespace.
///
/// Convenience helper that combines `list_global_names` + per-name reads +
/// [`bootcontrol_core::uefi_vars::parse_boot_entry`]. Skips entries whose
/// names don't match the `Boot####` shape (e.g. `Boot0000Backup`).
///
/// # Errors
///
/// - [`BootControlError::EspScanFailed`] — efivarfs unreachable or an
///   individual entry could not be read.
/// - [`BootControlError::MalformedValue`] — a `Boot####` payload is shorter
///   than the load-option header.
pub fn list_boot_entries(
    reader: &dyn UefiVarReader,
) -> Result<Vec<bootcontrol_core::uefi_vars::EfiBootEntry>, BootControlError> {
    use bootcontrol_core::uefi_vars::parse_boot_entry;

    let names = reader.list_global_names()?;
    let mut entries = Vec::new();
    for name in names {
        // "Boot" + 4 uppercase hex digits exactly.
        if name.len() != 8 || !name.starts_with("Boot") {
            continue;
        }
        let hex = &name[4..];
        let Ok(index) = u16::from_str_radix(hex, 16) else {
            continue;
        };
        // Some firmwares expose `BootOptionSupport` / `BootCurrent` /
        // `BootOrder` / `BootNext` — all 4 hex digits would only match the
        // per-entry `Boot####` form anyway because those four are not 4-hex-
        // digit suffixes after "Boot". Skip them explicitly for clarity.
        if matches!(name.as_str(), "BootNext" | "BootCurrent" | "BootOrder") {
            continue;
        }
        let payload = reader.read_global(&name)?;
        entries.push(parse_boot_entry(index, &payload)?);
    }
    entries.sort_by_key(|e| e.index);
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bootcontrol_core::uefi_vars::{encode_boot_order, parse_boot_order};
    use std::path::Path;

    /// Write a fake efivarfs entry: 4-byte attribute prefix + payload.
    fn write_entry(root: &Path, name: &str, attrs: u32, payload: &[u8]) {
        let path = root.join(format!("{name}-{EFI_GLOBAL_VARIABLE_GUID}"));
        let mut bytes = Vec::with_capacity(4 + payload.len());
        bytes.extend_from_slice(&attrs.to_le_bytes());
        bytes.extend_from_slice(payload);
        std::fs::write(&path, &bytes).unwrap();
    }

    fn build_boot_entry_payload(attrs: u32, description: &str) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&attrs.to_le_bytes());
        payload.extend_from_slice(&0u16.to_le_bytes()); // FilePathListLength
        for ch in description.encode_utf16() {
            payload.extend_from_slice(&ch.to_le_bytes());
        }
        payload.extend_from_slice(&0u16.to_le_bytes()); // null terminator
        payload
    }

    #[test]
    fn read_global_strips_attribute_prefix() {
        let dir = tempfile::TempDir::new().unwrap();
        write_entry(dir.path(), "BootOrder", 0x07, &encode_boot_order(&[1, 0]));

        let reader = EfivarFsReader::with_root(dir.path().to_path_buf());
        let payload = reader.read_global("BootOrder").unwrap();
        assert_eq!(parse_boot_order(&payload).unwrap(), vec![1u16, 0u16]);
    }

    #[test]
    fn read_global_rejects_truncated_entry() {
        let dir = tempfile::TempDir::new().unwrap();
        // Truncated below the 4-byte attribute header.
        std::fs::write(
            dir.path()
                .join(format!("Boot0000-{EFI_GLOBAL_VARIABLE_GUID}")),
            [0x01, 0x02, 0x03],
        )
        .unwrap();

        let reader = EfivarFsReader::with_root(dir.path().to_path_buf());
        let result = reader.read_global("Boot0000");
        assert!(matches!(
            result,
            Err(BootControlError::EspScanFailed { .. })
        ));
    }

    #[test]
    fn list_global_names_filters_by_vendor_guid() {
        let dir = tempfile::TempDir::new().unwrap();
        write_entry(dir.path(), "BootOrder", 0x07, &[]);
        write_entry(dir.path(), "BootNext", 0x07, &[]);
        // A file in a different vendor namespace should be ignored.
        std::fs::write(
            dir.path().join("PK-deadbeef-1234-1234-1234-deadbeefdead"),
            b"",
        )
        .unwrap();

        let reader = EfivarFsReader::with_root(dir.path().to_path_buf());
        let mut names = reader.list_global_names().unwrap();
        names.sort();
        assert_eq!(names, vec!["BootNext".to_string(), "BootOrder".to_string()]);
    }

    #[test]
    fn list_boot_entries_returns_sorted_active_entries() {
        let dir = tempfile::TempDir::new().unwrap();
        // Three entries: 0x0001 active, 0x0002 inactive, 0x0000 active.
        write_entry(
            dir.path(),
            "Boot0001",
            0x07,
            &build_boot_entry_payload(0x01, "ubuntu"),
        );
        write_entry(
            dir.path(),
            "Boot0002",
            0x07,
            &build_boot_entry_payload(0x00, "fedora"),
        );
        write_entry(
            dir.path(),
            "Boot0000",
            0x07,
            &build_boot_entry_payload(0x01, "Windows Boot Manager"),
        );

        let reader = EfivarFsReader::with_root(dir.path().to_path_buf());
        let entries = list_boot_entries(&reader).unwrap();
        let descriptions: Vec<_> = entries.iter().map(|e| e.description.as_str()).collect();
        assert_eq!(
            descriptions,
            vec!["Windows Boot Manager", "ubuntu", "fedora"]
        );
        assert_eq!(entries[0].index, 0);
        assert_eq!(entries[1].index, 1);
        assert_eq!(entries[2].index, 2);
        assert!(entries[0].active);
        assert!(entries[1].active);
        assert!(!entries[2].active);
    }

    #[test]
    fn list_boot_entries_skips_non_boot_entries() {
        let dir = tempfile::TempDir::new().unwrap();
        // `BootOrder` is also in the global namespace but is not a per-entry
        // `Boot####` — it must be filtered out so we don't try to parse it
        // as a load option.
        write_entry(dir.path(), "BootOrder", 0x07, &encode_boot_order(&[0]));
        write_entry(
            dir.path(),
            "Boot0000",
            0x07,
            &build_boot_entry_payload(0x01, "single"),
        );

        let reader = EfivarFsReader::with_root(dir.path().to_path_buf());
        let entries = list_boot_entries(&reader).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].description, "single");
    }
}
