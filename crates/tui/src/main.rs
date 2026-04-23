//! `bootcontrol-tui` — keyboard-driven Terminal User Interface for BootControl.
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────┐
//! │  main()                                                  │
//! │  ├─ resolve_backend (D-Bus or Mock)                      │
//! │  ├─ detect active backend (grub / systemd-boot / uki)    │
//! │  ├─ load entries → App::new_with_backend(entries, etag)  │
//! │  └─ event loop                                           │
//! │       ├─ Browse: ↑↓ navigate, Enter edit/set-default,   │
//! │       │          a=add-param (UKI), d=delete-param (UKI) │
//! │       │          r=reload, q=quit                        │
//! │       ├─ Editing: printable appended, Enter saves        │
//! │       └─ ErrorPopup: any key dismisses                   │
//! └──────────────────────────────────────────────────────────┘
//! ```

#![deny(warnings)]

pub mod app;
pub mod events;
pub mod popup;
pub mod ui;

use std::io;
use std::sync::Arc;

use app::{App, GrubEntry, Mode};
use bootcontrol_client::{BootBackend, resolve_backend, dbus_error_message};
use crossterm::{
    event::{EventStream, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use events::{AppEvent, is_quit_key, next_event};
use ratatui::{Terminal, backend::CrosstermBackend};
use tokio::time::{Duration, Instant};
use tracing::{info, warn};

// ──────────────────────────────────────────────────────────────────────────────
// Entry point
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let backend = resolve_backend().await;

    info!("loading initial boot configuration");
    let app = match load_app(backend.as_ref()).await {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: failed to read config from backend: {e}");
            return Err(e.into());
        }
    };

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend_tui = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend_tui)?;
    terminal.clear()?;

    let result = run_event_loop(&mut terminal, app, backend.clone()).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result?;
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Event loop
// ──────────────────────────────────────────────────────────────────────────────

async fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    mut app: App,
    backend: Arc<dyn BootBackend>,
) -> io::Result<()> {
    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();
    let mut stream = EventStream::new();

    loop {
        terminal.draw(|frame| ui::render(frame, &app))?;

        if app.should_quit {
            break;
        }

        let Some(event) = next_event(&mut stream, tick_rate, &mut last_tick).await else {
            break;
        };

        match event {
            AppEvent::Key(key) => handle_key_event(key, &mut app, backend.as_ref()).await,
            AppEvent::Resize | AppEvent::Tick => {}
        }
    }

    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Key event dispatch
// ──────────────────────────────────────────────────────────────────────────────

async fn handle_key_event(key: KeyEvent, app: &mut App, backend: &dyn BootBackend) {
    match app.mode {
        Mode::Browse => handle_browse_key(key, app, backend).await,
        Mode::Editing => handle_editing_key(key, app, backend).await,
        Mode::ErrorPopup => handle_error_key(key, app),
    }
}

// ── Browse mode ───────────────────────────────────────────────────────────────

async fn handle_browse_key(key: KeyEvent, app: &mut App, backend: &dyn BootBackend) {
    if is_quit_key(&key) || matches!(key.code, KeyCode::Esc) {
        app.should_quit = true;
        return;
    }

    let is_uki = app.backend_name.contains("uki");
    let is_sdb = app.backend_name.contains("systemd-boot");

    match key.code {
        KeyCode::Up | KeyCode::Char('k') => app.move_selection_up(),
        KeyCode::Down | KeyCode::Char('j') => app.move_selection_down(),
        KeyCode::Enter => {
            if is_sdb {
                // systemd-boot: Enter sets selected entry as default
                set_default_entry(app, backend).await;
            } else if is_uki {
                // UKI: Enter edits the parameter text
                app.open_edit_popup();
            } else {
                // GRUB: Enter opens value editor
                app.open_edit_popup();
            }
        }
        KeyCode::Char('a') | KeyCode::Char('A') if is_uki => {
            // UKI: 'a' opens empty editor to add a new parameter
            app.edit_buf.clear();
            app.mode = Mode::Editing;
            app.status_msg = "Type new kernel parameter, Enter to add.".into();
        }
        KeyCode::Char('d') | KeyCode::Char('D') if is_uki => {
            // UKI: 'd' removes the selected parameter
            delete_kernel_param(app, backend).await;
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            reload_config(app, backend).await;
        }
        _ => {}
    }
}

// ── Editing mode ──────────────────────────────────────────────────────────────

async fn handle_editing_key(key: KeyEvent, app: &mut App, backend: &dyn BootBackend) {
    match key {
        KeyEvent { code: KeyCode::Enter, .. } => {
            commit_edit(app, backend).await;
        }
        KeyEvent { code: KeyCode::Esc, .. } => {
            app.cancel_edit();
            app.status_msg = "Edit cancelled.".into();
        }
        KeyEvent { code: KeyCode::Backspace, .. } => {
            app.pop_char();
        }
        KeyEvent {
            code: KeyCode::Char(c),
            modifiers,
            ..
        } if modifiers == KeyModifiers::NONE || modifiers == KeyModifiers::SHIFT => {
            app.push_char(c);
        }
        _ => {}
    }
}

// ── Error popup mode ──────────────────────────────────────────────────────────

fn handle_error_key(key: KeyEvent, app: &mut App) {
    match key.code {
        KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => app.dismiss_error(),
        _ => {}
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Backend helpers — multi-backend aware
// ──────────────────────────────────────────────────────────────────────────────

/// Load the initial app state from the backend, branching on active backend type.
async fn load_app(backend: &dyn BootBackend) -> Result<App, zbus::Error> {
    let backend_name = backend.get_active_backend().await.unwrap_or_else(|_| "grub".to_string());

    if backend_name.contains("systemd-boot") {
        load_systemd_boot(backend, backend_name).await
    } else if backend_name.contains("uki") {
        load_uki(backend, backend_name).await
    } else {
        load_grub(backend, backend_name).await
    }
}

async fn load_grub(backend: &dyn BootBackend, backend_name: String) -> Result<App, zbus::Error> {
    let (config, etag) = backend.read_config().await?;
    let mut entries: Vec<GrubEntry> = config
        .into_iter()
        .map(|(key, value)| GrubEntry { key, value })
        .collect();
    entries.sort_by(|a, b| a.key.cmp(&b.key));
    Ok(App::new_with_backend(entries, etag, backend_name))
}

async fn load_systemd_boot(backend: &dyn BootBackend, backend_name: String) -> Result<App, zbus::Error> {
    let entries_dto = backend.list_loader_entries().await?;
    let etag = backend.get_loader_conf_etag().await.unwrap_or_default();
    let entries: Vec<GrubEntry> = entries_dto
        .iter()
        .map(|e| GrubEntry {
            key: e.id.clone(),
            value: e.title.clone().unwrap_or_else(|| "(no title)".to_string()),
        })
        .collect();
    Ok(App::new_with_backend(entries, etag, backend_name))
}

async fn load_uki(backend: &dyn BootBackend, backend_name: String) -> Result<App, zbus::Error> {
    let (params, etag) = backend.read_kernel_cmdline().await?;
    let entries: Vec<GrubEntry> = params
        .into_iter()
        .map(|p| GrubEntry { key: p, value: String::new() })
        .collect();
    Ok(App::new_with_backend(entries, etag, backend_name))
}

/// Reload the config and update `app` in place.
async fn reload_config(app: &mut App, backend: &dyn BootBackend) {
    let backend_name = app.backend_name.clone();

    if backend_name.contains("systemd-boot") {
        match backend.list_loader_entries().await {
            Ok(entries_dto) => {
                let etag = backend.get_loader_conf_etag().await.unwrap_or_default();
                let entries: Vec<GrubEntry> = entries_dto
                    .iter()
                    .map(|e| GrubEntry {
                        key: e.id.clone(),
                        value: e.title.clone().unwrap_or_else(|| "(no title)".to_string()),
                    })
                    .collect();
                app.apply_grub_entries(entries, etag);
                app.status_msg = "Loader entries reloaded.".into();
            }
            Err(e) => {
                warn!(error = %e, "reload failed");
                app.show_error(dbus_error_message(&e));
            }
        }
    } else if backend_name.contains("uki") {
        match backend.read_kernel_cmdline().await {
            Ok((params, etag)) => {
                let entries: Vec<GrubEntry> = params
                    .into_iter()
                    .map(|p| GrubEntry { key: p, value: String::new() })
                    .collect();
                app.apply_grub_entries(entries, etag);
                app.status_msg = "Kernel cmdline reloaded.".into();
            }
            Err(e) => {
                warn!(error = %e, "reload failed");
                app.show_error(dbus_error_message(&e));
            }
        }
    } else {
        match backend.read_config().await {
            Ok((config, etag)) => {
                let mut entries: Vec<GrubEntry> = config
                    .into_iter()
                    .map(|(key, value)| GrubEntry { key, value })
                    .collect();
                entries.sort_by(|a, b| a.key.cmp(&b.key));
                app.apply_grub_entries(entries, etag);
                app.status_msg = "Configuration reloaded.".into();
            }
            Err(e) => {
                warn!(error = %e, "reload failed");
                app.show_error(dbus_error_message(&e));
            }
        }
    }
}

/// Set the selected systemd-boot entry as default.
async fn set_default_entry(app: &mut App, backend: &dyn BootBackend) {
    let (id, etag) = match app.current_entry() {
        Some(entry) => (entry.key.clone(), app.etag.clone()),
        None => return,
    };

    match backend.set_loader_default(&id, &etag).await {
        Ok(()) => {
            app.status_msg = format!("Default entry set to: {id}");
            reload_config(app, backend).await;
        }
        Err(e) => {
            warn!(id = %id, error = %e, "SetLoaderDefault failed");
            app.show_error(dbus_error_message(&e));
        }
    }
}

/// Delete the selected UKI kernel parameter.
async fn delete_kernel_param(app: &mut App, backend: &dyn BootBackend) {
    let (param, etag) = match app.current_entry() {
        Some(entry) => (entry.key.clone(), app.etag.clone()),
        None => return,
    };

    match backend.remove_kernel_param(&param, &etag).await {
        Ok(()) => {
            app.status_msg = format!("Removed parameter: {param}");
            reload_config(app, backend).await;
        }
        Err(e) => {
            warn!(param = %param, error = %e, "RemoveKernelParam failed");
            app.show_error(dbus_error_message(&e));
        }
    }
}

/// Commit the current edit, branching on backend type.
async fn commit_edit(app: &mut App, backend: &dyn BootBackend) {
    let backend_name = app.backend_name.clone();

    if backend_name.contains("uki") {
        commit_uki_edit(app, backend).await;
    } else {
        commit_grub_edit(app, backend).await;
    }
}

async fn commit_grub_edit(app: &mut App, backend: &dyn BootBackend) {
    let (key, value, etag) = match app.current_entry() {
        Some(entry) => (entry.key.clone(), app.edit_buf.clone(), app.etag.clone()),
        None => {
            app.cancel_edit();
            return;
        }
    };

    match backend.set_value(&key, &value, &etag).await {
        Ok(()) => {
            app.cancel_edit();
            app.status_msg = format!("✓  {key} updated successfully.");
            reload_config(app, backend).await;
        }
        Err(e) => {
            warn!(key = %key, error = %e, "SetGrubValue failed");
            app.cancel_edit();
            app.show_error(dbus_error_message(&e));
        }
    }
}

async fn commit_uki_edit(app: &mut App, backend: &dyn BootBackend) {
    let new_param = app.edit_buf.trim().to_string();
    if new_param.is_empty() {
        app.cancel_edit();
        return;
    }

    // If there's a selected entry, it's an edit (remove old, add new).
    // If the edit_buf was pre-populated, remove the old param first.
    let old_param = app.current_entry().map(|e| e.key.clone());
    let etag = app.etag.clone();

    // Add the new parameter.
    match backend.add_kernel_param(&new_param, &etag).await {
        Ok(()) => {
            // If we replaced an existing param, also remove the old one.
            if let Some(old) = old_param {
                if old != new_param {
                    // Reload ETag after add, then remove old.
                    if let Ok((_, new_etag)) = backend.read_kernel_cmdline().await {
                        let _ = backend.remove_kernel_param(&old, &new_etag).await;
                    }
                }
            }
            app.cancel_edit();
            app.status_msg = format!("✓  Added: {new_param}");
            reload_config(app, backend).await;
        }
        Err(e) => {
            warn!(param = %new_param, error = %e, "AddKernelParam failed");
            app.cancel_edit();
            app.show_error(dbus_error_message(&e));
        }
    }
}
