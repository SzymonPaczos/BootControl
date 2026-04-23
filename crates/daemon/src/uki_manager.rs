//! Filesystem operations for UKI kernel command-line management.
//!
//! Manages `/etc/kernel/cmdline` — the single-line file that systemd-boot and
//! UKI builds read to assemble the kernel command line.
//!
//! All writes use the same atomic-rename + flock pattern as [`crate::grub_manager`].

use std::{
    fs::{self, File, OpenOptions},
    io::{BufReader, Read, Write},
    path::Path,
};

use bootcontrol_core::{
    backends::uki::{add_param, parse_cmdline, remove_param, validate_kernel_param},
    error::BootControlError,
    hash::{compute_etag_str, verify_etag},
};
use nix::fcntl::{Flock, FlockArg};
use tracing::{error, info, warn};

// ─────────────────────────────────────────────────────────────────────────────
// Read operations
// ─────────────────────────────────────────────────────────────────────────────

/// Read `/etc/kernel/cmdline` and return `(params, etag)`.
///
/// `params` is a sorted list of individual whitespace-delimited tokens.
///
/// # Errors
///
/// - [`BootControlError::EspScanFailed`] if the file cannot be read.
pub fn read_kernel_cmdline(path: &Path) -> Result<(Vec<String>, String), BootControlError> {
    let content = fs::read_to_string(path).map_err(|e| {
        error!(?path, io_error = %e, "failed to read kernel cmdline");
        BootControlError::EspScanFailed {
            reason: format!("cannot read kernel cmdline: {e}"),
        }
    })?;
    let etag = compute_etag_str(&content);
    let params = parse_cmdline(&content);
    Ok((params, etag))
}

/// Return only the ETag of the current on-disk cmdline file.
pub fn fetch_cmdline_etag(path: &Path) -> Result<String, BootControlError> {
    let content = fs::read_to_string(path).map_err(|e| {
        BootControlError::EspScanFailed {
            reason: format!("cannot read kernel cmdline for ETag: {e}"),
        }
    })?;
    Ok(compute_etag_str(&content))
}

// ─────────────────────────────────────────────────────────────────────────────
// Write operations
// ─────────────────────────────────────────────────────────────────────────────

/// Add a kernel parameter to `/etc/kernel/cmdline`.
///
/// Idempotent: if `param` is already present, this is a no-op.
/// `expected_etag` must match the current file's ETag.
///
/// Security: `param` is validated against the blacklist in
/// [`bootcontrol_core::backends::uki::validate_kernel_param`] before any write.
///
/// # Errors
///
/// - [`BootControlError::SecurityPolicyViolation`] — blacklisted pattern.
/// - [`BootControlError::ConcurrentModification`] — another process holds the lock.
/// - [`BootControlError::StateMismatch`] — stale ETag.
/// - [`BootControlError::EspScanFailed`] — I/O error.
pub fn add_kernel_param(
    path: &Path,
    param: &str,
    expected_etag: &str,
) -> Result<(), BootControlError> {
    validate_kernel_param(param)?;
    atomic_cmdline_update(path, expected_etag, |content| add_param(content, param))
}

/// Remove a kernel parameter from `/etc/kernel/cmdline`.
///
/// `expected_etag` must match the current file's ETag.
///
/// # Errors
///
/// - [`BootControlError::KeyNotFound`] — `param` is not present.
/// - [`BootControlError::ConcurrentModification`] — another process holds the lock.
/// - [`BootControlError::StateMismatch`] — stale ETag.
/// - [`BootControlError::EspScanFailed`] — I/O error.
pub fn remove_kernel_param(
    path: &Path,
    param: &str,
    expected_etag: &str,
) -> Result<(), BootControlError> {
    atomic_cmdline_update(path, expected_etag, |content| remove_param(content, param))
}

// ─────────────────────────────────────────────────────────────────────────────
// Private helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Generic atomic update of a cmdline file.
///
/// Opens the file, acquires an exclusive flock, verifies the ETag, applies
/// `transform` to produce the new content, then writes atomically.
fn atomic_cmdline_update<F>(
    path: &Path,
    expected_etag: &str,
    transform: F,
) -> Result<(), BootControlError>
where
    F: FnOnce(&str) -> Result<String, BootControlError>,
{
    // ── Step 1: Open ──────────────────────────────────────────────────────────
    let lock_file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(|e| {
            error!(?path, io_error = %e, "failed to open cmdline for locking");
            BootControlError::EspScanFailed {
                reason: format!("open failed: {e}"),
            }
        })?;

    // ── Step 2: flock ─────────────────────────────────────────────────────────
    let mut locked =
        Flock::lock(lock_file, FlockArg::LockExclusiveNonblock).map_err(|(_f, errno)| {
            if errno == nix::errno::Errno::EWOULDBLOCK {
                warn!(?path, "flock EWOULDBLOCK on cmdline");
                BootControlError::ConcurrentModification {
                    path: path.display().to_string(),
                }
            } else {
                BootControlError::EspScanFailed {
                    reason: format!("flock error: {errno}"),
                }
            }
        })?;

    // ── Step 3: Read through locked fd ───────────────────────────────────────
    let mut content = String::new();
    BufReader::new(&mut *locked)
        .read_to_string(&mut content)
        .map_err(|e| BootControlError::EspScanFailed {
            reason: format!("read error: {e}"),
        })?;

    // ── Step 4: ETag verification (under lock) ────────────────────────────────
    if !verify_etag(expected_etag, content.as_bytes()) {
        let actual = compute_etag_str(&content);
        warn!(provided = expected_etag, actual = %actual, "cmdline ETag mismatch");
        return Err(BootControlError::StateMismatch {
            expected: expected_etag.to_string(),
            actual,
        });
    }

    // ── Step 5: Transform ─────────────────────────────────────────────────────
    let new_content = transform(&content)?;

    // ── Step 6: Atomic write ──────────────────────────────────────────────────
    let tmp_path = path
        .parent()
        .unwrap_or(Path::new("/etc/kernel"))
        .join("cmdline.bootcontrol.tmp");

    let mut tmp = File::create(&tmp_path).map_err(|e| BootControlError::EspScanFailed {
        reason: format!("create tmp file failed: {e}"),
    })?;
    tmp.write_all(new_content.as_bytes())
        .map_err(|e| BootControlError::EspScanFailed {
            reason: format!("write tmp file failed: {e}"),
        })?;
    tmp.sync_all()
        .map_err(|e| BootControlError::EspScanFailed {
            reason: format!("fsync failed: {e}"),
        })?;
    drop(tmp);
    fs::rename(&tmp_path, path).map_err(|e| BootControlError::EspScanFailed {
        reason: format!("atomic rename failed: {e}"),
    })?;

    info!(?path, "kernel cmdline updated");
    Ok(())
    // `locked` drops here → flock released automatically.
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use bootcontrol_core::hash::compute_etag_str;
    use tempfile::NamedTempFile;

    fn write_temp(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().expect("tempfile");
        f.write_all(content.as_bytes()).expect("write");
        f.flush().expect("flush");
        f
    }

    const CMDLINE: &str = "root=/dev/sda1 rw quiet splash\n";

    // ── read_kernel_cmdline ───────────────────────────────────────────────────

    #[test]
    fn read_returns_parsed_params_and_64char_etag() {
        let f = write_temp(CMDLINE);
        let (params, etag) = read_kernel_cmdline(f.path()).unwrap();
        assert_eq!(etag.len(), 64);
        assert!(params.contains(&"quiet".to_string()));
        assert!(params.contains(&"root=/dev/sda1".to_string()));
    }

    // ── add_kernel_param ──────────────────────────────────────────────────────

    #[test]
    fn add_param_appends_to_cmdline() {
        let f = write_temp(CMDLINE);
        let etag = compute_etag_str(CMDLINE);
        add_kernel_param(f.path(), "loglevel=3", &etag).unwrap();

        let (params, _) = read_kernel_cmdline(f.path()).unwrap();
        assert!(params.contains(&"loglevel=3".to_string()));
    }

    #[test]
    fn add_param_rejects_stale_etag() {
        let f = write_temp(CMDLINE);
        let result = add_kernel_param(f.path(), "loglevel=3", "stale-etag");
        assert!(matches!(result, Err(BootControlError::StateMismatch { .. })));
    }

    #[test]
    fn add_param_rejects_blacklisted_param() {
        let f = write_temp(CMDLINE);
        let etag = compute_etag_str(CMDLINE);
        let result = add_kernel_param(f.path(), "selinux=0", &etag);
        assert!(matches!(
            result,
            Err(BootControlError::SecurityPolicyViolation { .. })
        ));
    }

    // ── remove_kernel_param ───────────────────────────────────────────────────

    #[test]
    fn remove_param_removes_from_cmdline() {
        let f = write_temp(CMDLINE);
        let etag = compute_etag_str(CMDLINE);
        remove_kernel_param(f.path(), "quiet", &etag).unwrap();

        let (params, _) = read_kernel_cmdline(f.path()).unwrap();
        assert!(!params.contains(&"quiet".to_string()));
    }

    #[test]
    fn remove_param_rejects_stale_etag() {
        let f = write_temp(CMDLINE);
        let result = remove_kernel_param(f.path(), "quiet", "stale-etag");
        assert!(matches!(result, Err(BootControlError::StateMismatch { .. })));
    }
}
