//! V4 M5: the genome's real-physics scorer — the cut-over that retires the synthetic jitter.
//!
//! V3's genome scored candidates with `compute_scores` (a deterministic hash-jitter around ~0.6)
//! that never touched the physics. `score_candidate` replaces it: it runs the candidate `Theory`
//! through the *same* deterministic physics the rest of the engine uses — the veto cascade and the
//! data fit (`evaluate` for ε over the GR baseline, `split_evaluate` for the sealed-holdout
//! generalization gap) — assembles a [`DataFitOutcome`], and adjudicates it veto-first against the
//! M3 [`ScorecardV4`] rubric. A vetoed or gate-failing candidate scores 0; survivors get the real
//! 100-point critic-proofness score. Pure given its inputs (the forward model is deterministic), so
//! the verdict replays without the LLM.

use openqg_core::cosmology::BackgroundForwardModel;
use openqg_core::theory::{
    alternating_split, evaluate_with_blocks, score_with_observables as scorecard_score,
    split_evaluate, ClaimGraph, DataFitOutcome, DerivationObligation, EvidenceStore, ScorecardV4,
    Theory, UnificationClaim,
};
use openqg_core::ObservableRecord;

fn finite_or(x: f64, default_value: f64) -> f64 {
    if x.is_finite() {
        x
    } else {
        default_value
    }
}

/// The GR/ΛCDM baseline log-likelihood on `observables` — ε is measured as improvement over this,
/// so the baseline sits near a neutral score rather than collapsing every candidate to ~0. Mirrors
/// `theory_evolve::baseline_log_likelihood`.
#[cfg_attr(not(test), allow(dead_code))] // diagonal helper kept for tests + tooling
pub(crate) fn baseline_log_likelihood(observables: &[ObservableRecord]) -> f64 {
    baseline_log_likelihood_cov(observables, &[])
}

/// V6: the baseline must be scored in the SAME likelihood mode as the candidates, or Δln Z is
/// meaningless across modes.
pub(crate) fn baseline_log_likelihood_cov(
    observables: &[ObservableRecord],
    blocks: &[openqg_core::scoring::CovarianceBlock],
) -> f64 {
    use openqg_core::cosmology::{CosmologyParams, ForwardModel};
    use openqg_core::scoring::{score_metrics_cov, LikelihoodData};
    let model = BackgroundForwardModel;
    let ids: Vec<String> = observables
        .iter()
        .map(|o| o.observable_id.clone())
        .collect();
    match model.predict(&CosmologyParams::planck_lcdm(), &ids) {
        Ok(preds) => {
            let data = LikelihoodData {
                observables: observables.to_vec(),
                blocks: blocks.to_vec(),
            };
            score_metrics_cov(&data, &preds, 3, 0.0).0.log_likelihood
        }
        Err(_) => 0.0,
    }
}

/// Score one candidate veto-first against the real physics + the M3 rubric. Returns a full
/// [`ScorecardV4`] (disqualified ⇒ `total == 0`).
#[allow(clippy::too_many_arguments)]
pub(crate) fn score_candidate(
    theory: &Theory,
    observables: &[ObservableRecord],
    blocks: &[openqg_core::scoring::CovarianceBlock],
    baseline_log_likelihood: f64,
    claim_graph: &ClaimGraph,
    obligations: &[DerivationObligation],
    unification: &UnificationClaim,
    store: &dyn EvidenceStore,
    evidence_schema: &str,
) -> ScorecardV4 {
    let model = BackgroundForwardModel;

    // Data fit (ε over baseline) + coverage from the deterministic forward model.
    let eval = evaluate_with_blocks(theory, observables, blocks, &model, baseline_log_likelihood);

    // Generalization on an alternating sealed split (smaller gap ⇒ more predictive, less overfit).
    let heldout = alternating_split(observables.len());
    let held = split_evaluate(theory, observables, &model, &heldout);

    // A vetoed candidate has no meaningful data fit; the scorecard's own veto gate will disqualify
    // it regardless, so we pass `None` (DataFit scores 0 with a full-uncertainty band).
    let data_fit = if eval.vetoed {
        None
    } else {
        let epsilon = finite_or(eval.epsilon_delta_log_likelihood.unwrap_or(0.0), 0.0);
        // V6.1 (P0.10): a real evidence proxy — the BIC/Laplace Occam term charges every free
        // dial (parameters AND drifted background coordinates) against the data improvement.
        let k = openqg_core::total_free_dof(theory) as f64;
        // V7 (review-05): effective independent modes, not record count — correlated blocks
        // carry less information than their member count suggests.
        let n_eff = {
            let data = openqg_core::scoring::LikelihoodData {
                observables: observables.to_vec(),
                blocks: blocks.to_vec(),
            };
            (openqg_core::scoring::effective_modes(&data) as f64).max(1.0)
        };
        let occam = 0.5 * k * n_eff.ln();
        Some(DataFitOutcome {
            // ΔAIC vs baseline with the 2k complexity term restored.
            delta_aic: -2.0 * epsilon + 2.0 * k,
            // Δln Z proxy: per-baseline log-likelihood improvement minus the Occam factor.
            delta_lnz: epsilon - occam,
            // V6.1: the gap is honest — a negative (heldout fits BETTER) is information, not
            // something to clamp into a perfect score.
            generalization_gap: finite_or(held.generalization_gap, 1.0),
            coverage: finite_or(eval.coverage, 0.0).clamp(0.0, 1.0),
            // Evolved theories carry fixed values (not multistart-fitted), so there is no prior
            // boundary to hit.
            boundary_hit: false,
            likelihood_mode: if blocks.is_empty() {
                openqg_core::LikelihoodMode::Diagonal
            } else {
                openqg_core::LikelihoodMode::Covariance
            },
            covariance_block_count: blocks.len() as u32,
            n_observations: observables.len() as u32,
            gof_outcome: None,
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
        observables,
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
            equation_match: None,
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
            &[],
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
            &[],
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
            &[],
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
            &[],
            base_ll,
            &cg,
            &obs_list,
            &uni,
            &store,
            "s",
        );
        assert_eq!(a, b);
    }

    /// The honest V7 version of the V5 "breakthrough is possible" smoke test. The certified
    /// suppressed-growth theory (planck_mu0_geff, μ0 = −0.1) is truth-bound and improves the RAW
    /// likelihood on the real low-fσ8/S8 records (the mechanism points the right way) — but
    /// under the V7 evidence economics its one post-search-chosen dof (μ0) must clear the BIC
    /// bar 0.5·ln(n)≈0.9 nat on n=6 points, and ~0.5 nat of raw gain does not. So: raw fit
    /// improves, data_fit credit is ZERO, and that is the honest state of this mechanism at
    /// this data volume (review-05's economics + review-09's "more growth data" in one test).
    #[test]
    fn suppressed_growth_genuinely_beats_lcdm_on_real_tension_data() {
        use openqg_core::theory::{evaluate, DerivedCertificate, Provenance};

        // Real records (growth-rsd.jsonl + wl-s8.jsonl, cited sources in the fixtures).
        let tension: Vec<ObservableRecord> = [
            ("fsigma8@0.067", 0.423, 0.055),
            ("fsigma8@0.38", 0.497, 0.045),
            ("fsigma8@0.51", 0.458, 0.038),
            ("fsigma8@0.61", 0.436, 0.034),
            ("fsigma8@1.48", 0.462, 0.045),
            ("s8", 0.776, 0.017),
        ]
        .iter()
        .map(|(id, v, u)| ObservableRecord {
            observable_id: (*id).into(),
            kind: "growth".into(),
            value: *v,
            uncertainty: *u,
            unit: "dimensionless".into(),
            source: None,
        })
        .collect();

        let base_ll = baseline_log_likelihood(&tension);

        // A certified suppressed-growth theory: G_eff/G = 0.9 via the Planck-2018 μ0 parametrization.
        let mut t = Theory::baseline_lcdm();
        t.terms.push(openqg_core::Term {
            name: "planck_mu_parametrization".into(),
            mass_dimension: 4,
            free_lorentz_indices: 0,
        });
        t.id = "suppressed-growth-smoke".into();
        let cert = DerivedCertificate {
            relation: "planck_mu0_geff".into(),
            inputs: vec![("mu0".into(), -0.1)],
            expected: 0.9,
            tolerance: 1e-9,
            ..Default::default()
        };
        t.parameters.push(openqg_core::theory::Parameter {
            symbol: "geff_over_g".into(),
            value: 0.9,
            physical_meaning: "suppressed effective coupling (Planck-2018 μ0 parametrization)"
                .into(),
            provenance: Provenance::derived_certified("Planck 2018 MG μ0", cert),
        });

        let model = BackgroundForwardModel;
        let eval = evaluate(&t, &tension, &model, base_ll);
        assert!(!eval.vetoed, "{:?}", eval.veto_reasons);
        let epsilon = eval
            .epsilon_delta_log_likelihood
            .expect("scored candidate has epsilon");
        assert!(
            epsilon > 0.0,
            "suppressed growth must BEAT the ΛCDM baseline on low-fσ8/S8 data (ε = {epsilon})"
        );

        // And the rubric must pay for it: data_fit > 0 through the full scorecard path.
        let (cg, obs_list, uni, store) = fixture_graph();
        let sc = score_candidate(
            &t,
            &tension,
            &[],
            base_ll,
            &cg,
            &obs_list,
            &uni,
            &store,
            "s",
        );
        assert!(!sc.disqualified, "{:?}", sc.kill_reasons);
        // The mechanism genuinely improves the raw fit: ΔAIC = −2ε + 2k with k = 1 ⇒ raw ε > 0
        // iff ΔAIC < 2.
        let data = sc.data_fit.expect("data fit computed");
        assert!(
            data.delta_aic < 2.0,
            "the raw likelihood must improve (ΔAIC − 2k < 0): {:?}",
            data
        );
        // ...but it does NOT clear the Occam evidence bar at n=6, k=1 — data credit is zero.
        let df = sc.components.iter().find(|c| c.name == "data_fit").unwrap();
        assert!(
            df.points == 0.0 && data.delta_lnz < 0.0,
            "below the evidence bar there is no data credit: {:?} / {:?}",
            df,
            data
        );
        assert!(sc.distinct_from_baseline);
    }
}

#[cfg(test)]
mod v6_gate_tests {
    use openqg_core::{physics_kills, Theory, VetoReason};

    // V5 campaign champion (thy-m22ea) inlined. Source file deleted in Wave 0.6 artifact hygiene.
    // alpha_m=-0.35 / Vainshtein screening declared but screening_recovery=null / drifted background.
    const V5_CHAMPION_JSON: &str = r#"{
  "alpha": {
    "alpha_b": -0.009130982153605888,
    "alpha_k": -0.47755097529613977,
    "alpha_m": -0.3488053977712303,
    "alpha_t": 0.0
  },
  "background": {
    "fr_log10_fr0": -30.0,
    "fr_n": 1.0,
    "h": 0.7176575652249619,
    "mg_family": "none",
    "mu0": 0.0,
    "n_eff": 3.046,
    "ndgp_omega_rc": 0.0,
    "omega_b_h2": 0.02237,
    "omega_k": 0.0,
    "omega_m": 0.2822055296326261,
    "sigma8": 0.811,
    "sum_mnu": 0.06,
    "w0": -1.1443535961059987,
    "wa": 0.0
  },
  "id": "thy-m22ea",
  "parameters": [
    {"physical_meaning": "present-day expansion rate",  "provenance": "fundamental", "symbol": "H0",      "value": 67.4},
    {"physical_meaning": "total matter density today",  "provenance": "fundamental", "symbol": "Omega_m", "value": 0.315}
  ],
  "screening": "vainshtein",
  "screening_recovery": null,
  "stability": {"has_nondegenerate_higher_derivatives": false, "kinetic_coefficient": 1.0, "q_s": 1.0, "sound_speed_sq": 1.0},
  "terms": [
    {"free_lorentz_indices": 0, "mass_dimension": 4, "name": "einstein_hilbert"},
    {"free_lorentz_indices": 0, "mass_dimension": 4, "name": "cosmological_constant"}
  ]
}"#;

    /// THE V6 acceptance regression (review-01's go/no-go): the V5 campaign champion — a
    /// 54.999-point candidate with unprovenanced alpha drift, a bare screening label
    /// (screening_recovery=null), and a drifted background — must be rejected by the unified
    /// physics gate. If this candidate ever passes again, V6 has regressed to V5's exploit.
    #[test]
    fn the_v5_champion_is_rejected_by_the_unified_gate() {
        let raw = V5_CHAMPION_JSON;
        let theory: Theory = serde_json::from_str(raw).expect("the vendored champion parses");
        let kills = physics_kills(&theory);
        assert!(
            kills
                .iter()
                .any(|k| matches!(k, VetoReason::ScreeningRecoveryUnquantified { .. })),
            "expected ScreeningRecoveryUnquantified, got {kills:?}"
        );
    }

    /// The inverse guard: a clean, *quantified* suppressed-growth theory (certified mu0 < 0, no
    /// alpha modification, GR-physical background) must still pass — the gate kills exploits,
    /// not legitimate phenomenology.
    #[test]
    fn a_quantified_suppressed_growth_theory_still_passes() {
        let mut theory = Theory::baseline_lcdm();
        theory.id = "v6-clean-suppressed-growth".into();
        theory.background.mu0 = -0.1;
        theory.terms.push(openqg_core::Term {
            name: "planck_mu_parametrization".into(),
            mass_dimension: 4,
            free_lorentz_indices: 0,
        });

        theory.parameters.push(openqg_core::Parameter {
            symbol: "geff_today".into(),
            value: 0.9,
            physical_meaning: "G_eff/G at a=1 (suppressed growth)".into(),
            provenance: openqg_core::Provenance::derived_certified(
                "Planck 2018 mu0 parametrization",
                openqg_core::DerivedCertificate {
                    relation: "planck_mu0_geff".into(),
                    inputs: vec![("mu0".into(), -0.1)],
                    expected: 0.9,
                    tolerance: 1e-9,
                    ..Default::default()
                },
            ),
        });
        let kills = physics_kills(&theory);
        assert!(kills.is_empty(), "clean theory killed: {kills:?}");
    }
}

#[cfg(test)]
mod v6_drift_tests {
    use openqg_core::{background_dof, Theory};

    // V5 champion inlined (same source as v6_gate_tests::V5_CHAMPION_JSON — duplicated here
    // because sibling test modules cannot share private items without pub re-exports).
    const V5_CHAMPION_JSON: &str = r#"{
  "alpha": {"alpha_b": -0.009130982153605888, "alpha_k": -0.47755097529613977, "alpha_m": -0.3488053977712303, "alpha_t": 0.0},
  "background": {"fr_log10_fr0": -30.0, "fr_n": 1.0, "h": 0.7176575652249619, "mg_family": "none", "mu0": 0.0, "n_eff": 3.046, "ndgp_omega_rc": 0.0, "omega_b_h2": 0.02237, "omega_k": 0.0, "omega_m": 0.2822055296326261, "sigma8": 0.811, "sum_mnu": 0.06, "w0": -1.1443535961059987, "wa": 0.0},
  "id": "thy-m22ea",
  "parameters": [
    {"physical_meaning": "present-day expansion rate", "provenance": "fundamental", "symbol": "H0", "value": 67.4},
    {"physical_meaning": "total matter density today", "provenance": "fundamental", "symbol": "Omega_m", "value": 0.315}
  ],
  "screening": "vainshtein", "screening_recovery": null,
  "stability": {"has_nondegenerate_higher_derivatives": false, "kinetic_coefficient": 1.0, "q_s": 1.0, "sound_speed_sq": 1.0},
  "terms": [
    {"free_lorentz_indices": 0, "mass_dimension": 4, "name": "einstein_hilbert"},
    {"free_lorentz_indices": 0, "mass_dimension": 4, "name": "cosmological_constant"}
  ]
}"#;

    /// V6 P2: background drift is a costed degree of freedom. The V5 champions moved h/Ω_m/w0
    /// with zero parsimony cost; each moved coordinate now counts.
    #[test]
    fn background_drift_costs_parsimony_dof() {
        let mut t = Theory::baseline_lcdm();
        assert_eq!(
            background_dof(&t),
            0,
            "the un-drifted baseline costs nothing"
        );
        t.background.h = 0.7176;
        t.background.omega_m = 0.2822;
        t.background.w0 = -1.144;
        t.terms.push(openqg_core::Term {
            name: "quintessence_scalar".into(),
            mass_dimension: 4,
            free_lorentz_indices: 0,
        });
        assert_eq!(background_dof(&t), 3, "three moved dials = three dof");
    }

    /// And the vendored V5 champion itself pays: its drift is no longer invisible.
    #[test]
    fn the_v5_champion_pays_background_dof() {
        let theory: Theory = serde_json::from_str(V5_CHAMPION_JSON).unwrap();
        assert!(
            background_dof(&theory) >= 3,
            "the champion moved h/omega_m/w0 (+ sigma8/wa drift): got {}",
            background_dof(&theory)
        );
    }
}

#[cfg(test)]
mod v6_covariance_tests {
    use super::*;
    use crate::zyal_genome::run_population::{load_covariance_blocks, load_observables};
    use crate::zyal_genome::theory_population::score_theory;
    use std::path::PathBuf;

    /// THE V6.1 RESOLUTION of the degeneracy-valley saga, in three acts (all permanent record):
    /// V5 diagonal: +54.9 nat (champion class). V6 covariance: +68.8 (the TRUE correlated 3×3
    /// was even cheaper along the valley — diagonal had over-stated the CMB). V6.1: the +0.755
    /// (8.4σ) engine lA bias is anchor-calibrated out (P0.11) and the Occam term charges the
    /// drift's 3 dof (P0.10) — the valley CLOSES: diag −20.5, cov −31.9. The "champion
    /// direction" loses to ΛCDM on the admitted data, and covariance now punishes it HARDER
    /// than diagonal (the correlated CMB block has real teeth once the model bias is gone).
    /// What looked like an open valley was ~35 nat of our own fitting-formula error.
    #[test]
    fn covariance_mode_is_recorded_and_the_compressed_cmb_cannot_close_the_valley() {
        let obs = load_observables(&PathBuf::from(
            "../../data/fixtures/cosmology/tier1-multisector.jsonl",
        ))
        .expect("observables");
        let blocks = load_covariance_blocks(&[
            PathBuf::from("../../data/fixtures/cosmology/covariance/planck18-distance-priors.json"),
            PathBuf::from("../../data/fixtures/cosmology/covariance/desi-dr1-bao.json"),
        ])
        .expect("covariance fixtures");
        assert!(blocks.len() >= 6, "expected planck + 5 DESI blocks");

        // The degeneracy move WITHOUT the screening/alpha exploit baggage (pure background shift).
        let mut t = Theory::baseline_lcdm();
        t.id = "degeneracy-direction".into();
        t.background.h = 0.7176;
        t.background.omega_m = 0.2822;
        t.background.w0 = -1.144;
        t.terms.push(openqg_core::Term {
            name: "quintessence_scalar".into(),
            mass_dimension: 4,
            free_lorentz_indices: 0,
        });

        let base_diag = baseline_log_likelihood(&obs);
        let sc_diag = score_theory(&t, &obs, &[], base_diag);
        let base_cov = baseline_log_likelihood_cov(&obs, &blocks);
        let sc_cov = score_theory(&t, &obs, &blocks, base_cov);

        let dlnz_diag = sc_diag
            .data_fit
            .as_ref()
            .map(|d| d.delta_lnz)
            .unwrap_or(0.0);
        let dlnz_cov = sc_cov.data_fit.as_ref().map(|d| d.delta_lnz).unwrap_or(0.0);
        // The blocks must actually engage (different number), both finite — and the V6.1
        // empirical truth is recorded: the valley is CLOSED (both negative) and the calibrated
        // covariance punishes the drift harder than diagonal.
        assert!(dlnz_diag.is_finite() && dlnz_cov.is_finite());
        assert!(
            (dlnz_cov - dlnz_diag).abs() > 1.0,
            "blocks must change the likelihood: diag {dlnz_diag:.2} vs cov {dlnz_cov:.2}"
        );
        assert!(
            dlnz_diag < 0.0 && dlnz_cov < dlnz_diag,
            "the valley is closed and covariance bites harder: diag {dlnz_diag:.2} vs cov {dlnz_cov:.2}"
        );
        // And the mode is on the record — a headline number can never hide its likelihood again.
        let cov_fit = sc_cov.data_fit.as_ref().unwrap();
        assert_eq!(
            cov_fit.likelihood_mode,
            openqg_core::LikelihoodMode::Covariance
        );
        assert_eq!(cov_fit.covariance_block_count as usize, blocks.len());
        println!("delta_lnz: diagonal {dlnz_diag:.2} -> covariance {dlnz_cov:.2}");
    }
}

#[cfg(test)]
mod v6_novelty_tests {
    use super::*;
    use crate::zyal_genome::theory_population::score_theory;
    use openqg_core::theory::{DerivationObligation, DerivationObligationKind};
    use openqg_core::{NovelPredictionWitness, Theory};

    fn obs() -> Vec<ObservableRecord> {
        ["fsigma8@0.51", "s8", "h0"]
            .iter()
            .enumerate()
            .map(|(i, id)| ObservableRecord {
                observable_id: id.to_string(),
                kind: "growth".into(),
                value: 0.46 + i as f64,
                uncertainty: 0.03,
                unit: "dimensionless".into(),
                source: None,
            })
            .collect()
    }

    /// V6 P4: the α-jitter half-novelty faucet is closed — distinct-with-no-witness earns ZERO.
    /// (The V5 champions banked 10 free points exactly here.)
    #[test]
    fn alpha_jitter_without_witness_earns_zero_novelty() {
        let mut t = Theory::baseline_lcdm();
        t.id = "alpha-jitter".into();
        t.alpha.alpha_m = 0.05; // distinct via modifies_gravity
        t.terms.push(openqg_core::Term {
            name: "horndeski_scalar".into(),
            mass_dimension: 4,
            free_lorentz_indices: 0,
        });
        // chameleon is density-threshold based: always structurally plausible even with alpha_b=0
        // (vainshtein requires |alpha_b| >= 0.01, which would kill this test theory)
        t.screening = Some("chameleon".into());
        t.screening_recovery = Some(0.999_999); // quantified — passes the unified gate
        let observables = obs();
        let sc = score_theory(&t, &observables, &[], baseline_log_likelihood(&observables));
        assert!(
            !sc.disqualified,
            "theory must survive the veto cascade; kill_reasons: {:?}",
            sc.kill_reasons
        );
        let nov = sc
            .components
            .iter()
            .find(|c| c.name == "novel_prediction")
            .unwrap();
        assert_eq!(
            nov.points, 0.0,
            "no witness => no novelty, got {}",
            nov.points
        );
    }

    /// V6 P4: declaring numbers >3× the engine-clamped tolerance from the computed truth is a
    /// FABRICATION KILL, not a demotion.
    #[test]
    fn fabricated_witness_values_are_a_kill() {
        let mut t = Theory::baseline_lcdm();
        t.id = "fabricator".into();
        t.background.mu0 = -0.1; // genuinely suppressed growth...
        t.terms.push(openqg_core::Term {
            name: "planck_mu_parametrization".into(),
            mass_dimension: 4,
            free_lorentz_indices: 0,
        });

        t.parameters.push(openqg_core::Parameter {
            symbol: "geff_today".into(),
            value: 0.9,
            physical_meaning: "G_eff/G at a=1".into(),
            provenance: openqg_core::Provenance::derived_certified(
                "Planck 2018 mu0 parametrization",
                openqg_core::DerivedCertificate {
                    relation: "planck_mu0_geff".into(),
                    inputs: vec![("mu0".into(), -0.1)],
                    expected: 0.9,
                    tolerance: 1e-9,
                    ..Default::default()
                },
            ),
        }); // ...properly certified — so the FABRICATED WITNESS is what kills, nothing else
        let obligations = vec![DerivationObligation {
            claim_id: "fab-claim".into(),
            kind: DerivationObligationKind::NovelPrediction,
            detail: "fabricated suppression".into(),
            certificate: None,
            limit: None,
            citation: None,
            novel: Some(NovelPredictionWitness {
                refreshed_by_engine: false,
                observable: "fsigma8@0.51".into(),
                predicted: 0.20, // wildly false — the model computes ~0.44 for mu0=-0.1
                baseline: 0.47,
                min_detectable: 0.01,
                falsifier: "DESI Y5 fsigma8".into(),
            }),
            equation_match: None,
        }];
        let observables = obs();
        let sc = score_candidate(
            &t,
            &observables,
            &[],
            baseline_log_likelihood(&observables),
            &openqg_core::ClaimGraph { claims: vec![] },
            &obligations,
            &openqg_core::UnificationClaim { shared: vec![] },
            &openqg_core::MapEvidenceStore::default(),
            "schema.v1",
        );
        assert!(sc.disqualified, "fabrication must DQ");
        assert!(
            sc.kill_reasons
                .iter()
                .any(|k| k.contains("fabricated novel prediction")),
            "kill reasons: {:?}",
            sc.kill_reasons
        );
    }
}

#[cfg(test)]
mod v7_gate_tests {
    use openqg_core::{physics_kills, Theory, VetoReason};

    // V6 campaign survivor (ndgp-proposed-fixed-rc) inlined. Source file deleted in Wave 0.6
    // artifact hygiene. Bound-nDGP coupling with no brane term — rejected by the term registry.
    const V6_SURVIVOR_JSON: &str = r#"{
  "alpha": {"alpha_b": 0.0, "alpha_k": 0.0, "alpha_m": 0.0, "alpha_t": 0.0},
  "background": {"drag_a": 0.0, "fr_log10_fr0": -30.0, "fr_n": 1.0, "h": 0.674, "mg_family": "none", "mu0": 0.0, "n_eff": 3.046, "ndgp_omega_rc": 0.0, "omega_b_h2": 0.02237, "omega_k": 0.0, "omega_m": 0.315, "sigma8": 0.811, "sum_mnu": 0.06, "w0": -1.0, "wa": 0.0},
  "id": "ndgp-proposed-fixed-rc",
  "parameters": [
    {"physical_meaning": "present-day expansion rate", "provenance": "fundamental", "symbol": "H0", "value": 67.4},
    {"physical_meaning": "total matter density today", "provenance": "fundamental", "symbol": "Omega_m", "value": 0.315},
    {"physical_meaning": "linear effective gravitational coupling (nDGP normal branch)",
     "provenance": {"derived": {"certificate": {"expected": 1.1666666666666667, "inputs": [["beta", 2.0]], "relation": "ndgp_geff_over_g", "tolerance": 0.000001}, "mechanism": "nDGP braneworld linear coupling"}},
     "symbol": "geff_over_g", "value": 1.1666666666666667}
  ],
  "screening": null, "screening_recovery": null,
  "stability": {"has_nondegenerate_higher_derivatives": false, "kinetic_coefficient": 1.0, "q_s": 1.0, "sound_speed_sq": 1.0},
  "terms": [
    {"free_lorentz_indices": 0, "mass_dimension": 4, "name": "einstein_hilbert"},
    {"free_lorentz_indices": 0, "mass_dimension": 4, "name": "cosmological_constant"}
  ]
}"#;

    /// THE V7 acceptance regression (review-07): the V6 campaign's lone survivor — a bound-nDGP
    /// lineage with no brane term — must be rejected. "If V6.1 disqualifies drifted dials
    /// without terms, it should also reject a bound nDGP dial without a term."
    #[test]
    fn the_v6_survivor_is_rejected_by_the_term_registry() {
        let raw = V6_SURVIVOR_JSON;
        let theory: Theory = serde_json::from_str(raw).expect("vendored survivor parses");
        let kills = physics_kills(&theory);
        assert!(
            kills.iter().any(|k| matches!(
                k,
                VetoReason::StructurallyUngenerated { field, .. } if field == "ndgp"
            )),
            "expected the bound-nDGP-without-brane-term kill, got {kills:?}"
        );
    }
}
