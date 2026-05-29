//! Which event chain a [`crate::LogEvent`] belongs to.

use serde::{Deserialize, Serialize};

/// Identifies whether an event is part of the main session chain or a
/// sub-agent sidechain.
///
/// Claude Code records `isSidechain: true` on events that originate inside
/// a spawned sub-agent (e.g. via the `Agent` tool). Keeping them separate
/// is required for correct DAG construction and for meaningful TUI display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StreamId {
    /// The main session chain — the top-level run the user started.
    #[default]
    Main,
    /// A sub-agent run spawned from the main chain or another sidechain.
    Sidechain,
}
