//! Initramfs generator driver implementations.
//!
//! This module contains all three drivers. Each driver implements
//! [`bootcontrol_core::initramfs::InitramfsDriver`].
//!
//! For unit testing, each driver accepts an `binary_override` field that
//! redirects the binary lookup to a specific path (e.g. a test script).
//! Production code leaves this field `None`.

#![deny(warnings)]
#![deny(missing_docs)]

pub mod dracut;
pub mod kernel_install;
pub mod mkinitcpio;

use std::path::PathBuf;

use bootcontrol_core::{error::BootControlError, initramfs::InitramfsDriver};

/// Detect the first available initramfs driver on this system.
///
/// Tries drivers in priority order: `mkinitcpio` → `dracut` → `kernel-install`.
/// Returns `None` if none of the three binaries are found on `$PATH`.
///
/// For `kernel-install`, `kernel_version` is required because the tool
/// needs an explicit kernel version argument.
///
/// # Arguments
///
/// * `kernel_version` — The running kernel version string (e.g. `"6.8.1-arch1-1"`).
///   Used only for the `KernelInstallDriver`. Pass `""` if not applicable.
///
/// # Examples
///
/// ```
/// // In production, call with the output of `uname -r`.
/// // Returns None in CI where no initramfs tool is installed.
/// ```
pub fn detect_driver(kernel_version: &str) -> Option<Box<dyn InitramfsDriver>> {
    let mkinitcpio = mkinitcpio::MkinitcpioDriver {
        binary_override: None,
    };
    if mkinitcpio.binary_path().is_some() {
        return Some(Box::new(mkinitcpio));
    }

    let dracut = dracut::DracutDriver {
        binary_override: None,
    };
    if dracut.binary_path().is_some() {
        return Some(Box::new(dracut));
    }

    let ki = kernel_install::KernelInstallDriver {
        kernel_version: kernel_version.to_string(),
        binary_override: None,
    };
    if ki.binary_path().is_some() {
        return Some(Box::new(ki));
    }

    None
}

/// Execute a subprocess command and return an error on non-zero exit.
///
/// # Arguments
///
/// * `binary`  — Path to the binary to execute.
/// * `args`    — Arguments to pass.
/// * `driver_name` — Used in error messages.
///
/// # Errors
///
/// - [`BootControlError::EspScanFailed`] if the command exits non-zero. The
///   `reason` field contains captured stderr.
pub(crate) fn run_command(
    binary: &PathBuf,
    args: &[&str],
    driver_name: &str,
) -> Result<(), BootControlError> {
    let output = std::process::Command::new(binary)
        .args(args)
        .output()
        .map_err(|e| BootControlError::EspScanFailed {
            reason: format!("{driver_name}: failed to spawn process: {e}"),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        return Err(BootControlError::EspScanFailed {
            reason: format!("{driver_name} exited with error:\n{stderr}"),
        });
    }

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── detect_driver ─────────────────────────────────────────────────────────
    //
    // `detect_driver` resolves binaries via `which::which` at runtime, so we
    // cannot control its output without controlling `$PATH`. We therefore test
    // the behaviour through the public contract: the return value is either
    // `None` (no tool found) or `Some(box dyn InitramfsDriver)` whose `name()`
    // matches the expected priority.
    //
    // To avoid flakiness on developer machines that have mkinitcpio or dracut
    // installed, these tests validate properties that always hold regardless of
    // the environment.

    #[test]
    fn detect_driver_returns_none_when_no_tool_on_path() {
        // Acquire the workspace-wide PATH lock to prevent concurrent modification.
        let original_path = std::env::var("PATH").unwrap_or_default();
        let _guard = crate::grub_rebuild::tests::PATH_LOCK
            .lock()
            .expect("PATH lock poisoned");

        // Point PATH at a temp dir that contains none of the expected binaries.
        let empty_dir = tempfile::tempdir().expect("tempdir");
        std::env::set_var("PATH", empty_dir.path());

        let result = detect_driver("6.8.1");

        // Restore PATH before any assertions that might panic.
        std::env::set_var("PATH", &original_path);
        // _guard drops here, releasing the lock.

        assert!(
            result.is_none(),
            "detect_driver must return None when no initramfs tool is on PATH"
        );
    }

    #[test]
    fn detect_driver_result_is_some_when_tool_is_available() {
        // Create a fake `mkinitcpio` script on PATH.
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;

        let _guard = crate::grub_rebuild::tests::PATH_LOCK
            .lock()
            .expect("PATH lock poisoned");
        let original_path = std::env::var("PATH").unwrap_or_default();

        let bin_dir = tempfile::tempdir().expect("tempdir");
        let script = bin_dir.path().join("mkinitcpio");

        {
            let mut f = std::fs::File::create(&script).expect("create stub");
            writeln!(f, "#!/bin/sh").expect("write shebang");
            writeln!(f, "exit 0").expect("write body");
        }
        let mut perms = std::fs::metadata(&script).expect("metadata").permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&script, perms).expect("chmod");

        std::env::set_var("PATH", bin_dir.path());
        let result = detect_driver("6.8.1");
        std::env::set_var("PATH", &original_path);
        // _guard drops here.

        let driver = result.expect("detect_driver must return Some when mkinitcpio is on PATH");
        assert_eq!(driver.name(), "mkinitcpio");
    }

    #[test]
    fn detect_driver_prefers_mkinitcpio_over_dracut() {
        // Create both `mkinitcpio` and `dracut` stubs on PATH.
        // detect_driver must return mkinitcpio (higher priority).
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;

        let _guard = crate::grub_rebuild::tests::PATH_LOCK
            .lock()
            .expect("PATH lock poisoned");
        let original_path = std::env::var("PATH").unwrap_or_default();

        let bin_dir = tempfile::tempdir().expect("tempdir");

        for name in ["mkinitcpio", "dracut"] {
            let script = bin_dir.path().join(name);
            let mut f = std::fs::File::create(&script).expect("create stub");
            writeln!(f, "#!/bin/sh").expect("write");
            writeln!(f, "exit 0").expect("write");
            drop(f);
            let mut perms = std::fs::metadata(&script).expect("metadata").permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&script, perms).expect("chmod");
        }

        std::env::set_var("PATH", bin_dir.path());
        let result = detect_driver("6.8.1");
        std::env::set_var("PATH", &original_path);
        // _guard drops here.

        let driver = result.expect("detect_driver must return Some");
        assert_eq!(
            driver.name(),
            "mkinitcpio",
            "mkinitcpio has higher priority than dracut"
        );
    }

    // ── run_command ───────────────────────────────────────────────────────────

    #[test]
    fn run_command_succeeds_for_true_binary() {
        // `/bin/true` exits 0 on any POSIX system.
        let bin = std::path::PathBuf::from("/bin/true");
        if bin.exists() {
            assert!(
                run_command(&bin, &[], "test").is_ok(),
                "run_command must return Ok for a zero-exit binary"
            );
        }
    }

    #[test]
    fn run_command_fails_for_false_binary() {
        // `/bin/false` exits 1 on any POSIX system.
        let bin = std::path::PathBuf::from("/bin/false");
        if bin.exists() {
            assert!(
                run_command(&bin, &[], "test").is_err(),
                "run_command must return Err for a non-zero-exit binary"
            );
        }
    }
}
