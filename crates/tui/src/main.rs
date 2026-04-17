//! `bootcontrol-tui` — keyboard-driven Terminal User Interface for BootControl.
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────┐
//! │  main()                                                  │
//! │  ├─ connect to D-Bus system bus                          │
//! │  ├─ build ManagerProxy (org.bootcontrol.Manager)         │
//! │  ├─ ReadGrubConfig → App::new(entries, etag)             │
//! │  └─ event loop                                           │
//! │       tokio::select!                                     │
//! │         ├─ crossterm event → handle_key_event()          │
//! │         │     ├─ Browse: ↑↓ navigate, Enter edit, r reload, q quit │
//! │         │     ├─ Editing: printable appended, Enter saves via D-Bus │
//! │         │     └─ ErrorPopup: any key dismisses           │
//! │         └─ 250 ms tick → Tick (reserved for animations)  │
//! └──────────────────────────────────────────────────────────┘
//! ```
//!
//! All D-Bus calls are `await`-ed inside the event loop.  The terminal is
//! always restored on exit, even when the daemon is unreachable.

#![deny(warnings)]

pub mod app;
pub mod dbus;
pub mod events;
pub mod popup;
pub mod ui;

use std::io;

use app::{App, GrubEntry, Mode};
use crossterm::{
    event::{EventStream, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use dbus::ManagerProxy;
use events::{AppEvent, is_quit_key, next_event};
use ratatui::{Terminal, backend::CrosstermBackend};
use tokio::time::{Duration, Instant};
use tracing::{info, warn};

// ──────────────────────────────────────────────────────────────────────────────
// Entry point
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ── Structured logging ────────────────────────────────────────────────────
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // ── Connect to D-Bus ─────────────────────────────────────────────────────
    info!("connecting to D-Bus system bus");
    let connection = zbus::Connection::system().await.map_err(|e| {
        eprintln!("error: cannot connect to D-Bus: {e}");
        e
    })?;

    let proxy = ManagerProxy::new(&connection).await.map_err(|e| {
        eprintln!("error: cannot reach bootcontrold: {e}");
        e
    })?;

    // ── Initial config load ───────────────────────────────────────────────────
    info!("loading initial GRUB configuration");
    let app = match load_app(&proxy).await {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: failed to read GRUB config from daemon: {e}");
            return Err(e.into());
        }
    };

    // ── Terminal setup ────────────────────────────────────────────────────────
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // ── Run the event loop ────────────────────────────────────────────────────
    let result = run_event_loop(&mut terminal, app, &proxy).await;

    // ── Terminal teardown (always runs) ───────────────────────────────────────
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result?;
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Event loop
// ──────────────────────────────────────────────────────────────────────────────

/// Run the main TUI event loop until the user requests a quit.
///
/// # Errors
///
/// Returns any terminal I/O error from `terminal.draw`.
async fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    mut app: App,
    proxy: &ManagerProxy<'_>,
) -> io::Result<()> {
    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();
    let mut stream = EventStream::new();

    loop {
        // ── Render ────────────────────────────────────────────────────────────
        terminal.draw(|frame| ui::render(frame, &app))?;

        if app.should_quit {
            break;
        }

        // ── Poll for the next event ───────────────────────────────────────────
        let Some(event) = next_event(&mut stream, tick_rate, &mut last_tick).await else {
            // Stream closed — terminal gone, exit cleanly.
            break;
        };

        match event {
            AppEvent::Key(key) => handle_key_event(key, &mut app, proxy).await,
            AppEvent::Resize | AppEvent::Tick => {} // just redraw on next iteration
        }
    }

    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Key event dispatch
// ──────────────────────────────────────────────────────────────────────────────

/// Dispatch a keyboard event to the correct handler based on `app.mode`.
async fn handle_key_event(key: KeyEvent, app: &mut App, proxy: &ManagerProxy<'_>) {
    match app.mode {
        Mode::Browse       => handle_browse_key(key, app, proxy).await,
        Mode::Editing      => handle_editing_key(key, app, proxy).await,
        Mode::ErrorPopup   => handle_error_key(key, app),
    }
}

// ── Browse mode ───────────────────────────────────────────────────────────────

async fn handle_browse_key(key: KeyEvent, app: &mut App, proxy: &ManagerProxy<'_>) {
    if is_quit_key(&key) || matches!(key.code, KeyCode::Esc) {
        app.should_quit = true;
        return;
    }

    match key.code {
        KeyCode::Up | KeyCode::Char('k') => app.move_selection_up(),
        KeyCode::Down | KeyCode::Char('j') => app.move_selection_down(),
        KeyCode::Enter => app.open_edit_popup(),
        KeyCode::Char('r') | KeyCode::Char('R') => {
            reload_config(app, proxy).await;
        }
        _ => {}
    }
}

// ── Editing mode ──────────────────────────────────────────────────────────────

async fn handle_editing_key(key: KeyEvent, app: &mut App, proxy: &ManagerProxy<'_>) {
    match key {
        // Confirm edit → call daemon
        KeyEvent {
            code: KeyCode::Enter,
            ..
        } => {
            commit_edit(app, proxy).await;
        }

        // Cancel edit
        KeyEvent {
            code: KeyCode::Esc,
            ..
        } => {
            app.cancel_edit();
            app.status_msg = "Edit cancelled.".into();
        }

        // Backspace
        KeyEvent {
            code: KeyCode::Backspace,
            ..
        } => {
            app.pop_char();
        }

        // Printable characters (no modifiers or SHIFT only)
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
// D-Bus helpers
// ──────────────────────────────────────────────────────────────────────────────

/// Call `ReadGrubConfig` and build a fresh [`App`].
async fn load_app(proxy: &ManagerProxy<'_>) -> Result<App, zbus::Error> {
    let (config, etag) = proxy.read_grub_config().await?;
    let entries: Vec<GrubEntry> = config
        .into_iter()
        .map(|(key, value)| GrubEntry { key, value })
        .collect();
    Ok(App::new(entries, etag))
}

/// Reload the GRUB config and update `app` in place.
async fn reload_config(app: &mut App, proxy: &ManagerProxy<'_>) {
    info!("reloading GRUB configuration from daemon");
    match proxy.read_grub_config().await {
        Ok((config, etag)) => {
            let entries: Vec<GrubEntry> = config
                .into_iter()
                .map(|(key, value)| GrubEntry { key, value })
                .collect();
            app.apply_grub_entries(entries, etag);
            app.status_msg = "Configuration reloaded.".into();
        }
        Err(e) => {
            warn!(error = %e, "reload failed");
            app.show_error(dbus_error_message(&e));
        }
    }
}

/// Call `SetGrubValue` with the current edit buffer contents.
///
/// On success, reloads the config (to refresh the ETag) and returns to Browse.
/// On failure, shows the error popup.
async fn commit_edit(app: &mut App, proxy: &ManagerProxy<'_>) {
    let (key, value, etag) = match app.current_entry() {
        Some(entry) => (entry.key.clone(), app.edit_buf.clone(), app.etag.clone()),
        None => {
            app.cancel_edit();
            return;
        }
    };

    info!(key = %key, "committing edit via D-Bus");
    match proxy.set_grub_value(&key, &value, &etag).await {
        Ok(()) => {
            app.cancel_edit();
            app.status_msg = format!("✓  {key} updated successfully.");
            reload_config(app, proxy).await;
        }
        Err(e) => {
            warn!(key = %key, error = %e, "SetGrubValue failed");
            app.cancel_edit();
            app.show_error(dbus_error_message(&e));
        }
    }
}

/// Extract a human-readable string from a [`zbus::Error`].
///
/// Prefers the D-Bus `org.bootcontrol.Error.*` error name when it is
/// available; falls back to the full `Display` representation.
fn dbus_error_message(e: &zbus::Error) -> String {
    // zbus::Error::MethodError carries (error_name, detail, message)
    if let zbus::Error::MethodError(name, detail, _) = e {
        let short_name = name
            .strip_prefix("org.bootcontrol.Error.")
            .unwrap_or(name.as_str());
        return match detail {
            Some(d) if !d.is_empty() => format!("{short_name}: {d}"),
            _ => short_name.to_string(),
        };
    }
    e.to_string()
}
