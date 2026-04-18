//! `dracut` initramfs driver.
//!
//! Invokes `dracut --regenerate-all` to regenerate all initramfs images.

#![deny(warnings)]
#![deny(missing_docs)]

use std::path::PathBuf;

use bootcontrol_core::{error::BootControlError, initramfs::InitramfsDriver};

use crate::initramfs::run_command;

/// Driver for the `dracut` initramfs generator (Fedora, openSUSE, RHEL).
///
/// Invokes `dracut --regenerate-all` to regenerate all initramfs images.
///
/// The `binary_override` field allows tests to point the driver at a
/// controlled binary path instead of searching `$PATH`.
pub struct DracutDriver {
    /// Override the binary path for testing. `None` in production.
    pub binary_override: Option<PathBuf>,
}

impl InitramfsDriver for DracutDriver {
    fn name(&self) -> &'static str {
        "dracut"
    }

    /// Return the path to `dracut`, or `None` if not on `$PATH`.
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
    /// use bootcontrold::initramfs::dracut::DracutDriver;
    /// use bootcontrol_core::initramfs::InitramfsDriver;
    /// use std::path::PathBuf;
    ///
    /// let driver = DracutDriver {
    ///     binary_override: Some(PathBuf::from("/nonexistent/dracut")),
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
        which::which("dracut").ok()
    }

    /// Regenerate initramfs using `dracut --regenerate-all`.
    ///
    /// # Arguments
    ///
    /// *(none)*
    ///
    /// # Errors
    ///
    /// - [`BootControlError::ToolNotFound`] — `dracut` binary not found.
    /// - [`BootControlError::EspScanFailed`] — `dracut --regenerate-all` exited non-zero.
    ///
    /// # Examples
    ///
    /// ```
    /// // In production: requires root and a real dracut installation.
    /// ```
    fn regenerate(&self) -> Result<(), BootControlError> {
        let bin = self
            .binary_path()
            .ok_or_else(|| BootControlError::ToolNotFound {
                tool: self.name().to_string(),
            })?;
        run_command(&bin, &["--regenerate-all"], self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_returns_dracut() {
        let d = DracutDriver {
            binary_override: None,
        };
        assert_eq!(d.name(), "dracut");
    }

    #[test]
    fn binary_path_returns_none_for_nonexistent_override() {
        let d = DracutDriver {
            binary_override: Some(PathBuf::from("/this/path/does/not/exist/dracut")),
        };
        assert!(d.binary_path().is_none());
    }

    #[test]
    fn regenerate_returns_tool_not_found_when_binary_missing() {
        let d = DracutDriver {
            binary_override: Some(PathBuf::from("/this/path/does/not/exist/dracut")),
        };
        assert!(matches!(
            d.regenerate(),
            Err(BootControlError::ToolNotFound { .. })
        ));
    }
}
