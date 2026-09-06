//! Staged state for the typed GRUB settings shown on the Bootloader page.

use bootcontrol_client::GrubSettingsDto;
use bootcontrol_core::grub_settings::{validate_grub_settings, GrubSettings, GrubTimeoutStyle};

/// Current typed-settings load state.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum GrubSettingsStatus {
    /// No request has started.
    #[default]
    Idle,
    /// A daemon read is in progress.
    Loading,
    /// Current values are ready.
    Ready,
    /// The read or typed parser failed.
    Error,
}

/// One setting changed relative to the last successful daemon read.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GrubSettingChange {
    /// GRUB variable name.
    pub key: String,
    /// Value read from the daemon.
    pub previous: String,
    /// Locally staged value.
    pub desired: String,
}

/// Exact data carried from staging through confirmation to Apply.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GrubSettingsRequest {
    /// Desired typed values.
    pub settings: GrubSettingsDto,
    /// ETag of `/etc/default/grub` at edit start.
    pub etag: String,
    /// Per-key diff used by the Confirmation Sheet.
    pub changes: Vec<GrubSettingChange>,
}

/// Loaded and locally staged Bootloader settings.
#[derive(Debug, Default)]
pub struct GrubSettingsModel {
    baseline: Option<GrubSettingsDto>,
    current: Option<GrubSettingsDto>,
    etag: String,
    error: String,
    status: GrubSettingsStatus,
}

impl GrubSettingsModel {
    /// Invalidate stale data while a new typed read is in progress.
    ///
    /// # Arguments
    ///
    /// This function takes no arguments.
    pub fn begin_load(&mut self) {
        self.baseline = None;
        self.current = None;
        self.etag.clear();
        self.error.clear();
        self.status = GrubSettingsStatus::Loading;
    }

    /// Store a successful typed read and use it as the staging baseline.
    ///
    /// # Arguments
    ///
    /// * `settings` - Typed values returned by the daemon.
    /// * `etag` - Source configuration version returned by the same call.
    pub fn finish_load(&mut self, settings: GrubSettingsDto, etag: String) {
        self.baseline = Some(settings.clone());
        self.current = Some(settings);
        self.etag = etag;
        self.error.clear();
        self.status = GrubSettingsStatus::Ready;
    }

    /// Store a load error and remove every stale writable value.
    ///
    /// # Arguments
    ///
    /// * `error` - User-facing failure description.
    pub fn fail_load(&mut self, error: String) {
        self.baseline = None;
        self.current = None;
        self.etag.clear();
        self.error = error;
        self.status = GrubSettingsStatus::Error;
    }

    /// Stage a validated decimal timeout.
    ///
    /// # Arguments
    ///
    /// * `value` - Decimal seconds entered by the user.
    ///
    /// # Errors
    ///
    /// Returns a message when the value is not an integer in `0..=1_000_000`
    /// or when no current settings are loaded.
    pub fn stage_timeout(&mut self, value: &str) -> Result<bool, String> {
        let timeout_seconds = value
            .parse::<u32>()
            .map_err(|_| "Timeout must be an integer from 0 to 1000000.".to_string())?;
        let current = self
            .current
            .as_mut()
            .ok_or_else(|| "Refresh typed GRUB settings before editing.".to_string())?;
        let validation = GrubSettings {
            timeout_seconds,
            timeout_style: style_from_str(&current.timeout_style)?,
            detect_other_os: current.detect_other_os,
            generate_recovery_entries: current.generate_recovery_entries,
        };
        validate_grub_settings(&validation).map_err(|error| error.to_string())?;
        let changed = current.timeout_seconds != timeout_seconds;
        current.timeout_seconds = timeout_seconds;
        Ok(changed)
    }

    /// Stage one of the three supported timeout styles.
    ///
    /// # Arguments
    ///
    /// * `style` - `menu`, `countdown`, or `hidden`.
    ///
    /// # Errors
    ///
    /// Returns a message for an unsupported style or missing loaded state.
    pub fn stage_timeout_style(&mut self, style: &str) -> Result<bool, String> {
        style_from_str(style)?;
        let current = self
            .current
            .as_mut()
            .ok_or_else(|| "Refresh typed GRUB settings before editing.".to_string())?;
        let changed = current.timeout_style != style;
        current.timeout_style = style.to_string();
        Ok(changed)
    }

    /// Stage the other-operating-system detection toggle.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Desired positive UI value.
    pub fn stage_detect_other_os(&mut self, enabled: bool) -> bool {
        let Some(current) = self.current.as_mut() else {
            return false;
        };
        let changed = current.detect_other_os != enabled;
        current.detect_other_os = enabled;
        changed
    }

    /// Stage the recovery-entry generation toggle.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Desired positive UI value.
    pub fn stage_recovery_entries(&mut self, enabled: bool) -> bool {
        let Some(current) = self.current.as_mut() else {
            return false;
        };
        let changed = current.generate_recovery_entries != enabled;
        current.generate_recovery_entries = enabled;
        changed
    }

    /// Discard every staged value.
    ///
    /// # Arguments
    ///
    /// This function takes no arguments.
    pub fn discard(&mut self) {
        self.current.clone_from(&self.baseline);
    }

    /// Return a versioned request when at least one value is staged.
    ///
    /// # Arguments
    ///
    /// This function takes no arguments.
    pub fn pending_request(&self) -> Option<GrubSettingsRequest> {
        let changes = self.changes();
        if changes.is_empty() || self.etag.is_empty() {
            return None;
        }
        Some(GrubSettingsRequest {
            settings: self.current.clone()?,
            etag: self.etag.clone(),
            changes,
        })
    }

    /// Return the number of staged keys.
    ///
    /// # Arguments
    ///
    /// This function takes no arguments.
    pub fn pending_count(&self) -> i32 {
        i32::try_from(self.changes().len()).unwrap_or(i32::MAX)
    }

    /// Return current staged values when loaded.
    ///
    /// # Arguments
    ///
    /// This function takes no arguments.
    pub fn settings(&self) -> Option<&GrubSettingsDto> {
        self.current.as_ref()
    }

    /// Return the source ETag.
    ///
    /// # Arguments
    ///
    /// This function takes no arguments.
    pub fn etag(&self) -> &str {
        &self.etag
    }

    /// Return the current error text.
    ///
    /// # Arguments
    ///
    /// This function takes no arguments.
    pub fn error(&self) -> &str {
        &self.error
    }

    /// Return the current load status.
    ///
    /// # Arguments
    ///
    /// This function takes no arguments.
    pub fn status(&self) -> GrubSettingsStatus {
        self.status
    }

    fn changes(&self) -> Vec<GrubSettingChange> {
        let (Some(baseline), Some(current)) = (&self.baseline, &self.current) else {
            return Vec::new();
        };
        let mut changes = Vec::new();
        push_change(
            &mut changes,
            "GRUB_TIMEOUT",
            baseline.timeout_seconds.to_string(),
            current.timeout_seconds.to_string(),
        );
        push_change(
            &mut changes,
            "GRUB_TIMEOUT_STYLE",
            baseline.timeout_style.clone(),
            current.timeout_style.clone(),
        );
        push_change(
            &mut changes,
            "GRUB_DISABLE_OS_PROBER",
            (!baseline.detect_other_os).to_string(),
            (!current.detect_other_os).to_string(),
        );
        push_change(
            &mut changes,
            "GRUB_DISABLE_RECOVERY",
            (!baseline.generate_recovery_entries).to_string(),
            (!current.generate_recovery_entries).to_string(),
        );
        changes
    }
}

fn style_from_str(style: &str) -> Result<GrubTimeoutStyle, String> {
    match style {
        "menu" => Ok(GrubTimeoutStyle::Menu),
        "countdown" => Ok(GrubTimeoutStyle::Countdown),
        "hidden" => Ok(GrubTimeoutStyle::Hidden),
        _ => Err("Timeout style must be menu, countdown, or hidden.".to_string()),
    }
}

fn push_change(changes: &mut Vec<GrubSettingChange>, key: &str, previous: String, desired: String) {
    if previous != desired {
        changes.push(GrubSettingChange {
            key: key.to_string(),
            previous,
            desired,
        });
    }
}
