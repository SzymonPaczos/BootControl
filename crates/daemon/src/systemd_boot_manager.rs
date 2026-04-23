//! Filesystem operations for systemd-boot loader entry management.
//!
//! Manages two filesystem locations:
//! - `/boot/loader/entries/*.conf`  — individual loader entry files
//! - `/boot/loader/loader.conf`     — global loader configuration (default entry)
//!
//! All writes use the same atomic-rename + flock pattern as [`crate::grub_manager`]:
//! lock → verify ETag → write temp → fsync → rename.

use std::{
    fs::{self, File, OpenOptions},
    io::{BufReader, Read, Write},
    path::{Path, PathBuf},
};

use bootcontrol_core::{
    backends::systemd_boot::{parse_loader_entry, serialize_loader_entry, LoaderEntry},
    error::BootControlError,
    hash::{compute_etag_str, verify_etag},
};
use nix::fcntl::{Flock, FlockArg};
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

// ─────────────────────────────────────────────────────────────────────────────
// Public types
// ─────────────────────────────────────────────────────────────────────────────

/// A loader entry record as returned to callers.
///
/// `id` is the filename stem (e.g. `"arch"` for `arch.conf`).
/// `etag` is the SHA-256 of that specific file — used for per-file writes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryRecord {
    /// Filename stem (e.g. `"arch"` for `/boot/loader/entries/arch.conf`).
    pub id: String,
    /// Entry title, if present.
    pub title: Option<String>,
    /// Kernel image path (e.g. `/vmlinuz-linux`).
    pub linux: Option<String>,
    /// Initramfs path.
    pub initrd: Option<String>,
    /// Kernel command-line options.
    pub options: Option<String>,
    /// Machine ID from `/etc/machine-id`.
    pub machine_id: Option<String>,
    /// Per-file SHA-256 ETag.
    pub etag: String,
    /// `true` when this entry matches the `default` in `loader.conf`.
    pub is_default: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// Read operations
// ─────────────────────────────────────────────────────────────────────────────

/// Read all `.conf` files from `entries_dir` and return a list of parsed entries.
///
/// `loader_conf_path` is used only to determine which entry is the current
/// default (reads the `default` key from `loader.conf`).
///
/// # Errors
///
/// - [`BootControlError::EspScanFailed`] if `entries_dir` cannot be read.
pub fn read_all_entries(
    entries_dir: &Path,
    loader_conf_path: &Path,
) -> Result<Vec<EntryRecord>, BootControlError> {
    let current_default = read_loader_conf_default(loader_conf_path).unwrap_or_default();

    let mut records: Vec<EntryRecord> = Vec::new();

    let read_dir = fs::read_dir(entries_dir).map_err(|e| {
        error!(?entries_dir, io_error = %e, "failed to read loader entries directory");
        BootControlError::EspScanFailed {
            reason: format!("cannot read entries dir: {e}"),
        }
    })?;

    for dir_entry in read_dir.flatten() {
        let path = dir_entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("conf") {
            continue;
        }

        let id = match path.file_stem().and_then(|s| s.to_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };

        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                warn!(?path, io_error = %e, "skipping unreadable loader entry");
                continue;
            }
        };

        let etag = compute_etag_str(&content);

        let entry = match parse_loader_entry(&content) {
            Ok(e) => e,
            Err(e) => {
                warn!(?path, parse_error = %e, "skipping malformed loader entry");
                continue;
            }
        };

        let is_default = is_entry_default(&id, &current_default);

        records.push(EntryRecord {
            id,
            title: entry.title,
            linux: entry.linux,
            initrd: entry.initrd,
            options: entry.options,
            machine_id: entry.machine_id,
            etag,
            is_default,
        });
    }

    records.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(records)
}

/// Read the `default` key from `loader.conf`, returning an empty string if
/// the file does not exist or has no `default` entry.
pub fn read_loader_conf_default(loader_conf_path: &Path) -> Result<String, BootControlError> {
    let content = fs::read_to_string(loader_conf_path).map_err(|e| {
        BootControlError::EspScanFailed {
            reason: format!("cannot read loader.conf: {e}"),
        }
    })?;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }
        let mut parts = trimmed.splitn(2, char::is_whitespace);
        let key = parts.next().unwrap_or("").to_lowercase();
        let value = parts.next().unwrap_or("").trim_start().to_string();
        if key == "default" {
            return Ok(value);
        }
    }
    Ok(String::new())
}

/// Compute the ETag of `loader.conf`.
pub fn fetch_loader_conf_etag(loader_conf_path: &Path) -> Result<String, BootControlError> {
    let content = fs::read_to_string(loader_conf_path).map_err(|e| {
        BootControlError::EspScanFailed {
            reason: format!("cannot read loader.conf for ETag: {e}"),
        }
    })?;
    Ok(compute_etag_str(&content))
}

/// Read a single loader entry file by ID.
///
/// Returns `(entry, file_etag)`.
pub fn read_entry(
    entries_dir: &Path,
    id: &str,
) -> Result<(LoaderEntry, String), BootControlError> {
    let path = entry_path(entries_dir, id);
    let content = fs::read_to_string(&path).map_err(|e| {
        BootControlError::EspScanFailed {
            reason: format!("cannot read entry '{id}': {e}"),
        }
    })?;
    let etag = compute_etag_str(&content);
    let entry = parse_loader_entry(&content)?;
    Ok((entry, etag))
}

// ─────────────────────────────────────────────────────────────────────────────
// Write operations
// ─────────────────────────────────────────────────────────────────────────────

/// Atomically write a loader entry file.
///
/// `expected_etag` must match the current file's ETag (TOCTOU-safe under flock).
/// If the file does not yet exist, `expected_etag` must be an empty string.
pub fn write_loader_entry(
    entries_dir: &Path,
    id: &str,
    entry: &LoaderEntry,
    expected_etag: &str,
) -> Result<(), BootControlError> {
    validate_entry_id(id)?;
    let path = entry_path(entries_dir, id);
    let new_content = serialize_loader_entry(entry);
    atomic_write_with_etag(&path, &new_content, expected_etag)
}

/// Update the `default` entry in `loader.conf`.
///
/// If `loader.conf` does not exist, it is created with only the `default` key.
/// `expected_etag` must match the current `loader.conf` ETag (or be empty if
/// the file does not exist yet).
pub fn set_loader_default(
    loader_conf_path: &Path,
    id: &str,
    expected_etag: &str,
) -> Result<(), BootControlError> {
    validate_entry_id(id)?;

    let new_content = if loader_conf_path.exists() {
        let content = fs::read_to_string(loader_conf_path).map_err(|e| {
            BootControlError::EspScanFailed {
                reason: format!("cannot read loader.conf: {e}"),
            }
        })?;
        update_or_append_key(&content, "default", id)
    } else {
        format!("default {id}\n")
    };

    atomic_write_with_etag(loader_conf_path, &new_content, expected_etag)
}

// ─────────────────────────────────────────────────────────────────────────────
// Private helpers
// ─────────────────────────────────────────────────────────────────────────────

fn entry_path(entries_dir: &Path, id: &str) -> PathBuf {
    entries_dir.join(format!("{id}.conf"))
}

/// Validate that an entry ID is a safe filename stem (no path separators, not empty).
fn validate_entry_id(id: &str) -> Result<(), BootControlError> {
    if id.is_empty() || id.contains('/') || id.contains('\\') || id.contains("..") {
        return Err(BootControlError::MalformedValue {
            key: "id".to_string(),
            reason: format!("invalid entry id: '{id}'"),
        });
    }
    Ok(())
}

/// Return `true` if `entry_id` matches the `default` value from `loader.conf`.
///
/// systemd-boot accepts either a bare stem (`arch`) or a full filename with
/// extension (`arch.conf`).  Both forms are checked.
fn is_entry_default(entry_id: &str, loader_conf_default: &str) -> bool {
    let default = loader_conf_default.trim();
    default == entry_id
        || default == format!("{entry_id}.conf")
        || default.trim_end_matches(".conf") == entry_id
}

/// Update or append `key value` in a loader.conf-style content string.
fn update_or_append_key(content: &str, key: &str, value: &str) -> String {
    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    let new_line = format!("{key} {value}");

    // Try to update existing key line.
    let mut found = false;
    for line in &mut lines {
        let trimmed = line.trim_start();
        if !trimmed.starts_with('#') && !trimmed.is_empty() {
            let mut parts = trimmed.splitn(2, char::is_whitespace);
            if parts.next().unwrap_or("").to_lowercase() == key {
                *line = new_line.clone();
                found = true;
                break;
            }
        }
    }
    if !found {
        lines.push(new_line);
    }
    lines.join("\n") + "\n"
}

/// Atomic write with ETag verification under flock.
fn atomic_write_with_etag(
    path: &Path,
    new_content: &str,
    expected_etag: &str,
) -> Result<(), BootControlError> {
    if path.exists() {
        // File exists — acquire lock and verify ETag.
        let lock_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(|e| BootControlError::EspScanFailed {
                reason: format!("cannot open '{path:?}' for locking: {e}"),
            })?;

        let mut locked =
            Flock::lock(lock_file, FlockArg::LockExclusiveNonblock).map_err(|(_f, errno)| {
                if errno == nix::errno::Errno::EWOULDBLOCK {
                    warn!(?path, "flock EWOULDBLOCK");
                    BootControlError::ConcurrentModification {
                        path: path.display().to_string(),
                    }
                } else {
                    BootControlError::EspScanFailed {
                        reason: format!("flock error: {errno}"),
                    }
                }
            })?;

        let mut content = String::new();
        BufReader::new(&mut *locked)
            .read_to_string(&mut content)
            .map_err(|e| BootControlError::EspScanFailed {
                reason: format!("read error: {e}"),
            })?;

        if !expected_etag.is_empty() && !verify_etag(expected_etag, content.as_bytes()) {
            let actual = compute_etag_str(&content);
            warn!(provided = expected_etag, actual = %actual, "ETag mismatch");
            return Err(BootControlError::StateMismatch {
                expected: expected_etag.to_string(),
                actual,
            });
        }
    }

    // Write temp file and rename atomically.
    let tmp_path = build_tmp_path(path);
    write_atomically(&tmp_path, path, new_content)
}

fn build_tmp_path(target: &Path) -> PathBuf {
    let name = target
        .file_name()
        .unwrap_or_else(|| std::ffi::OsStr::new("entry"))
        .to_string_lossy();
    target
        .parent()
        .unwrap_or(Path::new("/boot/loader"))
        .join(format!("{name}.bootcontrol.tmp"))
}

fn write_atomically(tmp_path: &Path, target: &Path, content: &str) -> Result<(), BootControlError> {
    let mut tmp = File::create(tmp_path).map_err(|e| BootControlError::EspScanFailed {
        reason: format!("create tmp file failed: {e}"),
    })?;
    tmp.write_all(content.as_bytes())
        .map_err(|e| BootControlError::EspScanFailed {
            reason: format!("write tmp file failed: {e}"),
        })?;
    tmp.sync_all().map_err(|e| BootControlError::EspScanFailed {
        reason: format!("fsync failed: {e}"),
    })?;
    drop(tmp);
    fs::rename(tmp_path, target).map_err(|e| BootControlError::EspScanFailed {
        reason: format!("atomic rename failed: {e}"),
    })?;
    info!(?target, "atomic write complete");
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_entry(dir: &Path, id: &str, content: &str) {
        fs::write(dir.join(format!("{id}.conf")), content).expect("write entry");
    }

    const ARCH_ENTRY: &str = "\
title   Arch Linux
linux   /vmlinuz-linux
initrd  /initramfs-linux.img
options root=/dev/sda1 rw quiet
";

    const FALLBACK_ENTRY: &str = "\
title   Arch Linux (fallback)
linux   /vmlinuz-linux
initrd  /initramfs-linux-fallback.img
options root=/dev/sda1 rw
";

    const LOADER_CONF: &str = "timeout 5\ndefault arch\n";

    // ── test helpers ─────────────────────────────────────────────────────────

    /// Create a temp layout mirroring the real filesystem:
    /// `loader/` (parent) and `loader/entries/` (entries dir).
    fn make_loader_layout() -> (TempDir, std::path::PathBuf, std::path::PathBuf) {
        let root = TempDir::new().unwrap();
        let entries_dir = root.path().join("entries");
        fs::create_dir_all(&entries_dir).unwrap();
        let loader_conf = root.path().join("loader.conf");
        (root, entries_dir, loader_conf)
    }

    // ── read_all_entries ──────────────────────────────────────────────────────

    #[test]
    fn read_all_entries_returns_sorted_records() {
        let (_root, entries_dir, loader_conf) = make_loader_layout();
        write_entry(&entries_dir, "arch", ARCH_ENTRY);
        write_entry(&entries_dir, "fallback", FALLBACK_ENTRY);
        fs::write(&loader_conf, LOADER_CONF).unwrap();

        let records = read_all_entries(&entries_dir, &loader_conf).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].id, "arch");
        assert_eq!(records[1].id, "fallback");
    }

    #[test]
    fn read_all_entries_marks_default_correctly() {
        let (_root, entries_dir, loader_conf) = make_loader_layout();
        write_entry(&entries_dir, "arch", ARCH_ENTRY);
        write_entry(&entries_dir, "fallback", FALLBACK_ENTRY);
        fs::write(&loader_conf, LOADER_CONF).unwrap();

        let records = read_all_entries(&entries_dir, &loader_conf).unwrap();
        let arch = records.iter().find(|r| r.id == "arch").unwrap();
        let fallback = records.iter().find(|r| r.id == "fallback").unwrap();
        assert!(arch.is_default);
        assert!(!fallback.is_default);
    }

    #[test]
    fn read_all_entries_skips_non_conf_files() {
        let (_root, entries_dir, loader_conf) = make_loader_layout();
        write_entry(&entries_dir, "arch", ARCH_ENTRY);
        fs::write(entries_dir.join("README.txt"), "ignored").unwrap();
        fs::write(&loader_conf, "").unwrap();

        let records = read_all_entries(&entries_dir, &loader_conf).unwrap();
        assert_eq!(records.len(), 1);
    }

    // ── is_entry_default ─────────────────────────────────────────────────────

    #[test]
    fn is_entry_default_bare_stem() {
        assert!(is_entry_default("arch", "arch"));
    }

    #[test]
    fn is_entry_default_with_conf_suffix() {
        assert!(is_entry_default("arch", "arch.conf"));
    }

    #[test]
    fn is_entry_default_no_match() {
        assert!(!is_entry_default("arch", "fallback"));
    }

    // ── set_loader_default ────────────────────────────────────────────────────

    #[test]
    fn set_loader_default_updates_existing_default() {
        let dir = TempDir::new().unwrap();
        let loader_conf = dir.path().join("loader.conf");
        fs::write(&loader_conf, LOADER_CONF).unwrap();

        let etag = fetch_loader_conf_etag(&loader_conf).unwrap();
        set_loader_default(&loader_conf, "fallback", &etag).unwrap();

        let new_default = read_loader_conf_default(&loader_conf).unwrap();
        assert_eq!(new_default, "fallback");
    }

    #[test]
    fn set_loader_default_rejects_stale_etag() {
        let dir = TempDir::new().unwrap();
        let loader_conf = dir.path().join("loader.conf");
        fs::write(&loader_conf, LOADER_CONF).unwrap();

        let result = set_loader_default(&loader_conf, "fallback", "stale-etag-xyz");
        assert!(matches!(result, Err(BootControlError::StateMismatch { .. })));
    }

    // ── write_loader_entry ────────────────────────────────────────────────────

    #[test]
    fn write_loader_entry_creates_new_file() {
        let dir = TempDir::new().unwrap();
        let entry = parse_loader_entry(ARCH_ENTRY).unwrap();
        write_loader_entry(dir.path(), "arch", &entry, "").unwrap();

        let path = dir.path().join("arch.conf");
        assert!(path.exists());
        let content = fs::read_to_string(path).unwrap();
        assert!(content.contains("Arch Linux"));
    }

    #[test]
    fn write_loader_entry_rejects_path_traversal() {
        let dir = TempDir::new().unwrap();
        let entry = parse_loader_entry(ARCH_ENTRY).unwrap();
        let result = write_loader_entry(dir.path(), "../evil", &entry, "");
        assert!(matches!(result, Err(BootControlError::MalformedValue { .. })));
    }

    // ── update_or_append_key ─────────────────────────────────────────────────

    #[test]
    fn update_or_append_key_updates_existing() {
        let content = "timeout 5\ndefault arch\n";
        let result = update_or_append_key(content, "default", "fallback");
        assert!(result.contains("default fallback"));
        assert!(!result.contains("default arch\n"));
    }

    #[test]
    fn update_or_append_key_appends_new() {
        let content = "timeout 5\n";
        let result = update_or_append_key(content, "default", "arch");
        assert!(result.contains("default arch"));
    }
}
