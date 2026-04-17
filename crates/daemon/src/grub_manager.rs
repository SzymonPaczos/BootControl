//! Filesystem operations for GRUB configuration management.
//!
//! This module contains all code that touches `/etc/default/grub`. It is
//! intentionally separated from the D-Bus interface layer so that each piece
//! can be tested independently — the filesystem logic with `tempfile`, the
//! D-Bus logic with a session bus.
//!
//! # Atomic write guarantee
//!
//! Every write follows the sequence:
//!
//! ```text
//! open(path)                     ← file handle (no data read yet)
//!   │
//! flock(LOCK_EX | LOCK_NB)       ← acquire lock FIRST
//!   │  ↳ EWOULDBLOCK → ConcurrentModification
//!   │
//! read through locked fd         ← TOCTOU-safe: no gap between lock and read
//!   │
//! verify_etag(...)               ← check freshness while still under lock
//!   │  ↳ mismatch → StateMismatch
//!   │
//! parse + reconstruct lines      ← comment-preserving edit
//!   │
//! write .bootcontrol.tmp         ← sibling file, same filesystem
//! fsync(.tmp)                    ← flush to persistent storage
//! rename(.tmp → path)            ← atomic on ext4/btrfs/xfs/tmpfs
//!   │
//! drop flock (implicit via Drop) ← lock released after rename
//! ```
//!
//! # TOCTOU rationale
//!
//! The lock is acquired **before** reading the file content. This closes the
//! window where `apt` or `pacman` could modify the file between our ETag check
//! and our write. Without this ordering, a concurrent package manager could
//! corrupt the file even if the ETag matched at validation time.
//!
//! [`ConcurrentModification`]: bootcontrol_core::error::BootControlError::ConcurrentModification

use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::{BufReader, Read, Write},
    path::{Path, PathBuf},
};

use bootcontrol_core::{
    error::BootControlError,
    grub::parse_grub_config,
    hash::{compute_etag_str, verify_etag},
};
use nix::fcntl::{Flock, FlockArg};
use tracing::{error, warn};

/// Read `/etc/default/grub` (or any path), parse it, and compute its ETag.
///
/// # Arguments
///
/// * `path` — Path to the GRUB default config file.
///
/// # Errors
///
/// - [`BootControlError::EspScanFailed`] if the file cannot be read.
/// - [`BootControlError::ComplexBashDetected`] if the file contains unsupported
///   Bash constructs.
///
/// # Examples
///
/// ```no_run
/// use bootcontrold::grub_manager::read_grub_config;
/// use std::path::Path;
///
/// let (map, etag) = read_grub_config(Path::new("/etc/default/grub")).unwrap();
/// assert_eq!(etag.len(), 64);
/// ```
pub fn read_grub_config(
    path: &Path,
) -> Result<(HashMap<String, String>, String), BootControlError> {
    let content = fs::read_to_string(path).map_err(|e| {
        error!(?path, io_error = %e, "failed to read GRUB config");
        BootControlError::EspScanFailed {
            reason: e.to_string(),
        }
    })?;

    let config = parse_grub_config(&content)?;
    let etag = compute_etag_str(&content);

    Ok((config.map, etag))
}

/// Return only the ETag of the current on-disk GRUB config.
///
/// Convenience function for clients that need to refresh their ETag after an
/// external change without reading the full key-value map.
///
/// # Arguments
///
/// * `path` — Path to the GRUB default config file.
///
/// # Errors
///
/// - [`BootControlError::EspScanFailed`] if the file cannot be read.
///
/// # Examples
///
/// ```no_run
/// use bootcontrold::grub_manager::fetch_etag;
/// use std::path::Path;
///
/// let etag = fetch_etag(Path::new("/etc/default/grub")).unwrap();
/// assert_eq!(etag.len(), 64);
/// ```
pub fn fetch_etag(path: &Path) -> Result<String, BootControlError> {
    let content = fs::read_to_string(path).map_err(|e| {
        error!(?path, io_error = %e, "failed to read GRUB config for ETag");
        BootControlError::EspScanFailed {
            reason: e.to_string(),
        }
    })?;
    Ok(compute_etag_str(&content))
}

/// Atomically set a key-value pair in `/etc/default/grub`.
///
/// Implements the full secure write pipeline with TOCTOU protection:
/// flock is acquired **before** reading the file content, eliminating the
/// race window between ETag check and write.
///
/// # Arguments
///
/// * `path`  — Path to `/etc/default/grub` (injectable for tests).
/// * `key`   — The GRUB variable name to set (e.g. `"GRUB_TIMEOUT"`).
/// * `value` — The new value. Pass without surrounding quotes; this function
///   will quote multi-word values automatically where needed.
/// * `etag`  — The ETag the caller received from the last `ReadGrubConfig` or
///   `GetEtag` call. Must match the current file's ETag.
///
/// # Errors
///
/// - [`BootControlError::EspScanFailed`] — file read/write I/O error.
/// - [`BootControlError::ConcurrentModification`] — another process holds an
///   exclusive lock (`flock EWOULDBLOCK`). This is checked **before** reading.
/// - [`BootControlError::StateMismatch`] — the provided ETag does not match
///   the current on-disk file (checked after acquiring the lock).
/// - [`BootControlError::ComplexBashDetected`] — the current on-disk file
///   contains Bash constructs; BootControl refuses to touch it.
///
/// # Examples
///
/// ```no_run
/// use bootcontrold::grub_manager::{read_grub_config, set_grub_value};
/// use std::path::Path;
///
/// let path = Path::new("/etc/default/grub");
/// let (_, etag) = read_grub_config(path).unwrap();
/// set_grub_value(path, "GRUB_TIMEOUT", "10", &etag).unwrap();
/// ```
pub fn set_grub_value(
    path: &Path,
    key: &str,
    value: &str,
    etag: &str,
) -> Result<(), BootControlError> {
    // ── Step 1: Open file handle ────────────────────────────────────────────
    // We open the file BEFORE acquiring the lock so we have a fd to flock on.
    // No file content is read yet — the lock must come first to prevent TOCTOU.
    let lock_file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(|e| {
            error!(?path, io_error = %e, "failed to open GRUB config for locking");
            BootControlError::EspScanFailed {
                reason: e.to_string(),
            }
        })?;

    // ── Step 2: Acquire exclusive non-blocking flock BEFORE reading ──────────
    //
    // TOCTOU safety: the lock is held from this point until the end of the
    // function (when `locked` is dropped). No other BootControl instance or
    // package manager can modify the file while we hold it.
    let mut locked = Flock::lock(lock_file, FlockArg::LockExclusiveNonblock).map_err(
        |(_file, errno)| {
            if errno == nix::errno::Errno::EWOULDBLOCK {
                warn!(?path, "flock EWOULDBLOCK — package manager holds the lock");
                BootControlError::ConcurrentModification {
                    path: path.display().to_string(),
                }
            } else {
                error!(?path, ?errno, "unexpected flock error");
                BootControlError::EspScanFailed {
                    reason: format!("flock error: {errno}"),
                }
            }
        },
    )?;

    // ── Step 3: Read content THROUGH the locked file handle ─────────────────
    //
    // Reading through the locked fd (not a new open()) guarantees we see the
    // file state that we have locked — no TOCTOU window.
    let mut content = String::new();
    BufReader::new(&mut *locked)
        .read_to_string(&mut content)
        .map_err(|e| {
            error!(?path, io_error = %e, "failed to read GRUB config through locked fd");
            BootControlError::EspScanFailed {
                reason: e.to_string(),
            }
        })?;
    let content_bytes = content.as_bytes();

    // ── Step 4: ETag verification (under lock) ──────────────────────────────
    if !verify_etag(etag, content_bytes) {
        let actual = compute_etag_str(&content);
        warn!(
            provided_etag = etag,
            actual_etag = %actual,
            "ETag mismatch — file was modified before lock was acquired"
        );
        return Err(BootControlError::StateMismatch {
            expected: etag.to_string(),
            actual,
        });
    }

    // ── Step 5: Parse current config (comment-preserving) ───────────────────
    let mut config = parse_grub_config(&content)?;

    // ── Step 6: Reconstruct lines — update LAST occurrence of the key ────────
    //
    // The GRUB parser uses "last assignment wins" semantics (like `source`).
    // We must update the **last** matching line to stay consistent with what
    // the shell would actually see after sourcing the file. Updating only the
    // first occurrence while a later duplicate exists would leave a stale value
    // as the effective one.
    let new_line = build_assignment_line(key, value);
    match config.lines.iter().rposition(|l| is_assignment_for_key(l, key)) {
        Some(idx) => config.lines[idx] = new_line,
        None => config.lines.push(new_line),
    }
    config.map.insert(key.to_string(), value.to_string());

    // ── Step 7: Atomic write via temp file + rename ───────────────────────
    let tmp_path = build_tmp_path(path);
    write_file_atomically(&tmp_path, path, &config.lines)?;

    Ok(())
    // `locked` drops here → flock released automatically via Drop.
}

// ─────────────────────────────────────────────────────────────────────────────
// Private helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Build the temporary file path for an atomic write.
///
/// The temp file is placed in the same directory as the target to ensure the
/// `rename()` syscall is atomic (both files on the same filesystem/mount).
fn build_tmp_path(target: &Path) -> PathBuf {
    let file_name = target
        .file_name()
        .unwrap_or_else(|| std::ffi::OsStr::new("grub"))
        .to_string_lossy();
    target
        .parent()
        .unwrap_or_else(|| Path::new("/etc/default"))
        .join(format!("{file_name}.bootcontrol.tmp"))
}

/// Determine whether a line string is the `KEY=...` assignment for `key`.
///
/// Strips leading whitespace and checks for `KEY=` prefix so that indented
/// assignments (non-standard but harmless) are also matched.
fn is_assignment_for_key(line: &str, key: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with(&format!("{key}="))
}

/// Build a canonical `KEY=VALUE` or `KEY="VALUE"` assignment string.
///
/// Values containing whitespace or empty values are double-quoted
/// automatically. Values without whitespace are left unquoted to match
/// GRUB conventions (`GRUB_TIMEOUT=5`, not `GRUB_TIMEOUT="5"`).
fn build_assignment_line(key: &str, value: &str) -> String {
    if value.contains(' ') || value.contains('\t') || value.is_empty() {
        format!(r#"{key}="{value}""#)
    } else {
        format!("{key}={value}")
    }
}

/// Write `lines` to `tmp_path`, fsync the file, then atomically rename it to
/// `target`.
fn write_file_atomically(
    tmp_path: &Path,
    target: &Path,
    lines: &[String],
) -> Result<(), BootControlError> {
    // Reconstruct the file content from lines, using '\n' as separator.
    // A trailing newline is appended to follow POSIX text file convention.
    let new_content = lines.join("\n") + "\n";

    let mut tmp_file = File::create(tmp_path).map_err(|e| {
        error!(?tmp_path, io_error = %e, "failed to create temp file for atomic write");
        BootControlError::EspScanFailed {
            reason: format!("temp file creation failed: {e}"),
        }
    })?;

    tmp_file
        .write_all(new_content.as_bytes())
        .map_err(|e| {
            error!(?tmp_path, io_error = %e, "failed to write temp file");
            BootControlError::EspScanFailed {
                reason: format!("temp file write failed: {e}"),
            }
        })?;

    // fsync ensures the data reaches persistent storage before the rename.
    tmp_file.sync_all().map_err(|e| {
        error!(?tmp_path, io_error = %e, "fsync failed on temp file");
        BootControlError::EspScanFailed {
            reason: format!("fsync failed: {e}"),
        }
    })?;

    // Drop the file handle before rename so metadata is flushed on all kernels.
    drop(tmp_file);

    // Atomic rename — on Linux this is guaranteed to be atomic within the same
    // filesystem (ext4, btrfs, xfs, tmpfs all honour this).
    fs::rename(tmp_path, target).map_err(|e| {
        error!(?tmp_path, ?target, io_error = %e, "atomic rename failed");
        BootControlError::EspScanFailed {
            reason: format!("atomic rename failed: {e}"),
        }
    })?;

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use bootcontrol_core::hash::compute_etag_str;
    use tempfile::NamedTempFile;

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn write_temp(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().expect("tempfile create");
        f.write_all(content.as_bytes()).expect("tempfile write");
        f.flush().expect("tempfile flush");
        f
    }

    const SIMPLE_GRUB: &str = "\
# This is a comment — preserved verbatim
GRUB_DEFAULT=0
GRUB_TIMEOUT=5
GRUB_CMDLINE_LINUX_DEFAULT=\"quiet splash\"
GRUB_DISTRIBUTOR=\"Ubuntu\"
";

    // ── read_grub_config ──────────────────────────────────────────────────────

    #[test]
    fn read_grub_config_returns_map_and_64_char_etag() {
        let f = write_temp(SIMPLE_GRUB);
        let (map, etag) = read_grub_config(f.path()).expect("read should succeed");
        assert_eq!(etag.len(), 64);
        assert_eq!(map.get("GRUB_TIMEOUT").map(String::as_str), Some("5"));
        assert_eq!(
            map.get("GRUB_CMDLINE_LINUX_DEFAULT").map(String::as_str),
            Some("quiet splash")
        );
    }

    #[test]
    fn read_grub_config_missing_file_returns_esp_scan_failed() {
        let result = read_grub_config(Path::new("/nonexistent/path/grub"));
        assert!(matches!(
            result,
            Err(BootControlError::EspScanFailed { .. })
        ));
    }

    // ── fetch_etag ────────────────────────────────────────────────────────────

    #[test]
    fn fetch_etag_matches_compute_etag_str() {
        let f = write_temp(SIMPLE_GRUB);
        let etag = fetch_etag(f.path()).expect("fetch_etag should succeed");
        assert_eq!(etag, compute_etag_str(SIMPLE_GRUB));
    }

    // ── set_grub_value — ETag verification ───────────────────────────────────

    #[test]
    fn set_grub_value_rejects_stale_etag() {
        let f = write_temp(SIMPLE_GRUB);
        let stale_etag = compute_etag_str("completely different content\n");
        let result = set_grub_value(f.path(), "GRUB_TIMEOUT", "10", &stale_etag);
        assert!(
            matches!(result, Err(BootControlError::StateMismatch { .. })),
            "expected StateMismatch, got {result:?}"
        );
    }

    #[test]
    fn set_grub_value_updates_existing_key() {
        let f = write_temp(SIMPLE_GRUB);
        let etag = compute_etag_str(SIMPLE_GRUB);
        set_grub_value(f.path(), "GRUB_TIMEOUT", "10", &etag).expect("write should succeed");

        let (map, _) = read_grub_config(f.path()).expect("re-read");
        assert_eq!(map.get("GRUB_TIMEOUT").map(String::as_str), Some("10"));
    }

    #[test]
    fn set_grub_value_appends_new_key() {
        let f = write_temp(SIMPLE_GRUB);
        let etag = compute_etag_str(SIMPLE_GRUB);
        set_grub_value(f.path(), "GRUB_GFXMODE", "auto", &etag).expect("write should succeed");

        let (map, _) = read_grub_config(f.path()).expect("re-read");
        assert_eq!(map.get("GRUB_GFXMODE").map(String::as_str), Some("auto"));
    }

    // ── Duplicate-key semantics ───────────────────────────────────────────────

    /// Weryfikacja poprawki błędu: parser używa semantyki "last wins".
    /// set_grub_value musi aktualizować OSTATNIE wystąpienie klucza,
    /// żeby wynik był spójny z tym, co shell odczyta po source'owaniu pliku.
    #[test]
    fn set_grub_value_updates_last_occurrence_of_duplicate_key() {
        // Plik z duplikatem: GRUB_TIMEOUT=5 (pierwsze), GRUB_TIMEOUT=99 (ostatnie).
        // Parser zwróci 99 jako aktywną wartość ("last wins").
        let content = "GRUB_TIMEOUT=5\nGRUB_DEFAULT=0\nGRUB_TIMEOUT=99\n";
        let f = write_temp(content);
        let etag = compute_etag_str(content);

        set_grub_value(f.path(), "GRUB_TIMEOUT", "10", &etag).expect("write");

        let written = fs::read_to_string(f.path()).expect("re-read raw");

        // Po aktualizacji OSTATNIEGO wystąpienia: pierwsze zostaje nienaruszone
        // (GRUB_TIMEOUT=5), ostatnie zmienia się na GRUB_TIMEOUT=10.
        // Shell po source'owaniu zobaczy 10 (ostatnie wygrywa).
        assert!(
            written.contains("GRUB_TIMEOUT=5"),
            "Pierwsze wystąpienie musi zostać nienaruszone:\n{written}"
        );
        assert!(
            written.contains("GRUB_TIMEOUT=10"),
            "Ostatnie wystąpienie musi być zaktualizowane:\n{written}"
        );
        assert!(
            !written.contains("GRUB_TIMEOUT=99"),
            "Stara wartość ostatniego wystąpienia nie może pozostać:\n{written}"
        );

        // Parser potwierdza, że efektywna wartość to 10.
        let (map, _) = read_grub_config(f.path()).expect("re-parse");
        assert_eq!(map.get("GRUB_TIMEOUT").map(String::as_str), Some("10"));
    }

    // ── Comment preservation ──────────────────────────────────────────────────

    #[test]
    fn set_grub_value_preserves_comments_exactly() {
        let f = write_temp(SIMPLE_GRUB);
        let etag = compute_etag_str(SIMPLE_GRUB);
        set_grub_value(f.path(), "GRUB_TIMEOUT", "99", &etag).expect("write");

        let written = fs::read_to_string(f.path()).expect("re-read");
        assert!(
            written.contains("# This is a comment — preserved verbatim"),
            "Comment was not preserved in:\n{written}"
        );
    }

    #[test]
    fn set_grub_value_preserves_other_assignments_verbatim() {
        let f = write_temp(SIMPLE_GRUB);
        let etag = compute_etag_str(SIMPLE_GRUB);
        set_grub_value(f.path(), "GRUB_TIMEOUT", "99", &etag).expect("write");

        let (map, _) = read_grub_config(f.path()).expect("re-read");
        assert_eq!(map.get("GRUB_DEFAULT").map(String::as_str), Some("0"));
        assert_eq!(
            map.get("GRUB_CMDLINE_LINUX_DEFAULT").map(String::as_str),
            Some("quiet splash")
        );
        assert_eq!(
            map.get("GRUB_DISTRIBUTOR").map(String::as_str),
            Some("Ubuntu")
        );
    }

    // ── ETag round-trip ───────────────────────────────────────────────────────

    #[test]
    fn etag_changes_after_external_modification() {
        let f = write_temp(SIMPLE_GRUB);
        let etag_before = compute_etag_str(SIMPLE_GRUB);

        // Symulacja zewnętrznej modyfikacji (np. przez apt).
        let modified = SIMPLE_GRUB.replace("GRUB_TIMEOUT=5", "GRUB_TIMEOUT=99");
        fs::write(f.path(), &modified).expect("external write");

        let etag_after = compute_etag_str(&modified);
        assert_ne!(etag_before, etag_after, "ETag musi się zmienić po mutacji");

        // Próba zapisu ze starym ETag musi się nie udać.
        let result = set_grub_value(f.path(), "GRUB_DEFAULT", "1", &etag_before);
        assert!(matches!(result, Err(BootControlError::StateMismatch { .. })));
    }

    // ── Helper unit tests ─────────────────────────────────────────────────────

    #[test]
    fn is_assignment_for_key_matches_plain() {
        assert!(is_assignment_for_key("GRUB_TIMEOUT=5", "GRUB_TIMEOUT"));
    }

    #[test]
    fn is_assignment_for_key_matches_quoted() {
        assert!(is_assignment_for_key(
            "GRUB_CMDLINE_LINUX_DEFAULT=\"quiet\"",
            "GRUB_CMDLINE_LINUX_DEFAULT"
        ));
    }

    #[test]
    fn is_assignment_for_key_does_not_match_different_key() {
        assert!(!is_assignment_for_key("GRUB_DEFAULT=0", "GRUB_TIMEOUT"));
    }

    #[test]
    fn is_assignment_for_key_does_not_match_comment() {
        assert!(!is_assignment_for_key("# GRUB_TIMEOUT=5", "GRUB_TIMEOUT"));
    }

    #[test]
    fn build_assignment_line_unquotes_simple_value() {
        assert_eq!(build_assignment_line("GRUB_TIMEOUT", "5"), "GRUB_TIMEOUT=5");
    }

    #[test]
    fn build_assignment_line_quotes_value_with_spaces() {
        assert_eq!(
            build_assignment_line("GRUB_CMDLINE_LINUX_DEFAULT", "quiet splash"),
            r#"GRUB_CMDLINE_LINUX_DEFAULT="quiet splash""#
        );
    }

    #[test]
    fn build_assignment_line_quotes_empty_value() {
        assert_eq!(
            build_assignment_line("GRUB_CMDLINE_LINUX", ""),
            r#"GRUB_CMDLINE_LINUX="""#
        );
    }

    #[test]
    fn build_tmp_path_in_same_dir() {
        let target = Path::new("/etc/default/grub");
        let tmp = build_tmp_path(target);
        assert_eq!(tmp.parent(), Some(Path::new("/etc/default")));
        assert!(tmp
            .file_name()
            .unwrap()
            .to_string_lossy()
            .contains("bootcontrol.tmp"));
    }
}
