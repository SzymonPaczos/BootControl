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
use tracing::{error, info, warn};

use crate::{failsafe, grub_rebuild};

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
/// After the atomic write succeeds this function:
/// 1. Calls [`failsafe::refresh_failsafe_entry`] to regenerate the
///    golden-parachute GRUB snippet (Step 8).
/// 2. Calls [`grub_rebuild::run_grub_mkconfig`] to regenerate
///    `/boot/grub/grub.cfg` so the change becomes active at the next boot
///    (Step 9).
///
/// # Arguments
///
/// * `path`         — Path to `/etc/default/grub` (injectable for tests).
/// * `key`          — The GRUB variable name to set (e.g. `"GRUB_TIMEOUT"`).
/// * `value`        — The new value. Pass without surrounding quotes; this
///   function will quote multi-word values automatically where needed.
/// * `etag`         — The ETag the caller received from the last
///   `ReadGrubConfig` or `GetEtag` call. Must match the current file's ETag.
/// * `failsafe_cfg` — Path where the failsafe GRUB snippet is written.
///   Production code passes `/etc/bootcontrol/failsafe.cfg`; tests pass a
///   path inside a `tempfile::TempDir`.
/// * `grub_cfg_path` — Destination for the regenerated `grub.cfg`.
///   Production code passes `/boot/grub/grub.cfg`; tests inject a path
///   inside a `tempfile::TempDir` so `grub-mkconfig` (if present) writes
///   there instead of touching the live boot partition.
///
/// # Errors
///
/// - [`BootControlError::EspScanFailed`] — file read/write I/O error,
///   failsafe entry write failure, or `grub-mkconfig` execution failure.
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
/// let failsafe = Path::new("/etc/bootcontrol/failsafe.cfg");
/// let grub_cfg = Path::new("/boot/grub/grub.cfg");
/// let (_, etag) = read_grub_config(path).unwrap();
/// set_grub_value(path, "GRUB_TIMEOUT", "10", &etag, failsafe, grub_cfg).unwrap();
/// ```
pub fn set_grub_value(
    path: &Path,
    key: &str,
    value: &str,
    etag: &str,
    failsafe_cfg: &Path,
    grub_cfg_path: &Path,
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
    let mut locked =
        Flock::lock(lock_file, FlockArg::LockExclusiveNonblock).map_err(|(_file, errno)| {
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
        })?;

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
    match config
        .lines
        .iter()
        .rposition(|l| is_assignment_for_key(l, key))
    {
        Some(idx) => config.lines[idx] = new_line,
        None => config.lines.push(new_line),
    }
    config.map.insert(key.to_string(), value.to_string());

    // ── Step 7: Atomic write via temp file + rename ───────────────────────
    let tmp_path = build_tmp_path(path);
    write_file_atomically(&tmp_path, path, &config.lines)?;

    // ── Step 8: Refresh golden-parachute failsafe entry ──────────────────
    //
    // Done AFTER the successful atomic write so that a failed failsafe write
    // does not roll back the user's intended change. The failsafe entry is
    // best-effort from the user's perspective, but we propagate the error so
    // callers can log / surface it if the output directory is inaccessible.
    info!(key, "GRUB config updated — refreshing failsafe entry");
    failsafe::refresh_failsafe_entry(failsafe_cfg)?;

    // ── Step 9: Regenerate /boot/grub/grub.cfg via grub-mkconfig ─────────
    //
    // The atomic write above only updated /etc/default/grub. GRUB reads
    // /boot/grub/grub.cfg at boot time. Without this call the user's change
    // would be written to disk but would never take effect at the next boot.
    //
    // run_grub_mkconfig is called AFTER the failsafe refresh so both
    // safeguards are in place before the live boot config is regenerated.
    info!(key, grub_cfg = ?grub_cfg_path, "triggering grub-mkconfig");
    grub_rebuild::run_grub_mkconfig(grub_cfg_path)?;

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

    tmp_file.write_all(new_content.as_bytes()).map_err(|e| {
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
    use tempfile::{NamedTempFile, TempDir};

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn write_temp(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().expect("tempfile create");
        f.write_all(content.as_bytes()).expect("tempfile write");
        f.flush().expect("tempfile flush");
        f
    }

    /// Create a temporary directory containing a `grub-mkconfig` stub that
    /// immediately exits 0, set `PATH` to that directory, and return a guard
    /// that holds the `PATH_LOCK`.
    ///
    /// Both the `TempDir` (keeps the fake binary alive) and the `MutexGuard`
    /// (prevents concurrent PATH manipulation by other tests) must be kept
    /// alive for the duration of the test.  When they are dropped, `PATH` is
    /// restored and the lock is released.
    ///
    /// # Returns
    ///
    /// A tuple of `(TempDir, MutexGuard)`.  Bind both to `_` prefixed names
    /// so Rust does not drop them before the assertion:
    ///
    /// ```text
    /// let (_bin_dir, _guard) = setup_fake_grub_mkconfig();
    /// ```
    fn setup_fake_grub_mkconfig() -> (TempDir, std::sync::MutexGuard<'static, ()>) {
        use std::os::unix::fs::PermissionsExt;

        // Acquire the global PATH lock FIRST so no concurrent test can observe
        // the changed PATH between set_var and the actual command execution.
        let guard = crate::grub_rebuild::tests::PATH_LOCK
            .lock()
            .expect("PATH lock poisoned");

        let dir = TempDir::new().expect("fake grub-mkconfig tempdir");
        let script_path = dir.path().join("grub-mkconfig");

        let mut f = std::fs::File::create(&script_path).expect("create fake grub-mkconfig");
        writeln!(f, "#!/bin/sh").expect("write shebang");
        writeln!(f, "exit 0").expect("write body");
        drop(f);

        let mut perms = std::fs::metadata(&script_path)
            .expect("metadata")
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&script_path, perms).expect("chmod");

        std::env::set_var("PATH", dir.path());
        (dir, guard)
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

    /// A stale ETag must be rejected before any write happens — grub-mkconfig
    /// is never reached, so no fake binary is needed for this test.
    #[test]
    fn set_grub_value_rejects_stale_etag() {
        let f = write_temp(SIMPLE_GRUB);
        let failsafe_dir = tempfile::tempdir().expect("failsafe tempdir");
        let failsafe_path = failsafe_dir.path().join("failsafe.cfg");
        let grub_cfg_dir = tempfile::tempdir().expect("grub cfg tempdir");
        let grub_cfg_path = grub_cfg_dir.path().join("grub.cfg");
        let stale_etag = compute_etag_str("completely different content\n");
        let result = set_grub_value(
            f.path(),
            "GRUB_TIMEOUT",
            "10",
            &stale_etag,
            &failsafe_path,
            &grub_cfg_path,
        );
        assert!(
            matches!(result, Err(BootControlError::StateMismatch { .. })),
            "expected StateMismatch, got {result:?}"
        );
    }

    #[test]
    fn set_grub_value_updates_existing_key() {
        let f = write_temp(SIMPLE_GRUB);
        let failsafe_dir = tempfile::tempdir().expect("failsafe tempdir");
        let failsafe_path = failsafe_dir.path().join("failsafe.cfg");
        let grub_cfg_dir = tempfile::tempdir().expect("grub cfg tempdir");
        let grub_cfg_path = grub_cfg_dir.path().join("grub.cfg");
        let etag = compute_etag_str(SIMPLE_GRUB);

        // Provide a fake grub-mkconfig on PATH so Step 9 succeeds.
        let (_fake_bin_dir, _guard) = setup_fake_grub_mkconfig();
        let result = set_grub_value(
            f.path(),
            "GRUB_TIMEOUT",
            "10",
            &etag,
            &failsafe_path,
            &grub_cfg_path,
        );
        std::env::set_var("PATH", "/usr/bin:/bin:/usr/sbin:/sbin");
        result.expect("write should succeed");

        let (map, _) = read_grub_config(f.path()).expect("re-read");
        assert_eq!(map.get("GRUB_TIMEOUT").map(String::as_str), Some("10"));
    }

    #[test]
    fn set_grub_value_appends_new_key() {
        let f = write_temp(SIMPLE_GRUB);
        let failsafe_dir = tempfile::tempdir().expect("failsafe tempdir");
        let failsafe_path = failsafe_dir.path().join("failsafe.cfg");
        let grub_cfg_dir = tempfile::tempdir().expect("grub cfg tempdir");
        let grub_cfg_path = grub_cfg_dir.path().join("grub.cfg");
        let etag = compute_etag_str(SIMPLE_GRUB);

        let (_fake_bin_dir, _guard) = setup_fake_grub_mkconfig();
        let result = set_grub_value(
            f.path(),
            "GRUB_GFXMODE",
            "auto",
            &etag,
            &failsafe_path,
            &grub_cfg_path,
        );
        std::env::set_var("PATH", "/usr/bin:/bin:/usr/sbin:/sbin");
        result.expect("write should succeed");

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
        let failsafe_dir = tempfile::tempdir().expect("failsafe tempdir");
        let failsafe_path = failsafe_dir.path().join("failsafe.cfg");
        let grub_cfg_dir = tempfile::tempdir().expect("grub cfg tempdir");
        let grub_cfg_path = grub_cfg_dir.path().join("grub.cfg");
        let etag = compute_etag_str(content);

        let (_fake_bin_dir, _guard) = setup_fake_grub_mkconfig();
        let result = set_grub_value(
            f.path(),
            "GRUB_TIMEOUT",
            "10",
            &etag,
            &failsafe_path,
            &grub_cfg_path,
        );
        std::env::set_var("PATH", "/usr/bin:/bin:/usr/sbin:/sbin");
        result.expect("write");

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
        let failsafe_dir = tempfile::tempdir().expect("failsafe tempdir");
        let failsafe_path = failsafe_dir.path().join("failsafe.cfg");
        let grub_cfg_dir = tempfile::tempdir().expect("grub cfg tempdir");
        let grub_cfg_path = grub_cfg_dir.path().join("grub.cfg");
        let etag = compute_etag_str(SIMPLE_GRUB);

        let (_fake_bin_dir, _guard) = setup_fake_grub_mkconfig();
        let result = set_grub_value(
            f.path(),
            "GRUB_TIMEOUT",
            "99",
            &etag,
            &failsafe_path,
            &grub_cfg_path,
        );
        std::env::set_var("PATH", "/usr/bin:/bin:/usr/sbin:/sbin");
        result.expect("write");

        let written = fs::read_to_string(f.path()).expect("re-read");
        assert!(
            written.contains("# This is a comment — preserved verbatim"),
            "Comment was not preserved in:\n{written}"
        );
    }

    #[test]
    fn set_grub_value_preserves_other_assignments_verbatim() {
        let f = write_temp(SIMPLE_GRUB);
        let failsafe_dir = tempfile::tempdir().expect("failsafe tempdir");
        let failsafe_path = failsafe_dir.path().join("failsafe.cfg");
        let grub_cfg_dir = tempfile::tempdir().expect("grub cfg tempdir");
        let grub_cfg_path = grub_cfg_dir.path().join("grub.cfg");
        let etag = compute_etag_str(SIMPLE_GRUB);

        let (_fake_bin_dir, _guard) = setup_fake_grub_mkconfig();
        let result = set_grub_value(
            f.path(),
            "GRUB_TIMEOUT",
            "99",
            &etag,
            &failsafe_path,
            &grub_cfg_path,
        );
        std::env::set_var("PATH", "/usr/bin:/bin:/usr/sbin:/sbin");
        result.expect("write");

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

    /// `StateMismatch` is returned before grub-mkconfig is reached, so no
    /// fake binary is needed here.
    #[test]
    fn etag_changes_after_external_modification() {
        let f = write_temp(SIMPLE_GRUB);
        let failsafe_dir = tempfile::tempdir().expect("failsafe tempdir");
        let failsafe_path = failsafe_dir.path().join("failsafe.cfg");
        let grub_cfg_dir = tempfile::tempdir().expect("grub cfg tempdir");
        let grub_cfg_path = grub_cfg_dir.path().join("grub.cfg");
        let etag_before = compute_etag_str(SIMPLE_GRUB);

        // Symulacja zewnętrznej modyfikacji (np. przez apt).
        let modified = SIMPLE_GRUB.replace("GRUB_TIMEOUT=5", "GRUB_TIMEOUT=99");
        fs::write(f.path(), &modified).expect("external write");

        let etag_after = compute_etag_str(&modified);
        assert_ne!(etag_before, etag_after, "ETag musi się zmienić po mutacji");

        // Próba zapisu ze starym ETag musi się nie udać.
        let result = set_grub_value(
            f.path(),
            "GRUB_DEFAULT",
            "1",
            &etag_before,
            &failsafe_path,
            &grub_cfg_path,
        );
        assert!(matches!(
            result,
            Err(BootControlError::StateMismatch { .. })
        ));
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
