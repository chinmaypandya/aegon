//! Variants describing what kind of event a [`LogEvent`] carries.

use crate::{TokenUsage, ToolCall, ToolResult};
use serde::{Deserialize, Serialize};

/// The payload of a single [`LogEvent`].
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
}
