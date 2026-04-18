//! UKI (Unified Kernel Image) kernel command-line manager.
//!
//! UKI kernel parameters are managed via `/etc/kernel/cmdline` — a single
//! line of space-separated parameters. This module provides pure functions
//! to parse, manipulate, and re-serialize that file.
//!
//! # Security
//!
//! [`validate_kernel_param`] mirrors the blacklist in
//! `crates/daemon/src/sanitize.rs`. Both lists **must be kept in sync**.
//! Any new blacklisted pattern must be added to both locations.

#![deny(warnings)]
#![deny(missing_docs)]

use crate::error::BootControlError;

/// Blacklisted kernel parameter substrings.
///
/// Mirrors `BLACKLISTED_PATTERNS` in `crates/daemon/src/sanitize.rs`.
/// Both lists must be kept in sync — any addition here must also be added
/// there and vice versa.
const BLACKLISTED_PARAMS: &[&str] = &[
    "init=",
    "selinux=0",
    "apparmor=0",
    "systemd.unit=",
    "rd.break",
    "single",
    "emergency",
];

/// Validate a single kernel parameter against the security blacklist.
///
/// This function mirrors `crates/daemon::sanitize::check_payload` for the
/// kernel cmdline context. If a parameter contains any blacklisted substring,
/// the operation is rejected.
///
/// # Arguments
///
/// * `param` — A single kernel parameter (e.g. `"quiet"`, `"root=/dev/sda1"`).
///
/// # Errors
///
/// Returns [`BootControlError::SecurityPolicyViolation`] if `param` contains
/// any substring from the security blacklist.
///
/// # Examples
///
/// ```
/// use bootcontrol_core::backends::uki::validate_kernel_param;
///
/// assert!(validate_kernel_param("quiet").is_ok());
/// assert!(validate_kernel_param("root=/dev/sda1").is_ok());
/// assert!(validate_kernel_param("selinux=0").is_err());
/// assert!(validate_kernel_param("init=/bin/bash").is_err());
/// ```
pub fn validate_kernel_param(param: &str) -> Result<(), BootControlError> {
    for &pattern in BLACKLISTED_PARAMS {
        if param.contains(pattern) {
            return Err(BootControlError::SecurityPolicyViolation {
                reason: format!(
                    "kernel parameter '{param}' contains blacklisted pattern '{pattern}'"
                ),
            });
        }
    }
    Ok(())
}

/// Parse `/etc/kernel/cmdline` content into a list of individual parameters.
///
/// Splits on whitespace. Empty tokens (arising from multiple consecutive
/// spaces) are filtered out.
///
/// # Arguments
///
/// * `content` — Raw text content of `/etc/kernel/cmdline`.
///
/// # Examples
///
/// ```
/// use bootcontrol_core::backends::uki::parse_cmdline;
///
/// let params = parse_cmdline("root=/dev/sda1 rw quiet splash\n");
/// assert_eq!(params, vec!["root=/dev/sda1", "rw", "quiet", "splash"]);
/// ```
pub fn parse_cmdline(content: &str) -> Vec<String> {
    content.split_whitespace().map(|s| s.to_string()).collect()
}

/// Serialize a list of kernel parameters back to cmdline file content.
///
/// Parameters are joined with a single space and a trailing newline is appended
/// to follow POSIX text file convention.
///
/// # Arguments
///
/// * `params` — Slice of individual kernel parameters.
///
/// # Examples
///
/// ```
/// use bootcontrol_core::backends::uki::serialize_cmdline;
///
/// let result = serialize_cmdline(&["root=/dev/sda1".to_string(), "quiet".to_string()]);
/// assert_eq!(result, "root=/dev/sda1 quiet\n");
/// ```
pub fn serialize_cmdline(params: &[String]) -> String {
    let mut out = params.join(" ");
    out.push('\n');
    out
}

/// Add a kernel parameter to the cmdline, if not already present.
///
/// # Arguments
///
/// * `content` — Raw text content of `/etc/kernel/cmdline`.
/// * `param`   — The parameter to add (e.g. `"quiet"`).
///
/// # Errors
///
/// Returns [`BootControlError::SecurityPolicyViolation`] if `param` matches
/// the security blacklist (mirrors `crates/daemon::sanitize::check_payload`).
///
/// # Examples
///
/// ```
/// use bootcontrol_core::backends::uki::add_param;
///
/// let result = add_param("root=/dev/sda1 rw\n", "quiet").unwrap();
/// assert!(result.contains("quiet"));
///
/// // Idempotent — adding an existing parameter is a no-op.
/// let result2 = add_param(&result, "quiet").unwrap();
/// let count = result2.split_whitespace().filter(|&p| p == "quiet").count();
/// assert_eq!(count, 1);
/// ```
pub fn add_param(content: &str, param: &str) -> Result<String, BootControlError> {
    validate_kernel_param(param)?;
    let mut params = parse_cmdline(content);
    if !params.iter().any(|p| p == param) {
        params.push(param.to_string());
    }
    Ok(serialize_cmdline(&params))
}

/// Remove a kernel parameter from the cmdline.
///
/// Removes all occurrences of `param`. If the parameter is not present,
/// the content is returned unchanged.
///
/// # Arguments
///
/// * `content` — Raw text content of `/etc/kernel/cmdline`.
/// * `param`   — The parameter to remove (e.g. `"quiet"`).
///
/// # Errors
///
/// Returns [`BootControlError::KeyNotFound`] if `param` is not present in
/// `content`.
///
/// # Examples
///
/// ```
/// use bootcontrol_core::backends::uki::remove_param;
///
/// let result = remove_param("root=/dev/sda1 rw quiet\n", "quiet").unwrap();
/// assert!(!result.contains("quiet"));
/// assert!(result.contains("root=/dev/sda1"));
/// ```
pub fn remove_param(content: &str, param: &str) -> Result<String, BootControlError> {
    let params = parse_cmdline(content);
    if !params.iter().any(|p| p == param) {
        return Err(BootControlError::KeyNotFound {
            key: param.to_string(),
        });
    }
    let filtered: Vec<String> = params.into_iter().filter(|p| p != param).collect();
    Ok(serialize_cmdline(&filtered))
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── validate_kernel_param ──────────────────────────────────────────────────

    #[test]
    fn validate_allows_safe_params() {
        assert!(validate_kernel_param("quiet").is_ok());
        assert!(validate_kernel_param("root=/dev/sda1").is_ok());
        assert!(validate_kernel_param("rw").is_ok());
        assert!(validate_kernel_param("loglevel=3").is_ok());
    }

    #[test]
    fn validate_blocks_selinux_zero() {
        assert!(matches!(
            validate_kernel_param("selinux=0"),
            Err(BootControlError::SecurityPolicyViolation { .. })
        ));
    }

    #[test]
    fn validate_blocks_init_equal() {
        assert!(matches!(
            validate_kernel_param("init=/bin/bash"),
            Err(BootControlError::SecurityPolicyViolation { .. })
        ));
    }

    #[test]
    fn validate_blocks_all_blacklisted_patterns() {
        for &pattern in BLACKLISTED_PARAMS {
            let result = validate_kernel_param(pattern);
            assert!(
                result.is_err(),
                "Expected blacklisted pattern '{pattern}' to be rejected"
            );
        }
    }

    // ── parse_cmdline ──────────────────────────────────────────────────────────

    #[test]
    fn parse_splits_on_whitespace() {
        let params = parse_cmdline("root=/dev/sda1 rw quiet splash\n");
        assert_eq!(params, vec!["root=/dev/sda1", "rw", "quiet", "splash"]);
    }

    #[test]
    fn parse_ignores_extra_whitespace() {
        let params = parse_cmdline("  quiet   splash  \n");
        assert_eq!(params, vec!["quiet", "splash"]);
    }

    #[test]
    fn parse_empty_content_returns_empty_vec() {
        let params = parse_cmdline("");
        assert!(params.is_empty());
    }

    // ── serialize_cmdline ──────────────────────────────────────────────────────

    #[test]
    fn serialize_joins_with_space_and_newline() {
        let result = serialize_cmdline(&["root=/dev/sda1".to_string(), "quiet".to_string()]);
        assert_eq!(result, "root=/dev/sda1 quiet\n");
    }

    #[test]
    fn serialize_empty_returns_just_newline() {
        let result = serialize_cmdline(&[]);
        assert_eq!(result, "\n");
    }

    // ── add_param ──────────────────────────────────────────────────────────────

    #[test]
    fn add_param_appends_new_param() {
        let result = add_param("root=/dev/sda1 rw\n", "quiet").unwrap();
        assert!(result.contains("quiet"));
    }

    #[test]
    fn add_param_is_idempotent() {
        let content = "root=/dev/sda1 quiet\n";
        let result = add_param(content, "quiet").unwrap();
        let count = result.split_whitespace().filter(|&p| p == "quiet").count();
        assert_eq!(count, 1);
    }

    #[test]
    fn add_param_rejects_blacklisted() {
        let result = add_param("quiet\n", "selinux=0");
        assert!(matches!(
            result,
            Err(BootControlError::SecurityPolicyViolation { .. })
        ));
    }

    // ── remove_param ───────────────────────────────────────────────────────────

    #[test]
    fn remove_param_removes_existing() {
        let result = remove_param("root=/dev/sda1 rw quiet\n", "quiet").unwrap();
        assert!(!result.contains("quiet"));
        assert!(result.contains("root=/dev/sda1"));
    }

    #[test]
    fn remove_param_returns_key_not_found_when_absent() {
        let result = remove_param("root=/dev/sda1 rw\n", "quiet");
        assert!(matches!(result, Err(BootControlError::KeyNotFound { .. })));
    }

    #[test]
    fn remove_param_removes_all_occurrences() {
        let result = remove_param("quiet root=/dev/sda1 quiet\n", "quiet").unwrap();
        assert!(!result.contains("quiet"));
    }
}
