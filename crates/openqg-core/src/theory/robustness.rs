//! Robustness-under-perturbation (structural stability): a credible theory must stay credible —
//! and keep a similar fitness — under small perturbations to its parameters. A theory fine-tuned
//! to sit exactly on a veto boundary (a near-zero no-ghost determinant, an α at the screening
//! edge) or on a data knife-edge is fragile and should score low. This is the structural-stability
//! anti-overfit device (cf. arXiv:2509.21780) and the operational form of the project's
//! robustness-over-likelihood preference: we reward theories that are robustly good, not narrowly.

use super::mutation::Rng;
use super::{assess, Theory};
use crate::cosmology::{CosmologyParams, ForwardModel};
use crate::types::ObservableRecord;

/// How far a perturbed candidate's final fitness may drift from the original and still count as
/// "the same" robust theory.
const FITNESS_DRIFT_TOLERANCE: f64 = 0.15;

/// Fraction of small random parameter perturbations under which the theory remains credible with a
/// similar fitness, in `[0,1]`. 1 ⇒ robustly good everywhere nearby; low ⇒ a knife-edge fit.
/// Deterministic in `seed`.
pub fn perturbation_robustness<M>(
    theory: &Theory,
    observables: &[ObservableRecord],
    model: &M,
    baseline_log_likelihood: f64,
    seed: u64,
    samples: usize,
) -> f64
where
    M: ForwardModel<Theory = CosmologyParams>,
{
    let base = assess(theory, observables, model, baseline_log_likelihood);
    if !base.is_credible() {
        return 0.0; // a non-credible theory has no robustness to speak of
    }
    if samples == 0 {
        return 1.0;
    }
    let mut rng = Rng::new(seed);
    let mut stable = 0usize;
    for _ in 0..samples {
        let mut p = theory.clone();
        p.stability.q_s += 0.05 * rng.signed();
        p.stability.sound_speed_sq += 0.05 * rng.signed();
        p.stability.kinetic_coefficient += 0.05 * rng.signed();
        // Perturb the modified-gravity sector only for theories that already modify gravity (and
        // thus declare screening): nudging an unscreened GR theory's alpha across the screening
        // threshold would test a different theory class, not this candidate's robustness.
        let d_alpha_m = 0.01 * rng.signed();
        let d_alpha_b = 0.01 * rng.signed();
        if p.screening.is_some() {
            p.alpha.alpha_m += d_alpha_m;
            p.alpha.alpha_b += d_alpha_b;
        }
        p.background.h += 0.005 * rng.signed();
        p.background.omega_m += 0.005 * rng.signed();
        let a = assess(&p, observables, model, baseline_log_likelihood);
        if a.is_credible()
            && (a.final_fitness - base.final_fitness).abs() <= FITNESS_DRIFT_TOLERANCE
        {
            stable += 1;
        }
    }
    stable as f64 / samples as f64
}

#[cfg(test)]
mod tests {
    use super::super::Theory;
    use super::*;
    use crate::cosmology::BackgroundForwardModel;
    use crate::types::ObservableRecord;

    fn desi() -> Vec<ObservableRecord> {
        vec![
            ObservableRecord {
                observable_id: "dm_over_rd@0.510".into(),
                kind: "bao".into(),
                value: 13.62,
                uncertainty: 0.25,
                unit: "dimensionless".into(),
                source: None,
            },
            ObservableRecord {
                observable_id: "bbn_yp".into(),
                kind: "bbn".into(),
                value: 0.2453,
                uncertainty: 0.0034,
                unit: "dimensionless".into(),
                source: None,
            },
        ]
    }

    #[test]
    fn the_baseline_is_robust() {
        let r = perturbation_robustness(
            &Theory::baseline_lcdm(),
            &desi(),
            &BackgroundForwardModel,
            0.0,
            42,
            64,
        );
        assert!(r > 0.8, "baseline robustness = {r}");
    }

    #[test]
    fn a_knife_edge_stability_theory_is_fragile() {
        // q_s barely above the no-ghost boundary: ~half of perturbations push it negative → ghost
        // veto → not credible, so robustness collapses well below the comfortable baseline.
        let mut edgy = Theory::baseline_lcdm();
        edgy.stability.q_s = 0.02;
        let r = perturbation_robustness(&edgy, &desi(), &BackgroundForwardModel, 0.0, 42, 64);
        let base = perturbation_robustness(
            &Theory::baseline_lcdm(),
            &desi(),
            &BackgroundForwardModel,
            0.0,
            42,
            64,
        );
        assert!(
            r < base,
            "knife-edge robustness {r} should be below baseline {base}"
        );
        assert!(r < 0.8, "knife-edge robustness = {r}");
    }

    #[test]
    fn a_vetoed_theory_has_zero_robustness() {
        let mut ghost = Theory::baseline_lcdm();
        ghost.stability.kinetic_coefficient = -1.0;
        let r = perturbation_robustness(&ghost, &desi(), &BackgroundForwardModel, 0.0, 1, 16);
        assert_eq!(r, 0.0);
    }
}
