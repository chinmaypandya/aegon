//! In-memory application state shared between the watcher and the TUI.
//!
//! Keeps a rolling list of the most recent events so the TUI can render them
//! without holding a lock on the full event store.

use aegon_types::LogEvent;

/// Maximum number of events retained in the display buffer.
const MAX_EVENTS: usize = 500;

/// Live application state rendered by the TUI.
pub struct App {
    /// Most recent events in arrival order, capped at [`MAX_EVENTS`].
    pub events: Vec<LogEvent>,
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
            should_quit: false,
        }
    }

    /// Append a new event, evicting the oldest if the buffer is full.
    pub fn push(&mut self, event: LogEvent) {
        if self.events.len() >= MAX_EVENTS {
            self.events.remove(0);
        }
        self.events.push(event);
    }
}
