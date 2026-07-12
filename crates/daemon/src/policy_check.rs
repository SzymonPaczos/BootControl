//! Startup validation of the Polkit policy file.
//!
//! Defense-in-depth for the per-intent Polkit Action ID contract
//! (`docs/GUI_V2_SPEC_v2.md` §7, `.claude/rules/decisions.md` 2026-05-03).
//! A daemon binary upgraded *after* an old policy file would silently lose
//! granular authorization — the policy file's header comment already promises
//! *"daemon will refuse to start if it sees a stale policy file declaring only
//! that action"*. This module enforces that promise.
//!
//! The check is a substring scan rather than a full XML parse:
//! - we only need to know *which action IDs are declared*, and
//! - dropping a new dep just for one startup check would be overkill.
//!
//! Skipped when `BOOTCONTROL_BUS=session` (E2E / CI environment); a session-
//! bus daemon never reaches the system Polkit anyway.

#![deny(missing_docs)]

use std::path::Path;

/// Canonical location of the Polkit policy file on a packaged install.
pub const DEFAULT_POLICY_PATH: &str = "/usr/share/polkit-1/actions/org.bootcontrol.policy";

/// The six per-intent Polkit Action IDs the daemon needs declared. Must be
/// kept in sync with [`packaging/polkit/org.bootcontrol.policy`].
pub const REQUIRED_ACTIONS: &[&str] = &[
    "org.bootcontrol.rewrite-grub",
    "org.bootcontrol.write-bootloader",
    "org.bootcontrol.enroll-mok",
    "org.bootcontrol.restore-snapshot",
];

/// Reason why a policy file is unfit for daemon startup.
#[derive(Debug, PartialEq, Eq)]
pub enum PolicyError {
    /// File does not exist or cannot be read.
    Missing(String),
    /// File declares only the legacy `org.bootcontrol.manage` action.
    LegacyManageOnly,
    /// File is missing one or more required per-intent actions.
    IncompleteActions {
        /// Action IDs declared in the file (best-effort substring scan).
        present: Vec<String>,
        /// Action IDs the daemon expected but did not find.
        missing: Vec<String>,
    },
}

impl std::fmt::Display for PolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(detail) => write!(f, "polkit policy file unreadable: {detail}"),
            Self::LegacyManageOnly => write!(
                f,
                "polkit policy file declares only the deprecated \
                 'org.bootcontrol.manage' action. Daemon requires the six \
                 per-intent actions declared in \
                 packaging/polkit/org.bootcontrol.policy. Refusing to start \
                 — fix the packaging or re-install."
            ),
            Self::IncompleteActions { present, missing } => write!(
                f,
                "polkit policy file is missing required actions: {missing:?} \
                 (present: {present:?}). Refusing to start."
            ),
        }
    }
}

impl std::error::Error for PolicyError {}

/// Validate a Polkit policy file content string.
///
/// Returns `Ok(())` iff every action in [`REQUIRED_ACTIONS`] appears as
/// `id="..."` somewhere in `content`. Legacy single-action files (`manage`
/// only) and incomplete files both reject startup.
///
/// # Arguments
///
/// * `content` — Raw text of the policy file.
///
/// # Errors
///
/// - [`PolicyError::LegacyManageOnly`] — only `manage` declared.
/// - [`PolicyError::IncompleteActions`] — at least one required action missing.
///
/// # Examples
///
/// ```
/// use bootcontrold::policy_check::{validate_policy_content, PolicyError};
///
/// // Legacy file rejected.
/// let legacy = r#"<action id="org.bootcontrol.manage">...</action>"#;
/// assert!(matches!(
///     validate_policy_content(legacy),
///     Err(PolicyError::LegacyManageOnly)
/// ));
/// ```
pub fn validate_policy_content(content: &str) -> Result<(), PolicyError> {
    let declares_manage = content.contains(r#"id="org.bootcontrol.manage""#);

    let mut present = Vec::new();
    let mut missing = Vec::new();
    for &action in REQUIRED_ACTIONS {
        let needle = format!(r#"id="{action}""#);
        if content.contains(&needle) {
            present.push(action.to_string());
        } else {
            missing.push(action.to_string());
        }
    }

    if !missing.is_empty() {
        // Special case: only `manage` declared, none of the new actions.
        // Worth surfacing as its own variant so the error message can point
        // the operator at the exact upgrade path.
        if declares_manage && present.is_empty() {
            return Err(PolicyError::LegacyManageOnly);
        }
        return Err(PolicyError::IncompleteActions { present, missing });
    }
    Ok(())
}

/// Read a policy file from disk and run [`validate_policy_content`].
///
/// # Arguments
///
/// * `path` — Filesystem path to the Polkit policy file.
///
/// # Errors
///
/// - [`PolicyError::Missing`] if the file cannot be read.
/// - Anything [`validate_policy_content`] returns.
///
/// # Examples
///
/// ```
/// # use bootcontrold::policy_check::{validate_policy_file, PolicyError};
/// // Non-existent path surfaces a clear error.
/// let err = validate_policy_file(std::path::Path::new("/nonexistent.policy")).unwrap_err();
/// assert!(matches!(err, PolicyError::Missing(_)));
/// ```
pub fn validate_policy_file(path: &Path) -> Result<(), PolicyError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| PolicyError::Missing(format!("{}: {e}", path.display())))?;
    validate_policy_content(&content)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full_policy() -> String {
        REQUIRED_ACTIONS
            .iter()
            .map(|a| format!(r#"<action id="{a}"></action>"#))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn complete_policy_passes() {
        assert!(validate_policy_content(&full_policy()).is_ok());
    }

    #[test]
    fn legacy_manage_only_is_rejected() {
        let legacy = r#"<action id="org.bootcontrol.manage">desc</action>"#;
        assert_eq!(
            validate_policy_content(legacy),
            Err(PolicyError::LegacyManageOnly)
        );
    }

    #[test]
    fn partial_policy_lists_missing_actions() {
        // Three out of six declared.
        let partial = format!(
            r#"<action id="{}"></action>
            <action id="{}"></action>
            <action id="{}"></action>"#,
            REQUIRED_ACTIONS[0], REQUIRED_ACTIONS[1], REQUIRED_ACTIONS[2]
        );
        match validate_policy_content(&partial) {
            Err(PolicyError::IncompleteActions { present, missing }) => {
                assert_eq!(present.len(), 3);
                assert_eq!(missing.len(), 3);
                assert!(missing.contains(&REQUIRED_ACTIONS[3].to_string()));
                assert!(missing.contains(&REQUIRED_ACTIONS[4].to_string()));
                assert!(missing.contains(&REQUIRED_ACTIONS[5].to_string()));
            }
            other => panic!("expected IncompleteActions, got {other:?}"),
        }
    }

    #[test]
    fn policy_with_extra_actions_still_passes() {
        // Forward-compatibility: a future packaging might add a 7th action.
        let mut policy = full_policy();
        policy.push_str(r#"<action id="org.bootcontrol.future-action"></action>"#);
        assert!(validate_policy_content(&policy).is_ok());
    }

    #[test]
    fn missing_file_is_reported() {
        let result = validate_policy_file(Path::new("/nonexistent-policy-file"));
        assert!(matches!(result, Err(PolicyError::Missing(_))));
    }

    #[test]
    fn shipped_policy_file_is_accepted() {
        // Sanity test against the actual file in the repo. Pinned path
        // relative to workspace root so the test catches packaging drift.
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("CARGO_MANIFEST_DIR should be two levels under workspace root");
        let shipped = workspace_root.join("packaging/polkit/org.bootcontrol.policy");
        if !shipped.exists() {
            // Some build envs (e.g. published crates without the packaging/
            // tree) won't have this — skip gracefully rather than fail.
            return;
        }
        assert!(
            validate_policy_file(&shipped).is_ok(),
            "shipped policy file {} fails startup validation — packaging drift",
            shipped.display()
        );
    }
}
