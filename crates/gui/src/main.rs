slint::include_modules!();

use bootcontrol_gui::view_model::ViewModel;
use tokio::sync::mpsc;

enum UiMessage {
    FetchEntries,
    SaveEntry(String, String),
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

    ui.on_dismiss_toast({
        let ui_handle = ui.as_weak();
        move || {
            let _ = ui_handle.upgrade_in_event_loop(|ui| {
                ui.set_show_toast(false);
            });
        }
    });

    // Initialize Backend (D-Bus on Linux, Mock on others or if DEMO=1)
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
                            // Map to Slint Model
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
                    match view_model.commit_edit(&key, &value).await {
                        Ok(_) => {
                            // Re-fetch everything to ensure it's in sync and update ETag
                            let _ = tx_clone.send(UiMessage::FetchEntries).await;
                            show_toast(
                                &ui_handle_async,
                                format!("Saved '{}' successfully", key),
                                "success",
                            );
                        }
                        Err(e) => {
                            let err_string = e.to_string();
                            drop(e); // Ensure non-Send Error is dropped before the .await
                            let dmsg = if err_string.contains("AccessDenied") {
                                "Access Denied. You need to authenticate via Polkit.".to_string()
                            } else {
                                format!("Failed to save: {}", err_string)
                            };
                            show_toast(&ui_handle_async, dmsg, "error");

                            // Re-fetch to revert the UI state to what is physically on disk
                            let _ = tx_clone.send(UiMessage::FetchEntries).await;
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
