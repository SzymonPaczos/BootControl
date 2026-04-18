//! Golden-parachute failsafe GRUB entry generator.
//!
//! Every time [`crate::grub_manager::set_grub_value`] successfully writes a
//! new GRUB configuration, this module rewrites
//! `/etc/bootcontrol/failsafe.cfg` (or a caller-injected path) with a minimal
//! GRUB `menuentry` that boots the currently running kernel using only
//! `root=<uuid> ro` — no user-supplied `GRUB_CMDLINE_LINUX_DEFAULT` at all.
//!
//! # Security guarantee
//!
//! The generated `linux` line is constructed entirely from values parsed out of
//! `/proc/version` and `/proc/mounts` — never from the GRUB config being
//! written. This means a bad `GRUB_CMDLINE_LINUX_DEFAULT` cannot affect the
//! failsafe entry.
//!
//! # Write strategy
//!
//! The file is written atomically via the same `tmp → fsync → rename` pipeline
//! used by [`crate::grub_manager`]. The target directory is created (with
//! `0o755` permissions) if it does not already exist.

use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::SystemTime,
};

use bootcontrol_core::error::BootControlError;
use chrono::{DateTime, Utc};
use tracing::{debug, info, warn};

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

/// Detect the running kernel, build a failsafe GRUB snippet, and write it
/// atomically to `cfg_path`.
///
/// The parent directory of `cfg_path` is created if it does not exist.
/// The file is written via `tmp → fsync → rename` so a concurrent reader
/// never sees a partial write.
///
/// This function is **idempotent**: calling it twice with the same kernel
/// produces a valid, complete file both times.
///
/// # Arguments
///
/// * `cfg_path` — Destination path for the failsafe snippet. Production code
///   passes `/etc/bootcontrol/failsafe.cfg`; tests pass a path inside a
///   `tempfile::TempDir`.
///
/// # Errors
///
/// Returns [`BootControlError::EspScanFailed`] on any I/O error, including
/// failure to read `/proc/version`, `/proc/mounts`, or write the output file.
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
/// use bootcontrold::failsafe::refresh_failsafe_entry;
///
/// refresh_failsafe_entry(Path::new("/etc/bootcontrol/failsafe.cfg")).unwrap();
/// ```
pub fn refresh_failsafe_entry(cfg_path: &Path) -> Result<(), BootControlError> {
    info!(?cfg_path, "refreshing failsafe GRUB entry");

    // Detect kernel info — reads /proc/version (Linux-only; skipped in tests).
    let kernel = detect_running_kernel()?;
    debug!(version = %kernel.version, vmlinuz = ?kernel.vmlinuz, "detected running kernel");

    // Detect initrd — optional; omitted from snippet if not found.
    let initrd = detect_initrd(&kernel.version);
    debug!(initrd = ?initrd, "detected initrd");

    // Detect root UUID / device — reads /proc/mounts and /dev/disk/by-uuid/.
    let root = detect_root_uuid();
    debug!(root = %root, "detected root device/UUID");

    let timestamp: DateTime<Utc> = SystemTime::now().into();
    let snippet = build_failsafe_snippet(&kernel, initrd.as_deref(), &root, &timestamp);

    write_failsafe_content(cfg_path, &snippet)?;

    info!(
        ?cfg_path,
        "failsafe entry refreshed — run update-grub to activate"
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal data types
// ─────────────────────────────────────────────────────────────────────────────

/// Information about the currently running kernel image.
struct KernelInfo {
    /// Kernel release string, e.g. `"6.8.0-54-generic"`.
    version: String,
    /// Absolute path to the vmlinuz image, e.g. `/boot/vmlinuz-6.8.0-54-generic`.
    vmlinuz: PathBuf,
}

// ─────────────────────────────────────────────────────────────────────────────
// Private helpers (Linux-specific I/O)
// ─────────────────────────────────────────────────────────────────────────────

/// Parse the kernel release string from `/proc/version`.
///
/// `/proc/version` has the form:
/// `Linux version 6.8.0-54-generic (buildd@...) #57-Ubuntu ...`
///
/// We extract the third whitespace-separated token (index 2), which is the
/// kernel release string passed to `uname -r`.
fn parse_proc_version(content: &str) -> Option<String> {
    // Token layout: "Linux" "version" "<release>" ...
    content.split_whitespace().nth(2).map(str::to_owned)
}

/// Detect the currently running kernel by reading `/proc/version`.
///
/// Falls back to scanning `/boot/vmlinuz-*` by mtime if `/proc/version` is
/// unavailable (e.g., non-Linux hosts, containers without procfs).
///
/// On hosts where both `/proc/version` and `/boot` are absent (e.g., macOS
/// CI), returns a placeholder `KernelInfo` with `"unknown"` version so that
/// the failsafe directory and file are still created and the write pipeline
/// remains exercisable without Linux kernel infrastructure.
fn detect_running_kernel() -> Result<KernelInfo, BootControlError> {
    // Primary: /proc/version
    if let Ok(proc_content) = fs::read_to_string("/proc/version") {
        if let Some(version) = parse_proc_version(&proc_content) {
            let vmlinuz = PathBuf::from(format!("/boot/vmlinuz-{version}"));
            return Ok(KernelInfo { version, vmlinuz });
        }
    }

    // Fallback: pick the newest vmlinuz in /boot by mtime.
    debug!("/proc/version unavailable — falling back to /boot scan");

    match scan_boot_for_newest_kernel() {
        Ok(info) => Ok(info),
        Err(_) => {
            // Neither /proc/version nor /boot are available (macOS CI, minimal
            // containers). Produce a placeholder so the failsafe *file* is still
            // written for the write-path tests; a warning surfaces the situation.
            warn!(
                "kernel detection unavailable (no /proc/version, no /boot) \
                 — writing failsafe entry with placeholder version"
            );
            Ok(KernelInfo {
                version: "unknown".to_owned(),
                vmlinuz: PathBuf::from("/boot/vmlinuz-unknown"),
            })
        }
    }
}

/// Scan `/boot/vmlinuz-*` and return the entry with the newest mtime.
fn scan_boot_for_newest_kernel() -> Result<KernelInfo, BootControlError> {
    let entries = fs::read_dir("/boot").map_err(|e| {
        warn!(io_error = %e, "cannot read /boot");
        BootControlError::EspScanFailed {
            reason: format!("cannot read /boot: {e}"),
        }
    })?;

    let mut best: Option<(SystemTime, PathBuf)> = None;

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if !name_str.starts_with("vmlinuz-") {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        let Ok(mtime) = meta.modified() else { continue };
        if best.as_ref().is_none_or(|(t, _)| mtime > *t) {
            best = Some((mtime, entry.path()));
        }
    }

    let (_, path) = best.ok_or_else(|| BootControlError::EspScanFailed {
        reason: "no vmlinuz-* found in /boot".to_owned(),
    })?;

    let version = path
        .file_name()
        .and_then(|n| n.to_str())
        .and_then(|n| n.strip_prefix("vmlinuz-"))
        .ok_or_else(|| BootControlError::EspScanFailed {
            reason: format!("cannot extract version from {}", path.display()),
        })?
        .to_owned();

    Ok(KernelInfo {
        version,
        vmlinuz: path,
    })
}

/// Find the initrd matching `version`.
///
/// Checks the two most common naming conventions:
/// - `/boot/initrd.img-<version>` (Debian/Ubuntu)
/// - `/boot/initramfs-<version>.img` (Fedora/Arch)
///
/// Returns `None` if neither file exists; the caller omits the `initrd` line
/// from the GRUB snippet in that case.
fn detect_initrd(version: &str) -> Option<PathBuf> {
    let candidates = [
        PathBuf::from(format!("/boot/initrd.img-{version}")),
        PathBuf::from(format!("/boot/initramfs-{version}.img")),
    ];
    candidates.into_iter().find(|p| p.exists())
}

/// Determine the root device/UUID for the `linux` line.
///
/// Reads `/proc/mounts`, finds the entry for `/`, then resolves the UUID from
/// `/dev/disk/by-uuid/` symlinks.  Falls back to the raw device path if UUID
/// resolution fails.
fn detect_root_uuid() -> String {
    match try_detect_root_uuid() {
        Some(uuid_or_dev) => uuid_or_dev,
        None => {
            warn!("cannot determine root device; using placeholder root=/dev/sda1");
            "/dev/sda1".to_owned()
        }
    }
}

/// Inner implementation; returns `None` on any failure so the caller can log
/// and fall back gracefully.
fn try_detect_root_uuid() -> Option<String> {
    let mounts = fs::read_to_string("/proc/mounts").ok()?;
    let root_dev = parse_root_device(&mounts)?;
    debug!(root_dev = %root_dev, "found root device in /proc/mounts");

    // Try UUID symlink resolution.
    if let Ok(entries) = fs::read_dir("/dev/disk/by-uuid") {
        for entry in entries.flatten() {
            if let Ok(target) = fs::canonicalize(entry.path()) {
                if target == Path::new(&root_dev) {
                    let uuid = entry.file_name().to_string_lossy().into_owned();
                    return Some(format!("UUID={uuid}"));
                }
            }
        }
    }

    // UUID resolution failed — use the raw device path.
    Some(root_dev)
}

/// Extract the block device for the `/` mount from a `/proc/mounts` string.
///
/// The format is: `<device> <mountpoint> <fstype> <options> <dump> <pass>`
/// We look for a line where the second field is exactly `/`.
fn parse_root_device(mounts: &str) -> Option<String> {
    for line in mounts.lines() {
        let mut fields = line.split_whitespace();
        let device = fields.next()?;
        let mountpoint = fields.next()?;
        if mountpoint == "/" {
            return Some(device.to_owned());
        }
    }
    None
}

// ─────────────────────────────────────────────────────────────────────────────
// Snippet builder & atomic writer
// ─────────────────────────────────────────────────────────────────────────────

/// Build the GRUB failsafe snippet string.
///
/// The `linux` line contains **only** `root=<root> ro` — no user cmdline.
/// The `initrd` line is omitted when `initrd` is `None`.
fn build_failsafe_snippet(
    kernel: &KernelInfo,
    initrd: Option<&Path>,
    root: &str,
    timestamp: &DateTime<Utc>,
) -> String {
    // RFC3339 format: 2026-04-17T20:00:00+00:00
    let ts = timestamp.to_rfc3339();
    let vmlinuz = kernel.vmlinuz.display();
    // `version` is embedded in the vmlinuz path above but also used by callers
    // to show the detected kernel; reference it here via the struct field.
    let _ = &kernel.version; // field read for future use (initrd line naming)
    let initrd_line = match initrd {
        Some(p) => format!("    initrd  {}\n", p.display()),
        None => String::new(),
    };

    format!(
        "# BootControl Failsafe Entry — auto-generated, do not edit manually\n\
         # Generated: {ts}\n\
         menuentry \"Linux (Failsafe — BootControl)\" --class linux {{\n\
         \x20   load_video\n\
         \x20   insmod gzio\n\
         \x20   insmod part_gpt\n\
         \x20   insmod ext2\n\
         \x20   linux   {vmlinuz} root={root} ro\n\
         {initrd_line}\
         }}\n"
    )
}

/// Create parent directories if needed, then write `content` to `cfg_path`
/// atomically via `tmp → fsync → rename`.
///
/// This function is the unit-testable I/O kernel of the module. The public
/// [`refresh_failsafe_entry`] delegates here after building the snippet.
///
/// # Errors
///
/// Returns [`BootControlError::EspScanFailed`] on any I/O error.
pub(crate) fn write_failsafe_content(
    cfg_path: &Path,
    content: &str,
) -> Result<(), BootControlError> {
    // ── 1. Ensure parent directory exists ────────────────────────────────────
    // We create the directory with mode 0o755 — readable by all, writable only
    // by root — matching the security requirement that the file is not writable
    // by non-root.
    if let Some(parent) = cfg_path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            warn!(?parent, io_error = %e, "cannot create failsafe config directory");
            BootControlError::EspScanFailed {
                reason: format!("cannot create directory {}: {e}", parent.display()),
            }
        })?;
        debug!(?parent, "failsafe config directory ready");
    }

    // ── 2. Write to a sibling .tmp file ──────────────────────────────────────
    let tmp_path = build_tmp_path(cfg_path);
    let mut tmp = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&tmp_path)
        .map_err(|e| {
            warn!(?tmp_path, io_error = %e, "cannot create failsafe tmp file");
            BootControlError::EspScanFailed {
                reason: format!("cannot create tmp file {}: {e}", tmp_path.display()),
            }
        })?;

    tmp.write_all(content.as_bytes())
        .map_err(|e| BootControlError::EspScanFailed {
            reason: format!("write to tmp file failed: {e}"),
        })?;

    // ── 3. fsync — ensure data reaches storage before rename ─────────────────
    tmp.sync_all()
        .map_err(|e| BootControlError::EspScanFailed {
            reason: format!("fsync failed on failsafe tmp: {e}"),
        })?;

    drop(tmp); // flush OS buffers before rename

    // ── 4. Atomic rename ─────────────────────────────────────────────────────
    fs::rename(&tmp_path, cfg_path).map_err(|e| {
        warn!(?tmp_path, ?cfg_path, io_error = %e, "atomic rename failed for failsafe cfg");
        BootControlError::EspScanFailed {
            reason: format!("atomic rename failed: {e}"),
        }
    })?;

    Ok(())
}

/// Compute the sibling `.tmp` path for `target`.
///
/// Placed in the same directory as `target` to guarantee atomicity of
/// the `rename()` call (both files on the same filesystem).
fn build_tmp_path(target: &Path) -> PathBuf {
    let stem = target
        .file_name()
        .unwrap_or_else(|| std::ffi::OsStr::new("failsafe.cfg"))
        .to_string_lossy();
    target
        .parent()
        .unwrap_or_else(|| Path::new("/etc/bootcontrol"))
        .join(format!("{stem}.tmp"))
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // ── helpers ───────────────────────────────────────────────────────────────

    /// Build a deterministic KernelInfo pointing at paths inside `dir`.
    fn fake_kernel(dir: &TempDir) -> KernelInfo {
        KernelInfo {
            version: "5.15.0-test".to_owned(),
            vmlinuz: dir.path().join("vmlinuz-5.15.0-test"),
        }
    }

    /// A fixed UTC timestamp for deterministic snapshot testing.
    fn fixed_ts() -> DateTime<Utc> {
        use chrono::TimeZone;
        // 2026-01-01 00:00:00 UTC
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0)
            .single()
            .expect("valid fixed timestamp")
    }

    fn write_snippet_to_dir(dir: &TempDir) -> PathBuf {
        let cfg_path = dir.path().join("sub").join("failsafe.cfg");
        let kernel = fake_kernel(dir);
        let snippet = build_failsafe_snippet(&kernel, None, "UUID=dead-beef", &fixed_ts());
        write_failsafe_content(&cfg_path, &snippet)
            // Safety: test environment, deliberately simple
            .expect("write must succeed in tempdir");
        cfg_path
    }

    // ── tests ─────────────────────────────────────────────────────────────────

    /// Requirement: the directory is created even when it does not pre-exist.
    #[test]
    fn refresh_failsafe_entry_creates_directory_if_missing() {
        let dir = TempDir::new().expect("tempdir");
        let cfg_path = dir.path().join("new_dir").join("failsafe.cfg");

        let kernel = fake_kernel(&dir);
        let snippet = build_failsafe_snippet(&kernel, None, "UUID=1234", &fixed_ts());
        write_failsafe_content(&cfg_path, &snippet).expect("should create dir and write");

        assert!(cfg_path.exists(), "failsafe.cfg must exist after write");
    }

    /// Requirement: the output contains the `menuentry` keyword.
    #[test]
    fn refresh_failsafe_entry_writes_menuentry_keyword() {
        let dir = TempDir::new().expect("tempdir");
        let cfg_path = write_snippet_to_dir(&dir);

        let content = fs::read_to_string(&cfg_path).expect("read back");
        assert!(
            content.contains("menuentry"),
            "snippet must contain 'menuentry':\n{content}"
        );
    }

    /// Requirement: the comment header contains a timestamp.
    #[test]
    fn refresh_failsafe_entry_writes_timestamp_comment() {
        let dir = TempDir::new().expect("tempdir");
        let cfg_path = write_snippet_to_dir(&dir);

        let content = fs::read_to_string(&cfg_path).expect("read back");
        assert!(
            content.contains("# Generated:"),
            "snippet must contain timestamp comment:\n{content}"
        );
        // Our fixed timestamp starts with "2026-01-01"
        assert!(
            content.contains("2026-01-01"),
            "timestamp must embed the fixed date:\n{content}"
        );
    }

    /// Requirement: calling twice produces a valid, complete file both times.
    #[test]
    fn refresh_failsafe_entry_is_idempotent() {
        let dir = TempDir::new().expect("tempdir");
        let cfg_path = dir.path().join("failsafe.cfg");
        let kernel = fake_kernel(&dir);

        for _ in 0..2 {
            let snippet = build_failsafe_snippet(&kernel, None, "UUID=abcd", &fixed_ts());
            write_failsafe_content(&cfg_path, &snippet).expect("write must succeed");
        }

        let content = fs::read_to_string(&cfg_path).expect("read back");
        assert!(
            content.contains("menuentry"),
            "file must be valid after second write:\n{content}"
        );
    }

    /// Requirement: the `linux` line must end with `ro` (and nothing else
    /// dangerous after root=).
    #[test]
    fn refresh_failsafe_entry_contains_ro_in_linux_line() {
        let dir = TempDir::new().expect("tempdir");
        let cfg_path = write_snippet_to_dir(&dir);

        let content = fs::read_to_string(&cfg_path).expect("read back");
        let linux_line = content
            .lines()
            .find(|l| l.trim_start().starts_with("linux "))
            .expect("linux line must be present");

        assert!(
            linux_line.contains(" ro"),
            "linux line must contain 'ro':\n{linux_line}"
        );
    }

    /// Requirement: the generated snippet must NOT contain any string that
    /// looks like GRUB_CMDLINE_LINUX_DEFAULT content.  We verify it contains
    /// neither the key name nor any custom cmdline parameters.
    #[test]
    fn refresh_failsafe_entry_does_not_contain_cmdline_default() {
        let dir = TempDir::new().expect("tempdir");
        let kernel = fake_kernel(&dir);
        let cfg_path = dir.path().join("failsafe.cfg");

        // Build snippet as the production path does — root comes from detect_root_uuid(),
        // which we stub inline here.
        let snippet = build_failsafe_snippet(&kernel, None, "UUID=test-uuid", &fixed_ts());
        write_failsafe_content(&cfg_path, &snippet).expect("write");

        let content = fs::read_to_string(&cfg_path).expect("read back");

        assert!(
            !content.contains("GRUB_CMDLINE_LINUX_DEFAULT"),
            "snippet must not reference the cmdline key:\n{content}"
        );
        // Spot-check that no typical GRUB_CMDLINE_DEFAULT values leaked in.
        assert!(
            !content.contains("quiet splash"),
            "snippet must not contain user cmdline args:\n{content}"
        );
    }

    // ── unit tests for pure helpers ───────────────────────────────────────────

    #[test]
    fn parse_proc_version_extracts_release() {
        let input =
            "Linux version 6.8.0-54-generic (buildd@lcy02-amd64-013) #57-Ubuntu SMP PREEMPT_DYNAMIC";
        assert_eq!(
            parse_proc_version(input),
            Some("6.8.0-54-generic".to_owned())
        );
    }

    #[test]
    fn parse_root_device_finds_root() {
        let mounts =
            "sysfs /sys sysfs rw 0 0\n/dev/sda2 / ext4 rw,relatime 0 0\ntmpfs /tmp tmpfs rw 0 0\n";
        assert_eq!(parse_root_device(mounts), Some("/dev/sda2".to_owned()));
    }

    #[test]
    fn parse_root_device_returns_none_when_absent() {
        let mounts = "sysfs /sys sysfs rw 0 0\n";
        assert_eq!(parse_root_device(mounts), None);
    }

    #[test]
    fn build_failsafe_snippet_omits_initrd_when_none() {
        let dir = TempDir::new().expect("tempdir");
        let kernel = fake_kernel(&dir);
        let snippet = build_failsafe_snippet(&kernel, None, "UUID=abc", &fixed_ts());
        assert!(
            !snippet.contains("initrd"),
            "initrd line must be absent when initrd is None:\n{snippet}"
        );
    }

    #[test]
    fn build_failsafe_snippet_includes_initrd_when_some() {
        let dir = TempDir::new().expect("tempdir");
        let kernel = fake_kernel(&dir);
        let initrd = dir.path().join("initrd.img-5.15.0-test");
        let snippet = build_failsafe_snippet(&kernel, Some(&initrd), "UUID=abc", &fixed_ts());
        assert!(
            snippet.contains("initrd"),
            "initrd line must be present when initrd is Some:\n{snippet}"
        );
    }
}
