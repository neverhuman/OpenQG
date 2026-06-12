//! V8 Phase 15 (#19): Engine KPI dashboard.
//!
//! Per-campaign KPI snapshot emitted as `engine_kpis.json`. All time fields in seconds.
//! `None` means the metric is not yet measurable (e.g. no surviving theories invalidated yet).
//!
//! Spec: SYNTHESIS #19: "time-to-invalidate (min/median/p90), exploit discovery rate,
//! regression-corpus growth, search volume covered, exclusion strength, cost-normalized score —
//! published per campaign as `engine_kpis.json`; sealed exploit reserve with z-test release gate."

use serde::{Deserialize, Serialize};

/// Per-campaign engine KPI snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineKpiReport {
    /// Unique campaign identifier (e.g. "v8-campaign-001").
    pub campaign_id: String,
    /// Schema version for forward-compat.
    pub schema_version: String,
    /// Total theories evaluated (including vetoed before scoring).
    pub theories_evaluated: u64,
    /// Time from campaign start to first surviving-theory invalidation (seconds).
    pub time_to_invalidate_min_s: Option<f64>,
    /// Median time across all invalidation events (seconds).
    pub time_to_invalidate_median_s: Option<f64>,
    /// 90th-percentile invalidation time (seconds).
    pub time_to_invalidate_p90_s: Option<f64>,
    /// Number of distinct exploit classes discovered (distinct `VetoReason` kinds that fired).
    pub exploits_discovered: u32,
    /// Size of the regression-lock corpus (number of fixture theories pinned to expected verdicts).
    pub regression_corpus_size: u32,
    /// Fraction of the frozen search volume `C_growth-suppression(V8)` covered by tested theories
    /// (0.0–1.0; requires a volume estimator wired in).
    pub search_volume_covered: f64,
    /// Trials-corrected exclusion strength in ln-units (None until evidence threshold is cleared).
    pub exclusion_strength_lnz: Option<f64>,
    /// Score points per USD of LLM cost (None until token ledger is active).
    pub cost_normalized_score: Option<f64>,
}

impl EngineKpiReport {
    /// Create a fresh KPI report with zero/None fields for a new campaign.
    pub fn new(campaign_id: impl Into<String>) -> Self {
        EngineKpiReport {
            campaign_id: campaign_id.into(),
            schema_version: "engine_kpis.v1".into(),
            theories_evaluated: 0,
            time_to_invalidate_min_s: None,
            time_to_invalidate_median_s: None,
            time_to_invalidate_p90_s: None,
            exploits_discovered: 0,
            regression_corpus_size: 0,
            search_volume_covered: 0.0,
            exclusion_strength_lnz: None,
            cost_normalized_score: None,
        }
    }

    /// True when the minimum reportable bar has been cleared: at least one surviving theory was
    /// invalidated AND at least one exploit class was discovered.
    pub fn is_reportable(&self) -> bool {
        self.time_to_invalidate_min_s.is_some() && self.exploits_discovered > 0
    }

    /// True when search volume coverage has reached a meaningful level (≥20%).
    pub fn has_meaningful_coverage(&self) -> bool {
        self.search_volume_covered >= 0.20
    }

    /// Update the time-to-invalidate statistics from a sorted list of invalidation times (seconds).
    /// Expects `times` sorted ascending. No-ops on an empty slice.
    pub fn set_invalidation_times(&mut self, times: &[f64]) {
        if times.is_empty() {
            return;
        }
        self.time_to_invalidate_min_s = Some(times[0]);
        let mid = times.len() / 2;
        self.time_to_invalidate_median_s = Some(if times.len() % 2 == 0 {
            (times[mid - 1] + times[mid]) / 2.0
        } else {
            times[mid]
        });
        let p90_idx = ((times.len() as f64 * 0.9).floor() as usize).min(times.len() - 1);
        self.time_to_invalidate_p90_s = Some(times[p90_idx]);
    }

    /// Serialize the report to a compact JSON string (for `engine_kpis.json`).
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_report_has_zero_theories() {
        let r = EngineKpiReport::new("test-campaign");
        assert_eq!(r.theories_evaluated, 0);
        assert_eq!(r.schema_version, "engine_kpis.v1");
        assert_eq!(r.campaign_id, "test-campaign");
    }

    #[test]
    fn new_report_is_not_reportable() {
        let r = EngineKpiReport::new("c");
        assert!(!r.is_reportable());
        assert!(!r.has_meaningful_coverage());
    }

    #[test]
    fn reportable_requires_both_invalidation_and_exploit() {
        let mut r = EngineKpiReport::new("c");
        r.time_to_invalidate_min_s = Some(120.0);
        assert!(!r.is_reportable(), "exploit must also be discovered");

        r.exploits_discovered = 1;
        r.time_to_invalidate_min_s = None;
        assert!(!r.is_reportable(), "invalidation must also exist");

        r.time_to_invalidate_min_s = Some(120.0);
        assert!(r.is_reportable());
    }

    #[test]
    fn meaningful_coverage_threshold_is_20_percent() {
        let mut r = EngineKpiReport::new("c");
        r.search_volume_covered = 0.19;
        assert!(!r.has_meaningful_coverage());
        r.search_volume_covered = 0.20;
        assert!(r.has_meaningful_coverage());
    }

    #[test]
    fn set_invalidation_times_single_element() {
        let mut r = EngineKpiReport::new("c");
        r.set_invalidation_times(&[60.0]);
        assert_eq!(r.time_to_invalidate_min_s, Some(60.0));
        assert_eq!(r.time_to_invalidate_median_s, Some(60.0));
        assert_eq!(r.time_to_invalidate_p90_s, Some(60.0));
    }

    #[test]
    fn set_invalidation_times_odd_count() {
        let mut r = EngineKpiReport::new("c");
        r.set_invalidation_times(&[10.0, 20.0, 30.0]);
        assert_eq!(r.time_to_invalidate_min_s, Some(10.0));
        assert_eq!(r.time_to_invalidate_median_s, Some(20.0));
        assert_eq!(r.time_to_invalidate_p90_s, Some(30.0));
    }

    #[test]
    fn set_invalidation_times_even_count() {
        let mut r = EngineKpiReport::new("c");
        r.set_invalidation_times(&[10.0, 20.0, 30.0, 40.0]);
        assert_eq!(r.time_to_invalidate_min_s, Some(10.0));
        // median of [10,20,30,40] = (20+30)/2 = 25
        assert_eq!(r.time_to_invalidate_median_s, Some(25.0));
    }

    #[test]
    fn set_invalidation_times_empty_is_noop() {
        let mut r = EngineKpiReport::new("c");
        r.set_invalidation_times(&[]);
        assert!(r.time_to_invalidate_min_s.is_none());
    }

    #[test]
    fn serde_round_trip() {
        let mut r = EngineKpiReport::new("v8-001");
        r.theories_evaluated = 1000;
        r.exploits_discovered = 7;
        r.regression_corpus_size = 42;
        r.search_volume_covered = 0.35;
        r.set_invalidation_times(&[60.0, 120.0, 300.0]);
        r.exclusion_strength_lnz = Some(4.3);

        let json = r.to_json_string().expect("serialize");
        let back: EngineKpiReport = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.campaign_id, "v8-001");
        assert_eq!(back.theories_evaluated, 1000);
        assert_eq!(back.exploits_discovered, 7);
        assert_eq!(back.exclusion_strength_lnz, Some(4.3));
    }
}
