//! `BootManager` — the abstract interface every bootloader backend implements.
//!
//! ## Design invariants
//!
//! - All methods are pure: they operate on pre-read content strings, not file
//!   paths. No file I/O or system calls are performed inside this trait or its
//!   implementations under `crates/core`.
//! - The daemon reads files, hands raw content to the backend, and writes the
//!   returned strings using the standard atomic pipeline
//!   (`flock` → read → ETag verify → write `.tmp` → `fsync` → `rename`).
//! - `Send + Sync` bounds allow the daemon to store the backend behind a
//!   `Box<dyn BootManager>` in an async context.

#![deny(warnings)]
#![deny(missing_docs)]

use crate::error::BootControlError;

/// A parsed, in-memory representation of a single boot entry.
///
/// The fields are intentionally minimal — each backend provides whatever
/// information it can extract from its native config format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootEntry {
    /// Stable, unique identifier for this entry.
    ///
    /// For GRUB backends this is the GRUB key (e.g. `"GRUB_DEFAULT"`).
    /// For `systemd-boot` this is the filename stem of the loader entry
    /// (e.g. `"arch"` for `/boot/loader/entries/arch.conf`).
    pub id: String,

    /// Human-readable display label shown in UIs.
    pub label: String,

    /// Whether this entry is the currently configured default.
    pub is_default: bool,
}

/// Abstract interface for a bootloader backend.
///
/// All implementations must be **pure**: they accept raw file content
/// (`&str`) and return modified content or structured data. File I/O is
/// the responsibility of the **daemon layer**, not the backend.
///
/// # Examples
///
/// ```
/// use bootcontrol_core::boot_manager::{BootManager, BootEntry};
/// use bootcontrol_core::backends::grub::GrubBackend;
///
/// let backend = GrubBackend;
/// assert_eq!(backend.name(), "grub");
/// ```
pub trait BootManager: Send + Sync {
    /// Parse raw config content and return the list of boot entries.
    ///
    /// # Arguments
    ///
    /// * `content` — Raw text content of the bootloader's primary config file.
    ///
    /// # Errors
    ///
    /// Returns [`BootControlError::ComplexBashDetected`] if the content
    /// contains unsafe Bash constructs (GRUB backend only).
    /// Returns [`BootControlError::MalformedValue`] if the content cannot
    /// be parsed by this backend.
    fn list_entries(&self, content: &str) -> Result<Vec<BootEntry>, BootControlError>;

    /// Return a modified copy of `content` with `id` set as the default entry.
    ///
    /// Does **not** write to disk. The daemon calls this function and then
    /// atomically writes the returned string following the same
    /// `flock` + `fsync` + `rename` pipeline as `crates/daemon/src/grub_manager.rs`.
    ///
    /// # Arguments
    ///
    /// * `content` — Raw text content of the config file to modify.
    /// * `id`      — The entry identifier to set as default.
    ///
    /// # Errors
    ///
    /// Returns [`BootControlError::KeyNotFound`] if `id` does not refer to
    /// a known entry in the given content.
    /// Returns [`BootControlError::ComplexBashDetected`] or
    /// [`BootControlError::MalformedValue`] if the content is unparseable.
    fn set_default(&self, content: &str, id: &str) -> Result<String, BootControlError>;

    /// Compute the SHA-256 ETag of the given config content.
    ///
    /// Delegates to [`bootcontrol_core::hash::compute_etag_str`]. Do **not**
    /// re-implement hashing — call the shared function directly so all backends
    /// produce ETags in the same format.
    ///
    /// # Arguments
    ///
    /// * `content` — Raw text content to hash.
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrol_core::boot_manager::BootManager;
    /// use bootcontrol_core::backends::grub::GrubBackend;
    ///
    /// let backend = GrubBackend;
    /// let etag = backend.compute_etag("GRUB_TIMEOUT=5\n");
    /// assert_eq!(etag.len(), 64);
    /// ```
    fn compute_etag(&self, content: &str) -> String;

    /// Human-readable name of this backend, used by `GetActiveBackend()` D-Bus method.
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrol_core::boot_manager::BootManager;
    /// use bootcontrol_core::backends::grub::GrubBackend;
    ///
    /// assert_eq!(GrubBackend.name(), "grub");
    /// ```
    fn name(&self) -> &'static str;
}
