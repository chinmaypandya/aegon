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
    /// Session ID — present on most record types at the top level.
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
    /// Auto-generated session title — present on `ai-title` records.
    #[serde(rename = "aiTitle")]
    pub ai_title: Option<String>,
    /// Session mode — present on `mode` records (`"normal"` / `"auto"`).
    pub mode: Option<String>,
    /// API error info — present on `system` records with `level: "error"`.
    pub error: Option<RawSystemError>,

    // ── queue-operation ──────────────────────────────────────────────────────
    /// Queue operation verb — present on `queue-operation` records.
    pub operation: Option<String>,

    // ── pr-link ──────────────────────────────────────────────────────────────
    /// PR number — present on `pr-link` records.
    #[serde(rename = "prNumber")]
    pub pr_number: Option<u64>,
    /// PR URL — present on `pr-link` records.
    #[serde(rename = "prUrl")]
    pub pr_url: Option<String>,
    /// GitHub repository slug (`"owner/repo"`) — present on `pr-link` records.
    #[serde(rename = "prRepository")]
    pub pr_repository: Option<String>,

    // ── last-prompt ──────────────────────────────────────────────────────────
    /// The last user prompt text — present on `last-prompt` records.
    #[serde(rename = "lastPrompt")]
    pub last_prompt: Option<String>,

    // ── file-history-snapshot ────────────────────────────────────────────────
    /// Whether this is a snapshot update (vs. initial snapshot).
    #[serde(rename = "isSnapshotUpdate", default)]
    pub is_snapshot_update: bool,

    // ── system (retry) ───────────────────────────────────────────────────────
    /// Milliseconds until the next retry — present on retryable `system` errors.
    #[serde(rename = "retryInMs")]
    pub retry_in_ms: Option<f64>,
    /// Which retry attempt this is (1-indexed).
    #[serde(rename = "retryAttempt")]
    pub retry_attempt: Option<u32>,
    /// Maximum number of retries the harness will attempt.
    #[serde(rename = "maxRetries")]
    pub max_retries: Option<u32>,

    // ── attachment ───────────────────────────────────────────────────────────
    /// Harness attachment — present on `attachment` records.
    pub attachment: Option<RawAttachment>,
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
    /// Image content — rendered as a placeholder label in the TUI.
    Image,
    #[serde(other)]
    Other,
}

/// Error payload from a `system` record.
#[derive(Debug, Deserialize)]
pub struct RawSystemError {
    pub message: Option<String>,
    pub formatted: Option<String>,
    pub connection: Option<RawConnectionError>,
}

/// Low-level connection error details inside a `system` error record.
#[derive(Debug, Deserialize)]
pub struct RawConnectionError {
    pub code: Option<String>,
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
    /// Token count for prompt tokens written into the cache this turn.
    #[serde(default)]
    pub cache_creation_input_tokens: u64,
    // NOTE: `cache_creation` and `server_tool_use` are nested objects in
    // the JSON, not numbers. They are intentionally absent here so serde
    // skips them rather than failing with "expected u64, got map".
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

/// The `attachment` object inside an `attachment` record.
///
/// Each subtype carries harness-level context injected between turns —
/// tool rosters, skill listings, hook outputs, plan mode state, etc.
/// Unknown subtypes are captured by `Other`.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RawAttachment {
    /// New tools were registered (or removed) from the agent's tool roster.
    DeferredToolsDelta {
        #[serde(rename = "addedNames", default)]
        added_names: Vec<String>,
        #[serde(rename = "removedNames", default)]
        removed_names: Vec<String>,
    },
    /// Available skill listing injected into context.
    SkillListing { content: String },
    /// Agent entered plan mode.
    PlanMode {
        #[serde(rename = "planFilePath")]
        plan_file_path: Option<String>,
        #[serde(rename = "planExists", default)]
        plan_exists: bool,
    },
    /// Agent exited plan mode.
    PlanModeExit {
        #[serde(rename = "planFilePath")]
        plan_file_path: Option<String>,
    },
    /// Current todo list state injected for context.
    TodoReminder {
        #[serde(rename = "itemCount", default)]
        item_count: u64,
    },
    /// A hook injected additional context after a tool use.
    HookAdditionalContext {
        #[serde(default)]
        content: Vec<String>,
        #[serde(rename = "hookName")]
        hook_name: String,
        #[serde(rename = "toolUseID")]
        tool_use_id: Option<String>,
    },
    /// A text file was modified; snippet shows the surrounding lines.
    EditedTextFile {
        filename: String,
        snippet: Option<String>,
    },
    /// The session's local date changed (crossed midnight).
    DateChange {
        #[serde(rename = "newDate")]
        new_date: String,
    },
    /// A queued command or background task notification was delivered.
    QueuedCommand {
        prompt: String,
        #[serde(rename = "commandMode")]
        command_mode: String,
    },
    /// Tool permission allowlist was updated for this session.
    CommandPermissions {
        #[serde(rename = "allowedTools", default)]
        allowed_tools: Vec<String>,
    },
    #[serde(other)]
    Other,
}
