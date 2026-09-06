//! State machine for the GRUB menu list and its Inspector selection.

use bootcontrol_client::GrubMenuEntryDto;

/// Current result of loading generated GRUB menu entries.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BootEntriesStatus {
    /// No load has started.
    #[default]
    Idle,
    /// A daemon request is in progress.
    Loading,
    /// The daemon returned no menu entries.
    Empty,
    /// Menu entries are available.
    Ready,
    /// The daemon request or JSON adapter failed.
    Error,
}

/// Immutable values required to confirm and apply a staged default selection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GrubDefaultRequest {
    /// Default path read from `/etc/default/grub` before staging.
    pub previous_path: String,
    /// Newly selected bootable menu path.
    pub selected_path: String,
    /// Version of generated `grub.cfg` that defined the selected path.
    pub menu_etag: String,
    /// Version of `/etc/default/grub` that will be rewritten.
    pub config_etag: String,
}

/// Loaded menu data and the selected Inspector row.
#[derive(Debug, Default)]
pub struct BootEntriesModel {
    entries: Vec<GrubMenuEntryDto>,
    etag: String,
    error: String,
    selected_index: Option<usize>,
    status: BootEntriesStatus,
    default_path: String,
    config_etag: String,
    staged_default: Option<String>,
}

impl BootEntriesModel {
    /// Enter loading state and invalidate the previous menu version.
    ///
    /// # Arguments
    ///
    /// This function takes no arguments.
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrol_gui::boot_entries::{BootEntriesModel, BootEntriesStatus};
    /// let mut model = BootEntriesModel::default();
    /// model.begin_load();
    /// assert_eq!(model.status(), BootEntriesStatus::Loading);
    /// ```
    pub fn begin_load(&mut self) {
        self.entries.clear();
        self.etag.clear();
        self.error.clear();
        self.selected_index = None;
        self.staged_default = None;
        self.status = BootEntriesStatus::Loading;
    }

    /// Store a successful daemon result and select its first row.
    ///
    /// # Arguments
    ///
    /// * `entries` - Parsed menu records in GRUB order.
    /// * `etag` - SHA-256 version of the generated `grub.cfg`.
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrol_gui::boot_entries::{BootEntriesModel, BootEntriesStatus};
    /// let mut model = BootEntriesModel::default();
    /// model.finish_load(Vec::new(), "etag".to_string());
    /// assert_eq!(model.status(), BootEntriesStatus::Empty);
    /// ```
    pub fn finish_load(&mut self, entries: Vec<GrubMenuEntryDto>, etag: String) {
        self.selected_index = (!entries.is_empty()).then_some(0);
        self.status = if entries.is_empty() {
            BootEntriesStatus::Empty
        } else {
            BootEntriesStatus::Ready
        };
        self.entries = entries;
        self.etag = etag;
        self.error.clear();
        self.staged_default = None;
    }

    /// Store a load error and invalidate all previously loaded rows.
    ///
    /// # Arguments
    ///
    /// * `error` - User-facing failure detail.
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrol_gui::boot_entries::{BootEntriesModel, BootEntriesStatus};
    /// let mut model = BootEntriesModel::default();
    /// model.fail_load("missing grub.cfg".to_string());
    /// assert_eq!(model.status(), BootEntriesStatus::Error);
    /// ```
    pub fn fail_load(&mut self, error: String) {
        self.entries.clear();
        self.etag.clear();
        self.selected_index = None;
        self.staged_default = None;
        self.error = error;
        self.status = BootEntriesStatus::Error;
    }

    /// Select a row by zero-based index when it exists.
    ///
    /// # Arguments
    ///
    /// * `index` - Requested row index.
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrol_gui::boot_entries::BootEntriesModel;
    /// let mut model = BootEntriesModel::default();
    /// model.select(4);
    /// assert!(model.selected().is_none());
    /// ```
    pub fn select(&mut self, index: usize) {
        if index < self.entries.len() {
            self.selected_index = Some(index);
        }
    }

    /// Move the selection by a signed keyboard delta and clamp at list edges.
    ///
    /// # Arguments
    ///
    /// * `delta` - Negative for earlier rows and positive for later rows.
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrol_gui::boot_entries::BootEntriesModel;
    /// let mut model = BootEntriesModel::default();
    /// model.move_selection(1);
    /// assert!(model.selected().is_none());
    /// ```
    pub fn move_selection(&mut self, delta: isize) {
        let Some(current) = self.selected_index else {
            return;
        };
        let last = self.entries.len().saturating_sub(1);
        let next = if delta.is_negative() {
            current.saturating_sub(delta.unsigned_abs())
        } else {
            current.saturating_add(delta as usize).min(last)
        };
        self.selected_index = Some(next);
    }

    /// Store the current source configuration value and version.
    ///
    /// Loading fresh source state discards any staged selection because its
    /// optimistic-lock base has changed.
    ///
    /// # Arguments
    ///
    /// * `default_path` - Current effective `GRUB_DEFAULT` value.
    /// * `config_etag` - Current `/etc/default/grub` ETag.
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrol_gui::boot_entries::BootEntriesModel;
    /// let mut model = BootEntriesModel::default();
    /// model.set_config_state("0".into(), "config-etag".into());
    /// assert_eq!(model.effective_default(), "0");
    /// ```
    pub fn set_config_state(&mut self, default_path: String, config_etag: String) {
        self.default_path = default_path;
        self.config_etag = config_etag;
        self.staged_default = None;
    }

    /// Stage the selected bootable entry as the persistent default.
    ///
    /// # Arguments
    ///
    /// This function takes no arguments.
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrol_gui::boot_entries::BootEntriesModel;
    /// let mut model = BootEntriesModel::default();
    /// assert!(!model.stage_selected_as_default());
    /// ```
    pub fn stage_selected_as_default(&mut self) -> bool {
        let Some(entry) = self.selected() else {
            return false;
        };
        if entry.is_submenu
            || entry.path == self.default_path
            || self.etag.is_empty()
            || self.config_etag.is_empty()
        {
            return false;
        }
        self.staged_default = Some(entry.path.clone());
        true
    }

    /// Discard the staged default selection.
    ///
    /// # Arguments
    ///
    /// This function takes no arguments.
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrol_gui::boot_entries::BootEntriesModel;
    /// let mut model = BootEntriesModel::default();
    /// model.discard_default_change();
    /// assert_eq!(model.pending_count(), 0);
    /// ```
    pub fn discard_default_change(&mut self) {
        self.staged_default = None;
    }

    /// Return the number of staged changes represented by this model.
    ///
    /// # Arguments
    ///
    /// This function takes no arguments.
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrol_gui::boot_entries::BootEntriesModel;
    /// assert_eq!(BootEntriesModel::default().pending_count(), 0);
    /// ```
    pub fn pending_count(&self) -> i32 {
        i32::from(self.staged_default.is_some())
    }

    /// Return the staged default path, or the persisted path when clean.
    ///
    /// # Arguments
    ///
    /// This function takes no arguments.
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrol_gui::boot_entries::BootEntriesModel;
    /// assert_eq!(BootEntriesModel::default().effective_default(), "");
    /// ```
    pub fn effective_default(&self) -> &str {
        self.staged_default
            .as_deref()
            .unwrap_or(self.default_path.as_str())
    }

    /// Build a versioned request for the currently staged selection.
    ///
    /// # Arguments
    ///
    /// This function takes no arguments.
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrol_gui::boot_entries::BootEntriesModel;
    /// assert!(BootEntriesModel::default().pending_default_request().is_none());
    /// ```
    pub fn pending_default_request(&self) -> Option<GrubDefaultRequest> {
        self.staged_default
            .as_ref()
            .map(|selected_path| GrubDefaultRequest {
                previous_path: self.default_path.clone(),
                selected_path: selected_path.clone(),
                menu_etag: self.etag.clone(),
                config_etag: self.config_etag.clone(),
            })
    }

    /// Return the current load status.
    ///
    /// # Arguments
    ///
    /// This function takes no arguments.
    pub fn status(&self) -> BootEntriesStatus {
        self.status
    }

    /// Return all loaded entries in menu order.
    ///
    /// # Arguments
    ///
    /// This function takes no arguments.
    pub fn entries(&self) -> &[GrubMenuEntryDto] {
        &self.entries
    }

    /// Return the loaded `grub.cfg` ETag.
    ///
    /// # Arguments
    ///
    /// This function takes no arguments.
    pub fn etag(&self) -> &str {
        &self.etag
    }

    /// Return the current load error or an empty string.
    ///
    /// # Arguments
    ///
    /// This function takes no arguments.
    pub fn error(&self) -> &str {
        &self.error
    }

    /// Return the selected row, if the list is non-empty.
    ///
    /// # Arguments
    ///
    /// This function takes no arguments.
    pub fn selected(&self) -> Option<&GrubMenuEntryDto> {
        self.selected_index
            .and_then(|index| self.entries.get(index))
    }

    /// Return the selected zero-based index, or `-1` for no selection.
    ///
    /// # Arguments
    ///
    /// This function takes no arguments.
    pub fn selected_index(&self) -> i32 {
        self.selected_index
            .and_then(|index| i32::try_from(index).ok())
            .unwrap_or(-1)
    }
}
