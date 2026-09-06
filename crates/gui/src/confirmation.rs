//! Confirmation state for destructive GUI operations.

/// Source of the data displayed in a confirmation preview.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfirmationMode {
    /// Data loaded from the running daemon.
    Live,
    /// Explicitly simulated data used by Demo Mode.
    Demo,
}

/// One preflight fact shown by the confirmation sheet.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfirmationCheck {
    /// Short check name.
    pub name: String,
    /// Whether the fact permits the operation.
    pub passed: bool,
    /// Concrete evidence or failure reason.
    pub detail: String,
}

/// One line of a preview diff.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfirmationDiffLine {
    /// Diff side: `context`, `add`, or `remove`.
    pub side: String,
    /// Displayed line content.
    pub text: String,
    /// File heading for the line.
    pub file_path: String,
}

/// Complete data rendered by the rebuild confirmation sheet.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfirmationPreview {
    /// Human-readable operation label.
    pub verb_label: String,
    /// Concrete operation target and source version.
    pub target: String,
    /// Exact phrase required by the type-to-confirm gate.
    pub required_text: String,
    /// Equivalent command-line operation.
    pub command_cli: String,
    /// Preview diff, empty when the daemon cannot calculate one before execution.
    pub diff: Vec<ConfirmationDiffLine>,
    /// Facts used to decide whether confirmation is allowed.
    pub preflight: Vec<ConfirmationCheck>,
    /// Whether all required live data is present.
    pub can_confirm: bool,
    /// Known snapshot identifier, empty until a live daemon creates one.
    pub snapshot_id: String,
}

/// One-shot gate for a prepared rebuild confirmation.
#[derive(Debug, Default)]
pub struct ConfirmationSession {
    rebuild_armed: bool,
}

impl ConfirmationSession {
    /// Prepare a GRUB rebuild preview and arm it only when its evidence is complete.
    ///
    /// # Arguments
    ///
    /// * `mode` - Whether the preview uses live or explicitly simulated data.
    /// * `active_backend` - Backend name obtained from the current view-model load.
    /// * `etag` - Version of `/etc/default/grub` obtained by that same load.
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrol_gui::confirmation::{ConfirmationMode, ConfirmationSession};
    ///
    /// let mut session = ConfirmationSession::default();
    /// let preview = session.prepare_rebuild(ConfirmationMode::Live, "grub", "sha256:abc");
    /// assert!(preview.can_confirm);
    /// assert!(session.confirm_rebuild());
    /// ```
    pub fn prepare_rebuild(
        &mut self,
        mode: ConfirmationMode,
        active_backend: &str,
        etag: &str,
    ) -> ConfirmationPreview {
        let preview = match mode {
            ConfirmationMode::Live => live_rebuild_preview(active_backend, etag),
            ConfirmationMode::Demo => demo_rebuild_preview(),
        };
        self.rebuild_armed = preview.can_confirm;
        preview
    }

    /// Consume a successfully prepared GRUB rebuild confirmation.
    ///
    /// A prepared action can succeed exactly once. A blocked, cancelled, or already
    /// consumed action returns `false`.
    ///
    /// # Arguments
    ///
    /// This function takes no arguments.
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrol_gui::confirmation::{ConfirmationMode, ConfirmationSession};
    ///
    /// let mut session = ConfirmationSession::default();
    /// session.prepare_rebuild(ConfirmationMode::Live, "grub", "sha256:abc");
    /// assert!(session.confirm_rebuild());
    /// assert!(!session.confirm_rebuild());
    /// ```
    pub fn confirm_rebuild(&mut self) -> bool {
        std::mem::take(&mut self.rebuild_armed)
    }

    /// Cancel and disarm the currently prepared action.
    ///
    /// # Arguments
    ///
    /// This function takes no arguments.
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrol_gui::confirmation::{ConfirmationMode, ConfirmationSession};
    ///
    /// let mut session = ConfirmationSession::default();
    /// session.prepare_rebuild(ConfirmationMode::Live, "grub", "sha256:abc");
    /// session.cancel();
    /// assert!(!session.confirm_rebuild());
    /// ```
    pub fn cancel(&mut self) {
        self.rebuild_armed = false;
    }
}

fn live_rebuild_preview(active_backend: &str, etag: &str) -> ConfirmationPreview {
    let backend_ready = active_backend == "grub";
    let etag_ready = !etag.is_empty();
    let can_confirm = backend_ready && etag_ready;
    let etag_label = if etag_ready { etag } else { "unavailable" };

    ConfirmationPreview {
        verb_label: "Rewrite GRUB".to_string(),
        target: format!(
            "bootcontrold will regenerate /boot/grub/grub.cfg from the loaded \
             /etc/default/grub state (ETag {etag_label})."
        ),
        required_text: "rewrite GRUB".to_string(),
        command_cli: "bootcontrol rebuild".to_string(),
        diff: Vec::new(),
        preflight: vec![
            ConfirmationCheck {
                name: "Active backend".to_string(),
                passed: backend_ready,
                detail: if backend_ready {
                    "grub (reported by bootcontrold)".to_string()
                } else if active_backend.is_empty() {
                    "unavailable; refresh live state".to_string()
                } else {
                    format!("{active_backend}; GRUB rebuild is unavailable")
                },
            },
            ConfirmationCheck {
                name: "Source configuration version".to_string(),
                passed: etag_ready,
                detail: if etag_ready {
                    etag.to_string()
                } else {
                    "unavailable; refresh live state".to_string()
                },
            },
        ],
        can_confirm,
        snapshot_id: String::new(),
    }
}

fn demo_rebuild_preview() -> ConfirmationPreview {
    ConfirmationPreview {
        verb_label: "Rewrite GRUB".to_string(),
        target: "Demo Mode simulation: no boot files will be changed.".to_string(),
        required_text: "rewrite GRUB".to_string(),
        command_cli: "bootcontrol rebuild".to_string(),
        diff: vec![
            ConfirmationDiffLine {
                side: "remove".to_string(),
                text: "GRUB_TIMEOUT=10".to_string(),
                file_path: "DEMO /etc/default/grub".to_string(),
            },
            ConfirmationDiffLine {
                side: "add".to_string(),
                text: "GRUB_TIMEOUT=5".to_string(),
                file_path: "DEMO /etc/default/grub".to_string(),
            },
        ],
        preflight: vec![ConfirmationCheck {
            name: "Demo Mode".to_string(),
            passed: true,
            detail: "simulated check; no daemon or boot files are used".to_string(),
        }],
        can_confirm: true,
        snapshot_id: "demo-snapshot-preview".to_string(),
    }
}
