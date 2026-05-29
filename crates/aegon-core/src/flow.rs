//! Causal-chain representation of a session's event sequence.
//!
//! A [`FlowNode`] is the unit of the flow visualization. The sequence of
//! nodes in [`SessionState::flow`] tells the story of what happened and in
//! what order — human input, thinking, parallel tool dispatches, and the
//! assistant's response.

/// One node in the causal flow of a session.
///
/// Consecutive [`FlowNode::Tool`] entries that share the same `parent_id`
/// in the original events are considered a parallel group and rendered
/// side-by-side (e.g. `[Bash ‖ Read]`).
#[derive(Debug, Clone, PartialEq)]
pub enum FlowNode {
    /// A user turn (human input or tool results fed back).
    Human,
    /// An extended-thinking block produced by the model.
    Thinking,
    /// One or more tool calls dispatched together.
    ///
    /// Multiple names = parallel dispatch (same `parent_id`).
    /// Single name = sequential.
    ToolGroup(Vec<String>),
    /// A text response from the assistant.
    Assistant,
}

impl FlowNode {
    /// Short label used in the flow string rendered by the TUI.
    pub fn label(&self) -> String {
        match self {
            Self::Human => "USER".into(),
            Self::Thinking => "THINK".into(),
            Self::ToolGroup(names) => {
                if names.len() == 1 {
                    names[0].clone()
                } else {
                    format!("[{}]", names.join(" ‖ "))
                }
            }
            Self::Assistant => "ASST".into(),
        }
    }
}
