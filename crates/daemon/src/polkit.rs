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

/// Verify that the calling process is authorized to perform privileged boot
/// configuration via the `org.bootcontrol.manage` Polkit action.
///
/// In production builds (without `polkit-mock` feature), this function performs
/// a real D-Bus call to the Polkit authority daemon
/// (`org.freedesktop.PolicyKit1`) to check authorization for the frozen
/// action ID `org.bootcontrol.manage`.
///
/// In CI builds compiled with the `polkit-mock` feature, the call is replaced
/// by an always-`Ok` stub that requires no systemd stack.
///
/// # Arguments
///
/// * `caller_uid` — The Unix UID of the D-Bus caller as reported by the
///   D-Bus daemon via `org.freedesktop.DBus.GetConnectionUnixUser`. This is
///   used to construct the `unix-user` Polkit subject for authorization.
///
/// # Errors
///
/// Returns [`BootControlError::PolkitDenied`] when:
/// - The Polkit policy denies the action (`is_authorized == false`).
/// - Authentication challenge is presented but fails or is dismissed.
/// - Any internal error occurs while communicating with the Polkit daemon.
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "polkit-mock")]
/// # {
/// use bootcontrold::polkit::authorize_with_polkit;
/// // In polkit-mock mode the call always succeeds.
/// let rt = tokio::runtime::Runtime::new().unwrap();
/// rt.block_on(async {
///     assert!(authorize_with_polkit(1000).await.is_ok());
/// });
/// # }
/// ```
pub async fn authorize_with_polkit(caller_uid: u32) -> Result<(), BootControlError> {
    #[cfg(feature = "polkit-mock")]
    {
        // Suppress unused-variable warning in mock mode.
        let _ = caller_uid;
        // Mock implementation: always grants authorization.
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
                "org.bootcontrol.manage",
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

    /// In polkit-mock mode, authorize_with_polkit must always return Ok
    /// regardless of uid — including the boundary values 0, 1000, and u32::MAX.
    #[cfg(feature = "polkit-mock")]
    #[tokio::test]
    async fn mock_always_grants_authorization() {
        assert!(authorize_with_polkit(0).await.is_ok());
        assert!(authorize_with_polkit(1000).await.is_ok());
        assert!(authorize_with_polkit(u32::MAX).await.is_ok());
    }

    /// The mock must grant authorization for the root UID (0), even though
    /// real Polkit would not require a password for root — this keeps the mock
    /// consistent with the real path's success contract.
    #[cfg(feature = "polkit-mock")]
    #[tokio::test]
    async fn mock_grants_for_root_uid() {
        assert!(authorize_with_polkit(0).await.is_ok());
    }

    /// The mock must grant authorization for a typical unprivileged UID,
    /// simulating a successful interactive Polkit prompt.
    #[cfg(feature = "polkit-mock")]
    #[tokio::test]
    async fn mock_grants_for_unprivileged_uid() {
        assert!(authorize_with_polkit(1000).await.is_ok());
    }
}
