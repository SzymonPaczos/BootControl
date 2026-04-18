//! Payload sanitization for GRUB configuration write operations.
//!
//! This module enforces the security blacklist that prevents an unprivileged
//! user from smuggling dangerous kernel parameters through the D-Bus interface.
//!
//! # Threat model
//!
//! A malicious user-space application could send a `SetGrubValue` request with
//! a key or value containing kernel parameters that disable security
//! subsystems at boot time (e.g. `selinux=0`, `apparmor=0`, `init=/bin/bash`).
//! Even though Polkit authorizes the requesting user, we must refuse payloads
//! that would weaken system security.
//!
//! The blacklist is a named compile-time constant, not inline strings, so that
//! auditors can find and review it from a single location.

use bootcontrol_core::error::BootControlError;

/// Blacklisted substrings for GRUB key and value payloads.
///
/// Any key or value that **contains** one of these substrings (case-sensitive)
/// is rejected with [`BootControlError::SecurityPolicyViolation`] before any
/// file write is attempted.
///
/// The list is intentionally conservative: it blocks the most dangerous
/// kernel parameters while avoiding false positives on legitimate values.
pub const BLACKLISTED_PATTERNS: &[&str] = &[
    "init=",
    "selinux=0",
    "apparmor=0",
    "systemd.unit=",
    "rd.break",
    "single",
    "emergency",
];

/// Verify that a GRUB key-value pair does not violate the security policy.
///
/// Both `key` and `value` are checked against [`BLACKLISTED_PATTERNS`]. The
/// check is case-sensitive and substring-based: if either string contains any
/// blacklisted pattern as a substring, the operation is rejected.
///
/// # Arguments
///
/// * `key`   — The GRUB configuration key to write (e.g. `"GRUB_CMDLINE_LINUX_DEFAULT"`).
/// * `value` — The value to assign to the key (e.g. `"quiet splash"`).
///
/// # Errors
///
/// Returns [`BootControlError::SecurityPolicyViolation`] if `key` or `value`
/// contains any substring from [`BLACKLISTED_PATTERNS`].
///
/// # Examples
///
/// ```
/// use bootcontrold::sanitize::check_payload;
///
/// // Safe values are accepted.
/// assert!(check_payload("GRUB_TIMEOUT", "5").is_ok());
/// assert!(check_payload("GRUB_CMDLINE_LINUX_DEFAULT", "quiet splash").is_ok());
///
/// // Dangerous values are rejected.
/// assert!(check_payload("GRUB_CMDLINE_LINUX_DEFAULT", "init=/bin/bash").is_err());
/// assert!(check_payload("GRUB_CMDLINE_LINUX_DEFAULT", "selinux=0").is_err());
/// ```
pub fn check_payload(key: &str, value: &str) -> Result<(), BootControlError> {
    for &pattern in BLACKLISTED_PATTERNS {
        if key.contains(pattern) {
            return Err(BootControlError::SecurityPolicyViolation {
                reason: format!("key '{key}' contains blacklisted pattern '{pattern}'"),
            });
        }
        if value.contains(pattern) {
            return Err(BootControlError::SecurityPolicyViolation {
                reason: format!("value for key '{key}' contains blacklisted pattern '{pattern}'"),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bootcontrol_core::error::BootControlError;

    // ── Allowed payloads ──────────────────────────────────────────────────────

    #[test]
    fn safe_timeout_is_allowed() {
        assert!(check_payload("GRUB_TIMEOUT", "5").is_ok());
    }

    #[test]
    fn safe_cmdline_is_allowed() {
        assert!(check_payload("GRUB_CMDLINE_LINUX_DEFAULT", "quiet splash").is_ok());
    }

    #[test]
    fn safe_empty_value_is_allowed() {
        assert!(check_payload("GRUB_CMDLINE_LINUX_DEFAULT", "").is_ok());
    }

    #[test]
    fn safe_distributor_is_allowed() {
        assert!(check_payload("GRUB_DISTRIBUTOR", "Ubuntu").is_ok());
    }

    // ── Blocked by value ──────────────────────────────────────────────────────

    #[test]
    fn blocks_init_in_value() {
        let result = check_payload("GRUB_CMDLINE_LINUX_DEFAULT", "quiet init=/bin/sh splash");
        assert!(matches!(
            result,
            Err(BootControlError::SecurityPolicyViolation { .. })
        ));
    }

    #[test]
    fn blocks_selinux_zero_in_value() {
        let result = check_payload("GRUB_CMDLINE_LINUX_DEFAULT", "selinux=0 quiet");
        assert!(matches!(
            result,
            Err(BootControlError::SecurityPolicyViolation { .. })
        ));
    }

    #[test]
    fn blocks_apparmor_zero_in_value() {
        let result = check_payload("GRUB_CMDLINE_LINUX_DEFAULT", "apparmor=0");
        assert!(matches!(
            result,
            Err(BootControlError::SecurityPolicyViolation { .. })
        ));
    }

    #[test]
    fn blocks_systemd_unit_in_value() {
        let result = check_payload("GRUB_CMDLINE_LINUX_DEFAULT", "systemd.unit=rescue.target");
        assert!(matches!(
            result,
            Err(BootControlError::SecurityPolicyViolation { .. })
        ));
    }

    #[test]
    fn blocks_rd_break_in_value() {
        let result = check_payload("GRUB_CMDLINE_LINUX_DEFAULT", "rd.break quiet");
        assert!(matches!(
            result,
            Err(BootControlError::SecurityPolicyViolation { .. })
        ));
    }

    #[test]
    fn blocks_single_in_value() {
        let result = check_payload("GRUB_CMDLINE_LINUX_DEFAULT", "single");
        assert!(matches!(
            result,
            Err(BootControlError::SecurityPolicyViolation { .. })
        ));
    }

    #[test]
    fn blocks_emergency_in_value() {
        let result = check_payload("GRUB_CMDLINE_LINUX_DEFAULT", "emergency");
        assert!(matches!(
            result,
            Err(BootControlError::SecurityPolicyViolation { .. })
        ));
    }

    // ── Blocked by key ────────────────────────────────────────────────────────

    #[test]
    fn blocks_selinux_zero_in_key() {
        let result = check_payload("selinux=0_key", "somevalue");
        assert!(matches!(
            result,
            Err(BootControlError::SecurityPolicyViolation { .. })
        ));
    }

    // ── Edge cases ────────────────────────────────────────────────────────────

    #[test]
    fn blocks_init_embedded_in_longer_string() {
        // The pattern "init=" must be rejected even when embedded in a longer string.
        let result = check_payload(
            "GRUB_CMDLINE_LINUX_DEFAULT",
            "quiet noinit=something init=/usr/lib/systemd/systemd",
        );
        assert!(matches!(
            result,
            Err(BootControlError::SecurityPolicyViolation { .. })
        ));
    }

    #[test]
    fn single_not_blocked_as_prefix_of_nonisolated_word() {
        // "single" appears as a standalone pattern — even embedded in a value it
        // triggers the guard because substring matching is intentionally strict.
        // This test documents the intended behavior.
        let result = check_payload("GRUB_CMDLINE_LINUX_DEFAULT", "nosingle");
        // "nosingle" DOES contain the substring "single", so it IS blocked.
        assert!(matches!(
            result,
            Err(BootControlError::SecurityPolicyViolation { .. })
        ));
    }

    #[test]
    fn all_blacklisted_patterns_are_tested() {
        // Ensure the BLACKLISTED_PATTERNS constant is non-empty and every
        // documented entry is present.
        assert!(BLACKLISTED_PATTERNS.contains(&"init="));
        assert!(BLACKLISTED_PATTERNS.contains(&"selinux=0"));
        assert!(BLACKLISTED_PATTERNS.contains(&"apparmor=0"));
        assert!(BLACKLISTED_PATTERNS.contains(&"systemd.unit="));
        assert!(BLACKLISTED_PATTERNS.contains(&"rd.break"));
        assert!(BLACKLISTED_PATTERNS.contains(&"single"));
        assert!(BLACKLISTED_PATTERNS.contains(&"emergency"));
        assert_eq!(BLACKLISTED_PATTERNS.len(), 7);
    }
}
