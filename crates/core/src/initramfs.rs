//! Abstract interface for initramfs generator drivers.
//!
//! The trait carries **no I/O**. Implementations live in
//! `crates/daemon/src/initramfs/` where subprocess calls are permitted.
//! This module only defines the contract that implementations must fulfil.
//!
//! # Supported drivers
//!
//! | Driver | Binary | Invocation |
//! |--------|--------|-----------|
//! | `mkinitcpio` | `mkinitcpio` | `mkinitcpio -P` |
//! | `dracut` | `dracut` | `dracut --regenerate-all` |
//! | `kernel-install` | `kernel-install` | `kernel-install add <version>` |

#![deny(warnings)]
#![deny(missing_docs)]

use std::path::PathBuf;

use crate::error::BootControlError;

/// Abstract interface for an initramfs generator driver.
///
/// # Contract
///
/// - [`InitramfsDriver::binary_path`] must never panic. Return `None` if the
///   tool is absent.
/// - [`InitramfsDriver::regenerate`] must return
///   [`BootControlError::ToolNotFound`] (not panic) if the binary is missing.
/// - Never use `shell=true` or string-based command construction. Use
///   [`std::process::Command`] with explicit argument arrays only.
pub trait InitramfsDriver: Send + Sync {
    /// Human-readable driver name (e.g. `"mkinitcpio"`).
    ///
    /// # Examples
    ///
    /// ```
    /// // Implemented by concrete drivers in crates/daemon.
    /// // This example shows the expected shape.
    /// ```
    fn name(&self) -> &'static str;

    /// Return the absolute path to the driver binary, or `None` if not installed.
    ///
    /// Implementations search `$PATH` at call time using [`which::which`] or
    /// equivalent. This method must never panic.
    ///
    /// # Arguments
    ///
    /// *(none)*
    ///
    /// # Errors
    ///
    /// This method does not return a `Result` — absence of the binary is
    /// represented by `None`, not an error, to allow callers to enumerate
    /// available drivers without allocating errors.
    fn binary_path(&self) -> Option<PathBuf>;

    /// Regenerate the initramfs using this driver.
    ///
    /// # Arguments
    ///
    /// *(none — driver-specific parameters are provided at construction time)*
    ///
    /// # Errors
    ///
    /// - [`BootControlError::ToolNotFound`] — binary not found on `$PATH`.
    /// - [`BootControlError::EspScanFailed`] — binary exited with a non-zero
    ///   status; the `reason` field contains captured `stderr` output.
    fn regenerate(&self) -> Result<(), BootControlError>;
}
