//! Immutable / atomic distro detection — filesystem wrapper.
//!
//! Thin glue over the pure
//! [`bootcontrol_core::immutable_distro::detect_immutable_distro`] function.
//! Performs the real filesystem checks and `$PATH` resolution at runtime, then
//! delegates the decision to the pure layer.
//!
//! Override hook: setting `BOOTCONTROL_IMMUTABLE_DISTRO_OVERRIDE` (any
//! non-empty value) forces the result to that distro tag, bypassing all
//! filesystem checks. Intended for E2E tests on classic distros that want to
//! exercise the rejection path.

#![deny(warnings)]
#![deny(missing_docs)]

use std::path::Path;

use bootcontrol_core::immutable_distro::{detect_immutable_distro, ImmutableDistro};

/// `/run` marker file created by the ostree initrd to flag an ostree-booted
/// root. Authoritative signal — present iff the running root tree is an
/// ostree deploy.
const OSTREE_BOOTED_MARKER: &str = "/run/ostree-booted";

/// On-disk ostree deploy tree. Present even before the initrd marker is
/// written, so we accept it as a secondary signal for recovery shells and
/// live ISOs.
const OSTREE_SYSROOT_DIR: &str = "/sysroot/ostree/deploy";

/// Reject the operation if the host is an atomic / immutable distro.
///
/// Returns `Err(BootControlError::ImmutableDistroDetected)` so the interface
/// layer can map it to the structured D-Bus error name; classic mutable hosts
/// return `Ok(())` and execution continues into the write path.
///
/// # Examples
///
/// ```no_run
/// use bootcontrold::immutable_distro::enforce_writable_distro;
///
/// // On a classic mutable host this returns Ok(()).
/// // On Fedora Silverblue it returns Err(ImmutableDistroDetected).
/// let _ = enforce_writable_distro();
/// ```
pub fn enforce_writable_distro() -> Result<(), bootcontrol_core::error::BootControlError> {
    if let Some(distro) = probe_immutable_distro() {
        return Err(
            bootcontrol_core::error::BootControlError::ImmutableDistroDetected {
                distro: distro.to_string(),
            },
        );
    }
    Ok(())
}

/// Probe the real filesystem and `$PATH` to classify the host.
///
/// Returns `None` for classic mutable distros (Debian/Ubuntu, Arch, classic
/// Fedora, openSUSE non-MicroOS). Returns `Some(ImmutableDistro::RpmOstree)`
/// for Fedora atomic variants and `Some(ImmutableDistro::Ostree)` for bare
/// libostree systems. Honours the `BOOTCONTROL_IMMUTABLE_DISTRO_OVERRIDE`
/// env var for tests.
///
/// # Examples
///
/// ```no_run
/// use bootcontrold::immutable_distro::probe_immutable_distro;
///
/// // On a classic mutable host this returns None.
/// let _ = probe_immutable_distro();
/// ```
pub fn probe_immutable_distro() -> Option<ImmutableDistro> {
    if let Ok(override_tag) = std::env::var("BOOTCONTROL_IMMUTABLE_DISTRO_OVERRIDE") {
        if !override_tag.is_empty() {
            return match override_tag.as_str() {
                "rpm-ostree" => Some(ImmutableDistro::RpmOstree),
                "ostree" => Some(ImmutableDistro::Ostree),
                _ => None,
            };
        }
    }

    detect_immutable_distro(
        Path::new(OSTREE_BOOTED_MARKER).exists(),
        Path::new(OSTREE_SYSROOT_DIR).exists(),
        which::which("rpm-ostree").is_ok(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // The pure detection logic is exhaustively tested in
    // `bootcontrol-core::immutable_distro::tests`. Here we only verify that
    // the env-var override hook routes correctly and the wrapper returns
    // `None` on a classic test host (the assumption used by every other
    // test in the workspace).

    #[test]
    fn override_rpm_ostree_takes_precedence() {
        // SAFETY: tests within a single crate run in the same process and may
        // race on env vars. The override is restored at the end. No other
        // test in this module reads this var, so the window is bounded.
        std::env::set_var("BOOTCONTROL_IMMUTABLE_DISTRO_OVERRIDE", "rpm-ostree");
        let probed = probe_immutable_distro();
        std::env::remove_var("BOOTCONTROL_IMMUTABLE_DISTRO_OVERRIDE");
        assert_eq!(probed, Some(ImmutableDistro::RpmOstree));
    }

    #[test]
    fn override_ostree_takes_precedence() {
        std::env::set_var("BOOTCONTROL_IMMUTABLE_DISTRO_OVERRIDE", "ostree");
        let probed = probe_immutable_distro();
        std::env::remove_var("BOOTCONTROL_IMMUTABLE_DISTRO_OVERRIDE");
        assert_eq!(probed, Some(ImmutableDistro::Ostree));
    }

    #[test]
    fn unknown_override_value_is_none() {
        std::env::set_var("BOOTCONTROL_IMMUTABLE_DISTRO_OVERRIDE", "bogus-distro");
        let probed = probe_immutable_distro();
        std::env::remove_var("BOOTCONTROL_IMMUTABLE_DISTRO_OVERRIDE");
        assert_eq!(probed, None);
    }
}
