//! Payload sanitization for GRUB configuration write operations.
//!
//! Re-exports from [`bootcontrol_core::security`] (single source of truth for
//! the kernel cmdline blacklist). This module exists so daemon-internal
//! callers keep their `crate::sanitize::*` import paths; all logic lives in
//! `core::security`.
//!
//! # Threat model
//!
//! A malicious user-space application could send a `SetGrubValue` request with
//! a key or value containing kernel parameters that disable security
//! subsystems at boot time (e.g. `selinux=0`, `apparmor=0`, `init=/bin/bash`).
//! Even though Polkit authorizes the requesting user, we must refuse payloads
//! that would weaken system security. See [`bootcontrol_core::security`] for
//! the canonical blacklist and rationale.

/// Blacklisted substrings for GRUB key and value payloads. Single source of
/// truth: [`bootcontrol_core::security::KERNEL_CMDLINE_BLACKLIST`].
pub use bootcontrol_core::security::KERNEL_CMDLINE_BLACKLIST as BLACKLISTED_PATTERNS;

/// Verify that a GRUB key-value pair does not violate the security policy.
/// Thin re-export of [`bootcontrol_core::security::validate_grub_payload`] so
/// daemon callers keep their existing import paths.
pub use bootcontrol_core::security::validate_grub_payload as check_payload;

#[cfg(test)]
mod tests {
    // Test suite preserved verbatim from the pre-consolidation sanitize.rs —
    // the public surface (`check_payload`, `BLACKLISTED_PATTERNS`) is now a
    // re-export of `bootcontrol_core::security`, and these tests still pass
    // because the canonical implementation behind the re-export emits
    // identical error messages. Treating them as regression coverage for
    // the daemon-side import path.
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
