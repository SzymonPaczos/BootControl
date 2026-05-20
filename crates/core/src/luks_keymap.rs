//! Pre-flight validation of console-keymap settings before an initramfs
//! rebuild on LUKS-encrypted systems.
//!
//! When the root filesystem is LUKS-encrypted, the initramfs prompts the user
//! for their passphrase at boot. The prompt uses whatever keymap the
//! initramfs was generated with — by convention the value of `KEYMAP=` in
//! `/etc/vconsole.conf`. If that file is missing, contains an empty value,
//! or names a keymap that does not resolve to a real `.map` file on disk,
//! the user can be silently locked out after a kernel update: the
//! passphrase they type matches their *current* keymap but not the one
//! frozen into the new initramfs.
//!
//! This module is the **pure** layer: it takes the already-read contents of
//! `/etc/vconsole.conf` plus a closure that checks whether a candidate
//! keymap name resolves on this system, and returns a verdict. The
//! filesystem-touching wrapper lives in `crates/daemon/src/luks_keymap.rs`.

#![deny(warnings)]
#![deny(missing_docs)]

use crate::error::BootControlError;

/// Possible verdicts from [`validate_vconsole_keymap`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeymapVerdict {
    /// `KEYMAP=…` was found in `/etc/vconsole.conf`, has a syntactically
    /// valid name, and resolves to a real keymap on this system.
    Ok {
        /// The keymap name as extracted from `KEYMAP=…`. Useful for audit
        /// logging.
        keymap: String,
    },
    /// `/etc/vconsole.conf` did not contain a `KEYMAP=` assignment, or the
    /// value was empty. On a LUKS-encrypted host this is a real problem;
    /// on an unencrypted host the failsafe layer should treat it as a
    /// warning rather than a hard fail.
    Missing,
    /// `KEYMAP=` was set but the value contains characters that are not
    /// legal in a keymap name (anything outside `[A-Za-z0-9._-]`). Most
    /// often this is a quoting / shell-substitution mistake in the
    /// vconsole.conf file.
    Malformed {
        /// The raw value as read from the file.
        value: String,
    },
    /// `KEYMAP=` had a syntactically valid value, but no matching `.map[.gz]`
    /// file was found under any of the standard `kbd` data dirs. This is
    /// the dangerous case: the initramfs will be built with the kernel
    /// default (`us`) and the user can be locked out.
    UnresolvedName {
        /// The keymap name that did not resolve.
        keymap: String,
    },
}

impl KeymapVerdict {
    /// Project the verdict into the workspace-canonical [`BootControlError`].
    ///
    /// `Ok` becomes `Ok(())`; every other variant becomes
    /// [`BootControlError::SecurityPolicyViolation`] with a human-readable
    /// reason. The variant name flows through unchanged via the structured
    /// `Display` impl, so frontends can pattern-match on the **reason
    /// prefix** if they need to render a localised message.
    pub fn into_result(self) -> Result<String, BootControlError> {
        match self {
            KeymapVerdict::Ok { keymap } => Ok(keymap),
            KeymapVerdict::Missing => Err(BootControlError::SecurityPolicyViolation {
                reason:
                    "/etc/vconsole.conf has no KEYMAP= setting; the LUKS prompt would fall back \
                     to the kernel default and may not match the user's actual layout"
                        .to_string(),
            }),
            KeymapVerdict::Malformed { value } => Err(BootControlError::SecurityPolicyViolation {
                reason: format!(
                    "KEYMAP={value} in /etc/vconsole.conf contains characters that are not legal \
                     in a keymap name; fix the value before rebuilding the initramfs"
                ),
            }),
            KeymapVerdict::UnresolvedName { keymap } => {
                Err(BootControlError::SecurityPolicyViolation {
                    reason: format!(
                        "KEYMAP={keymap} in /etc/vconsole.conf does not resolve to any .map file \
                         under the kbd data dirs; the LUKS prompt would fall back to `us`"
                    ),
                })
            }
        }
    }
}

/// Validate the `KEYMAP=` setting in the supplied `vconsole.conf` content.
///
/// # Arguments
///
/// * `vconsole_conf` — raw contents of `/etc/vconsole.conf` (or an empty
///   string if the file does not exist; the caller decides how strict to
///   be in that case).
/// * `resolve_keymap` — a closure that returns `true` iff the candidate
///   keymap name resolves to a real `.map[.gz]` file on this system.
///   Injected so this function stays pure and unit-testable.
///
/// # Examples
///
/// ```
/// use bootcontrol_core::luks_keymap::{validate_vconsole_keymap, KeymapVerdict};
///
/// // Real keymap that resolves.
/// let verdict = validate_vconsole_keymap("KEYMAP=pl\n", |name| name == "pl");
/// assert_eq!(verdict, KeymapVerdict::Ok { keymap: "pl".into() });
///
/// // KEYMAP unset.
/// let verdict = validate_vconsole_keymap("FONT=lat2-16\n", |_| true);
/// assert_eq!(verdict, KeymapVerdict::Missing);
///
/// // KEYMAP set but not resolvable.
/// let verdict = validate_vconsole_keymap("KEYMAP=bogus-layout\n", |_| false);
/// assert_eq!(verdict, KeymapVerdict::UnresolvedName { keymap: "bogus-layout".into() });
///
/// // KEYMAP value has shell metacharacters.
/// let verdict = validate_vconsole_keymap("KEYMAP=$(cat /etc/passwd)\n", |_| true);
/// matches!(verdict, KeymapVerdict::Malformed { .. });
/// ```
pub fn validate_vconsole_keymap<F>(vconsole_conf: &str, resolve_keymap: F) -> KeymapVerdict
where
    F: FnOnce(&str) -> bool,
{
    let raw = match extract_keymap(vconsole_conf) {
        Some(v) => v,
        None => return KeymapVerdict::Missing,
    };

    if raw.is_empty() {
        return KeymapVerdict::Missing;
    }

    if !is_valid_keymap_name(&raw) {
        return KeymapVerdict::Malformed { value: raw };
    }

    if resolve_keymap(&raw) {
        KeymapVerdict::Ok { keymap: raw }
    } else {
        KeymapVerdict::UnresolvedName { keymap: raw }
    }
}

/// Return the value of the first `KEYMAP=` assignment in `content`, with
/// surrounding double or single quotes stripped. Lines starting with `#` are
/// comments and skipped. Returns `None` if no `KEYMAP=` line is present.
fn extract_keymap(content: &str) -> Option<String> {
    for line in content.lines() {
        let line = line.trim_start();
        if line.starts_with('#') {
            continue;
        }
        if let Some(value) = line.strip_prefix("KEYMAP=") {
            let trimmed = value.trim().trim_matches('"').trim_matches('\'');
            return Some(trimmed.to_string());
        }
    }
    None
}

/// Keymap names follow the kbd convention: lowercase letters, digits, dot,
/// underscore and dash. Anything else (whitespace, `$`, `(`, quotes inside
/// the value) is rejected as malformed to defend against accidental shell
/// substitution slipping past the parser.
fn is_valid_keymap_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_keymap_value() {
        assert_eq!(extract_keymap("KEYMAP=pl\n").as_deref(), Some("pl"));
    }

    #[test]
    fn extracts_keymap_strips_quotes() {
        assert_eq!(extract_keymap("KEYMAP=\"pl\"\n").as_deref(), Some("pl"));
        assert_eq!(extract_keymap("KEYMAP='pl'\n").as_deref(), Some("pl"));
    }

    #[test]
    fn extracts_skips_comments() {
        let body = "# this is a comment\n# KEYMAP=fake\nKEYMAP=de\n";
        assert_eq!(extract_keymap(body).as_deref(), Some("de"));
    }

    #[test]
    fn extracts_returns_none_when_absent() {
        assert_eq!(extract_keymap("FONT=lat2-16\nLOCALE=pl_PL\n"), None);
    }

    #[test]
    fn validates_legal_name() {
        assert!(is_valid_keymap_name("pl"));
        assert!(is_valid_keymap_name("us"));
        assert!(is_valid_keymap_name("de-latin1-nodeadkeys"));
        assert!(is_valid_keymap_name("ru_win"));
        assert!(is_valid_keymap_name("dvorak.swap"));
    }

    #[test]
    fn rejects_shell_metacharacters() {
        assert!(!is_valid_keymap_name("$(cat /etc/passwd)"));
        assert!(!is_valid_keymap_name("pl;rm"));
        assert!(!is_valid_keymap_name("us with spaces"));
        assert!(!is_valid_keymap_name(""));
    }

    #[test]
    fn verdict_ok_when_resolves() {
        let v = validate_vconsole_keymap("KEYMAP=pl\n", |name| name == "pl");
        assert_eq!(
            v,
            KeymapVerdict::Ok {
                keymap: "pl".into()
            }
        );
    }

    #[test]
    fn verdict_missing_when_no_assignment() {
        let v = validate_vconsole_keymap("FONT=lat2\n", |_| true);
        assert_eq!(v, KeymapVerdict::Missing);
    }

    #[test]
    fn verdict_missing_when_value_empty() {
        let v = validate_vconsole_keymap("KEYMAP=\n", |_| true);
        assert_eq!(v, KeymapVerdict::Missing);
    }

    #[test]
    fn verdict_malformed_on_shell_chars() {
        let v = validate_vconsole_keymap("KEYMAP=$(id)\n", |_| true);
        match v {
            KeymapVerdict::Malformed { value } => assert!(value.contains("$(")),
            other => panic!("expected Malformed, got {other:?}"),
        }
    }

    #[test]
    fn verdict_unresolved_when_resolver_says_no() {
        let v = validate_vconsole_keymap("KEYMAP=bogus\n", |_| false);
        assert_eq!(
            v,
            KeymapVerdict::UnresolvedName {
                keymap: "bogus".into()
            }
        );
    }

    #[test]
    fn into_result_ok_returns_keymap_name() {
        let result = KeymapVerdict::Ok {
            keymap: "pl".into(),
        }
        .into_result();
        assert_eq!(result.unwrap(), "pl");
    }

    #[test]
    fn into_result_missing_returns_policy_violation() {
        let result = KeymapVerdict::Missing.into_result();
        assert!(matches!(
            result,
            Err(BootControlError::SecurityPolicyViolation { .. })
        ));
    }
}
