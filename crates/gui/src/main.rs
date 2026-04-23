slint::include_modules!();

use bootcontrol_gui::view_model::ViewModel;
use tokio::sync::mpsc;

enum UiMessage {
    FetchEntries,
    SaveEntry(String, String),
    RebuildGrub,
    BackupNvram,
    EnrollMok,
    GenerateParanoia,
    MergeParanoia,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ui = AppWindow::new()?;

    let (tx, mut rx) = mpsc::channel::<UiMessage>(32);
    let tx_clone = tx.clone();

    // Bind Slint callbacks
    ui.on_fetch_entries({
        let tx = tx.clone();
        move || {
            let _ = tx.blocking_send(UiMessage::FetchEntries);
        }
    });

    ui.on_save_entry({
        let tx = tx.clone();
        move |key, value| {
            let _ = tx.blocking_send(UiMessage::SaveEntry(key.to_string(), value.to_string()));
        }
    });

    ui.on_rebuild_grub({
        let tx = tx.clone();
        move || {
            let _ = tx.blocking_send(UiMessage::RebuildGrub);
        }
    });

    ui.on_backup_nvram({
        let tx = tx.clone();
        move || {
            let _ = tx.blocking_send(UiMessage::BackupNvram);
        }
    });

    ui.on_enroll_mok({
        let tx = tx.clone();
        move || {
            let _ = tx.blocking_send(UiMessage::EnrollMok);
        }
    });

    ui.on_generate_paranoia({
        let tx = tx.clone();
        move || {
            let _ = tx.blocking_send(UiMessage::GenerateParanoia);
        }
    });

    ui.on_merge_paranoia({
        let tx = tx.clone();
        move || {
            let _ = tx.blocking_send(UiMessage::MergeParanoia);
        }
    });

    ui.on_dismiss_toast({
        let ui_handle = ui.as_weak();
        move || {
            let _ = ui_handle.upgrade_in_event_loop(|ui| {
                ui.set_show_toast(false);
            });
        }
    });

    // Initialize Backend (D-Bus on Linux, Mock on others or if BOOTCONTROL_DEMO=1)
    let backend = bootcontrol_client::resolve_backend().await;
    let mut view_model = ViewModel::new(backend);

    // Initial fetch
    let _ = tx_clone.send(UiMessage::FetchEntries).await;

    // Spawn async backend task
    let ui_handle_async = ui.as_weak();
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            match msg {
                UiMessage::FetchEntries => {
                    match view_model.load().await {
                        Ok(_) => {
                            let mut entries: Vec<GrubEntry> = view_model
                                .entries
                                .iter()
                                .map(|(k, v)| GrubEntry {
                                    key: k.as_str().into(),
                                    value: v.as_str().into(),
                                    original_value: v.as_str().into(),
                                    is_modified: false,
                                })
                                .collect();
                            entries.sort_by(|a, b| a.key.cmp(&b.key));

                            let backend_name = view_model.active_backend.clone();
                            let _ = ui_handle_async.upgrade_in_event_loop(move |ui| {
                                let model = std::rc::Rc::new(slint::VecModel::from(entries));
                                ui.set_entries(model.into());
                                ui.set_active_backend(backend_name.into());
                            });
                        }
                        Err(e) => {
                            let err_msg = format!("Failed to read GRUB config: {:?}", e);
                            show_toast(&ui_handle_async, err_msg, "error");
                        }
                    }
                }
                UiMessage::SaveEntry(key, value) => {
                    set_loading(&ui_handle_async, true, format!("Saving '{}'...", key));
                    match view_model.commit_edit(&key, &value).await {
                        Ok(_) => {
                            set_loading(&ui_handle_async, false, String::new());
                            let _ = tx_clone.send(UiMessage::FetchEntries).await;
                            show_toast(
                                &ui_handle_async,
                                format!("Saved '{}' successfully", key),
                                "success",
                            );
                        }
                        Err(e) => {
                            set_loading(&ui_handle_async, false, String::new());
                            let err_string = e.to_string();
                            drop(e);
                            let dmsg = if err_string.contains("AccessDenied") {
                                "Access Denied. You need to authenticate via Polkit.".to_string()
                            } else {
                                format!("Failed to save: {}", err_string)
                            };
                            show_toast(&ui_handle_async, dmsg, "error");
                            let _ = tx_clone.send(UiMessage::FetchEntries).await;
                        }
                    }
                }
                UiMessage::RebuildGrub => {
                    set_loading(&ui_handle_async, true, "Rebuilding GRUB config...".to_string());
                    match view_model.rebuild_grub().await {
                        Ok(_) => {
                            set_loading(&ui_handle_async, false, String::new());
                            show_toast(&ui_handle_async, "GRUB config rebuilt successfully".to_string(), "success");
                        }
                        Err(e) => {
                            set_loading(&ui_handle_async, false, String::new());
                            show_toast(&ui_handle_async, format!("Rebuild failed: {}", bootcontrol_client::dbus_error_message(&e)), "error");
                        }
                    }
                }
                UiMessage::BackupNvram => {
                    set_loading(&ui_handle_async, true, "Backing up EFI variables...".to_string());
                    match view_model.backup_nvram().await {
                        Ok(json_paths) => {
                            set_loading(&ui_handle_async, false, String::new());
                            let count = json_paths.matches(".efi").count()
                                + json_paths.matches(".auth").count()
                                + json_paths.matches(".bin").count();
                            let msg = if count > 0 {
                                format!("Backup complete: {} file(s) saved to /var/lib/bootcontrol/certs/", count)
                            } else {
                                "Backup complete. Files saved to /var/lib/bootcontrol/certs/".to_string()
                            };
                            show_toast(&ui_handle_async, msg, "success");
                        }
                        Err(e) => {
                            set_loading(&ui_handle_async, false, String::new());
                            show_toast(&ui_handle_async, format!("Backup failed: {}", bootcontrol_client::dbus_error_message(&e)), "error");
                        }
                    }
                }
                UiMessage::EnrollMok => {
                    set_loading(&ui_handle_async, true, "Signing UKI and enrolling MOK key...".to_string());
                    match view_model.enroll_mok().await {
                        Ok(_) => {
                            set_loading(&ui_handle_async, false, String::new());
                            show_toast(&ui_handle_async, "MOK enrolled. Reboot to complete enrollment.".to_string(), "success");
                        }
                        Err(e) => {
                            set_loading(&ui_handle_async, false, String::new());
                            show_toast(&ui_handle_async, format!("MOK enrollment failed: {}", bootcontrol_client::dbus_error_message(&e)), "error");
                        }
                    }
                }
                UiMessage::GenerateParanoia => {
                    set_loading(&ui_handle_async, true, "Generating custom PK/KEK/db keys...".to_string());
                    match view_model.generate_paranoia().await {
                        Ok(_json_paths) => {
                            set_loading(&ui_handle_async, false, String::new());
                            show_toast(&ui_handle_async, "Keys generated at /var/lib/bootcontrol/paranoia-keys/".to_string(), "success");
                        }
                        Err(e) => {
                            set_loading(&ui_handle_async, false, String::new());
                            show_toast(&ui_handle_async, format!("Key generation failed: {}", bootcontrol_client::dbus_error_message(&e)), "error");
                        }
                    }
                }
                UiMessage::MergeParanoia => {
                    set_loading(&ui_handle_async, true, "Merging Microsoft signatures...".to_string());
                    match view_model.merge_paranoia().await {
                        Ok(auth_path) => {
                            set_loading(&ui_handle_async, false, String::new());
                            show_toast(&ui_handle_async, format!("Microsoft db merged: {}", auth_path), "success");
                        }
                        Err(e) => {
                            set_loading(&ui_handle_async, false, String::new());
                            show_toast(&ui_handle_async, format!("Merge failed: {}", bootcontrol_client::dbus_error_message(&e)), "error");
                        }
                    }
                }
            }
        }
    });

    ui.run()?;
    Ok(())
}

fn show_toast(ui: &slint::Weak<AppWindow>, message: String, toast_type: &str) {
    let t_type = toast_type.to_string();
    let _ = ui.upgrade_in_event_loop(move |u| {
        u.set_toast_message(message.into());
        u.set_toast_type(t_type.into());
        u.set_show_toast(true);
    });
}

fn set_loading(ui: &slint::Weak<AppWindow>, active: bool, message: String) {
    let _ = ui.upgrade_in_event_loop(move |u| {
        u.set_show_loading(active);
        u.set_loading_message(message.into());
    });
}
