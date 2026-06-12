//! V4 M3: the veto-first ScorecardV4 and the fixed 100-point critic-proofness rubric.
//!
//! This is the single trustworthy score. It fuses the V4 trust-spine pieces — content-bound
//! evidence (M0), the derivation-obligation oracle (M1), the ClaimGraph / UnificationClaim (M2) —
//! with the existing deterministic physics (`vetoes`, `model_league`, `split_evaluate`) into one
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
//! produces a `DataFitOutcome` (via `model_league` + `split_evaluate`) is done by the caller (the
//! genome in M5, the contender league in M4) and passed in, so this module stays deterministic and
//! unit-testable, and a verdict replays bit-for-bit from the same inputs ([`ScorecardReceipt`]).

use serde::{Deserialize, Serialize};

use super::{
    obligation_vetoes, physics_kills, profundity, Claim, ClaimGraph, DerivationObligation,
    EvidenceStore, MaterializedEvidenceAudit, Theory, UnificationClaim,
};

/// Evidential category a scorecard achieves, given data scale, instrument tier, and fit quality.
///
/// Derived automatically; cannot be upgraded by the proposer.
/// Only data scale, solver quality, and trial-correction context determine the maximum class.
///
/// At n=23 (current compressed-likelihood scale), the default cap is `InterestingFit`.
/// `PromotionCandidate` requires covariance-grade evidence + a promotion-grade solver.
/// `DiscoveryClaim` additionally requires a sealed-holdout forecast from the prediction registry.
/// `ExclusionClaim` is set via `ExclusionSentence`, not derived from a single scorecard.
///
/// Reference: S07 §8 "State the n=23 claim boundary in paper and receipts".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ClaimClass {
    /// No actionable claim: data scale or instrument tier too low, or candidate is disqualified.
    #[default]
    Triage,
    /// Compressed likelihoods, moderate evidence — notable but not promotion-grade.
    /// n=23 with diagonal/BIC mode is capped here regardless of delta_lnz.
    InterestingFit,
    /// Covariance-grade evidence, ΔlnZ > 2, T1Emulator or higher, not boundary-pinned.
    PromotionCandidate,
    /// Promotion-grade nested sampling, ΔlnZ > 5 post-trials, rank-stable, sealed-holdout forecast.
    /// Requires T2Boltzmann or higher; controlled by the prediction registry, not the scorecard.
    DiscoveryClaim,
    /// Full exclusion: set via `ExclusionSentence`, not derived from a scorecard.
    ExclusionClaim,
}

impl ClaimClass {
    /// Derive the claim class from scorecard context.
    ///
    /// - `n_observations`: number of data points in the scored likelihood (0 → `Triage`).
    /// - `disqualified`: hard kill → `Triage` regardless of other factors.
    /// - `boundary_hit`: best fit sits on a prior boundary → caps at `InterestingFit`.
    pub fn derive(
        n_observations: u32,
        likelihood_mode: LikelihoodMode,
        delta_lnz: f64,
        instrument_tier: Option<crate::cosmology::ForwardTier>,
        disqualified: bool,
        boundary_hit: bool,
    ) -> Self {
        use crate::cosmology::ForwardTier;
        if disqualified {
            return ClaimClass::Triage;
        }
        // n=0 or n≤15 with diagonal mode → Triage (insufficient data scale).
        if n_observations == 0
            || (n_observations <= 15 && likelihood_mode == LikelihoodMode::Diagonal)
        {
            return ClaimClass::Triage;
        }
        // n≤30 with diagonal mode → InterestingFit (the n=23 wall from S07 §8).
        if n_observations <= 30 && likelihood_mode == LikelihoodMode::Diagonal {
            return ClaimClass::InterestingFit;
        }
        // Boundary-pinned improvement blocks promotion.
        if boundary_hit {
            return ClaimClass::InterestingFit;
        }
        let tier = instrument_tier.unwrap_or(ForwardTier::T0Formula);
        if tier < ForwardTier::T1Emulator {
            return ClaimClass::InterestingFit;
        }
        // Covariance mode + T1+ + ΔlnZ > 2 → PromotionCandidate.
        // DiscoveryClaim requires a sealed-holdout forecast from the prediction registry —
        // that gate is enforced externally, not here.
        if delta_lnz > 2.0 {
            return ClaimClass::PromotionCandidate;
        }
        ClaimClass::InterestingFit
    }

    pub fn label(self) -> &'static str {
        match self {
            ClaimClass::Triage => "triage",
            ClaimClass::InterestingFit => "interesting_fit",
            ClaimClass::PromotionCandidate => "promotion_candidate",
            ClaimClass::DiscoveryClaim => "discovery_claim",
            ClaimClass::ExclusionClaim => "exclusion_claim",
        }
    }

    /// True when this class supports a publishable claim (PromotionCandidate or above).
    pub fn is_publishable(self) -> bool {
        self >= ClaimClass::PromotionCandidate
    }

    /// Attempt to upgrade a `PromotionCandidate` to a `DiscoveryClaim` if all gates pass.
    ///
    /// This is the only code path that can return `DiscoveryClaim`; `derive()` never does.
    /// A class below `PromotionCandidate` is returned unchanged regardless of gate state.
    pub fn try_upgrade_to_discovery(self, gate: &super::profundity::DiscoveryClaimGate) -> Self {
        if self >= ClaimClass::PromotionCandidate && gate.passes_all_gates() {
            ClaimClass::DiscoveryClaim
        } else {
            self
        }
    }
}

/// Absolute goodness-of-fit assessment for a theory on the scored dataset.
///
/// Unlike [`DataFitOutcome`] (deltas vs ΛCDM), this carries the absolute χ²/dof
/// and global p-value that determine whether the model describes the data at all.
/// A positive Bayes factor vs ΛCDM means nothing if both models are bad fits.
///
/// Reference: S07 §"The fourth failure is GoF".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GoFOutcome {
    /// χ² / effective degrees of freedom. Values > 2.0 indicate a poor absolute fit.
    pub chi_sq_dof: f64,
    /// Effective data modes (from covariance decomposition when available).
    pub n_eff_modes: u32,
    /// Global p-value from χ² distribution (`None` if covariance unavailable for CDF computation).
    pub global_p_value: Option<f64>,
    /// True when the absolute fit is acceptable: chi_sq_dof < 2.0 AND
    /// (global_p_value is None or global_p_value ≥ 0.01).
    pub gof_passed: bool,
    /// True when the best fit sits on a prior boundary AND the chi_sq_dof is suspect.
    /// Blocks promotion to PromotionCandidate even when ΔlnZ > 2.
    pub boundary_pinned_unresolved: bool,
}

impl GoFOutcome {
    /// Construct from a chi-squared statistic without a full CDF (conservative path).
    pub fn from_chi_sq(chi_sq: f64, n_eff_modes: u32, boundary_hit: bool) -> Self {
        let dof = n_eff_modes.max(1) as f64;
        let chi_sq_dof = chi_sq / dof;
        let gof_passed = chi_sq_dof < 2.0;
        GoFOutcome {
            chi_sq_dof,
            n_eff_modes,
            global_p_value: None,
            gof_passed,
            boundary_pinned_unresolved: boundary_hit && !gof_passed,
        }
    }
}

/// Pre-computed data-fit summary for a candidate, produced by the caller from `model_league`
/// (covariance-aware ΔAIC / Δln Z vs ΛCDM) and `split_evaluate` (train/test generalization gap).
/// `None` passed to [`score`] means "no data fit available" → the DataFit component scores 0
/// with a maximal uncertainty band (we never invent a fit).
/// V6: which likelihood scored the data — independent Gaussians or covariance-aware blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LikelihoodMode {
    #[default]
    Diagonal,
    Covariance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataFitOutcome {
    /// ΔAIC vs the ΛCDM reference (negative = candidate preferred).
    pub delta_aic: f64,
    /// Δln Z ≈ −0.5·ΔBIC vs ΛCDM (positive = candidate preferred).
    pub delta_lnz: f64,
    /// Train/test split generalization gap (train_ll − test_ll per point; smaller = generalizes).
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
    /// V8 Phase 5: number of data points that entered the scored likelihood.
    /// Used to derive [`ClaimClass`] (n=23 wall). Zero means unknown (Triage).
    #[serde(default)]
    pub n_observations: u32,
    /// V8 Phase 5: absolute goodness-of-fit assessment.
    /// `None` when GoF has not been computed (legacy path).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gof_outcome: Option<GoFOutcome>,
}

impl DataFitOutcome {
    /// V8 (Wave 0.4) evidence gate: the theory must not be substantially worse than ΛCDM.
    ///
    /// Gate fails (returns `false`) when BOTH indicators are bad:
    /// - `delta_lnz < -2.0` (candidate ≥2 ln-evidence units worse than ΛCDM), AND
    /// - `delta_aic > 4.0` (ΛCDM preferred by ≥4 AIC units).
    ///
    /// A theory that passes on EITHER axis is not condemned as substantially inferior.
    /// Diagonal-mode (BIC-only) fits can still fail this gate; covariance-mode lnZ is more
    /// discriminating. See SYNTHESIS §5 Phase 0 item #3 and S07 §4.2.
    pub fn fit_gate_passed(&self) -> bool {
        self.delta_lnz > -2.0 || self.delta_aic <= 4.0
    }

    /// V8 Phase 5: absolute GoF gate. Returns `None` when `gof_outcome` is unavailable
    /// (no kill raised). Returns `Some(false)` when GoF explicitly failed.
    pub fn gof_gate_passed(&self) -> Option<bool> {
        self.gof_outcome.as_ref().map(|g| g.gof_passed)
    }

    /// Derive the `ClaimClass` from this fit context.
    pub fn claim_class(
        &self,
        instrument_tier: Option<crate::cosmology::ForwardTier>,
        disqualified: bool,
    ) -> ClaimClass {
        ClaimClass::derive(
            self.n_observations,
            self.likelihood_mode,
            self.delta_lnz,
            instrument_tier,
            disqualified,
            self.boundary_hit,
        )
    }
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
    /// V8 Phase 1 (#6): the fidelity tier of the forward model that produced `data_fit`.
    /// `None` means the scorecard was produced without a manifested forward model (legacy path).
    /// T0/T1 instrument → promotion-grade claims are flagged by the InstrumentRisk gate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instrument_tier: Option<crate::cosmology::ForwardTier>,
    /// V8 Phase 1 (#5): the trials gate applied to this theory's evidence claim, if a search
    /// ledger was active. `None` means no ledger was attached (single-hypothesis evaluation).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trials_correction: Option<super::search_ledger::TrialsGate>,
    /// V8 Phase 1 (#3): points earned from the pre-registered prediction registry (0–30).
    /// Added to `total` on top of the 100-point rubric. Zero until the prediction registry is
    /// wired (Phase 1 #8 / #9 growth verdict pack).
    #[serde(default)]
    pub forecast_points: f64,
    /// V8 Phase 5: evidential category derived from data scale + instrument tier + fit quality.
    /// `Triage` for disqualified candidates or unknown n_observations.
    #[serde(default)]
    pub claim_class: ClaimClass,
    /// V8 Phase 7: result of the discovery-class gate aggregation. `Some` when the engine ran all
    /// five sub-gates (profundity, post-search p-value, GoF, instrument tier, sealed forecast).
    /// `None` means the candidate was not evaluated against the full discovery gate (e.g., it was
    /// disqualified early or the null distribution has not been computed yet).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discovery_gate_result: Option<profundity::DiscoveryClaimGateResult>,
    /// V8 Phase 11 (SYNTHESIS #3): `PricingLedger` for this scorecard's DOF choices.
    /// `None` means no pricing ledger was attached (legacy path; no pricing gate applied).
    /// When present and `fails_closed()`, the candidate is disqualified.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pricing_ledger: Option<super::pricing::PricingLedger>,
    /// V8 Phase 11 (SYNTHESIS #8): coverage gate for suppressed-growth claims.
    /// `None` means the gate has not been evaluated (single-observable scoring path).
    /// When present and `!coverage_satisfied()`, the candidate is disqualified.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub growth_coverage_gate: Option<super::growth_verdict::GrowthCoverageGate>,
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

    // --- Gate 3.5: claim-graph lint (V8 Wave 0.8) — catches NO_EVIDENCE_HASH,
    //     BEATS_LCDM_WITHOUT_TRIALS_CORRECTION, FIT_SET_NOVELTY_CREDIT ---
    {
        let report = crate::validation::lint_claims(cg);
        for f in &report.findings {
            kill.push(format!(
                "claim_lint[{}] on {}: {}",
                f.rule, f.claim_id, f.message
            ));
        }
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

    // V8 (Wave 0.4): evidence gate — kill a theory that is substantially worse than ΛCDM on
    // BOTH indicators. Covariance-mode lnZ is the primary signal; diagonal BIC is a gate only.
    if let Some(ref f) = data_fit {
        if !f.fit_gate_passed() {
            kill.push(format!(
                "data_fit_gate_failed: theory is substantially worse than ΛCDM \
                 (delta_lnz={:.2}, delta_aic={:.2}); covariance-aware evidence required to \
                 overcome this gate",
                f.delta_lnz, f.delta_aic
            ));
        }
        // V8 Phase 5: absolute GoF gate — a positive Bayes factor over a bad model is not evidence.
        if let Some(false) = f.gof_gate_passed() {
            kill.push(format!(
                "gof_gate_failed: absolute fit is unacceptable \
                 (chi_sq_dof={:.2}); must pass chi_sq_dof < 2.0 before reporting beats-ΛCDM",
                f.gof_outcome.as_ref().map_or(0.0, |g| g.chi_sq_dof)
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
            data_fit: data_fit.clone(),
            unification_claimed,
            no_hidden_knob,
            free_dof: free,
            distinct_from_baseline: physically_distinct,
            binding: binding_outcome.report,
            prediction_audits,
            components: Vec::new(),
            total: 0.0,
            total_band: (0.0, 0.0),
            instrument_tier: None,
            trials_correction: None,
            forecast_points: 0.0,
            claim_class: ClaimClass::Triage,
            discovery_gate_result: None,
            pricing_ledger: None,
            growth_coverage_gate: None,
        };
    }

    // --- Survivor: earn the five weighted components ---
    // 1. Derivation rigor.
    let dr = derivation_rigor_raw(cg, obligations);
    let c_dr = component("derivation_rigor", dr, dr, dr);

    // 2. Data fit — V8 (Wave 0.4) BIC demotion: diagonal (BIC/AIC-only) fits award 0 points;
    //    only covariance-aware Δln Z earns credit. Fit-set data → evidence gate, not score.
    //    Forecast points (Phase 1 item #3, prediction registry) will be added separately.
    let c_df = match &data_fit {
        Some(f) => {
            let raw = if f.likelihood_mode == LikelihoodMode::Covariance {
                data_fit_raw(f.delta_lnz)
            } else {
                // Diagonal mode (BIC/AIC proxy): demoted to 0. Gate passed above; no points awarded.
                0.0
            };
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
    let c_rob = match &data_fit {
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

    // V8 Phase 5: derive ClaimClass from data fit context (instrument_tier set later in
    // score_with_v5_context; here we use None as a conservative default).
    let claim_class = data_fit
        .as_ref()
        .map(|f| f.claim_class(None, false))
        .unwrap_or(ClaimClass::Triage);

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
        instrument_tier: None,
        trials_correction: None,
        forecast_points: 0.0,
        claim_class,
        discovery_gate_result: None,
        pricing_ledger: None,
        growth_coverage_gate: None,
    }
}

/// V8 Phase 1: instrument risk + trials correction overlay on top of the standard scorecard.
///
/// Calls `score_with_observables` then applies:
/// - **InstrumentRisk gate**: if `instrument_tier` is T0Formula or T1Emulator (not promotion-grade)
///   AND the data fit claims strong positive evidence (Δln Z > 2.0), the score is flagged with
///   an instrument-risk kill reason and `disqualified = true`. T2/T3 instruments are unaffected.
/// - **Forecast points**: added directly to `total` and `total_band` (not a rubric component).
/// - **Trials gate**: stored on the scorecard; callers can check `gate.passes(delta_lnz)`.
#[allow(clippy::too_many_arguments)]
pub fn score_with_v5_context(
    theory: &Theory,
    cg: &ClaimGraph,
    obligations: &[DerivationObligation],
    unification: &UnificationClaim,
    store: &dyn EvidenceStore,
    evidence_schema: &str,
    data_fit: Option<DataFitOutcome>,
    fit_observables: &[crate::ObservableRecord],
    instrument_tier: Option<crate::cosmology::ForwardTier>,
    trials_correction: Option<super::search_ledger::TrialsGate>,
    forecast_points: f64,
) -> ScorecardV4 {
    let mut sc = score_with_observables(
        theory,
        cg,
        obligations,
        unification,
        store,
        evidence_schema,
        data_fit.clone(),
        fit_observables,
    );

    sc.instrument_tier = instrument_tier;
    sc.trials_correction = trials_correction;
    sc.forecast_points = forecast_points.max(0.0).min(30.0);

    // Re-derive ClaimClass now that instrument_tier is known.
    sc.claim_class = data_fit
        .as_ref()
        .map(|f| f.claim_class(instrument_tier, sc.disqualified))
        .unwrap_or(ClaimClass::Triage);

    // InstrumentRisk gate: a sub-promotion-grade instrument cannot report strong evidence.
    if !sc.disqualified {
        if let Some(tier) = instrument_tier {
            if !tier.is_promotion_grade() {
                if let Some(ref f) = data_fit {
                    if f.delta_lnz > 2.0 {
                        sc.disqualified = true;
                        sc.kill_reasons.push(format!(
                            "instrument_risk[{}]: data_fit delta_lnz={:.2} > 2.0 but instrument \
                             tier {} is not promotion-grade (requires T2Boltzmann or T3CrossSolver); \
                             upgrade the solver before reporting this as evidence",
                            tier, f.delta_lnz, tier
                        ));
                        sc.components.clear();
                        sc.total = 0.0;
                        sc.total_band = (0.0, 0.0);
                        sc.forecast_points = 0.0;
                        return sc;
                    }
                }
            }
        }
    }

    // Apply forecast points to total.
    if sc.forecast_points > 0.0 && !sc.disqualified {
        sc.total += sc.forecast_points;
        sc.total_band = (
            sc.total_band.0 + sc.forecast_points,
            sc.total_band.1 + sc.forecast_points,
        );
    }

    sc
}

/// V8 Phase 11 (SYNTHESIS #3, #8): attach pricing and growth-coverage gates to a scorecard.
///
/// Call this after any of the `score*` functions to apply the two post-scoring gates:
///
/// - **PricingLedger gate** (SYNTHESIS #3): if `pricing.fails_closed()` (any DOF choice lacks an
///   explicit rubric price), the candidate is disqualified. The ledger is stored on the scorecard
///   for audit even when the gate fails.
/// - **GrowthCoverageGate** (SYNTHESIS #8): if `!growth.coverage_satisfied()` (fewer than the
///   five mandatory datasets are present for a suppressed-growth claim), the candidate is
///   disqualified. The gate is stored for audit.
///
/// If the scorecard is already disqualified (from an earlier gate), the fields are set for
/// auditability but no new kill reason is appended (the existing kill already blocks scoring).
pub fn apply_pricing_and_growth_gates(
    sc: &mut ScorecardV4,
    pricing: Option<super::pricing::PricingLedger>,
    growth: Option<super::growth_verdict::GrowthCoverageGate>,
) {
    // Store for audit regardless of current disqualification state.
    sc.pricing_ledger = pricing.clone();
    sc.growth_coverage_gate = growth.clone();

    if sc.disqualified {
        return; // already killed; don't append redundant reasons
    }

    // Gate 1: PricingLedger fails closed.
    if let Some(ref pl) = pricing {
        if pl.fails_closed() {
            let unpriced = pl.unpriced_count();
            sc.disqualified = true;
            sc.kill_reasons.push(format!(
                "pricing_ledger_fails_closed: {unpriced} DOF choice(s) lack an explicit \
                 rubric price; add pricing entries for all DOF before scoring"
            ));
            sc.components.clear();
            sc.total = 0.0;
            sc.total_band = (0.0, 0.0);
            sc.forecast_points = 0.0;
            return;
        }
    }

    // Gate 2: GrowthCoverageGate must be satisfied when present.
    if let Some(ref gcg) = growth {
        if !gcg.coverage_satisfied() {
            let missing = gcg.missing_coverage();
            sc.disqualified = true;
            sc.kill_reasons.push(format!(
                "growth_coverage_gate_failed: suppressed-growth claim requires all 5 \
                 mandatory dataset categories; missing: [{}]",
                missing.join(", ")
            ));
            sc.components.clear();
            sc.total = 0.0;
            sc.total_band = (0.0, 0.0);
            sc.forecast_points = 0.0;
        }
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
            equation_match: None,
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
            n_observations: 23,
            gof_outcome: None,
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
            n_observations: 23,
            gof_outcome: None,
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
            ..Default::default()
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
            equation_match: None,
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

    // ---- V8 Phase 1: InstrumentRisk gate + forecast_points ----

    fn strong_fit() -> DataFitOutcome {
        DataFitOutcome {
            delta_aic: -6.0,
            delta_lnz: 3.5,
            generalization_gap: 0.01,
            coverage: 1.0,
            boundary_hit: false,
            likelihood_mode: LikelihoodMode::Covariance,
            covariance_block_count: 3,
            n_observations: 23,
            gof_outcome: None,
        }
    }

    #[test]
    fn instrument_risk_gate_blocks_t0_with_strong_evidence() {
        use crate::cosmology::ForwardTier;
        let (t, cg, obs, uni, store) = fixture();
        // T0Formula + delta_lnz = 3.5 > 2.0 → must be blocked by InstrumentRisk gate.
        let sc = score_with_v5_context(
            &t,
            &cg,
            &obs,
            &uni,
            &store,
            "schema.v1",
            Some(strong_fit()),
            &[],
            Some(ForwardTier::T0Formula),
            None,
            0.0,
        );
        assert!(
            sc.disqualified,
            "T0 with delta_lnz > 2 must be disqualified"
        );
        assert!(
            sc.kill_reasons
                .iter()
                .any(|r| r.contains("instrument_risk")),
            "expected instrument_risk kill reason; got {:?}",
            sc.kill_reasons
        );
        assert_eq!(sc.total, 0.0);
    }

    #[test]
    fn instrument_risk_gate_blocks_t1_with_strong_evidence() {
        use crate::cosmology::ForwardTier;
        let (t, cg, obs, uni, store) = fixture();
        let sc = score_with_v5_context(
            &t,
            &cg,
            &obs,
            &uni,
            &store,
            "schema.v1",
            Some(strong_fit()),
            &[],
            Some(ForwardTier::T1Emulator),
            None,
            0.0,
        );
        assert!(sc.disqualified);
        assert!(sc
            .kill_reasons
            .iter()
            .any(|r| r.contains("instrument_risk")));
    }

    #[test]
    fn instrument_risk_gate_allows_t2_with_strong_evidence() {
        use crate::cosmology::ForwardTier;
        let (t, cg, obs, uni, store) = fixture();
        let sc = score_with_v5_context(
            &t,
            &cg,
            &obs,
            &uni,
            &store,
            "schema.v1",
            Some(strong_fit()),
            &[],
            Some(ForwardTier::T2Boltzmann),
            None,
            0.0,
        );
        // T2 is promotion-grade: strong evidence is reportable.
        assert!(
            !sc.disqualified,
            "T2 must not be blocked; reasons={:?}",
            sc.kill_reasons
        );
        assert!(sc.total > 0.0);
    }

    #[test]
    fn instrument_risk_gate_allows_t1_with_weak_evidence() {
        use crate::cosmology::ForwardTier;
        let (t, cg, obs, uni, store) = fixture();
        // T1 + delta_lnz = 1.0 (≤ 2.0) → not strong enough to trigger the gate.
        let sc = score_with_v5_context(
            &t,
            &cg,
            &obs,
            &uni,
            &store,
            "schema.v1",
            Some(good_fit()),
            &[],
            Some(ForwardTier::T1Emulator),
            None,
            0.0,
        );
        assert!(
            !sc.disqualified,
            "T1 with weak evidence must not be blocked; reasons={:?}",
            sc.kill_reasons
        );
    }

    #[test]
    fn forecast_points_added_to_total() {
        use crate::cosmology::ForwardTier;
        let (t, cg, obs, uni, store) = fixture();
        let sc_base = score(&t, &cg, &obs, &uni, &store, "schema.v1", Some(good_fit()));
        let sc_v5 = score_with_v5_context(
            &t,
            &cg,
            &obs,
            &uni,
            &store,
            "schema.v1",
            Some(good_fit()),
            &[],
            Some(ForwardTier::T1Emulator),
            None,
            15.0,
        );
        assert!(
            (sc_v5.total - sc_base.total - 15.0).abs() < 1e-9,
            "forecast_points must add to total: base={} v5={}",
            sc_base.total,
            sc_v5.total
        );
        assert_eq!(sc_v5.forecast_points, 15.0);
    }

    #[test]
    fn forecast_points_capped_at_30() {
        use crate::cosmology::ForwardTier;
        let (t, cg, obs, uni, store) = fixture();
        let sc = score_with_v5_context(
            &t,
            &cg,
            &obs,
            &uni,
            &store,
            "schema.v1",
            Some(good_fit()),
            &[],
            Some(ForwardTier::T2Boltzmann),
            None,
            999.0,
        );
        assert_eq!(
            sc.forecast_points, 30.0,
            "forecast_points must be capped at 30"
        );
    }

    #[test]
    fn trials_gate_stored_on_scorecard() {
        use crate::cosmology::ForwardTier;
        use crate::theory::search_ledger::TrialsGate;
        let (t, cg, obs, uni, store) = fixture();
        let gate = TrialsGate {
            n_trials: 50,
            base_ln_z: 2.0,
        };
        let sc = score_with_v5_context(
            &t,
            &cg,
            &obs,
            &uni,
            &store,
            "schema.v1",
            Some(good_fit()),
            &[],
            Some(ForwardTier::T1Emulator),
            Some(gate),
            0.0,
        );
        assert_eq!(sc.trials_correction, Some(gate));
    }

    // ---- V8 Phase 5: ClaimClass and GoFOutcome ----

    #[test]
    fn claim_class_n23_diagonal_is_interesting_fit() {
        let cc = ClaimClass::derive(23, LikelihoodMode::Diagonal, 5.0, None, false, false);
        assert_eq!(
            cc,
            ClaimClass::InterestingFit,
            "n=23 diagonal must be capped at InterestingFit"
        );
    }

    #[test]
    fn claim_class_n15_diagonal_is_triage() {
        let cc = ClaimClass::derive(15, LikelihoodMode::Diagonal, 5.0, None, false, false);
        assert_eq!(cc, ClaimClass::Triage, "n≤15 diagonal must be Triage");
    }

    #[test]
    fn claim_class_disqualified_is_triage() {
        use crate::cosmology::ForwardTier;
        let cc = ClaimClass::derive(
            100,
            LikelihoodMode::Covariance,
            10.0,
            Some(ForwardTier::T2Boltzmann),
            true,
            false,
        );
        assert_eq!(cc, ClaimClass::Triage, "disqualified must always be Triage");
    }

    #[test]
    fn claim_class_boundary_hit_caps_at_interesting_fit() {
        use crate::cosmology::ForwardTier;
        let cc = ClaimClass::derive(
            100,
            LikelihoodMode::Covariance,
            5.0,
            Some(ForwardTier::T2Boltzmann),
            false,
            true,
        );
        assert_eq!(
            cc,
            ClaimClass::InterestingFit,
            "boundary-hit blocks promotion"
        );
    }

    #[test]
    fn claim_class_covariance_strong_evidence_is_promotion_candidate() {
        use crate::cosmology::ForwardTier;
        let cc = ClaimClass::derive(
            50,
            LikelihoodMode::Covariance,
            3.0,
            Some(ForwardTier::T1Emulator),
            false,
            false,
        );
        assert_eq!(cc, ClaimClass::PromotionCandidate);
        assert!(cc.is_publishable());
    }

    #[test]
    fn scorecard_claim_class_set_on_survivor() {
        let (t, cg, obs, uni, store) = fixture();
        let sc = score(&t, &cg, &obs, &uni, &store, "schema.v1", Some(good_fit()));
        assert!(!sc.disqualified);
        assert_eq!(
            sc.claim_class,
            ClaimClass::InterestingFit,
            "n=23 diagonal survivor → InterestingFit"
        );
    }

    #[test]
    fn gof_gate_kills_bad_absolute_fit() {
        let (t, cg, obs, uni, store) = fixture();
        let bad_gof = GoFOutcome::from_chi_sq(50.0, 10, false);
        assert!(!bad_gof.gof_passed);
        let mut fit = good_fit();
        fit.likelihood_mode = LikelihoodMode::Covariance;
        fit.delta_lnz = 3.0;
        fit.gof_outcome = Some(bad_gof);
        let sc = score(&t, &cg, &obs, &uni, &store, "schema.v1", Some(fit));
        assert!(sc.disqualified, "chi_sq_dof=5.0 must kill the candidate");
        assert!(sc
            .kill_reasons
            .iter()
            .any(|r| r.contains("gof_gate_failed")));
    }

    #[test]
    fn gof_gate_passes_when_outcome_missing() {
        let (t, cg, obs, uni, store) = fixture();
        let sc = score(&t, &cg, &obs, &uni, &store, "schema.v1", Some(good_fit()));
        assert!(!sc.disqualified, "no gof_outcome → gate not triggered");
    }

    #[test]
    fn gof_outcome_from_chi_sq_smoke() {
        let g = GoFOutcome::from_chi_sq(20.0, 10, false);
        assert_eq!(g.chi_sq_dof, 2.0);
        assert!(!g.gof_passed);
        let g2 = GoFOutcome::from_chi_sq(10.0, 10, false);
        assert_eq!(g2.chi_sq_dof, 1.0);
        assert!(g2.gof_passed);
    }

    #[test]
    fn claim_class_label_round_trips() {
        for cc in [
            ClaimClass::Triage,
            ClaimClass::InterestingFit,
            ClaimClass::PromotionCandidate,
            ClaimClass::DiscoveryClaim,
            ClaimClass::ExclusionClaim,
        ] {
            assert!(!cc.label().is_empty());
        }
        assert!(!ClaimClass::Triage.is_publishable());
        assert!(!ClaimClass::InterestingFit.is_publishable());
        assert!(ClaimClass::PromotionCandidate.is_publishable());
    }

    fn passing_discovery_gate() -> super::profundity::DiscoveryClaimGate {
        use super::profundity::{DiscoveryClaimGate, MechanismOffTwin, ProfundityGate};
        DiscoveryClaimGate {
            profundity_gate: ProfundityGate {
                sigma_significance: 5.2,
                delta_ln_z_corrected: 6.0,
                mechanism_off_twin: Some(MechanismOffTwin {
                    delta_ln_z_with_mechanism: 6.0,
                    delta_ln_z_without_mechanism: 0.8,
                    zeroed_parameters: vec!["mu0".into()],
                }),
            },
            post_search_p_value: 0.001,
            post_search_n_replications: 300,
            gof_passed: true,
            instrument_tier: crate::cosmology::ForwardTier::T2Boltzmann,
            has_sealed_forecast: true,
        }
    }

    #[test]
    fn try_upgrade_promotion_candidate_with_passing_gate_yields_discovery_claim() {
        let gate = passing_discovery_gate();
        let cc = ClaimClass::PromotionCandidate.try_upgrade_to_discovery(&gate);
        assert_eq!(cc, ClaimClass::DiscoveryClaim);
    }

    #[test]
    fn try_upgrade_interesting_fit_stays_interesting_fit_even_with_passing_gate() {
        let gate = passing_discovery_gate();
        let cc = ClaimClass::InterestingFit.try_upgrade_to_discovery(&gate);
        assert_eq!(
            cc,
            ClaimClass::InterestingFit,
            "below PromotionCandidate cannot upgrade"
        );
    }

    #[test]
    fn try_upgrade_triage_stays_triage_with_passing_gate() {
        let gate = passing_discovery_gate();
        let cc = ClaimClass::Triage.try_upgrade_to_discovery(&gate);
        assert_eq!(cc, ClaimClass::Triage);
    }

    #[test]
    fn try_upgrade_fails_when_gate_blocks_missing_forecast() {
        use super::profundity::DiscoveryClaimGate;
        let mut gate = passing_discovery_gate();
        gate.has_sealed_forecast = false;
        let cc = ClaimClass::PromotionCandidate.try_upgrade_to_discovery(&gate);
        assert_eq!(
            cc,
            ClaimClass::PromotionCandidate,
            "blocked gate must not upgrade"
        );
    }

    #[test]
    fn discovery_gate_result_evaluate_records_pass() {
        let gate = passing_discovery_gate();
        let result = gate.evaluate();
        assert!(result.passed);
        assert!(result.blocking_reasons.is_empty());
        assert_eq!(result.mechanism_attribution_passed, Some(true));
    }

    #[test]
    fn discovery_gate_result_evaluate_records_failure_reasons() {
        use super::profundity::DiscoveryClaimGate;
        let mut gate = passing_discovery_gate();
        gate.has_sealed_forecast = false;
        gate.gof_passed = false;
        let result = gate.evaluate();
        assert!(!result.passed);
        assert!(
            result.blocking_reasons.len() >= 2,
            "expected ≥2 blocking reasons: {:?}",
            result.blocking_reasons
        );
    }

    // ---- V8 Phase 11: apply_pricing_and_growth_gates ----

    fn unpriced_ledger() -> super::super::pricing::PricingLedger {
        let mut pl = super::super::pricing::PricingLedger::new();
        pl.add("alpha_m", "alpha_m dial", 5.0, false); // not priced
        pl
    }

    fn priced_ledger() -> super::super::pricing::PricingLedger {
        let mut pl = super::super::pricing::PricingLedger::new();
        pl.add("alpha_m", "alpha_m dial", 5.0, true); // explicitly priced
        pl
    }

    fn unsatisfied_coverage() -> super::super::growth_verdict::GrowthCoverageGate {
        super::super::growth_verdict::GrowthCoverageGate {
            n_rsd_datasets: 0,
            n_wl_surveys: 0,
            has_cmb_lensing: false,
            has_bao: false,
            has_sne_pantheon_plus: false,
        }
    }

    fn satisfied_coverage() -> super::super::growth_verdict::GrowthCoverageGate {
        super::super::growth_verdict::GrowthCoverageGate {
            n_rsd_datasets: 1,
            n_wl_surveys: 2,
            has_cmb_lensing: true,
            has_bao: true,
            has_sne_pantheon_plus: true,
        }
    }

    #[test]
    fn pricing_fails_closed_disqualifies_survivor() {
        let (t, cg, obs, uni, store) = fixture();
        let mut sc = score(&t, &cg, &obs, &uni, &store, "schema.v1", Some(good_fit()));
        assert!(!sc.disqualified, "pre-condition: survivor");
        apply_pricing_and_growth_gates(&mut sc, Some(unpriced_ledger()), None);
        assert!(sc.disqualified, "unpriced ledger must kill");
        assert!(
            sc.kill_reasons
                .iter()
                .any(|r| r.contains("pricing_ledger_fails_closed")),
            "expected pricing kill reason; got {:?}",
            sc.kill_reasons
        );
        assert_eq!(sc.total, 0.0);
        assert!(sc.components.is_empty());
    }

    #[test]
    fn pricing_fully_priced_does_not_kill() {
        let (t, cg, obs, uni, store) = fixture();
        let mut sc = score(&t, &cg, &obs, &uni, &store, "schema.v1", Some(good_fit()));
        assert!(!sc.disqualified);
        let total_before = sc.total;
        apply_pricing_and_growth_gates(&mut sc, Some(priced_ledger()), None);
        assert!(!sc.disqualified, "priced ledger must not kill");
        assert!((sc.total - total_before).abs() < 1e-9);
        assert!(sc.pricing_ledger.is_some());
    }

    #[test]
    fn growth_coverage_not_satisfied_disqualifies() {
        let (t, cg, obs, uni, store) = fixture();
        let mut sc = score(&t, &cg, &obs, &uni, &store, "schema.v1", Some(good_fit()));
        assert!(!sc.disqualified);
        apply_pricing_and_growth_gates(&mut sc, None, Some(unsatisfied_coverage()));
        assert!(sc.disqualified, "unsatisfied coverage must kill");
        assert!(
            sc.kill_reasons
                .iter()
                .any(|r| r.contains("growth_coverage_gate_failed")),
            "expected coverage kill; got {:?}",
            sc.kill_reasons
        );
        assert_eq!(sc.total, 0.0);
    }

    #[test]
    fn growth_coverage_satisfied_does_not_kill() {
        let (t, cg, obs, uni, store) = fixture();
        let mut sc = score(&t, &cg, &obs, &uni, &store, "schema.v1", Some(good_fit()));
        assert!(!sc.disqualified);
        let total_before = sc.total;
        apply_pricing_and_growth_gates(&mut sc, None, Some(satisfied_coverage()));
        assert!(!sc.disqualified, "satisfied coverage must not kill");
        assert!((sc.total - total_before).abs() < 1e-9);
        assert!(sc.growth_coverage_gate.is_some());
    }

    #[test]
    fn already_disqualified_scorecard_gets_fields_set_without_new_kill_reason() {
        let (mut t, cg, obs, uni, store) = fixture();
        t.parameters.push(Parameter {
            symbol: "xi".into(),
            value: 0.3,
            physical_meaning: "free knob".into(),
            provenance: Provenance::Free,
        });
        let mut sc = score(&t, &cg, &obs, &uni, &store, "schema.v1", Some(good_fit()));
        assert!(sc.disqualified, "pre-condition: already killed by veto");
        let reason_count = sc.kill_reasons.len();
        apply_pricing_and_growth_gates(
            &mut sc,
            Some(unpriced_ledger()),
            Some(unsatisfied_coverage()),
        );
        assert!(sc.disqualified);
        assert_eq!(
            sc.kill_reasons.len(),
            reason_count,
            "no new kill reasons should be added to an already-disqualified scorecard"
        );
        assert!(
            sc.pricing_ledger.is_some(),
            "pricing_ledger field set for audit"
        );
        assert!(
            sc.growth_coverage_gate.is_some(),
            "growth_coverage_gate field set for audit"
        );
    }
}
