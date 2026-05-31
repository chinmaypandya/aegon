//! Live state for a single Claude Code session.
//!
//! [`SessionState`] is the heart of `aegon-core`. It owns no I/O — callers
//! feed it [`aegon_types::LogEvent`]s one at a time via [`SessionState::ingest`] and read back whatever
//! the dashboard needs through its public fields and methods.

use crate::flow::FlowNode;
use crate::step::{Step, StepStatus};
use crate::token_totals::TokenTotals;
use aegon_types::{EventKind, LogEvent};
use std::collections::HashMap;
use uuid::Uuid;

/// All live state for one Claude Code session.
///
/// Tracks running tool calls, completed step history, cumulative token
/// usage, and the ordered causal flow for visualization. Feed events via
/// [`SessionState::ingest`]; read state through the public fields and helper methods.
#[derive(Debug, Default)]
pub struct SessionState {
    /// Unique session identifier — matches `LogEvent.session_id`.
    pub session_id: Uuid,
    /// Completed steps in arrival order.
    pub steps: Vec<Step>,
    /// Tool calls that have been dispatched but not yet resolved.
    ///
    /// Keyed by `tool_use_id`. Entries are removed when the matching
    /// `ToolResult` arrives.
    pub pending: HashMap<String, Step>,
    /// Ordered causal flow nodes for the flow visualization.
    pub flow: Vec<FlowNode>,
    /// Cumulative token usage across all assistant turns.
    pub token_totals: TokenTotals,
    /// Token usage from the most recent assistant turn only.
    ///
    /// Useful for per-turn gauges that would be meaningless if shown
    /// as a cumulative total across hundreds of turns.
    pub last_turn_usage: Option<aegon_types::TokenUsage>,
    /// Whether the session is waiting for an assistant response.
    ///
    /// Set to `true` when a `UserMessage` arrives; cleared when the
    /// `AssistantMessage` arrives. Drives the "Thinking…" spinner in the
    /// dashboard — the model is reasoning but the JSONL record hasn't been
    /// flushed yet.
    pub awaiting_response: bool,
    /// Auto-generated session title from the `ai-title` record, if seen.
    pub title: Option<String>,
    /// Current session mode — `"normal"` or `"auto"`.
    pub mode: Option<String>,
    /// API errors and retries recorded during the session, most recent last.
    pub errors: Vec<String>,
    /// Total number of events ingested (for display).
    pub event_count: usize,

    // ── extended state ───────────────────────────────────────────────────────
    /// Whether the agent is currently in plan mode.
    pub in_plan_mode: bool,
    /// PRs that were opened from this session (number, url).
    pub pr_links: Vec<(u64, String)>,
    /// Tools currently registered in the deferred tool roster.
    ///
    /// Updated incrementally as `ToolsRegistered` events arrive.
    pub registered_tools: Vec<String>,
    /// Number of background tasks that have completed in this session.
    pub background_tasks_completed: u64,
    /// Total number of API retry attempts observed in this session.
    pub retry_count: u64,
}

impl SessionState {
    /// Create an empty state for the given session.
    pub fn new(session_id: Uuid) -> Self {
        Self {
            session_id,
            ..Default::default()
        }
    }

    /// Ingest one event, updating all derived state.
    ///
    /// This is the only mutation point. Call it for every [`LogEvent`] that
    /// belongs to this session (i.e. `event.session_id == self.session_id`).
    pub fn ingest(&mut self, event: &LogEvent) {
        self.event_count += 1;

        match &event.kind {
            EventKind::ToolCall(tc) => {
                let step = Step {
                    id: tc.id.clone(),
                    tool_name: tc.name.clone(),
                    parent_id: event.parent_id,
                    status: StepStatus::Running {
                        started_at: event.timestamp,
                    },
                };
                self.pending.insert(tc.id.clone(), step);
                self.push_tool_to_flow(tc.name.clone(), event.parent_id);
            }

            EventKind::ToolResult(tr) => {
                if let Some(mut step) = self.pending.remove(&tr.tool_use_id) {
                    let duration_ms = match &step.status {
                        StepStatus::Running { started_at } => {
                            (event.timestamp - started_at).num_milliseconds().max(0) as u64
                        }
                        _ => 0,
                    };
                    step.status = if tr.is_error {
                        StepStatus::Failed { duration_ms }
                    } else {
                        StepStatus::Done { duration_ms }
                    };
                    self.steps.push(step);
                }
            }

            EventKind::AssistantMessage { usage, .. } => {
                if let Some(u) = usage {
                    self.token_totals.add(u);
                    self.last_turn_usage = Some(u.clone());
                }
                self.awaiting_response = false;
                self.push_flow(FlowNode::Assistant);
            }

            EventKind::TokenUsage(u) => {
                self.token_totals.add(u);
            }

            EventKind::Thinking { .. } => {
                self.push_flow(FlowNode::Thinking);
            }

            EventKind::UserMessage { .. } => {
                self.drain_orphaned_pending(event.timestamp);
                self.awaiting_response = true;
                self.push_flow(FlowNode::Human);
            }

            EventKind::SessionTitle { title } => {
                self.title = Some(title.clone());
            }

            EventKind::SessionMode { mode } => {
                self.mode = Some(mode.clone());
            }

            EventKind::SystemError {
                message,
                retry_attempt,
                ..
            } => {
                // Only record a new error episode when this is not a retry leg.
                // Retry legs (retry_attempt.is_some()) share a root cause with
                // the first failure — counting each leg separately would inflate
                // errors.len() and the dashboard badge to N for one incident.
                if retry_attempt.is_none() {
                    self.errors.push(message.clone());
                }
                // Count retry legs so the dashboard can say "N retries" even when
                // the error itself was deduplicated above.
                if retry_attempt.is_some() {
                    self.retry_count += 1;
                }
            }

            EventKind::PlanModeEntered { .. } => {
                self.in_plan_mode = true;
            }

            EventKind::PlanModeExited { .. } => {
                self.in_plan_mode = false;
            }

            EventKind::PrLinked {
                pr_number, pr_url, ..
            } => {
                self.pr_links.push((*pr_number, pr_url.clone()));
            }

            EventKind::ToolsRegistered { added, removed } => {
                for name in removed {
                    self.registered_tools.retain(|t| t != name);
                }
                for name in added {
                    if !self.registered_tools.contains(name) {
                        self.registered_tools.push(name.clone());
                    }
                }
            }

            EventKind::BackgroundTaskResult { .. } => {
                self.background_tasks_completed += 1;
            }

            // These events are surfaced in the feed but require no derived state update.
            EventKind::QueueOperation { .. }
            | EventKind::LastPrompt { .. }
            | EventKind::FileSnapshot { .. }
            | EventKind::SkillsLoaded { .. }
            | EventKind::TodoUpdated { .. }
            | EventKind::HookOutput { .. }
            | EventKind::FileEdited { .. }
            | EventKind::DateChange { .. }
            | EventKind::PermissionsUpdated { .. }
            | EventKind::Unknown => {}
        }
    }

    /// All steps currently waiting for a result, in arrival order.
    pub fn running_steps(&self) -> Vec<&Step> {
        self.pending.values().collect()
    }

    /// Completed and failed steps, most recent last.
    pub fn completed_steps(&self) -> &[Step] {
        &self.steps
    }

    /// Whether any tool calls are currently in flight.
    pub fn is_active(&self) -> bool {
        !self.pending.is_empty()
    }

    // ── private helpers ──────────────────────────────────────────────────────

    /// Move all still-pending steps to completed as `Done`.
    ///
    /// Called when a new human turn arrives, which means the previous round-trip
    /// is finished and any unmatched tool calls were orphaned (aborted session,
    /// reordered events, etc.).
    fn drain_orphaned_pending(&mut self, now: chrono::DateTime<chrono::Utc>) {
        let orphans: Vec<String> = self.pending.keys().cloned().collect();
        for id in orphans {
            if let Some(mut step) = self.pending.remove(&id) {
                let duration_ms = match &step.status {
                    StepStatus::Running { started_at } => {
                        (now - started_at).num_milliseconds().max(0) as u64
                    }
                    _ => 0,
                };
                step.status = StepStatus::Done { duration_ms };
                self.steps.push(step);
            }
        }
    }

    /// Add a tool call to the flow, merging into an existing ToolGroup when
    /// the previous node shares the same parent_id (parallel dispatch).
    fn push_tool_to_flow(&mut self, tool_name: String, parent_id: Option<Uuid>) {
        if let Some(FlowNode::ToolGroup(names)) = self.flow.last_mut() {
            if parent_id.is_some() && self.pending.values().any(|s| s.parent_id == parent_id) {
                names.push(tool_name);
                return;
            }
        }
        self.flow.push(FlowNode::ToolGroup(vec![tool_name]));
    }

    fn push_flow(&mut self, node: FlowNode) {
        if self.flow.last() != Some(&node) {
            self.flow.push(node);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aegon_types::{EventKind, LogEvent, StreamId, TokenUsage, ToolCall, ToolResult};
    use uuid::Uuid;

    fn session() -> SessionState {
        SessionState::new(Uuid::nil())
    }

    fn make_event(kind: EventKind) -> LogEvent {
        LogEvent {
            id: Uuid::new_v4(),
            session_id: Uuid::nil(),
            parent_id: None,
            timestamp: chrono::Utc::now(),
            stream: StreamId::Main,
            kind,
        }
    }

    fn system_error(retry_attempt: Option<u32>) -> EventKind {
        EventKind::SystemError {
            message: "Connection error.".into(),
            code: Some("ECONNRESET".into()),
            retry_attempt,
            max_retries: Some(10),
            retry_in_ms: Some(500.0),
        }
    }

    // ── SystemError deduplication ────────────────────────────────────────────

    #[test]
    fn system_error_without_retry_pushes_to_errors() {
        let mut s = session();
        s.ingest(&make_event(system_error(None)));
        assert_eq!(s.errors.len(), 1);
        assert_eq!(s.retry_count, 0);
    }

    #[test]
    fn system_error_retry_legs_do_not_inflate_errors_vec() {
        let mut s = session();
        // Three retry legs for one error episode.
        s.ingest(&make_event(system_error(Some(1))));
        s.ingest(&make_event(system_error(Some(2))));
        s.ingest(&make_event(system_error(Some(3))));
        // errors should stay empty — no non-retry error was emitted.
        assert_eq!(s.errors.len(), 0, "retry legs must not inflate errors");
        assert_eq!(s.retry_count, 3);
    }

    #[test]
    fn system_error_episode_plus_retries_counts_separately() {
        let mut s = session();
        // First record is the root error (no retry_attempt).
        s.ingest(&make_event(system_error(None)));
        // Two retry legs follow.
        s.ingest(&make_event(system_error(Some(1))));
        s.ingest(&make_event(system_error(Some(2))));
        assert_eq!(s.errors.len(), 1, "only the root error counts in errors");
        assert_eq!(s.retry_count, 2);
    }

    #[test]
    fn two_independent_errors_each_push_to_errors() {
        let mut s = session();
        s.ingest(&make_event(system_error(None)));
        s.ingest(&make_event(system_error(None)));
        assert_eq!(s.errors.len(), 2);
        assert_eq!(s.retry_count, 0);
    }

    // ── Plan mode ────────────────────────────────────────────────────────────

    #[test]
    fn plan_mode_entered_sets_in_plan_mode() {
        let mut s = session();
        assert!(!s.in_plan_mode);
        s.ingest(&make_event(EventKind::PlanModeEntered {
            plan_file: Some("/tmp/plan.md".into()),
            plan_exists: false,
        }));
        assert!(s.in_plan_mode);
    }

    #[test]
    fn plan_mode_exited_clears_in_plan_mode() {
        let mut s = session();
        s.ingest(&make_event(EventKind::PlanModeEntered {
            plan_file: None,
            plan_exists: false,
        }));
        s.ingest(&make_event(EventKind::PlanModeExited { plan_file: None }));
        assert!(!s.in_plan_mode);
    }

    #[test]
    fn plan_mode_toggle_is_idempotent() {
        let mut s = session();
        s.ingest(&make_event(EventKind::PlanModeEntered {
            plan_file: None,
            plan_exists: false,
        }));
        s.ingest(&make_event(EventKind::PlanModeEntered {
            plan_file: None,
            plan_exists: true,
        }));
        assert!(s.in_plan_mode);
        s.ingest(&make_event(EventKind::PlanModeExited { plan_file: None }));
        s.ingest(&make_event(EventKind::PlanModeExited { plan_file: None }));
        assert!(!s.in_plan_mode);
    }

    // ── PR links ─────────────────────────────────────────────────────────────

    #[test]
    fn pr_linked_appends_to_pr_links() {
        let mut s = session();
        assert!(s.pr_links.is_empty());
        s.ingest(&make_event(EventKind::PrLinked {
            pr_number: 42,
            pr_url: "https://github.com/org/repo/pull/42".into(),
            repository: "org/repo".into(),
        }));
        assert_eq!(s.pr_links.len(), 1);
        assert_eq!(s.pr_links[0].0, 42);
        assert_eq!(s.pr_links[0].1, "https://github.com/org/repo/pull/42");
    }

    #[test]
    fn multiple_prs_all_accumulated() {
        let mut s = session();
        s.ingest(&make_event(EventKind::PrLinked {
            pr_number: 1,
            pr_url: "https://github.com/org/repo/pull/1".into(),
            repository: "org/repo".into(),
        }));
        s.ingest(&make_event(EventKind::PrLinked {
            pr_number: 2,
            pr_url: "https://github.com/org/repo/pull/2".into(),
            repository: "org/repo".into(),
        }));
        assert_eq!(s.pr_links.len(), 2);
        assert_eq!(s.pr_links[1].0, 2);
    }

    // ── Tool roster ──────────────────────────────────────────────────────────

    #[test]
    fn tools_registered_adds_new_tools() {
        let mut s = session();
        s.ingest(&make_event(EventKind::ToolsRegistered {
            added: vec!["TodoWrite".into(), "WebFetch".into()],
            removed: vec![],
        }));
        assert_eq!(s.registered_tools.len(), 2);
        assert!(s.registered_tools.contains(&"TodoWrite".to_string()));
        assert!(s.registered_tools.contains(&"WebFetch".to_string()));
    }

    #[test]
    fn tools_registered_removes_existing_tools() {
        let mut s = session();
        s.ingest(&make_event(EventKind::ToolsRegistered {
            added: vec!["TodoWrite".into(), "WebFetch".into()],
            removed: vec![],
        }));
        s.ingest(&make_event(EventKind::ToolsRegistered {
            added: vec![],
            removed: vec!["WebFetch".into()],
        }));
        assert_eq!(s.registered_tools.len(), 1);
        assert!(s.registered_tools.contains(&"TodoWrite".to_string()));
        assert!(!s.registered_tools.contains(&"WebFetch".to_string()));
    }

    #[test]
    fn tools_registered_does_not_duplicate_existing_tool() {
        let mut s = session();
        s.ingest(&make_event(EventKind::ToolsRegistered {
            added: vec!["TodoWrite".into()],
            removed: vec![],
        }));
        s.ingest(&make_event(EventKind::ToolsRegistered {
            added: vec!["TodoWrite".into()],
            removed: vec![],
        }));
        assert_eq!(
            s.registered_tools.len(),
            1,
            "duplicate add must not grow the list"
        );
    }

    #[test]
    fn tools_registered_remove_nonexistent_is_harmless() {
        let mut s = session();
        s.ingest(&make_event(EventKind::ToolsRegistered {
            added: vec![],
            removed: vec!["NonExistent".into()],
        }));
        assert!(s.registered_tools.is_empty());
    }

    // ── Background tasks ─────────────────────────────────────────────────────

    #[test]
    fn background_task_result_increments_counter() {
        let mut s = session();
        assert_eq!(s.background_tasks_completed, 0);
        s.ingest(&make_event(EventKind::BackgroundTaskResult {
            task_id: Some("abc".into()),
            status: "completed".into(),
            summary: "Build succeeded".into(),
        }));
        assert_eq!(s.background_tasks_completed, 1);
        s.ingest(&make_event(EventKind::BackgroundTaskResult {
            task_id: Some("def".into()),
            status: "failed".into(),
            summary: "Tests failed".into(),
        }));
        assert_eq!(s.background_tasks_completed, 2);
    }

    // ── Display-only events (no state mutation) ───────────────────────────────

    #[test]
    fn display_only_events_do_not_mutate_derived_state() {
        let mut s = session();
        let before_count = s.event_count;

        for kind in [
            EventKind::QueueOperation {
                operation: "enqueue".into(),
            },
            EventKind::LastPrompt {
                content: "hello".into(),
            },
            EventKind::FileSnapshot { is_update: false },
            EventKind::SkillsLoaded {
                content: "- foo: bar".into(),
            },
            EventKind::TodoUpdated { item_count: 3 },
            EventKind::HookOutput {
                hook_name: "PostToolUse:Edit".into(),
                tool_use_id: None,
                content: vec![],
            },
            EventKind::FileEdited {
                path: "/tmp/foo.rs".into(),
                snippet: None,
            },
            EventKind::DateChange {
                new_date: "2026-06-01".into(),
            },
            EventKind::PermissionsUpdated {
                allowed_tools: vec![],
            },
        ] {
            s.ingest(&make_event(kind));
        }

        // event_count increments, but nothing else changes.
        assert_eq!(s.event_count, before_count + 9);
        assert!(s.errors.is_empty());
        assert_eq!(s.retry_count, 0);
        assert!(!s.in_plan_mode);
        assert!(s.pr_links.is_empty());
        assert!(s.registered_tools.is_empty());
        assert_eq!(s.background_tasks_completed, 0);
    }

    // ── event_count ──────────────────────────────────────────────────────────

    #[test]
    fn event_count_increments_for_every_ingested_event() {
        let mut s = session();
        assert_eq!(s.event_count, 0);
        s.ingest(&make_event(EventKind::QueueOperation {
            operation: "enqueue".into(),
        }));
        s.ingest(&make_event(EventKind::DateChange {
            new_date: "2026-06-01".into(),
        }));
        assert_eq!(s.event_count, 2);
    }

    // ── TokenUsage passthrough ───────────────────────────────────────────────

    #[test]
    fn token_usage_event_accumulates_into_totals() {
        let mut s = session();
        s.ingest(&make_event(EventKind::TokenUsage(TokenUsage {
            input_tokens: 10,
            output_tokens: 5,
            cache_read_input_tokens: 2,
            cache_creation_input_tokens: 0,
        })));
        assert_eq!(s.token_totals.input, 10);
        assert_eq!(s.token_totals.output, 5);
    }

    // ── is_active ────────────────────────────────────────────────────────────

    #[test]
    fn is_active_true_while_tool_call_pending() {
        let mut s = session();
        s.ingest(&make_event(EventKind::ToolCall(ToolCall {
            id: "tc1".into(),
            name: "Bash".into(),
            input: serde_json::json!({}),
        })));
        assert!(s.is_active());
    }

    #[test]
    fn is_active_false_after_tool_result_resolves_pending() {
        let mut s = session();
        s.ingest(&make_event(EventKind::ToolCall(ToolCall {
            id: "tc1".into(),
            name: "Bash".into(),
            input: serde_json::json!({}),
        })));
        s.ingest(&make_event(EventKind::ToolResult(ToolResult {
            tool_use_id: "tc1".into(),
            content: "file.txt".into(),
            is_error: false,
            metadata: None,
        })));
        assert!(!s.is_active());
    }
}
