//! Cumulative token usage across all turns in a session.

use aegon_types::TokenUsage;

/// Running token totals for one session, accumulated across every assistant turn.
///
/// Use [`Self::add`] each time a [`aegon_types::TokenUsage`] arrives. The `total` method gives
/// the combined input+output count for progress-bar rendering.
#[derive(Debug, Clone, Default)]
pub struct TokenTotals {
    /// Total prompt tokens (non-cached) across all turns.
    pub input: u64,
    /// Total generated tokens across all turns.
    pub output: u64,
    /// Total tokens served from the prompt cache.
    pub cache_read: u64,
    /// Total tokens written into the prompt cache.
    pub cache_created: u64,
}

impl TokenTotals {
    /// Accumulate one turn's usage into the running totals.
    pub fn add(&mut self, usage: &TokenUsage) {
        self.input += usage.input_tokens;
        self.output += usage.output_tokens;
        self.cache_read += usage.cache_read_input_tokens;
        self.cache_created += usage.cache_creation_input_tokens;
    }

    /// Total tokens consumed (input + output), used as the numerator for gauges.
    pub fn total(&self) -> u64 {
        self.input + self.output
    }

    /// Estimated cost in USD using approximate claude-sonnet-4 pricing.
    ///
    /// Intentionally a rough guide — do not use for billing.
    pub fn estimated_cost_usd(&self) -> f64 {
        // Approximate rates ($/1M tokens): input $3, output $15, cache_read $0.30
        (self.input as f64 * 3.0 + self.output as f64 * 15.0 + self.cache_read as f64 * 0.30)
            / 1_000_000.0
    }
}
