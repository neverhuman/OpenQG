//! Out-of-sample predictive scoring via a deterministic train/test *split*.
//!
//! Splits the observable set into a train set and a disjoint test set, scores the *same* theory's
//! predictions on each, and reports the generalization gap: a theory that matches the train
//! observables far better than it predicts the test ones is overfit, not predictive.
//!
//! **Naming note:** This is a deterministic alternating *split* over known rows — NOT a sealed
//! holdout. The name was updated from "holdout" to "split" (V8 Wave 0.5) to be honest: a real
//! sealed holdout requires the prediction registry (Phase 0 item #2), canary-leak tests, and
//! data tiers. Until those are wired in, callers should not describe this as "sealed".

use super::{physics_kills, Theory};
use crate::cosmology::{CosmologyParams, ForwardModel};
use crate::scoring::score_metrics;
use crate::types::ObservableRecord;

/// Train/test split predictive evaluation of a theory.
#[derive(Debug, Clone, PartialEq)]
pub struct SplitScore {
    /// True if the theory is vetoed (then the scores are not meaningful).
    pub vetoed: bool,
    /// Log-likelihood on the train split.
    pub train_log_likelihood: f64,
    /// Log-likelihood on the test split.
    pub heldout_log_likelihood: f64,
    /// Fraction of test observables the model could derive.
    pub heldout_coverage: f64,
    /// Per-observable (train − test) log-likelihood. Large positive ⇒ overfit: fits the train
    /// observables far better than it predicts the unseen ones. Near zero ⇒ genuinely predictive.
    pub generalization_gap: f64,
}

/// Backward-compat alias — prefer [`SplitScore`].
pub type HeldOutScore = SplitScore;

fn subset(
    observables: &[ObservableRecord],
    take_heldout: bool,
    heldout: &[usize],
) -> Vec<ObservableRecord> {
    observables
        .iter()
        .enumerate()
        .filter(|(i, _)| heldout.contains(i) == take_heldout)
        .map(|(_, o)| o.clone())
        .collect()
}

fn log_likelihood(
    theory: &Theory,
    obs: &[ObservableRecord],
    model: &impl ForwardModel<Theory = CosmologyParams>,
) -> (f64, f64) {
    if obs.is_empty() {
        return (0.0, 1.0);
    }
    let ids: Vec<String> = obs.iter().map(|o| o.observable_id.clone()).collect();
    // V5: predict on the truth-bound background (keeps train/holdout consistent with evaluate).
    let bound = super::binding::bind_modified_background(theory).theory;
    let preds = match model.predict(&bound.background, &ids) {
        Ok(p) => p,
        Err(_) => return (f64::NEG_INFINITY, 0.0),
    };
    let (m, _) = score_metrics(obs, &preds, theory.parameters.len().max(1), 0.0);
    (m.log_likelihood, m.coverage)
}

/// Evaluate a theory's train vs test split predictive performance. `split` lists the indices of
/// the test-split observables.
pub fn split_evaluate<M>(
    theory: &Theory,
    observables: &[ObservableRecord],
    model: &M,
    split: &[usize],
) -> SplitScore
where
    M: ForwardModel<Theory = CosmologyParams>,
{
    if !physics_kills(theory).is_empty() {
        return SplitScore {
            vetoed: true,
            train_log_likelihood: f64::NEG_INFINITY,
            heldout_log_likelihood: f64::NEG_INFINITY,
            heldout_coverage: 0.0,
            generalization_gap: f64::INFINITY,
        };
    }
    let train = subset(observables, false, split);
    let test = subset(observables, true, split);
    let (train_ll, _) = log_likelihood(theory, &train, model);
    let (test_ll, test_cov) = log_likelihood(theory, &test, model);
    let train_per = if train.is_empty() {
        0.0
    } else {
        train_ll / train.len() as f64
    };
    let test_per = if test.is_empty() {
        0.0
    } else {
        test_ll / test.len() as f64
    };
    SplitScore {
        vetoed: false,
        train_log_likelihood: train_ll,
        heldout_log_likelihood: test_ll,
        heldout_coverage: test_cov,
        generalization_gap: train_per - test_per,
    }
}

/// Backward-compat alias — prefer [`split_evaluate`].
pub fn held_out_evaluate<M>(
    theory: &Theory,
    observables: &[ObservableRecord],
    model: &M,
    split: &[usize],
) -> SplitScore
where
    M: ForwardModel<Theory = CosmologyParams>,
{
    split_evaluate(theory, observables, model, split)
}

/// Convenience: hold out every other observable (odd indices), for a balanced 50% split.
pub fn alternating_split(n: usize) -> Vec<usize> {
    (0..n).filter(|i| i % 2 == 1).collect()
}

/// Backward-compat alias — prefer [`alternating_split`].
pub fn alternating_holdout(n: usize) -> Vec<usize> {
    alternating_split(n)
}

#[cfg(test)]
mod tests {
    use super::super::Theory;
    use super::*;
    use crate::cosmology::BackgroundForwardModel;
    use crate::types::ObservableRecord;

    fn bao(id: &str, value: f64, unc: f64) -> ObservableRecord {
        ObservableRecord {
            observable_id: id.into(),
            kind: "bao".into(),
            value,
            uncertainty: unc,
            unit: "dimensionless".into(),
            source: None,
        }
    }

    fn desi() -> Vec<ObservableRecord> {
        vec![
            bao("dm_over_rd@0.510", 13.62, 0.25),
            bao("dh_over_rd@0.510", 20.98, 0.61),
            bao("dm_over_rd@0.706", 16.85, 0.32),
            bao("dh_over_rd@0.706", 20.08, 0.60),
        ]
    }

    #[test]
    fn lcdm_generalizes_to_split_bao() {
        let obs = desi();
        let split = alternating_split(obs.len());
        let s = split_evaluate(
            &Theory::baseline_lcdm(),
            &obs,
            &BackgroundForwardModel,
            &split,
        );
        assert!(!s.vetoed);
        assert!((s.heldout_coverage - 1.0).abs() < 1e-9);
        // A genuinely predictive theory has a small generalization gap.
        assert!(
            s.generalization_gap.abs() < 5.0,
            "gap = {}",
            s.generalization_gap
        );
    }

    #[test]
    fn a_vetoed_theory_reports_infinite_gap() {
        let mut ghost = Theory::baseline_lcdm();
        ghost.stability.q_s = -1.0;
        let s = split_evaluate(&ghost, &desi(), &BackgroundForwardModel, &[1]);
        assert!(s.vetoed);
        assert!(s.generalization_gap.is_infinite());
    }

    #[test]
    fn a_detuned_theory_predicts_split_worse() {
        // ΛCDM vs a wrong-Omega_m theory: the detuned one predicts the test split BAO worse.
        let obs = desi();
        let split = alternating_split(obs.len());
        let good = split_evaluate(
            &Theory::baseline_lcdm(),
            &obs,
            &BackgroundForwardModel,
            &split,
        );
        let mut detuned = Theory::baseline_lcdm();
        detuned.background.omega_m = 0.45;
        let bad = split_evaluate(&detuned, &obs, &BackgroundForwardModel, &split);
        assert!(bad.heldout_log_likelihood < good.heldout_log_likelihood);
    }
}
