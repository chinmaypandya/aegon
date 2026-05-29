//! Integration tests for the ClaudeAdapter public API.
//!
//! Tests call through the `Adapter` trait so the interface contract is
//! verified from the caller's perspective, not the implementation's.

use aegon_adapters::{Adapter, ClaudeAdapter};
use aegon_types::EventKind;

fn adapter() -> ClaudeAdapter {
    ClaudeAdapter
}

#[test]
fn parse_assistant_turn_with_tool_use_expands_to_multiple_events() {
    let line = r#"{
        "type": "assistant",
        "uuid": "00000000-0000-0000-0000-000000000010",
        "timestamp": "2026-05-30T10:00:00Z",
        "message": {
            "role": "assistant",
            "content": [
                { "type": "text", "text": "Running the command" },
                { "type": "tool_use", "id": "t1", "name": "Bash", "input": { "command": "ls" } }
            ],
            "usage": { "input_tokens": 10, "output_tokens": 4 }
        }
    }"#;

    let events = adapter().parse_line(line).unwrap();
    assert_eq!(events.len(), 2);

    let has_call = events
        .iter()
        .any(|e| matches!(&e.kind, EventKind::ToolCall(tc) if tc.name == "Bash"));
    let has_msg = events
        .iter()
        .any(|e| matches!(&e.kind, EventKind::AssistantMessage { content, .. } if content.contains("Running")));

    assert!(has_call, "expected a ToolCall event");
    assert!(has_msg, "expected an AssistantMessage event");
}

#[test]
fn parse_user_turn_with_tool_result() {
    let line = r#"{
        "type": "user",
        "uuid": "00000000-0000-0000-0000-000000000011",
        "timestamp": "2026-05-30T10:00:01Z",
        "message": {
            "role": "user",
            "content": [{
                "type": "tool_result",
                "tool_use_id": "t1",
                "content": "file.txt",
                "is_error": false
            }]
        }
    }"#;

    let events = adapter().parse_line(line).unwrap();
    assert_eq!(events.len(), 1);
    match &events[0].kind {
        EventKind::ToolResult(tr) => {
            assert_eq!(tr.tool_use_id, "t1");
            assert_eq!(tr.content, "file.txt");
        }
        other => panic!("expected ToolResult, got {other:?}"),
    }
}

#[test]
fn parse_unknown_type_returns_empty_vec() {
    let line = r#"{"type":"summary","timestamp":"2026-05-30T10:00:02Z"}"#;
    let events = adapter().parse_line(line).unwrap();
    assert!(events.is_empty());
}

#[test]
fn parse_invalid_json_returns_err() {
    assert!(adapter().parse_line("{{not valid").is_err());
}

#[test]
fn tool_result_with_json_array_content_is_flattened() {
    let line = r#"{
        "type": "user",
        "uuid": "00000000-0000-0000-0000-000000000012",
        "timestamp": "2026-05-30T10:00:03Z",
        "message": {
            "role": "user",
            "content": [{
                "type": "tool_result",
                "tool_use_id": "t2",
                "content": [
                    { "type": "text", "text": "line one" },
                    { "type": "text", "text": "line two" }
                ]
            }]
        }
    }"#;

    let events = adapter().parse_line(line).unwrap();
    assert_eq!(events.len(), 1);
    if let EventKind::ToolResult(tr) = &events[0].kind {
        assert!(tr.content.contains("line one"));
        assert!(tr.content.contains("line two"));
    } else {
        panic!("expected ToolResult");
    }
}
