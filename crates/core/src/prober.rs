//! Bootloader auto-detection prober.
//!
//! This module provides a **pure function** that determines which bootloader
//! is active on the system from a set of pre-checked filesystem indicators.
//! The function accepts boolean flags rather than calling
//! [`std::path::Path::exists`] directly, so it can be unit-tested without
//! touching the real filesystem.
//!
//! The thin wrapper that queries the real filesystem lives in
//! `crates/daemon/src/prober.rs`, which also builds the corresponding
//! [`BootManager`](crate::boot_manager::BootManager) instance.
//!
//! # Detection priority (deterministic — first match wins)
//!
//! | Priority | Condition | Result |
//! |----------|-----------|--------|
//! | 1 | `/boot/loader/loader.conf` exists AND `/boot/EFI/systemd/` exists | `SystemdBoot` |
//! | 2 | `/boot/loader/loader.conf` exists (without EFI dir) | `SystemdBoot` |
//! | 3 | `/etc/default/grub` exists | `Grub` |
//! | 4 | None of the above | `Unknown` |

#![deny(warnings)]
#![deny(missing_docs)]

/// The bootloader detected on this system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectedBootloader {
    /// systemd-boot was detected (presence of `/boot/loader/loader.conf`).
    SystemdBoot,
    /// GRUB was detected (presence of `/etc/default/grub`).
    Grub,
    /// No supported bootloader was detected.
    Unknown,
}

impl std::fmt::Display for DetectedBootloader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DetectedBootloader::SystemdBoot => write!(f, "systemd-boot"),
            DetectedBootloader::Grub => write!(f, "grub"),
            DetectedBootloader::Unknown => write!(f, "unknown"),
        }
    }
}

/// Detect the active bootloader from pre-checked filesystem indicators.
///
/// This is a pure function: all filesystem queries are performed by the caller
/// and passed as boolean parameters, making this function testable without
/// any filesystem access.
///
/// # Detection priority
///
/// 1. `loader_conf_exists` → [`DetectedBootloader::SystemdBoot`]  
///    (presence of `/boot/loader/loader.conf` is sufficient; the EFI dir is
///    informational but does not change the outcome)
/// 2. `grub_default_exists` → [`DetectedBootloader::Grub`]
/// 3. Otherwise → [`DetectedBootloader::Unknown`]
///
/// # Arguments
///
/// * `loader_conf_exists`  — `true` if `/boot/loader/loader.conf` is present.
/// * `systemd_efi_exists`  — `true` if `/boot/EFI/systemd/` directory is present.
///   Informational — does not currently change detection outcome.
/// * `grub_default_exists` — `true` if `/etc/default/grub` is present.
///
/// # Examples
///
/// ```
/// use bootcontrol_core::prober::{detect_bootloader, DetectedBootloader};
///
/// // systemd-boot with full EFI structure
/// assert_eq!(
///     detect_bootloader(true, true, false),
///     DetectedBootloader::SystemdBoot
/// );
///
/// // GRUB when no loader.conf
/// assert_eq!(
///     detect_bootloader(false, false, true),
///     DetectedBootloader::Grub
/// );
///
/// // Nothing found
/// assert_eq!(
///     detect_bootloader(false, false, false),
///     DetectedBootloader::Unknown
/// );
/// ```
pub fn detect_bootloader(
    loader_conf_exists: bool,
    _systemd_efi_exists: bool,
    grub_default_exists: bool,
) -> DetectedBootloader {
    if loader_conf_exists {
        return DetectedBootloader::SystemdBoot;
    }
    if grub_default_exists {
        return DetectedBootloader::Grub;
    }
    DetectedBootloader::Unknown
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_systemd_boot_full() {
        assert_eq!(
            detect_bootloader(true, true, false),
            DetectedBootloader::SystemdBoot
        );
    }

    #[test]
    fn detects_systemd_boot_loader_only() {
        assert_eq!(
            detect_bootloader(true, false, false),
            DetectedBootloader::SystemdBoot
        );
    }

    #[test]
    fn detects_grub_when_no_loader_conf() {
        assert_eq!(
            detect_bootloader(false, false, true),
            DetectedBootloader::Grub
        );
    }

    #[test]
    fn detects_unknown_when_nothing() {
        assert_eq!(
            detect_bootloader(false, false, false),
            DetectedBootloader::Unknown
        );
    }

    #[test]
    fn systemd_boot_takes_priority_over_grub() {
        assert_eq!(
            detect_bootloader(true, true, true),
            DetectedBootloader::SystemdBoot
        );
    }

    #[test]
    fn display_names_are_lowercase() {
        assert_eq!(DetectedBootloader::SystemdBoot.to_string(), "systemd-boot");
        assert_eq!(DetectedBootloader::Grub.to_string(), "grub");
        assert_eq!(DetectedBootloader::Unknown.to_string(), "unknown");
    }
}
