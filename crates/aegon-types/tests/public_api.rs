//! Integration tests for aegon-types public API.
//!
//! These tests import only through the crate's public surface to catch
//! accidental breakage of the external interface.

use aegon_types::{EventKind, LogEvent, Session, StreamId, TokenUsage, ToolCall, ToolResult};
use chrono::Utc;
use uuid::Uuid;

fn make_log_event(kind: EventKind) -> LogEvent {
    LogEvent {
        id: Uuid::new_v4(),
        session_id: Uuid::new_v4(),
        parent_id: None,
        timestamp: Utc::now(),
        stream: StreamId::Main,
        kind,
    }
}

#[test]
fn log_event_with_tool_call_is_constructible() {
    let tc = ToolCall {
        id: "tc1".into(),
        name: "Bash".into(),
        input: serde_json::json!({"command": "ls"}),
    };
    let event = make_log_event(EventKind::ToolCall(tc));
    assert!(matches!(event.kind, EventKind::ToolCall(_)));
}

#[test]
fn log_event_with_tool_result_is_constructible() {
    let tr = ToolResult {
        tool_use_id: "tc1".into(),
        content: "file.txt".into(),
        is_error: false,
        metadata: None,
    };
    let event = make_log_event(EventKind::ToolResult(tr));
    assert!(matches!(event.kind, EventKind::ToolResult(_)));
}

#[test]
fn log_event_with_assistant_message_round_trips_json() {
    let event = make_log_event(EventKind::AssistantMessage {
        content: "Hello".into(),
        usage: Some(TokenUsage {
            input_tokens: 5,
            output_tokens: 2,
            ..Default::default()
        }),
    });
    let json = serde_json::to_string(&event).unwrap();
    let back: LogEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(event.id, back.id);
    assert_eq!(event.kind, back.kind);
}

#[test]
fn session_is_constructible() {
    let s = Session {
        id: Uuid::new_v4(),
        path: "/tmp/session.jsonl".into(),
        started_at: Utc::now(),
        ended_at: None,
    };
    assert!(s.ended_at.is_none());
}

#[test]
fn token_usage_default_is_all_zeros() {
    let u = TokenUsage::default();
    assert_eq!(u.input_tokens + u.output_tokens, 0);
}

#[test]
fn unknown_event_kind_survives_json_round_trip() {
    let json = r#"{"type":"not_a_real_type"}"#;
    let kind: EventKind = serde_json::from_str(json).unwrap();
    assert_eq!(kind, EventKind::Unknown);
}
