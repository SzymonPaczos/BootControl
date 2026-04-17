//! Pure rendering layer for the BootControl TUI.
//!
//! The single entry-point [`render`] accepts a Ratatui [`Frame`] and an
//! immutable snapshot of [`App`] and produces the complete widget tree for
//! that frame.  It has **no I/O, no D-Bus calls, and no filesystem access**.
//!
//! # Layout
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────┐
//! │  BootControl TUI  •  /etc/default/grub                     │ ← header (3 rows)
//! ├────────────────────────────────────────────────────────────┤
//! │  KEY                      │  VALUE                         │ ← table (fills remaining)
//! │  GRUB_CMDLINE_LINUX       │  quiet splash                  │
//! │  GRUB_DEFAULT             │  0                             │ ← highlighted row
//! │  GRUB_TIMEOUT             │  5                             │
//! ├────────────────────────────────────────────────────────────┤
//! │  [↑↓] Navigate  [Enter] Edit  [r] Reload  [q] Quit        │ ← footer (1 row)
//! └────────────────────────────────────────────────────────────┘
//! ```
//!
//! Overlays (floating, rendered on top when active):
//! - **Edit popup** — [`crate::popup::edit_popup`]
//! - **Error popup** — [`crate::popup::error_popup`]

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table, TableState},
};

use crate::app::{App, Mode};
use crate::popup;

// ──────────────────────────────────────────────────────────────────────────────
// Colour palette
// ──────────────────────────────────────────────────────────────────────────────

const ACCENT:       Color = Color::Cyan;
const HEADER_FG:    Color = Color::White;
const HEADER_BG:    Color = Color::Rgb(20, 20, 40);
const ROW_SELECTED: Color = Color::Rgb(0, 80, 120);
const ROW_ALT:      Color = Color::Rgb(18, 18, 30);
const ROW_NORMAL:   Color = Color::Rgb(12, 12, 22);
const FOOTER_FG:    Color = Color::DarkGray;
const KEY_FG:       Color = Color::Rgb(140, 210, 255);
const VALUE_FG:     Color = Color::Rgb(200, 255, 200);

// ──────────────────────────────────────────────────────────────────────────────
// Public entry point
// ──────────────────────────────────────────────────────────────────────────────

/// Render the entire TUI into `frame` based on the current `app` state.
///
/// This function is **pure** — calling it repeatedly with the same `app`
/// produces identical output.  The [`ratatui::backend::TestBackend`] used in
/// tests makes this straightforward to verify.
///
/// # Arguments
///
/// * `frame` — Mutable reference to the current Ratatui frame.
/// * `app`   — Immutable snapshot of the application state.
pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // ── Global vertical layout ────────────────────────────────────────────────
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Fill(1),   // table
            Constraint::Length(1), // footer
        ])
        .split(area);

    render_header(frame, app, chunks[0]);
    render_table(frame, app, chunks[1]);
    render_footer(frame, app, chunks[2]);

    // ── Floating overlays (rendered last so they appear on top) ───────────────
    match &app.mode {
        Mode::Editing => {
            let key = app
                .current_entry()
                .map(|e| e.key.as_str())
                .unwrap_or("?");
            popup::edit_popup(frame, area, key, &app.edit_buf);
        }
        Mode::ErrorPopup => {
            let msg = app.error_msg.as_deref().unwrap_or("Unknown error");
            popup::error_popup(frame, area, msg);
        }
        Mode::Browse => {}
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Header
// ──────────────────────────────────────────────────────────────────────────────

fn render_header(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let etag_short = if app.etag.len() >= 8 {
        &app.etag[..8]
    } else {
        &app.etag
    };

    let title_line = Line::from(vec![
        Span::styled(
            "  BootControl TUI",
            Style::default()
                .fg(ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "  •  /etc/default/grub",
            Style::default().fg(HEADER_FG),
        ),
        Span::styled(
            format!("  [etag: {etag_short}…]"),
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    let status_line = Line::from(Span::styled(
        format!("  {}", app.status_msg),
        Style::default().fg(Color::DarkGray),
    ));

    let header_block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(ACCENT))
        .border_type(BorderType::Plain)
        .style(Style::default().bg(HEADER_BG));

    let header_widget = Paragraph::new(vec![title_line, status_line])
        .block(header_block);

    frame.render_widget(header_widget, area);
}

// ──────────────────────────────────────────────────────────────────────────────
// Main table
// ──────────────────────────────────────────────────────────────────────────────

fn render_table(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let header_cells = [
        Cell::from("  KEY").style(
            Style::default()
                .fg(ACCENT)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        ),
        Cell::from("VALUE").style(
            Style::default()
                .fg(ACCENT)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        ),
    ];
    let header_row = Row::new(header_cells).height(1);

    let rows: Vec<Row> = app
        .entries
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let bg = if i % 2 == 0 { ROW_NORMAL } else { ROW_ALT };
            Row::new([
                Cell::from(format!("  {}", entry.key)).style(
                    Style::default().fg(KEY_FG).bg(bg),
                ),
                Cell::from(entry.value.clone()).style(
                    Style::default().fg(VALUE_FG).bg(bg),
                ),
            ])
            .height(1)
        })
        .collect();

    let highlight_style = Style::default()
        .bg(ROW_SELECTED)
        .add_modifier(Modifier::BOLD);

    let table = Table::new(rows, [Constraint::Percentage(45), Constraint::Fill(1)])
        .header(header_row)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(ACCENT))
                .style(Style::default().bg(ROW_NORMAL)),
        )
        .row_highlight_style(highlight_style)
        .highlight_symbol("▶ ");

    let mut table_state = TableState::default().with_selected(Some(app.selected));
    frame.render_stateful_widget(table, area, &mut table_state);
}

// ──────────────────────────────────────────────────────────────────────────────
// Footer
// ──────────────────────────────────────────────────────────────────────────────

fn render_footer(frame: &mut Frame, _app: &App, area: ratatui::layout::Rect) {
    let help = match _app.mode {
        Mode::Browse => {
            " [↑↓] Navigate  [Enter] Edit  [r] Reload  [q] Quit "
        }
        Mode::Editing => " [Enter] Save  [Esc] Cancel  [Backspace] Delete char ",
        Mode::ErrorPopup => " [Esc] Dismiss error ",
    };

    let footer = Paragraph::new(Line::from(Span::styled(
        help,
        Style::default().fg(FOOTER_FG),
    )))
    .style(Style::default().bg(HEADER_BG));

    frame.render_widget(footer, area);
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::GrubEntry;
    use ratatui::{Terminal, backend::TestBackend};

    fn make_app(keys: &[(&str, &str)]) -> App {
        let entries = keys
            .iter()
            .map(|(k, v)| GrubEntry {
                key: k.to_string(),
                value: v.to_string(),
            })
            .collect();
        App::new(entries, "deadbeef".into())
    }

    #[test]
    fn render_does_not_panic_with_empty_config() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("terminal must init");
        let app = make_app(&[]);
        terminal.draw(|f| render(f, &app)).expect("draw must succeed");
    }

    #[test]
    fn render_does_not_panic_with_typical_config() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).expect("terminal must init");
        let app = make_app(&[
            ("GRUB_TIMEOUT", "5"),
            ("GRUB_DEFAULT", "0"),
            ("GRUB_CMDLINE_LINUX", "quiet splash"),
            ("GRUB_GFXMODE", "auto"),
        ]);
        terminal.draw(|f| render(f, &app)).expect("draw must succeed");
    }

    #[test]
    fn render_does_not_panic_in_editing_mode() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("terminal must init");
        let mut app = make_app(&[("GRUB_TIMEOUT", "5")]);
        app.open_edit_popup();
        terminal.draw(|f| render(f, &app)).expect("draw must succeed");
    }

    #[test]
    fn render_does_not_panic_in_error_mode() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("terminal must init");
        let mut app = make_app(&[("GRUB_TIMEOUT", "5")]);
        app.show_error("org.bootcontrol.Error.PolkitDenied: not authorised".into());
        terminal.draw(|f| render(f, &app)).expect("draw must succeed");
    }

    #[test]
    fn render_does_not_panic_with_very_narrow_terminal() {
        let backend = TestBackend::new(20, 5);
        let mut terminal = Terminal::new(backend).expect("terminal must init");
        let app = make_app(&[("GRUB_TIMEOUT", "5")]);
        terminal.draw(|f| render(f, &app)).expect("draw on narrow terminal must not panic");
    }

    #[test]
    fn render_does_not_panic_with_unicode_values() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("terminal must init");
        let app = make_app(&[("GRUB_CMDLINE_LINUX", "résumé 日本語 αβγ")]);
        terminal.draw(|f| render(f, &app)).expect("unicode values must not panic");
    }
}
