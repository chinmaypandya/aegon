//! Live state for a single Claude Code session.
//!
//! [`SessionState`] is the heart of `aegon-core`. It owns no I/O — callers
//! feed it [`LogEvent`]s one at a time via [`ingest`] and read back whatever
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
/// [`ingest`]; read state through the public fields and helper methods.
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
    /// Auto-generated session title from the `ai-title` record, if seen.
    pub title: Option<String>,
    /// Total number of events ingested (for display).
    pub event_count: usize,
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
                }
                self.push_flow(FlowNode::Assistant);
            }

            EventKind::TokenUsage(u) => {
                self.token_totals.add(u);
            }

            EventKind::Thinking { .. } => {
                self.push_flow(FlowNode::Thinking);
            }

            EventKind::UserMessage { .. } => {
                self.push_flow(FlowNode::Human);
            }

            EventKind::Unknown => {}
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

    /// Add a tool call to the flow, merging into an existing ToolGroup when
    /// the previous node shares the same parent_id (parallel dispatch).
    fn push_tool_to_flow(&mut self, tool_name: String, parent_id: Option<Uuid>) {
        // If the last flow node is a ToolGroup whose members share this
        // parent_id, merge into it (parallel branch).
        if let Some(FlowNode::ToolGroup(names)) = self.flow.last_mut() {
            // We detect parallelism by checking that the previous ToolCall
            // was also dispatched from the same parent. Since we don't store
            // the parent_id of the group itself, we use a heuristic: if the
            // last flow node is a ToolGroup and the new call has the same
            // parent_id as the current pending calls, it's parallel.
            if parent_id.is_some() && self.pending.values().any(|s| s.parent_id == parent_id) {
                names.push(tool_name);
                return;
            }
        }
        self.flow.push(FlowNode::ToolGroup(vec![tool_name]));
    }

    fn push_flow(&mut self, node: FlowNode) {
        // De-duplicate consecutive identical nodes (e.g. multiple UserMessage
        // blocks in the same turn shouldn't create duplicate Human nodes).
        if self.flow.last() != Some(&node) {
            self.flow.push(node);
        }
    }
}
