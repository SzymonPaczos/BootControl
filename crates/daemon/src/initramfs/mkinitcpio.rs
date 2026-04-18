//! `mkinitcpio` initramfs driver.
//!
//! Invokes `mkinitcpio -P` to regenerate all presets.

#![deny(warnings)]
#![deny(missing_docs)]

use std::path::PathBuf;

use bootcontrol_core::{error::BootControlError, initramfs::InitramfsDriver};

use crate::initramfs::run_command;

/// Driver for the `mkinitcpio` initramfs generator (Arch Linux).
///
/// Invokes `mkinitcpio -P` to regenerate initramfs for all presets.
///
/// The `binary_override` field allows tests to point the driver at a
/// controlled binary path instead of searching `$PATH`.
pub struct MkinitcpioDriver {
    /// Override the binary path for testing. `None` in production.
    pub binary_override: Option<PathBuf>,
}

impl InitramfsDriver for MkinitcpioDriver {
    fn name(&self) -> &'static str {
        "mkinitcpio"
    }

    /// Return the path to `mkinitcpio`, or `None` if not on `$PATH`.
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
    /// use bootcontrold::initramfs::mkinitcpio::MkinitcpioDriver;
    /// use bootcontrol_core::initramfs::InitramfsDriver;
    /// use std::path::PathBuf;
    ///
    /// // With an override path that doesn't exist:
    /// let driver = MkinitcpioDriver {
    ///     binary_override: Some(PathBuf::from("/nonexistent/mkinitcpio")),
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
        which::which("mkinitcpio").ok()
    }

    /// Regenerate initramfs using `mkinitcpio -P`.
    ///
    /// # Arguments
    ///
    /// *(none)*
    ///
    /// # Errors
    ///
    /// - [`BootControlError::ToolNotFound`] — `mkinitcpio` binary not found.
    /// - [`BootControlError::EspScanFailed`] — `mkinitcpio -P` exited non-zero.
    ///
    /// # Examples
    ///
    /// ```
    /// // In production: requires root and a real Arch Linux system.
    /// // In tests: use binary_override to point at a mock script.
    /// ```
    fn regenerate(&self) -> Result<(), BootControlError> {
        let bin = self
            .binary_path()
            .ok_or_else(|| BootControlError::ToolNotFound {
                tool: self.name().to_string(),
            })?;
        run_command(&bin, &["-P"], self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_returns_mkinitcpio() {
        let d = MkinitcpioDriver {
            binary_override: None,
        };
        assert_eq!(d.name(), "mkinitcpio");
    }

    #[test]
    fn binary_path_returns_none_for_nonexistent_override() {
        let d = MkinitcpioDriver {
            binary_override: Some(PathBuf::from("/this/path/does/not/exist/mkinitcpio")),
        };
        assert!(d.binary_path().is_none());
    }

    #[test]
    fn regenerate_returns_tool_not_found_when_binary_missing() {
        let d = MkinitcpioDriver {
            binary_override: Some(PathBuf::from("/this/path/does/not/exist/mkinitcpio")),
        };
        assert!(matches!(
            d.regenerate(),
            Err(BootControlError::ToolNotFound { .. })
        ));
    }
}
