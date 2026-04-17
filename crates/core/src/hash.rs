//! Stateless SHA-256 hashing and ETag generation for BootControl.
//!
//! This module is the single source of truth for file identity throughout
//! BootControl's optimistic concurrency control loop:
//!
//! 1. The daemon reads a file and calls [`compute_etag`] to derive its current
//!    identity (ETag), which is returned to the caller together with the file
//!    contents.
//! 2. The caller sends a write request that includes the ETag it received.
//! 3. Before applying the write, the daemon recomputes the ETag and compares
//!    it to the one in the request. A mismatch means the file has been
//!    modified externally → reject with
//!    [`BootControlError::StateMismatch`](crate::error::BootControlError::StateMismatch).
//!
//! # No I/O
//!
//! All functions in this module are **pure**: they accept bytes or strings and
//! return deterministic values. Disk I/O is the daemon's responsibility.

use sha2::{Digest, Sha256};

/// Compute a lowercase hex-encoded SHA-256 ETag from raw bytes.
///
/// The ETag is a 64-character lowercase hexadecimal string representing the
/// SHA-256 digest of `content`. It is used for optimistic concurrency control
/// on every BootControl write operation.
///
/// # Arguments
///
/// * `content` — The raw bytes of the file to hash. Passing an empty slice is
///   valid and returns the well-known SHA-256 hash of the empty string.
///
/// # Examples
///
/// ```
/// use bootcontrol_core::hash::compute_etag;
///
/// let etag = compute_etag(b"GRUB_TIMEOUT=5\n");
/// // ETag is always exactly 64 lowercase hex characters.
/// assert_eq!(etag.len(), 64);
/// assert!(etag.chars().all(|c| c.is_ascii_hexdigit()));
/// ```
///
/// Two identical byte sequences always produce the same ETag:
///
/// ```
/// use bootcontrol_core::hash::compute_etag;
///
/// let a = compute_etag(b"hello");
/// let b = compute_etag(b"hello");
/// assert_eq!(a, b);
/// ```
///
/// Different byte sequences produce different ETags with overwhelming probability:
///
/// ```
/// use bootcontrol_core::hash::compute_etag;
///
/// assert_ne!(compute_etag(b"hello"), compute_etag(b"world"));
/// ```
pub fn compute_etag(content: &[u8]) -> String {
    let digest = Sha256::digest(content);
    hex::encode(digest)
}

/// Compute a lowercase hex-encoded SHA-256 ETag from a UTF-8 string slice.
///
/// This is a convenience wrapper around [`compute_etag`] for callers that
/// already have the file contents as `&str` (e.g., after parsing). The ETag
/// is computed from the UTF-8 byte representation of the string, which is
/// identical to what you would get by calling
/// `compute_etag(content.as_bytes())`.
///
/// # Arguments
///
/// * `content` — The raw text contents of the file to hash.
///
/// # Examples
///
/// ```
/// use bootcontrol_core::hash::{compute_etag, compute_etag_str};
///
/// let text = "GRUB_TIMEOUT=5\nGRUB_DEFAULT=0\n";
/// assert_eq!(compute_etag_str(text), compute_etag(text.as_bytes()));
/// ```
pub fn compute_etag_str(content: &str) -> String {
    compute_etag(content.as_bytes())
}

/// Verify that a caller-supplied ETag matches the ETag computed from
/// `current_content`.
///
/// Returns `true` if the ETags match (file has not changed), `false`
/// otherwise. The daemon must call this before every write; if this function
/// returns `false`, the write must be aborted and
/// [`BootControlError::StateMismatch`](crate::error::BootControlError::StateMismatch)
/// must be returned to the caller.
///
/// # Arguments
///
/// * `claimed_etag` — The ETag sent by the caller in the write request.
/// * `current_content` — The raw bytes of the file as it exists on disk **at
///   write time** (read the file again immediately before calling this).
///
/// # Examples
///
/// ```
/// use bootcontrol_core::hash::{compute_etag, verify_etag};
///
/// let content = b"GRUB_TIMEOUT=5\n";
/// let etag = compute_etag(content);
///
/// // Happy path: ETag is still valid.
/// assert!(verify_etag(&etag, content));
///
/// // Stale ETag: file was modified externally.
/// let modified_content = b"GRUB_TIMEOUT=10\n";
/// assert!(!verify_etag(&etag, modified_content));
/// ```
pub fn verify_etag(claimed_etag: &str, current_content: &[u8]) -> bool {
    let actual = compute_etag(current_content);
    // Use a constant-time-ish comparison to avoid leaking information via
    // timing side-channels. For ETags this is not a security-critical secret,
    // but the habit is good practice at the systems level.
    claimed_etag.len() == actual.len()
        && claimed_etag
            .bytes()
            .zip(actual.bytes())
            .all(|(a, b)| a == b)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ────────────────────────────────────────────────────────────────────────
    // compute_etag
    // ────────────────────────────────────────────────────────────────────────

    /// The output must be exactly 64 lowercase hex characters (SHA-256 = 32 bytes).
    #[test]
    fn etag_is_64_hex_chars() {
        let etag = compute_etag(b"any content");
        assert_eq!(etag.len(), 64);
        assert!(etag.chars().all(|c| c.is_ascii_hexdigit()));
    }

    /// SHA-256 of the empty string is a well-known constant.
    #[test]
    fn etag_of_empty_bytes_is_known_constant() {
        // echo -n "" | sha256sum
        let expected = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        assert_eq!(compute_etag(b""), expected);
    }

    /// Identical inputs must produce identical ETags (pure/deterministic).
    #[test]
    fn etag_is_deterministic() {
        let a = compute_etag(b"GRUB_TIMEOUT=5\n");
        let b = compute_etag(b"GRUB_TIMEOUT=5\n");
        assert_eq!(a, b);
    }

    /// Different inputs must produce different ETags.
    #[test]
    fn different_content_produces_different_etag() {
        let a = compute_etag(b"GRUB_TIMEOUT=5\n");
        let b = compute_etag(b"GRUB_TIMEOUT=10\n");
        assert_ne!(a, b);
    }

    /// A single-byte change must change the ETag (avalanche effect sanity check).
    #[test]
    fn single_byte_change_changes_etag() {
        let base = b"GRUB_DEFAULT=0";
        let mut modified = base.to_vec();
        modified[13] = b'1'; // change '0' → '1'
        assert_ne!(compute_etag(base), compute_etag(&modified));
    }

    // ────────────────────────────────────────────────────────────────────────
    // compute_etag_str
    // ────────────────────────────────────────────────────────────────────────

    /// compute_etag_str must equal compute_etag on the same UTF-8 bytes.
    #[test]
    fn etag_str_equals_bytes_etag() {
        let text = "GRUB_CMDLINE_LINUX_DEFAULT=\"quiet splash\"\n";
        assert_eq!(compute_etag_str(text), compute_etag(text.as_bytes()));
    }

    // ────────────────────────────────────────────────────────────────────────
    // verify_etag
    // ────────────────────────────────────────────────────────────────────────

    /// The "happy path": a fresh ETag must verify against the same content.
    #[test]
    fn verify_etag_accepts_correct_etag() {
        let content = b"GRUB_TIMEOUT=5\nGRUB_DEFAULT=0\n";
        let etag = compute_etag(content);
        assert!(verify_etag(&etag, content));
    }

    /// A stale ETag must be rejected when the content has changed.
    #[test]
    fn verify_etag_rejects_stale_etag() {
        let original = b"GRUB_TIMEOUT=5\n";
        let etag = compute_etag(original);
        let modified = b"GRUB_TIMEOUT=10\n";
        assert!(!verify_etag(&etag, modified));
    }

    /// An entirely wrong ETag (e.g. all zeros) must be rejected.
    #[test]
    fn verify_etag_rejects_wrong_etag() {
        let content = b"GRUB_TIMEOUT=5\n";
        let wrong_etag = "0".repeat(64);
        assert!(!verify_etag(&wrong_etag, content));
    }

    /// An ETag of wrong length must be rejected without panicking.
    #[test]
    fn verify_etag_rejects_wrong_length() {
        let content = b"GRUB_TIMEOUT=5\n";
        assert!(!verify_etag("tooshort", content));
        assert!(!verify_etag(&"a".repeat(65), content));
    }

    /// The empty file has a well-known ETag; verify must confirm it.
    #[test]
    fn verify_etag_works_for_empty_content() {
        let expected = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        assert!(verify_etag(expected, b""));
    }
}
