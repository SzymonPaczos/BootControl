//! Crossterm input adapter — converts raw terminal events into a simple
//! [`AppEvent`] enum consumed by the event loop in [`crate::main`].
//!
//! The async [`event_stream`] function returns a [`tokio_stream::Stream`] of
//! [`AppEvent`] values.  It wraps `crossterm::event::EventStream` and emits
//! a [`AppEvent::Tick`] every 250 ms (heartbeat) via a `tokio::select!`.
//!
//! # Design note
//!
//! Splitting terminal event polling into its own module keeps `main.rs` focused
//! on orchestration and makes the event adapter unit-testable in isolation.

use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyModifiers};
use tokio::time::{Duration, Instant};
use tokio_stream::StreamExt as _;

/// Events produced by the terminal input adapter.
///
/// The TUI event loop ([`crate::main`]) matches on these variants — each
/// represents one possible reason the loop just woke up. `Key` is the only
/// variant that carries data; `Tick` and `Resize` are signals.
///
/// # Examples
///
/// ```
/// use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
/// use bootcontrol_tui::events::AppEvent;
///
/// // The Debug impl is what the TUI main loop logs at trace-level.
/// let k = AppEvent::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
/// let dbg = format!("{k:?}");
/// assert!(dbg.contains("Key"));
///
/// let tick = AppEvent::Tick;
/// assert_eq!(format!("{tick:?}"), "Tick");
/// ```
#[derive(Debug)]
pub enum AppEvent {
    /// A keyboard event delivered by crossterm.
    Key(KeyEvent),
    /// 250 ms heartbeat — can be used for animations or spinner updates.
    Tick,
    /// Terminal window resize — triggers a redraw at the new dimensions.
    Resize,
}

/// Return `true` if `key` is a "quit" chord in Browse mode.
///
/// Accepted quit signals: `q`, `Q`, `Ctrl-C`, `Ctrl-D`.
///
/// # Arguments
///
/// * `key` — The keyboard event to test.
///
/// # Examples
///
/// ```
/// use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
/// use bootcontrol_tui::events::is_quit_key;
///
/// let q = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
/// assert!(is_quit_key(&q));
///
/// let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
/// assert!(!is_quit_key(&enter));
/// ```
pub fn is_quit_key(key: &KeyEvent) -> bool {
    matches!(
        key,
        KeyEvent {
            code: KeyCode::Char('q') | KeyCode::Char('Q'),
            modifiers: KeyModifiers::NONE,
            ..
        } | KeyEvent {
            code: KeyCode::Char('c') | KeyCode::Char('d'),
            modifiers: KeyModifiers::CONTROL,
            ..
        }
    )
}

/// Drive the crossterm `EventStream` and inject regular `Tick` heartbeats.
///
/// This function runs inside the main `tokio::select!` loop.  It returns the
/// next [`AppEvent`], blocking until either a terminal event arrives or the
/// 250 ms tick interval fires.
///
/// # Arguments
///
/// * `stream`      — Mutable reference to the crossterm `EventStream`.
/// * `tick_rate`   — Heartbeat interval (use `Duration::from_millis(250)`).
/// * `last_tick`   — `Instant` of the last tick emission (updated in-place).
///
/// # Errors
///
/// Returns `None` when the crossterm event stream ends (terminal closed).
///
/// # Examples
///
/// ```no_run
/// # use crossterm::event::EventStream;
/// # use tokio::time::{Duration, Instant};
/// # use bootcontrol_tui::events::next_event;
/// # async fn run() {
/// // Real usage requires a live crossterm terminal; flagged `no_run` so the
/// // doctest still type-checks under `cargo test --doc` without a TTY.
/// let mut stream = EventStream::new();
/// let mut last_tick = Instant::now();
/// let _ev = next_event(&mut stream, Duration::from_millis(250), &mut last_tick).await;
/// # }
/// ```
pub async fn next_event(
    stream: &mut EventStream,
    tick_rate: Duration,
    last_tick: &mut Instant,
) -> Option<AppEvent> {
    let timeout = tick_rate.saturating_sub(last_tick.elapsed());

    tokio::select! {
        maybe_event = stream.next() => {
            match maybe_event? {
                Ok(Event::Key(key))    => Some(AppEvent::Key(key)),
                Ok(Event::Resize(..))  => Some(AppEvent::Resize),
                Ok(_)                  => Some(AppEvent::Tick),
                Err(_)                 => None,
            }
        }
        _ = tokio::time::sleep(timeout) => {
            *last_tick = Instant::now();
            Some(AppEvent::Tick)
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quit_key_q_lowercase_detected() {
        let k = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert!(is_quit_key(&k));
    }

    #[test]
    fn quit_key_q_uppercase_detected() {
        let k = KeyEvent::new(KeyCode::Char('Q'), KeyModifiers::NONE);
        assert!(is_quit_key(&k));
    }

    #[test]
    fn quit_key_ctrl_c_detected() {
        let k = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert!(is_quit_key(&k));
    }

    #[test]
    fn quit_key_ctrl_d_detected() {
        let k = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL);
        assert!(is_quit_key(&k));
    }

    #[test]
    fn enter_is_not_a_quit_key() {
        let k = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        assert!(!is_quit_key(&k));
    }

    #[test]
    fn arrow_up_is_not_a_quit_key() {
        let k = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        assert!(!is_quit_key(&k));
    }

    #[test]
    fn ctrl_q_is_not_a_quit_key() {
        // Only lowercase/uppercase 'q' without modifiers qualifies.
        let k = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL);
        assert!(!is_quit_key(&k));
    }
}
