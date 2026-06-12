//! V8 Phase 1 (#19): engine-level KPIs emitted alongside each scoring run.
//!
//! These are not theory-level scores — they measure how well the *engine itself* is working:
//! how quickly it invalidates bad theories, how many exploits it has discovered and closed,
//! what fraction of the search space is covered, and at what instrument tier.
//!
//! The KPIs are written to `engine_kpis.json` in the run directory by `RunDirSink`. They are
//! diagnostic output, not scoring inputs — no theory score depends on them.

use serde::{Deserialize, Serialize};

/// Engine performance indicators for one scoring run (session-level, not theory-level).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EngineKpis {
    /// Number of theories evaluated in this run (from the SearchLedger).
    pub theories_evaluated: u64,

    /// Number of null-replay trials run to establish the empirical null distribution.
    pub null_replays: u64,

    /// Fraction of evaluated theories that were invalidated (vetoed or disqualified) before
    /// reaching the scoring stage. High value = good; means the gates are filtering effectively.
    /// Range: [0.0, 1.0].
    pub invalidation_rate: f64,

    /// Number of jailgun exploit specs that were discovered and closed by this engine version.
    /// Incremented when a new spec is added to `tips/jailgun/next-level/` and a regression test
    /// locks the fix.
    pub exploit_specs_closed: u32,

    /// Fraction of the theory search space that has been evaluated at least once.
    /// Estimated from the SearchLedger and the known theory vocabulary. A rough measure of
    /// how much of the space the engine has seen.
    /// Range: [0.0, 1.0]; `None` if the estimate is not available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search_coverage_estimate: Option<f64>,

    /// Highest `ForwardTier` used in this run. T2 or higher = promotion-grade results available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub peak_instrument_tier: Option<crate::cosmology::ForwardTier>,

    /// Mean time (in milliseconds) from theory proposal to invalidation verdict, measured over
    /// all invalidated theories this run. Low value = fast gating.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mean_ms_to_invalidate: Option<f64>,

    /// Estimated cost (USD) of this scoring run, from the token ledger.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_cost_usd: Option<f64>,
}

impl EngineKpis {
    /// Construct a minimal set of KPIs from available data.
    pub fn new(
        theories_evaluated: u64,
        null_replays: u64,
        theories_invalidated: u64,
        exploit_specs_closed: u32,
    ) -> Self {
        let invalidation_rate = if theories_evaluated > 0 {
            theories_invalidated as f64 / theories_evaluated as f64
        } else {
            0.0
        };
        EngineKpis {
            theories_evaluated,
            null_replays,
            invalidation_rate,
            exploit_specs_closed,
            search_coverage_estimate: None,
            peak_instrument_tier: None,
            mean_ms_to_invalidate: None,
            estimated_cost_usd: None,
        }
    }

    /// The engine is operating in a healthy regime when:
    /// - invalidation_rate ≥ 0.50 (more than half of proposals are caught by the gates)
    /// - null_replays ≥ 200 (enough for a reliable null distribution)
    ///
    /// This is informational — it does not affect any theory score.
    pub fn is_healthy(&self) -> bool {
        self.invalidation_rate >= 0.50 && self.null_replays >= 200
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cosmology::ForwardTier;

    #[test]
    fn invalidation_rate_computed_correctly() {
        let kpi = EngineKpis::new(100, 200, 75, 12);
        assert!((kpi.invalidation_rate - 0.75).abs() < 1e-10);
    }

    #[test]
    fn zero_evaluated_gives_zero_rate() {
        let kpi = EngineKpis::new(0, 0, 0, 0);
        assert_eq!(kpi.invalidation_rate, 0.0);
    }

    #[test]
    fn healthy_when_high_invalidation_and_enough_replays() {
        let mut kpi = EngineKpis::new(100, 200, 60, 12);
        assert!(kpi.is_healthy());
        kpi.null_replays = 199;
        assert!(!kpi.is_healthy());
        kpi.null_replays = 200;
        kpi.invalidation_rate = 0.49;
        assert!(!kpi.is_healthy());
    }

    #[test]
    fn optional_fields_default_to_none() {
        let kpi = EngineKpis::new(10, 200, 8, 5);
        assert!(kpi.peak_instrument_tier.is_none());
        assert!(kpi.mean_ms_to_invalidate.is_none());
        assert!(kpi.estimated_cost_usd.is_none());
        assert!(kpi.search_coverage_estimate.is_none());
    }

    #[test]
    fn peak_instrument_tier_roundtrips() {
        let mut kpi = EngineKpis::new(10, 200, 8, 5);
        kpi.peak_instrument_tier = Some(ForwardTier::T2Boltzmann);
        let json = serde_json::to_string(&kpi).unwrap();
        let back: EngineKpis = serde_json::from_str(&json).unwrap();
        assert_eq!(back.peak_instrument_tier, Some(ForwardTier::T2Boltzmann));
    }

    #[test]
    fn serializes_and_deserializes_full_kpi() {
        let mut kpi = EngineKpis::new(500, 250, 420, 12);
        kpi.search_coverage_estimate = Some(0.32);
        kpi.peak_instrument_tier = Some(ForwardTier::T1Emulator);
        kpi.mean_ms_to_invalidate = Some(42.5);
        kpi.estimated_cost_usd = Some(0.0012);

        let json = serde_json::to_string_pretty(&kpi).unwrap();
        let back: EngineKpis = serde_json::from_str(&json).unwrap();
        assert_eq!(back, kpi);
    }
}
