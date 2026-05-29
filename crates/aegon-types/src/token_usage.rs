//! Token consumption figures for a single assistant turn.

use serde::{Deserialize, Serialize};

/// Token counts reported by the model for one assistant response.
///
/// Cache fields are zero when the model did not use prompt caching.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Tokens in the prompt (non-cached portion).
    pub input_tokens: u64,
    /// Tokens in the model's response.
    pub output_tokens: u64,
    /// Prompt tokens served from cache (not re-billed at full rate).
    #[serde(default)]
    pub cache_read_input_tokens: u64,
    /// Prompt tokens written into the cache this turn.
    #[serde(default)]
    pub cache_creation_input_tokens: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_all_zeros() {
        let u = TokenUsage::default();
        assert_eq!(u.input_tokens, 0);
        assert_eq!(u.output_tokens, 0);
        assert_eq!(u.cache_read_input_tokens, 0);
        assert_eq!(u.cache_creation_input_tokens, 0);
    }

    #[test]
    fn serde_round_trip_with_cache_fields() {
        let u = TokenUsage {
            input_tokens: 10,
            output_tokens: 5,
            cache_read_input_tokens: 3,
            cache_creation_input_tokens: 1,
        };
        let json = serde_json::to_string(&u).unwrap();
        let back: TokenUsage = serde_json::from_str(&json).unwrap();
        assert_eq!(u, back);
    }

    #[test]
    fn serde_missing_cache_fields_defaults_to_zero() {
        let json = r#"{"input_tokens":8,"output_tokens":4}"#;
        let u: TokenUsage = serde_json::from_str(json).unwrap();
        assert_eq!(u.input_tokens, 8);
        assert_eq!(u.cache_read_input_tokens, 0);
    }
}
