// The daemon is Linux-only by design: it depends on `nix::fcntl::flock`,
// efivarfs at `/sys/firmware/efi/efivars`, systemd socket activation,
// Polkit, and D-Bus name `org.bootcontrol.Manager` on the system bus.
// On Windows the analogous code lives in the per-frontend binaries that
// call `SetFirmwareEnvironmentVariableExW` directly with the user's UAC
// elevation. That port is out of scope for this crate.
//
// Gate the entire lib content on Linux so `cargo build --workspace` on
// any other target produces an empty (but valid) library and links
// against the `main.rs` Windows stub. See Phase 7 PR4-6 in ROADMAP.md.
#![cfg(target_os = "linux")]

//! `bootcontrold` — privileged D-Bus backend for BootControl.
//!
//! This crate implements the system daemon that reads and writes
//! `/etc/default/grub`. All user-space frontends (CLI, TUI, GUI) communicate
//! with this daemon exclusively over D-Bus. The daemon never exposes a raw
//! filesystem API.
//!
//! # Module layout
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | [`interface`] | D-Bus `org.bootcontrol.Manager` interface implementation. |
//! | [`grub_manager`] | Filesystem read/write with atomic write and flock. |
//! | [`grub_rebuild`] | Locate and invoke `grub-mkconfig` to regenerate `/boot/grub/grub.cfg`. |
//! | [`failsafe`] | Golden-parachute GRUB entry generator written after every successful write. |
//! | [`policy_check`] | Startup validation of the polkit policy file (refuse to start on stale policy). |
//! | [`polkit`] | Polkit authorization (mock or real, feature-gated). |
//! | [`sanitize`] | Payload blacklist enforcement. |
//! | [`dbus_error`] | `BootControlError` → `zbus::fdo::Error` mapping. |
//! | [`secureboot`] | Secure Boot utilities: NVRAM backup, MOK signing and enrollment. |
//! | [`systemd_boot_manager`] | Filesystem read/write for systemd-boot loader entries. |
//! | [`uki_manager`] | Filesystem read/write for `/etc/kernel/cmdline`. |
//! | [`snapshot`] | Pre-write filesystem snapshots (PR 5). Not yet integrated into write-paths. |
//! | [`audit`] | Structured journald audit emission (PR 5). Not yet integrated into write-paths. |

#![deny(missing_docs)]

pub mod audit;
pub mod dbus_error;
pub mod failsafe;
pub mod grub_manager;
pub mod grub_rebuild;
pub mod immutable_distro;
pub mod initramfs;
pub mod interface;
pub mod luks_keymap;
pub mod policy_check;
pub mod polkit;
pub mod prober;
pub mod rpm_ostree;
pub mod sanitize;
pub mod secureboot;
pub mod snapshot;
pub mod systemd_boot_manager;
pub mod uefi_vars_linux;
pub mod uki_manager;
