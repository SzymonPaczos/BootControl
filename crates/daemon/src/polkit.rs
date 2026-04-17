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
//! Real Polkit authorization (`libpolkit-gobject-1`) is only validated in
//! end-to-end tests executed inside a containerized environment with a
//! running systemd (Phase 2+).

use bootcontrol_core::error::BootControlError;

/// Verify that the calling process is authorized to perform privileged boot
/// configuration via the `org.bootcontrol.manage` Polkit action.
///
/// # Arguments
///
/// * `caller_uid` — The Unix UID of the D-Bus caller as reported by the
///   D-Bus daemon. This is obtained from `zbus::Connection::peer_credentials`.
///
/// # Errors
///
/// Returns [`BootControlError::PolkitDenied`] when:
/// - The real Polkit policy denies the action (non-mock build).
/// - Any internal error occurs during the authorization check.
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "polkit-mock")]
/// # {
/// use bootcontrold::polkit::authorize_with_polkit;
/// // In polkit-mock mode the call always succeeds.
/// assert!(authorize_with_polkit(1000).is_ok());
/// # }
/// ```
pub fn authorize_with_polkit(_caller_uid: u32) -> Result<(), BootControlError> {
    #[cfg(feature = "polkit-mock")]
    {
        // Mock implementation: always grants authorization.
        // Used in tests and CI where a real systemd/Polkit stack is unavailable.
        Ok(())
    }

    #[cfg(not(feature = "polkit-mock"))]
    {
        // Real Polkit integration is deferred to Phase 2 of the roadmap.
        // Returning `todo!` here will cause a runtime panic if this code path
        // is reached in a non-mock build, which is the intended behavior to
        // ensure the feature is not accidentally shipped incomplete.
        todo!("real Polkit integration (libpolkit-gobject-1) not yet implemented — Phase 2")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// In polkit-mock mode, authorize_with_polkit must always return Ok regardless of uid.
    #[cfg(feature = "polkit-mock")]
    #[test]
    fn mock_always_grants_authorization() {
        assert!(authorize_with_polkit(0).is_ok());
        assert!(authorize_with_polkit(1000).is_ok());
        assert!(authorize_with_polkit(u32::MAX).is_ok());
    }
}
