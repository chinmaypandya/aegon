//! The central event record that flows through the Aegon pipeline.

use crate::event::EventKind;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// One discrete event recorded during a Claude Code session.
///
/// `session_id` groups all events that belong to the same run.
/// `parent_id` links each event to the one that caused it, forming a chain
/// that can be replayed deterministically. The `kind` field carries the actual
/// payload; everything else is observability metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogEvent {
    /// Unique identifier for this event.
    pub id: Uuid,
    /// Session this event belongs to.
    pub session_id: Uuid,
    /// The event immediately before this one in causal order, if any.
    pub parent_id: Option<Uuid>,
    /// Wall-clock time the event was recorded.
    pub timestamp: DateTime<Utc>,
    /// What this event represents.
    pub kind: EventKind,
}
