//! In-memory application state shared between the watcher and the TUI.
//!
//! Holds both the raw event feed (for the left panel) and the session
//! registry (for the right dashboard panel).

use aegon_core::SessionRegistry;
use aegon_types::LogEvent;

/// Maximum number of raw events retained in the display buffer.
const MAX_EVENTS: usize = 500;

/// Live application state rendered by the TUI.
pub struct App {
    /// Most recent raw events in arrival order, capped at 500 entries.
    pub events: Vec<LogEvent>,
    /// Per-session aggregated state consumed by the dashboard panel.
    pub sessions: SessionRegistry,
    /// Increments each frame to drive spinner animation.
    pub tick: u64,
    /// Whether the user has requested the application to quit.
    pub should_quit: bool,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            sessions: SessionRegistry::new(),
            tick: 0,
            should_quit: false,
        }
    }

    /// Ingest one event into both the raw feed and the session registry.
    pub fn push(&mut self, event: LogEvent) {
        self.sessions.ingest(&event);
        if self.events.len() >= MAX_EVENTS {
            self.events.remove(0);
        }
        self.events.push(event);
    }

    /// Advance the animation tick (call once per frame).
    pub fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
    }
}
