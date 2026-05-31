//! Anthropic SSE event decoder.
//!
//! Knows the Anthropic streaming API's specific event schema and converts raw
//! `SseEvent`s into `EventKind::TokenChunk` log events. All other event types
//! (ping, message_start, content_block_stop, etc.) are silently ignored.
//!
//! The decoder is stateful per HTTP request: it tracks which content block
//! indices are `thinking` vs `text` blocks, so that `content_block_delta`
//! events can be labelled correctly.

use crate::sse::SseEvent;
use aegon_types::{EventKind, LogEvent, StreamId};
use chrono::Utc;
use serde::Deserialize;
use std::collections::HashMap;
use uuid::Uuid;

/// Per-request state needed to decode a stream of SSE events.
///
/// `content_block_start` events establish which block index is `thinking` vs
/// `text`; subsequent `content_block_delta` events reference those indices.
#[derive(Default)]
pub struct TurnState {
    /// Maps content block index → whether it is a thinking block.
    block_is_thinking: HashMap<u32, bool>,
}

impl TurnState {
    pub fn new() -> Self {
        Self::default()
    }
}

// ── Anthropic SSE JSON shapes ────────────────────────────────────────────────

#[derive(Deserialize)]
struct ContentBlockStart {
    index: u32,
    content_block: ContentBlockMeta,
}

#[derive(Deserialize)]
struct ContentBlockMeta {
    #[serde(rename = "type")]
    block_type: String,
}

#[derive(Deserialize)]
struct ContentBlockDelta {
    index: u32,
    delta: Delta,
}

#[derive(Deserialize)]
struct Delta {
    #[serde(rename = "type")]
    delta_type: String,
    /// Present on `text_delta` events.
    #[serde(default)]
    text: String,
    /// Present on `thinking_delta` events.
    #[serde(default)]
    thinking: String,
}

// ── Public decoder ────────────────────────────────────────────────────────────

/// Decode one SSE event into a `LogEvent`, updating `state` as a side-effect.
///
/// Returns `None` for event types that carry no useful signal for Aegon
/// (`ping`, `message_start`, `content_block_stop`, `message_delta`, `message_stop`).
pub fn decode_sse_event(
    event: &SseEvent,
    state: &mut TurnState,
    request_id: &str,
    session_id: Uuid,
) -> Option<LogEvent> {
    match event.event_type.as_str() {
        "content_block_start" => {
            // Register block type so we can label deltas correctly.
            if let Ok(body) = serde_json::from_str::<ContentBlockStart>(&event.data) {
                let is_thinking = body.content_block.block_type == "thinking";
                state.block_is_thinking.insert(body.index, is_thinking);
            }
            None
        }

        "content_block_delta" => {
            let body = serde_json::from_str::<ContentBlockDelta>(&event.data).ok()?;
            let is_thinking = *state.block_is_thinking.get(&body.index).unwrap_or(&false);
            let text = match body.delta.delta_type.as_str() {
                "text_delta" => body.delta.text,
                "thinking_delta" => body.delta.thinking,
                // input_json_delta (tool call arguments) — skip for now.
                _ => return None,
            };
            if text.is_empty() {
                return None;
            }
            Some(LogEvent {
                id: Uuid::new_v4(),
                session_id,
                parent_id: None,
                timestamp: Utc::now(),
                stream: StreamId::Main,
                kind: EventKind::TokenChunk {
                    request_id: request_id.to_owned(),
                    text,
                    is_thinking,
                },
            })
        }

        // All other event types carry no mid-turn token signal.
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sse::SseEvent;

    fn sid() -> Uuid {
        Uuid::nil()
    }

    fn event(event_type: &str, data: &str) -> SseEvent {
        SseEvent {
            event_type: event_type.into(),
            data: data.into(),
        }
    }

    #[test]
    fn content_block_start_registers_text_block() {
        let mut state = TurnState::new();
        let ev = event(
            "content_block_start",
            r#"{"index":0,"content_block":{"type":"text","text":""}}"#,
        );
        let result = decode_sse_event(&ev, &mut state, "req-1", sid());
        assert!(result.is_none(), "content_block_start produces no event");
        assert!(!state.block_is_thinking[&0]);
    }

    #[test]
    fn content_block_start_registers_thinking_block() {
        let mut state = TurnState::new();
        let ev = event(
            "content_block_start",
            r#"{"index":0,"content_block":{"type":"thinking","thinking":""}}"#,
        );
        decode_sse_event(&ev, &mut state, "req-1", sid());
        assert!(state.block_is_thinking[&0]);
    }

    #[test]
    fn text_delta_produces_token_chunk() {
        let mut state = TurnState::new();
        state.block_is_thinking.insert(1, false);
        let ev = event(
            "content_block_delta",
            r#"{"index":1,"delta":{"type":"text_delta","text":"Hello"}}"#,
        );
        let log = decode_sse_event(&ev, &mut state, "req-42", sid()).unwrap();
        match log.kind {
            EventKind::TokenChunk {
                text,
                is_thinking,
                request_id,
            } => {
                assert_eq!(text, "Hello");
                assert!(!is_thinking);
                assert_eq!(request_id, "req-42");
            }
            other => panic!("expected TokenChunk, got {other:?}"),
        }
    }

    #[test]
    fn thinking_delta_sets_is_thinking_true() {
        let mut state = TurnState::new();
        state.block_is_thinking.insert(0, true);
        let ev = event(
            "content_block_delta",
            r#"{"index":0,"delta":{"type":"thinking_delta","thinking":"I wonder..."}}"#,
        );
        let log = decode_sse_event(&ev, &mut state, "req-1", sid()).unwrap();
        match log.kind {
            EventKind::TokenChunk {
                is_thinking, text, ..
            } => {
                assert!(is_thinking);
                assert_eq!(text, "I wonder...");
            }
            other => panic!("expected TokenChunk, got {other:?}"),
        }
    }

    #[test]
    fn empty_text_delta_produces_no_event() {
        let mut state = TurnState::new();
        state.block_is_thinking.insert(0, false);
        let ev = event(
            "content_block_delta",
            r#"{"index":0,"delta":{"type":"text_delta","text":""}}"#,
        );
        assert!(decode_sse_event(&ev, &mut state, "req-1", sid()).is_none());
    }

    #[test]
    fn ping_produces_no_event() {
        let mut state = TurnState::new();
        let ev = event("ping", r#"{"type":"ping"}"#);
        assert!(decode_sse_event(&ev, &mut state, "req-1", sid()).is_none());
    }

    #[test]
    fn message_stop_produces_no_event() {
        let mut state = TurnState::new();
        let ev = event("message_stop", r#"{"type":"message_stop"}"#);
        assert!(decode_sse_event(&ev, &mut state, "req-1", sid()).is_none());
    }
}
