//! Veto-gated dual-objective candidate evaluation (the M3 ε/β fitness core).
//!
//! Every candidate is scored on two axes, after the deterministic veto cascade has gated it:
//! - **ε (epsilon)** — how well it fits the data: `delta_log_likelihood` from the *real* forward
//!   model versus a baseline. None when the theory is vetoed (a broken theory is never scored).
//! - **β (beta)** — how *derivable* it is: a `[0,1]` score rewarding fully-provenanced parameters
//!   and a safe margin from the structural veto thresholds (stability, GW170817). This is what
//!   makes the engine chase real, derivable physics rather than a better curve fit — see
//!   `docs/research/automated-theory-discovery.md` §2 (the AI-Descartes ε/β split).

use super::vetoes::physics_kills;
use super::{Theory, VetoReason};
use crate::cosmology::{CosmologyParams, ForwardModel};
use crate::scoring::score_metrics;
use crate::types::ObservableRecord;

/// The dual-objective evaluation of one candidate theory.
#[derive(Debug, Clone, PartialEq)]
pub struct Evaluation {
    /// True if the deterministic veto cascade killed the theory (then ε is `None`).
    pub vetoed: bool,
    /// The veto reasons (empty unless `vetoed`).
    pub veto_reasons: Vec<VetoReason>,
    /// ε: improvement in data log-likelihood over the baseline (`delta_log_likelihood`).
    pub epsilon_delta_log_likelihood: Option<f64>,
    /// Raw data log-likelihood (None if vetoed).
    pub log_likelihood: Option<f64>,
    /// Fraction of requested observables the forward model could actually derive.
    pub coverage: f64,
    /// β: derivation/consistency score in `[0,1]` (always computed; it is a property of the
    /// theory's structure, independent of the data fit).
    pub beta: f64,
}

impl Evaluation {
    /// A scalar fitness for selection consumers that want one number: 0 for a vetoed theory,
    /// otherwise the data-fit improvement squashed to `[0,1]` and **weighted by β** so a better
    /// fit only counts if the theory is derivable. M3's selection may refine this blend, but the
    /// β-gating of ε is the invariant: undeliverable theories cannot win on fit alone.
    pub fn combined_fitness(&self) -> f64 {
        match self.epsilon_delta_log_likelihood {
            None => 0.0,
            Some(delta) => {
                let fit = 1.0 / (1.0 + (-0.12 * delta).exp()); // monotone, plateau-free
                (fit * self.beta).clamp(0.0, 1.0)
            }
        }
    }
}

/// Compute β (derivation/consistency) for a theory, in `[0,1]`.
pub fn derivation_score(theory: &Theory) -> f64 {
    // Provenance completeness: fraction of parameters that are whitebox (fundamental / derived).
    let provenance = if theory.parameters.is_empty() {
        1.0
    } else {
        let whitebox = theory
            .parameters
            .iter()
            .filter(|p| p.provenance.is_whitebox())
            .count();
        whitebox as f64 / theory.parameters.len() as f64
    };
    // Stability margin: how safely inside the no-ghost / no-gradient-instability region it sits.
    let s = &theory.stability;
    let min_stab = s
        .kinetic_coefficient
        .min(s.q_s)
        .min(s.sound_speed_sq)
        .max(0.0);
    let stability_margin = min_stab / (min_stab + 0.5);
    // GW170817 margin: distance of the tensor-speed excess from zero, normalized to the veto band.
    let alpha_t_margin = (1.0 - theory.alpha.alpha_t.abs() / 1e-2).clamp(0.0, 1.0);
    (0.4 * provenance + 0.4 * stability_margin + 0.2 * alpha_t_margin).clamp(0.0, 1.0)
}

/// Evaluate a candidate: run the veto cascade, and only if it survives, run the forward model on
/// the theory's background and score it against the data. β is always reported.
pub fn evaluate<M>(
    theory: &Theory,
    observables: &[ObservableRecord],
    model: &M,
    baseline_log_likelihood: f64,
) -> Evaluation
where
    M: ForwardModel<Theory = CosmologyParams>,
{
    let beta = derivation_score(theory);
    let veto_reasons = physics_kills(theory);
    if !veto_reasons.is_empty() {
        return Evaluation {
            vetoed: true,
            veto_reasons,
            epsilon_delta_log_likelihood: None,
            log_likelihood: None,
            coverage: 0.0,
            beta,
        };
    }

    let ids: Vec<String> = observables
        .iter()
        .map(|o| o.observable_id.clone())
        .collect();
    // V5: predict on the truth-BOUND background, so certified modifications actually drive
    // the computed observables (binding is pure + idempotent; vetoes already ran above).
    let bound = super::binding::bind_modified_background(theory).theory;
    let predictions = match model.predict(&bound.background, &ids) {
        Ok(p) => p,
        // A forward-model failure (e.g. a pathological background) is a lethal candidate.
        Err(_) => {
            return Evaluation {
                vetoed: true,
                veto_reasons: Vec::new(),
                epsilon_delta_log_likelihood: None,
                log_likelihood: None,
                coverage: 0.0,
                beta,
            }
        }
    };
    let (metrics, _) = score_metrics(
        observables,
        &predictions,
        theory.parameters.len().max(1),
        baseline_log_likelihood,
    );
    Evaluation {
        vetoed: false,
        veto_reasons: Vec::new(),
        epsilon_delta_log_likelihood: Some(metrics.delta_log_likelihood),
        log_likelihood: Some(metrics.log_likelihood),
        coverage: metrics.coverage,
        beta,
    }
}

#[cfg(test)]
mod tests {
    use super::super::mutation::{inject_free_parameter, Rng};
    use super::super::Theory;
    use super::*;
    use crate::cosmology::BackgroundForwardModel;

    fn desi() -> Vec<ObservableRecord> {
        // A couple of real DESI DR1 BAO points, inline for a self-contained unit test.
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
                observable_id: "dh_over_rd@0.510".into(),
                kind: "bao".into(),
                value: 20.98,
                uncertainty: 0.61,
                unit: "dimensionless".into(),
                source: None,
            },
        ]
    }

    #[test]
    fn baseline_is_scored_with_high_beta() {
        let obs = desi();
        let model = BackgroundForwardModel;
        let e = evaluate(&Theory::baseline_lcdm(), &obs, &model, 0.0);
        assert!(!e.vetoed);
        assert!(e.log_likelihood.unwrap().is_finite());
        assert!((e.coverage - 1.0).abs() < 1e-9);
        assert!(e.beta > 0.8, "baseline beta = {}", e.beta);
        assert!(e.combined_fitness() > 0.0);
    }

    #[test]
    fn a_gray_box_theory_is_vetoed_and_unscored_with_zero_fitness() {
        let obs = desi();
        let model = BackgroundForwardModel;
        let gray = inject_free_parameter(&Theory::baseline_lcdm(), "f_ede", 0.07);
        let e = evaluate(&gray, &obs, &model, 0.0);
        assert!(e.vetoed);
        assert!(e.epsilon_delta_log_likelihood.is_none());
        assert_eq!(e.combined_fitness(), 0.0);
        // β reflects the gray-box parameter even though it was vetoed: it drops below baseline.
        let baseline_beta = derivation_score(&Theory::baseline_lcdm());
        assert!(
            e.beta < baseline_beta,
            "gray-box beta {} should drop below baseline {}",
            e.beta,
            baseline_beta
        );
    }

    #[test]
    fn beta_rewards_safer_stability_margins() {
        let safe = Theory::baseline_lcdm();
        let mut edgy = Theory::baseline_lcdm();
        edgy.stability.q_s = 0.01; // barely non-ghost
        assert!(derivation_score(&safe) > derivation_score(&edgy));
    }

    #[test]
    fn a_better_background_fit_raises_epsilon_over_a_detuned_one() {
        let obs = desi();
        let model = BackgroundForwardModel;
        let good = evaluate(&Theory::baseline_lcdm(), &obs, &model, 0.0);
        let mut detuned = Theory::baseline_lcdm();
        detuned.background = {
            let mut c = detuned.background.clone();
            c.omega_m = 0.45;
            c
        };
        let bad = evaluate(&detuned, &obs, &model, 0.0);
        let _ = Rng::new(1); // (determinism util available to the engine)
        assert!(
            good.epsilon_delta_log_likelihood.unwrap() > bad.epsilon_delta_log_likelihood.unwrap()
        );
    }
}
