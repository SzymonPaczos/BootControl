//! Bootloader backend implementations.
//!
//! Each module implements the [`crate::boot_manager::BootManager`] trait for
//! a specific bootloader. Backends are pure — they perform no I/O.
//!
//! | Module | Bootloader |
//! |--------|-----------|
//! | [`grub`] | GRUB — parses `/etc/default/grub` |
//! | [`systemd_boot`] | systemd-boot — parses `/boot/loader/entries/*.conf` |
//! | [`uki`] | UKI — manages `/etc/kernel/cmdline` |

#![deny(warnings)]
#![deny(missing_docs)]

pub mod grub;
pub mod systemd_boot;
pub mod uki;
