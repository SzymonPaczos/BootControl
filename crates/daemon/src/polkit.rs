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
/// (`generate-keys` and `replace-pk` were removed together with Paranoia
/// Mode — scope decision "1.0 GRUB-first", 2026-07-12.)
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
/// * `caller_uid` — The Unix UID of the D-Bus caller as reported by the
///   D-Bus daemon via `org.freedesktop.DBus.GetConnectionUnixUser`. Used to
///   construct the `unix-user` Polkit subject for authorization.
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
///     assert!(authorize_with_polkit(1000, actions::REWRITE_GRUB).await.is_ok());
/// });
/// # }
/// ```
pub async fn authorize_with_polkit(caller_uid: u32, action: &str) -> Result<(), BootControlError> {
    // Defensive contract: action must be one of the per-intent IDs. Shared by
    // mock and real paths so packaging drift fails loudly in CI.
    const KNOWN: &[&str] = &[
        actions::REWRITE_GRUB,
        actions::WRITE_BOOTLOADER,
        actions::ENROLL_MOK,
        actions::RESTORE_SNAPSHOT,
    ];
    if !KNOWN.contains(&action) {
        return Err(BootControlError::PolkitDenied);
    }

    #[cfg(feature = "polkit-mock")]
    {
        // Suppress unused-variable warning in mock mode.
        let _ = caller_uid;
        // Mock implementation: always grants authorization for a known action.
        // Used in tests and CI where a real systemd/Polkit stack is unavailable.
        Ok(())
    }

    #[cfg(not(feature = "polkit-mock"))]
    {
        use std::collections::HashMap;
        use zbus_polkit::policykit1::{AuthorityProxy, CheckAuthorizationFlags, Subject};

        // Build the Polkit subject: a unix-user identified by UID.
        let mut subject_details: HashMap<String, zbus::zvariant::OwnedValue> = HashMap::new();
        subject_details.insert(
            "uid".to_string(),
            zbus::zvariant::Value::from(caller_uid)
                .try_to_owned()
                .map_err(|_| BootControlError::PolkitDenied)?,
        );
        let subject = Subject {
            subject_kind: "unix-user".to_string(),
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
        for uid in [0u32, 1000, u32::MAX] {
            assert!(authorize_with_polkit(uid, actions::REWRITE_GRUB)
                .await
                .is_ok());
            assert!(authorize_with_polkit(uid, actions::WRITE_BOOTLOADER)
                .await
                .is_ok());
            assert!(authorize_with_polkit(uid, actions::ENROLL_MOK)
                .await
                .is_ok());
            assert!(authorize_with_polkit(uid, actions::RESTORE_SNAPSHOT)
                .await
                .is_ok());
        }
    }

    /// Unknown / legacy action IDs are rejected even in mock mode — defends
    /// against packaging drift where a renamed action would otherwise pass
    /// silently in CI and only break on production Polkit.
    #[cfg(feature = "polkit-mock")]
    #[tokio::test]
    async fn mock_rejects_legacy_manage_action() {
        let result = authorize_with_polkit(1000, "org.bootcontrol.manage").await;
        assert!(
            result.is_err(),
            "legacy 'manage' action must be rejected at the call boundary"
        );
    }

    #[cfg(feature = "polkit-mock")]
    #[tokio::test]
    async fn mock_rejects_typo_action() {
        // Underscore instead of hyphen: catches drift between code and policy XML.
        let result = authorize_with_polkit(1000, "org.bootcontrol.rewrite_grub").await;
        assert!(result.is_err());
    }
}
