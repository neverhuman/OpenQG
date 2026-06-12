//! V8 Phase 6: post-search null distribution and p-value.
//!
//! ## The problem (S07 §3 "Fail 3: Search selection")
//!
//! A champion is the maximum of a long adaptive process over many candidates.
//! The per-candidate ΔlnZ is NOT the right statistic to report — it is optimistic
//! because the champion was selected after seeing all results. The correct statistic
//! is the **post-search p-value**: the fraction of null-model replications whose
//! maximum ΔlnZ meets or exceeds the observed champion score.
//!
//! ## What this module provides
//!
//! - [`SearchNullDistribution`]: stores the null distribution of max ΔlnZ from
//!   ≥200 ΛCDM-null replications and computes the post-search p-value.
//! - [`PostSearchVerdict`]: structured gate result used by `ClaimClass::DiscoveryClaim`.
//!
//! The actual null replications (re-running the proposer loop under synthetic ΛCDM)
//! are a runtime operation outside this module. This module consumes the results
//! and computes statistics from them.
//!
//! **Minimum replications**: S07 requires ≥200. Fewer than 200 replications → the
//! distribution is marked insufficient and `passes_discovery_gate()` returns `false`.
//!
//! Reference: S07 §3 and §5 "Post-search p-value".

use serde::{Deserialize, Serialize};

/// The threshold below which the post-search p-value must fall for a discovery claim.
///
/// 0.003 corresponds to ~3σ one-sided. This is the minimum requirement; Nature-tier
/// claims require the 5σ equivalent (p < 3e-7), which needs a much larger null ensemble.
pub const DISCOVERY_P_VALUE_THRESHOLD: f64 = 0.003;

/// Minimum number of null replications required before the distribution is trusted.
pub const MIN_NULL_REPLICATIONS: u32 = 200;

/// One simulated null replication: the maximum ΔlnZ observed when the proposer loop
/// is run against synthetic ΛCDM data drawn from the null prior.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NullReplication {
    /// Maximum ΔlnZ observed in this replication (across all candidates evaluated).
    pub max_delta_ln_z: f64,
    /// Number of candidates evaluated in this replication (for auditing).
    pub n_candidates: u32,
    /// Deterministic seed used to generate the synthetic ΛCDM data.
    pub rng_seed: u64,
}

/// The post-search null distribution of max ΔlnZ and the associated p-value.
///
/// Built by running the full proposer/evaluator/selector loop ≥200 times under
/// draws from the null prior (ΛCDM with no modification), recording the maximum
/// ΔlnZ in each replication, then computing the fraction of replications whose
/// maximum meets or exceeds the champion's observed ΔlnZ.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchNullDistribution {
    /// The champion's observed maximum ΔlnZ (the test statistic).
    pub observed_max_delta_ln_z: f64,
    /// All null replications, sorted by max_delta_ln_z ascending.
    pub replications: Vec<NullReplication>,
    /// Post-search p-value: fraction of null replications with max_delta_ln_z
    /// ≥ observed_max_delta_ln_z. Computed at construction time.
    pub p_value_post_search: f64,
    /// True when ≥ MIN_NULL_REPLICATIONS replications are present.
    pub sufficient_replications: bool,
}

impl SearchNullDistribution {
    /// Build from a set of null replications.
    ///
    /// Sorts replications ascending, computes p-value and sufficiency flag.
    pub fn new(observed_max_delta_ln_z: f64, mut replications: Vec<NullReplication>) -> Self {
        replications.sort_by(|a, b| {
            a.max_delta_ln_z
                .partial_cmp(&b.max_delta_ln_z)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let n = replications.len() as f64;
        let n_exceeding = replications
            .iter()
            .filter(|r| r.max_delta_ln_z >= observed_max_delta_ln_z)
            .count() as f64;
        let p_value_post_search = if n > 0.0 { n_exceeding / n } else { 1.0 };
        let sufficient_replications = replications.len() as u32 >= MIN_NULL_REPLICATIONS;
        SearchNullDistribution {
            observed_max_delta_ln_z,
            replications,
            p_value_post_search,
            sufficient_replications,
        }
    }

    /// True when the post-search p-value meets the discovery threshold.
    ///
    /// Requirements: sufficient replications AND p_value_post_search < DISCOVERY_P_VALUE_THRESHOLD.
    pub fn passes_discovery_gate(&self) -> bool {
        self.sufficient_replications && self.p_value_post_search < DISCOVERY_P_VALUE_THRESHOLD
    }

    /// Number of null replications stored.
    pub fn n_replications(&self) -> u32 {
        self.replications.len() as u32
    }

    /// Empirical quantile of the null distribution (0.0 → min, 1.0 → max).
    /// Returns `None` when the distribution is empty.
    pub fn quantile(&self, q: f64) -> Option<f64> {
        if self.replications.is_empty() {
            return None;
        }
        let idx = ((q * (self.replications.len() - 1) as f64).round() as usize)
            .min(self.replications.len() - 1);
        Some(self.replications[idx].max_delta_ln_z)
    }

    /// The 95th-percentile of the null distribution.
    pub fn p95(&self) -> Option<f64> {
        self.quantile(0.95)
    }

    /// The 99th-percentile of the null distribution.
    pub fn p99(&self) -> Option<f64> {
        self.quantile(0.99)
    }
}

/// Structured gate verdict for discovery-class claims.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostSearchVerdict {
    pub observed_max_delta_ln_z: f64,
    pub p_value_post_search: f64,
    pub n_replications: u32,
    pub passes: bool,
    /// Short explanation of why the gate passed or failed.
    pub reason: String,
}

impl PostSearchVerdict {
    /// Derive the verdict from a null distribution.
    pub fn from_distribution(dist: &SearchNullDistribution) -> Self {
        let passes = dist.passes_discovery_gate();
        let reason = if !dist.sufficient_replications {
            format!(
                "insufficient null replications: {} < {} required",
                dist.n_replications(),
                MIN_NULL_REPLICATIONS
            )
        } else if passes {
            format!(
                "post-search p={:.4} < {:.3} threshold with {} replications",
                dist.p_value_post_search,
                DISCOVERY_P_VALUE_THRESHOLD,
                dist.n_replications()
            )
        } else {
            format!(
                "post-search p={:.4} ≥ {:.3} threshold — not discovery-grade",
                dist.p_value_post_search, DISCOVERY_P_VALUE_THRESHOLD,
            )
        };
        PostSearchVerdict {
            observed_max_delta_ln_z: dist.observed_max_delta_ln_z,
            p_value_post_search: dist.p_value_post_search,
            n_replications: dist.n_replications(),
            passes,
            reason,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn null_rep(max_dlnz: f64, seed: u64) -> NullReplication {
        NullReplication {
            max_delta_ln_z: max_dlnz,
            n_candidates: 100,
            rng_seed: seed,
        }
    }

    fn flat_null_distribution(n: u32, max_dlnz: f64) -> Vec<NullReplication> {
        (0..n)
            .map(|i| null_rep(max_dlnz * i as f64 / n as f64, i as u64))
            .collect()
    }

    #[test]
    fn p_value_zero_when_observed_exceeds_all_null() {
        let reps: Vec<_> = (0..200)
            .map(|i| null_rep(i as f64 * 0.01, i as u64))
            .collect();
        let dist = SearchNullDistribution::new(5.0, reps);
        assert_eq!(dist.p_value_post_search, 0.0);
        assert!(dist.passes_discovery_gate());
        assert!(dist.sufficient_replications);
    }

    #[test]
    fn p_value_one_when_observed_below_all_null() {
        let reps: Vec<_> = (0..200)
            .map(|i| null_rep(10.0 + i as f64, i as u64))
            .collect();
        let dist = SearchNullDistribution::new(1.0, reps);
        assert_eq!(dist.p_value_post_search, 1.0);
        assert!(!dist.passes_discovery_gate());
    }

    #[test]
    fn insufficient_replications_blocks_discovery_gate() {
        let reps: Vec<_> = (0..50)
            .map(|i| null_rep(i as f64 * 0.01, i as u64))
            .collect();
        let dist = SearchNullDistribution::new(5.0, reps);
        assert!(!dist.sufficient_replications);
        assert!(
            !dist.passes_discovery_gate(),
            "< 200 reps must block discovery gate"
        );
    }

    #[test]
    fn exactly_200_replications_is_sufficient() {
        let reps = flat_null_distribution(200, 2.0);
        let dist = SearchNullDistribution::new(5.0, reps);
        assert!(dist.sufficient_replications);
    }

    #[test]
    fn p_value_computed_from_fraction() {
        // 198 reps at 0.0, then 2 reps at or above the observed value (5.0).
        let mut reps: Vec<NullReplication> = (0..198).map(|i| null_rep(0.0, i as u64)).collect();
        reps.push(null_rep(5.0, 998));
        reps.push(null_rep(6.0, 999));
        let dist = SearchNullDistribution::new(5.0, reps);
        // Exactly 2 out of 200 replications have max_delta_ln_z >= 5.0
        let expected = 2.0 / 200.0;
        assert!((dist.p_value_post_search - expected).abs() < 1e-9);
    }

    #[test]
    fn quantile_and_percentile_smoke() {
        let reps: Vec<_> = (0..200).map(|i| null_rep(i as f64, i as u64)).collect();
        let dist = SearchNullDistribution::new(300.0, reps);
        assert!(dist.p95().is_some());
        let p95 = dist.p95().unwrap();
        assert!(p95 >= 189.0 && p95 <= 200.0);
    }

    #[test]
    fn post_search_verdict_from_distribution() {
        let reps: Vec<_> = (0..200)
            .map(|i| null_rep(i as f64 * 0.01, i as u64))
            .collect();
        let dist = SearchNullDistribution::new(5.0, reps);
        let v = PostSearchVerdict::from_distribution(&dist);
        assert!(v.passes);
        assert_eq!(v.n_replications, 200);
        assert!(v.reason.contains("0.003"));
    }

    #[test]
    fn empty_distribution_p_value_is_one() {
        let dist = SearchNullDistribution::new(3.0, vec![]);
        assert_eq!(dist.p_value_post_search, 1.0);
        assert!(!dist.passes_discovery_gate());
    }
}
