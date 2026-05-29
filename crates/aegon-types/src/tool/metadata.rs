//! Supplementary execution metadata attached to a [`crate::ToolResult`].
//!
//! Claude Code writes a `toolUseResult` field on every `user` record that
//! carries structured information about how the tool ran. This is richer
//! than the `content` string sent back to the model.

use serde::{Deserialize, Serialize};

/// Structured metadata about how a tool executed.
///
/// Always present alongside the tool result content — it is not an alternate
/// representation of the result, but supplementary detail. The shape varies
/// by tool: shell tools carry `stdout`/`stderr`; file tools carry a `file`
/// path.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolMetadata {
    /// Standard output from the tool process, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdout: Option<String>,
    /// Standard error from the tool process, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,
    /// Whether the tool run was interrupted before completing.
    #[serde(default)]
    pub interrupted: bool,
    /// Whether the output is an image rather than text.
    #[serde(default)]
    pub is_image: bool,
    /// Path of the file that was read or written, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
}
