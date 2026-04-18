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
    // ── Structured logging ────────────────────────────────────────────────────
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // ── Resolve Backend (D-Bus or Mock) ───────────────────────────────────────
    let backend = resolve_backend().await;

    // ── Initial config load ───────────────────────────────────────────────────
    info!("loading initial boot configuration");
    let app = match load_app(backend.as_ref()).await {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: failed to read config from backend: {e}");
            return Err(e.into());
        }
    };

    // ── Terminal setup ────────────────────────────────────────────────────────
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend_tui = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend_tui)?;
    terminal.clear()?;

    // ── Run the event loop ────────────────────────────────────────────────────
    let result = run_event_loop(&mut terminal, app, backend.clone()).await;

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
async fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    mut app: App,
    backend: Arc<dyn BootBackend>,
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

/// Dispatch a keyboard event to the correct handler based on `app.mode`.
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

    match key.code {
        KeyCode::Up | KeyCode::Char('k') => app.move_selection_up(),
        KeyCode::Down | KeyCode::Char('j') => app.move_selection_down(),
        KeyCode::Enter => app.open_edit_popup(),
        KeyCode::Char('r') | KeyCode::Char('R') => {
            reload_config(app, backend).await;
        }
        _ => {}
    }
}

// ── Editing mode ──────────────────────────────────────────────────────────────

async fn handle_editing_key(key: KeyEvent, app: &mut App, backend: &dyn BootBackend) {
    match key {
        // Confirm edit → call backend
        KeyEvent {
            code: KeyCode::Enter,
            ..
        } => {
            commit_edit(app, backend).await;
        }

        // Cancel edit
        KeyEvent {
            code: KeyCode::Esc, ..
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

        // Printable characters
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
// Backend helpers
// ──────────────────────────────────────────────────────────────────────────────

/// Call `read_config` and build a fresh [`App`].
async fn load_app(backend: &dyn BootBackend) -> Result<App, zbus::Error> {
    let (config, etag) = backend.read_config().await?;
    let mut entries: Vec<GrubEntry> = config
        .into_iter()
        .map(|(key, value)| GrubEntry { key, value })
        .collect();
    entries.sort_by(|a, b| a.key.cmp(&b.key));
    Ok(App::new(entries, etag))
}

/// Reload the config and update `app` in place.
async fn reload_config(app: &mut App, backend: &dyn BootBackend) {
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

/// Call `set_value` with the current edit buffer contents.
async fn commit_edit(app: &mut App, backend: &dyn BootBackend) {
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

