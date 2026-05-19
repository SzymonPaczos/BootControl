//! Detection of immutable / atomic distros where a naive write to
//! `/etc/default/grub`, `/boot/loader/entries/*.conf` or the ESP would either
//! fail on a read-only mount or get rolled back on the next rebase.
//!
//! This is the **pure** detection layer: the caller (the daemon) is
//! responsible for the actual filesystem checks and passes the resulting
//! booleans here. That keeps this function unit-testable without root, mounts,
//! or a real ostree tree.
//!
//! # What we detect (Phase 6 PR1)
//!
//! Only ostree-managed roots, in two flavours:
//!
//! | Flavour | Signal(s) | Real-world example |
//! |---------|----------|--------------------|
//! | `ImmutableDistro::RpmOstree` | `/run/ostree-booted` exists AND `rpm-ostree` resolvable on `$PATH` | Fedora Silverblue / Kinoite / IoT |
//! | `ImmutableDistro::Ostree`    | `/run/ostree-booted` exists OR `/sysroot/ostree/deploy/` exists | bare libostree systems, Endless OS |
//!
//! Other immutable styles (SteamOS btrfs snapshots, NixOS declarative root,
//! Vanilla OS A/B) are intentionally out of scope here — see Phase 6 PR3 in
//! [`ROADMAP.md`](../../../ROADMAP.md).
//!
//! # Why a pre-flight check at all
//!
//! BootControl's write-path is built around: parse → atomic rename → ETag
//! verify → failsafe entry. On ostree both the rename and the failsafe become
//! load-bearing assumptions that no longer hold: `/etc/` overlays are
//! ephemeral on next deploy, the ESP is owned by ostree's own boot-cfg
//! generator, and `grub-mkconfig` is replaced by ostree-grub-generator.
//! Refusing the write up-front is safer than producing a "successful" write
//! that vanishes silently after `rpm-ostree upgrade`.

#![deny(warnings)]
#![deny(missing_docs)]

/// Classification of an immutable / atomic distro layout.
///
/// Returned by [`detect_immutable_distro`] when the host is one of the
/// supported atomic variants. Phase 6 PR2 will plug `RpmOstree` into a
/// dedicated `rpm-ostree kargs` code path; Phase 6 PR3 will add more variants
/// (SteamOS, NixOS, Vanilla OS).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImmutableDistro {
    /// Generic ostree-managed root with no `rpm-ostree` wrapper available.
    /// BootControl must refuse the write and let the user reach for the
    /// distro-specific tool.
    Ostree,

    /// Fedora atomic variants (Silverblue, Kinoite, IoT, CoreOS). Detection
    /// requires both the `/run/ostree-booted` marker and a resolvable
    /// `rpm-ostree` binary so we can later (Phase 6 PR2) delegate kernel-
    /// argument changes to `rpm-ostree kargs`.
    RpmOstree,
}

impl std::fmt::Display for ImmutableDistro {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImmutableDistro::Ostree => write!(f, "ostree"),
            ImmutableDistro::RpmOstree => write!(f, "rpm-ostree"),
        }
    }
}

/// Detect whether the host is an immutable / atomic distro from pre-checked
/// filesystem and `$PATH` indicators.
///
/// This is a pure function: all I/O is performed by the caller and passed in
/// as booleans, so the function is unit-testable without root.
///
/// # Detection priority (deterministic — first match wins)
///
/// 1. `ostree_booted_marker` AND `rpm_ostree_available` → `RpmOstree`
/// 2. `ostree_booted_marker` OR `ostree_sysroot_exists` → `Ostree`
/// 3. Otherwise → `None`
///
/// # Arguments
///
/// * `ostree_booted_marker` — `true` if `/run/ostree-booted` exists. This
///   marker is created by ostree's initrd and is the canonical "we're running
///   under ostree" signal.
/// * `ostree_sysroot_exists` — `true` if `/sysroot/ostree/deploy/` exists.
///   Secondary signal for hosts where the runtime marker is missing but the
///   on-disk layout is still ostree (e.g. a live ISO booted into the deploy
///   tree).
/// * `rpm_ostree_available` — `true` if `rpm-ostree` resolves on `$PATH`. Used
///   to distinguish Fedora atomic variants from bare libostree.
///
/// # Examples
///
/// ```
/// use bootcontrol_core::immutable_distro::{detect_immutable_distro, ImmutableDistro};
///
/// // Fedora Silverblue: runtime marker + rpm-ostree present.
/// assert_eq!(
///     detect_immutable_distro(true, true, true),
///     Some(ImmutableDistro::RpmOstree)
/// );
///
/// // Bare ostree: marker but no rpm-ostree.
/// assert_eq!(
///     detect_immutable_distro(true, false, false),
///     Some(ImmutableDistro::Ostree)
/// );
///
/// // Static layout only (e.g. recovery shell that skipped ostree-prepare-root).
/// assert_eq!(
///     detect_immutable_distro(false, true, false),
///     Some(ImmutableDistro::Ostree)
/// );
///
/// // Vanilla Ubuntu / Arch / classic Fedora — no markers anywhere.
/// assert_eq!(detect_immutable_distro(false, false, false), None);
///
/// // rpm-ostree on $PATH without an ostree-booted system (developer host
/// // with the CLI installed for inspection) does NOT count as immutable.
/// assert_eq!(detect_immutable_distro(false, false, true), None);
/// ```
pub fn detect_immutable_distro(
    ostree_booted_marker: bool,
    ostree_sysroot_exists: bool,
    rpm_ostree_available: bool,
) -> Option<ImmutableDistro> {
    if ostree_booted_marker && rpm_ostree_available {
        return Some(ImmutableDistro::RpmOstree);
    }
    if ostree_booted_marker || ostree_sysroot_exists {
        return Some(ImmutableDistro::Ostree);
    }
    None
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fedora_silverblue_returns_rpm_ostree() {
        assert_eq!(
            detect_immutable_distro(true, true, true),
            Some(ImmutableDistro::RpmOstree)
        );
    }

    #[test]
    fn rpm_ostree_requires_runtime_marker() {
        // Static layout + rpm-ostree CLI present, but no runtime marker.
        // We refuse to call this RpmOstree because we cannot be sure the
        // CLI was installed against THIS root deploy.
        assert_eq!(
            detect_immutable_distro(false, true, true),
            Some(ImmutableDistro::Ostree)
        );
    }

    #[test]
    fn bare_ostree_when_no_rpm_ostree_wrapper() {
        assert_eq!(
            detect_immutable_distro(true, false, false),
            Some(ImmutableDistro::Ostree)
        );
    }

    #[test]
    fn sysroot_layout_alone_counts_as_ostree() {
        assert_eq!(
            detect_immutable_distro(false, true, false),
            Some(ImmutableDistro::Ostree)
        );
    }

    #[test]
    fn rpm_ostree_cli_alone_is_not_immutable() {
        // Developer host with the CLI installed for inspection but running
        // a classic mutable root.
        assert_eq!(detect_immutable_distro(false, false, true), None);
    }

    #[test]
    fn classic_distro_returns_none() {
        assert_eq!(detect_immutable_distro(false, false, false), None);
    }

    #[test]
    fn display_names_match_d_bus_distro_tag() {
        assert_eq!(ImmutableDistro::Ostree.to_string(), "ostree");
        assert_eq!(ImmutableDistro::RpmOstree.to_string(), "rpm-ostree");
    }
}
