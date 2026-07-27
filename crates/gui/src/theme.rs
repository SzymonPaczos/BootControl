//! Runtime theme application — Stacja (GUI v2.1).
//!
//! Slint globals are compile-time singletons, so a wholesale global swap is
//! not possible; instead every color property of `Tokens` is rewritten in
//! one batch from one of the four palette globals declared in
//! `ui/tokens.slint` (`DarkPalette` / `LightPalette` / `HighContrastDark` /
//! `HighContrastLight`). Reading the values back from the generated globals
//! keeps the hex values single-sourced in `tokens.slint` — the Rust side
//! never spells a color.
//!
//! Palette resolution (`resolve_from_environment`):
//!   theme  = BOOTCONTROL_THEME=light|dark, else follow the system
//!            (GNOME `color-scheme`, dark on detection failure)
//!   HC     = BOOTCONTROL_HIGH_CONTRAST=1 or KDE HighContrast color scheme
//! The matrix of the two axes picks one of the four palettes.

use crate::{AppWindow, DarkPalette, HighContrastDark, HighContrastLight, LightPalette, Tokens};
use slint::ComponentHandle;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Palette {
    Dark,
    Light,
    HcDark,
    HcLight,
}

/// Copies every color property shared by the palette globals into `Tokens`.
/// Duck-typed at macro expansion, so one body serves all four global types.
macro_rules! apply_palette_to_tokens {
    ($t:expr, $p:expr) => {{
        $t.set_surface($p.get_surface());
        $t.set_surface_container($p.get_surface_container());
        $t.set_surface_container_high($p.get_surface_container_high());
        $t.set_surface_1($p.get_surface_1());
        $t.set_surface_2($p.get_surface_2());
        $t.set_surface_3($p.get_surface_3());
        $t.set_surface_sidebar($p.get_surface_sidebar());
        $t.set_surface_error_tint($p.get_surface_error_tint());
        $t.set_surface_error_tint_strong($p.get_surface_error_tint_strong());
        $t.set_on_surface($p.get_on_surface());
        $t.set_on_surface_muted($p.get_on_surface_muted());
        $t.set_on_surface_dim($p.get_on_surface_dim());
        $t.set_on_surface_faint($p.get_on_surface_faint());
        $t.set_on_surface_disabled($p.get_on_surface_disabled());
        $t.set_accent($p.get_accent());
        $t.set_accent_secondary($p.get_accent_secondary());
        $t.set_accent_info($p.get_accent_info());
        $t.set_on_accent($p.get_on_accent());
        $t.set_info($p.get_info());
        $t.set_success($p.get_success());
        $t.set_warning($p.get_warning());
        $t.set_warning_soft($p.get_warning_soft());
        $t.set_error($p.get_error());
        $t.set_on_error($p.get_on_error());
        $t.set_focus_ring_outer($p.get_focus_ring_outer());
        $t.set_focus_ring_inner($p.get_focus_ring_inner());
        $t.set_hairline($p.get_hairline());
        $t.set_hairline_strong($p.get_hairline_strong());
        $t.set_border_interactive($p.get_border_interactive());
    }};
}

pub fn apply(ui: &AppWindow, palette: Palette) {
    let t = ui.global::<Tokens>();
    match palette {
        Palette::Dark => apply_palette_to_tokens!(t, ui.global::<DarkPalette>()),
        Palette::Light => apply_palette_to_tokens!(t, ui.global::<LightPalette>()),
        Palette::HcDark => apply_palette_to_tokens!(t, ui.global::<HighContrastDark>()),
        Palette::HcLight => apply_palette_to_tokens!(t, ui.global::<HighContrastLight>()),
    }
    t.set_high_contrast(matches!(palette, Palette::HcDark | Palette::HcLight));
    t.set_light(matches!(palette, Palette::Light | Palette::HcLight));
}

pub fn resolve_from_environment() -> Palette {
    let high_contrast = std::env::var("BOOTCONTROL_HIGH_CONTRAST")
        .map(|v| v == "1")
        .unwrap_or(false)
        || kde_high_contrast_active();

    let light = match std::env::var("BOOTCONTROL_THEME").as_deref() {
        Ok("light") => true,
        Ok("dark") => false,
        _ => system_prefers_light(),
    };

    match (high_contrast, light) {
        (false, false) => Palette::Dark,
        (false, true) => Palette::Light,
        (true, false) => Palette::HcDark,
        (true, true) => Palette::HcLight,
    }
}

/// Best-effort GNOME preference probe; dark on any failure (the app's
/// historical default). `color-scheme` values: 'default' (light),
/// 'prefer-light', 'prefer-dark'.
fn system_prefers_light() -> bool {
    std::process::Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "color-scheme"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| {
            let scheme = String::from_utf8_lossy(&o.stdout);
            !scheme.contains("prefer-dark")
        })
        .unwrap_or(false)
}

pub fn gnome_animations_disabled() -> bool {
    // gsettings get org.gnome.desktop.interface enable-animations → "false"
    std::process::Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "enable-animations"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "false")
        .unwrap_or(false)
}

pub fn kde_high_contrast_active() -> bool {
    // KDE: ~/.config/kdeglobals → [General] / ColorScheme=… containing
    // "HighContrast" (case-insensitive) or "Breeze High Contrast" preset.
    let home = match std::env::var_os("HOME") {
        Some(h) => h,
        None => return false,
    };
    let path = std::path::PathBuf::from(home)
        .join(".config")
        .join("kdeglobals");
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return false,
    };
    content.lines().any(|l| {
        let lc = l.to_lowercase();
        lc.starts_with("colorscheme=") && lc.contains("high") && lc.contains("contrast")
    })
}
