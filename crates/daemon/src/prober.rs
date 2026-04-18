//! Bootloader auto-detection — filesystem wrapper.
//!
//! This module provides the thin wrapper that calls [`std::path::Path::exists`]
//! and delegates to the pure [`bootcontrol_core::prober::detect_bootloader`]
//! function. The pure function is tested in isolation; this module is tested
//! only via E2E tests where real filesystem access is acceptable.

#![deny(warnings)]
#![deny(missing_docs)]

use std::path::Path;

use bootcontrol_core::{
    backends::{grub::GrubBackend, systemd_boot::SystemdBootBackend},
    boot_manager::BootManager,
    prober::{detect_bootloader, DetectedBootloader},
};

/// Probe the real filesystem and determine the active bootloader.
///
/// Checks for the presence of:
/// - `/boot/loader/loader.conf` (systemd-boot indicator)
/// - `/boot/EFI/systemd/` (systemd-boot EFI binary directory)
/// - `/etc/default/grub` (GRUB indicator)
///
/// # Examples
///
/// ```
/// // In production this calls Path::exists() on real paths.
/// // Use detect_bootloader() from core for unit-testable logic.
/// ```
pub fn probe_system() -> DetectedBootloader {
    detect_bootloader(
        Path::new("/boot/loader/loader.conf").exists(),
        Path::new("/boot/EFI/systemd/").exists(),
        Path::new("/etc/default/grub").exists(),
    )
}

/// Build the appropriate [`BootManager`] backend for a detected bootloader.
///
/// Falls back to [`GrubBackend`] for `Unknown` because GRUB is the most
/// widely deployed bootloader and a graceful degradation is preferable to
/// refusing to start.
///
/// # Arguments
///
/// * `detected` — The result of [`probe_system`] or
///   [`bootcontrol_core::prober::detect_bootloader`].
///
/// # Examples
///
/// ```
/// use bootcontrold::prober::build_backend;
/// use bootcontrol_core::prober::DetectedBootloader;
///
/// let backend = build_backend(DetectedBootloader::Grub);
/// assert_eq!(backend.name(), "grub");
///
/// let backend = build_backend(DetectedBootloader::SystemdBoot);
/// assert_eq!(backend.name(), "systemd-boot");
/// ```
pub fn build_backend(detected: DetectedBootloader) -> Box<dyn BootManager> {
    match detected {
        DetectedBootloader::SystemdBoot => Box::new(SystemdBootBackend),
        DetectedBootloader::Grub | DetectedBootloader::Unknown => Box::new(GrubBackend),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_backend_grub_returns_grub_name() {
        let backend = build_backend(DetectedBootloader::Grub);
        assert_eq!(backend.name(), "grub");
    }

    #[test]
    fn build_backend_systemd_boot_returns_systemd_boot_name() {
        let backend = build_backend(DetectedBootloader::SystemdBoot);
        assert_eq!(backend.name(), "systemd-boot");
    }

    #[test]
    fn build_backend_unknown_falls_back_to_grub() {
        let backend = build_backend(DetectedBootloader::Unknown);
        assert_eq!(backend.name(), "grub");
    }
}
