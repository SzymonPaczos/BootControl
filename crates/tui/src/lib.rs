//! `bootcontrol-tui` — library root for the Terminal User Interface.
//!
//! Exposes the TUI state machine and rendering modules so that integration
//! tests (and the doctests in each module) can `use bootcontrol_tui::...`
//! without depending on the binary entry point.
//!
//! The actual application entry point is in `src/main.rs`.

pub mod app;
pub mod events;
pub mod popup;
pub mod ui;
