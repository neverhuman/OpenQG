//! V8 Wave 0.7: LLM call receipts for total-observability of token/cost spend.
//!
//! Every call to an LLM backend (jnoccio gateway, jekko, judge) must emit an [`LlmCallReceipt`]
//! so the `token-ledger.jsonl` event stream is complete and `unattributed_calls = 0` is
//! verifiable. Where exact usage is unavailable, `flagged_estimate = true` marks the record so
//! downstream analysis can distinguish exact from estimated cost.

use serde::{Deserialize, Serialize};

/// Token counts for one LLM API call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input: u64,
    pub output: u64,
    pub total: u64,
}

impl TokenUsage {
    pub fn new(input: u64, output: u64) -> Self {
        Self {
            input,
            output,
            total: input + output,
        }
    }
}

/// USD cost breakdown for one LLM API call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CostUsage {
    pub input_usd: f64,
    pub output_usd: f64,
    pub total_usd: f64,
}

impl CostUsage {
    pub fn new(input_usd: f64, output_usd: f64) -> Self {
        Self {
            input_usd,
            output_usd,
            total_usd: input_usd + output_usd,
        }
    }

    /// Compute cost from token counts and per-million-token rates.
    pub fn from_tokens(usage: &TokenUsage, input_per_1m: f64, output_per_1m: f64) -> Self {
        let input_usd = usage.input as f64 * input_per_1m / 1_000_000.0;
        let output_usd = usage.output as f64 * output_per_1m / 1_000_000.0;
        Self::new(input_usd, output_usd)
    }
}

/// A receipt for one LLM API call — written to `token-ledger.jsonl` via [`LedgerSink`].
///
/// `attributed_to` identifies the engine component that made the call so cost dashboards can
/// break down spend by role. `flagged_estimate = true` means exact usage was not available from
/// the API response and the counts were estimated from the tokenizer registry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LlmCallReceipt {
    /// Monotonically unique ID for this call (e.g. UUID or generation:sequence counter).
    pub call_id: String,
    /// RFC 3339 UTC timestamp of the call.
    pub timestamp_utc: String,
    /// API provider, e.g. `"anthropic"`, `"openai"`.
    pub provider: String,
    /// Model identifier as returned by the API.
    pub model: String,
    pub usage: TokenUsage,
    pub cost: CostUsage,
    /// Engine component that made this call: `"proposer"`, `"router"`, `"judge"`, `"linter"`, …
    pub attributed_to: String,
    /// `true` when exact token counts were unavailable and were estimated from the tokenizer registry.
    pub flagged_estimate: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_usage_total_is_sum() {
        let u = TokenUsage::new(1000, 500);
        assert_eq!(u.total, 1500);
    }

    #[test]
    fn cost_from_tokens_rounds_correctly() {
        // Claude Sonnet 4.6: $3.00/1M input, $15.00/1M output
        let usage = TokenUsage::new(1_000_000, 100_000);
        let cost = CostUsage::from_tokens(&usage, 3.00, 15.00);
        assert!((cost.input_usd - 3.00).abs() < 1e-9);
        assert!((cost.output_usd - 1.50).abs() < 1e-9);
        assert!((cost.total_usd - 4.50).abs() < 1e-9);
    }

    #[test]
    fn receipt_roundtrips_json() {
        let r = LlmCallReceipt {
            call_id: "gen1:0".into(),
            timestamp_utc: "2026-06-11T00:00:00Z".into(),
            provider: "anthropic".into(),
            model: "claude-sonnet-4-6".into(),
            usage: TokenUsage::new(500, 200),
            cost: CostUsage::new(0.0015, 0.003),
            attributed_to: "proposer".into(),
            flagged_estimate: false,
        };
        let json = serde_json::to_string(&r).unwrap();
        let rt: LlmCallReceipt = serde_json::from_str(&json).unwrap();
        assert_eq!(r, rt);
    }
}
