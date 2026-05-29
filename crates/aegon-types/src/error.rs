//! Crate-wide error and result types.

/// Errors that can originate from parsing or constructing domain types.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A required field was absent in the source data.
    #[error("missing required field: {0}")]
    MissingField(&'static str),

    /// A field value did not match the expected format or range.
    #[error("invalid field '{field}': {reason}")]
    InvalidField { field: &'static str, reason: String },
}

/// Convenience alias used throughout the workspace.
pub type Result<T> = std::result::Result<T, Error>;
