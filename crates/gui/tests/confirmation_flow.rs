use bootcontrol_gui::boot_entries::GrubDefaultRequest;
use bootcontrol_gui::confirmation::{ConfirmationMode, ConfirmationSession};

fn default_request() -> GrubDefaultRequest {
    GrubDefaultRequest {
        previous_path: "0".into(),
        selected_path: "1>0".into(),
        menu_etag: "menu-etag".into(),
        config_etag: "config-etag".into(),
    }
}

#[test]
fn live_rebuild_uses_loaded_backend_and_etag_without_invented_artifacts() {
    let mut session = ConfirmationSession::default();

    let preview = session.prepare_rebuild(ConfirmationMode::Live, "grub", "sha256:abc123");

    assert!(preview.can_confirm);
    assert!(preview.target.contains("sha256:abc123"));
    assert_eq!(preview.command_cli, "bootcontrol rebuild");
    assert!(preview.diff.is_empty());
    assert!(preview.snapshot_id.is_empty());
    assert!(preview.preflight.iter().all(|check| check.passed));
}

#[test]
fn missing_live_state_blocks_rebuild_and_disarms_an_earlier_preview() {
    let mut session = ConfirmationSession::default();
    assert!(
        session
            .prepare_rebuild(ConfirmationMode::Live, "grub", "sha256:current")
            .can_confirm
    );

    let preview = session.prepare_rebuild(ConfirmationMode::Live, "", "");

    assert!(!preview.can_confirm);
    assert!(preview.preflight.iter().any(|check| !check.passed));
    assert!(!session.confirm_rebuild());
}

#[test]
fn non_grub_backend_blocks_grub_rebuild() {
    let mut session = ConfirmationSession::default();

    let preview = session.prepare_rebuild(ConfirmationMode::Live, "systemd-boot", "sha256:loader");

    assert!(!preview.can_confirm);
    assert!(preview
        .preflight
        .iter()
        .any(|check| check.name == "Active backend" && !check.passed));
}

#[test]
fn demo_preview_is_explicitly_simulated() {
    let mut session = ConfirmationSession::default();

    let preview = session.prepare_rebuild(ConfirmationMode::Demo, "grub (mock)", "mock-etag");

    assert!(preview.can_confirm);
    assert!(preview.target.contains("Demo Mode"));
    assert!(preview
        .preflight
        .iter()
        .all(|check| check.detail.contains("simulated")));
    assert!(preview
        .diff
        .iter()
        .all(|line| line.file_path.contains("DEMO")));
    assert!(preview.snapshot_id.contains("demo"));
}

#[test]
fn cancel_and_confirm_each_consume_the_pending_action() {
    let mut session = ConfirmationSession::default();
    let mut backend_calls = 0;
    session.prepare_rebuild(ConfirmationMode::Live, "grub", "sha256:current");
    session.cancel();
    if session.confirm_rebuild() {
        backend_calls += 1;
    }

    session.prepare_rebuild(ConfirmationMode::Live, "grub", "sha256:current");
    if session.confirm_rebuild() {
        backend_calls += 1;
    }
    if session.confirm_rebuild() {
        backend_calls += 1;
    }

    assert_eq!(backend_calls, 1);
}

#[test]
fn default_preview_contains_real_diff_and_consumes_exact_request_once() {
    let mut session = ConfirmationSession::default();
    let request = default_request();

    let preview = session.prepare_grub_default(ConfirmationMode::Live, request.clone());

    assert!(preview.can_confirm);
    assert_eq!(preview.verb_label, "Set default entry");
    assert!(preview
        .diff
        .iter()
        .any(|line| { line.side == "remove" && line.text == "GRUB_DEFAULT=0" }));
    assert!(preview
        .diff
        .iter()
        .any(|line| { line.side == "add" && line.text == "GRUB_DEFAULT=1>0" }));
    assert!(preview.preflight.iter().all(|check| check.passed));
    assert_eq!(session.confirm_grub_default(), Some(request));
    assert_eq!(session.confirm_grub_default(), None);
}

#[test]
fn missing_default_version_blocks_apply_and_cancel_discards_request() {
    let mut session = ConfirmationSession::default();
    let mut invalid = default_request();
    invalid.menu_etag.clear();

    let preview = session.prepare_grub_default(ConfirmationMode::Live, invalid);

    assert!(!preview.can_confirm);
    assert_eq!(session.confirm_grub_default(), None);

    session.prepare_grub_default(ConfirmationMode::Live, default_request());
    session.cancel();
    assert_eq!(session.confirm_grub_default(), None);
}
