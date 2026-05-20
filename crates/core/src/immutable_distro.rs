//! Detection of immutable / atomic distros where a naive write to
//! `/etc/default/grub`, `/boot/loader/entries/*.conf` or the ESP would either
//! fail on a read-only mount or get rolled back on the next rebase.
//!
//! This is the **pure** detection layer: the caller (the daemon) is
//! responsible for the actual filesystem checks and passes the resulting
//! booleans here. That keeps this function unit-testable without root, mounts,
//! or a real ostree tree.
//!
//! # What we detect
//!
//! Five immutable / atomic root layouts, in priority order. The first match
//! wins — `RpmOstree` is checked before bare `Ostree` because the
//! delegation path (Phase 6 PR2) only works when `rpm-ostree` is available.
//!
//! | Flavour | Signal(s) | Real-world example |
//! |---------|----------|--------------------|
//! | `ImmutableDistro::RpmOstree` | `/run/ostree-booted` AND `rpm-ostree` resolvable on `$PATH` | Fedora Silverblue / Kinoite / IoT |
//! | `ImmutableDistro::Ostree`    | `/run/ostree-booted` OR `/sysroot/ostree/deploy/` | bare libostree, Endless OS |
//! | `ImmutableDistro::SteamOs`   | `/etc/os-release` `ID=steamos` (or `holo`) OR `/etc/steamos-release` | Steam Deck / SteamOS 3.x A/B |
//! | `ImmutableDistro::NixOs`     | `/etc/NIXOS` OR `/etc/os-release` `ID=nixos` | NixOS (declarative root) |
//! | `ImmutableDistro::VanillaOs` | `/etc/os-release` `ID=vanilla` (or `vanilla-os`) | Vanilla OS A/B atomic |
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

#![deny(missing_docs)]

/// Classification of an immutable / atomic distro layout.
///
/// Returned by [`detect_immutable_distro`] when the host is one of the
/// supported atomic variants. PR2 already plugs `RpmOstree` into a dedicated
/// `rpm-ostree kargs` code path. The remaining variants share the
/// "refuse-with-clear-error" behaviour from PR1.
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

    /// SteamOS 3.x A/B layout (Steam Deck, Holo). The `/etc` overlay survives
    /// reboots but a `steamos-readonly disable` is required to write the root
    /// partition, and even then the next system update can wipe the changes.
    /// User-facing knob is the SteamOS Settings → "Developer Mode" menu.
    SteamOs,

    /// NixOS declarative root. `/etc/default/grub` does not exist; the
    /// bootloader configuration is regenerated from `configuration.nix` by
    /// `nixos-rebuild`. Touching it from BootControl would be unfixable
    /// drift.
    NixOs,

    /// Vanilla OS A/B atomic layout. `/etc/default/grub` is overwritten on
    /// every system update via `abroot`. User-facing knob is the Vanilla OS
    /// settings GUI or `abroot kargs`.
    VanillaOs,
}

impl std::fmt::Display for ImmutableDistro {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImmutableDistro::Ostree => write!(f, "ostree"),
            ImmutableDistro::RpmOstree => write!(f, "rpm-ostree"),
            ImmutableDistro::SteamOs => write!(f, "steamos"),
            ImmutableDistro::NixOs => write!(f, "nixos"),
            ImmutableDistro::VanillaOs => write!(f, "vanilla-os"),
        }
    }
}

/// Pre-checked filesystem and `$PATH` indicators that drive
/// [`detect_immutable_distro`].
///
/// Keeping the indicators in a struct (rather than a growing positional
/// argument list) makes new distros additive without touching every call-
/// site, and keeps the test truth tables easy to read.
///
/// Every field is `false` by default so callers only have to fill in the
/// signals they actually checked — handy in unit tests.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DistroIndicators {
    /// `/run/ostree-booted` exists. Canonical "running under ostree" signal,
    /// written by ostree's initrd.
    pub ostree_booted_marker: bool,
    /// `/sysroot/ostree/deploy/` exists. Secondary signal for hosts where the
    /// runtime marker is missing but the on-disk layout is still ostree
    /// (e.g. a live ISO booted into the deploy tree).
    pub ostree_sysroot_exists: bool,
    /// `rpm-ostree` resolves on `$PATH`. Distinguishes Fedora atomic variants
    /// from bare libostree so PR2's delegation path can fire.
    pub rpm_ostree_available: bool,
    /// SteamOS marker. Set when `/etc/steamos-release` exists OR
    /// `/etc/os-release`'s `ID=` value is `steamos` or `holo`.
    pub steamos_marker: bool,
    /// NixOS marker. Set when `/etc/NIXOS` exists OR `/etc/os-release`'s
    /// `ID=` value is `nixos`.
    pub nixos_marker: bool,
    /// Vanilla OS marker. Set when `/etc/os-release`'s `ID=` value is
    /// `vanilla` or `vanilla-os`.
    pub vanillaos_marker: bool,
}

/// Detect whether the host is an immutable / atomic distro from pre-checked
/// filesystem and `$PATH` indicators.
///
/// This is a pure function: all I/O is performed by the caller and passed in
/// as a [`DistroIndicators`] struct, so the function is unit-testable
/// without root.
///
/// # Detection priority (deterministic — first match wins)
///
/// 1. `ostree_booted_marker` AND `rpm_ostree_available` → `RpmOstree`
/// 2. `ostree_booted_marker` OR `ostree_sysroot_exists` → `Ostree`
/// 3. `steamos_marker` → `SteamOs`
/// 4. `nixos_marker` → `NixOs`
/// 5. `vanillaos_marker` → `VanillaOs`
/// 6. Otherwise → `None`
///
/// # Examples
///
/// ```
/// use bootcontrol_core::immutable_distro::{
///     detect_immutable_distro, DistroIndicators, ImmutableDistro,
/// };
///
/// // Fedora Silverblue: runtime marker + rpm-ostree present.
/// assert_eq!(
///     detect_immutable_distro(&DistroIndicators {
///         ostree_booted_marker: true,
///         rpm_ostree_available: true,
///         ..Default::default()
///     }),
///     Some(ImmutableDistro::RpmOstree)
/// );
///
/// // Bare ostree: marker but no rpm-ostree.
/// assert_eq!(
///     detect_immutable_distro(&DistroIndicators {
///         ostree_booted_marker: true,
///         ..Default::default()
///     }),
///     Some(ImmutableDistro::Ostree)
/// );
///
/// // Steam Deck.
/// assert_eq!(
///     detect_immutable_distro(&DistroIndicators {
///         steamos_marker: true,
///         ..Default::default()
///     }),
///     Some(ImmutableDistro::SteamOs)
/// );
///
/// // Vanilla Ubuntu / Arch / classic Fedora — no markers anywhere.
/// assert_eq!(
///     detect_immutable_distro(&DistroIndicators::default()),
///     None
/// );
/// ```
pub fn detect_immutable_distro(ind: &DistroIndicators) -> Option<ImmutableDistro> {
    if ind.ostree_booted_marker && ind.rpm_ostree_available {
        return Some(ImmutableDistro::RpmOstree);
    }
    if ind.ostree_booted_marker || ind.ostree_sysroot_exists {
        return Some(ImmutableDistro::Ostree);
    }
    if ind.steamos_marker {
        return Some(ImmutableDistro::SteamOs);
    }
    if ind.nixos_marker {
        return Some(ImmutableDistro::NixOs);
    }
    if ind.vanillaos_marker {
        return Some(ImmutableDistro::VanillaOs);
    }
    None
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn silverblue() -> DistroIndicators {
        DistroIndicators {
            ostree_booted_marker: true,
            rpm_ostree_available: true,
            ..Default::default()
        }
    }

    #[test]
    fn fedora_silverblue_returns_rpm_ostree() {
        assert_eq!(
            detect_immutable_distro(&silverblue()),
            Some(ImmutableDistro::RpmOstree)
        );
    }

    #[test]
    fn rpm_ostree_requires_runtime_marker() {
        // Static layout + rpm-ostree CLI present, but no runtime marker.
        // We refuse to call this RpmOstree because we cannot be sure the
        // CLI was installed against THIS root deploy.
        assert_eq!(
            detect_immutable_distro(&DistroIndicators {
                ostree_sysroot_exists: true,
                rpm_ostree_available: true,
                ..Default::default()
            }),
            Some(ImmutableDistro::Ostree)
        );
    }

    #[test]
    fn bare_ostree_when_no_rpm_ostree_wrapper() {
        assert_eq!(
            detect_immutable_distro(&DistroIndicators {
                ostree_booted_marker: true,
                ..Default::default()
            }),
            Some(ImmutableDistro::Ostree)
        );
    }

    #[test]
    fn sysroot_layout_alone_counts_as_ostree() {
        assert_eq!(
            detect_immutable_distro(&DistroIndicators {
                ostree_sysroot_exists: true,
                ..Default::default()
            }),
            Some(ImmutableDistro::Ostree)
        );
    }

    #[test]
    fn rpm_ostree_cli_alone_is_not_immutable() {
        // Developer host with the CLI installed for inspection but running
        // a classic mutable root.
        assert_eq!(
            detect_immutable_distro(&DistroIndicators {
                rpm_ostree_available: true,
                ..Default::default()
            }),
            None
        );
    }

    #[test]
    fn classic_distro_returns_none() {
        assert_eq!(detect_immutable_distro(&DistroIndicators::default()), None);
    }

    #[test]
    fn steamos_marker_returns_steamos() {
        assert_eq!(
            detect_immutable_distro(&DistroIndicators {
                steamos_marker: true,
                ..Default::default()
            }),
            Some(ImmutableDistro::SteamOs)
        );
    }

    #[test]
    fn nixos_marker_returns_nixos() {
        assert_eq!(
            detect_immutable_distro(&DistroIndicators {
                nixos_marker: true,
                ..Default::default()
            }),
            Some(ImmutableDistro::NixOs)
        );
    }

    #[test]
    fn vanillaos_marker_returns_vanilla() {
        assert_eq!(
            detect_immutable_distro(&DistroIndicators {
                vanillaos_marker: true,
                ..Default::default()
            }),
            Some(ImmutableDistro::VanillaOs)
        );
    }

    #[test]
    fn ostree_takes_priority_over_steamos() {
        // If both markers somehow appear, prefer ostree because it has the
        // working delegation path on rpm-ostree.
        assert_eq!(
            detect_immutable_distro(&DistroIndicators {
                ostree_booted_marker: true,
                steamos_marker: true,
                ..Default::default()
            }),
            Some(ImmutableDistro::Ostree)
        );
    }

    #[test]
    fn display_names_match_d_bus_distro_tag() {
        assert_eq!(ImmutableDistro::Ostree.to_string(), "ostree");
        assert_eq!(ImmutableDistro::RpmOstree.to_string(), "rpm-ostree");
        assert_eq!(ImmutableDistro::SteamOs.to_string(), "steamos");
        assert_eq!(ImmutableDistro::NixOs.to_string(), "nixos");
        assert_eq!(ImmutableDistro::VanillaOs.to_string(), "vanilla-os");
    }
}
