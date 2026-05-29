//! Core domain types for Aegon — the single source of truth for all data shapes.
//!
//! Every other crate depends on this one. No logic, no I/O — pure data shapes
//! and their `serde` impls.

pub mod error;
pub mod event;
pub mod session;
pub mod token_usage;
pub mod tool;

pub use error::{Error, Result};
pub use event::{EventKind, LogEvent};
pub use session::Session;
pub use token_usage::TokenUsage;
pub use tool::{ToolCall, ToolResult};
