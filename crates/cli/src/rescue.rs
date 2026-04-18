//! Rescue helper for `bootcontrol rescue`.
//!
//! This module implements a best-effort root filesystem scanner. It reads
//! `/proc/partitions` to enumerate block devices, attempts to mount each
//! candidate partition to a temporary directory, and checks for the presence
//! of `/etc/fstab` as evidence of a root filesystem.
//!
//! # What this module does NOT do
//!
//! - It does **not** perform `chroot` or any bind mount.
//! - It does **not** require elevated privileges for the scan itself (though
//!   the `mount(2)` calls will fail without them, and the code handles this
//!   gracefully by returning `None` for that partition).
//! - All chroot/mount commands are **printed** for the user to run manually.
//!
//! # Testability
//!
//! The core scan logic ([`scan_partitions`]) accepts a `&str` mock input
//! instead of reading `/proc/partitions` directly, enabling unit tests without
//! kernel-level I/O or root access.

use std::{
    fmt,
    path::{Path, PathBuf},
    process::Command,
};

use tracing::{debug, info, warn};

// ─────────────────────────────────────────────────────────────────────────────
// Error type
// ─────────────────────────────────────────────────────────────────────────────

/// Errors produced by the rescue scanner.
///
/// These are CLI-level errors: they are displayed to the user on stderr and
/// translate directly into non-zero exit codes.
#[derive(Debug)]
pub enum RescueError {
    /// `/proc/partitions` could not be read.
    ProcPartitionsUnreadable(std::io::Error),
    /// No partition with `/etc/fstab` was found.
    NoRootFilesystemFound,
}

impl fmt::Display for RescueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RescueError::ProcPartitionsUnreadable(e) => {
                write!(f, "cannot read /proc/partitions: {e}")
            }
            RescueError::NoRootFilesystemFound => {
                write!(
                    f,
                    "no root filesystem found — check that the target disk is present \
                     and that you ran this command with sufficient privileges"
                )
            }
        }
    }
}

impl std::error::Error for RescueError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RescueError::ProcPartitionsUnreadable(e) => Some(e),
            RescueError::NoRootFilesystemFound => None,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

/// Run the rescue scan: find a root filesystem and print chroot instructions.
///
/// Reads `/proc/partitions`, filters out whole-disk entries, attempts to mount
/// each partition, and checks for `/etc/fstab`. On success, prints the
/// mountpoint and the commands the user should run to chroot. On failure,
/// prints a clear error and returns [`RescueError::NoRootFilesystemFound`].
///
/// # Errors
///
/// - [`RescueError::ProcPartitionsUnreadable`] — `/proc/partitions` is not
///   readable (not Linux, or permission denied).
/// - [`RescueError::NoRootFilesystemFound`] — none of the scanned partitions
///   contained `/etc/fstab`.
///
/// # Examples
///
/// ```no_run
/// use bootcontrol_cli::rescue::run_rescue;
/// run_rescue().unwrap();
/// ```
pub fn run_rescue() -> Result<(), RescueError> {
    let proc_content = std::fs::read_to_string("/proc/partitions")
        .map_err(RescueError::ProcPartitionsUnreadable)?;

    info!("scanning /proc/partitions for root filesystems");

    let partitions = scan_partitions(&proc_content);
    debug!(count = partitions.len(), "candidate partitions found");

    for partition in &partitions {
        let dev_path = Path::new("/dev").join(partition);
        debug!(?dev_path, "checking partition");

        // Create a temporary mountpoint for this attempt.
        let Ok(tmp) = tempfile_mountpoint(partition) else {
            warn!(partition, "cannot create temp mountpoint — skipping");
            continue;
        };

        if let Some(mountpoint) = check_for_root(&dev_path, &tmp) {
            print_rescue_instructions(partition, &mountpoint);
            return Ok(());
        }
    }

    eprintln!("error: no root filesystem found on any of: {partitions:?}");
    Err(RescueError::NoRootFilesystemFound)
}

/// Parse `/proc/partitions` content and return a list of partition device names.
///
/// Whole-disk entries (names with no trailing digit, e.g. `sda`, `nvme0n1`)
/// are excluded because mounting them is meaningless for a root scan.
///
/// # Arguments
///
/// * `proc_partitions` — Raw content of `/proc/partitions`. Accepts mock
///   input for unit tests.
///
/// # Examples
///
/// ```no_run
/// // scan_partitions is part of bootcontrol-cli (binary crate)
/// // In production: content comes from std::fs::read_to_string("/proc/partitions")
/// // In tests: feed a mock string directly (see module tests).
/// ```
pub fn scan_partitions(proc_partitions: &str) -> Vec<String> {
    proc_partitions
        .lines()
        .filter_map(|line| {
            // The partition table header and blank lines contain no digits
            // in the fourth column, so the split_whitespace check below
            // naturally skips them.
            let mut cols = line.split_whitespace();
            // Skip major / minor / blocks
            cols.next()?; // major
            cols.next()?; // minor
            cols.next()?; // #blocks
            let name = cols.next()?;

            if is_partition(name) {
                Some(name.to_owned())
            } else {
                None
            }
        })
        .collect()
}

/// Determine whether a block device name represents a partition (not a whole disk).
///
/// Block device naming conventions differ by controller type:
///
/// | Controller | Whole disk | Partition |
/// |------------|-----------|-----------|
/// | SCSI/SATA  | `sda`     | `sda1`    |
/// | NVMe       | `nvme0n1` | `nvme0n1p1` |
/// | eMMC       | `mmcblk0` | `mmcblk0p1` |
///
/// The NVMe and eMMC conventions use a `p` separator before the partition
/// number because the whole-disk name already ends in a digit.
fn is_partition(name: &str) -> bool {
    if name.starts_with("nvme") || name.starts_with("mmcblk") {
        // For NVMe/eMMC: partition names contain a 'p' before the trailing digits.
        // Whole disks end in a digit but have no 'p' partition separator.
        // e.g. nvme0n1 → whole disk, nvme0n1p1 → partition
        name.contains('p') && name.chars().last().is_some_and(|c| c.is_ascii_digit())
    } else {
        // For sd*, hd*, vd*, xvd*, etc.: a trailing digit indicates a partition.
        name.chars().last().is_some_and(|c| c.is_ascii_digit())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Private helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Create a temporary directory path for mounting `partition`.
///
/// Returns a `PathBuf` of the form `/tmp/bootcontrol-rescue-<partition>`.
/// The directory is created before returning.
fn tempfile_mountpoint(partition: &str) -> std::io::Result<PathBuf> {
    let path = PathBuf::from(format!("/tmp/bootcontrol-rescue-{partition}"));
    std::fs::create_dir_all(&path)?;
    Ok(path)
}

/// Attempt to mount `dev_path` at `tmp_mount`, then check for `/etc/fstab`.
///
/// Returns `Some(mountpoint)` if `/etc/fstab` is found, `None` otherwise.
/// Permission errors (e.g., `EPERM` when not running as root) are handled
/// gracefully and produce a `None` return so the caller can keep scanning.
pub(crate) fn check_for_root(dev_path: &Path, tmp_mount: &Path) -> Option<PathBuf> {
    // We use the `mount` command rather than the `mount(2)` syscall so that
    // the CLI remains a safe, unprivileged binary — the OS will reject the
    // mount if insufficient privileges, which we handle below.
    let status = Command::new("mount").arg(dev_path).arg(tmp_mount).status();

    match status {
        Err(e) => {
            // `mount` binary not found or similar OS error.
            warn!(?dev_path, io_error = %e, "mount command failed");
            return None;
        }
        Ok(s) if !s.success() => {
            // Permission denied, unrecognised filesystem, etc. — not our root.
            debug!(?dev_path, exit_code = ?s.code(), "mount failed (non-root or wrong fs)");
            return None;
        }
        Ok(_) => {}
    }

    let fstab = tmp_mount.join("etc/fstab");
    if fstab.exists() {
        info!(?dev_path, ?tmp_mount, "found root filesystem");
        // Leave it mounted so the user can immediately chroot.
        Some(tmp_mount.to_owned())
    } else {
        debug!(?dev_path, "no /etc/fstab — unmounting");
        // Best-effort unmount; ignore errors.
        let _ = Command::new("umount").arg(tmp_mount).status();
        None
    }
}

/// Print the chroot instructions to stdout.
fn print_rescue_instructions(partition: &str, mountpoint: &Path) {
    let mp = mountpoint.display();
    println!("Found root filesystem on /dev/{partition} (mounted at {mp})");
    println!("Run the following to chroot into the system:");
    println!("  sudo mount --bind /dev  {mp}/dev");
    println!("  sudo mount --bind /sys  {mp}/sys");
    println!("  sudo mount --bind /proc {mp}/proc");
    println!("  sudo chroot {mp}");
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const MOCK_PROC_PARTITIONS: &str = "\
major minor  #blocks  name

   8        0  488386584 sda
   8        1     524288 sda1
   8        2  487860224 sda2
 259        0  976773168 nvme0n1
 259        1    1048576 nvme0n1p1
 259        2  975724032 nvme0n1p2
";

    /// Verify that partition names are extracted from a realistic mock input.
    #[test]
    fn scan_partitions_parses_mock_input() {
        let parts = scan_partitions(MOCK_PROC_PARTITIONS);
        assert_eq!(
            parts,
            vec!["sda1", "sda2", "nvme0n1p1", "nvme0n1p2"],
            "expected only partitions (with trailing digit), got: {parts:?}"
        );
    }

    /// Whole-disk entries must be excluded: they end in a letter, not a digit.
    #[test]
    fn scan_partitions_skips_whole_disks() {
        let parts = scan_partitions(MOCK_PROC_PARTITIONS);
        assert!(
            !parts.contains(&"sda".to_owned()),
            "whole disk 'sda' must be excluded"
        );
        assert!(
            !parts.contains(&"nvme0n1".to_owned()),
            "whole disk 'nvme0n1' must be excluded"
        );
    }

    /// Empty input must produce an empty list without panicking.
    #[test]
    fn scan_partitions_handles_empty_input() {
        let parts = scan_partitions("");
        assert!(parts.is_empty());
    }

    /// Header-only input (the first two lines) must produce an empty list.
    #[test]
    fn scan_partitions_handles_header_only() {
        let header = "major minor  #blocks  name\n\n";
        let parts = scan_partitions(header);
        assert!(
            parts.is_empty(),
            "header-only input must yield no partitions"
        );
    }

    /// `check_for_root` on a path we definitely cannot mount must return `None`
    /// rather than panicking.  This exercises the graceful permission-error path.
    #[test]
    fn check_for_root_returns_none_on_permission_error() {
        // `/dev/nonexistent999` does not exist; `mount` will fail immediately.
        // The test passes as long as `check_for_root` returns `None` instead of
        // panicking or returning `Some`.
        let dev = Path::new("/dev/nonexistent-bootcontrol-test");
        let tmp = PathBuf::from("/tmp/bootcontrol-rescue-test-nonexistent");
        // We create the dir so `mount` at least gets that far.
        let _ = std::fs::create_dir_all(&tmp);

        let result = check_for_root(dev, &tmp);
        assert!(
            result.is_none(),
            "check_for_root on a non-existent device must return None"
        );
    }
}
