//! `kernel-install` initramfs driver.
//!
//! Invokes `kernel-install add <version>` to trigger a full kernel installation
//! including initramfs generation.

#![deny(warnings)]
#![deny(missing_docs)]

use std::path::PathBuf;

use bootcontrol_core::{error::BootControlError, initramfs::InitramfsDriver};

use crate::initramfs::run_command;

/// Driver for the `kernel-install` tool (systemd-integrated distributions).
///
/// Invokes `kernel-install add <kernel_version>` to trigger initramfs
/// regeneration and kernel installation.
///
/// Note: unlike `mkinitcpio` and `dracut`, `kernel-install add` requires
/// an explicit kernel version argument. Set [`KernelInstallDriver::kernel_version`]
/// to the output of `uname -r` or equivalent.
///
/// The `binary_override` field allows tests to point the driver at a
/// controlled binary path instead of searching `$PATH`.
pub struct KernelInstallDriver {
    /// The kernel version string (e.g. `"6.8.1-arch1-1"`).
    /// Required by `kernel-install add <version>`.
    pub kernel_version: String,

    /// Override the binary path for testing. `None` in production.
    pub binary_override: Option<PathBuf>,
}

impl InitramfsDriver for KernelInstallDriver {
    fn name(&self) -> &'static str {
        "kernel-install"
    }

    /// Return the path to `kernel-install`, or `None` if not on `$PATH`.
    ///
    /// # Arguments
    ///
    /// *(none)*
    ///
    /// # Errors
    ///
    /// This method does not return errors — absence is represented by `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrold::initramfs::kernel_install::KernelInstallDriver;
    /// use bootcontrol_core::initramfs::InitramfsDriver;
    /// use std::path::PathBuf;
    ///
    /// let driver = KernelInstallDriver {
    ///     kernel_version: "6.8.1".to_string(),
    ///     binary_override: Some(PathBuf::from("/nonexistent/kernel-install")),
    /// };
    /// assert!(driver.binary_path().is_none());
    /// ```
    fn binary_path(&self) -> Option<PathBuf> {
        if let Some(ref p) = self.binary_override {
            if p.exists() {
                return Some(p.clone());
            }
            return None;
        }
        which::which("kernel-install").ok()
    }

    /// Regenerate initramfs using `kernel-install add <kernel_version>`.
    ///
    /// # Arguments
    ///
    /// *(none — kernel version is set at construction time via `kernel_version`)*
    ///
    /// # Errors
    ///
    /// - [`BootControlError::ToolNotFound`] — `kernel-install` binary not found.
    /// - [`BootControlError::EspScanFailed`] — command exited non-zero.
    ///
    /// # Examples
    ///
    /// ```
    /// // Requires kernel-install on $PATH and root privileges.
    /// ```
    fn regenerate(&self) -> Result<(), BootControlError> {
        let bin = self
            .binary_path()
            .ok_or_else(|| BootControlError::ToolNotFound {
                tool: self.name().to_string(),
            })?;
        run_command(&bin, &["add", &self.kernel_version], self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_driver(version: &str) -> KernelInstallDriver {
        KernelInstallDriver {
            kernel_version: version.to_string(),
            binary_override: None,
        }
    }

    #[test]
    fn name_returns_kernel_install() {
        assert_eq!(make_driver("6.8.1").name(), "kernel-install");
    }

    #[test]
    fn binary_path_returns_none_for_nonexistent_override() {
        let d = KernelInstallDriver {
            kernel_version: "6.8.1".to_string(),
            binary_override: Some(PathBuf::from("/this/path/does/not/exist/kernel-install")),
        };
        assert!(d.binary_path().is_none());
    }

    #[test]
    fn regenerate_returns_tool_not_found_when_binary_missing() {
        let d = KernelInstallDriver {
            kernel_version: "6.8.1".to_string(),
            binary_override: Some(PathBuf::from("/this/path/does/not/exist/kernel-install")),
        };
        assert!(matches!(
            d.regenerate(),
            Err(BootControlError::ToolNotFound { .. })
        ));
    }
}
