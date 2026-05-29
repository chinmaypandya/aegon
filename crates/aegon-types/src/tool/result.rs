//! The output returned after a tool has executed.

use crate::tool::ToolMetadata;
use serde::{Deserialize, Serialize};

/// Output produced by running a [`crate::ToolCall`].
///
/// `is_error` reflects a tool-level failure (e.g. command exited non-zero),
/// not a parse error in the adapter. `metadata` carries supplementary
/// execution detail from the `toolUseResult` field when present.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolResult {
    /// ID of the [`crate::ToolCall`] this result corresponds to.
    pub tool_use_id: String,
    /// Output text returned by the tool (the content sent back to the model).
    pub content: String,
    /// Whether the tool itself reported an error.
    #[serde(default)]
    pub is_error: bool,
    /// Supplementary execution metadata from `toolUseResult`, if present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ToolMetadata>,
}
