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

/// Pricing entry for one model, mirroring `data/tokenizer-registry.yml`.
/// Rates are in USD per one million tokens.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelPricing {
    /// Model identifier as used in the API (e.g. `"claude-sonnet-4-6"`).
    pub model_id: &'static str,
    /// API provider (e.g. `"anthropic"`).
    pub provider: &'static str,
    /// Input cost per million tokens (USD).
    pub input_usd_per_1m: f64,
    /// Output cost per million tokens (USD).
    pub output_usd_per_1m: f64,
    /// When `true`, any cost computed using this entry should be flagged as an estimate because
    /// this entry is a catch-all (e.g. `anthropic-unknown`).
    pub flagged_estimate_default: bool,
}

impl ModelPricing {
    /// Estimate cost from token counts.
    pub fn estimate_cost(&self, usage: &TokenUsage) -> CostUsage {
        CostUsage::from_tokens(usage, self.input_usd_per_1m, self.output_usd_per_1m)
    }
}

/// In-process tokenizer/pricing registry, mirroring `data/tokenizer-registry.yml`.
///
/// Hardcoded to avoid a `serde_yaml` dependency in openqg-bench; update both the YAML and this
/// struct when pricing changes.
pub struct TokenizerRegistry {
    entries: &'static [ModelPricing],
}

static REGISTRY_ENTRIES: &[ModelPricing] = &[
    ModelPricing {
        model_id: "claude-sonnet-4-6",
        provider: "anthropic",
        input_usd_per_1m: 3.00,
        output_usd_per_1m: 15.00,
        flagged_estimate_default: false,
    },
    ModelPricing {
        model_id: "claude-opus-4-8",
        provider: "anthropic",
        input_usd_per_1m: 15.00,
        output_usd_per_1m: 75.00,
        flagged_estimate_default: false,
    },
    ModelPricing {
        model_id: "claude-haiku-4-5-20251001",
        provider: "anthropic",
        input_usd_per_1m: 0.80,
        output_usd_per_1m: 4.00,
        flagged_estimate_default: false,
    },
    ModelPricing {
        model_id: "claude-fable-5",
        provider: "anthropic",
        input_usd_per_1m: 3.00,
        output_usd_per_1m: 15.00,
        flagged_estimate_default: false,
    },
    // Catch-all: unknown Anthropic models use Sonnet-tier pricing.
    ModelPricing {
        model_id: "anthropic-unknown",
        provider: "anthropic",
        input_usd_per_1m: 3.00,
        output_usd_per_1m: 15.00,
        flagged_estimate_default: true,
    },
];

impl TokenizerRegistry {
    /// The global registry derived from `data/tokenizer-registry.yml`.
    pub fn global() -> Self {
        TokenizerRegistry {
            entries: REGISTRY_ENTRIES,
        }
    }

    /// Look up exact pricing for `model_id`. Returns `None` when the model is not registered.
    pub fn lookup(&self, model_id: &str) -> Option<&ModelPricing> {
        self.entries.iter().find(|e| e.model_id == model_id)
    }

    /// Look up pricing for `model_id`, returning the `anthropic-unknown` catch-all entry when the
    /// model is not registered.
    ///
    /// The catch-all entry always has `flagged_estimate_default = true`; the caller should reflect
    /// that in the resulting receipt when the model was not found.
    pub fn lookup_or_default(&self, model_id: &str) -> &ModelPricing {
        self.lookup(model_id).unwrap_or_else(|| {
            self.entries
                .iter()
                .find(|e| e.model_id == "anthropic-unknown")
                .expect("registry must always contain the anthropic-unknown catch-all entry")
        })
    }

    /// Estimate cost for a call to `model_id` with the given token counts.
    ///
    /// Returns `(CostUsage, flagged_estimate)`. `flagged_estimate` is true when the model was not
    /// found in the registry (catch-all used) or the matching entry has `flagged_estimate_default`.
    pub fn estimate_cost(&self, model_id: &str, usage: &TokenUsage) -> (CostUsage, bool) {
        let is_exact = self.lookup(model_id).is_some();
        let entry = self.lookup_or_default(model_id);
        let cost = entry.estimate_cost(usage);
        let flagged = !is_exact || entry.flagged_estimate_default;
        (cost, flagged)
    }

    /// All entries in the registry.
    pub fn entries(&self) -> &[ModelPricing] {
        self.entries
    }
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

    // ---- TokenizerRegistry ----

    #[test]
    fn sonnet_is_registered_with_correct_rates() {
        let reg = TokenizerRegistry::global();
        let entry = reg
            .lookup("claude-sonnet-4-6")
            .expect("Sonnet must be registered");
        assert!((entry.input_usd_per_1m - 3.00).abs() < 1e-9);
        assert!((entry.output_usd_per_1m - 15.00).abs() < 1e-9);
        assert!(!entry.flagged_estimate_default);
    }

    #[test]
    fn opus_is_registered_with_correct_rates() {
        let reg = TokenizerRegistry::global();
        let entry = reg
            .lookup("claude-opus-4-8")
            .expect("Opus must be registered");
        assert!((entry.input_usd_per_1m - 15.00).abs() < 1e-9);
        assert!((entry.output_usd_per_1m - 75.00).abs() < 1e-9);
    }

    #[test]
    fn haiku_is_registered_and_cheapest() {
        let reg = TokenizerRegistry::global();
        let haiku = reg
            .lookup("claude-haiku-4-5-20251001")
            .expect("Haiku must be registered");
        let sonnet = reg.lookup("claude-sonnet-4-6").unwrap();
        assert!(haiku.input_usd_per_1m < sonnet.input_usd_per_1m);
    }

    #[test]
    fn unknown_model_uses_anthropic_unknown_catch_all() {
        let reg = TokenizerRegistry::global();
        assert!(reg.lookup("gpt-5-turbo").is_none());
        let entry = reg.lookup_or_default("gpt-5-turbo");
        assert_eq!(entry.model_id, "anthropic-unknown");
        assert!(entry.flagged_estimate_default);
    }

    #[test]
    fn estimate_cost_exact_model_not_flagged() {
        let reg = TokenizerRegistry::global();
        let usage = TokenUsage::new(1_000_000, 0);
        let (cost, flagged) = reg.estimate_cost("claude-sonnet-4-6", &usage);
        assert!(
            (cost.input_usd - 3.00).abs() < 1e-9,
            "1M input tokens @ $3/M"
        );
        assert!(!flagged, "known exact model must not be flagged");
    }

    #[test]
    fn estimate_cost_unknown_model_is_flagged() {
        let reg = TokenizerRegistry::global();
        let usage = TokenUsage::new(1_000_000, 0);
        let (_cost, flagged) = reg.estimate_cost("some-unknown-model", &usage);
        assert!(flagged, "unknown model must be flagged as estimate");
    }

    #[test]
    fn all_entries_have_positive_rates() {
        let reg = TokenizerRegistry::global();
        for entry in reg.entries() {
            assert!(
                entry.input_usd_per_1m > 0.0,
                "entry {} must have positive input rate",
                entry.model_id
            );
            assert!(
                entry.output_usd_per_1m > 0.0,
                "entry {} must have positive output rate",
                entry.model_id
            );
        }
    }

    #[test]
    fn registry_always_has_catch_all_entry() {
        let reg = TokenizerRegistry::global();
        assert!(
            reg.lookup("anthropic-unknown").is_some(),
            "registry must always contain the anthropic-unknown catch-all entry"
        );
    }
}
