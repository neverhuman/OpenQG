//! V8 Phase 1 (#19): engine-level KPIs emitted alongside each scoring run.
//!
//! These are not theory-level scores — they measure how well the *engine itself* is working:
//! how quickly it invalidates bad theories, how many exploits it has discovered and closed,
//! what fraction of the search space is covered, and at what instrument tier.
//!
//! The KPIs are written to `engine_kpis.json` in the run directory by `RunDirSink`. They are
//! diagnostic output, not scoring inputs — no theory score depends on them.
//!
//! ## V8 Phase 8: `from_ledger_entries()`
//!
//! The SYNTHESIS (#19) requires "KPIs reproduce from ledgers alone." `from_ledger_entries()`
//! accepts a slice of deserialized JSONL records from a scoring-run ledger and computes all
//! KPI fields that can be derived from those records:
//!
//! - `theories_evaluated`: count of records
//! - `theories_invalidated`: records with `disqualified == true`
//! - `invalidation_rate`: invalidated / evaluated
//! - `time_to_invalidate_stats`: min/median/p90 of per-theory `elapsed_ms` for killed theories
//! - `peak_instrument_tier`: maximum `instrument_tier` seen across all survivors
//! - `estimated_cost_usd`: sum of `cost_usd` fields (from token-ledger records if present)
//! - `cost_normalized_score`: total score / total cost (only when cost > 0)
//!
//! Fields that cannot be derived from ledger records alone (e.g. `null_replays`,
//! `exploit_specs_closed`, `search_coverage_estimate`) are left at their defaults.

use serde::{Deserialize, Serialize};
use serde_json::Value;

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

    /// V8 Phase 8: time-to-invalidate statistics across all invalidated theories.
    /// `None` if no invalidated theories were recorded with elapsed_ms.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_to_invalidate_stats: Option<TimeToInvalidateStats>,

    /// V8 Phase 8: total score / total estimated cost USD (cost-efficiency metric).
    /// `None` if cost is zero or unavailable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_normalized_score: Option<f64>,
}

/// Percentile statistics for the time (in milliseconds) from proposal to invalidation.
///
/// Measured over all theories that were invalidated (disqualified) in this run.
/// Low values indicate the gating cascade fires quickly. p90 > 5000ms suggests
/// a slow gate is not catching bad candidates early.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeToInvalidateStats {
    /// Fastest invalidation in this run (ms).
    pub min_ms: f64,
    /// Median invalidation time (ms).
    pub median_ms: f64,
    /// 90th-percentile invalidation time (ms).
    pub p90_ms: f64,
    /// Number of invalidated theories that contributed timing data.
    pub n_timed: u32,
}

impl TimeToInvalidateStats {
    /// Compute statistics from a sorted list of elapsed_ms values.
    /// Returns `None` if the list is empty.
    pub fn from_sorted(mut values: Vec<f64>) -> Option<Self> {
        if values.is_empty() {
            return None;
        }
        values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let n = values.len();
        let min_ms = values[0];
        let median_ms = if n % 2 == 0 {
            (values[n / 2 - 1] + values[n / 2]) / 2.0
        } else {
            values[n / 2]
        };
        let p90_idx = ((n as f64 * 0.90).ceil() as usize).min(n) - 1;
        let p90_ms = values[p90_idx];
        Some(TimeToInvalidateStats {
            min_ms,
            median_ms,
            p90_ms,
            n_timed: n as u32,
        })
    }
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
            time_to_invalidate_stats: None,
            cost_normalized_score: None,
        }
    }

    /// Compute KPIs from a slice of deserialized ledger records (one per theory evaluation).
    ///
    /// Each record is a JSON object with these optional fields:
    /// - `disqualified: bool` — whether this theory was invalidated
    /// - `elapsed_ms: f64` — wall-clock time for this theory (ms)
    /// - `total: f64` — scorecard total (points) for survivors
    /// - `cost_usd: f64` — token cost for this theory (from token-ledger attribution)
    /// - `instrument_tier: string` — forward-model tier ("t0_formula", "t1_emulator", "t2_boltzmann", "t3_cross_solver")
    ///
    /// Fields not derivable from records alone (`null_replays`, `exploit_specs_closed`,
    /// `search_coverage_estimate`) are left at defaults.
    pub fn from_ledger_entries(entries: &[Value]) -> Self {
        use crate::cosmology::ForwardTier;

        let theories_evaluated = entries.len() as u64;
        let mut theories_invalidated = 0u64;
        let mut invalidated_times_ms: Vec<f64> = Vec::new();
        let mut total_score = 0.0_f64;
        let mut total_cost_usd = 0.0_f64;
        let mut has_any_cost = false;
        let mut peak_tier: Option<ForwardTier> = None;
        let mut mean_times_ms: Vec<f64> = Vec::new();

        for entry in entries {
            let disq = entry
                .get("disqualified")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let elapsed = entry.get("elapsed_ms").and_then(Value::as_f64);
            let total = entry.get("total").and_then(Value::as_f64).unwrap_or(0.0);
            let cost = entry.get("cost_usd").and_then(Value::as_f64);
            let tier_str = entry
                .get("instrument_tier")
                .and_then(Value::as_str)
                .unwrap_or("");

            if disq {
                theories_invalidated += 1;
                if let Some(ms) = elapsed {
                    invalidated_times_ms.push(ms);
                }
            } else {
                total_score += total;
                if let Some(ms) = elapsed {
                    mean_times_ms.push(ms);
                }
            }

            if let Some(c) = cost {
                if c > 0.0 {
                    total_cost_usd += c;
                    has_any_cost = true;
                }
            }

            let tier = match tier_str {
                "t0_formula" => Some(ForwardTier::T0Formula),
                "t1_emulator" => Some(ForwardTier::T1Emulator),
                "t2_boltzmann" => Some(ForwardTier::T2Boltzmann),
                "t3_cross_solver" => Some(ForwardTier::T3CrossSolver),
                _ => None,
            };
            if let Some(t) = tier {
                peak_tier = Some(match peak_tier {
                    None => t,
                    Some(existing) => existing.max(t),
                });
            }
        }

        let invalidation_rate = if theories_evaluated > 0 {
            theories_invalidated as f64 / theories_evaluated as f64
        } else {
            0.0
        };

        let mean_ms_to_invalidate = if !mean_times_ms.is_empty() {
            Some(mean_times_ms.iter().sum::<f64>() / mean_times_ms.len() as f64)
        } else {
            None
        };

        let time_to_invalidate_stats = TimeToInvalidateStats::from_sorted(invalidated_times_ms);

        let estimated_cost_usd = if has_any_cost {
            Some(total_cost_usd)
        } else {
            None
        };

        let cost_normalized_score = if total_cost_usd > 0.0 {
            Some(total_score / total_cost_usd)
        } else {
            None
        };

        EngineKpis {
            theories_evaluated,
            null_replays: 0, // not derivable from theory-level records alone
            invalidation_rate,
            exploit_specs_closed: 0, // not derivable from theory-level records alone
            search_coverage_estimate: None,
            peak_instrument_tier: peak_tier,
            mean_ms_to_invalidate,
            estimated_cost_usd,
            time_to_invalidate_stats,
            cost_normalized_score,
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

    // ---- TimeToInvalidateStats ----

    #[test]
    fn time_stats_from_empty_is_none() {
        assert!(TimeToInvalidateStats::from_sorted(vec![]).is_none());
    }

    #[test]
    fn time_stats_single_value() {
        let s = TimeToInvalidateStats::from_sorted(vec![42.0]).unwrap();
        assert_eq!(s.min_ms, 42.0);
        assert_eq!(s.median_ms, 42.0);
        assert_eq!(s.p90_ms, 42.0);
        assert_eq!(s.n_timed, 1);
    }

    #[test]
    fn time_stats_percentiles_correct() {
        // 10 values: 0..9 ms
        let vals: Vec<f64> = (0..10).map(|i| i as f64).collect();
        let s = TimeToInvalidateStats::from_sorted(vals).unwrap();
        assert_eq!(s.min_ms, 0.0);
        assert_eq!(s.n_timed, 10);
        // median of 0..9: average of 4 and 5 = 4.5
        assert!((s.median_ms - 4.5).abs() < 1e-9);
        // p90: 90th percentile of 10 = index 8 = 8.0
        assert!((s.p90_ms - 8.0).abs() < 1e-9);
    }

    // ---- from_ledger_entries ----

    fn make_entry(disq: bool, elapsed: f64, total: f64, tier: &str, cost: f64) -> Value {
        serde_json::json!({
            "disqualified": disq,
            "elapsed_ms": elapsed,
            "total": total,
            "instrument_tier": tier,
            "cost_usd": cost,
        })
    }

    #[test]
    fn from_ledger_entries_counts_theories() {
        let entries = vec![
            make_entry(true, 10.0, 0.0, "t0_formula", 0.001),
            make_entry(false, 50.0, 72.0, "t1_emulator", 0.002),
            make_entry(true, 8.0, 0.0, "t0_formula", 0.001),
        ];
        let kpi = EngineKpis::from_ledger_entries(&entries);
        assert_eq!(kpi.theories_evaluated, 3);
        assert!((kpi.invalidation_rate - 2.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn from_ledger_entries_computes_invalidation_timing() {
        let entries = vec![
            make_entry(true, 10.0, 0.0, "t0_formula", 0.0),
            make_entry(true, 20.0, 0.0, "t0_formula", 0.0),
            make_entry(false, 100.0, 60.0, "t1_emulator", 0.0),
        ];
        let kpi = EngineKpis::from_ledger_entries(&entries);
        let stats = kpi.time_to_invalidate_stats.unwrap();
        assert_eq!(stats.n_timed, 2);
        assert_eq!(stats.min_ms, 10.0);
        // median of [10, 20] = 15
        assert!((stats.median_ms - 15.0).abs() < 1e-9);
    }

    #[test]
    fn from_ledger_entries_peak_tier_is_max() {
        let entries = vec![
            make_entry(false, 50.0, 60.0, "t0_formula", 0.0),
            make_entry(false, 50.0, 70.0, "t2_boltzmann", 0.0),
            make_entry(true, 10.0, 0.0, "t1_emulator", 0.0),
        ];
        let kpi = EngineKpis::from_ledger_entries(&entries);
        assert_eq!(kpi.peak_instrument_tier, Some(ForwardTier::T2Boltzmann));
    }

    #[test]
    fn from_ledger_entries_cost_normalized_score() {
        let entries = vec![
            make_entry(false, 50.0, 80.0, "t1_emulator", 0.01),
            make_entry(false, 60.0, 40.0, "t1_emulator", 0.01),
        ];
        let kpi = EngineKpis::from_ledger_entries(&entries);
        assert!(kpi.estimated_cost_usd.is_some());
        assert!((kpi.estimated_cost_usd.unwrap() - 0.02).abs() < 1e-9);
        let cnr = kpi.cost_normalized_score.unwrap();
        // total score = 120, total cost = 0.02 → 6000 pts/USD
        assert!((cnr - 6000.0).abs() < 1e-6);
    }

    #[test]
    fn from_ledger_entries_no_cost_means_none() {
        let entries = vec![make_entry(false, 50.0, 60.0, "t0_formula", 0.0)];
        let kpi = EngineKpis::from_ledger_entries(&entries);
        assert!(kpi.estimated_cost_usd.is_none());
        assert!(kpi.cost_normalized_score.is_none());
    }

    #[test]
    fn from_ledger_entries_empty_slice() {
        let kpi = EngineKpis::from_ledger_entries(&[]);
        assert_eq!(kpi.theories_evaluated, 0);
        assert_eq!(kpi.invalidation_rate, 0.0);
        assert!(kpi.time_to_invalidate_stats.is_none());
    }

    #[test]
    fn from_ledger_entries_missing_fields_dont_panic() {
        // Records with no elapsed_ms, no tier, no cost — should still count
        let entries = vec![
            serde_json::json!({ "disqualified": true }),
            serde_json::json!({ "disqualified": false, "total": 55.0 }),
        ];
        let kpi = EngineKpis::from_ledger_entries(&entries);
        assert_eq!(kpi.theories_evaluated, 2);
        assert!((kpi.invalidation_rate - 0.5).abs() < 1e-9);
        assert!(kpi.peak_instrument_tier.is_none());
        assert!(kpi.time_to_invalidate_stats.is_none()); // no elapsed_ms on disq record
    }
}
