//! Confirmation state for destructive GUI operations.

use crate::boot_entries::GrubDefaultRequest;

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
    grub_default: Option<GrubDefaultRequest>,
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
        self.grub_default = None;
        preview
    }

    /// Prepare a concrete `GRUB_DEFAULT` diff and arm its versioned request.
    ///
    /// # Arguments
    ///
    /// * `mode` - Whether the preview is live or explicitly simulated.
    /// * `request` - Staged paths and both optimistic-lock versions.
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrol_gui::boot_entries::GrubDefaultRequest;
    /// use bootcontrol_gui::confirmation::{ConfirmationMode, ConfirmationSession};
    /// let mut session = ConfirmationSession::default();
    /// let preview = session.prepare_grub_default(ConfirmationMode::Live, GrubDefaultRequest {
    ///     previous_path: "0".into(), selected_path: "1>0".into(),
    ///     menu_etag: "menu-etag".into(), config_etag: "config-etag".into(),
    /// });
    /// assert!(preview.can_confirm);
    /// ```
    pub fn prepare_grub_default(
        &mut self,
        mode: ConfirmationMode,
        request: GrubDefaultRequest,
    ) -> ConfirmationPreview {
        let preview = grub_default_preview(mode, &request);
        self.rebuild_armed = false;
        self.grub_default = preview.can_confirm.then_some(request);
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

    /// Consume the exact versioned default-selection request once.
    ///
    /// # Arguments
    ///
    /// This function takes no arguments.
    pub fn confirm_grub_default(&mut self) -> Option<GrubDefaultRequest> {
        self.grub_default.take()
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
        self.grub_default = None;
    }
}

fn grub_default_preview(
    mode: ConfirmationMode,
    request: &GrubDefaultRequest,
) -> ConfirmationPreview {
    let menu_ready = !request.menu_etag.is_empty();
    let config_ready = !request.config_etag.is_empty();
    let change_ready = !request.previous_path.is_empty()
        && !request.selected_path.is_empty()
        && request.previous_path != request.selected_path;
    let simulated = mode == ConfirmationMode::Demo;
    let file_path = if simulated {
        "DEMO /etc/default/grub"
    } else {
        "/etc/default/grub"
    };
    let detail_suffix = if simulated { " (simulated)" } else { "" };

    ConfirmationPreview {
        verb_label: "Set default entry".to_string(),
        target: if simulated {
            format!(
                "Demo Mode simulation: set GRUB_DEFAULT from {} to {}.",
                request.previous_path, request.selected_path
            )
        } else {
            format!(
                "Change GRUB_DEFAULT in /etc/default/grub from {} to {}; bootcontrold will verify both source versions before writing.",
                request.previous_path, request.selected_path
            )
        },
        required_text: String::new(),
        command_cli: String::new(),
        diff: vec![
            ConfirmationDiffLine {
                side: "context".to_string(),
                text: String::new(),
                file_path: file_path.to_string(),
            },
            ConfirmationDiffLine {
                side: "remove".to_string(),
                text: format!("GRUB_DEFAULT={}", request.previous_path),
                file_path: String::new(),
            },
            ConfirmationDiffLine {
                side: "add".to_string(),
                text: format!("GRUB_DEFAULT={}", request.selected_path),
                file_path: String::new(),
            },
        ],
        preflight: vec![
            ConfirmationCheck {
                name: "Generated menu version".to_string(),
                passed: menu_ready,
                detail: if menu_ready {
                    format!("{}{}", request.menu_etag, detail_suffix)
                } else {
                    "unavailable; refresh the GRUB menu".to_string()
                },
            },
            ConfirmationCheck {
                name: "Source configuration version".to_string(),
                passed: config_ready,
                detail: if config_ready {
                    format!("{}{}", request.config_etag, detail_suffix)
                } else {
                    "unavailable; refresh the source configuration".to_string()
                },
            },
            ConfirmationCheck {
                name: "Staged selection".to_string(),
                passed: change_ready,
                detail: if change_ready {
                    format!(
                        "{} → {}{}",
                        request.previous_path, request.selected_path, detail_suffix
                    )
                } else {
                    "no distinct bootable entry is staged".to_string()
                },
            },
        ],
        can_confirm: menu_ready && config_ready && change_ready,
        snapshot_id: String::new(),
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
