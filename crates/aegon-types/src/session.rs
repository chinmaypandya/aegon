//! A single Claude Code session — the top-level container for a run's events.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// One Claude Code agentic session, identified by the JSONL file it came from.
///
/// A session groups all [`LogEvent`]s that share the same `session_id`. The
/// `path` field is the source JSONL file so events can be reloaded from disk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Session {
    /// Unique identifier for this session.
    pub id: Uuid,
    /// Absolute path to the JSONL file this session was read from.
    pub path: String,
    /// Wall-clock time of the first event seen in this session.
    pub started_at: DateTime<Utc>,
    /// Wall-clock time of the last event seen, if the session has ended.
    pub ended_at: Option<DateTime<Utc>>,
}
