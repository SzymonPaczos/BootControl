//! Cross-platform UEFI variable parsing and abstraction.
//!
//! This module owns the **pure** layer of EFI-variable handling: the data
//! types ([`EfiBootEntry`], [`EfiAttributes`]), the parsers
//! ([`parse_boot_order`], [`parse_boot_entry`]) and the abstract
//! [`UefiVarReader`] trait. Concrete backends — Linux's
//! `/sys/firmware/efi/efivars/` and Windows's
//! `GetFirmwareEnvironmentVariable` — live in `crates/daemon` (see Phase 7
//! PR1's `daemon/src/uefi_vars_linux.rs`; the Windows half lands in a
//! follow-up).
//!
//! # UEFI variable layout primer
//!
//! Every EFI variable is identified by a (`name`, `vendor_guid`) pair. The
//! variables relevant to bootloader management all live under the global
//! vendor namespace `8be4df61-93ca-11d2-aa0d-00e098032b8c`:
//!
//! | Variable | Encoding | Meaning |
//! |----------|----------|---------|
//! | `BootOrder` | UTF-16 LE? No — packed `u16` LE array | Order in which EFI tries `Boot#### entries` |
//! | `BootCurrent` | single `u16` LE | Index of the entry actually used this boot |
//! | `BootNext` | single `u16` LE | One-shot override for the next boot (cleared by firmware) |
//! | `Boot####` | EFI_LOAD_OPTION | Per-entry record: attrs + description + device path |
//!
//! Each variable is prefixed with a 4-byte `EfiAttributes` bitfield in the
//! Linux efivarfs view; Windows hides the attributes behind the API.
//!
//! # What this module does NOT do
//!
//! * No I/O. Backends read raw bytes; we parse them.
//! * No write support yet — that's Phase 7 PR2 (BootNext) and PR3 (BootOrder).

#![deny(warnings)]
#![deny(missing_docs)]

use crate::error::BootControlError;

/// Vendor GUID for the EFI Global Variable namespace.
///
/// Used as the second half of the file name in `efivarfs`
/// (`BootOrder-8be4df61-93ca-11d2-aa0d-00e098032b8c`) and as the
/// `lpGuid` argument on Windows's `GetFirmwareEnvironmentVariableExW`.
pub const EFI_GLOBAL_VARIABLE_GUID: &str = "8be4df61-93ca-11d2-aa0d-00e098032b8c";

/// Bitfield of UEFI variable attributes (UEFI 2.10 §8.2).
///
/// Stored as the first 4 bytes of every efivarfs entry on Linux and exposed
/// through `GetFirmwareEnvironmentVariableEx` `pdwAttributes` on Windows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EfiAttributes {
    /// Variable persists across resets (`EFI_VARIABLE_NON_VOLATILE`).
    pub non_volatile: bool,
    /// Variable is accessible from boot services (`EFI_VARIABLE_BOOTSERVICE_ACCESS`).
    pub bootservice_access: bool,
    /// Variable is accessible from runtime services (`EFI_VARIABLE_RUNTIME_ACCESS`).
    pub runtime_access: bool,
    /// Reserved / write-once-protected (`EFI_VARIABLE_HARDWARE_ERROR_RECORD`).
    pub hardware_error_record: bool,
    /// Authenticated write required (deprecated since UEFI 2.5).
    pub authenticated_write_access: bool,
    /// Newer auth-write variant (`EFI_VARIABLE_TIME_BASED_AUTH...`).
    pub time_based_authenticated_write_access: bool,
    /// Append-only writes (`EFI_VARIABLE_APPEND_WRITE`).
    pub append_write: bool,
}

impl EfiAttributes {
    /// Decode the 4-byte little-endian attribute prefix used by efivarfs.
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrol_core::uefi_vars::EfiAttributes;
    /// // Typical boot-variable attrs: NV + BS + RT  = 0x07.
    /// let attrs = EfiAttributes::from_u32(0x07);
    /// assert!(attrs.non_volatile);
    /// assert!(attrs.bootservice_access);
    /// assert!(attrs.runtime_access);
    /// assert!(!attrs.append_write);
    /// ```
    pub fn from_u32(raw: u32) -> Self {
        Self {
            non_volatile: raw & 0x0000_0001 != 0,
            bootservice_access: raw & 0x0000_0002 != 0,
            runtime_access: raw & 0x0000_0004 != 0,
            hardware_error_record: raw & 0x0000_0008 != 0,
            authenticated_write_access: raw & 0x0000_0010 != 0,
            time_based_authenticated_write_access: raw & 0x0000_0020 != 0,
            append_write: raw & 0x0000_0040 != 0,
        }
    }

    /// Encode back to the raw `u32` form for writes.
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrol_core::uefi_vars::EfiAttributes;
    /// let attrs = EfiAttributes::from_u32(0x07);
    /// assert_eq!(attrs.to_u32(), 0x07);
    /// ```
    pub fn to_u32(&self) -> u32 {
        (if self.non_volatile { 0x01 } else { 0 })
            | (if self.bootservice_access { 0x02 } else { 0 })
            | (if self.runtime_access { 0x04 } else { 0 })
            | (if self.hardware_error_record { 0x08 } else { 0 })
            | (if self.authenticated_write_access {
                0x10
            } else {
                0
            })
            | (if self.time_based_authenticated_write_access {
                0x20
            } else {
                0
            })
            | (if self.append_write { 0x40 } else { 0 })
    }
}

/// A parsed `Boot####` EFI variable.
///
/// Only the human-relevant fields are surfaced; the full `EFI_LOAD_OPTION`
/// also carries an `OptionalData` blob and one or more EFI device-path
/// nodes. Frontends generally only need the description and whether the
/// entry is currently active.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EfiBootEntry {
    /// Hex index from the variable name (e.g. `0x0001` for `Boot0001`).
    pub index: u16,
    /// `LOAD_OPTION_ACTIVE` bit. Inactive entries are skipped by firmware.
    pub active: bool,
    /// `LOAD_OPTION_HIDDEN` bit. Hidden entries do not appear in firmware menus.
    pub hidden: bool,
    /// Human-readable description (UTF-16 → UTF-8). Examples:
    /// `"Windows Boot Manager"`, `"ubuntu"`, `"UEFI: SK hynix BC711..."`.
    pub description: String,
}

/// Abstract interface for reading EFI variables.
///
/// Implemented by `daemon::uefi_vars_linux::EfivarFsReader` on Linux and
/// (eventually) a `WindowsFwApiReader` on Windows. Both backends return
/// the raw bytes (without the 4-byte attribute prefix on Linux); the
/// callers pair these bytes with the parsers below.
///
/// # Errors
///
/// All methods return [`BootControlError`] — usually `EspScanFailed` for
/// generic I/O errors or `ToolNotFound` when the backend itself is not
/// present (no `/sys/firmware/efi`, no Windows admin token, …).
pub trait UefiVarReader: Send + Sync {
    /// Read the raw value bytes of a variable in the global EFI namespace.
    ///
    /// On Linux the 4-byte attribute prefix has already been consumed; the
    /// returned slice starts at the actual variable payload.
    fn read_global(&self, name: &str) -> Result<Vec<u8>, BootControlError>;

    /// List names of variables present in the global EFI namespace.
    ///
    /// The shape is the bare `Name` without any `-{GUID}` suffix, so
    /// callers can match on e.g. `"BootOrder"` directly.
    fn list_global_names(&self) -> Result<Vec<String>, BootControlError>;
}

/// Parse the `BootOrder` variable payload into a vector of entry indices.
///
/// `BootOrder` is a packed `u16` little-endian array. The result preserves
/// firmware-defined order — index 0 boots first.
///
/// # Errors
///
/// - [`BootControlError::MalformedValue`] when `payload.len()` is not a
///   multiple of 2.
///
/// # Examples
///
/// ```
/// use bootcontrol_core::uefi_vars::parse_boot_order;
///
/// // Two entries: Boot0001, then Boot0000.
/// let payload = &[0x01, 0x00, 0x00, 0x00];
/// assert_eq!(parse_boot_order(payload).unwrap(), vec![1u16, 0u16]);
/// ```
pub fn parse_boot_order(payload: &[u8]) -> Result<Vec<u16>, BootControlError> {
    if !payload.len().is_multiple_of(2) {
        return Err(BootControlError::MalformedValue {
            key: "BootOrder".to_string(),
            reason: format!("payload length {} is not a multiple of 2", payload.len()),
        });
    }
    Ok(payload
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect())
}

/// Parse a single `Boot####` (`EFI_LOAD_OPTION`) variable into the
/// human-facing fields surfaced by [`EfiBootEntry`].
///
/// The on-disk layout is:
///
/// ```text
/// offset  size  field
/// 0       4     Attributes        (u32 LE, LOAD_OPTION_* bits)
/// 4       2     FilePathListLength (u16 LE)
/// 6       N     Description       (UTF-16 LE, terminated by a u16 zero)
/// 6+N     M     FilePathList      (EFI device path nodes)
/// 6+N+M   …     OptionalData      (opaque to us)
/// ```
///
/// We only need the attributes and the description — the device path
/// parsing is out of scope for this PR.
///
/// # Errors
///
/// - [`BootControlError::MalformedValue`] when the payload is shorter than
///   the mandatory header, or the description is missing its null
///   terminator.
///
/// # Examples
///
/// ```
/// use bootcontrol_core::uefi_vars::parse_boot_entry;
///
/// // Minimal: active=1, empty path list, description "Boot".
/// let mut payload = Vec::new();
/// payload.extend_from_slice(&0x01u32.to_le_bytes()); // LOAD_OPTION_ACTIVE
/// payload.extend_from_slice(&0u16.to_le_bytes());    // FilePathListLength
/// // "Boot" in UTF-16 LE + null terminator.
/// for ch in "Boot".encode_utf16() {
///     payload.extend_from_slice(&ch.to_le_bytes());
/// }
/// payload.extend_from_slice(&0u16.to_le_bytes());
///
/// let entry = parse_boot_entry(0, &payload).unwrap();
/// assert_eq!(entry.description, "Boot");
/// assert!(entry.active);
/// assert!(!entry.hidden);
/// ```
pub fn parse_boot_entry(index: u16, payload: &[u8]) -> Result<EfiBootEntry, BootControlError> {
    const LOAD_OPTION_ACTIVE: u32 = 0x0000_0001;
    const LOAD_OPTION_HIDDEN: u32 = 0x0000_0008;
    const HEADER_LEN: usize = 4 + 2;

    if payload.len() < HEADER_LEN {
        return Err(BootControlError::MalformedValue {
            key: format!("Boot{index:04X}"),
            reason: format!(
                "payload length {} is below the 6-byte header",
                payload.len()
            ),
        });
    }

    let attrs = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);

    // FilePathListLength tells us how many bytes follow the description; we
    // don't consume it for description extraction but its presence is what
    // bounds the description's location.
    let _file_path_len = u16::from_le_bytes([payload[4], payload[5]]) as usize;

    // Walk the UTF-16 LE description until we hit a u16 = 0 terminator.
    let mut chars: Vec<u16> = Vec::new();
    let mut i = HEADER_LEN;
    loop {
        if i + 1 >= payload.len() {
            return Err(BootControlError::MalformedValue {
                key: format!("Boot{index:04X}"),
                reason: "description has no null terminator before end of payload".to_string(),
            });
        }
        let unit = u16::from_le_bytes([payload[i], payload[i + 1]]);
        i += 2;
        if unit == 0 {
            break;
        }
        chars.push(unit);
    }

    let description = String::from_utf16_lossy(&chars);

    Ok(EfiBootEntry {
        index,
        active: attrs & LOAD_OPTION_ACTIVE != 0,
        hidden: attrs & LOAD_OPTION_HIDDEN != 0,
        description,
    })
}

/// Encode a list of `Boot####` indices back into the raw `BootOrder`
/// payload. Inverse of [`parse_boot_order`].
///
/// # Examples
///
/// ```
/// use bootcontrol_core::uefi_vars::encode_boot_order;
/// assert_eq!(encode_boot_order(&[1, 0]), vec![0x01, 0x00, 0x00, 0x00]);
/// ```
pub fn encode_boot_order(indices: &[u16]) -> Vec<u8> {
    let mut out = Vec::with_capacity(indices.len() * 2);
    for &i in indices {
        out.extend_from_slice(&i.to_le_bytes());
    }
    out
}

/// Parse `BootCurrent` or `BootNext`: a single `u16` little-endian.
///
/// # Errors
///
/// - [`BootControlError::MalformedValue`] when the payload is not exactly
///   2 bytes.
///
/// # Examples
///
/// ```
/// use bootcontrol_core::uefi_vars::parse_single_u16_var;
/// assert_eq!(parse_single_u16_var("BootNext", &[0x01, 0x00]).unwrap(), 1);
/// ```
pub fn parse_single_u16_var(name: &str, payload: &[u8]) -> Result<u16, BootControlError> {
    if payload.len() != 2 {
        return Err(BootControlError::MalformedValue {
            key: name.to_string(),
            reason: format!(
                "{name} payload length {} ≠ 2 (expected single u16)",
                payload.len()
            ),
        });
    }
    Ok(u16::from_le_bytes([payload[0], payload[1]]))
}

/// Encode a `Boot####` index into the raw `BootNext` or `BootCurrent`
/// payload form.
///
/// # Examples
///
/// ```
/// use bootcontrol_core::uefi_vars::encode_single_u16_var;
/// assert_eq!(encode_single_u16_var(1), vec![0x01, 0x00]);
/// ```
pub fn encode_single_u16_var(index: u16) -> Vec<u8> {
    index.to_le_bytes().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attributes_round_trip() {
        for raw in [0x00, 0x01, 0x02, 0x04, 0x07, 0x0f, 0x1f, 0x3f, 0x7f] {
            assert_eq!(EfiAttributes::from_u32(raw).to_u32(), raw);
        }
    }

    #[test]
    fn boot_order_parses_two_entries() {
        // Boot0001, Boot0002 — note little-endian per element.
        let payload = &[0x01, 0x00, 0x02, 0x00];
        assert_eq!(parse_boot_order(payload).unwrap(), vec![1u16, 2u16]);
    }

    #[test]
    fn boot_order_empty_is_ok() {
        assert_eq!(parse_boot_order(&[]).unwrap(), Vec::<u16>::new());
    }

    #[test]
    fn boot_order_odd_length_rejected() {
        let result = parse_boot_order(&[0x01, 0x02, 0x03]);
        assert!(matches!(
            result,
            Err(BootControlError::MalformedValue { .. })
        ));
    }

    #[test]
    fn boot_order_round_trip() {
        let indices = vec![5u16, 1u16, 0u16, 0xFFFF];
        let encoded = encode_boot_order(&indices);
        assert_eq!(parse_boot_order(&encoded).unwrap(), indices);
    }

    #[test]
    fn single_u16_parses_boot_next() {
        assert_eq!(parse_single_u16_var("BootNext", &[0x01, 0x00]).unwrap(), 1);
    }

    #[test]
    fn single_u16_rejects_wrong_length() {
        let result = parse_single_u16_var("BootNext", &[0x01]);
        assert!(matches!(
            result,
            Err(BootControlError::MalformedValue { .. })
        ));
    }

    #[test]
    fn single_u16_round_trip() {
        for i in [0u16, 1u16, 0xFFFFu16] {
            let bytes = encode_single_u16_var(i);
            assert_eq!(parse_single_u16_var("X", &bytes).unwrap(), i);
        }
    }

    fn build_boot_entry(attrs: u32, description: &str, file_path_bytes: &[u8]) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&attrs.to_le_bytes());
        payload.extend_from_slice(&(file_path_bytes.len() as u16).to_le_bytes());
        for ch in description.encode_utf16() {
            payload.extend_from_slice(&ch.to_le_bytes());
        }
        payload.extend_from_slice(&0u16.to_le_bytes()); // null terminator
        payload.extend_from_slice(file_path_bytes);
        payload
    }

    #[test]
    fn boot_entry_extracts_description() {
        let payload = build_boot_entry(0x01, "ubuntu", &[]);
        let entry = parse_boot_entry(1, &payload).unwrap();
        assert_eq!(entry.index, 1);
        assert_eq!(entry.description, "ubuntu");
        assert!(entry.active);
        assert!(!entry.hidden);
    }

    #[test]
    fn boot_entry_recognises_hidden_flag() {
        let payload = build_boot_entry(0x01 | 0x08, "Recovery", &[]);
        let entry = parse_boot_entry(0xAA, &payload).unwrap();
        assert!(entry.active);
        assert!(entry.hidden);
    }

    #[test]
    fn boot_entry_inactive_flag() {
        let payload = build_boot_entry(0x00, "Disabled", &[]);
        let entry = parse_boot_entry(7, &payload).unwrap();
        assert!(!entry.active);
        assert!(!entry.hidden);
    }

    #[test]
    fn boot_entry_handles_utf16_unicode() {
        // "Réseau" — `é` is a single u16 in BMP.
        let payload = build_boot_entry(0x01, "Réseau", &[]);
        let entry = parse_boot_entry(0, &payload).unwrap();
        assert_eq!(entry.description, "Réseau");
    }

    #[test]
    fn boot_entry_truncated_header_rejected() {
        let result = parse_boot_entry(0, &[0x01, 0x02, 0x03]);
        assert!(matches!(
            result,
            Err(BootControlError::MalformedValue { .. })
        ));
    }

    #[test]
    fn boot_entry_no_null_terminator_rejected() {
        // Header + a single u16 character with no terminator.
        let mut payload = Vec::new();
        payload.extend_from_slice(&0x01u32.to_le_bytes());
        payload.extend_from_slice(&0u16.to_le_bytes()); // FilePathListLength
        payload.extend_from_slice(&0x41u16.to_le_bytes()); // 'A'
        let result = parse_boot_entry(0, &payload);
        assert!(matches!(
            result,
            Err(BootControlError::MalformedValue { .. })
        ));
    }
}
