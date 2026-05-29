//! Adapter for Claude Code JSONL session files.
//!
//! Parses the raw records Claude writes to `~/.claude/projects/**/*.jsonl`
//! and normalises them into `Vec<LogEvent>`. One raw line may expand to
//! multiple events (e.g. an assistant turn with several tool calls).

mod raw;

use crate::Adapter;
use aegon_types::{EventKind, LogEvent, Result, TokenUsage, ToolCall, ToolResult};
use chrono::DateTime;
use raw::{RawContent, RawRecord};
use uuid::Uuid;

/// Adapter for Claude Code session JSONL files.
pub struct ClaudeAdapter;

impl Adapter for ClaudeAdapter {
    fn parse_line(&self, line: &str) -> Result<Vec<LogEvent>> {
        let record: RawRecord =
            serde_json::from_str(line).map_err(|e| aegon_types::Error::InvalidField {
                field: "line",
                reason: e.to_string(),
            })?;

        let session_id = record.session_id().unwrap_or_else(Uuid::new_v4);

        let event_id = record
            .uuid
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(Uuid::new_v4);

        let parent_id = record.parent_uuid.and_then(|s| s.parse().ok());

        let timestamp = record
            .timestamp
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(chrono::Utc::now);

        let mut events = Vec::new();

        match record.record_type.as_deref() {
            Some("assistant") => {
                if let Some(msg) = record.message {
                    let usage = msg.usage.map(|u| TokenUsage {
                        input_tokens: u.input_tokens,
                        output_tokens: u.output_tokens,
                        cache_read_input_tokens: u.cache_read_input_tokens,
                        cache_creation_input_tokens: u.cache_creation_input_tokens,
                    });

                    let mut text_parts: Vec<String> = Vec::new();

                    for content in msg.content.unwrap_or_default() {
                        match content {
                            RawContent::Text { text } => text_parts.push(text),
                            RawContent::ToolUse { id, name, input } => {
                                events.push(make_event(
                                    Uuid::new_v4(),
                                    session_id,
                                    Some(event_id),
                                    timestamp,
                                    EventKind::ToolCall(ToolCall { id, name, input }),
                                ));
                            }
                            _ => {}
                        }
                    }

                    if !text_parts.is_empty() || usage.is_some() {
                        events.push(make_event(
                            event_id,
                            session_id,
                            parent_id,
                            timestamp,
                            EventKind::AssistantMessage {
                                content: text_parts.join("\n"),
                                usage,
                            },
                        ));
                    }
                }
            }

            Some("user") => {
                if let Some(msg) = record.message {
                    for content in msg.content.unwrap_or_default() {
                        match content {
                            RawContent::Text { text } => {
                                events.push(make_event(
                                    event_id,
                                    session_id,
                                    parent_id,
                                    timestamp,
                                    EventKind::UserMessage { content: text },
                                ));
                            }
                            RawContent::ToolResult {
                                tool_use_id,
                                content,
                                is_error,
                            } => {
                                events.push(make_event(
                                    Uuid::new_v4(),
                                    session_id,
                                    Some(event_id),
                                    timestamp,
                                    EventKind::ToolResult(ToolResult {
                                        tool_use_id,
                                        content: flatten_content(content),
                                        is_error: is_error.unwrap_or(false),
                                    }),
                                ));
                            }
                            _ => {}
                        }
                    }
                }
            }

            // queue-operation, summary, and future types — no observable signal
            _ => {}
        }

        Ok(events)
    }
}

fn make_event(
    id: Uuid,
    session_id: Uuid,
    parent_id: Option<Uuid>,
    timestamp: chrono::DateTime<chrono::Utc>,
    kind: EventKind,
) -> LogEvent {
    LogEvent {
        id,
        session_id,
        parent_id,
        timestamp,
        kind,
    }
}

/// Tool result content can be a plain string or a JSON array of text blocks.
fn flatten_content(raw: serde_json::Value) -> String {
    match raw {
        serde_json::Value::String(s) => s,
        serde_json::Value::Array(arr) => arr
            .into_iter()
            .filter_map(|v| v.get("text").and_then(|t| t.as_str()).map(str::to_owned))
            .collect::<Vec<_>>()
            .join("\n"),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Adapter;
    use aegon_types::EventKind;

    fn adapter() -> ClaudeAdapter {
        ClaudeAdapter
    }

    // ── parse_line happy paths ───────────────────────────────────────────────

    #[test]
    fn queue_operation_produces_no_events() {
        let line = r#"{"type":"queue-operation","operation":"enqueue","timestamp":"2026-05-29T14:00:00Z","sessionId":"00000000-0000-0000-0000-000000000001"}"#;
        let events = adapter().parse_line(line).unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn user_text_produces_user_message_event() {
        let line = r#"{
            "type":"user",
            "uuid":"00000000-0000-0000-0000-000000000002",
            "timestamp":"2026-05-29T14:00:00Z",
            "message":{"content":[{"type":"text","text":"hello"}]}
        }"#;
        let events = adapter().parse_line(line).unwrap();
        assert_eq!(events.len(), 1);
        assert!(
            matches!(&events[0].kind, EventKind::UserMessage { content } if content == "hello")
        );
    }

    #[test]
    fn user_tool_result_produces_tool_result_event() {
        let line = r#"{
            "type":"user",
            "uuid":"00000000-0000-0000-0000-000000000003",
            "timestamp":"2026-05-29T14:00:00Z",
            "message":{"content":[{
                "type":"tool_result",
                "tool_use_id":"tid1",
                "content":"output text",
                "is_error":false
            }]}
        }"#;
        let events = adapter().parse_line(line).unwrap();
        assert_eq!(events.len(), 1);
        match &events[0].kind {
            EventKind::ToolResult(tr) => {
                assert_eq!(tr.tool_use_id, "tid1");
                assert_eq!(tr.content, "output text");
                assert!(!tr.is_error);
            }
            other => panic!("expected ToolResult, got {other:?}"),
        }
    }

    #[test]
    fn assistant_with_tool_use_produces_tool_call_and_message() {
        let line = r#"{
            "type":"assistant",
            "uuid":"00000000-0000-0000-0000-000000000004",
            "timestamp":"2026-05-29T14:00:00Z",
            "message":{
                "role":"assistant",
                "content":[
                    {"type":"text","text":"I will run bash"},
                    {"type":"tool_use","id":"tc1","name":"Bash","input":{"command":"ls"}}
                ],
                "usage":{"input_tokens":10,"output_tokens":5}
            }
        }"#;
        let events = adapter().parse_line(line).unwrap();
        // Expect one ToolCall + one AssistantMessage
        assert_eq!(events.len(), 2);
        let has_tool_call = events
            .iter()
            .any(|e| matches!(&e.kind, EventKind::ToolCall(tc) if tc.name == "Bash"));
        let has_message = events.iter().any(|e| matches!(&e.kind, EventKind::AssistantMessage { content, .. } if content.contains("I will run bash")));
        assert!(has_tool_call);
        assert!(has_message);
    }

    #[test]
    fn assistant_with_usage_attaches_token_counts() {
        let line = r#"{
            "type":"assistant",
            "uuid":"00000000-0000-0000-0000-000000000005",
            "timestamp":"2026-05-29T14:00:00Z",
            "message":{
                "role":"assistant",
                "content":[{"type":"text","text":"ok"}],
                "usage":{"input_tokens":20,"output_tokens":3,"cache_read_input_tokens":7}
            }
        }"#;
        let events = adapter().parse_line(line).unwrap();
        let msg = events.iter().find_map(|e| {
            if let EventKind::AssistantMessage { usage, .. } = &e.kind {
                usage.as_ref()
            } else {
                None
            }
        });
        let usage = msg.expect("should have usage");
        assert_eq!(usage.input_tokens, 20);
        assert_eq!(usage.cache_read_input_tokens, 7);
    }

    // ── parse_line error path ────────────────────────────────────────────────

    #[test]
    fn malformed_json_returns_err() {
        let result = adapter().parse_line("not json at all {{{");
        assert!(result.is_err());
    }

    // ── flatten_content ──────────────────────────────────────────────────────

    #[test]
    fn flatten_content_plain_string() {
        let v = serde_json::Value::String("hello".into());
        assert_eq!(flatten_content(v), "hello");
    }

    #[test]
    fn flatten_content_text_block_array() {
        let v = serde_json::json!([{"type":"text","text":"line1"},{"type":"text","text":"line2"}]);
        assert_eq!(flatten_content(v), "line1\nline2");
    }

    #[test]
    fn flatten_content_empty_array_gives_empty_string() {
        let v = serde_json::Value::Array(vec![]);
        assert_eq!(flatten_content(v), "");
    }
}
