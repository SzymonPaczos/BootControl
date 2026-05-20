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

use bootcontrol_core::immutable_distro::{
    detect_immutable_distro, DistroIndicators, ImmutableDistro,
};

/// `/run` marker file created by the ostree initrd to flag an ostree-booted
/// root. Authoritative signal — present iff the running root tree is an
/// ostree deploy.
const OSTREE_BOOTED_MARKER: &str = "/run/ostree-booted";

/// On-disk ostree deploy tree. Present even before the initrd marker is
/// written, so we accept it as a secondary signal for recovery shells and
/// live ISOs.
const OSTREE_SYSROOT_DIR: &str = "/sysroot/ostree/deploy";

/// SteamOS marker file shipped on `/etc` of Steam Deck / Holo systems.
const STEAMOS_RELEASE: &str = "/etc/steamos-release";

/// NixOS marker file written by the activation script on every boot.
const NIXOS_MARKER: &str = "/etc/NIXOS";

/// systemd-canonical machine identity file. Used as a fallback signal for
/// SteamOS / NixOS / Vanilla OS via the `ID=` field.
const OS_RELEASE: &str = "/etc/os-release";

/// Parse `/etc/os-release` and return the value of the `ID=` field with
/// surrounding quotes stripped. Returns `None` on any I/O or shape error —
/// the caller treats that as "no signal", which is the correct fallback on
/// classic distros where the field is `ubuntu` / `arch` / etc.
fn read_os_release_id(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    for line in content.lines() {
        if let Some(value) = line.strip_prefix("ID=") {
            // `ID=ubuntu` and `ID="ubuntu"` are both legal per the spec.
            let trimmed = value.trim().trim_matches('"').trim_matches('\'');
            return Some(trimmed.to_ascii_lowercase());
        }
    }
    None
}

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
                "steamos" => Some(ImmutableDistro::SteamOs),
                "nixos" => Some(ImmutableDistro::NixOs),
                "vanilla-os" => Some(ImmutableDistro::VanillaOs),
                _ => None,
            };
        }
    }

    let os_id = read_os_release_id(Path::new(OS_RELEASE));
    let id_is = |needle: &[&str]| -> bool {
        os_id
            .as_deref()
            .map(|id| needle.contains(&id))
            .unwrap_or(false)
    };

    let indicators = DistroIndicators {
        ostree_booted_marker: Path::new(OSTREE_BOOTED_MARKER).exists(),
        ostree_sysroot_exists: Path::new(OSTREE_SYSROOT_DIR).exists(),
        rpm_ostree_available: which::which("rpm-ostree").is_ok(),
        steamos_marker: Path::new(STEAMOS_RELEASE).exists() || id_is(&["steamos", "holo"]),
        nixos_marker: Path::new(NIXOS_MARKER).exists() || id_is(&["nixos"]),
        vanillaos_marker: id_is(&["vanilla", "vanilla-os"]),
    };

    detect_immutable_distro(&indicators)
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

    #[test]
    fn override_steamos_takes_precedence() {
        std::env::set_var("BOOTCONTROL_IMMUTABLE_DISTRO_OVERRIDE", "steamos");
        let probed = probe_immutable_distro();
        std::env::remove_var("BOOTCONTROL_IMMUTABLE_DISTRO_OVERRIDE");
        assert_eq!(probed, Some(ImmutableDistro::SteamOs));
    }

    #[test]
    fn override_nixos_takes_precedence() {
        std::env::set_var("BOOTCONTROL_IMMUTABLE_DISTRO_OVERRIDE", "nixos");
        let probed = probe_immutable_distro();
        std::env::remove_var("BOOTCONTROL_IMMUTABLE_DISTRO_OVERRIDE");
        assert_eq!(probed, Some(ImmutableDistro::NixOs));
    }

    #[test]
    fn override_vanilla_takes_precedence() {
        std::env::set_var("BOOTCONTROL_IMMUTABLE_DISTRO_OVERRIDE", "vanilla-os");
        let probed = probe_immutable_distro();
        std::env::remove_var("BOOTCONTROL_IMMUTABLE_DISTRO_OVERRIDE");
        assert_eq!(probed, Some(ImmutableDistro::VanillaOs));
    }

    #[test]
    fn read_os_release_id_extracts_value() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("os-release");
        std::fs::write(&path, "NAME=\"Steam Deck\"\nID=steamos\nVERSION_ID=3.4\n").unwrap();
        assert_eq!(read_os_release_id(&path).as_deref(), Some("steamos"));
    }

    #[test]
    fn read_os_release_id_handles_quoted_value() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("os-release");
        std::fs::write(&path, "ID=\"nixos\"\n").unwrap();
        assert_eq!(read_os_release_id(&path).as_deref(), Some("nixos"));
    }

    #[test]
    fn read_os_release_id_missing_file_returns_none() {
        assert_eq!(
            read_os_release_id(Path::new("/nonexistent-os-release")),
            None
        );
    }
}
