//! GRUB backend implementing [`crate::boot_manager::BootManager`].
//!
//! Wraps the existing [`crate::grub`] pure parser to expose GRUB as a
//! pluggable backend in the `BootManager` trait system.

#![deny(warnings)]
#![deny(missing_docs)]

use crate::{
    boot_manager::{BootEntry, BootManager},
    error::BootControlError,
    grub::parse_grub_config,
    hash::compute_etag_str,
};

/// GRUB bootloader backend.
///
/// Implements [`BootManager`] by delegating to the existing pure parser in
/// [`crate::grub`]. All operations are stateless and free of I/O.
///
/// # Examples
///
/// ```
/// use bootcontrol_core::backends::grub::GrubBackend;
/// use bootcontrol_core::boot_manager::BootManager;
///
/// let backend = GrubBackend;
/// assert_eq!(backend.name(), "grub");
/// let etag = backend.compute_etag("GRUB_TIMEOUT=5\n");
/// assert_eq!(etag.len(), 64);
/// ```
pub struct GrubBackend;

impl BootManager for GrubBackend {
    /// Parse a GRUB config and return one [`BootEntry`] per key-value pair.
    ///
    /// # Arguments
    ///
    /// * `content` — Raw text of `/etc/default/grub`.
    ///
    /// # Errors
    ///
    /// Returns [`BootControlError::ComplexBashDetected`] if any executable
    /// Bash construct is found. Returns [`BootControlError::MalformedValue`]
    /// if parsing fails for another reason.
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrol_core::backends::grub::GrubBackend;
    /// use bootcontrol_core::boot_manager::BootManager;
    ///
    /// let backend = GrubBackend;
    /// let entries = backend.list_entries("GRUB_TIMEOUT=5\nGRUB_DEFAULT=0\n").unwrap();
    /// assert_eq!(entries.len(), 2);
    /// ```
    fn list_entries(&self, content: &str) -> Result<Vec<BootEntry>, BootControlError> {
        let config = parse_grub_config(content)?;
        let mut entries: Vec<BootEntry> = config
            .map
            .into_iter()
            .map(|(key, value)| BootEntry {
                id: key.clone(),
                label: format!("{key} = {value}"),
                is_default: false,
            })
            .collect();
        // Stable sort so tests are deterministic.
        entries.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(entries)
    }

    /// Return a modified copy of the GRUB config with `GRUB_DEFAULT` set to `id`.
    ///
    /// # Arguments
    ///
    /// * `content` — Raw text of `/etc/default/grub`.
    /// * `id`      — New value for `GRUB_DEFAULT` (e.g. `"1"` or `"saved"`).
    ///
    /// # Errors
    ///
    /// Returns [`BootControlError::ComplexBashDetected`] if the file contains
    /// unsupported Bash constructs.
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrol_core::backends::grub::GrubBackend;
    /// use bootcontrol_core::boot_manager::BootManager;
    ///
    /// let backend = GrubBackend;
    /// let content = "GRUB_DEFAULT=0\nGRUB_TIMEOUT=5\n";
    /// let modified = backend.set_default(content, "saved").unwrap();
    /// assert!(modified.contains("GRUB_DEFAULT=saved"));
    /// ```
    fn set_default(&self, content: &str, id: &str) -> Result<String, BootControlError> {
        let config = parse_grub_config(content)?;
        let mut lines = config.lines;

        let new_line = format!("GRUB_DEFAULT={id}");
        match lines
            .iter()
            .rposition(|l| l.trim_start().starts_with("GRUB_DEFAULT="))
        {
            Some(idx) => lines[idx] = new_line,
            None => lines.push(new_line),
        }

        Ok(lines.join("\n") + "\n")
    }

    /// Compute the SHA-256 ETag of the raw GRUB config content.
    ///
    /// # Arguments
    ///
    /// * `content` — Raw text to hash.
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrol_core::backends::grub::GrubBackend;
    /// use bootcontrol_core::boot_manager::BootManager;
    /// use bootcontrol_core::hash::compute_etag_str;
    ///
    /// let backend = GrubBackend;
    /// let content = "GRUB_TIMEOUT=5\n";
    /// assert_eq!(backend.compute_etag(content), compute_etag_str(content));
    /// ```
    fn compute_etag(&self, content: &str) -> String {
        compute_etag_str(content)
    }

    fn name(&self) -> &'static str {
        "grub"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIMPLE: &str = "GRUB_DEFAULT=0\nGRUB_TIMEOUT=5\n";

    #[test]
    fn name_returns_grub() {
        assert_eq!(GrubBackend.name(), "grub");
    }

    #[test]
    fn compute_etag_delegates_to_hash_module() {
        let etag = GrubBackend.compute_etag(SIMPLE);
        assert_eq!(etag, compute_etag_str(SIMPLE));
        assert_eq!(etag.len(), 64);
    }

    #[test]
    fn list_entries_returns_all_keys() {
        let entries = GrubBackend.list_entries(SIMPLE).unwrap();
        assert_eq!(entries.len(), 2);
        let ids: Vec<&str> = entries.iter().map(|e| e.id.as_str()).collect();
        assert!(ids.contains(&"GRUB_DEFAULT"));
        assert!(ids.contains(&"GRUB_TIMEOUT"));
    }

    #[test]
    fn list_entries_rejects_complex_bash() {
        let result = GrubBackend.list_entries("GRUB_TIMEOUT=$(uname -r)\n");
        assert!(matches!(
            result,
            Err(BootControlError::ComplexBashDetected { .. })
        ));
    }

    #[test]
    fn set_default_updates_existing_key() {
        let result = GrubBackend.set_default(SIMPLE, "saved").unwrap();
        assert!(result.contains("GRUB_DEFAULT=saved"));
        assert!(!result.contains("GRUB_DEFAULT=0"));
    }

    #[test]
    fn set_default_appends_when_key_absent() {
        let content = "GRUB_TIMEOUT=5\n";
        let result = GrubBackend.set_default(content, "1").unwrap();
        assert!(result.contains("GRUB_DEFAULT=1"));
    }

    #[test]
    fn set_default_rejects_complex_bash() {
        let result = GrubBackend.set_default("$(evil)\n", "0");
        assert!(matches!(
            result,
            Err(BootControlError::ComplexBashDetected { .. })
        ));
    }
}
