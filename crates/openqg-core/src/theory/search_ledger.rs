//! V8 Phase 1 (#5): SearchLedger and TrialsGate.
//!
//! Any time an engine evaluates multiple theories against the same dataset the result is *one of
//! N candidates* — not a single independent test. Without a trials correction, any headline result
//! (ΔlnZ, ΔBIC) is a look-elsewhere artifact. These types enforce the correction.
//!
//! Design:
//! - `SearchLedger` accumulates the count of distinct theories evaluated.
//! - `TrialsGate` computes the Bonferroni-corrected evidence threshold given N trials:
//!   `threshold = base_ln_z + 0.5 * ln(N)`
//!   where `base_ln_z` defaults to 2.0 (i.e., "substantial" evidence in Jeffreys' scale,
//!   before correction).
//! - A theory that clears `TrialsGate::passes(obs_delta_ln_z)` is considered to have survived
//!   the trials correction. Otherwise the evidence is not strong enough to report.

use serde::{Deserialize, Serialize};

/// Accumulates the count of distinct theory evaluations against a fixed dataset.
///
/// Call `record_evaluation` once per theory evaluated. When the engine commits a theory to the
/// scorecard, call `trials_gate()` to obtain the corrected threshold.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchLedger {
    /// Number of distinct theories evaluated so far (including any that were vetoed before
    /// reaching the scoring stage — they still consumed a hypothesis slot).
    pub theories_evaluated: u64,

    /// Number of null-replay evaluations used to establish the empirical null distribution.
    /// Each null replay is a theory sampled from the null model (e.g. ΛCDM permutation) that
    /// was run through the full scoring pipeline. Used by the GoF gate.
    pub null_replays: u64,
}

impl SearchLedger {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that one more theory (real or decoy) was evaluated.
    pub fn record_evaluation(&mut self) {
        self.theories_evaluated += 1;
    }

    /// Record a null-replay trial.
    pub fn record_null_replay(&mut self) {
        self.null_replays += 1;
    }

    /// Build a `TrialsGate` from the current ledger state.
    ///
    /// `base_ln_z`: the unadjusted evidence threshold. Default 2.0 (Jeffreys "substantial").
    pub fn trials_gate(&self, base_ln_z: f64) -> TrialsGate {
        TrialsGate {
            n_trials: self.theories_evaluated.max(1),
            base_ln_z,
        }
    }

    /// Convenience: build a gate with the standard base threshold (2.0).
    pub fn standard_gate(&self) -> TrialsGate {
        self.trials_gate(2.0)
    }

    /// True when enough null replays exist to establish a meaningful null distribution
    /// (the SYNTHESIS requires ≥ 200 for any GoF claim).
    pub fn has_sufficient_null_replays(&self) -> bool {
        self.null_replays >= 200
    }
}

/// Evidence threshold after Bonferroni correction for `n_trials` simultaneous hypotheses.
///
/// The corrected threshold is:
///   threshold = base_ln_z + 0.5 · ln(n_trials)
///
/// A theory passes if its observed ΔlnZ exceeds this threshold.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TrialsGate {
    /// Number of hypotheses tested (from `SearchLedger::theories_evaluated`, min 1).
    pub n_trials: u64,
    /// Base evidence threshold before correction.
    pub base_ln_z: f64,
}

impl TrialsGate {
    /// The Bonferroni-corrected evidence threshold.
    pub fn threshold(&self) -> f64 {
        self.base_ln_z + 0.5 * (self.n_trials as f64).ln()
    }

    /// True when `observed_delta_ln_z` clears the corrected threshold.
    pub fn passes(&self, observed_delta_ln_z: f64) -> bool {
        observed_delta_ln_z >= self.threshold()
    }

    /// How much headroom (or shortfall) the observed evidence has relative to the threshold.
    /// Positive = passes, negative = fails.
    pub fn headroom(&self, observed_delta_ln_z: f64) -> f64 {
        observed_delta_ln_z - self.threshold()
    }
}

/// Empirical GoF null distribution collected from null-replay theories (SYNTHESIS #5).
///
/// After running ≥200 null theories through the full scoring pipeline, use this report to gate
/// any "beats ΛCDM" claim: a theory's ΔlnZ must be significant relative to the null distribution,
/// not merely positive.
///
/// The gate **fails closed**: fewer than 200 replays → `gate_passes()` returns false regardless
/// of the observed evidence, preventing any claim before the null distribution is established.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GoFNullReport {
    /// ΔlnZ scores from null-replay theories (sampled from the null model, scored in full).
    pub null_lnz_scores: Vec<f64>,
}

impl GoFNullReport {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a null-replay ΔlnZ score into the distribution.
    pub fn add_null_score(&mut self, score: f64) {
        self.null_lnz_scores.push(score);
    }

    /// True when ≥200 null replays have been recorded (SYNTHESIS #5 minimum).
    pub fn has_sufficient_replays(&self) -> bool {
        self.null_lnz_scores.len() >= 200
    }

    /// Empirical one-sided p-value: fraction of null scores ≥ observed (the proportion of
    /// null theories that beat the candidate by chance). Returns `None` when fewer than 200
    /// null replays are recorded — use of p-value requires a calibrated null.
    pub fn empirical_pvalue(&self, observed: f64) -> Option<f64> {
        if self.null_lnz_scores.len() < 200 {
            return None;
        }
        let n = self.null_lnz_scores.len() as f64;
        let exceedances = self
            .null_lnz_scores
            .iter()
            .filter(|&&s| s >= observed)
            .count() as f64;
        Some(exceedances / n)
    }

    /// True when the GoF gate passes at significance level `alpha`: the empirical p-value is at
    /// most `alpha`, meaning the candidate's ΔlnZ is in the top-`alpha` tail of the null.
    /// Returns `false` (fails closed) when insufficient null replays exist.
    pub fn gate_passes(&self, observed: f64, alpha: f64) -> bool {
        self.empirical_pvalue(observed)
            .map_or(false, |p| p <= alpha)
    }

    /// The q-th quantile of the null ΔlnZ distribution (0.0 ≤ q ≤ 1.0).
    /// Returns `None` when the distribution is empty.
    pub fn quantile(&self, q: f64) -> Option<f64> {
        if self.null_lnz_scores.is_empty() {
            return None;
        }
        let mut sorted = self.null_lnz_scores.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let idx = ((q * sorted.len() as f64).floor() as usize).min(sorted.len() - 1);
        Some(sorted[idx])
    }
}

/// Absolute χ²/dof gate: a theory whose chi-squared per degree of freedom exceeds `threshold`
/// is disqualified from any "beats ΛCDM" claim even when ΔlnZ > 0. Absolute goodness of fit
/// must pass before relative comparisons are meaningful.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Chi2DofGate {
    /// Maximum acceptable χ²/dof (inclusive). Standard is 2.0.
    pub threshold: f64,
}

impl Chi2DofGate {
    /// Standard GoF gate: χ²/dof ≤ 2.0.
    pub fn standard() -> Self {
        Chi2DofGate { threshold: 2.0 }
    }

    /// True when `chi2_per_dof` is finite and within the threshold.
    pub fn passes(&self, chi2_per_dof: f64) -> bool {
        chi2_per_dof.is_finite() && chi2_per_dof <= self.threshold
    }

    /// Diagnostic message when the gate fails.
    pub fn failure_message(&self, chi2_per_dof: f64) -> String {
        format!(
            "chi2/dof {chi2_per_dof:.3} exceeds GoF gate {:.3}: theory not reportable as beating ΛCDM",
            self.threshold
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- SearchLedger ----

    #[test]
    fn ledger_starts_at_zero() {
        let l = SearchLedger::new();
        assert_eq!(l.theories_evaluated, 0);
        assert_eq!(l.null_replays, 0);
    }

    #[test]
    fn record_evaluation_increments_count() {
        let mut l = SearchLedger::new();
        l.record_evaluation();
        l.record_evaluation();
        assert_eq!(l.theories_evaluated, 2);
    }

    #[test]
    fn null_replays_tracked_separately() {
        let mut l = SearchLedger::new();
        for _ in 0..200 {
            l.record_null_replay();
        }
        assert!(l.has_sufficient_null_replays());
    }

    #[test]
    fn has_sufficient_null_replays_false_below_200() {
        let mut l = SearchLedger::new();
        for _ in 0..199 {
            l.record_null_replay();
        }
        assert!(!l.has_sufficient_null_replays());
    }

    // ---- TrialsGate arithmetic ----

    #[test]
    fn single_hypothesis_threshold_equals_base() {
        // With N=1: correction = 0.5 * ln(1) = 0.0; threshold = base_ln_z.
        let gate = TrialsGate {
            n_trials: 1,
            base_ln_z: 2.0,
        };
        assert!((gate.threshold() - 2.0).abs() < 1e-12);
    }

    #[test]
    fn ten_hypotheses_threshold_is_correct() {
        // N=10: threshold = 2.0 + 0.5*ln(10) = 2.0 + 1.1513... = 3.1513...
        let gate = TrialsGate {
            n_trials: 10,
            base_ln_z: 2.0,
        };
        let expected = 2.0 + 0.5 * (10.0_f64).ln();
        assert!((gate.threshold() - expected).abs() < 1e-12);
    }

    #[test]
    fn passes_when_above_threshold() {
        let gate = TrialsGate {
            n_trials: 10,
            base_ln_z: 2.0,
        };
        assert!(gate.passes(gate.threshold() + 0.01));
    }

    #[test]
    fn fails_when_below_threshold() {
        let gate = TrialsGate {
            n_trials: 10,
            base_ln_z: 2.0,
        };
        assert!(!gate.passes(gate.threshold() - 0.01));
    }

    #[test]
    fn headroom_positive_when_passing() {
        let gate = TrialsGate {
            n_trials: 5,
            base_ln_z: 2.0,
        };
        let obs = gate.threshold() + 1.0;
        assert!(gate.headroom(obs) > 0.0);
    }

    #[test]
    fn headroom_negative_when_failing() {
        let gate = TrialsGate {
            n_trials: 5,
            base_ln_z: 2.0,
        };
        let obs = gate.threshold() - 1.0;
        assert!(gate.headroom(obs) < 0.0);
    }

    // ---- Ledger → gate integration ----

    #[test]
    fn standard_gate_n1_baseline() {
        let mut l = SearchLedger::new();
        l.record_evaluation();
        let gate = l.standard_gate();
        assert_eq!(gate.n_trials, 1);
        assert!((gate.base_ln_z - 2.0).abs() < 1e-12);
    }

    #[test]
    fn gate_threshold_grows_with_n() {
        let mut l = SearchLedger::new();
        for _ in 0..100 {
            l.record_evaluation();
        }
        let gate_100 = l.standard_gate();

        let mut l2 = SearchLedger::new();
        l2.record_evaluation();
        let gate_1 = l2.standard_gate();

        assert!(gate_100.threshold() > gate_1.threshold());
    }

    #[test]
    fn ledger_min_n_is_one_for_gate() {
        // Even an empty ledger produces a gate with n_trials = 1 (never log(0)).
        let l = SearchLedger::new();
        let gate = l.standard_gate();
        assert_eq!(gate.n_trials, 1);
        assert!((gate.threshold() - 2.0).abs() < 1e-12);
    }

    // ---- GoFNullReport ----

    fn null_report_with(n: usize, score: f64) -> GoFNullReport {
        let mut r = GoFNullReport::new();
        for _ in 0..n {
            r.add_null_score(score);
        }
        r
    }

    #[test]
    fn gof_null_report_insufficient_replays_returns_none() {
        let r = null_report_with(199, 0.0);
        assert!(!r.has_sufficient_replays());
        assert!(r.empirical_pvalue(0.0).is_none());
        assert!(!r.gate_passes(0.0, 0.05));
    }

    #[test]
    fn gof_null_report_all_zeros_pvalue_for_positive_observation() {
        // All null scores = 0.0; observed = 5.0 → 0 nulls beat it → p = 0/200 = 0.
        let r = null_report_with(200, 0.0);
        assert!(r.has_sufficient_replays());
        let p = r.empirical_pvalue(5.0).unwrap();
        assert_eq!(p, 0.0);
        assert!(r.gate_passes(5.0, 0.05));
    }

    #[test]
    fn gof_null_report_all_nulls_beat_observation() {
        // All null scores = 10.0; observed = 5.0 → all 200 beat it → p = 1.0.
        let r = null_report_with(200, 10.0);
        let p = r.empirical_pvalue(5.0).unwrap();
        assert_eq!(p, 1.0);
        assert!(!r.gate_passes(5.0, 0.05));
    }

    #[test]
    fn gof_null_report_mixed_distribution() {
        // 100 nulls at 0.0, 100 at 10.0; observed = 5.0 → 100/200 = 0.5 exceed.
        let mut r = GoFNullReport::new();
        for _ in 0..100 {
            r.add_null_score(0.0);
        }
        for _ in 0..100 {
            r.add_null_score(10.0);
        }
        let p = r.empirical_pvalue(5.0).unwrap();
        assert!((p - 0.5).abs() < 1e-12);
    }

    #[test]
    fn gof_null_report_quantile() {
        // 200 equally spaced scores 0.0..1.0; median ≈ 0.5.
        let mut r = GoFNullReport::new();
        for i in 0..200usize {
            r.add_null_score(i as f64 / 199.0);
        }
        let median = r.quantile(0.5).unwrap();
        assert!(median >= 0.4 && median <= 0.6, "median {median}");
    }

    #[test]
    fn gof_null_report_quantile_empty_returns_none() {
        let r = GoFNullReport::new();
        assert!(r.quantile(0.5).is_none());
    }

    // ---- Chi2DofGate ----

    #[test]
    fn chi2_gate_standard_threshold_is_2() {
        assert_eq!(Chi2DofGate::standard().threshold, 2.0);
    }

    #[test]
    fn chi2_gate_passes_within_threshold() {
        let g = Chi2DofGate::standard();
        assert!(g.passes(1.0));
        assert!(g.passes(2.0));
    }

    #[test]
    fn chi2_gate_fails_above_threshold() {
        let g = Chi2DofGate::standard();
        assert!(!g.passes(2.001));
        assert!(!g.passes(f64::INFINITY));
        assert!(!g.passes(f64::NAN));
    }

    #[test]
    fn chi2_gate_failure_message_contains_values() {
        let g = Chi2DofGate::standard();
        let msg = g.failure_message(3.14);
        assert!(msg.contains("3.140") && msg.contains("2.000"), "{msg}");
    }
}
