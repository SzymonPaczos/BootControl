#![deny(warnings)]

use bootcontrol_client::GrubSettingsDto;
use bootcontrol_gui::grub_settings::{GrubSettingsModel, GrubSettingsStatus};

fn settings() -> GrubSettingsDto {
    GrubSettingsDto {
        timeout_seconds: 5,
        timeout_style: "menu".into(),
        detect_other_os: true,
        generate_recovery_entries: true,
    }
}

#[test]
fn load_states_invalidate_stale_settings_and_versions() {
    let mut model = GrubSettingsModel::default();
    model.begin_load();
    assert_eq!(model.status(), GrubSettingsStatus::Loading);
    model.finish_load(settings(), "config-etag".into());
    assert_eq!(model.status(), GrubSettingsStatus::Ready);
    assert_eq!(model.etag(), "config-etag");

    model.fail_load("typed read failed".into());
    assert_eq!(model.status(), GrubSettingsStatus::Error);
    assert_eq!(model.error(), "typed read failed");
    assert!(model.settings().is_none());
    assert!(model.etag().is_empty());
    assert_eq!(model.pending_count(), 0);
}

#[test]
fn edits_are_staged_locally_and_discard_restores_loaded_values() {
    let mut model = GrubSettingsModel::default();
    model.finish_load(settings(), "config-etag".into());

    assert!(model.stage_timeout("12").unwrap());
    assert!(model.stage_timeout_style("countdown").unwrap());
    assert!(model.stage_detect_other_os(false));
    assert!(model.stage_recovery_entries(false));
    assert_eq!(model.pending_count(), 4);
    assert_eq!(model.settings().unwrap().timeout_seconds, 12);
    assert_eq!(model.settings().unwrap().timeout_style, "countdown");

    let request = model.pending_request().expect("versioned request");
    assert_eq!(request.etag, "config-etag");
    assert_eq!(request.changes.len(), 4);
    model.discard();
    assert_eq!(model.pending_count(), 0);
    assert_eq!(model.settings().unwrap(), &settings());
}

#[test]
fn invalid_values_are_rejected_without_creating_pending_changes() {
    let mut model = GrubSettingsModel::default();
    model.finish_load(settings(), "config-etag".into());

    for invalid in ["", "-1", "1000001", "five"] {
        assert!(model.stage_timeout(invalid).is_err());
    }
    assert!(model.stage_timeout_style("fade").is_err());
    assert_eq!(model.pending_count(), 0);
    assert_eq!(model.settings().unwrap(), &settings());
}
