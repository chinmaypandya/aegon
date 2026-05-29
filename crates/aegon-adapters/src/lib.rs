//! Per-tool JSONL adapters that normalise raw events into `aegon-types`.
//!
//! Each sub-module knows the JSONL schema a specific tool emits and converts
//! it into `Vec<LogEvent>`. New tool support means adding a new module here;
//! nothing else in the workspace needs to change.

pub mod claude;

pub use claude::ClaudeAdapter;

use aegon_types::{LogEvent, Result};

/// Common interface all adapters must implement.
///
/// An adapter receives one raw JSONL line at a time and produces zero or more
/// normalised [`LogEvent`]s. A single raw line can expand to multiple events
/// (e.g. an assistant turn containing several tool calls).
pub trait Adapter {
    /// Parse one raw JSONL line into zero or more [`LogEvent`]s.
    ///
    /// Returns `Ok(vec![])` for lines that carry no observable signal
    /// (e.g. queue-operation bookkeeping). Returns `Err` only for lines that
    /// are structurally invalid JSON.
    fn parse_line(&self, line: &str) -> Result<Vec<LogEvent>>;
}
