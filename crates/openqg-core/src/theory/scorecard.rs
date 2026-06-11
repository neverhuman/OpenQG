//! V4 M3: the veto-first ScorecardV4 and the fixed 100-point critic-proofness rubric.
//!
//! This is the single trustworthy score. It fuses the V4 trust-spine pieces — content-bound
//! evidence (M0), the derivation-obligation oracle (M1), the ClaimGraph / UnificationClaim (M2) —
//! with the existing deterministic physics (`vetoes`, `model_league`, `held_out_evaluate`) into one
//! verdict that ranks the system's champion against human contenders and decoys on identical terms.
//!
//! **Veto-first invariant.** No positive credit is ever awarded before the hard gates pass. In
//! order: (1) every claim's evidence must materialize and content-hash-verify; (2) the physical
//! veto cascade must pass; (3) every physics claim must carry ≥1 obligation and every obligation
//! must verify; (4) a theory that *claims* unification must pass `no_hidden_knob_test`. Any failure
//! ⇒ `disqualified = true, total = 0.0` and we return before scoring. Only survivors earn the five
//! weighted rubric components (which sum to 100) plus uncertainty bands.
//!
//! `score()` is a pure function of (theory, claim graph, obligations, *materialized evidence bytes*,
//! and an optional pre-computed [`DataFitOutcome`]). The heavy forward-model / dataset work that
//! produces a `DataFitOutcome` (via `model_league` + `held_out_evaluate`) is done by the caller (the
//! genome in M5, the contender league in M4) and passed in, so this module stays deterministic and
//! unit-testable, and a verdict replays bit-for-bit from the same inputs ([`ScorecardReceipt`]).

use serde::{Deserialize, Serialize};

use super::{
    obligation_vetoes, physics_kills, Claim, ClaimGraph, DerivationObligation, EvidenceStore,
    MaterializedEvidenceAudit, Theory, UnificationClaim,
};

/// Pre-computed data-fit summary for a candidate, produced by the caller from `model_league`
/// (covariance-aware ΔAIC / Δln Z vs ΛCDM) and `held_out_evaluate` (sealed-holdout generalization
/// gap). `None` passed to [`score`] means "no data fit available" → the DataFit component scores 0
/// with a maximal uncertainty band (we never invent a fit).
/// V6: which likelihood scored the data — independent Gaussians or covariance-aware blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LikelihoodMode {
    #[default]
    Diagonal,
    Covariance,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DataFitOutcome {
    /// ΔAIC vs the ΛCDM reference (negative = candidate preferred).
    pub delta_aic: f64,
    /// Δln Z ≈ −0.5·ΔBIC vs ΛCDM (positive = candidate preferred).
    pub delta_lnz: f64,
    /// Sealed-holdout generalization gap (train_ll − heldout_ll per point; smaller = generalizes).
    pub generalization_gap: f64,
    /// League coverage in [0,1]; below the floor the fit is not trustworthy.
    pub coverage: f64,
    /// True if the best fit landed on a prior bound (the fit is suspect → wider band).
    pub boundary_hit: bool,
    /// V6: a headline Δln Z must state its likelihood mode.
    #[serde(default)]
    pub likelihood_mode: LikelihoodMode,
    /// V6: how many covariance blocks entered the likelihood (0 = pure diagonal).
    #[serde(default)]
    pub covariance_block_count: u32,
}

/// One weighted rubric dimension. `raw ∈ [0,1]`, `points = raw·weight`, `band` is the
/// (low, high) point uncertainty interval (always contains `points`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RubricComponent {
    pub name: String,
    pub weight: f64,
    pub raw: f64,
    pub points: f64,
    pub band: (f64, f64),
}

/// The fixed V4 rubric: six weighted components summing to exactly 100. Veto-survival is a *gate*,
/// not a component. `novel_prediction` scores distinctness-from-ΛCDM + a verified falsifiable
/// prediction (V4.1 — closes the "rediscover ΛCDM and score high" hole); `data_fit` now credits only
/// *beating* the baseline, not tying it.
pub struct RubricV4;

impl RubricV4 {
    pub const WEIGHTS: [(&'static str, f64); 6] = [
        ("derivation_rigor", 20.0),
        ("data_fit", 20.0),
        ("novel_prediction", 20.0),
        ("unification", 15.0),
        ("robustness_under_judge", 13.0),
        ("parsimony", 12.0),
    ];

    /// Total of the weights — asserted to be 100 by [`assert_weights_sum_to_100`].
    pub fn weight_sum() -> f64 {
        Self::WEIGHTS.iter().map(|(_, w)| w).sum()
    }
}

/// Compile-time-ish guarantee (checked in tests and at the top of `score`) that the rubric is a
/// true 100-point scale.
fn assert_weights_sum_to_100() {
    debug_assert!(
        (RubricV4::weight_sum() - 100.0).abs() < 1e-9,
        "rubric weights must sum to 100"
    );
}

/// The full verdict for one candidate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScorecardV4 {
    pub theory_id: String,
    /// True ⇒ a hard gate failed; `total == 0.0` and `components` is empty.
    pub disqualified: bool,
    /// Stringified kill reasons (vetoes / failed obligations / evidence / hidden-knob).
    pub kill_reasons: Vec<String>,
    /// Per-claim evidence materialization audits (the content-binding proof).
    pub evidence_audits: Vec<MaterializedEvidenceAudit>,
    pub claim_graph_digest: String,
    pub data_fit: Option<DataFitOutcome>,
    pub unification_claimed: bool,
    pub no_hidden_knob: bool,
    pub free_dof: u32,
    /// V4.1: whether the candidate makes a physical departure from ΛCDM/GR. `false` ⇒ a
    /// rediscovery, however well-certified. Reported even on disqualified candidates.
    pub distinct_from_baseline: bool,
    /// V5: what the truth-binder did — which background fields the certified claims set, with
    /// sources and fidelity caveats. Empty for unbound (GR) candidates.
    #[serde(default)]
    pub binding: super::binding::BindingReport,
    /// V5: each novel-prediction witness audited against the machine-computed truth (the model's
    /// prediction for the bound background vs the ΛCDM baseline) — declared vs computed on record.
    #[serde(default)]
    pub prediction_audits: Vec<super::binding::NovelPredictionAudit>,
    pub components: Vec<RubricComponent>,
    pub total: f64,
    pub total_band: (f64, f64),
}

/// A replay receipt: the canonical hash of the scorecard's inputs and of the scorecard itself, so a
/// referee can confirm the verdict is reproducible from the recorded inputs without re-running the
/// LLM. Mirrors the sealed-nondeterminism contract of `ProposalReceipt`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScorecardReceipt {
    pub theory_id: String,
    pub inputs_sha256: String,
    pub scorecard_sha256: String,
}

fn clamp01(x: f64) -> f64 {
    x.max(0.0).min(1.0)
}

/// Logistic squash centred so `x = 0 → 0.5`; `scale` in nat (log-evidence units).
fn logistic(x: f64, scale: f64) -> f64 {
    1.0 / (1.0 + (-x / scale).exp())
}

/// One-sided data-fit credit: **tying ΛCDM (Δln Z ≤ 0) earns 0**; only positive evidence over the
/// baseline climbs toward 1 (Δln Z = 1 ⇒ ~0.10, large ⇒ →1). This is the V4.1 fix for "ΛCDM
/// equivalence banks half credit" — a model that does not beat the baseline earns no data points.
fn data_fit_raw(delta_lnz: f64) -> f64 {
    clamp01(2.0 * logistic(delta_lnz, 5.0) - 1.0)
}

/// Whether a theory makes a *physical* departure from ΛCDM/GR — the distinctness signal that
/// separates a real candidate from a relabelled ΛCDM. True iff its linear gravity is modified
/// (`modifies_gravity`) OR it carries a *verified* certified-derived parameter on a known
/// modified-gravity relation whose value departs from that relation's GR-limit value. A theory that
/// only "derives" definitions (H0=100h, flat closure) and sits at every GR limit is **not** distinct.
fn distinct_from_lcdm(theory: &Theory) -> bool {
    if theory.modifies_gravity() {
        return true;
    }
    use super::Provenance::Derived;
    theory.parameters.iter().any(|p| {
        if let Derived {
            certificate: Some(c),
            ..
        } = &p.provenance
        {
            if let Some(gr) = super::certificate::relation_gr_value(&c.relation) {
                return c.verify() && (c.expected - gr).abs() > 1e-6;
            }
        }
        false
    })
}

/// Count genuine free degrees of freedom: a `Free` parameter, or a `Derived` parameter whose value
/// is not pinned by a certificate (an uncertified knob). `Fundamental` and certified-`Derived`
/// parameters cost nothing. This is the parsimony/complexity ledger.
/// V6.1: the total complexity ledger — free/uncertified parameters plus drifted background
/// coordinates. Public so the data-fit Occam term and the parsimony component charge the SAME k.
pub fn total_free_dof(theory: &Theory) -> u32 {
    free_dof(theory)
}

/// V7 (review-05): mechanism inputs to verified MG certificates that are proposer-CHOSEN
/// numbers (β, A_drag, μ0, f_R0 …) are post-search choices — fitted dials wearing a
/// certificate. Background-derived inputs (Ω_m, Ω_k, w0, Ω_de0 …) are not chosen; they are
/// read off the theory and reconciled by the binder.
fn post_search_choice_dof(theory: &Theory) -> u32 {
    use super::Provenance::Derived;
    const BACKGROUND_DERIVED: [&str; 8] = [
        "omega_m",
        "omega_r",
        "omega_k",
        "omega_de0",
        "w0",
        "wa",
        "h",
        "omega_b_h2",
    ];
    const MG_RELATIONS: [&str; 6] = [
        "planck_mu0_geff",
        "ndgp_geff_over_g",
        "ndgp_beta_from_omega_rc",
        "fr_alpha_m",
        "coupled_de_geff_over_g",
        "dark_scattering_growth_drag",
    ];
    let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for p in &theory.parameters {
        if let Derived {
            certificate: Some(cert),
            ..
        } = &p.provenance
        {
            if !MG_RELATIONS.contains(&cert.relation.as_str()) {
                continue;
            }
            if !matches!(cert.check(), super::CertificateOutcome::Verified { .. }) {
                continue;
            }
            for (name, _) in &cert.inputs {
                if !BACKGROUND_DERIVED.contains(&name.as_str()) {
                    seen.insert(name.clone());
                }
            }
        }
    }
    seen.len() as u32
}

fn free_dof(theory: &Theory) -> u32 {
    use super::Provenance::*;
    let param_dof = theory
        .parameters
        .iter()
        .filter(|p| {
            matches!(
                &p.provenance,
                Free | Derived {
                    certificate: None,
                    ..
                }
            )
        })
        .count() as u32;
    param_dof + background_dof(theory) + post_search_choice_dof(theory)
}

/// V6: every standard background coordinate moved off the Planck-ΛCDM reference is a fitted
/// degree of freedom and costs parsimony — UNLESS a verified relation certificate derives it
/// (then the certificate's own inputs carry the cost). The V5 champions moved h/Ω_m/w0 for free;
/// that drift is now an explicit, costed dial. MG background fields (mu0, fr_*, ndgp_omega_rc)
/// are excluded here: truth-binding already kills any uncertified non-GR field
/// (`UnexplainedModification`), and a certified one is derived, not free.
pub fn background_dof(theory: &Theory) -> u32 {
    let reference = crate::cosmology::CosmologyParams::planck_lcdm();
    let bg = &theory.background;
    // (drifted?, matching parameter symbol, scale from background units to parameter units)
    let coords: [(f64, f64, Option<(&str, f64)>); 9] = [
        (bg.h, reference.h, Some(("H0", 100.0))),
        (bg.omega_m, reference.omega_m, Some(("Omega_m", 1.0))),
        (bg.omega_b_h2, reference.omega_b_h2, None),
        (bg.n_eff, reference.n_eff, None),
        (bg.sum_mnu, reference.sum_mnu, None),
        (bg.w0, reference.w0, Some(("w0", 1.0))),
        (bg.wa, reference.wa, Some(("wa", 1.0))),
        (bg.omega_k, reference.omega_k, None),
        (bg.sigma8, reference.sigma8, Some(("sigma8", 1.0))),
    ];
    coords
        .iter()
        .filter(|(cur, refv, sym)| {
            let drifted = (cur - refv).abs() > 1e-9;
            if !drifted {
                return false;
            }
            // V6.1 (P0.4): the exemption demands a verified certificate on a relation that is
            // ALLOWED to derive this exact coordinate, whose expected value matches the
            // parameter within its own tolerance. Any-verified-cert exemption was a hole (a
            // verified-but-unrelated cert on a parameter merely NAMED "w0" exempted w0 drift),
            // and zero-rigor definitional relations (h0_from_h, flat closure) never exempt —
            // a definition is bookkeeping, not a derivation.
            let allowed: &[(&str, &str)] = &[]; // no registered relation may derive a standard
                                                // background coordinate today; the map exists so
                                                // a future mechanism relation can register here.
            let certified = sym.map_or(false, |(symbol, scale)| {
                theory.parameters.iter().any(|p| {
                    p.symbol == symbol
                        && (p.value - cur * scale).abs() <= 1e-6 * scale.max(1.0)
                        && matches!(
                            &p.provenance,
                            super::Provenance::Derived {
                                certificate: Some(cert),
                                ..
                            } if matches!(cert.check(), super::CertificateOutcome::Verified { .. })
                                && allowed
                                    .iter()
                                    .any(|(rel, sm)| *rel == cert.relation && *sm == symbol)
                                && super::certificate::relation_rigor_weight(&cert.relation) > 0.0
                                && (cert.expected - p.value).abs() <= cert.tolerance.max(1e-12)
                        )
                })
            });
            !certified
        })
        .count() as u32
}

fn component(name: &str, raw: f64, band_lo_raw: f64, band_hi_raw: f64) -> RubricComponent {
    let weight = RubricV4::WEIGHTS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, w)| *w)
        .expect("known rubric component");
    let raw = clamp01(raw);
    RubricComponent {
        name: name.to_string(),
        weight,
        raw,
        points: raw * weight,
        band: (clamp01(band_lo_raw) * weight, clamp01(band_hi_raw) * weight),
    }
}

/// Derivation-rigor raw score: mean over physics claims of the best *verified* obligation rigor
/// weight for that claim (0 if a claim has no verified obligation). Obligations are matched to a
/// claim by id appearing in the claim's `obligations` list.
fn derivation_rigor_raw(cg: &ClaimGraph, obligations: &[DerivationObligation]) -> f64 {
    let physics: Vec<&Claim> = cg.physics_claims();
    if physics.is_empty() {
        return 0.0;
    }
    let mut acc = 0.0;
    for c in &physics {
        let best = obligations
            .iter()
            .filter(|o| c.obligations.contains(&o.claim_id))
            .filter(|o| o.check().is_verified())
            .map(|o| o.effective_rigor_weight())
            .fold(0.0_f64, f64::max);
        acc += best;
    }
    acc / physics.len() as f64
}

/// Score a candidate veto-first against the fixed rubric. See module docs for the gate order.
pub fn score(
    theory: &Theory,
    cg: &ClaimGraph,
    obligations: &[DerivationObligation],
    unification: &UnificationClaim,
    store: &dyn EvidenceStore,
    evidence_schema: &str,
    data_fit: Option<DataFitOutcome>,
) -> ScorecardV4 {
    score_with_observables(
        theory,
        cg,
        obligations,
        unification,
        store,
        evidence_schema,
        data_fit,
        &[],
    )
}

/// V6.1: the context-aware scorecard — `fit_observables` powers the novelty fit-set cap, the
/// measurement-σ floor, and mechanism-attributable distinctness (P0.8). `score()` without
/// context behaves as before (no cap), for callers with no fit set in hand.
#[allow(clippy::too_many_arguments)]
pub fn score_with_observables(
    theory: &Theory,
    cg: &ClaimGraph,
    obligations: &[DerivationObligation],
    unification: &UnificationClaim,
    store: &dyn EvidenceStore,
    evidence_schema: &str,
    data_fit: Option<DataFitOutcome>,
    fit_observables: &[crate::ObservableRecord],
) -> ScorecardV4 {
    assert_weights_sum_to_100();
    let mut kill = Vec::new();

    // --- Gate 1: evidence materialization (content-bound; no laundering) ---
    let mut evidence_audits = Vec::new();
    for c in &cg.claims {
        for r in &c.evidence {
            let audit = store.materialize(r, evidence_schema);
            if !audit.ok {
                kill.push(format!(
                    "evidence[{}] for claim {} failed: {}",
                    audit.path, c.id, audit.detail
                ));
            }
            evidence_audits.push(audit);
        }
    }

    // --- Gate 2: physical sanity (the deterministic veto cascade) ---
    for v in physics_kills(theory) {
        kill.push(format!("veto: {v:?}"));
    }

    // --- Gate 3: derivation integrity (every physics claim obligated; every obligation verifies) ---
    if cg.validate_dag().is_err() {
        kill.push("claim graph is not a valid DAG".to_string());
    }
    for c in cg.unobligated_physics_claims() {
        kill.push(format!(
            "physics claim {} carries no derivation obligation",
            c.id
        ));
    }
    for v in obligation_vetoes(obligations) {
        kill.push(format!("obligation: {v:?}"));
    }

    // --- Gate 4: if it claims unification, it must not hide a per-sector knob ---
    let unification_claimed = !unification.shared.is_empty();
    let no_hidden_knob = unification.no_hidden_knob_test(theory);
    if unification_claimed && !no_hidden_knob {
        kill.push("claims unification but fails no_hidden_knob_test".to_string());
    }

    let free = free_dof(theory);
    let digest = cg.digest();
    // V5 truth-binding: translate certified MG claims into the background the model computes.
    // (Binding vetoes already flowed into `kill` via Gate 2's cascade — this re-bind is the cheap,
    // pure call that yields the report + bound background for the audits below.)
    let binding_outcome = super::binding::bind_modified_background(theory);
    // V4.1/V5 distinctness — declared certs OR a bound non-GR background; computed before the gate
    // so it is reported even on disqualified candidates.
    let physically_distinct = distinct_from_lcdm(theory) || binding_outcome.report.bound_non_gr;

    // V6.1 (P0.7): an obligation certificate on a binding-relevant relation must describe the
    // SAME modification the binder applied — rigor earned for mu0=-0.05 while the theory binds
    // and fits mu0=-0.1 is claim/physics incoherence, killed like any field conflict.
    {
        const MG_RELATIONS: [&str; 6] = [
            "planck_mu0_geff",
            "ndgp_geff_over_g",
            "ndgp_beta_from_omega_rc",
            "fr_alpha_m",
            "coupled_de_geff_over_g",
            "dark_scattering_growth_drag",
        ];
        for o in obligations {
            let Some(ocert) = &o.certificate else {
                continue;
            };
            if !MG_RELATIONS.contains(&ocert.relation.as_str()) {
                continue;
            }
            for p in &theory.parameters {
                if let super::Provenance::Derived {
                    certificate: Some(pcert),
                    ..
                } = &p.provenance
                {
                    if pcert.relation != ocert.relation {
                        continue;
                    }
                    for (name, oval) in &ocert.inputs {
                        if let Some((_, pval)) = pcert.inputs.iter().find(|(n, _)| n == name) {
                            if (oval - pval).abs() > 1e-6 + 1e-3 * pval.abs() {
                                kill.push(format!(
                                    "claim/physics incoherence: obligation {} certifies {}={} \
                                     but the bound parameter certificate uses {}={}",
                                    o.claim_id, name, oval, name, pval
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    // V6: audit the novel-prediction witnesses BEFORE the gate — fabrication (>3× the
    // engine-clamped tolerance) is itself a kill, and audits are reported even on DQ.
    let prediction_audits = super::binding::audit_novel_predictions_with_context(
        &binding_outcome.theory.background,
        obligations,
        fit_observables,
    );
    for a in &prediction_audits {
        if a.verdict == super::binding::NoveltyAuditVerdict::Fabricated {
            kill.push(format!(
                "fabricated novel prediction: claim {} declared {}={} but the model computes {} \
                 (>3x the engine tolerance {})",
                a.claim_id,
                a.observable_raw,
                a.declared_predicted,
                a.computed_predicted.unwrap_or(f64::NAN),
                a.tolerance,
            ));
        }
    }

    if !kill.is_empty() {
        return ScorecardV4 {
            theory_id: theory.id.clone(),
            disqualified: true,
            kill_reasons: kill,
            evidence_audits,
            claim_graph_digest: digest,
            data_fit,
            unification_claimed,
            no_hidden_knob,
            free_dof: free,
            distinct_from_baseline: physically_distinct,
            binding: binding_outcome.report,
            prediction_audits,
            components: Vec::new(),
            total: 0.0,
            total_band: (0.0, 0.0),
        };
    }

    // --- Survivor: earn the five weighted components ---
    // 1. Derivation rigor.
    let dr = derivation_rigor_raw(cg, obligations);
    let c_dr = component("derivation_rigor", dr, dr, dr);

    // 2. Data fit — one-sided: tying ΛCDM (Δln Z ≤ 0) → 0; only beating it earns credit. None ⇒ 0.
    let c_df = match data_fit {
        Some(f) => {
            let raw = data_fit_raw(f.delta_lnz);
            // boundary-hit / sub-coverage widen the band downward (the fit is suspect).
            let suspect = f.boundary_hit || f.coverage < 1.0 - 1e-9;
            let lo = if suspect { raw * 0.6 } else { raw };
            component("data_fit", raw, lo, raw)
        }
        None => component("data_fit", 0.0, 0.0, 1.0),
    };

    // 3. Novel prediction — V5: distinctness and honesty are MACHINE-COMPUTED, never declared.
    //    Each NovelPrediction witness is audited against the forward model's prediction for the
    //    BOUND background vs the ΛCDM baseline: full credit only when the computed deviation is
    //    detectable AND the declared numbers agree with the computed truth (within the witness's
    //    own claimed resolution). A rediscovery earns 0; an unverifiable falsifier (an observable
    //    the model cannot predict) earns the old half-credit tier; a computable-but-wrong or
    //    dishonest declaration is demoted to 0 and flagged in the audits.
    // V6 novelty: verdict-driven, no free half-credit. No witness ⇒ 0 (the V5 α-jitter faucet);
    // uncomputable-only witnesses ⇒ 0 (the model can't check it ⇒ it earns nothing); full credit
    // ONLY for a computed, honest, distinct prediction.
    use super::binding::NoveltyAuditVerdict as NV;
    // V6.1 (P0.8a): a witness on FITTED data is a fit explanation, capped at 0.25 — full
    // novelty requires an honest, mechanism-distinct prediction OUTSIDE the fit set.
    let nov_raw =
        if !physically_distinct {
            0.0
        } else if prediction_audits.iter().any(|a| {
            a.verdict == NV::ComputedHonestDistinct && !a.in_fit_set && !a.refreshed_by_engine
        }) {
            1.0
        } else if prediction_audits.iter().any(|a| {
            a.verdict == NV::ComputedHonestDistinct && !a.in_fit_set && a.refreshed_by_engine
        }) {
            0.5 // engine-attested (P0.9): falsifiable + mechanism-distinct, but not proposer-authored
        } else if prediction_audits
            .iter()
            .any(|a| a.verdict == NV::ComputedHonestDistinct && a.in_fit_set)
        {
            0.25 // fit explanation (P0.8a)
        } else {
            0.0
        };
    let c_nov = component("novel_prediction", nov_raw, nov_raw, nov_raw);

    // 3. Unification — credit for a genuine, hidden-knob-free shared-parameter claim + self-consistency.
    let uni_raw = {
        let shared = unification.shared_parameter_audit(theory);
        let mut r = 0.0;
        if shared {
            r += 0.6;
        }
        if unification_claimed && no_hidden_knob {
            r += 0.4;
        }
        r
    };
    let c_uni = component("unification", uni_raw, uni_raw, uni_raw);

    // 4. Robustness under judge — generalization gap (smaller = better); adversarial panel folds in
    //    here in M4+. None ⇒ 0.5 with full band (unknown).
    let c_rob = match data_fit {
        Some(f) => {
            let raw = clamp01(1.0 / (1.0 + (f.generalization_gap.max(0.0) / 0.05)));
            component("robustness_under_judge", raw, raw, raw)
        }
        None => component("robustness_under_judge", 0.5, 0.0, 1.0),
    };

    // 5. Parsimony — fewer free dof is better. 0 free dof ⇒ 1.0.
    // V6.1 (P0.10): linear, non-saturating dof penalty — the harmonic 1/(1+k) made each extra
    // dial nearly free past the first (unlimited drift was rational).
    let pars_raw = (1.0 - free as f64 / 4.0).max(0.0);
    let c_par = component("parsimony", pars_raw, pars_raw, pars_raw);

    let components = vec![c_dr, c_df, c_nov, c_uni, c_rob, c_par];
    let total: f64 = components.iter().map(|c| c.points).sum();
    let band_lo: f64 = components.iter().map(|c| c.band.0).sum();
    let band_hi: f64 = components.iter().map(|c| c.band.1).sum();

    ScorecardV4 {
        theory_id: theory.id.clone(),
        disqualified: false,
        kill_reasons: Vec::new(),
        evidence_audits,
        claim_graph_digest: digest,
        data_fit,
        unification_claimed,
        no_hidden_knob,
        free_dof: free,
        distinct_from_baseline: physically_distinct,
        binding: binding_outcome.report,
        prediction_audits,
        components,
        total,
        total_band: (band_lo, band_hi),
    }
}

/// Build the replay receipt for a scorecard given a canonical rendering of its inputs.
pub fn scorecard_receipt(scorecard: &ScorecardV4, inputs_canonical: &str) -> ScorecardReceipt {
    let scorecard_json = serde_json::to_string(scorecard).unwrap_or_default();
    ScorecardReceipt {
        theory_id: scorecard.theory_id.clone(),
        inputs_sha256: crate::sha256_digest(inputs_canonical.as_bytes()),
        scorecard_sha256: crate::sha256_digest(scorecard_json.as_bytes()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::claim_graph::{ClaimKind, Sector};
    use crate::theory::{
        audit_bytes, DerivationObligationKind, EvidenceRef, EvidenceTier, LimitWitness, Parameter,
        Provenance, SharedParam,
    };
    use std::collections::BTreeMap;

    struct MemStore(BTreeMap<String, Vec<u8>>);
    impl EvidenceStore for MemStore {
        fn materialize(&self, r: &EvidenceRef, schema_id: &str) -> MaterializedEvidenceAudit {
            match self.0.get(&r.path) {
                Some(b) => audit_bytes(r, b, schema_id),
                None => MaterializedEvidenceAudit {
                    path: r.path.clone(),
                    sha256: String::new(),
                    byte_len: 0,
                    jsonl_count: 0,
                    schema_id: schema_id.to_string(),
                    tier: r.tier,
                    ok: false,
                    detail: "missing".into(),
                },
            }
        }
    }

    fn bound_ref(path: &str, bytes: &[u8]) -> EvidenceRef {
        EvidenceRef::new(path, crate::sha256_digest(bytes), EvidenceTier::T2)
    }

    // A minimal valid candidate: ΛCDM baseline + one obligated, evidence-bound physics claim.
    fn fixture() -> (
        Theory,
        ClaimGraph,
        Vec<DerivationObligation>,
        UnificationClaim,
        MemStore,
    ) {
        let theory = Theory::baseline_lcdm();
        let bytes = b"{\"e2\":1.0}\n".to_vec();
        let claim = Claim {
            id: "c-bg".into(),
            sector: Sector::Background,
            kind: ClaimKind::Physics,
            statement: "background recovers GR".into(),
            evidence: vec![bound_ref("bg.json", &bytes)],
            obligations: vec!["ob-limit".into()],
            depends_on: vec![],
        };
        let cg = ClaimGraph {
            claims: vec![claim],
        };
        let ob = DerivationObligation {
            claim_id: "ob-limit".into(),
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
        let store = MemStore(BTreeMap::from([("bg.json".to_string(), bytes)]));
        (
            theory,
            cg,
            vec![ob],
            UnificationClaim { shared: vec![] },
            store,
        )
    }

    fn good_fit() -> DataFitOutcome {
        DataFitOutcome {
            delta_aic: -2.0,
            delta_lnz: 1.0,
            generalization_gap: 0.01,
            coverage: 1.0,
            boundary_hit: false,
            likelihood_mode: LikelihoodMode::Diagonal,
            covariance_block_count: 0,
        }
    }

    #[test]
    fn weights_sum_to_100() {
        assert!((RubricV4::weight_sum() - 100.0).abs() < 1e-9);
    }

    #[test]
    fn a_clean_survivor_scores_and_bands_contain_the_total() {
        let (t, cg, obs, uni, store) = fixture();
        let sc = score(&t, &cg, &obs, &uni, &store, "schema.v1", Some(good_fit()));
        assert!(!sc.disqualified, "{:?}", sc.kill_reasons);
        assert!(sc.total > 0.0 && sc.total <= 100.0);
        assert!(sc.total_band.0 <= sc.total && sc.total <= sc.total_band.1 + 1e-9);
        assert_eq!(sc.components.len(), 6);
    }

    #[test]
    fn a_free_parameter_is_disqualified_with_zero_total() {
        let (mut t, cg, obs, uni, store) = fixture();
        t.parameters.push(Parameter {
            symbol: "xi".into(),
            value: 0.3,
            physical_meaning: "gray-box knob".into(),
            provenance: Provenance::Free,
        });
        let sc = score(&t, &cg, &obs, &uni, &store, "schema.v1", Some(good_fit()));
        assert!(sc.disqualified);
        assert_eq!(sc.total, 0.0);
        assert!(sc.components.is_empty());
        assert!(sc.kill_reasons.iter().any(|r| r.contains("veto")));
    }

    #[test]
    fn unmaterializable_evidence_disqualifies() {
        let (t, mut cg, obs, uni, _store) = fixture();
        // Point a claim at evidence the store does not have.
        cg.claims[0].evidence = vec![EvidenceRef::new(
            "absent.json",
            "0".repeat(64),
            EvidenceTier::T2,
        )];
        let empty = MemStore(BTreeMap::new());
        let sc = score(&t, &cg, &obs, &uni, &empty, "schema.v1", Some(good_fit()));
        assert!(sc.disqualified);
        assert_eq!(sc.total, 0.0);
        assert!(sc.kill_reasons.iter().any(|r| r.contains("evidence")));
    }

    #[test]
    fn a_physics_claim_without_an_obligation_is_disqualified() {
        let (t, mut cg, _obs, uni, store) = fixture();
        cg.claims[0].obligations.clear(); // physics claim with no obligation
        let sc = score(&t, &cg, &[], &uni, &store, "schema.v1", Some(good_fit()));
        assert!(sc.disqualified);
        assert!(sc
            .kill_reasons
            .iter()
            .any(|r| r.contains("no derivation obligation")));
    }

    #[test]
    fn claimed_unification_with_a_hidden_knob_is_disqualified() {
        let (mut t, cg, obs, _uni, store) = fixture();
        // Add an uncertified derived knob NOT declared in the shared set.
        t.parameters.push(Parameter {
            symbol: "g_x".into(),
            value: 0.5,
            physical_meaning: "sector-private knob".into(),
            provenance: Provenance::derived("hand-wave"),
        });
        // Claim unification over a different (matching) shared param, leaving g_x hidden.
        let uni = UnificationClaim {
            shared: vec![SharedParam {
                symbol: "H0".into(),
                value: 67.4,
                sectors: vec![Sector::Background, Sector::Growth],
            }],
        };
        let sc = score(&t, &cg, &obs, &uni, &store, "schema.v1", Some(good_fit()));
        assert!(sc.disqualified);
        assert!(sc.kill_reasons.iter().any(|r| r.contains("no_hidden_knob")));
    }

    #[test]
    fn none_datafit_scores_zero_datafit_with_full_band() {
        let (t, cg, obs, uni, store) = fixture();
        let sc = score(&t, &cg, &obs, &uni, &store, "schema.v1", None);
        assert!(!sc.disqualified, "{:?}", sc.kill_reasons);
        let df = sc.components.iter().find(|c| c.name == "data_fit").unwrap();
        assert_eq!(df.points, 0.0);
        assert_eq!(df.band, (0.0, 20.0));
    }

    #[test]
    fn tie_data_fit_scores_zero() {
        let (t, cg, obs, uni, store) = fixture();
        let tie = DataFitOutcome {
            delta_aic: 0.0,
            delta_lnz: 0.0,
            generalization_gap: 0.01,
            coverage: 1.0,
            boundary_hit: false,
            likelihood_mode: LikelihoodMode::Diagonal,
            covariance_block_count: 0,
        };
        let sc = score(&t, &cg, &obs, &uni, &store, "schema.v1", Some(tie));
        assert!(!sc.disqualified);
        let df = sc.components.iter().find(|c| c.name == "data_fit").unwrap();
        assert_eq!(df.points, 0.0, "tying ΛCDM (Δln Z=0) earns no data credit");
    }

    #[test]
    fn lcdm_rediscovery_is_not_distinct_and_scores_no_novelty() {
        // The baseline fixture is pure ΛCDM with only a GR-limit obligation: not distinct.
        let (t, cg, obs, uni, store) = fixture();
        let sc = score(&t, &cg, &obs, &uni, &store, "schema.v1", Some(good_fit()));
        assert!(!sc.disqualified);
        assert!(
            !sc.distinct_from_baseline,
            "a ΛCDM rediscovery must be flagged not-distinct"
        );
        let nov = sc
            .components
            .iter()
            .find(|c| c.name == "novel_prediction")
            .unwrap();
        assert_eq!(nov.points, 0.0);
    }

    #[test]
    fn a_real_modification_is_distinct_and_outscores_a_rediscovery() {
        use crate::theory::{DerivedCertificate, NovelPredictionWitness};
        let (mut t, mut cg, mut obs, uni, store) = fixture();
        // A genuine modified-gravity parameter: G_eff/G = 7/6 via nDGP (departs from GR value 1).
        let cert = DerivedCertificate {
            relation: "ndgp_geff_over_g".into(),
            inputs: vec![("beta".into(), 2.0)],
            expected: 1.0 + 1.0 / 6.0,
            tolerance: 1e-9,
        };
        t.terms.push(crate::theory::Term {
            name: "dgp_brane".into(),
            mass_dimension: 4,
            free_lorentz_indices: 0,
        });
        t.parameters.push(Parameter {
            symbol: "geff_over_g".into(),
            value: 1.0 + 1.0 / 6.0,
            physical_meaning: "nDGP linear coupling".into(),
            provenance: Provenance::derived_certified("nDGP braneworld", cert),
        });
        // A verified falsifiable prediction on fσ8 — V5: the witness numbers are computed from the
        // bound background through the actual model (declared values must match the truth).
        let (predicted, baseline) = {
            use crate::cosmology::{BackgroundForwardModel, CosmologyParams, ForwardModel};
            let bound = crate::theory::bind_modified_background(&t).theory;
            let model = BackgroundForwardModel;
            let ids = vec!["fsigma8@0.51".to_string()];
            let p = model.predict(&bound.background, &ids).unwrap()[0].value;
            let b = model
                .predict(&CosmologyParams::planck_lcdm(), &ids)
                .unwrap()[0]
                .value;
            (p, b)
        };
        assert!(
            predicted > baseline + 0.01,
            "normal-branch nDGP must ENHANCE growth detectably: {predicted} vs {baseline}"
        );
        cg.claims[0].obligations.push("ob-novel".into());
        obs.push(DerivationObligation {
            claim_id: "ob-novel".into(),
            kind: DerivationObligationKind::NovelPrediction,
            detail: "fσ8 enhancement (normal-branch nDGP)".into(),
            certificate: None,
            limit: None,
            citation: None,
            novel: Some(NovelPredictionWitness {
                refreshed_by_engine: false,
                observable: "fsigma8_z051".into(),
                predicted,
                baseline,
                min_detectable: 0.01,
                falsifier: "DESI/Euclid fσ8".into(),
            }),
        });
        let modi = score(&t, &cg, &obs, &uni, &store, "schema.v1", Some(good_fit()));
        assert!(!modi.disqualified, "{:?}", modi.kill_reasons);
        assert!(modi.distinct_from_baseline);
        // The truth-audit must be on record: computed values present and honest.
        assert_eq!(modi.prediction_audits.len(), 1);
        assert_eq!(modi.prediction_audits[0].honest, Some(true));
        assert_eq!(modi.prediction_audits[0].computed_distinct, Some(true));
        let nov = modi
            .components
            .iter()
            .find(|c| c.name == "novel_prediction")
            .unwrap();
        assert_eq!(
            nov.points, 20.0,
            "distinct + computed-honest falsifier ⇒ full novelty"
        );

        // It must outscore a pure-ΛCDM rediscovery scored the same way.
        let (t0, cg0, obs0, uni0, store0) = fixture();
        let redisc = score(
            &t0,
            &cg0,
            &obs0,
            &uni0,
            &store0,
            "schema.v1",
            Some(good_fit()),
        );
        assert!(
            modi.total > redisc.total,
            "real modification {} must beat rediscovery {}",
            modi.total,
            redisc.total
        );
    }

    #[test]
    fn receipt_is_deterministic() {
        let (t, cg, obs, uni, store) = fixture();
        let sc = score(&t, &cg, &obs, &uni, &store, "schema.v1", Some(good_fit()));
        let r1 = scorecard_receipt(&sc, "canonical-inputs");
        let r2 = scorecard_receipt(&sc, "canonical-inputs");
        assert_eq!(r1, r2);
        assert_eq!(r1.inputs_sha256.len(), 64);
        assert_eq!(r1.scorecard_sha256.len(), 64);
    }
}
