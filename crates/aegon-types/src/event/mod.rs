//! The central event types that flow through the Aegon pipeline.

mod kind;
mod log_event;

pub use kind::EventKind;
pub use log_event::LogEvent;
