//! Adapter for Claude Code JSONL session files.
//!
//! Parses the raw records Claude writes to `~/.claude/projects/**/*.jsonl`
//! and normalises them into `Vec<LogEvent>`. One raw line may expand to
//! multiple events (e.g. an assistant turn with several tool calls).

mod raw;

use crate::Adapter;
use aegon_types::{
    EventKind, LogEvent, Result, StreamId, TokenUsage, ToolCall, ToolMetadata, ToolResult,
};
use chrono::DateTime;
use raw::{RawAttachment, RawContent, RawRecord, RawToolUseResult};
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
        let stream = if record.is_sidechain {
            StreamId::Sidechain
        } else {
            StreamId::Main
        };

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

        let tool_use_result = record.tool_use_result;
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
                            RawContent::Image => text_parts.push("[image]".into()),
                            RawContent::ToolUse { id, name, input } => {
                                events.push(make_event(
                                    Uuid::new_v4(),
                                    session_id,
                                    Some(event_id),
                                    timestamp,
                                    stream,
                                    EventKind::ToolCall(ToolCall { id, name, input }),
                                ));
                            }
                            RawContent::Thinking {
                                thinking,
                                signature,
                            } => {
                                events.push(make_event(
                                    Uuid::new_v4(),
                                    session_id,
                                    Some(event_id),
                                    timestamp,
                                    stream,
                                    EventKind::Thinking {
                                        text: thinking,
                                        signature,
                                    },
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
                            stream,
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
                                    stream,
                                    EventKind::UserMessage { content: text },
                                ));
                            }
                            RawContent::ToolResult {
                                tool_use_id,
                                content,
                                is_error,
                            } => {
                                let metadata = tool_use_result.as_ref().map(build_tool_metadata);
                                events.push(make_event(
                                    Uuid::new_v4(),
                                    session_id,
                                    Some(event_id),
                                    timestamp,
                                    stream,
                                    EventKind::ToolResult(ToolResult {
                                        tool_use_id,
                                        content: flatten_content(content),
                                        is_error: is_error.unwrap_or(false),
                                        metadata,
                                    }),
                                ));
                            }
                            _ => {}
                        }
                    }
                }
            }

            Some("ai-title") => {
                if let Some(title) = record.ai_title {
                    events.push(make_event(
                        event_id,
                        session_id,
                        parent_id,
                        timestamp,
                        stream,
                        EventKind::SessionTitle { title },
                    ));
                }
            }

            Some("mode") => {
                if let Some(mode) = record.mode {
                    events.push(make_event(
                        event_id,
                        session_id,
                        parent_id,
                        timestamp,
                        stream,
                        EventKind::SessionMode { mode },
                    ));
                }
            }

            Some("system") => {
                if let Some(err) = record.error {
                    let message = err
                        .formatted
                        .or(err.message)
                        .unwrap_or_else(|| "Unknown system error".into());
                    let code = err.connection.and_then(|c| c.code);
                    events.push(make_event(
                        event_id,
                        session_id,
                        parent_id,
                        timestamp,
                        stream,
                        EventKind::SystemError {
                            message,
                            code,
                            retry_attempt: record.retry_attempt,
                            max_retries: record.max_retries,
                            retry_in_ms: record.retry_in_ms,
                        },
                    ));
                }
            }

            Some("queue-operation") => {
                if let Some(operation) = record.operation {
                    events.push(make_event(
                        event_id,
                        session_id,
                        parent_id,
                        timestamp,
                        stream,
                        EventKind::QueueOperation { operation },
                    ));
                }
            }

            Some("pr-link") => {
                if let (Some(pr_number), Some(pr_url)) = (record.pr_number, record.pr_url) {
                    let repository = record
                        .pr_repository
                        .unwrap_or_else(|| "unknown/repo".into());
                    events.push(make_event(
                        event_id,
                        session_id,
                        parent_id,
                        timestamp,
                        stream,
                        EventKind::PrLinked {
                            pr_number,
                            pr_url,
                            repository,
                        },
                    ));
                }
            }

            Some("last-prompt") => {
                if let Some(content) = record.last_prompt {
                    events.push(make_event(
                        event_id,
                        session_id,
                        parent_id,
                        timestamp,
                        stream,
                        EventKind::LastPrompt { content },
                    ));
                }
            }

            Some("file-history-snapshot") => {
                events.push(make_event(
                    event_id,
                    session_id,
                    parent_id,
                    timestamp,
                    stream,
                    EventKind::FileSnapshot {
                        is_update: record.is_snapshot_update,
                    },
                ));
            }

            Some("attachment") => {
                if let Some(attachment) = record.attachment {
                    let kind = attachment_to_kind(attachment);
                    if let Some(kind) = kind {
                        events.push(make_event(
                            event_id, session_id, parent_id, timestamp, stream, kind,
                        ));
                    }
                }
            }

            _ => {}
        }

        Ok(events)
    }
}

/// Convert a parsed `RawAttachment` into an `EventKind`, or `None` for noise-only attachments.
fn attachment_to_kind(attachment: RawAttachment) -> Option<EventKind> {
    match attachment {
        RawAttachment::DeferredToolsDelta {
            added_names,
            removed_names,
        } => Some(EventKind::ToolsRegistered {
            added: added_names,
            removed: removed_names,
        }),

        RawAttachment::SkillListing { content } => Some(EventKind::SkillsLoaded { content }),

        RawAttachment::PlanMode {
            plan_file_path,
            plan_exists,
        } => Some(EventKind::PlanModeEntered {
            plan_file: plan_file_path,
            plan_exists,
        }),

        RawAttachment::PlanModeExit { plan_file_path } => Some(EventKind::PlanModeExited {
            plan_file: plan_file_path,
        }),

        RawAttachment::TodoReminder { item_count } => Some(EventKind::TodoUpdated { item_count }),

        RawAttachment::HookAdditionalContext {
            content,
            hook_name,
            tool_use_id,
        } => Some(EventKind::HookOutput {
            hook_name,
            tool_use_id,
            content,
        }),

        RawAttachment::EditedTextFile { filename, snippet } => Some(EventKind::FileEdited {
            path: filename,
            snippet,
        }),

        RawAttachment::DateChange { new_date } => Some(EventKind::DateChange { new_date }),

        RawAttachment::QueuedCommand {
            prompt,
            command_mode,
        } => {
            // task-notification payloads embed structured XML in `prompt`.
            // Other command modes (inline commands, etc.) are harness-internal
            // and not yet worth surfacing.
            if command_mode == "task-notification" {
                let task_id = extract_xml_tag(&prompt, "task-id");
                let status = extract_xml_tag(&prompt, "status").unwrap_or_else(|| "unknown".into());
                let summary = extract_xml_tag(&prompt, "summary")
                    .unwrap_or_else(|| "Background task completed".into());
                Some(EventKind::BackgroundTaskResult {
                    task_id,
                    status,
                    summary,
                })
            } else {
                None
            }
        }

        RawAttachment::CommandPermissions { allowed_tools } => {
            Some(EventKind::PermissionsUpdated { allowed_tools })
        }

        RawAttachment::Other => None,
    }
}

/// Extract the text content of a simple XML tag (no attributes, no nesting).
fn extract_xml_tag(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = xml.find(&open)? + open.len();
    let end = xml.find(&close)?;
    if start <= end {
        Some(xml[start..end].trim().to_owned())
    } else {
        None
    }
}

fn make_event(
    id: Uuid,
    session_id: Uuid,
    parent_id: Option<Uuid>,
    timestamp: chrono::DateTime<chrono::Utc>,
    stream: StreamId,
    kind: EventKind,
) -> LogEvent {
    LogEvent {
        id,
        session_id,
        parent_id,
        timestamp,
        stream,
        kind,
    }
}

fn build_tool_metadata(raw: &RawToolUseResult) -> ToolMetadata {
    ToolMetadata {
        stdout: raw.stdout.clone(),
        stderr: raw.stderr.clone(),
        interrupted: raw.interrupted,
        is_image: raw.is_image,
        file_path: raw.file.as_ref().and_then(|f| f.file_path.clone()),
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

    // ── existing record types ────────────────────────────────────────────────

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
        assert_eq!(events.len(), 2);
        let has_tool_call = events
            .iter()
            .any(|e| matches!(&e.kind, EventKind::ToolCall(tc) if tc.name == "Bash"));
        let has_message = events.iter().any(
            |e| matches!(&e.kind, EventKind::AssistantMessage { content, .. } if content.contains("I will run bash")),
        );
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

    #[test]
    fn thinking_block_produces_thinking_event() {
        let line = r#"{
            "type":"assistant",
            "uuid":"00000000-0000-0000-0000-000000000006",
            "timestamp":"2026-05-29T14:00:00Z",
            "message":{
                "role":"assistant",
                "content":[
                    {"type":"thinking","thinking":"I should use ls","signature":"sig123"},
                    {"type":"text","text":"ok"}
                ]
            }
        }"#;
        let events = adapter().parse_line(line).unwrap();
        let thinking = events
            .iter()
            .find(|e| matches!(&e.kind, EventKind::Thinking { .. }));
        assert!(thinking.is_some(), "expected a Thinking event");
        match &thinking.unwrap().kind {
            EventKind::Thinking { text, signature } => {
                assert_eq!(text, "I should use ls");
                assert_eq!(signature.as_deref(), Some("sig123"));
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn sidechain_record_sets_stream_to_sidechain() {
        let line = r#"{
            "type":"assistant",
            "isSidechain":true,
            "uuid":"00000000-0000-0000-0000-000000000007",
            "timestamp":"2026-05-29T14:00:00Z",
            "message":{"role":"assistant","content":[{"type":"text","text":"sub"}]}
        }"#;
        let events = adapter().parse_line(line).unwrap();
        assert!(!events.is_empty());
        assert_eq!(events[0].stream, StreamId::Sidechain);
    }

    #[test]
    fn main_chain_record_sets_stream_to_main() {
        let line = r#"{
            "type":"assistant",
            "isSidechain":false,
            "uuid":"00000000-0000-0000-0000-000000000008",
            "timestamp":"2026-05-29T14:00:00Z",
            "message":{"role":"assistant","content":[{"type":"text","text":"main"}]}
        }"#;
        let events = adapter().parse_line(line).unwrap();
        assert!(!events.is_empty());
        assert_eq!(events[0].stream, StreamId::Main);
    }

    #[test]
    fn tool_result_with_tool_use_result_metadata_is_enriched() {
        let line = r#"{
            "type":"user",
            "uuid":"00000000-0000-0000-0000-000000000009",
            "timestamp":"2026-05-29T14:00:00Z",
            "toolUseResult":{"stdout":"file.txt","stderr":"","interrupted":false,"isImage":false},
            "message":{"content":[{
                "type":"tool_result",
                "tool_use_id":"t1",
                "content":"file.txt"
            }]}
        }"#;
        let events = adapter().parse_line(line).unwrap();
        if let EventKind::ToolResult(tr) = &events[0].kind {
            let meta = tr.metadata.as_ref().expect("metadata should be present");
            assert_eq!(meta.stdout.as_deref(), Some("file.txt"));
            assert!(!meta.interrupted);
        } else {
            panic!("expected ToolResult");
        }
    }

    #[test]
    fn object_valued_usage_fields_do_not_crash_parser() {
        let line = r#"{
            "type":"assistant",
            "uuid":"00000000-0000-0000-0000-000000000010",
            "timestamp":"2026-05-29T14:00:00Z",
            "message":{
                "role":"assistant",
                "content":[{"type":"text","text":"ok"}],
                "usage":{
                    "input_tokens":5,
                    "output_tokens":2,
                    "cache_creation":{"some_nested":"object"},
                    "server_tool_use":{"web_search_requests":0,"web_fetch_requests":0}
                }
            }
        }"#;
        let events = adapter().parse_line(line).unwrap();
        assert!(
            !events.is_empty(),
            "should still produce events despite unknown object fields"
        );
    }

    // ── new record types ─────────────────────────────────────────────────────

    #[test]
    fn queue_operation_enqueue_produces_event() {
        let line = r#"{"type":"queue-operation","operation":"enqueue","timestamp":"2026-05-29T14:00:00Z","sessionId":"00000000-0000-0000-0000-000000000001","uuid":"00000000-0000-0000-0000-000000000011"}"#;
        let events = adapter().parse_line(line).unwrap();
        assert_eq!(events.len(), 1);
        assert!(
            matches!(&events[0].kind, EventKind::QueueOperation { operation } if operation == "enqueue")
        );
    }

    #[test]
    fn queue_operation_dequeue_produces_event() {
        let line = r#"{"type":"queue-operation","operation":"dequeue","timestamp":"2026-05-29T14:00:00Z","sessionId":"00000000-0000-0000-0000-000000000001","uuid":"00000000-0000-0000-0000-000000000012"}"#;
        let events = adapter().parse_line(line).unwrap();
        assert_eq!(events.len(), 1);
        assert!(
            matches!(&events[0].kind, EventKind::QueueOperation { operation } if operation == "dequeue")
        );
    }

    #[test]
    fn pr_link_produces_pr_linked_event() {
        let line = r#"{
            "type":"pr-link",
            "uuid":"00000000-0000-0000-0000-000000000013",
            "timestamp":"2026-05-29T14:00:00Z",
            "sessionId":"00000000-0000-0000-0000-000000000001",
            "prNumber":4,
            "prUrl":"https://github.com/org/repo/pull/4",
            "prRepository":"org/repo"
        }"#;
        let events = adapter().parse_line(line).unwrap();
        assert_eq!(events.len(), 1);
        match &events[0].kind {
            EventKind::PrLinked {
                pr_number,
                pr_url,
                repository,
            } => {
                assert_eq!(*pr_number, 4);
                assert_eq!(pr_url, "https://github.com/org/repo/pull/4");
                assert_eq!(repository, "org/repo");
            }
            other => panic!("expected PrLinked, got {other:?}"),
        }
    }

    #[test]
    fn last_prompt_produces_last_prompt_event() {
        let line = r#"{
            "type":"last-prompt",
            "uuid":"00000000-0000-0000-0000-000000000014",
            "timestamp":"2026-05-29T14:00:00Z",
            "sessionId":"00000000-0000-0000-0000-000000000001",
            "lastPrompt":"do the thing",
            "leafUuid":"00000000-0000-0000-0000-000000000099"
        }"#;
        let events = adapter().parse_line(line).unwrap();
        assert_eq!(events.len(), 1);
        assert!(
            matches!(&events[0].kind, EventKind::LastPrompt { content } if content == "do the thing")
        );
    }

    #[test]
    fn file_history_snapshot_produces_file_snapshot_event() {
        let line = r#"{
            "type":"file-history-snapshot",
            "uuid":"00000000-0000-0000-0000-000000000015",
            "timestamp":"2026-05-29T14:00:00Z",
            "sessionId":"00000000-0000-0000-0000-000000000001",
            "messageId":"00000000-0000-0000-0000-000000000020",
            "snapshot":{"messageId":"00000000-0000-0000-0000-000000000020","trackedFileBackups":{},"timestamp":"2026-05-29T14:00:00Z"},
            "isSnapshotUpdate":false
        }"#;
        let events = adapter().parse_line(line).unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0].kind,
            EventKind::FileSnapshot { is_update: false }
        ));
    }

    #[test]
    fn system_error_with_retry_fields_parsed() {
        let line = r#"{
            "type":"system",
            "uuid":"00000000-0000-0000-0000-000000000016",
            "timestamp":"2026-05-29T14:00:00Z",
            "sessionId":"00000000-0000-0000-0000-000000000001",
            "subtype":"api_error",
            "level":"error",
            "error":{"message":"Connection error.","formatted":"Unable to connect (ECONNRESET)","connection":{"code":"ECONNRESET"}},
            "retryInMs":534.4,
            "retryAttempt":1,
            "maxRetries":10
        }"#;
        let events = adapter().parse_line(line).unwrap();
        assert_eq!(events.len(), 1);
        match &events[0].kind {
            EventKind::SystemError {
                code,
                retry_attempt,
                max_retries,
                retry_in_ms,
                ..
            } => {
                assert_eq!(code.as_deref(), Some("ECONNRESET"));
                assert_eq!(*retry_attempt, Some(1));
                assert_eq!(*max_retries, Some(10));
                assert!(retry_in_ms.is_some());
            }
            other => panic!("expected SystemError, got {other:?}"),
        }
    }

    #[test]
    fn attachment_deferred_tools_delta_produces_tools_registered() {
        let line = r#"{
            "type":"attachment",
            "uuid":"00000000-0000-0000-0000-000000000017",
            "timestamp":"2026-05-29T14:00:00Z",
            "sessionId":"00000000-0000-0000-0000-000000000001",
            "attachment":{
                "type":"deferred_tools_delta",
                "addedNames":["TodoWrite","WebFetch"],
                "addedLines":["TodoWrite","WebFetch"],
                "removedNames":[]
            }
        }"#;
        let events = adapter().parse_line(line).unwrap();
        assert_eq!(events.len(), 1);
        match &events[0].kind {
            EventKind::ToolsRegistered { added, removed } => {
                assert_eq!(added, &["TodoWrite", "WebFetch"]);
                assert!(removed.is_empty());
            }
            other => panic!("expected ToolsRegistered, got {other:?}"),
        }
    }

    #[test]
    fn attachment_plan_mode_produces_plan_mode_entered() {
        let line = r#"{
            "type":"attachment",
            "uuid":"00000000-0000-0000-0000-000000000018",
            "timestamp":"2026-05-29T14:00:00Z",
            "sessionId":"00000000-0000-0000-0000-000000000001",
            "attachment":{
                "type":"plan_mode",
                "reminderType":"full",
                "isSubAgent":false,
                "planFilePath":"/tmp/my-plan.md",
                "planExists":false
            }
        }"#;
        let events = adapter().parse_line(line).unwrap();
        assert_eq!(events.len(), 1);
        match &events[0].kind {
            EventKind::PlanModeEntered {
                plan_file,
                plan_exists,
            } => {
                assert_eq!(plan_file.as_deref(), Some("/tmp/my-plan.md"));
                assert!(!plan_exists);
            }
            other => panic!("expected PlanModeEntered, got {other:?}"),
        }
    }

    #[test]
    fn attachment_hook_context_produces_hook_output() {
        let line = r#"{
            "type":"attachment",
            "uuid":"00000000-0000-0000-0000-000000000019",
            "timestamp":"2026-05-29T14:00:00Z",
            "sessionId":"00000000-0000-0000-0000-000000000001",
            "attachment":{
                "type":"hook_additional_context",
                "content":["<ide_diagnostics>error here</ide_diagnostics>"],
                "hookName":"PostToolUse:Edit",
                "toolUseID":"toolu_abc123",
                "hookEvent":"PostToolUse"
            }
        }"#;
        let events = adapter().parse_line(line).unwrap();
        assert_eq!(events.len(), 1);
        match &events[0].kind {
            EventKind::HookOutput {
                hook_name,
                tool_use_id,
                content,
            } => {
                assert_eq!(hook_name, "PostToolUse:Edit");
                assert_eq!(tool_use_id.as_deref(), Some("toolu_abc123"));
                assert_eq!(content.len(), 1);
            }
            other => panic!("expected HookOutput, got {other:?}"),
        }
    }

    #[test]
    fn attachment_task_notification_extracts_summary() {
        let prompt = "<task-notification>\n<task-id>abc123</task-id>\n<status>completed</status>\n<summary>Build succeeded (exit code 0)</summary>\n</task-notification>";
        let line = format!(
            r#"{{
            "type":"attachment",
            "uuid":"00000000-0000-0000-0000-000000000020",
            "timestamp":"2026-05-29T14:00:00Z",
            "sessionId":"00000000-0000-0000-0000-000000000001",
            "attachment":{{
                "type":"queued_command",
                "prompt":{prompt:?},
                "commandMode":"task-notification"
            }}
        }}"#
        );
        let events = adapter().parse_line(&line).unwrap();
        assert_eq!(events.len(), 1);
        match &events[0].kind {
            EventKind::BackgroundTaskResult {
                task_id,
                status,
                summary,
            } => {
                assert_eq!(task_id.as_deref(), Some("abc123"));
                assert_eq!(status, "completed");
                assert!(summary.contains("Build succeeded"));
            }
            other => panic!("expected BackgroundTaskResult, got {other:?}"),
        }
    }

    #[test]
    fn attachment_unknown_subtype_produces_no_events() {
        let line = r#"{
            "type":"attachment",
            "uuid":"00000000-0000-0000-0000-000000000021",
            "timestamp":"2026-05-29T14:00:00Z",
            "sessionId":"00000000-0000-0000-0000-000000000001",
            "attachment":{"type":"some_future_type","data":"irrelevant"}
        }"#;
        let events = adapter().parse_line(line).unwrap();
        assert!(
            events.is_empty(),
            "unknown attachment types should produce no events"
        );
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

    // ── extract_xml_tag ──────────────────────────────────────────────────────

    #[test]
    fn extract_xml_tag_finds_simple_tag() {
        let xml = "<status>completed</status>";
        assert_eq!(extract_xml_tag(xml, "status").as_deref(), Some("completed"));
    }

    #[test]
    fn extract_xml_tag_returns_none_for_missing_tag() {
        let xml = "<other>value</other>";
        assert_eq!(extract_xml_tag(xml, "status"), None);
    }
}
