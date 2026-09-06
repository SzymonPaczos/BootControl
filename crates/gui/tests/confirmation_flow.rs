use bootcontrol_gui::confirmation::{ConfirmationMode, ConfirmationSession};

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
