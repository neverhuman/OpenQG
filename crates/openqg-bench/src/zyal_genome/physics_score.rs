//! V4 M5: the genome's real-physics scorer — the cut-over that retires the synthetic jitter.
//!
//! V3's genome scored candidates with `compute_scores` (a deterministic hash-jitter around ~0.6)
//! that never touched the physics. `score_candidate` replaces it: it runs the candidate `Theory`
//! through the *same* deterministic physics the rest of the engine uses — the veto cascade and the
//! data fit (`evaluate` for ε over the GR baseline, `held_out_evaluate` for the sealed-holdout
//! generalization gap) — assembles a [`DataFitOutcome`], and adjudicates it veto-first against the
//! M3 [`ScorecardV4`] rubric. A vetoed or gate-failing candidate scores 0; survivors get the real
//! 100-point critic-proofness score. Pure given its inputs (the forward model is deterministic), so
//! the verdict replays without the LLM.

use openqg_core::cosmology::BackgroundForwardModel;
use openqg_core::theory::{
    alternating_holdout, evaluate, held_out_evaluate, score as scorecard_score, ClaimGraph,
    DataFitOutcome, DerivationObligation, EvidenceStore, ScorecardV4, Theory, UnificationClaim,
};
use openqg_core::{score_metrics, ObservableRecord};

fn finite_or(x: f64, fallback: f64) -> f64 {
    if x.is_finite() {
        x
    } else {
        fallback
    }
}

/// The GR/ΛCDM baseline log-likelihood on `observables` — ε is measured as improvement over this,
/// so the baseline sits near a neutral score rather than collapsing every candidate to ~0. Mirrors
/// `theory_evolve::baseline_log_likelihood`.
pub(crate) fn baseline_log_likelihood(observables: &[ObservableRecord]) -> f64 {
    use openqg_core::cosmology::{CosmologyParams, ForwardModel};
    let model = BackgroundForwardModel;
    let ids: Vec<String> = observables
        .iter()
        .map(|o| o.observable_id.clone())
        .collect();
    match model.predict(&CosmologyParams::planck_lcdm(), &ids) {
        Ok(preds) => score_metrics(observables, &preds, 3, 0.0).0.log_likelihood,
        Err(_) => 0.0,
    }
}

/// Score one candidate veto-first against the real physics + the M3 rubric. Returns a full
/// [`ScorecardV4`] (disqualified ⇒ `total == 0`).
#[allow(clippy::too_many_arguments)]
pub(crate) fn score_candidate(
    theory: &Theory,
    observables: &[ObservableRecord],
    baseline_log_likelihood: f64,
    claim_graph: &ClaimGraph,
    obligations: &[DerivationObligation],
    unification: &UnificationClaim,
    store: &dyn EvidenceStore,
    evidence_schema: &str,
) -> ScorecardV4 {
    let model = BackgroundForwardModel;

    // Data fit (ε over baseline) + coverage from the deterministic forward model.
    let eval = evaluate(theory, observables, &model, baseline_log_likelihood);

    // Generalization on an alternating sealed split (smaller gap ⇒ more predictive, less overfit).
    let heldout = alternating_holdout(observables.len());
    let held = held_out_evaluate(theory, observables, &model, &heldout);

    // A vetoed candidate has no meaningful data fit; the scorecard's own veto gate will disqualify
    // it regardless, so we pass `None` (DataFit scores 0 with a full-uncertainty band).
    let data_fit = if eval.vetoed {
        None
    } else {
        let epsilon = finite_or(eval.epsilon_delta_log_likelihood.unwrap_or(0.0), 0.0);
        Some(DataFitOutcome {
            // ΔAIC vs baseline: improvement in 2·LL (complexity is separately charged by the
            // scorecard's parsimony component, so we keep this the data-only term).
            delta_aic: -2.0 * epsilon,
            // Δln Z proxy: the per-baseline log-likelihood improvement.
            delta_lnz: epsilon,
            generalization_gap: finite_or(held.generalization_gap, 1.0).max(0.0),
            coverage: finite_or(eval.coverage, 0.0).clamp(0.0, 1.0),
            // Evolved theories carry fixed values (not multistart-fitted), so there is no prior
            // boundary to hit.
            boundary_hit: false,
        })
    };

    scorecard_score(
        theory,
        claim_graph,
        obligations,
        unification,
        store,
        evidence_schema,
        data_fit,
    )
}

/// The genome's headline fitness in `[0,1]`: the scorecard total / 100 (0 if disqualified).
pub(crate) fn final_score_unit(scorecard: &ScorecardV4) -> f64 {
    (scorecard.total / 100.0).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use openqg_core::theory::claim_graph::{Claim, ClaimKind, Sector};
    use openqg_core::theory::{
        ClaimGraph, DerivationObligation, DerivationObligationKind, EvidenceRef, EvidenceTier,
        LimitWitness, MapEvidenceStore, Parameter, Provenance, Theory, UnificationClaim,
    };

    fn obs() -> Vec<ObservableRecord> {
        // A few records so the holdout split is non-trivial; ids the background model may or may
        // not predict — score_candidate is robust either way (guards non-finite, clamps coverage).
        [
            "bao_dv_over_rs_z038",
            "bao_dv_over_rs_z051",
            "fsigma8_z038",
            "fsigma8_z051",
        ]
        .iter()
        .enumerate()
        .map(|(i, id)| ObservableRecord {
            observable_id: (*id).into(),
            kind: "cosmology".into(),
            value: 1.0 + i as f64 * 0.1,
            uncertainty: 0.05,
            unit: "dimensionless".into(),
            source: None,
        })
        .collect()
    }

    fn fixture_graph() -> (
        ClaimGraph,
        Vec<DerivationObligation>,
        UnificationClaim,
        MapEvidenceStore,
    ) {
        let bytes = b"{\"e2\":1.0}\n".to_vec();
        let mut store = MapEvidenceStore::default();
        store.0.insert("bg.json".into(), bytes.clone());
        let ev = EvidenceRef::new(
            "bg.json",
            openqg_core::sha256_digest(&bytes),
            EvidenceTier::T2,
        );
        let claim = Claim {
            id: "bg".into(),
            sector: Sector::Background,
            kind: ClaimKind::Physics,
            statement: "background recovers GR".into(),
            evidence: vec![ev],
            obligations: vec!["bg-ob".into()],
            depends_on: vec![],
        };
        let ob = DerivationObligation {
            claim_id: "bg-ob".into(),
            kind: DerivationObligationKind::Limit,
            detail: "GR limit".into(),
            certificate: None,
            limit: Some(LimitWitness {
                name: "gr".into(),
                residual: 0.0,
                bound: 1e-6,
            }),
            citation: None,
            novel: None,
        };
        (
            ClaimGraph {
                claims: vec![claim],
            },
            vec![ob],
            UnificationClaim { shared: vec![] },
            store,
        )
    }

    #[test]
    fn baseline_survives_with_a_finite_positive_score() {
        let observables = obs();
        let base_ll = baseline_log_likelihood(&observables);
        let (cg, obs_list, uni, store) = fixture_graph();
        let sc = score_candidate(
            &Theory::baseline_lcdm(),
            &observables,
            base_ll,
            &cg,
            &obs_list,
            &uni,
            &store,
            "schema.v1",
        );
        assert!(
            !sc.disqualified,
            "baseline should survive: {:?}",
            sc.kill_reasons
        );
        assert!(sc.total.is_finite() && sc.total > 0.0 && sc.total <= 100.0);
        assert!((0.0..=1.0).contains(&final_score_unit(&sc)));
    }

    #[test]
    fn a_free_parameter_candidate_is_disqualified_to_zero() {
        let observables = obs();
        let base_ll = baseline_log_likelihood(&observables);
        let (cg, obs_list, uni, store) = fixture_graph();
        let mut t = Theory::baseline_lcdm();
        t.parameters.push(Parameter {
            symbol: "w_fit".into(),
            value: -0.9,
            physical_meaning: "fitted knob".into(),
            provenance: Provenance::Free,
        });
        let sc = score_candidate(
            &t,
            &observables,
            base_ll,
            &cg,
            &obs_list,
            &uni,
            &store,
            "schema.v1",
        );
        assert!(sc.disqualified);
        assert_eq!(sc.total, 0.0);
        assert_eq!(final_score_unit(&sc), 0.0);
    }

    #[test]
    fn score_candidate_is_deterministic() {
        let observables = obs();
        let base_ll = baseline_log_likelihood(&observables);
        let (cg, obs_list, uni, store) = fixture_graph();
        let a = score_candidate(
            &Theory::baseline_lcdm(),
            &observables,
            base_ll,
            &cg,
            &obs_list,
            &uni,
            &store,
            "s",
        );
        let b = score_candidate(
            &Theory::baseline_lcdm(),
            &observables,
            base_ll,
            &cg,
            &obs_list,
            &uni,
            &store,
            "s",
        );
        assert_eq!(a, b);
    }
}
