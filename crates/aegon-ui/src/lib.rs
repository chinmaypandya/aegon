//! Terminal UI for Aegon — live ratatui dashboard for Claude Code session events.
//!
//! Entry point for callers is [`tui::run`], which takes ownership of the event
//! receiver and blocks until the user quits.

pub mod app;
pub mod tui;
