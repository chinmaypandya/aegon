//! Pure session-state and flow logic for Aegon.
//!
//! This crate has no I/O and no storage. It receives [`LogEvent`]s and
//! maintains live [`SessionState`] per session — tracking running tool
//! calls, timing, token totals, and causal flow. Everything a dashboard
//! needs to render is produced here.

pub mod flow;
pub mod registry;
pub mod session;
pub mod step;
pub mod token_totals;

pub use flow::FlowNode;
pub use registry::SessionRegistry;
pub use session::SessionState;
pub use step::{Step, StepStatus};
pub use token_totals::TokenTotals;
