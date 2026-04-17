slint::include_modules!();

mod dbus;

use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

enum UiMessage {
    FetchEntries,
    SaveEntry(String, String),
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ui = AppWindow::new()?;
    let ui_handle = ui.as_weak();

    let (tx, mut rx) = mpsc::channel::<UiMessage>(32);
    let tx_clone = tx.clone();
    
    // Store ETag globally to be used for transactions.
    let etag = Arc::new(Mutex::new(String::new()));

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

    // Spawn async backend task
    let ui_handle_async = ui.as_weak();
    tokio::spawn(async move {
        let conn = match zbus::Connection::system().await {
            Ok(c) => c,
            Err(e) => {
                show_toast(&ui_handle_async, format!("Failed to connect to system bus: {}", e), "error");
                return;
            }
        };
        
        let manager = match dbus::ManagerProxy::new(&conn).await {
            Ok(m) => m,
            Err(e) => {
                show_toast(&ui_handle_async, format!("Failed to create D-Bus proxy: {}", e), "error");
                return;
            }
        };

        // Initial fetch
        let _ = tx_clone.send(UiMessage::FetchEntries).await;

        while let Some(msg) = rx.recv().await {
            match msg {
                UiMessage::FetchEntries => {
                    match manager.read_grub_config().await {
                        Ok((config, new_etag)) => {
                            *etag.lock().await = new_etag;
                            
                            // Map to Slint Model
                            let mut entries: Vec<GrubEntry> = config.into_iter().map(|(k, v)| GrubEntry {
                                key: k.into(),
                                value: v.clone().into(),
                                original_value: v.into(),
                                is_modified: false,
                            }).collect();
                            entries.sort_by(|a, b| a.key.cmp(&b.key));
                            
                            let _ = ui_handle_async.upgrade_in_event_loop(move |ui| {
                                let model = std::rc::Rc::new(slint::VecModel::from(entries));
                                ui.set_entries(model.into());
                            });
                        }
                        Err(e) => {
                            let err_msg = format!("Failed to read GRUB config: {}", e);
                            show_toast(&ui_handle_async, err_msg, "error");
                        }
                    }
                }
                UiMessage::SaveEntry(key, value) => {
                    let current_etag = etag.lock().await.clone();
                    match manager.set_grub_value(&key, &value, &current_etag).await {
                        Ok(_) => {
                            // Re-fetch everything to ensure it's in sync and update ETag
                            let _ = tx_clone.send(UiMessage::FetchEntries).await;
                            show_toast(&ui_handle_async, format!("Saved '{}' successfully", key), "success");
                        }
                        Err(e) => {
                            let err_string = e.to_string();
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
