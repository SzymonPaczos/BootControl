//! # `bootcontrol-core`
//!
//! Pure logic library for BootControl — no I/O, no system calls, no global
//! state. Every function in this crate is a pure function or a stateless
//! computation that can be unit-tested without root privileges or a real
//! filesystem.
//!
//! ## Modules
//!
//! | Module | Responsibility |
//! |--------|---------------|
//! | [`error`] | The canonical [`error::BootControlError`] enum. All other crates return this type; the daemon maps it 1:1 to D-Bus error names. |
//! | [`hash`] | SHA-256 ETag generation and stateless concurrency verification. |
//! | [`grub`] | Strict-subset parser for `/etc/default/grub`. Bails out on any executable Bash construct. |
//!
//! ## Design constraints
//!
//! - **No `unwrap()` / `expect()`** anywhere in production code paths.
//! - **No I/O** — callers (the daemon) are responsible for reading files and
//!   passing the raw bytes/string into these functions.
//! - **Pure functions** — every public function is deterministic and free of
//!   side effects, making it straightforward to test under `cfg(test)` without
//!   mocks.

#![deny(warnings)]
#![deny(missing_docs)]

pub mod error;
pub mod grub;
pub mod hash;
