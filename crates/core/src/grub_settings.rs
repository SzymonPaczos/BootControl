//! Typed, pure parsing for the supported `/etc/default/grub` settings.

use crate::error::BootControlError;
use crate::grub::parse_grub_config;

const MAX_TIMEOUT_SECONDS: u32 = 1_000_000;

/// Supported rendering mode for the GRUB timeout.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GrubTimeoutStyle {
    /// Display the complete menu.
    Menu,
    /// Display a countdown before the menu.
    Countdown,
    /// Keep the menu hidden unless the user requests it during boot.
    Hidden,
}

impl GrubTimeoutStyle {
    /// Return the value written to `GRUB_TIMEOUT_STYLE`.
    ///
    /// # Arguments
    ///
    /// This function takes no arguments.
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrol_core::grub_settings::GrubTimeoutStyle;
    /// assert_eq!(GrubTimeoutStyle::Countdown.as_str(), "countdown");
    /// ```
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Menu => "menu",
            Self::Countdown => "countdown",
            Self::Hidden => "hidden",
        }
    }
}

/// Typed values exposed by the supported Bootloader controls.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GrubSettings {
    /// Seconds before GRUB starts its default entry.
    pub timeout_seconds: u32,
    /// How GRUB presents the timeout.
    pub timeout_style: GrubTimeoutStyle,
    /// Whether os-prober integration is enabled.
    pub detect_other_os: bool,
    /// Whether recovery menu entries are generated.
    pub generate_recovery_entries: bool,
}

/// Validate settings constructed by a typed client before serialization.
///
/// # Arguments
///
/// * `settings` - Candidate values for the supported GRUB controls.
///
/// # Errors
///
/// Returns [`BootControlError::MalformedValue`] when `timeout_seconds` exceeds
/// `1_000_000`.
///
/// # Examples
///
/// ```
/// use bootcontrol_core::grub_settings::{validate_grub_settings, GrubSettings, GrubTimeoutStyle};
/// let settings = GrubSettings {
///     timeout_seconds: 5,
///     timeout_style: GrubTimeoutStyle::Menu,
///     detect_other_os: true,
///     generate_recovery_entries: true,
/// };
/// assert!(validate_grub_settings(&settings).is_ok());
/// ```
pub fn validate_grub_settings(settings: &GrubSettings) -> Result<(), BootControlError> {
    if settings.timeout_seconds > MAX_TIMEOUT_SECONDS {
        Err(BootControlError::MalformedValue {
            key: "GRUB_TIMEOUT".to_string(),
            reason: format!("expected an integer from 0 to {MAX_TIMEOUT_SECONDS}"),
        })
    } else {
        Ok(())
    }
}

/// Parse supported GRUB settings from a strict-subset shell configuration.
///
/// Missing values use GRUB-compatible defaults: timeout `5`, style `menu`,
/// other-OS detection enabled, and recovery entries enabled.
///
/// # Arguments
///
/// * `input` - Raw contents of `/etc/default/grub`.
///
/// # Errors
///
/// Returns [`BootControlError::ComplexBashDetected`] for executable shell
/// constructs. Returns [`BootControlError::MalformedValue`] for a timeout
/// outside `0..=1_000_000`, a style other than `menu`, `countdown`, or
/// `hidden`, or a boolean other than `true` or `false`.
///
/// # Examples
///
/// ```
/// use bootcontrol_core::grub_settings::{parse_grub_settings, GrubTimeoutStyle};
/// let settings = parse_grub_settings("GRUB_TIMEOUT=8\nGRUB_TIMEOUT_STYLE=menu\n").unwrap();
/// assert_eq!(settings.timeout_seconds, 8);
/// assert_eq!(settings.timeout_style, GrubTimeoutStyle::Menu);
/// ```
pub fn parse_grub_settings(input: &str) -> Result<GrubSettings, BootControlError> {
    let config = parse_grub_config(input)?;
    let timeout_seconds = match config.map.get("GRUB_TIMEOUT").map(String::as_str) {
        Some(value) => parse_timeout(value)?,
        None => 5,
    };
    let timeout_style = match config.map.get("GRUB_TIMEOUT_STYLE").map(String::as_str) {
        Some("menu") | None => GrubTimeoutStyle::Menu,
        Some("countdown") => GrubTimeoutStyle::Countdown,
        Some("hidden") => GrubTimeoutStyle::Hidden,
        Some(value) => {
            return Err(BootControlError::MalformedValue {
                key: "GRUB_TIMEOUT_STYLE".to_string(),
                reason: format!("unsupported timeout style '{value}'"),
            });
        }
    };
    let disable_os_prober = parse_optional_bool(
        config.map.get("GRUB_DISABLE_OS_PROBER").map(String::as_str),
        "GRUB_DISABLE_OS_PROBER",
    )?;
    let disable_recovery = parse_optional_bool(
        config.map.get("GRUB_DISABLE_RECOVERY").map(String::as_str),
        "GRUB_DISABLE_RECOVERY",
    )?;

    Ok(GrubSettings {
        timeout_seconds,
        timeout_style,
        detect_other_os: !disable_os_prober,
        generate_recovery_entries: !disable_recovery,
    })
}

fn parse_timeout(value: &str) -> Result<u32, BootControlError> {
    value
        .parse::<u32>()
        .ok()
        .filter(|timeout| *timeout <= MAX_TIMEOUT_SECONDS)
        .ok_or_else(|| BootControlError::MalformedValue {
            key: "GRUB_TIMEOUT".to_string(),
            reason: format!("expected an integer from 0 to {MAX_TIMEOUT_SECONDS}"),
        })
}

fn parse_optional_bool(value: Option<&str>, key: &str) -> Result<bool, BootControlError> {
    match value {
        None | Some("false") => Ok(false),
        Some("true") => Ok(true),
        Some(other) => Err(BootControlError::MalformedValue {
            key: key.to_string(),
            reason: format!("expected true or false, found '{other}'"),
        }),
    }
}
