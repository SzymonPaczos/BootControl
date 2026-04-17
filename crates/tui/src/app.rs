//! Application state machine for the BootControl TUI.
//!
//! All state mutations are **pure functions** — no I/O, no D-Bus, no filesystem
//! access.  The async event loop in [`crate::main`] drives state changes and is
//! responsible for persisting results back to the daemon.
//!
//! # Mode transitions
//!
//! ```text
//! Browse ──[Enter]──► Editing ──[Enter]──► Browse  (write accepted)
//!                             ──[Esc]───► Browse  (cancelled)
//!        ──[D-Bus error]────► ErrorPopup ──[Esc/Enter]──► Browse
//! ```

// ──────────────────────────────────────────────────────────────────────────────
// Public types
// ──────────────────────────────────────────────────────────────────────────────

/// A single GRUB configuration entry shown in the table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrubEntry {
    /// GRUB variable name, e.g. `GRUB_TIMEOUT`.
    pub key: String,
    /// Raw value with surrounding double-quotes stripped by the daemon.
    pub value: String,
}

/// Interaction mode — drives which key bindings are active.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Mode {
    /// Default — navigate the table with arrow keys.
    #[default]
    Browse,
    /// Edit popup is open; printable keys append to [`App::edit_buf`].
    Editing,
    /// Error overlay is shown; any key dismisses it.
    ErrorPopup,
}

/// Full immutable snapshot of the TUI state used by [`crate::ui::render`].
///
/// The fields are public so the renderer can read them directly without
/// accessor boilerplate.  Mutation is only done through the `App` methods below
/// to enforce invariants (e.g. selection always in bounds).
#[derive(Debug)]
pub struct App {
    /// Sorted list of GRUB entries displayed in the table.
    pub entries: Vec<GrubEntry>,
    /// Zero-based index of the highlighted row.
    pub selected: usize,
    /// Current ETag — must be sent with every `SetGrubValue` call.
    pub etag: String,
    /// Which interaction mode the UI is currently in.
    pub mode: Mode,
    /// Buffer backing the inline editor in [`Mode::Editing`].
    pub edit_buf: String,
    /// One-line status message shown in the footer.
    pub status_msg: String,
    /// Error message shown in the error popup, if any.
    pub error_msg: Option<String>,
    /// Set to `true` by `q` / `Esc` in Browse mode to signal the event loop to exit.
    pub should_quit: bool,
}

impl App {
    /// Construct a new [`App`] from the initial D-Bus snapshot.
    ///
    /// `entries` are sorted alphabetically by key so the table is stable across
    /// daemon reloads.
    ///
    /// # Arguments
    ///
    /// * `entries` — GRUB key-value pairs as returned by `ReadGrubConfig`.
    /// * `etag`    — The ETag returned alongside the entries.
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrol_tui::app::{App, GrubEntry};
    ///
    /// let entries = vec![
    ///     GrubEntry { key: "GRUB_TIMEOUT".into(), value: "5".into() },
    /// ];
    /// let app = App::new(entries, "abc".into());
    /// assert_eq!(app.selected, 0);
    /// assert!(!app.should_quit);
    /// ```
    pub fn new(mut entries: Vec<GrubEntry>, etag: String) -> Self {
        entries.sort_by(|a, b| a.key.cmp(&b.key));
        Self {
            entries,
            selected: 0,
            etag,
            mode: Mode::default(),
            edit_buf: String::new(),
            status_msg: String::from("Ready."),
            error_msg: None,
            should_quit: false,
        }
    }

    /// Replace the entry list and ETag after a successful reload from the daemon.
    ///
    /// The selection is clamped to stay within the new list length.
    ///
    /// # Arguments
    ///
    /// * `entries` — Fresh GRUB key-value pairs.
    /// * `etag`    — Fresh ETag.
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrol_tui::app::{App, GrubEntry};
    ///
    /// let mut app = App::new(vec![
    ///     GrubEntry { key: "A".into(), value: "1".into() },
    ///     GrubEntry { key: "B".into(), value: "2".into() },
    /// ], "old".into());
    /// app.selected = 1;
    ///
    /// // Reload with only one entry — selection must be clamped.
    /// app.apply_grub_entries(vec![GrubEntry { key: "A".into(), value: "99".into() }], "new".into());
    /// assert_eq!(app.selected, 0);
    /// assert_eq!(app.etag, "new");
    /// ```
    pub fn apply_grub_entries(&mut self, mut entries: Vec<GrubEntry>, etag: String) {
        entries.sort_by(|a, b| a.key.cmp(&b.key));
        self.entries = entries;
        self.etag = etag;
        // Keep selection in bounds.
        if !self.entries.is_empty() {
            self.selected = self.selected.min(self.entries.len() - 1);
        } else {
            self.selected = 0;
        }
    }

    /// Move the table selection up by one row, wrapping at the top.
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrol_tui::app::{App, GrubEntry};
    ///
    /// let entries = vec![
    ///     GrubEntry { key: "A".into(), value: "1".into() },
    ///     GrubEntry { key: "B".into(), value: "2".into() },
    /// ];
    /// let mut app = App::new(entries, "e".into());
    /// app.selected = 0;
    /// app.move_selection_up();
    /// assert_eq!(app.selected, 1, "wraps from top to bottom");
    /// ```
    pub fn move_selection_up(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        if self.selected == 0 {
            self.selected = self.entries.len() - 1;
        } else {
            self.selected -= 1;
        }
    }

    /// Move the table selection down by one row, wrapping at the bottom.
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrol_tui::app::{App, GrubEntry};
    ///
    /// let entries = vec![
    ///     GrubEntry { key: "A".into(), value: "1".into() },
    ///     GrubEntry { key: "B".into(), value: "2".into() },
    /// ];
    /// let mut app = App::new(entries, "e".into());
    /// app.selected = 1;
    /// app.move_selection_down();
    /// assert_eq!(app.selected, 0, "wraps from bottom to top");
    /// ```
    pub fn move_selection_down(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        self.selected = (self.selected + 1) % self.entries.len();
    }

    /// Return the currently selected entry, if the list is non-empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrol_tui::app::{App, GrubEntry};
    ///
    /// let mut app = App::new(vec![
    ///     GrubEntry { key: "GRUB_TIMEOUT".into(), value: "5".into() },
    /// ], "e".into());
    /// let entry = app.current_entry().unwrap();
    /// assert_eq!(entry.key, "GRUB_TIMEOUT");
    /// ```
    pub fn current_entry(&self) -> Option<&GrubEntry> {
        self.entries.get(self.selected)
    }

    /// Open the edit popup pre-filled with the current entry's value.
    ///
    /// No-op if the entry list is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrol_tui::app::{App, GrubEntry, Mode};
    ///
    /// let mut app = App::new(vec![
    ///     GrubEntry { key: "GRUB_TIMEOUT".into(), value: "5".into() },
    /// ], "e".into());
    /// app.open_edit_popup();
    /// assert_eq!(app.mode, Mode::Editing);
    /// assert_eq!(app.edit_buf, "5");
    /// ```
    pub fn open_edit_popup(&mut self) {
        if let Some(entry) = self.entries.get(self.selected) {
            self.edit_buf = entry.value.clone();
            self.mode = Mode::Editing;
        }
    }

    /// Append a character to the edit buffer.
    ///
    /// # Arguments
    ///
    /// * `c` — Character to append.
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrol_tui::app::{App, GrubEntry};
    ///
    /// let mut app = App::new(vec![GrubEntry { key: "K".into(), value: "".into() }], "e".into());
    /// app.push_char('5');
    /// assert_eq!(app.edit_buf, "5");
    /// ```
    pub fn push_char(&mut self, c: char) {
        self.edit_buf.push(c);
    }

    /// Remove the last character from the edit buffer (backspace).
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrol_tui::app::{App, GrubEntry};
    ///
    /// let mut app = App::new(vec![GrubEntry { key: "K".into(), value: "".into() }], "e".into());
    /// app.edit_buf = "hello".into();
    /// app.pop_char();
    /// assert_eq!(app.edit_buf, "hell");
    /// ```
    pub fn pop_char(&mut self) {
        self.edit_buf.pop();
    }

    /// Cancel the edit popup and return to Browse mode without making any change.
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrol_tui::app::{App, GrubEntry, Mode};
    ///
    /// let mut app = App::new(vec![GrubEntry { key: "K".into(), value: "v".into() }], "e".into());
    /// app.open_edit_popup();
    /// app.cancel_edit();
    /// assert_eq!(app.mode, Mode::Browse);
    /// ```
    pub fn cancel_edit(&mut self) {
        self.edit_buf.clear();
        self.mode = Mode::Browse;
    }

    /// Show error popup with the given message.
    ///
    /// # Arguments
    ///
    /// * `msg` — Human-readable error string (D-Bus error name + detail).
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrol_tui::app::{App, GrubEntry, Mode};
    ///
    /// let mut app = App::new(vec![], "e".into());
    /// app.show_error("PolkitDenied: not authorised".into());
    /// assert_eq!(app.mode, Mode::ErrorPopup);
    /// ```
    pub fn show_error(&mut self, msg: String) {
        self.error_msg = Some(msg);
        self.mode = Mode::ErrorPopup;
    }

    /// Dismiss the error popup and return to Browse mode.
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrol_tui::app::{App, GrubEntry, Mode};
    ///
    /// let mut app = App::new(vec![], "e".into());
    /// app.show_error("oops".into());
    /// app.dismiss_error();
    /// assert_eq!(app.mode, Mode::Browse);
    /// assert!(app.error_msg.is_none());
    /// ```
    pub fn dismiss_error(&mut self) {
        self.error_msg = None;
        self.mode = Mode::Browse;
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entries(keys: &[&str]) -> Vec<GrubEntry> {
        keys.iter()
            .map(|k| GrubEntry {
                key: k.to_string(),
                value: "v".to_string(),
            })
            .collect()
    }

    // ── App::new ──────────────────────────────────────────────────────────────

    #[test]
    fn new_sorts_entries_alphabetically() {
        let entries = make_entries(&["GRUB_TIMEOUT", "GRUB_CMDLINE_LINUX", "GRUB_DEFAULT"]);
        let app = App::new(entries, "e".into());
        let keys: Vec<&str> = app.entries.iter().map(|e| e.key.as_str()).collect();
        assert_eq!(keys, ["GRUB_CMDLINE_LINUX", "GRUB_DEFAULT", "GRUB_TIMEOUT"]);
    }

    #[test]
    fn new_starts_with_zero_selection_and_not_quit() {
        let app = App::new(make_entries(&["A"]), "e".into());
        assert_eq!(app.selected, 0);
        assert!(!app.should_quit);
    }

    // ── apply_grub_entries ────────────────────────────────────────────────────

    #[test]
    fn apply_grub_entries_clamps_selection_when_list_shrinks() {
        let mut app = App::new(make_entries(&["A", "B", "C"]), "old".into());
        app.selected = 2;
        app.apply_grub_entries(make_entries(&["A"]), "new".into());
        assert_eq!(app.selected, 0);
        assert_eq!(app.etag, "new");
    }

    #[test]
    fn apply_grub_entries_handles_empty_list() {
        let mut app = App::new(make_entries(&["A"]), "e".into());
        app.selected = 0;
        app.apply_grub_entries(vec![], "e2".into());
        assert_eq!(app.selected, 0); // must not underflow
    }

    // ── move_selection_up ─────────────────────────────────────────────────────

    #[test]
    fn move_up_wraps_from_top_to_bottom() {
        let mut app = App::new(make_entries(&["A", "B"]), "e".into());
        app.selected = 0;
        app.move_selection_up();
        assert_eq!(app.selected, 1);
    }

    #[test]
    fn move_up_decrements_normally() {
        let mut app = App::new(make_entries(&["A", "B"]), "e".into());
        app.selected = 1;
        app.move_selection_up();
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn move_up_noop_on_empty_list() {
        let mut app = App::new(vec![], "e".into());
        app.move_selection_up(); // must not panic
        assert_eq!(app.selected, 0);
    }

    // ── move_selection_down ───────────────────────────────────────────────────

    #[test]
    fn move_down_wraps_from_bottom_to_top() {
        let mut app = App::new(make_entries(&["A", "B"]), "e".into());
        app.selected = 1;
        app.move_selection_down();
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn move_down_increments_normally() {
        let mut app = App::new(make_entries(&["A", "B"]), "e".into());
        app.selected = 0;
        app.move_selection_down();
        assert_eq!(app.selected, 1);
    }

    #[test]
    fn move_down_noop_on_empty_list() {
        let mut app = App::new(vec![], "e".into());
        app.move_selection_down(); // must not panic or overflow
        assert_eq!(app.selected, 0);
    }

    // ── open_edit_popup ───────────────────────────────────────────────────────

    #[test]
    fn open_edit_popup_pre_fills_current_value() {
        let mut app = App::new(
            vec![GrubEntry {
                key: "GRUB_TIMEOUT".into(),
                value: "5".into(),
            }],
            "e".into(),
        );
        app.open_edit_popup();
        assert_eq!(app.mode, Mode::Editing);
        assert_eq!(app.edit_buf, "5");
    }

    #[test]
    fn open_edit_popup_noop_on_empty_list() {
        let mut app = App::new(vec![], "e".into());
        app.open_edit_popup();
        assert_eq!(app.mode, Mode::Browse); // must stay in Browse
    }

    // ── push_char / pop_char ──────────────────────────────────────────────────

    #[test]
    fn push_and_pop_edit_buf() {
        let mut app = App::new(make_entries(&["A"]), "e".into());
        app.push_char('1');
        app.push_char('0');
        assert_eq!(app.edit_buf, "10");
        app.pop_char();
        assert_eq!(app.edit_buf, "1");
    }

    #[test]
    fn pop_char_on_empty_buf_does_not_panic() {
        let mut app = App::new(make_entries(&["A"]), "e".into());
        app.pop_char(); // must not panic
    }

    // ── cancel_edit ───────────────────────────────────────────────────────────

    #[test]
    fn cancel_edit_returns_to_browse_and_clears_buf() {
        let mut app = App::new(make_entries(&["A"]), "e".into());
        app.open_edit_popup();
        app.push_char('X');
        app.cancel_edit();
        assert_eq!(app.mode, Mode::Browse);
        assert!(app.edit_buf.is_empty());
    }

    // ── error popup ───────────────────────────────────────────────────────────

    #[test]
    fn show_and_dismiss_error_popup() {
        let mut app = App::new(make_entries(&["A"]), "e".into());
        app.show_error("PolkitDenied: nope".into());
        assert_eq!(app.mode, Mode::ErrorPopup);
        assert_eq!(app.error_msg.as_deref(), Some("PolkitDenied: nope"));
        app.dismiss_error();
        assert_eq!(app.mode, Mode::Browse);
        assert!(app.error_msg.is_none());
    }

    // ── current_entry ─────────────────────────────────────────────────────────

    #[test]
    fn current_entry_returns_none_on_empty() {
        let app = App::new(vec![], "e".into());
        assert!(app.current_entry().is_none());
    }

    #[test]
    fn current_entry_returns_selected_row() {
        let mut app = App::new(make_entries(&["A", "B"]), "e".into());
        app.selected = 1;
        assert_eq!(app.current_entry().unwrap().key, "B");
    }
}
