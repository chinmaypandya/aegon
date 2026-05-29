//! Raw serde types mirroring the Claude Code JSONL schema.
//!
//! These are intentionally lenient — unknown fields are ignored so the
//! adapter does not break when Claude adds new fields to its output.

use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

/// One raw line from a Claude Code JSONL session file.
#[derive(Debug, Deserialize)]
pub struct RawRecord {
    /// Top-level event type: `"user"`, `"assistant"`, `"queue-operation"`, etc.
    #[serde(rename = "type")]
    pub record_type: Option<String>,
    /// Stable ID for this record.
    pub uuid: Option<String>,
    /// ID of the record that causally preceded this one.
    #[serde(rename = "parentUuid")]
    pub parent_uuid: Option<String>,
    /// ISO-8601 wall-clock time.
    pub timestamp: Option<String>,
    /// Session ID — present on `queue-operation` lines at the top level.
    #[serde(rename = "sessionId")]
    pub session_id_field: Option<String>,
    /// Message body for `user` and `assistant` records.
    pub message: Option<RawMessage>,
}

impl RawRecord {
    /// Extract the session UUID, preferring the top-level `sessionId` field
    /// and falling back to parsing `uuid` as a session hint.
    pub fn session_id(&self) -> Option<Uuid> {
        self.session_id_field
            .as_deref()
            .and_then(|s| s.parse().ok())
    }
}

/// The `message` object inside a `user` or `assistant` record.
#[derive(Debug, Deserialize)]
pub struct RawMessage {
    pub content: Option<Vec<RawContent>>,
    pub usage: Option<RawUsage>,
}

/// One item in a `message.content` array.
///
/// Tagged by the `type` field. Unknown variants are captured by `Other` so
/// new content types (e.g. `"thinking"`) don't cause parse failures.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RawContent {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    ToolResult {
        tool_use_id: String,
        /// Can be a plain string or a JSON array of `{type, text}` blocks.
        #[serde(default = "Value::default")]
        content: Value,
        is_error: Option<bool>,
    },
    #[serde(other)]
    Other,
}

/// Raw token usage from the `usage` field on assistant messages.
#[derive(Debug, Deserialize)]
pub struct RawUsage {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_read_input_tokens: u64,
    #[serde(default)]
    pub cache_creation_input_tokens: u64,
}
