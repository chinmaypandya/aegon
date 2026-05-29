//! A single tool-call step and its lifecycle status.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// One tool invocation tracked through its full lifecycle.
///
/// Created when a [`aegon_types::ToolCall`] event arrives; updated to [`StepStatus::Done`]
/// or [`StepStatus::Failed`] when the matching [`aegon_types::ToolResult`] is ingested.
#[derive(Debug, Clone)]
pub struct Step {
    /// Matches `ToolCall.id` / `ToolResult.tool_use_id`.
    pub id: String,
    /// Human-readable tool name (e.g. `"Bash"`, `"Read"`).
    pub tool_name: String,
    /// Event that causally preceded this tool call, used for parallel detection.
    pub parent_id: Option<Uuid>,
    /// Current lifecycle stage.
    pub status: StepStatus,
}

/// Lifecycle stage of a [`Step`].
#[derive(Debug, Clone)]
pub enum StepStatus {
    /// Tool call sent; result not yet received.
    Running { started_at: DateTime<Utc> },
    /// Result received successfully.
    Done { duration_ms: u64 },
    /// Result received with `is_error: true`.
    Failed { duration_ms: u64 },
}

impl StepStatus {
    /// Wall-clock milliseconds the step took, if it has finished.
    pub fn duration_ms(&self) -> Option<u64> {
        match self {
            Self::Running { .. } => None,
            Self::Done { duration_ms } | Self::Failed { duration_ms } => Some(*duration_ms),
        }
    }

    /// Whether this step is still waiting for a result.
    pub fn is_running(&self) -> bool {
        matches!(self, Self::Running { .. })
    }
}
