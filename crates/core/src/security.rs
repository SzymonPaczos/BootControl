//! Shared security primitives — the single source of truth for the kernel
//! command-line blacklist used by all write paths.
//!
//! # Why this lives in `bootcontrol-core`
//!
//! Both [`crate::backends::uki::validate_kernel_param`] (UKI cmdline writes)
//! and `bootcontrold::sanitize::check_payload` (GRUB env writes, rpm-ostree
//! kargs delegation) reject the same substrings. Before this module existed,
//! the list lived in two places — `crates/daemon/src/sanitize.rs` and
//! `crates/core/src/backends/uki.rs` — with a "must be kept in sync" comment
//! holding the contract by hand. Any new dangerous pattern added to one and
//! forgotten in the other would leave a silent gap on at least one backend.
//!
//! The constant now lives here. The crate layering still forbids `daemon →`
//! anything but `core`, and `core` cannot grow `daemon` dependencies, so this
//! is the only crate that can host the shared definition.
//!
//! # What's blacklisted and why
//!
//! Substrings (case-sensitive) that, if smuggled into a kernel parameter or
//! into the value half of a GRUB key=value pair, would weaken or bypass
//! system security at boot:
//!
//! | Pattern | Threat |
//! |---------|--------|
//! | `init=` | Replaces PID 1 (`init=/bin/sh` → root shell before anything starts). |
//! | `selinux=0` | Disables SELinux LSM. |
//! | `apparmor=0` | Disables AppArmor LSM. |
//! | `systemd.unit=` | Redirects boot target (`rescue.target`, `emergency.target` skip multi-user). |
//! | `rd.break` | Drops initramfs into a debug shell before the real root is mounted. |
//! | `single` | Single-user (no networking, no most services). |
//! | `emergency` | Emergency target (even more minimal than `rescue`). |

use crate::error::BootControlError;

/// Blacklisted substrings for kernel command-line / GRUB config payloads.
///
/// The check is **case-sensitive** and **substring-based**: if a checked
/// string contains any element of this slice anywhere, the operation is
/// rejected. Adding a new pattern here propagates to every caller —
/// `daemon::sanitize::check_payload` and `core::backends::uki::validate_kernel_param`
/// both delegate to [`contains_blacklisted_substring`] / [`validate_kernel_param_str`].
pub const KERNEL_CMDLINE_BLACKLIST: &[&str] = &[
    "init=",
    "selinux=0",
    "apparmor=0",
    "systemd.unit=",
    "rd.break",
    "single",
    "emergency",
];

/// If `haystack` contains any blacklisted substring from
/// [`KERNEL_CMDLINE_BLACKLIST`], return the matched pattern. Otherwise `None`.
///
/// Callers use this to decide whether to reject a payload — the matched
/// pattern is included in the error message so the user knows exactly which
/// rule fired.
///
/// # Examples
///
/// ```
/// use bootcontrol_core::security::first_blacklisted_match;
///
/// assert_eq!(first_blacklisted_match("quiet splash"), None);
/// assert_eq!(first_blacklisted_match("init=/bin/sh quiet"), Some("init="));
/// assert_eq!(first_blacklisted_match("apparmor=0"), Some("apparmor=0"));
/// ```
pub fn first_blacklisted_match(haystack: &str) -> Option<&'static str> {
    KERNEL_CMDLINE_BLACKLIST
        .iter()
        .copied()
        .find(|&pattern| haystack.contains(pattern))
}

/// Validate a single kernel parameter against [`KERNEL_CMDLINE_BLACKLIST`].
///
/// This is the shape used by callers that take *one* parameter at a time
/// (UKI `add_kernel_param`, rpm-ostree `kargs_append`). Callers that take
/// a GRUB key=value pair use [`validate_grub_payload`] for richer error
/// messages.
///
/// # Errors
///
/// Returns [`BootControlError::SecurityPolicyViolation`] if `param` contains
/// any substring from [`KERNEL_CMDLINE_BLACKLIST`].
///
/// # Examples
///
/// ```
/// use bootcontrol_core::security::validate_kernel_param_str;
///
/// assert!(validate_kernel_param_str("quiet").is_ok());
/// assert!(validate_kernel_param_str("root=/dev/sda1").is_ok());
/// assert!(validate_kernel_param_str("selinux=0").is_err());
/// assert!(validate_kernel_param_str("init=/bin/bash").is_err());
/// ```
pub fn validate_kernel_param_str(param: &str) -> Result<(), BootControlError> {
    if let Some(pattern) = first_blacklisted_match(param) {
        return Err(BootControlError::SecurityPolicyViolation {
            reason: format!("kernel parameter '{param}' contains blacklisted pattern '{pattern}'"),
        });
    }
    Ok(())
}

/// Validate a GRUB `key=value` write against [`KERNEL_CMDLINE_BLACKLIST`].
///
/// Both halves of the pair are checked separately so that the error message
/// can distinguish "rejected key" from "rejected value" — useful for the
/// user when the GUI surfaces the violation.
///
/// # Errors
///
/// Returns [`BootControlError::SecurityPolicyViolation`] if either `key` or
/// `value` contains any substring from [`KERNEL_CMDLINE_BLACKLIST`].
///
/// # Examples
///
/// ```
/// use bootcontrol_core::security::validate_grub_payload;
///
/// assert!(validate_grub_payload("GRUB_TIMEOUT", "5").is_ok());
/// assert!(validate_grub_payload("GRUB_CMDLINE_LINUX_DEFAULT", "quiet splash").is_ok());
/// assert!(validate_grub_payload("GRUB_CMDLINE_LINUX_DEFAULT", "init=/bin/bash").is_err());
/// assert!(validate_grub_payload("selinux=0_key", "anything").is_err());
/// ```
pub fn validate_grub_payload(key: &str, value: &str) -> Result<(), BootControlError> {
    if let Some(pattern) = first_blacklisted_match(key) {
        return Err(BootControlError::SecurityPolicyViolation {
            reason: format!("key '{key}' contains blacklisted pattern '{pattern}'"),
        });
    }
    if let Some(pattern) = first_blacklisted_match(value) {
        return Err(BootControlError::SecurityPolicyViolation {
            reason: format!("value for key '{key}' contains blacklisted pattern '{pattern}'"),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Blacklist completeness ────────────────────────────────────────────────

    #[test]
    fn blacklist_has_seven_documented_entries() {
        // Pinned: anybody adding a new pattern must update this test (and the
        // module docs table). Keeps the single source of truth honest.
        assert_eq!(KERNEL_CMDLINE_BLACKLIST.len(), 7);
        assert!(KERNEL_CMDLINE_BLACKLIST.contains(&"init="));
        assert!(KERNEL_CMDLINE_BLACKLIST.contains(&"selinux=0"));
        assert!(KERNEL_CMDLINE_BLACKLIST.contains(&"apparmor=0"));
        assert!(KERNEL_CMDLINE_BLACKLIST.contains(&"systemd.unit="));
        assert!(KERNEL_CMDLINE_BLACKLIST.contains(&"rd.break"));
        assert!(KERNEL_CMDLINE_BLACKLIST.contains(&"single"));
        assert!(KERNEL_CMDLINE_BLACKLIST.contains(&"emergency"));
    }

    // ── first_blacklisted_match ───────────────────────────────────────────────

    #[test]
    fn first_match_finds_init_anywhere() {
        assert_eq!(
            first_blacklisted_match("quiet init=/bin/sh splash"),
            Some("init=")
        );
    }

    #[test]
    fn first_match_returns_none_for_clean_string() {
        assert_eq!(
            first_blacklisted_match("quiet splash root=/dev/sda1 rw"),
            None
        );
    }

    #[test]
    fn first_match_is_first_pattern_in_slice_order() {
        // `init=` precedes `selinux=0` in KERNEL_CMDLINE_BLACKLIST so a string
        // containing both reports init= first. Documents the stable iteration
        // order in case callers come to depend on it.
        assert_eq!(
            first_blacklisted_match("init=/bin/sh selinux=0"),
            Some("init=")
        );
    }

    // ── validate_kernel_param_str ─────────────────────────────────────────────

    #[test]
    fn kernel_param_allows_safe_tokens() {
        assert!(validate_kernel_param_str("quiet").is_ok());
        assert!(validate_kernel_param_str("root=/dev/sda1").is_ok());
        assert!(validate_kernel_param_str("loglevel=3").is_ok());
        assert!(validate_kernel_param_str("rw").is_ok());
    }

    #[test]
    fn kernel_param_rejects_each_blacklisted_pattern() {
        for &pattern in KERNEL_CMDLINE_BLACKLIST {
            let result = validate_kernel_param_str(pattern);
            assert!(
                matches!(
                    result,
                    Err(BootControlError::SecurityPolicyViolation { .. })
                ),
                "pattern {pattern:?} should be blacklisted but validate accepted it"
            );
        }
    }

    #[test]
    fn kernel_param_rejects_blacklisted_embedded() {
        // Embedded in a longer string — substring match catches it.
        let result = validate_kernel_param_str("noinit=foo init=/bin/sh");
        assert!(matches!(
            result,
            Err(BootControlError::SecurityPolicyViolation { .. })
        ));
    }

    // ── validate_grub_payload ─────────────────────────────────────────────────

    #[test]
    fn grub_payload_allows_clean_pair() {
        assert!(validate_grub_payload("GRUB_TIMEOUT", "5").is_ok());
        assert!(validate_grub_payload("GRUB_CMDLINE_LINUX_DEFAULT", "quiet splash").is_ok());
        assert!(validate_grub_payload("GRUB_DISTRIBUTOR", "Ubuntu").is_ok());
    }

    #[test]
    fn grub_payload_allows_empty_value() {
        assert!(validate_grub_payload("GRUB_CMDLINE_LINUX_DEFAULT", "").is_ok());
    }

    #[test]
    fn grub_payload_rejects_blacklisted_value() {
        let result = validate_grub_payload("GRUB_CMDLINE_LINUX_DEFAULT", "selinux=0 quiet");
        assert!(matches!(
            result,
            Err(BootControlError::SecurityPolicyViolation { .. })
        ));
    }

    #[test]
    fn grub_payload_rejects_blacklisted_key() {
        let result = validate_grub_payload("selinux=0_key", "anything");
        match result {
            Err(BootControlError::SecurityPolicyViolation { reason }) => {
                assert!(
                    reason.contains("key"),
                    "error message should identify which half failed: {reason}"
                );
            }
            other => panic!("expected SecurityPolicyViolation, got {other:?}"),
        }
    }

    #[test]
    fn grub_payload_message_distinguishes_key_vs_value() {
        let key_err = validate_grub_payload("rd.break_key", "safe").unwrap_err();
        let val_err = validate_grub_payload("KEY", "rd.break in value").unwrap_err();
        let key_msg = key_err.to_string();
        let val_msg = val_err.to_string();
        assert!(key_msg.contains("key"), "{key_msg}");
        assert!(val_msg.contains("value"), "{val_msg}");
    }
}
