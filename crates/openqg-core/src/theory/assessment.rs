//! Capstone candidate assessment: the full verdict the rebuilt engine selects on, combining the
//! veto-gated cosmology fit (ε weighted by β) with the cross-domain unification channel.
//!
//! The point: a theory that merely *fits* the cosmological data is not credible if it is
//! inconsistent in another domain. Some inconsistencies are hard vetoes (ghosts, GW170817,
//! unscreened PPN); others — notably BBN abundances and the GW-friction siren ratio — are *not*
//! structural vetoes but still disqualify a "unified" theory. `final_fitness` gates the cosmology
//! fit by the weakest cross-domain sector, so a candidate that fits BAO yet over-produces helium
//! scores near zero. This is what "beyond critique by stern outside experts" means operationally.

use super::{evaluate, unification_report, Evaluation, Theory, UnificationReport};
use crate::cosmology::{CosmologyParams, ForwardModel};
use crate::types::ObservableRecord;

/// The combined verdict on a candidate theory.
#[derive(Debug, Clone, PartialEq)]
pub struct CandidateAssessment {
    /// Veto-gated dual-objective cosmology evaluation (ε/β).
    pub evaluation: Evaluation,
    /// Cross-domain unification report (orthogonal to the cosmology fit).
    pub unification: UnificationReport,
    /// Final fitness in `[0,1]`: the β-weighted cosmology fit, gated by the weakest cross-domain
    /// sector. Zero if vetoed; low if any domain is inconsistent even when the data fit is good.
    pub final_fitness: f64,
}

impl CandidateAssessment {
    /// A candidate is a credible unified theory only if it survives the vetoes, fits the data, and
    /// clears every cross-domain sector.
    pub fn is_credible(&self) -> bool {
        !self.evaluation.vetoed && self.unification.is_unified() && self.final_fitness > 0.0
    }
}

/// Assess a candidate end-to-end: cosmology evaluation × cross-domain unification.
pub fn assess<M>(
    theory: &Theory,
    observables: &[ObservableRecord],
    model: &M,
    baseline_log_likelihood: f64,
) -> CandidateAssessment
where
    M: ForwardModel<Theory = CosmologyParams>,
{
    let evaluation = evaluate(theory, observables, model, baseline_log_likelihood);
    let unification = unification_report(theory);
    let final_fitness = evaluation.combined_fitness() * unification.score;
    CandidateAssessment {
        evaluation,
        unification,
        final_fitness,
    }
}

#[cfg(test)]
mod tests {
    use super::super::mutation::inject_free_parameter;
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
    fn baseline_is_credible() {
        let a = assess(
            &Theory::baseline_lcdm(),
            &desi(),
            &BackgroundForwardModel,
            0.0,
        );
        assert!(a.is_credible());
        assert!(a.final_fitness > 0.0);
    }

    #[test]
    fn gray_box_is_not_credible() {
        let gray = inject_free_parameter(&Theory::baseline_lcdm(), "f_ede", 0.07);
        let a = assess(&gray, &desi(), &BackgroundForwardModel, 0.0);
        assert!(!a.is_credible());
        assert_eq!(a.final_fitness, 0.0);
    }

    #[test]
    fn a_theory_that_fits_cosmology_but_breaks_bbn_is_not_credible() {
        // n_eff=4.5 is NOT a structural veto — the cascade passes it and it still produces a real
        // cosmology fit — but it over-produces primordial helium, so the cross-domain channel
        // disqualifies it. The data fit alone is not enough.
        let mut hot = Theory::baseline_lcdm();
        hot.background.n_eff = 4.5;
        let a = assess(&hot, &desi(), &BackgroundForwardModel, 0.0);
        assert!(!a.evaluation.vetoed, "n_eff is not a hard veto");
        assert!(
            a.evaluation.log_likelihood.is_some(),
            "it still gets a cosmology fit"
        );
        assert!(!a.unification.is_unified(), "but BBN cross-domain fails");
        assert_eq!(
            a.final_fitness, 0.0,
            "so its final fitness is gated to zero"
        );
        assert!(!a.is_credible());
    }
}
