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
    /// Whether this record originated in a sub-agent sidechain.
    #[serde(rename = "isSidechain", default)]
    pub is_sidechain: bool,
    /// Message body for `user` and `assistant` records.
    pub message: Option<RawMessage>,
    /// Supplementary execution metadata on `user` records with tool results.
    #[serde(rename = "toolUseResult")]
    pub tool_use_result: Option<RawToolUseResult>,
}

impl RawRecord {
    /// Extract the session UUID from the top-level `sessionId` field.
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
/// new content types don't cause parse failures.
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
    Thinking {
        thinking: String,
        signature: Option<String>,
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
    /// Primary cache creation key.
    #[serde(default)]
    pub cache_creation_input_tokens: u64,
    /// Alternate cache creation key seen on some responses.
    #[serde(default)]
    pub cache_creation: u64,
}

/// Supplementary execution metadata written to `toolUseResult` on `user` records.
///
/// Shape varies by tool — shell tools carry `stdout`/`stderr`; file tools
/// carry a nested `file` object with `filePath`.
#[derive(Debug, Deserialize)]
pub struct RawToolUseResult {
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    #[serde(default)]
    pub interrupted: bool,
    #[serde(rename = "isImage", default)]
    pub is_image: bool,
    pub file: Option<RawToolFile>,
}

/// File metadata inside a `toolUseResult` for Read/Write tools.
#[derive(Debug, Deserialize)]
pub struct RawToolFile {
    #[serde(rename = "filePath")]
    pub file_path: Option<String>,
}
