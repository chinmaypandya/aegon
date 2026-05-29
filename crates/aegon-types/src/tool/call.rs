//! A tool invocation requested by the model.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A single tool call issued by the model during a session.
///
/// `input` is kept as raw JSON so downstream consumers can decode it into
/// the tool's own argument type without the types crate knowing tool schemas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    /// Stable ID used to match this call with its [`ToolResult`].
    pub id: String,
    /// Tool name as declared in the tool list (e.g. `"Bash"`, `"Read"`).
    pub name: String,
    /// Raw JSON arguments passed to the tool.
    pub input: Value,
}
