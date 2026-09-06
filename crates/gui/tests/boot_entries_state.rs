use bootcontrol_client::GrubMenuEntryDto;
use bootcontrol_gui::boot_entries::{BootEntriesModel, BootEntriesStatus};

fn entries() -> Vec<GrubMenuEntryDto> {
    vec![
        GrubMenuEntryDto {
            title: "Linux".into(),
            id: Some("linux".into()),
            path: "0".into(),
            depth: 0,
            is_submenu: false,
        },
        GrubMenuEntryDto {
            title: "Advanced options".into(),
            id: None,
            path: "1".into(),
            depth: 0,
            is_submenu: true,
        },
        GrubMenuEntryDto {
            title: "Linux recovery".into(),
            id: Some("recovery".into()),
            path: "1>0".into(),
            depth: 1,
            is_submenu: false,
        },
    ]
}

#[test]
fn load_states_cover_loading_empty_error_and_success() {
    let mut model = BootEntriesModel::default();
    model.begin_load();
    assert_eq!(model.status(), BootEntriesStatus::Loading);

    model.finish_load(Vec::new(), "empty-etag".into());
    assert_eq!(model.status(), BootEntriesStatus::Empty);
    assert!(model.selected().is_none());

    model.finish_load(entries(), "menu-etag".into());
    assert_eq!(model.status(), BootEntriesStatus::Ready);
    assert_eq!(model.etag(), "menu-etag");
    assert_eq!(
        model.selected().map(|entry| entry.title.as_str()),
        Some("Linux")
    );

    model.fail_load("grub.cfg cannot be read".into());
    assert_eq!(model.status(), BootEntriesStatus::Error);
    assert_eq!(model.error(), "grub.cfg cannot be read");
    assert!(model.entries().is_empty());
    assert!(model.etag().is_empty());
    assert!(model.selected().is_none());
}

#[test]
fn selection_supports_rows_submenus_and_keyboard_delta() {
    let mut model = BootEntriesModel::default();
    model.finish_load(entries(), "menu-etag".into());

    model.select(1);
    assert!(model.selected().unwrap().is_submenu);
    model.move_selection(1);
    assert_eq!(model.selected().unwrap().path, "1>0");
    model.move_selection(1);
    assert_eq!(model.selected().unwrap().path, "1>0");
    model.move_selection(-2);
    assert_eq!(model.selected().unwrap().path, "0");
}

#[test]
fn reload_replaces_stale_menu_and_resets_selection() {
    let mut model = BootEntriesModel::default();
    model.finish_load(entries(), "old-etag".into());
    model.select(2);

    model.begin_load();
    assert!(model.entries().is_empty());
    assert!(model.selected().is_none());
    model.finish_load(
        vec![GrubMenuEntryDto {
            title: "New Linux".into(),
            id: Some("new".into()),
            path: "0".into(),
            depth: 0,
            is_submenu: false,
        }],
        "new-etag".into(),
    );

    assert_eq!(model.selected().unwrap().title, "New Linux");
    assert_eq!(model.etag(), "new-etag");
}

#[test]
fn default_change_is_staged_with_both_versions_until_discarded() {
    let mut model = BootEntriesModel::default();
    model.finish_load(entries(), "menu-etag".into());
    model.set_config_state("0".into(), "config-etag".into());
    model.select(2);

    assert!(model.stage_selected_as_default());
    assert_eq!(model.pending_count(), 1);
    assert_eq!(model.effective_default(), "1>0");
    let request = model.pending_default_request().expect("staged request");
    assert_eq!(request.previous_path, "0");
    assert_eq!(request.selected_path, "1>0");
    assert_eq!(request.menu_etag, "menu-etag");
    assert_eq!(request.config_etag, "config-etag");

    model.discard_default_change();
    assert_eq!(model.pending_count(), 0);
    assert_eq!(model.effective_default(), "0");
}

#[test]
fn submenu_current_default_and_missing_versions_cannot_be_staged() {
    let mut model = BootEntriesModel::default();
    model.finish_load(entries(), "menu-etag".into());
    model.set_config_state("0".into(), "config-etag".into());

    model.select(1);
    assert!(!model.stage_selected_as_default());
    model.select(0);
    assert!(!model.stage_selected_as_default());

    model.set_config_state("2".into(), String::new());
    model.select(2);
    assert!(!model.stage_selected_as_default());
    assert_eq!(model.pending_count(), 0);
}
