//! The output returned after a tool has executed.

use serde::{Deserialize, Serialize};

/// Output produced by running a [`ToolCall`].
///
/// `is_error` reflects a tool-level failure (e.g. command exited non-zero),
/// not a parse error in the adapter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolResult {
    /// ID of the [`ToolCall`] this result corresponds to.
    pub tool_use_id: String,
    /// Output text returned by the tool.
    pub content: String,
    /// Whether the tool itself reported an error.
    #[serde(default)]
    pub is_error: bool,
}
