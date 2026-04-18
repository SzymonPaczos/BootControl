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
