//! Terminal UI for Aegon — live event feed and aggregated dashboard.
//!
//! Entry point for callers is [`tui::run`], which takes ownership of the event
//! receiver and blocks until the user quits.

pub mod app;
pub mod dashboard;
pub mod tui;
