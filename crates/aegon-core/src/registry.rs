//! Multi-session registry — routes events to the right [`SessionState`].
//!
//! The CLI receives events from potentially many concurrent JSONL files.
//! [`SessionRegistry`] owns one [`SessionState`] per `session_id` and
//! creates new entries on first sight.

use crate::session::SessionState;
use aegon_types::LogEvent;
use std::collections::HashMap;
use uuid::Uuid;

/// Owns and routes events to per-session state.
///
/// The typical call pattern is: receive a [`LogEvent`] from the watcher
/// channel, call [`ingest`], then read from [`sessions`] or [`latest`] to
/// refresh the dashboard.
#[derive(Debug, Default)]
pub struct SessionRegistry {
    sessions: HashMap<Uuid, SessionState>,
    /// Insertion-ordered session IDs so the UI can show them in arrival order.
    order: Vec<Uuid>,
}

impl SessionRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Route an event to its session, creating the session on first sight.
    pub fn ingest(&mut self, event: &LogEvent) {
        let id = event.session_id;
        let state = self.sessions.entry(id).or_insert_with(|| {
            self.order.push(id);
            SessionState::new(id)
        });
        state.ingest(event);
    }

    /// All sessions in arrival order.
    pub fn sessions(&self) -> impl Iterator<Item = &SessionState> {
        self.order.iter().filter_map(|id| self.sessions.get(id))
    }

    /// The most recently active session (last to receive an event).
    pub fn latest(&self) -> Option<&SessionState> {
        self.order.last().and_then(|id| self.sessions.get(id))
    }

    /// Number of sessions seen so far.
    pub fn len(&self) -> usize {
        self.sessions.len()
    }

    /// Whether no sessions have been seen yet.
    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }
}
