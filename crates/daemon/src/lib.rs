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
//! | [`polkit`] | Polkit authorization (mock or real, feature-gated). |
//! | [`sanitize`] | Payload blacklist enforcement. |
//! | [`dbus_error`] | `BootControlError` → `zbus::fdo::Error` mapping. |

#![deny(warnings)]
#![deny(missing_docs)]

pub mod dbus_error;
pub mod grub_manager;
pub mod interface;
pub mod polkit;
pub mod sanitize;
