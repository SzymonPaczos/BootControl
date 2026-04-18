//! Workspace-level end-to-end test suite for BootControl.
//!
//! # Overview
//!
//! These tests spawn a real `bootcontrold` binary on the D-Bus **session bus**
//! and drive it through the full call chain:
//!
//! ```text
//! [test process] ──zbus──► [bootcontrold (session bus, polkit-mock)]
//!                                  │
//!                          BOOTCONTROL_GRUB_PATH=<tempfile>
//!                          BOOTCONTROL_FAILSAFE_PATH=<tempdir>
//! ```
//!
//! # Running
//!
//! ```bash
//! # All E2E tests (requires a running session bus):
//! BOOTCONTROL_BUS=session cargo test --test e2e -- --ignored
//!
//! # With log output:
//! BOOTCONTROL_BUS=session cargo test --test e2e -- --ignored --nocapture
//! ```
//!
//! # Design constraints (from AGENT.md)
//!
//! - Every test is `#[ignore]` by default — `cargo test --workspace` stays fast.
//! - No `unwrap()` in helper code — all errors propagate via `?`.
//! - Tests are Linux-only and will not compile on macOS or Windows.

// ── Linux-only gate ──────────────────────────────────────────────────────────
// D-Bus + session bus + POSIX flock semantics are Linux-specific.
// The entire test binary is excluded on other platforms at compile time.
#![cfg(target_os = "linux")]

mod concurrent_write;
mod etag_mismatch;
mod grub_roundtrip;
mod secureboot_mok;
#[cfg(feature = "experimental_paranoia")]
mod secureboot_paranoia;
pub mod helpers;
