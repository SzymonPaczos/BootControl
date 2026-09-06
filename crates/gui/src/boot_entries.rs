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

/// Loaded menu data and the selected Inspector row.
#[derive(Debug, Default)]
pub struct BootEntriesModel {
    entries: Vec<GrubMenuEntryDto>,
    etag: String,
    error: String,
    selected_index: Option<usize>,
    status: BootEntriesStatus,
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
