//! Polkit authorization layer for BootControl.
//!
//! All write operations in BootControl require explicit Polkit authorization
//! before any disk I/O is performed. This module provides the
//! [`authorize_with_polkit`] function that enforces this requirement.
//!
//! # Mock strategy for CI
//!
//! The `polkit-mock` Cargo feature replaces the real Polkit call with an
//! always-`Ok` stub so that integration tests can run on the session bus
//! in GitHub Actions without root or a full systemd stack.
//!
//! Real Polkit authorization is only validated in end-to-end tests executed
//! inside a containerized environment with a running systemd (Phase 2+).

use bootcontrol_core::error::BootControlError;

/// The four per-intent Polkit Action IDs declared in
/// `packaging/polkit/org.bootcontrol.policy`. Single source of truth for
/// callers — every `authorize_with_polkit` call site picks one of these.
///
/// Was six until 2026-07-12, when the scope decision "1.0 GRUB-first"
/// removed Paranoia Mode along with `generate-keys` and `replace-pk`.
/// [`KNOWN_ACTIONS`] pins this set against
/// [`crate::policy_check::REQUIRED_ACTIONS`].
pub mod actions {
    /// Modify `/etc/default/grub`, `/etc/kernel/cmdline`, or rpm-ostree kargs.
    pub const REWRITE_GRUB: &str = "org.bootcontrol.rewrite-grub";
    /// Modify a boot entry / loader configuration / UEFI BootOrder / BootNext.
    pub const WRITE_BOOTLOADER: &str = "org.bootcontrol.write-bootloader";
    /// Enroll a Machine Owner Key (MOK / shim path) or back up NVRAM keys.
    pub const ENROLL_MOK: &str = "org.bootcontrol.enroll-mok";
    /// Restore boot configuration from a previously captured snapshot.
    pub const RESTORE_SNAPSHOT: &str = "org.bootcontrol.restore-snapshot";
}

/// The closed set of action IDs [`authorize_with_polkit`] will accept.
///
/// Defensive contract shared by the mock and real paths so packaging drift
/// fails loudly in CI. Must stay identical (as a set) to
/// [`crate::policy_check::REQUIRED_ACTIONS`] — the startup validation list —
/// which the `known_actions_match_required_policy_actions` test pins.
///
/// Widening this list re-enables authorization for an action the policy file
/// no longer declares: on a system whose Polkit configuration defaults to
/// implicit-yes that is an authorization bypass, not a missing prompt. Add an
/// entry here only together with its `<action id="…">` in
/// `packaging/polkit/org.bootcontrol.policy` and its `REQUIRED_ACTIONS` entry.
pub(crate) const KNOWN_ACTIONS: &[&str] = &[
    actions::REWRITE_GRUB,
    actions::WRITE_BOOTLOADER,
    actions::ENROLL_MOK,
    actions::RESTORE_SNAPSHOT,
];

/// Verify that the calling process is authorized to perform `action` via Polkit.
///
/// `action` must be one of the per-intent Action IDs declared in
/// `packaging/polkit/org.bootcontrol.policy` — see the [`actions`] module for
/// the closed set. The legacy single-action `org.bootcontrol.manage` is
/// **rejected by the policy file**; passing it here in production would either
/// surface as `PolkitDenied` or fall through to Polkit's implicit-yes default
/// depending on the system configuration. Either outcome is wrong.
///
/// In CI builds compiled with the `polkit-mock` feature, the real Polkit call
/// is replaced by an always-`Ok` stub that requires no systemd stack — the
/// `action` argument is still validated against [`actions`] so call sites
/// passing a typo (e.g. `"org.bootcontrol.rewrite_grub"` with underscore) fail
/// loudly in tests.
///
/// # Arguments
///
/// * `caller_bus_name` — The unique D-Bus sender name from the method call
///   header (for example, `":1.42"`). Polkit resolves this unspoofable name
///   to the originating process and login session.
/// * `action` — The per-intent action ID. Must be a constant from [`actions`].
///
/// # Errors
///
/// Returns [`BootControlError::PolkitDenied`] when:
/// - `action` is not one of the per-intent IDs in [`actions`] (defensive check
///   shared by mock and real paths — catches drift between policy file and code).
/// - The Polkit policy denies the action (`is_authorized == false`).
/// - Authentication challenge is presented but fails or is dismissed.
/// - Any internal error occurs while communicating with the Polkit daemon.
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "polkit-mock")]
/// # {
/// use bootcontrold::polkit::{authorize_with_polkit, actions};
/// // In polkit-mock mode a known action always succeeds.
/// let rt = tokio::runtime::Runtime::new().unwrap();
/// rt.block_on(async {
///     assert!(authorize_with_polkit(":1.42", actions::REWRITE_GRUB).await.is_ok());
/// });
/// # }
/// ```
pub async fn authorize_with_polkit(
    caller_bus_name: &str,
    action: &str,
) -> Result<(), BootControlError> {
    // Defensive contract: action must be one of the per-intent IDs. Shared by
    // mock and real paths so packaging drift fails loudly in CI.
    if !KNOWN_ACTIONS.contains(&action) {
        return Err(BootControlError::PolkitDenied);
    }

    #[cfg(feature = "polkit-mock")]
    {
        // Suppress unused-variable warning in mock mode.
        let _ = caller_bus_name;
        // Mock implementation: always grants authorization for a known action.
        // Used in tests and CI where a real systemd/Polkit stack is unavailable.
        Ok(())
    }

    #[cfg(not(feature = "polkit-mock"))]
    {
        use std::collections::HashMap;
        use zbus_polkit::policykit1::{AuthorityProxy, CheckAuthorizationFlags, Subject};

        // A system-bus-name subject lets Polkit securely resolve the exact
        // calling process and its active login session. A bare unix-user
        // subject loses that session association and cannot be challenged by
        // an authentication agent reliably.
        let mut subject_details: HashMap<String, zbus::zvariant::OwnedValue> = HashMap::new();
        subject_details.insert(
            "name".to_string(),
            zbus::zvariant::Value::from(caller_bus_name)
                .try_to_owned()
                .map_err(|_| BootControlError::PolkitDenied)?,
        );
        let subject = Subject {
            subject_kind: "system-bus-name".to_string(),
            subject_details,
        };

        // Connect to the system bus and create the Polkit authority proxy.
        let connection = zbus::Connection::system()
            .await
            .map_err(|_| BootControlError::PolkitDenied)?;

        let authority = AuthorityProxy::new(&connection)
            .await
            .map_err(|_| BootControlError::PolkitDenied)?;

        // Call CheckAuthorization with AllowUserInteraction so the agent
        // can prompt the user if the policy requires authentication.
        let details: HashMap<&str, &str> = HashMap::new();
        let result = authority
            .check_authorization(
                &subject,
                action,
                &details,
                CheckAuthorizationFlags::AllowUserInteraction.into(),
                "",
            )
            .await
            .map_err(|_| BootControlError::PolkitDenied)?;

        if result.is_authorized {
            Ok(())
        } else {
            Err(BootControlError::PolkitDenied)
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "polkit-mock")]
    use super::{actions, authorize_with_polkit};

    /// Mock grants any known per-intent action for any UID — covers root,
    /// typical user, and `u32::MAX`.
    #[cfg(feature = "polkit-mock")]
    #[tokio::test]
    async fn mock_grants_known_action_for_any_uid() {
        for bus_name in [":1.0", ":1.42", ":1.4294967295"] {
            assert!(authorize_with_polkit(bus_name, actions::REWRITE_GRUB)
                .await
                .is_ok());
            assert!(authorize_with_polkit(bus_name, actions::WRITE_BOOTLOADER)
                .await
                .is_ok());
            assert!(authorize_with_polkit(bus_name, actions::ENROLL_MOK)
                .await
                .is_ok());
            assert!(authorize_with_polkit(bus_name, actions::RESTORE_SNAPSHOT)
                .await
                .is_ok());
        }
    }

    /// Removed high-risk flows retain stable identifiers but must not become
    /// authorizable until their implementation and policy are restored.
    #[cfg(feature = "polkit-mock")]
    #[tokio::test]
    async fn mock_rejects_reserved_paranoia_actions() {
        for action in [
            "org.bootcontrol.generate-keys",
            "org.bootcontrol.replace-pk",
        ] {
            assert!(authorize_with_polkit(":1.42", action).await.is_err());
        }
    }

    /// Unknown / legacy action IDs are rejected even in mock mode — defends
    /// against packaging drift where a renamed action would otherwise pass
    /// silently in CI and only break on production Polkit.
    #[cfg(feature = "polkit-mock")]
    #[tokio::test]
    async fn mock_rejects_legacy_manage_action() {
        let result = authorize_with_polkit(":1.42", "org.bootcontrol.manage").await;
        assert!(
            result.is_err(),
            "legacy 'manage' action must be rejected at the call boundary"
        );
    }

    #[cfg(feature = "polkit-mock")]
    #[tokio::test]
    async fn mock_rejects_typo_action() {
        // Underscore instead of hyphen: catches drift between code and policy XML.
        let result = authorize_with_polkit(":1.42", "org.bootcontrol.rewrite_grub").await;
        assert!(result.is_err());
    }

    /// The authorization gate and the startup policy validation must describe
    /// the same closed set of actions. Two independent lists drifted apart
    /// once already (audit 2026-08-23: `4fcf14c` dropped two constants and
    /// left dangling references); this pins them to one another so a future
    /// edit to either side fails here instead of at runtime, where the
    /// failure mode is either a daemon that refuses to start or an action
    /// authorized without a policy declaration.
    ///
    /// Runs without the `polkit-mock` feature — the invariant is about the
    /// lists themselves, not about the authorization backend.
    #[test]
    fn known_actions_match_required_policy_actions() {
        let mut known: Vec<&str> = super::KNOWN_ACTIONS.to_vec();
        let mut required: Vec<&str> = crate::policy_check::REQUIRED_ACTIONS.to_vec();
        known.sort_unstable();
        required.sort_unstable();
        assert_eq!(
            known, required,
            "polkit::KNOWN_ACTIONS and policy_check::REQUIRED_ACTIONS diverged — \
             every accepted action must also be declared in \
             packaging/polkit/org.bootcontrol.policy"
        );
    }
}
