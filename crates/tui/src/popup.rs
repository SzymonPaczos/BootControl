//! Floating popup widgets for the BootControl TUI.
//!
//! This module provides two widget builders:
//!
//! - [`edit_popup`] — A centred input box that lets the user type a new value
//!   for the selected GRUB variable.
//! - [`error_popup`] — A centred error overlay with a red border that displays
//!   a D-Bus error message.
//!
//! Both functions are **pure**: they only compute [`ratatui`] layout geometry
//! and widget trees; they have no I/O or side effects.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};

// ──────────────────────────────────────────────────────────────────────────────
// Geometry helper
// ──────────────────────────────────────────────────────────────────────────────

/// Return a [`Rect`] centred inside `area` with the given dimensions.
///
/// Both `width` and `height` are clamped to `area`'s dimensions to avoid a
/// popup that is larger than the terminal.
///
/// # Arguments
///
/// * `area`   — The bounding rectangle to centre inside (usually the full frame).
/// * `width`  — Desired popup width in columns.
/// * `height` — Desired popup height in rows.
///
/// # Examples
///
/// ```
/// use ratatui::layout::Rect;
/// use bootcontrol_tui::popup::centered_rect;
///
/// let area   = Rect::new(0, 0, 80, 24);
/// let popup  = centered_rect(area, 60, 10);
/// assert_eq!(popup.width,  60);
/// assert_eq!(popup.height, 10);
/// // Centred horizontally: (80 - 60) / 2 = 10
/// assert_eq!(popup.x, 10);
/// ```
pub fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);

    let popup_layout = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(h),
        Constraint::Fill(1),
    ])
    .flex(Flex::Center)
    .split(area);

    Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(w),
        Constraint::Fill(1),
    ])
    .flex(Flex::Center)
    .split(popup_layout[1])[1]
}

// ──────────────────────────────────────────────────────────────────────────────
// Edit popup
// ──────────────────────────────────────────────────────────────────────────────

/// Render an inline edit popup for the given `key` with the current `edit_buf`.
///
/// The popup is centred in `area`.  A fake cursor (`▌`) is appended to the
/// buffer text so the user can see the insertion point.
///
/// # Arguments
///
/// * `frame`    — Mutable reference to the current Ratatui frame.
/// * `area`     — Bounding rectangle for centring (typically the full terminal).
/// * `key`      — Name of the GRUB variable being edited (shown in the title).
/// * `edit_buf` — Current contents of the text input buffer.
pub fn edit_popup(frame: &mut Frame, area: Rect, key: &str, edit_buf: &str) {
    let popup_area = centered_rect(area, 64, 7);

    // Clear the cells beneath the popup first so it appears to float.
    frame.render_widget(Clear, popup_area);

    let title = format!(" Edit: {key} ");
    let block = Block::default()
        .title(title.as_str())
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Black));

    let cursor_line = format!("{edit_buf}▌");
    let inner_text = Text::from(vec![
        Line::from(""),
        Line::from(Span::styled(
            cursor_line,
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            " [Enter] Save   [Esc] Cancel ",
            Style::default().fg(Color::DarkGray),
        )),
    ]);

    let paragraph = Paragraph::new(inner_text)
        .block(block)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, popup_area);
}

// ──────────────────────────────────────────────────────────────────────────────
// Error popup
// ──────────────────────────────────────────────────────────────────────────────

/// Render an error overlay with the given `message`.
///
/// The popup has a red border to visually distinguish it from the edit popup.
/// The user dismisses it with `Esc` or `Enter`.
///
/// # Arguments
///
/// * `frame`   — Mutable reference to the current Ratatui frame.
/// * `area`    — Bounding rectangle for centring (typically the full terminal).
/// * `message` — The D-Bus error text to display.
pub fn error_popup(frame: &mut Frame, area: Rect, message: &str) {
    let popup_area = centered_rect(area, 64, 9);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" ⚠  Error ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Red))
        .style(Style::default().bg(Color::Black));

    let inner_text = Text::from(vec![
        Line::from(""),
        Line::from(Span::styled(
            message,
            Style::default().fg(Color::LightRed),
        )),
        Line::from(""),
        Line::from(Span::styled(
            " [Esc] Dismiss ",
            Style::default().fg(Color::DarkGray),
        )),
    ]);

    let paragraph = Paragraph::new(inner_text)
        .block(block)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, popup_area);
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Rect;

    #[test]
    fn centered_rect_produces_correct_dimensions() {
        let area = Rect::new(0, 0, 80, 24);
        let r = centered_rect(area, 60, 10);
        assert_eq!(r.width, 60);
        assert_eq!(r.height, 10);
    }

    #[test]
    fn centered_rect_is_horizontally_centred() {
        let area = Rect::new(0, 0, 80, 24);
        let r = centered_rect(area, 60, 10);
        // x should be (80 − 60) / 2 = 10
        assert_eq!(r.x, 10);
    }

    #[test]
    fn centered_rect_is_vertically_centred() {
        let area = Rect::new(0, 0, 80, 24);
        let r = centered_rect(area, 60, 10);
        // y should be (24 − 10) / 2 = 7
        assert_eq!(r.y, 7);
    }

    #[test]
    fn centered_rect_clamps_to_area_when_popup_too_large() {
        let area = Rect::new(0, 0, 40, 10);
        let r = centered_rect(area, 200, 100);
        assert_eq!(r.width, 40);
        assert_eq!(r.height, 10);
    }

    /// [`edit_popup`] and [`error_popup`] must not panic with a minimal TestBackend.
    #[test]
    fn popups_render_without_panic() {
        use ratatui::{Terminal, backend::TestBackend};

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("TestBackend must not fail");

        terminal
            .draw(|frame| {
                let area = frame.area();
                edit_popup(frame, area, "GRUB_TIMEOUT", "5");
            })
            .expect("draw must not fail");

        terminal
            .draw(|frame| {
                let area = frame.area();
                error_popup(frame, area, "PolkitDenied: not authorised");
            })
            .expect("draw must not fail");
    }
}
