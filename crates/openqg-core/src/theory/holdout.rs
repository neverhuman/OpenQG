//! Out-of-sample (held-out) predictive scoring. The most respected credibility signal is
//! predicting observables the theory was *not* scored against (research §3). We split the
//! observable set into a train set and a disjoint held-out set, score the *same* theory's
//! predictions on each, and report the generalization gap: a theory that matches the train
//! observables far better than it predicts the held-out ones is overfit, not predictive.
//!
//! For the engine this is used as an honesty gate on a champion: evolve against the train split,
//! then demand it still predicts the held-out split — anti-recitation / anti-overfit.

use super::{physics_kills, Theory};
use crate::cosmology::{CosmologyParams, ForwardModel};
use crate::scoring::score_metrics;
use crate::types::ObservableRecord;

/// Held-out predictive evaluation of a theory.
#[derive(Debug, Clone, PartialEq)]
pub struct HeldOutScore {
    /// True if the theory is vetoed (then the scores are not meaningful).
    pub vetoed: bool,
    /// Log-likelihood on the train split.
    pub train_log_likelihood: f64,
    /// Log-likelihood on the held-out split.
    pub heldout_log_likelihood: f64,
    /// Fraction of held-out observables the model could derive.
    pub heldout_coverage: f64,
    /// Per-observable (train − held-out) log-likelihood. Large positive ⇒ overfit: fits the train
    /// observables far better than it predicts the unseen ones. Near zero ⇒ genuinely predictive.
    pub generalization_gap: f64,
}

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

/// Evaluate a theory's train vs held-out predictive performance. `heldout` lists the indices of
/// the held-out observables.
pub fn held_out_evaluate<M>(
    theory: &Theory,
    observables: &[ObservableRecord],
    model: &M,
    heldout: &[usize],
) -> HeldOutScore
where
    M: ForwardModel<Theory = CosmologyParams>,
{
    if !physics_kills(theory).is_empty() {
        return HeldOutScore {
            vetoed: true,
            train_log_likelihood: f64::NEG_INFINITY,
            heldout_log_likelihood: f64::NEG_INFINITY,
            heldout_coverage: 0.0,
            generalization_gap: f64::INFINITY,
        };
    }
    let train = subset(observables, false, heldout);
    let test = subset(observables, true, heldout);
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
    HeldOutScore {
        vetoed: false,
        train_log_likelihood: train_ll,
        heldout_log_likelihood: test_ll,
        heldout_coverage: test_cov,
        generalization_gap: train_per - test_per,
    }
}

/// Convenience: hold out every other observable (odd indices), for a balanced split.
pub fn alternating_holdout(n: usize) -> Vec<usize> {
    (0..n).filter(|i| i % 2 == 1).collect()
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
    fn lcdm_generalizes_to_held_out_bao() {
        let obs = desi();
        let heldout = alternating_holdout(obs.len());
        let s = held_out_evaluate(
            &Theory::baseline_lcdm(),
            &obs,
            &BackgroundForwardModel,
            &heldout,
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
        let s = held_out_evaluate(&ghost, &desi(), &BackgroundForwardModel, &[1]);
        assert!(s.vetoed);
        assert!(s.generalization_gap.is_infinite());
    }

    #[test]
    fn a_detuned_theory_predicts_held_out_worse() {
        // ΛCDM vs a wrong-Omega_m theory: the detuned one predicts the held-out BAO worse, so its
        // held-out log-likelihood is lower than ΛCDM's.
        let obs = desi();
        let heldout = alternating_holdout(obs.len());
        let good = held_out_evaluate(
            &Theory::baseline_lcdm(),
            &obs,
            &BackgroundForwardModel,
            &heldout,
        );
        let mut detuned = Theory::baseline_lcdm();
        detuned.background.omega_m = 0.45;
        let bad = held_out_evaluate(&detuned, &obs, &BackgroundForwardModel, &heldout);
        assert!(bad.heldout_log_likelihood < good.heldout_log_likelihood);
    }
}
