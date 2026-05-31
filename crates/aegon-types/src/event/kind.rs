//! Variants describing what kind of event a [`crate::LogEvent`] carries.

use crate::{TokenUsage, ToolCall, ToolResult};
use serde::{Deserialize, Serialize};

/// The payload of a single [`crate::LogEvent`].
///
/// Each variant corresponds to one observable action in a Claude Code session.
/// Unknown or future event types are captured by [`EventKind::Unknown`] so the
/// parser never breaks on new fields added by the tool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventKind {
    /// The model requested a tool to be run.
    ToolCall(ToolCall),
    /// A tool finished and returned output to the model.
    ToolResult(ToolResult),
    /// A text response from the assistant, optionally with token counts.
    AssistantMessage {
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        usage: Option<TokenUsage>,
    },
    /// A message sent by the user (human turn).
    UserMessage { content: String },
    /// Token usage reported as a standalone event (some providers emit it separately).
    TokenUsage(TokenUsage),
    /// Extended thinking produced by the model before its response.
    ///
    /// Only present when the model uses extended thinking mode. The `text`
    /// field contains the raw reasoning; `signature` is an opaque integrity
    /// token produced by the API.
    Thinking {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        signature: Option<String>,
    },
    /// Auto-generated title for the session, emitted by Claude Code after the first exchange.
    SessionTitle { title: String },
    /// Session mode change — `"normal"` or `"auto"`.
    SessionMode { mode: String },
    /// An API-level error or retry event recorded by Claude Code.
    ///
    /// Covers connection failures, rate limits, and transient errors. The
    /// `code` field is the low-level error code (e.g. `"ECONNRESET"`).
    /// When `retry_attempt` is present the harness is retrying automatically.
    SystemError {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        code: Option<String>,
        /// Which retry attempt this is (1-indexed), if the harness is retrying.
        #[serde(skip_serializing_if = "Option::is_none")]
        retry_attempt: Option<u32>,
        /// Maximum retries the harness will attempt before surfacing the error.
        #[serde(skip_serializing_if = "Option::is_none")]
        max_retries: Option<u32>,
        /// Milliseconds until the next retry attempt.
        #[serde(skip_serializing_if = "Option::is_none")]
        retry_in_ms: Option<f64>,
    },

    // ── Session-level bookkeeping ────────────────────────────────────────────
    /// An internal message queue operation (`enqueue`, `dequeue`, or `remove`).
    QueueOperation { operation: String },

    /// A GitHub PR was linked to this session.
    PrLinked {
        pr_number: u64,
        pr_url: String,
        repository: String,
    },

    /// The most recent user prompt, stored at session boundaries for replay.
    LastPrompt { content: String },

    /// A file-history snapshot was written to support undo of edits.
    FileSnapshot { is_update: bool },

    // ── Harness attachment events ────────────────────────────────────────────
    /// Deferred tools were registered into (or removed from) the tool roster.
    ///
    /// Emitted when the harness expands the set of tools available to the
    /// model mid-session (e.g. skill tools, MCP tools, plugin tools).
    ToolsRegistered {
        added: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        removed: Vec<String>,
    },

    /// The harness injected the available skill listing into context.
    SkillsLoaded { content: String },

    /// The agent entered plan mode.
    PlanModeEntered {
        #[serde(skip_serializing_if = "Option::is_none")]
        plan_file: Option<String>,
        plan_exists: bool,
    },

    /// The agent exited plan mode.
    PlanModeExited {
        #[serde(skip_serializing_if = "Option::is_none")]
        plan_file: Option<String>,
    },

    /// The agent's todo list state was injected into context.
    TodoUpdated { item_count: u64 },

    /// A hook provided additional context after a tool use.
    ///
    /// Fired by `PostToolUse` hooks (e.g. IDE diagnostics after an `Edit`).
    HookOutput {
        hook_name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_use_id: Option<String>,
        content: Vec<String>,
    },

    /// A file was modified, captured by a PostToolUse hook for context injection.
    FileEdited {
        path: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        snippet: Option<String>,
    },

    /// The session crossed a date boundary (local clock rolled past midnight).
    DateChange { new_date: String },

    /// A background task completed and its result was delivered to the agent.
    BackgroundTaskResult {
        #[serde(skip_serializing_if = "Option::is_none")]
        task_id: Option<String>,
        status: String,
        summary: String,
    },

    /// Tool permission allowlist was updated for this session.
    PermissionsUpdated { allowed_tools: Vec<String> },

    /// An event type not yet handled by this version of Aegon.
    #[serde(other)]
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ToolCall, ToolResult};
    use serde_json::json;

    #[test]
    fn tool_call_round_trips() {
        let kind = EventKind::ToolCall(ToolCall {
            id: "id1".into(),
            name: "Bash".into(),
            input: json!({"command": "ls"}),
        });
        let json = serde_json::to_string(&kind).unwrap();
        let back: EventKind = serde_json::from_str(&json).unwrap();
        assert_eq!(kind, back);
    }

    #[test]
    fn tool_result_round_trips() {
        let kind = EventKind::ToolResult(ToolResult {
            tool_use_id: "id1".into(),
            content: "ok".into(),
            is_error: false,
            metadata: None,
        });
        let json = serde_json::to_string(&kind).unwrap();
        let back: EventKind = serde_json::from_str(&json).unwrap();
        assert_eq!(kind, back);
    }

    #[test]
    fn assistant_message_without_usage_omits_field() {
        let kind = EventKind::AssistantMessage {
            content: "hi".into(),
            usage: None,
        };
        let json = serde_json::to_string(&kind).unwrap();
        assert!(!json.contains("usage"));
    }

    #[test]
    fn unknown_variant_deserialises_from_unrecognised_type() {
        let json = r#"{"type":"future_unknown_thing"}"#;
        let kind: EventKind = serde_json::from_str(json).unwrap();
        assert_eq!(kind, EventKind::Unknown);
    }

    #[test]
    fn system_error_with_retry_fields_round_trips() {
        let kind = EventKind::SystemError {
            message: "Connection error.".into(),
            code: Some("ECONNRESET".into()),
            retry_attempt: Some(1),
            max_retries: Some(10),
            retry_in_ms: Some(534.4),
        };
        let json = serde_json::to_string(&kind).unwrap();
        let back: EventKind = serde_json::from_str(&json).unwrap();
        assert_eq!(kind, back);
    }

    #[test]
    fn system_error_without_retry_omits_optional_fields() {
        let kind = EventKind::SystemError {
            message: "oops".into(),
            code: None,
            retry_attempt: None,
            max_retries: None,
            retry_in_ms: None,
        };
        let json = serde_json::to_string(&kind).unwrap();
        assert!(!json.contains("retry_attempt"));
        assert!(!json.contains("retry_in_ms"));
    }

    #[test]
    fn tools_registered_round_trips() {
        let kind = EventKind::ToolsRegistered {
            added: vec!["TodoWrite".into(), "WebFetch".into()],
            removed: vec![],
        };
        let json = serde_json::to_string(&kind).unwrap();
        let back: EventKind = serde_json::from_str(&json).unwrap();
        assert_eq!(kind, back);
    }

    #[test]
    fn pr_linked_round_trips() {
        let kind = EventKind::PrLinked {
            pr_number: 42,
            pr_url: "https://github.com/org/repo/pull/42".into(),
            repository: "org/repo".into(),
        };
        let json = serde_json::to_string(&kind).unwrap();
        let back: EventKind = serde_json::from_str(&json).unwrap();
        assert_eq!(kind, back);
    }

    #[test]
    fn background_task_result_round_trips() {
        let kind = EventKind::BackgroundTaskResult {
            task_id: Some("abc123".into()),
            status: "completed".into(),
            summary: "Build succeeded".into(),
        };
        let json = serde_json::to_string(&kind).unwrap();
        let back: EventKind = serde_json::from_str(&json).unwrap();
        assert_eq!(kind, back);
    }
}
