//! `systemd-boot` backend implementing [`crate::boot_manager::BootManager`].
//!
//! Parses and writes loader entry files in `/boot/loader/entries/*.conf`.
//! Also handles `loader.conf` for default entry management.
//!
//! # Format
//!
//! Loader entry files use a simple `key value` format (space-separated, **not** `=`):
//!
//! ```text
//! title   Arch Linux
//! linux   /vmlinuz-linux
//! initrd  /initramfs-linux.img
//! options root=/dev/sda1 rw quiet
//! ```
//!
//! Lines starting with `#` are comments. Blank lines are ignored.
//! Unknown fields are preserved verbatim (forward compatibility).
//!
//! # ETag algorithm (frozen)
//!
//! To compute an ETag for a directory of loader entries:
//! 1. Compute SHA-256 of each `.conf` file's content.
//! 2. Collect `(filename, sha256_hex)` pairs.
//! 3. Sort by filename lexicographically.
//! 4. Concatenate as `"filename:sha256\n"` for each pair.
//! 5. SHA-256 the resulting string.

#![deny(warnings)]
#![deny(missing_docs)]

use crate::{
    boot_manager::{BootEntry, BootManager},
    error::BootControlError,
    hash::compute_etag_str,
};

// ─────────────────────────────────────────────────────────────────────────────
// Data types
// ─────────────────────────────────────────────────────────────────────────────

/// A parsed systemd-boot loader entry (`.conf` file).
///
/// Unknown fields are preserved in [`LoaderEntry::extra`] so that
/// serialisation is a lossless round-trip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoaderEntry {
    /// Entry title displayed in the boot menu.
    pub title: Option<String>,
    /// Path to the kernel image (e.g. `/vmlinuz-linux`).
    pub linux: Option<String>,
    /// Path to the initramfs image (e.g. `/initramfs-linux.img`).
    pub initrd: Option<String>,
    /// Kernel command-line options.
    pub options: Option<String>,
    /// Machine ID from `/etc/machine-id`.
    pub machine_id: Option<String>,
    /// Unknown key-value pairs, preserved verbatim.
    pub extra: Vec<(String, String)>,
    /// Comment and blank lines, preserved verbatim with original positions.
    /// Each element is `(line_index, raw_line)`.
    pub preserved_lines: Vec<(usize, String)>,
}

impl LoaderEntry {
    fn new() -> Self {
        Self {
            title: None,
            linux: None,
            initrd: None,
            options: None,
            machine_id: None,
            extra: Vec::new(),
            preserved_lines: Vec::new(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Pure functions
// ─────────────────────────────────────────────────────────────────────────────

/// Parse the content of a single systemd-boot loader entry `.conf` file.
///
/// The format is `key<whitespace>value` per line. Lines starting with `#`
/// and blank lines are preserved in [`LoaderEntry::preserved_lines`].
///
/// # Arguments
///
/// * `content` — Raw text content of a `.conf` loader entry file.
///
/// # Errors
///
/// - [`BootControlError::MalformedValue`] — if the file is completely empty
///   or contains no recognisable fields.
/// - [`BootControlError::MalformedValue`] — if a non-blank, non-comment line
///   has no whitespace separator between key and value.
///
/// # Examples
///
/// ```
/// use bootcontrol_core::backends::systemd_boot::parse_loader_entry;
///
/// let content = "title   Arch Linux\nlinux   /vmlinuz-linux\n";
/// let entry = parse_loader_entry(content).unwrap();
/// assert_eq!(entry.title.as_deref(), Some("Arch Linux"));
/// assert_eq!(entry.linux.as_deref(), Some("/vmlinuz-linux"));
/// ```
pub fn parse_loader_entry(content: &str) -> Result<LoaderEntry, BootControlError> {
    if content.trim().is_empty() {
        return Err(BootControlError::MalformedValue {
            key: "loader_entry".to_string(),
            reason: "file is empty".to_string(),
        });
    }

    let mut entry = LoaderEntry::new();
    let mut line_index = 0usize;

    for line in content.lines() {
        let trimmed = line.trim();

        // Blank lines and comments — preserve verbatim.
        if trimmed.is_empty() || trimmed.starts_with('#') {
            entry.preserved_lines.push((line_index, line.to_string()));
            line_index += 1;
            continue;
        }

        // Split on the first whitespace run: `key<ws>value`.
        let mut parts = trimmed.splitn(2, char::is_whitespace);
        let key = parts.next().unwrap_or("").to_lowercase();
        let value = match parts.next() {
            Some(v) => v.trim_start().to_string(),
            None => {
                return Err(BootControlError::MalformedValue {
                    key: key.clone(),
                    reason: format!("line {line_index}: no separator between key and value"),
                });
            }
        };

        match key.as_str() {
            "title" => entry.title = Some(value),
            "linux" => entry.linux = Some(value),
            "initrd" => entry.initrd = Some(value),
            "options" => entry.options = Some(value),
            "machine-id" => entry.machine_id = Some(value),
            _ => entry.extra.push((key, value)),
        }

        line_index += 1;
    }

    Ok(entry)
}

/// Serialize a [`LoaderEntry`] back to its `.conf` file string representation.
///
/// Preserves all original comment and blank lines at their original positions.
/// Known fields are emitted in a canonical order; unknown fields follow.
///
/// # Arguments
///
/// * `entry` — The parsed loader entry to serialise.
///
/// # Examples
///
/// ```
/// use bootcontrol_core::backends::systemd_boot::{parse_loader_entry, serialize_loader_entry};
///
/// let original = "title   Arch Linux\nlinux   /vmlinuz-linux\n";
/// let entry = parse_loader_entry(original).unwrap();
/// let serialized = serialize_loader_entry(&entry);
/// // Must contain the same fields
/// assert!(serialized.contains("title"));
/// assert!(serialized.contains("Arch Linux"));
/// ```
pub fn serialize_loader_entry(entry: &LoaderEntry) -> String {
    let mut lines: Vec<String> = Vec::new();

    if let Some(ref v) = entry.title {
        lines.push(format!("title   {v}"));
    }
    if let Some(ref v) = entry.linux {
        lines.push(format!("linux   {v}"));
    }
    if let Some(ref v) = entry.initrd {
        lines.push(format!("initrd  {v}"));
    }
    if let Some(ref v) = entry.options {
        lines.push(format!("options {v}"));
    }
    if let Some(ref v) = entry.machine_id {
        lines.push(format!("machine-id {v}"));
    }
    for (k, v) in &entry.extra {
        lines.push(format!("{k} {v}"));
    }

    // Re-insert preserved lines (comments/blanks) at their original positions.
    for (idx, raw) in &entry.preserved_lines {
        let insert_at = (*idx).min(lines.len());
        lines.insert(insert_at, raw.clone());
    }

    let mut out = lines.join("\n");
    out.push('\n');
    out
}

/// Compute the ETag for a directory of loader entries.
///
/// # Algorithm (frozen — must not change across versions)
///
/// 1. `file_digests` is a slice of `(filename, sha256_hex_of_content)` pairs.
/// 2. Sort by filename lexicographically.
/// 3. Concatenate as `"filename:sha256\n"` for each pair.
/// 4. SHA-256 the resulting concatenated string via
///    [`bootcontrol_core::hash::compute_etag_str`].
///
/// # Arguments
///
/// * `file_digests` — Slice of `(filename, sha256_hex)` pairs for every `.conf`
///   file in the entries directory. The caller is responsible for computing
///   individual file ETags using [`crate::hash::compute_etag_str`].
///
/// # Examples
///
/// ```
/// use bootcontrol_core::backends::systemd_boot::compute_directory_etag;
///
/// let digests = vec![
///     ("arch.conf".to_string(), "a".repeat(64)),
///     ("windows.conf".to_string(), "b".repeat(64)),
/// ];
/// let etag = compute_directory_etag(&digests);
/// assert_eq!(etag.len(), 64);
/// ```
pub fn compute_directory_etag(file_digests: &[(String, String)]) -> String {
    let mut sorted = file_digests.to_vec();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    let combined: String = sorted
        .iter()
        .map(|(name, digest)| format!("{name}:{digest}\n"))
        .collect();
    compute_etag_str(&combined)
}

// ─────────────────────────────────────────────────────────────────────────────
// Backend
// ─────────────────────────────────────────────────────────────────────────────

/// `systemd-boot` bootloader backend.
///
/// Implements [`BootManager`] by parsing loader entry files and `loader.conf`.
///
/// # Examples
///
/// ```
/// use bootcontrol_core::backends::systemd_boot::SystemdBootBackend;
/// use bootcontrol_core::boot_manager::BootManager;
///
/// let backend = SystemdBootBackend;
/// assert_eq!(backend.name(), "systemd-boot");
/// ```
pub struct SystemdBootBackend;

impl BootManager for SystemdBootBackend {
    /// Parse a `loader.conf` or a single `.conf` entry and return boot entries.
    ///
    /// For this method, `content` is treated as the content of a **single**
    /// loader entry file. In the daemon, iterate over all `.conf` files and
    /// call this once per file.
    ///
    /// # Arguments
    ///
    /// * `content` — Raw text of a loader entry `.conf` file.
    ///
    /// # Errors
    ///
    /// Returns [`BootControlError::MalformedValue`] if the content is empty or
    /// unparseable.
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrol_core::backends::systemd_boot::SystemdBootBackend;
    /// use bootcontrol_core::boot_manager::BootManager;
    ///
    /// let backend = SystemdBootBackend;
    /// let content = "title Arch Linux\nlinux /vmlinuz-linux\n";
    /// let entries = backend.list_entries(content).unwrap();
    /// assert_eq!(entries.len(), 1);
    /// assert_eq!(entries[0].label, "Arch Linux");
    /// ```
    fn list_entries(&self, content: &str) -> Result<Vec<BootEntry>, BootControlError> {
        let entry = parse_loader_entry(content)?;
        let label = entry.title.clone().unwrap_or_else(|| "Unknown".to_string());
        let id = entry
            .linux
            .clone()
            .unwrap_or_else(|| label.to_lowercase().replace(' ', "-"));
        Ok(vec![BootEntry {
            id,
            label,
            is_default: false,
        }])
    }

    /// Return a modified copy of `loader.conf` with `default` set to `id`.
    ///
    /// `content` is the content of `/boot/loader/loader.conf`. `id` is the
    /// filename stem of the entry to set as default (without `.conf`).
    ///
    /// # Arguments
    ///
    /// * `content` — Raw text of `loader.conf`.
    /// * `id`      — Filename stem of the entry to set as default.
    ///
    /// # Errors
    ///
    /// Does not fail — if the `default` line is absent it is appended.
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrol_core::backends::systemd_boot::SystemdBootBackend;
    /// use bootcontrol_core::boot_manager::BootManager;
    ///
    /// let backend = SystemdBootBackend;
    /// let loader_conf = "timeout 3\ndefault arch\n";
    /// let modified = backend.set_default(loader_conf, "fedora").unwrap();
    /// assert!(modified.contains("default fedora"));
    /// assert!(!modified.contains("default arch"));
    /// ```
    fn set_default(&self, content: &str, id: &str) -> Result<String, BootControlError> {
        let new_line = format!("default {id}");
        let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();

        match lines
            .iter()
            .rposition(|l| l.trim_start().starts_with("default "))
        {
            Some(idx) => lines[idx] = new_line,
            None => lines.push(new_line),
        }

        Ok(lines.join("\n") + "\n")
    }

    /// Compute the SHA-256 ETag of the raw content.
    ///
    /// # Arguments
    ///
    /// * `content` — Raw text to hash.
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrol_core::backends::systemd_boot::SystemdBootBackend;
    /// use bootcontrol_core::boot_manager::BootManager;
    ///
    /// let backend = SystemdBootBackend;
    /// let etag = backend.compute_etag("title Arch Linux\n");
    /// assert_eq!(etag.len(), 64);
    /// ```
    fn compute_etag(&self, content: &str) -> String {
        compute_etag_str(content)
    }

    fn name(&self) -> &'static str {
        "systemd-boot"
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── 1. Minimal valid entry ────────────────────────────────────────────────
    #[test]
    fn parses_minimal_entry() {
        let content = "title   Arch Linux\nlinux   /vmlinuz-linux\n";
        let entry = parse_loader_entry(content).unwrap();
        assert_eq!(entry.title.as_deref(), Some("Arch Linux"));
        assert_eq!(entry.linux.as_deref(), Some("/vmlinuz-linux"));
        assert!(entry.initrd.is_none());
        assert!(entry.options.is_none());
    }

    // ── 2. Full entry ─────────────────────────────────────────────────────────
    #[test]
    fn parses_full_entry() {
        let content = "\
title   Arch Linux
linux   /vmlinuz-linux
initrd  /initramfs-linux.img
options root=/dev/sda1 rw quiet
machine-id abc123
";
        let entry = parse_loader_entry(content).unwrap();
        assert_eq!(entry.title.as_deref(), Some("Arch Linux"));
        assert_eq!(entry.linux.as_deref(), Some("/vmlinuz-linux"));
        assert_eq!(entry.initrd.as_deref(), Some("/initramfs-linux.img"));
        assert_eq!(entry.options.as_deref(), Some("root=/dev/sda1 rw quiet"));
        assert_eq!(entry.machine_id.as_deref(), Some("abc123"));
    }

    // ── 3. Unknown fields preserved ───────────────────────────────────────────
    #[test]
    fn unknown_fields_preserved_in_extra() {
        let content = "title Test\nversion 1.2.3\nlinux /vmlinuz\n";
        let entry = parse_loader_entry(content).unwrap();
        assert!(entry
            .extra
            .iter()
            .any(|(k, v)| k == "version" && v == "1.2.3"));
    }

    // ── 4. Comments and blank lines preserved ─────────────────────────────────
    #[test]
    fn comment_and_blank_lines_preserved() {
        let content = "# Main entry\n\ntitle Arch\nlinux /vmlinuz\n";
        let entry = parse_loader_entry(content).unwrap();
        assert!(!entry.preserved_lines.is_empty());
        let has_comment = entry
            .preserved_lines
            .iter()
            .any(|(_, l)| l.starts_with('#'));
        assert!(has_comment, "comment line must be in preserved_lines");
    }

    // ── 5. Malformed line with no separator ───────────────────────────────────
    #[test]
    fn malformed_line_without_separator_returns_error() {
        let content = "title Arch\ntitleonly\n";
        let result = parse_loader_entry(content);
        assert!(
            matches!(result, Err(BootControlError::MalformedValue { .. })),
            "expected MalformedValue, got: {result:?}"
        );
    }

    // ── 6. Empty file returns error ───────────────────────────────────────────
    #[test]
    fn empty_file_returns_malformed_value() {
        let result = parse_loader_entry("");
        assert!(matches!(
            result,
            Err(BootControlError::MalformedValue { .. })
        ));
    }

    // ── 7. Round-trip ─────────────────────────────────────────────────────────
    #[test]
    fn round_trip_parse_serialize_parse() {
        let content = "\
title   Arch Linux
linux   /vmlinuz-linux
initrd  /initramfs-linux.img
options root=/dev/sda1 rw quiet
";
        let entry1 = parse_loader_entry(content).unwrap();
        let serialized = serialize_loader_entry(&entry1);
        let entry2 = parse_loader_entry(&serialized).unwrap();
        assert_eq!(entry1.title, entry2.title);
        assert_eq!(entry1.linux, entry2.linux);
        assert_eq!(entry1.initrd, entry2.initrd);
        assert_eq!(entry1.options, entry2.options);
    }

    // ── 8. compute_directory_etag — sorting + determinism ────────────────────
    #[test]
    fn directory_etag_is_deterministic_and_sorted() {
        let digests_asc = vec![
            ("arch.conf".to_string(), "a".repeat(64)),
            ("windows.conf".to_string(), "b".repeat(64)),
        ];
        let digests_desc = vec![
            ("windows.conf".to_string(), "b".repeat(64)),
            ("arch.conf".to_string(), "a".repeat(64)),
        ];
        let etag1 = compute_directory_etag(&digests_asc);
        let etag2 = compute_directory_etag(&digests_desc);
        assert_eq!(
            etag1, etag2,
            "ETag must be the same regardless of input order"
        );
        assert_eq!(etag1.len(), 64);
    }

    // ── Backend tests ─────────────────────────────────────────────────────────
    #[test]
    fn backend_name_is_systemd_boot() {
        assert_eq!(SystemdBootBackend.name(), "systemd-boot");
    }

    #[test]
    fn backend_set_default_updates_existing() {
        let loader_conf = "timeout 3\ndefault arch\n";
        let modified = SystemdBootBackend
            .set_default(loader_conf, "fedora")
            .unwrap();
        assert!(modified.contains("default fedora"));
        assert!(!modified.contains("default arch"));
    }

    #[test]
    fn backend_set_default_appends_when_absent() {
        let loader_conf = "timeout 3\n";
        let modified = SystemdBootBackend.set_default(loader_conf, "arch").unwrap();
        assert!(modified.contains("default arch"));
    }

    #[test]
    fn backend_compute_etag_returns_64_chars() {
        let etag = SystemdBootBackend.compute_etag("title Arch\n");
        assert_eq!(etag.len(), 64);
    }

    #[test]
    fn backend_list_entries_extracts_title_as_label() {
        let content = "title Fedora Linux\nlinux /vmlinuz\n";
        let entries = SystemdBootBackend.list_entries(content).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].label, "Fedora Linux");
    }
}
