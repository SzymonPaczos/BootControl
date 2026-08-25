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

/// Abstract interface for writing EFI variables.
///
/// Kept as a separate trait from [`UefiVarReader`] so a read-only backend
/// (e.g. running BootControl as a non-privileged user, or on a host where
/// efivarfs is mounted `ro`) can implement just `UefiVarReader` and surface
/// a clear "writes unsupported on this backend" error at the wiring layer.
pub trait UefiVarWriter: Send + Sync {
    /// Write the raw payload of a variable in the global EFI namespace.
    ///
    /// `attrs` is the EFI attribute bitfield — for a one-shot `BootNext`
    /// override the canonical value is
    /// `non_volatile | bootservice_access | runtime_access` (`0x07`).
    ///
    /// Implementations must make the write atomic (single `write(2)` on
    /// efivarfs; `SetFirmwareEnvironmentVariableEx` on Windows). They must
    /// not split the write across multiple syscalls because firmware may
    /// observe a half-written variable.
    ///
    /// # Errors
    ///
    /// - [`BootControlError::EspScanFailed`] — I/O error, immutable-bit
    ///   set on Linux without `chattr -i`, or Windows
    ///   `SetFirmwareEnvironmentVariable` returning failure.
    fn write_global(
        &self,
        name: &str,
        payload: &[u8],
        attrs: EfiAttributes,
    ) -> Result<(), BootControlError>;

    /// Delete a variable from the global EFI namespace.
    ///
    /// On Linux this is `rm <root>/<name>-{guid}` after un-immutabling the
    /// file. On Windows it is `SetFirmwareEnvironmentVariableEx(name, guid,
    /// nullptr, 0)`.
    ///
    /// # Errors
    ///
    /// - [`BootControlError::EspScanFailed`] — variable not found is **not**
    ///   an error; only real I/O failures propagate. Treating missing as a
    ///   no-op keeps `unset_boot_next` idempotent.
    fn delete_global(&self, name: &str) -> Result<(), BootControlError>;
}

/// Set `BootNext` to a specific entry index, after validating the index
/// appears in the current `BootOrder`.
///
/// This is a one-shot override consumed by the firmware on next boot: after
/// reboot the variable is cleared by UEFI itself. Writing a `BootNext` that
/// does not match any active `Boot####` is silently ignored by most
/// firmware (the user gets the normal `BootOrder` boot), so we sanity-check
/// the index against the current order to surface obvious typos before the
/// reboot.
///
/// # Errors
///
/// - [`BootControlError::KeyNotFound`] — `index` is not in `BootOrder`.
///   Pass `enforce_order=false` to skip this check (useful when the caller
///   already validated against `list_boot_entries`).
/// - [`BootControlError::EspScanFailed`] — backend I/O failure.
pub fn set_boot_next(
    reader: &dyn UefiVarReader,
    writer: &dyn UefiVarWriter,
    index: u16,
    enforce_order: bool,
) -> Result<(), BootControlError> {
    if enforce_order {
        let order_payload = reader.read_global("BootOrder")?;
        let order = parse_boot_order(&order_payload)?;
        if !order.contains(&index) {
            return Err(BootControlError::KeyNotFound {
                key: format!("Boot{index:04X}"),
            });
        }
    }
    let attrs = EfiAttributes::from_u32(0x07); // NV | BS | RT
    writer.write_global("BootNext", &encode_single_u16_var(index), attrs)
}

/// Clear `BootNext` so the next boot falls back to the normal `BootOrder`.
/// Idempotent — succeeds even if the variable is already absent.
///
/// # Errors
///
/// - [`BootControlError::EspScanFailed`] — backend I/O failure.
pub fn clear_boot_next(writer: &dyn UefiVarWriter) -> Result<(), BootControlError> {
    writer.delete_global("BootNext")
}

/// Replace `BootOrder` with `new_order`, preserving the set of entries.
///
/// `new_order` must be a permutation of the **current** `BootOrder` — same
/// set of indices, no duplicates, possibly reordered. Adding or removing
/// entries is not supported through this helper because both operations
/// require firmware-aware sanity checks (the entry to add must exist as a
/// `Boot####`; removing the last fallback entry can brick a host). Those
/// arrive as separate operations.
///
/// # Errors
///
/// - [`BootControlError::MalformedValue`] — `new_order` has duplicates or
///   does not match the current set of indices.
/// - [`BootControlError::EspScanFailed`] — backend I/O failure.
///
/// # Examples
///
/// ```
/// // See `set_boot_order` tests in this module for end-to-end usage with
/// // the mock reader/writer.
/// ```
pub fn set_boot_order(
    reader: &dyn UefiVarReader,
    writer: &dyn UefiVarWriter,
    new_order: &[u16],
) -> Result<(), BootControlError> {
    let current_payload = reader.read_global("BootOrder")?;
    let current = parse_boot_order(&current_payload)?;

    // Reject duplicates upfront.
    let mut seen = std::collections::HashSet::new();
    for &idx in new_order {
        if !seen.insert(idx) {
            return Err(BootControlError::MalformedValue {
                key: "BootOrder".to_string(),
                reason: format!("duplicate index Boot{idx:04X} in new order"),
            });
        }
    }

    // Same set check — sort both copies and compare.
    let mut current_sorted = current.clone();
    current_sorted.sort_unstable();
    let mut new_sorted = new_order.to_vec();
    new_sorted.sort_unstable();
    if current_sorted != new_sorted {
        return Err(BootControlError::MalformedValue {
            key: "BootOrder".to_string(),
            reason: format!(
                "new order {new_order:?} is not a permutation of current {current:?} \
                 (add/remove is a separate operation)"
            ),
        });
    }

    let attrs = EfiAttributes::from_u32(0x07);
    writer.write_global("BootOrder", &encode_boot_order(new_order), attrs)
}

/// Move the `Boot####` at `from_index` to `to_position` in the current
/// `BootOrder`. Both arguments are zero-based positions in the order list
/// (not entry indices). Out-of-range positions are clamped to the valid
/// range.
///
/// Returns the new order so the caller can echo it back into the UI without
/// a second read.
///
/// # Errors
///
/// - [`BootControlError::EspScanFailed`] — backend I/O failure.
/// - [`BootControlError::KeyNotFound`] — `from_position` is past the end of
///   the current order (i.e. fewer entries than the caller assumed).
pub fn move_boot_order_entry(
    reader: &dyn UefiVarReader,
    writer: &dyn UefiVarWriter,
    from_position: usize,
    to_position: usize,
) -> Result<Vec<u16>, BootControlError> {
    let current_payload = reader.read_global("BootOrder")?;
    let mut order = parse_boot_order(&current_payload)?;

    if from_position >= order.len() {
        return Err(BootControlError::KeyNotFound {
            key: format!("BootOrder[{from_position}]"),
        });
    }
    let to = to_position.min(order.len().saturating_sub(1));
    let entry = order.remove(from_position);
    order.insert(to, entry);

    set_boot_order(reader, writer, &order)?;
    Ok(order)
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
    // `as_chunks` returns (complete chunks, remainder). The remainder is
    // provably empty here — the length guard above rejects odd payloads — so
    // taking `.0` drops nothing. Keep the guard and this call together.
    Ok(payload
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_le_bytes(*c))
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
    use std::sync::Mutex;

    /// In-memory writer used by the `set_boot_next` / `clear_boot_next`
    /// tests below. Holds the most recent (name, payload, attrs) call so
    /// the test can assert on the exact wire bytes. Uses `Mutex` (not
    /// `RefCell`) because `UefiVarWriter: Send + Sync`.
    struct MockWriter {
        last_write: Mutex<Option<(String, Vec<u8>, EfiAttributes)>>,
        last_delete: Mutex<Option<String>>,
    }
    impl MockWriter {
        fn new() -> Self {
            Self {
                last_write: Mutex::new(None),
                last_delete: Mutex::new(None),
            }
        }
    }
    impl UefiVarWriter for MockWriter {
        fn write_global(
            &self,
            name: &str,
            payload: &[u8],
            attrs: EfiAttributes,
        ) -> Result<(), BootControlError> {
            *self.last_write.lock().unwrap() = Some((name.to_string(), payload.to_vec(), attrs));
            Ok(())
        }
        fn delete_global(&self, name: &str) -> Result<(), BootControlError> {
            *self.last_delete.lock().unwrap() = Some(name.to_string());
            Ok(())
        }
    }

    struct MockReader {
        boot_order: Vec<u16>,
    }
    impl UefiVarReader for MockReader {
        fn read_global(&self, name: &str) -> Result<Vec<u8>, BootControlError> {
            if name == "BootOrder" {
                Ok(encode_boot_order(&self.boot_order))
            } else {
                Err(BootControlError::EspScanFailed {
                    reason: format!("MockReader has no {name}"),
                })
            }
        }
        fn list_global_names(&self) -> Result<Vec<String>, BootControlError> {
            Ok(vec!["BootOrder".to_string()])
        }
    }

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
    fn boot_order_never_truncates_a_trailing_odd_byte() {
        // Guards the shape of the parser, not just its happy path: a payload
        // with a dangling byte must be REJECTED, never silently parsed as the
        // complete pairs with the remainder dropped. `as_chunks` hands the
        // remainder back separately, so dropping the length guard would turn
        // a malformed BootOrder into a plausible-looking short boot order —
        // the firmware would boot something the user never chose.
        let truncated = &[0x01, 0x00, 0x02, 0x00, 0x03];
        assert!(
            matches!(
                parse_boot_order(truncated),
                Err(BootControlError::MalformedValue { .. })
            ),
            "odd-length payload must be rejected, not truncated to its complete pairs"
        );

        // Same bytes without the dangling one parse fine — proves the
        // rejection above is about the odd byte, not about the content.
        assert_eq!(parse_boot_order(&truncated[..4]).unwrap(), vec![1u16, 2u16]);
    }

    #[test]
    fn boot_order_preserves_little_endian_high_bytes() {
        // High bytes exercise the per-element byte order: a parser that read
        // big-endian would return 0x0102 here instead of 0x0201.
        let payload = &[0x01, 0x02, 0xFF, 0xFF, 0x00, 0x80];
        assert_eq!(
            parse_boot_order(payload).unwrap(),
            vec![0x0201u16, 0xFFFFu16, 0x8000u16]
        );
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
    fn set_boot_next_writes_index_with_canonical_attrs() {
        let reader = MockReader {
            boot_order: vec![0, 1, 2],
        };
        let writer = MockWriter::new();
        set_boot_next(&reader, &writer, 1, true).unwrap();

        let (name, payload, attrs) = writer.last_write.lock().unwrap().clone().unwrap();
        assert_eq!(name, "BootNext");
        assert_eq!(payload, vec![0x01, 0x00]);
        // NV | BS | RT
        assert!(attrs.non_volatile);
        assert!(attrs.bootservice_access);
        assert!(attrs.runtime_access);
        assert!(!attrs.append_write);
    }

    #[test]
    fn set_boot_next_rejects_unknown_index_when_enforced() {
        let reader = MockReader {
            boot_order: vec![0, 1, 2],
        };
        let writer = MockWriter::new();
        let result = set_boot_next(&reader, &writer, 7, true);
        assert!(matches!(result, Err(BootControlError::KeyNotFound { .. })));
        // No write should have happened.
        assert!(writer.last_write.lock().unwrap().is_none());
    }

    #[test]
    fn set_boot_next_skips_enforcement_when_disabled() {
        let reader = MockReader { boot_order: vec![] };
        let writer = MockWriter::new();
        set_boot_next(&reader, &writer, 99, false).unwrap();
        assert!(writer.last_write.lock().unwrap().is_some());
    }

    /// Read-write mock: holds variables in a Mutex-guarded HashMap.
    /// Used for `set_boot_order` / `move_boot_order_entry` which need the
    /// reader to see writes applied earlier in the same test.
    struct StatefulMock {
        vars: Mutex<std::collections::HashMap<String, Vec<u8>>>,
    }
    impl StatefulMock {
        fn with_boot_order(order: &[u16]) -> Self {
            let mut vars = std::collections::HashMap::new();
            vars.insert("BootOrder".to_string(), encode_boot_order(order));
            Self {
                vars: Mutex::new(vars),
            }
        }
    }
    impl UefiVarReader for StatefulMock {
        fn read_global(&self, name: &str) -> Result<Vec<u8>, BootControlError> {
            self.vars.lock().unwrap().get(name).cloned().ok_or_else(|| {
                BootControlError::EspScanFailed {
                    reason: format!("no such variable: {name}"),
                }
            })
        }
        fn list_global_names(&self) -> Result<Vec<String>, BootControlError> {
            Ok(self.vars.lock().unwrap().keys().cloned().collect())
        }
    }
    impl UefiVarWriter for StatefulMock {
        fn write_global(
            &self,
            name: &str,
            payload: &[u8],
            _attrs: EfiAttributes,
        ) -> Result<(), BootControlError> {
            self.vars
                .lock()
                .unwrap()
                .insert(name.to_string(), payload.to_vec());
            Ok(())
        }
        fn delete_global(&self, name: &str) -> Result<(), BootControlError> {
            self.vars.lock().unwrap().remove(name);
            Ok(())
        }
    }

    #[test]
    fn set_boot_order_writes_permutation() {
        let mock = StatefulMock::with_boot_order(&[0, 1, 2]);
        set_boot_order(&mock, &mock, &[2, 0, 1]).unwrap();
        let payload = mock.read_global("BootOrder").unwrap();
        assert_eq!(parse_boot_order(&payload).unwrap(), vec![2u16, 0, 1]);
    }

    #[test]
    fn set_boot_order_rejects_duplicate_index() {
        let mock = StatefulMock::with_boot_order(&[0, 1, 2]);
        let result = set_boot_order(&mock, &mock, &[0, 0, 1]);
        match result {
            Err(BootControlError::MalformedValue { reason, .. }) => {
                assert!(reason.contains("duplicate"));
            }
            other => panic!("expected MalformedValue, got {other:?}"),
        }
    }

    #[test]
    fn set_boot_order_rejects_changed_set() {
        let mock = StatefulMock::with_boot_order(&[0, 1, 2]);
        let result = set_boot_order(&mock, &mock, &[0, 1, 3]);
        match result {
            Err(BootControlError::MalformedValue { reason, .. }) => {
                assert!(reason.contains("permutation"));
            }
            other => panic!("expected MalformedValue, got {other:?}"),
        }
    }

    #[test]
    fn move_boot_order_entry_swaps_positions() {
        let mock = StatefulMock::with_boot_order(&[5, 0, 3, 2]);
        // Move index at position 0 (Boot0005) to position 2.
        let new_order = move_boot_order_entry(&mock, &mock, 0, 2).unwrap();
        assert_eq!(new_order, vec![0, 3, 5, 2]);
    }

    #[test]
    fn move_boot_order_entry_clamps_to_position() {
        let mock = StatefulMock::with_boot_order(&[5, 0, 3]);
        // Caller asks to move to position 99 — clamp to last (2).
        let new_order = move_boot_order_entry(&mock, &mock, 0, 99).unwrap();
        assert_eq!(new_order, vec![0, 3, 5]);
    }

    #[test]
    fn move_boot_order_entry_rejects_out_of_range_from() {
        let mock = StatefulMock::with_boot_order(&[5, 0]);
        let result = move_boot_order_entry(&mock, &mock, 5, 0);
        assert!(matches!(result, Err(BootControlError::KeyNotFound { .. })));
    }

    #[test]
    fn clear_boot_next_calls_delete() {
        let writer = MockWriter::new();
        clear_boot_next(&writer).unwrap();
        assert_eq!(
            writer.last_delete.lock().unwrap().as_deref(),
            Some("BootNext")
        );
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
