//! The `ForwardModel` seam: the single abstraction the evolution loop talks to, so that *how*
//! a theory's observables are computed (parametric stand-in today, in-repo derivation engine,
//! or an external Boltzmann backend tomorrow) can change without touching selection.
//!
//! A model maps a theory representation to `PredictionRecord`s and reports a `ForwardManifest`
//! describing exactly which code + data produced them, so a score is reproducible and the
//! provenance is auditable.

use crate::types::PredictionRecord;
use serde::{Deserialize, Serialize};

/// Typed failure modes for a forward model — emitted instead of an opaque anyhow error so the
/// engine can route each failure class (log it, skip the theory, escalate to a Boltzmann
/// backend, etc.) without parsing error strings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForwardFailure {
    /// This observable / sector combination is not implemented by this model tier.
    Unsupported,
    /// Physical parameters are outside the model's valid domain (e.g. negative Omega_m).
    DomainError,
    /// Numerical failure: overflow, NaN, or non-convergence in an integration / ODE solver.
    PrecisionFailure,
    /// Solver exceeded its wall-clock or iteration budget.
    Timeout,
    /// A subprocess or external backend (CLASS, hi_class) exited unexpectedly.
    Crash,
}

impl std::fmt::Display for ForwardFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ForwardFailure::Unsupported => write!(f, "forward_failure:unsupported"),
            ForwardFailure::DomainError => write!(f, "forward_failure:domain_error"),
            ForwardFailure::PrecisionFailure => write!(f, "forward_failure:precision_failure"),
            ForwardFailure::Timeout => write!(f, "forward_failure:timeout"),
            ForwardFailure::Crash => write!(f, "forward_failure:crash"),
        }
    }
}

impl std::error::Error for ForwardFailure {}

/// Bundles a successful prediction set with the reproducibility receipt for the model that
/// produced it — useful when callers need both in one place.
#[derive(Debug, Clone)]
pub struct ForwardOutcome {
    pub records: Vec<PredictionRecord>,
    pub manifest: ForwardManifest,
}

/// What a forward model fundamentally is — used to flag the fidelity of a prediction set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForwardKind {
    /// Transparent parametric stand-in (e.g. an identity/heuristic map). NOT a solver.
    Parametric,
    /// In-repo numeric derivation (ODE / quadrature integration from physical parameters).
    Derivation,
    /// External Boltzmann solver backend (CLASS / hi_class / cobaya).
    Boltzmann,
}

/// V8 Phase 1 (#6): the instrument tier of a score — how trustworthy the forward model is.
/// Ordered from lowest (T0) to highest (T3) fidelity. Any promotion-grade claim (ΔlnZ > 2,
/// "beats ΛCDM") requires T2 or above; a T0 score is provisional only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForwardTier {
    /// T0: Fitting formula / parametric stand-in. Fast; instrument risk HIGH. Cannot promote.
    T0Formula,
    /// T1: In-repo numerical integration or fitted emulator. Medium accuracy. Provisional results.
    T1Emulator,
    /// T2: External Boltzmann solver (CLASS / hi_class). Required for promotion-grade claims.
    T2Boltzmann,
    /// T3: Cross-solver consistency (UltraNest + dynesty). Both solvers agree within 2 SE.
    /// Publication grade — this tier is required for the 5σ threshold.
    T3CrossSolver,
}

impl ForwardTier {
    /// True when this tier is sufficient to make promotion-grade claims (ΔlnZ ≥ 2 reportable).
    pub fn is_promotion_grade(self) -> bool {
        self >= ForwardTier::T2Boltzmann
    }

    /// True when this tier is sufficient to make publication-grade claims (5σ threshold).
    pub fn is_publication_grade(self) -> bool {
        self == ForwardTier::T3CrossSolver
    }

    pub fn as_str(self) -> &'static str {
        match self {
            ForwardTier::T0Formula => "T0Formula",
            ForwardTier::T1Emulator => "T1Emulator",
            ForwardTier::T2Boltzmann => "T2Boltzmann",
            ForwardTier::T3CrossSolver => "T3CrossSolver",
        }
    }
}

impl std::fmt::Display for ForwardTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Reproducibility receipt for a forward model: stamped into every score so two runs with the
/// same manifest hash are provably comparable, and a different code/data version is visible.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForwardManifest {
    /// Stable identifier of the model implementation.
    pub model_id: String,
    /// Version of the model / code path.
    pub version: String,
    /// What class of computation produced the predictions.
    pub kind: ForwardKind,
    /// V8 Phase 1: instrument fidelity tier (T0–T3). Governs what claims are reportable.
    #[serde(default = "default_forward_tier")]
    pub tier: ForwardTier,
    /// Content hash of the external code + data the predictions depend on. Empty for a pure,
    /// deterministic in-repo model that needs no external inputs.
    pub provenance_hash: String,
}

fn default_forward_tier() -> ForwardTier {
    ForwardTier::T0Formula
}

/// A forward model turns a theory representation into predicted observables.
///
/// The `Theory` associated type keeps this decoupled from any one representation: the
/// background engine below uses [`super::CosmologyParams`]; a future symbolic-derivation
/// engine will use the `Theory` AST without changing this trait or the selection loop.
pub trait ForwardModel {
    /// The theory representation this model maps to observables.
    type Theory;

    /// Predict the requested observables. A model returns a record only for the observables it
    /// can genuinely compute — an unpredictable observable (e.g. σ8 from a background-only
    /// model) is *omitted*, never faked, so coverage honestly reflects what the theory derives.
    ///
    /// Returns `Err(ForwardFailure::Unsupported)` when the model cannot handle this theory /
    /// observable combination at all; other variants signal numeric or external-process failures.
    fn predict(
        &self,
        theory: &Self::Theory,
        observable_ids: &[String],
    ) -> Result<Vec<PredictionRecord>, ForwardFailure>;

    /// Reproducibility receipt for this model.
    fn manifest(&self) -> ForwardManifest;
}

// =====================================================================================
// V8 Phase 24 (SYNTHESIS #6): SolverManifest + InstrumentRiskKind
//
// A `ForwardManifest` describes what code produced a score. When that code is a Boltzmann
// solver (CLASS, hi_class), the `SolverManifest` records the *pinned binary* — its name,
// version string, and SHA-256 content hash — so silent solver drift is detected before a
// score is accepted as promotion-grade. The `InstrumentRiskKind` gate produces a typed
// reason when the instrument tier is inconsistent with the claim being made.
// =====================================================================================

/// Hash-sealed pin for a Boltzmann solver binary (CLASS, hi_class, etc.).
///
/// Stamped into the `ForwardManifest` for T2/T3 scores. The contract: if the solver binary
/// changes (version bump, security patch, accidental swap), the SHA-256 diverges from the
/// pinned value and the gate rejects the score before it enters the ranking. Two runs with the
/// same `binary_sha256` used the same solver and their ΔlnZ values are directly comparable.
///
/// The hash is a hex-encoded SHA-256 of the solver binary. For classical text-mode Boltzmann
/// codes (CLASS, hi_class) this is `sha256sum <binary>`. The engine does NOT compute this at
/// runtime; the caller supplies it (typically from a CI artifact or a `sha256sum` hook) to
/// keep the core crate free of file I/O.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SolverManifest {
    /// Short solver name (e.g. `"CLASS"`, `"hi_class"`, `"UltraNest"`).
    pub solver_name: String,
    /// Version string as reported by the binary (e.g. `"3.2.2"`, `"1.3.5"`).
    pub version_string: String,
    /// Hex-encoded SHA-256 of the solver binary (64 lowercase hex chars).
    /// Empty string means the binary was not pinned; scores from an unpinned solver
    /// are at most T1 (the tier gate enforces this via `InstrumentRiskKind`).
    pub binary_sha256: String,
    /// Optional hex SHA-256 of the solver's parameter/configuration file.
    /// Empty string means the configuration was not pinned.
    pub config_sha256: String,
    /// The tier this solver is certified for. Must be ≥ T2Boltzmann.
    pub tier: ForwardTier,
}

impl SolverManifest {
    /// True when the binary hash is non-empty (the binary has been pinned).
    pub fn is_pinned(&self) -> bool {
        !self.binary_sha256.is_empty()
    }

    /// True when the binary hash is exactly 64 lowercase hex characters.
    pub fn has_valid_hash_format(&self) -> bool {
        self.binary_sha256.len() == 64 && self.binary_sha256.chars().all(|c| c.is_ascii_hexdigit())
    }

    /// Compare the observed binary hash against the pinned value. Returns true when they match
    /// (or when no pin is declared — callers should treat an unpinned manifest as T1).
    /// The comparison is case-insensitive to tolerate `sha256sum` vs uppercase hex sources.
    pub fn verify_hash(&self, observed_sha256: &str) -> bool {
        if self.binary_sha256.is_empty() {
            return true; // no pin declared — caller decides how to handle
        }
        self.binary_sha256.eq_ignore_ascii_case(observed_sha256)
    }

    /// Canonical T2 CLASS pin stub (for tests / offline environments where the binary is absent).
    /// The hash is a placeholder zeros string — not a real binary hash.
    pub fn class_stub() -> Self {
        SolverManifest {
            solver_name: "CLASS".into(),
            version_string: "3.2.2".into(),
            binary_sha256: "0".repeat(64),
            config_sha256: String::new(),
            tier: ForwardTier::T2Boltzmann,
        }
    }

    /// Canonical T2 hi_class pin stub (for tests / offline environments).
    pub fn hi_class_stub() -> Self {
        SolverManifest {
            solver_name: "hi_class".into(),
            version_string: "1.3.5".into(),
            binary_sha256: "0".repeat(64),
            config_sha256: String::new(),
            tier: ForwardTier::T2Boltzmann,
        }
    }
}

/// The kind of instrument-risk mismatch found by the tier-consistency gate (SYNTHESIS #6).
///
/// When a score's `ForwardTier` is lower than the claim being made, the gate produces a typed
/// reason rather than a numeric penalty — the score is blocked, not down-weighted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstrumentRiskKind {
    /// The claim requires T2 (Boltzmann) or T3 but the score is T0 or T1.
    TierTooLow {
        required: ForwardTier,
        actual: ForwardTier,
    },
    /// The solver manifest is absent but a T2 score was claimed.
    SolverManifestMissing,
    /// The solver binary hash diverges from the pinned value — the binary changed.
    SolverHashMismatch { pinned: String, observed: String },
    /// The solver manifest declares a tier lower than T2 for a Boltzmann-required claim.
    ManifestTierInsufficient { manifest_tier: ForwardTier },
}

impl std::fmt::Display for InstrumentRiskKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstrumentRiskKind::TierTooLow { required, actual } => {
                write!(
                    f,
                    "instrument_risk:tier_too_low(required={required:?},actual={actual:?})"
                )
            }
            InstrumentRiskKind::SolverManifestMissing => {
                write!(f, "instrument_risk:solver_manifest_missing")
            }
            InstrumentRiskKind::SolverHashMismatch { pinned, observed } => write!(
                f,
                "instrument_risk:solver_hash_mismatch(pinned={}…,observed={}…)",
                &pinned[..8.min(pinned.len())],
                &observed[..8.min(observed.len())]
            ),
            InstrumentRiskKind::ManifestTierInsufficient { manifest_tier } => write!(
                f,
                "instrument_risk:manifest_tier_insufficient({manifest_tier:?})"
            ),
        }
    }
}

/// Run the tier-consistency instrument risk gate.
///
/// Returns `None` when the score is consistent with its claimed tier. Returns
/// `Some(InstrumentRiskKind)` when the first inconsistency is found — the score must be
/// rejected or downgraded before the ranking accepts it.
///
/// Rules (applied in order):
/// 1. If the score tier is below the required minimum, emit `TierTooLow`.
/// 2. If the score is T2/T3 and no solver manifest is supplied, emit `SolverManifestMissing`.
/// 3. If the manifest's `tier` is below `T2Boltzmann`, emit `ManifestTierInsufficient`.
/// 4. If the manifest declares a hash and the observed hash diverges, emit `SolverHashMismatch`.
pub fn check_instrument_risk(
    score_tier: ForwardTier,
    required_tier: ForwardTier,
    solver_manifest: Option<&SolverManifest>,
    observed_binary_sha256: Option<&str>,
) -> Option<InstrumentRiskKind> {
    if score_tier < required_tier {
        return Some(InstrumentRiskKind::TierTooLow {
            required: required_tier,
            actual: score_tier,
        });
    }
    if score_tier >= ForwardTier::T2Boltzmann {
        match solver_manifest {
            None => return Some(InstrumentRiskKind::SolverManifestMissing),
            Some(manifest) => {
                if manifest.tier < ForwardTier::T2Boltzmann {
                    return Some(InstrumentRiskKind::ManifestTierInsufficient {
                        manifest_tier: manifest.tier,
                    });
                }
                if let Some(observed) = observed_binary_sha256 {
                    if !manifest.verify_hash(observed) {
                        return Some(InstrumentRiskKind::SolverHashMismatch {
                            pinned: manifest.binary_sha256.clone(),
                            observed: observed.to_string(),
                        });
                    }
                }
            }
        }
    }
    None
}

/// The pure-Rust background-cosmology forward model (Tier-0 observables). Deterministic and
/// self-contained: distances, BAO ratios, sound horizon, and BBN Y_p are integrated from the
/// parameters; observables that require linear growth or the full CMB are deliberately omitted
/// (they are the optional Boltzmann backend's job).
#[derive(Debug, Clone, Default)]
pub struct BackgroundForwardModel;

impl BackgroundForwardModel {
    pub const MODEL_ID: &'static str = "openqg-background";
    pub const VERSION: &'static str = "0.1.0";

    /// Compute a single observable id, or `None` if this background model cannot derive it.
    /// Redshift-dependent observables use the `name@<z>` convention, e.g. `dv_over_rd@0.51`.
    fn compute(&self, c: &super::CosmologyParams, id: &str) -> Option<(f64, f64, &'static str)> {
        // (value, default 1σ model uncertainty, unit). The uncertainty is a small numerical
        // floor; the data record's own uncertainty dominates the likelihood.
        let parse_z = |id: &str, prefix: &str| -> Option<f64> {
            id.strip_prefix(prefix)
                .and_then(|rest| rest.strip_prefix('@'))
                .and_then(|z| z.parse::<f64>().ok())
        };
        match id {
            "h0" | "h0_local" => Some((c.h0(), 0.001, "km s^-1 Mpc^-1")),
            "omega_m" => Some((c.omega_m, 1e-6, "dimensionless")),
            "sum_mnu" => Some((c.sum_mnu, 1e-6, "eV")),
            "n_eff" => Some((c.n_eff, 1e-6, "dimensionless")),
            "omega_b_h2" => Some((c.omega_b_h2, 1e-9, "dimensionless")),
            "r_drag" => Some((c.sound_horizon_drag(), 0.05, "Mpc")),
            "bbn_yp" | "yp" => Some((c.bbn_helium_fraction(), 1e-5, "dimensionless")),
            // Compressed CMB distance priors (computable from the background alone).
            // V6.1 (P0.11, CRITICAL): fitting-formula-grade predictions must never meet
            // Boltzmann-grade data raw — the engine's lA carried a +0.755 (8.4σ) bias at the
            // Planck anchor and the V6 campaign optimizer harvested ~35 nat of pure model
            // error by drifting h to shift lA. Anchor-calibrate at Planck-2018 best fit
            // (same precedent as the Aubourg r_drag treatment): planck_lcdm() now predicts the
            // published distance priors exactly; deviations measure PHYSICS, not formula bias.
            // Guard: `calibrated_anchor_matches_planck_distance_priors` below.
            "cmb_R" => Some((
                c.cmb_shift_r() + CMB_R_ANCHOR_CALIBRATION,
                0.001,
                "dimensionless",
            )),
            "cmb_lA" => Some((
                c.cmb_acoustic_scale() + CMB_LA_ANCHOR_CALIBRATION,
                0.01,
                "dimensionless",
            )),
            // The third Planck-2018 compressed-CMB prior (Chen, Huang & Wang 2019,
            // arXiv:1808.05724): the baryon density ω_b h². It is a background parameter, so the
            // model just reports it; the `cmb_` alias lets the 3×3 (R, ℓ_A, ω_b h²) covariance
            // block (`scoring/covariance.rs`) name a single ordered observable set.
            "cmb_omega_b_h2" => Some((c.omega_b_h2, 1e-9, "dimensionless")),
            // Growth-of-structure observables (Tier-1): linear growth integrated from the
            // background + the late-time μ0 modified-gravity handle.
            "s8" | "S8" => Some((c.s8(), 1e-4, "dimensionless")),
            "sigma8" => Some((c.sigma8, 1e-6, "dimensionless")),
            _ => {
                if let Some(z) = parse_z(id, "dm_over_rd") {
                    Some((c.bao_dm_over_rd(z), 0.01, "dimensionless"))
                } else if let Some(z) = parse_z(id, "dh_over_rd") {
                    Some((c.bao_dh_over_rd(z), 0.01, "dimensionless"))
                } else if let Some(z) = parse_z(id, "dv_over_rd") {
                    Some((c.bao_dv_over_rd(z), 0.01, "dimensionless"))
                } else if let Some(z) = parse_z(id, "mu") {
                    Some((c.distance_modulus(z), 1e-4, "mag"))
                } else if let Some(z) = parse_z(id, "fsigma8") {
                    // M4: a declared *derived* MG family (f(R)/nDGP) grows the reference k-mode with
                    // its computed scale-dependent μ(a,k); plain ΛCDM/μ0 keeps the scale-free path
                    // (growth_fsigma8_kref == growth_fsigma8 when mg_family == None — byte-identical).
                    let fs8 = if c.mg_family == super::MgFamily::None {
                        c.growth_fsigma8(z)
                    } else {
                        c.growth_fsigma8_kref(z)
                    };
                    Some((fs8, 1e-4, "dimensionless"))
                } else {
                    None
                }
            }
        }
    }
}

/// V6.1 anchor calibrations: published Planck-2018 distance priors (Chen, Huang & Wang 2019,
/// Table I) minus this engine's fitting-formula predictions at `CosmologyParams::planck_lcdm()`.
/// Measured 2026-06-11: raw lA = 302.225598 vs 301.471 published; raw R = 1.749003 vs 1.7502.
pub const CMB_LA_ANCHOR_CALIBRATION: f64 = 301.471 - 302.225_598;
pub const CMB_R_ANCHOR_CALIBRATION: f64 = 1.7502 - 1.749_003;

impl ForwardModel for BackgroundForwardModel {
    type Theory = super::CosmologyParams;

    fn predict(
        &self,
        theory: &Self::Theory,
        observable_ids: &[String],
    ) -> Result<Vec<PredictionRecord>, ForwardFailure> {
        let mut out = Vec::new();
        for id in observable_ids {
            if let Some((value, uncertainty, unit)) = self.compute(theory, id) {
                out.push(PredictionRecord {
                    observable_id: id.clone(),
                    value,
                    uncertainty,
                    unit: unit.to_string(),
                    theory_id: Some(Self::MODEL_ID.to_string()),
                });
            }
        }
        Ok(out)
    }

    fn manifest(&self) -> ForwardManifest {
        ForwardManifest {
            model_id: Self::MODEL_ID.to_string(),
            version: Self::VERSION.to_string(),
            kind: ForwardKind::Derivation,
            // In-repo numerical derivation (ODE integration + fitting formulae) → T1Emulator.
            // Promotion-grade claims require upgrading to CLASS/hi_class (T2Boltzmann).
            tier: ForwardTier::T1Emulator,
            // Pure, deterministic, no external inputs ⇒ no external provenance to hash.
            provenance_hash: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::CosmologyParams;
    use super::*;

    #[test]
    fn predicts_derived_and_parameter_observables() {
        let model = BackgroundForwardModel;
        let c = CosmologyParams::planck_lcdm();
        let ids: Vec<String> = ["h0", "dv_over_rd@0.51", "bbn_yp", "r_drag", "mu@0.5"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let preds = model.predict(&c, &ids).unwrap();
        assert_eq!(preds.len(), 5);
        let h0 = preds.iter().find(|p| p.observable_id == "h0").unwrap();
        assert!((h0.value - 67.4).abs() < 1e-6);
        let dv = preds
            .iter()
            .find(|p| p.observable_id == "dv_over_rd@0.51")
            .unwrap();
        assert!(dv.value > 10.0 && dv.value < 16.0);
    }

    #[test]
    fn omits_observables_it_cannot_derive_rather_than_faking() {
        // The background+growth model must NOT invent a full-CMB-spectrum observable (a C_ℓ band
        // power needs the Boltzmann backend) — coverage stays honest.
        let model = BackgroundForwardModel;
        let c = CosmologyParams::planck_lcdm();
        let preds = model
            .predict(&c, &["cl_tt@220".to_string(), "h0".to_string()])
            .unwrap();
        assert_eq!(preds.len(), 1);
        assert_eq!(preds[0].observable_id, "h0");
    }

    #[test]
    fn predicts_growth_observables() {
        let model = BackgroundForwardModel;
        let c = CosmologyParams::planck_lcdm();
        let ids: Vec<String> = ["fsigma8@0.5", "s8", "sigma8"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let preds = model.predict(&c, &ids).unwrap();
        assert_eq!(preds.len(), 3);
        let fs8 = preds
            .iter()
            .find(|p| p.observable_id == "fsigma8@0.5")
            .unwrap();
        assert!(fs8.value > 0.40 && fs8.value < 0.50, "fσ8 = {}", fs8.value);
    }

    #[test]
    fn manifest_is_stable_and_marks_derivation() {
        let m = BackgroundForwardModel.manifest();
        assert_eq!(m.kind, ForwardKind::Derivation);
        assert_eq!(m.tier, ForwardTier::T1Emulator);
        assert!(
            !m.tier.is_promotion_grade(),
            "background model is not promotion grade"
        );
        assert_eq!(m.model_id, "openqg-background");
        assert!(m.provenance_hash.is_empty());
    }
}

#[cfg(test)]
mod v61_calibration_guard {
    use super::*;
    use crate::cosmology::CosmologyParams;

    /// THE P0.11 guard: the ΛCDM baseline must hit the published Planck distance priors at the
    /// anchor — a drifting fitting formula re-opens the model-bias exploit and fails here first.
    #[test]
    fn calibrated_anchor_matches_planck_distance_priors() {
        let model = BackgroundForwardModel;
        let p = CosmologyParams::planck_lcdm();
        let ids = vec![
            "cmb_R".to_string(),
            "cmb_lA".to_string(),
            "cmb_omega_b_h2".to_string(),
        ];
        let preds = model.predict(&p, &ids).unwrap();
        let get = |id: &str| preds.iter().find(|x| x.observable_id == id).unwrap().value;
        // Published values + sigmas: R 1.7502±0.0046, lA 301.471±0.090, wb 0.02236±0.00015.
        assert!((get("cmb_R") - 1.7502).abs() < 0.2 * 0.0046, "R off anchor");
        assert!(
            (get("cmb_lA") - 301.471).abs() < 0.2 * 0.090,
            "lA off anchor"
        );
        assert!(
            (get("cmb_omega_b_h2") - 0.02236).abs() < 0.5 * 0.000_15,
            "omega_b_h2 off anchor"
        );
    }
}

#[cfg(test)]
mod paper_probe {
    use super::*;
    use crate::cosmology::CosmologyParams;

    #[test]
    #[ignore] // paper-figure data probe, run explicitly
    fn print_predictions_for_paper() {
        let model = BackgroundForwardModel;
        let ids: Vec<String> = [
            "h0",
            "s8",
            "cmb_R",
            "cmb_lA",
            "cmb_omega_b_h2",
            "fsigma8@0.067",
            "fsigma8@0.38",
            "fsigma8@0.51",
            "fsigma8@0.61",
            "fsigma8@1.48",
            "dv_over_rd@0.295",
            "dm_over_rd@0.510",
            "dh_over_rd@0.510",
            "dm_over_rd@0.930",
            "dh_over_rd@0.930",
            "dm_over_rd@2.330",
            "dh_over_rd@2.330",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let lcdm = CosmologyParams::planck_lcdm();
        let mut mu = lcdm.clone();
        mu.mu0 = -0.1;
        let mut drag = lcdm.clone();
        drag.w0 = -0.9;
        drag.drag_a = 2.0;
        for (name, bg) in [("lcdm", &lcdm), ("mu0", &mu), ("drag", &drag)] {
            for p in model.predict(bg, &ids).unwrap() {
                println!("PROBE {} {} {:.6}", name, p.observable_id, p.value);
            }
        }
    }
}

// ---- V8 Phase 24: SolverManifest + InstrumentRiskKind tests ----

#[cfg(test)]
mod solver_manifest_tests {
    use super::*;

    fn real_hash() -> String {
        "a".repeat(64)
    }

    fn different_hash() -> String {
        "b".repeat(64)
    }

    // ---- SolverManifest ----

    #[test]
    fn class_stub_is_pinned_and_t2() {
        let m = SolverManifest::class_stub();
        assert!(m.is_pinned());
        assert_eq!(m.tier, ForwardTier::T2Boltzmann);
        assert_eq!(m.solver_name, "CLASS");
    }

    #[test]
    fn hi_class_stub_is_pinned_and_t2() {
        let m = SolverManifest::hi_class_stub();
        assert!(m.is_pinned());
        assert_eq!(m.tier, ForwardTier::T2Boltzmann);
        assert_eq!(m.solver_name, "hi_class");
    }

    #[test]
    fn empty_hash_is_not_pinned() {
        let mut m = SolverManifest::class_stub();
        m.binary_sha256 = String::new();
        assert!(!m.is_pinned());
    }

    #[test]
    fn valid_hash_format_requires_64_hex_chars() {
        let mut m = SolverManifest::class_stub();
        m.binary_sha256 = real_hash();
        assert!(m.has_valid_hash_format());
        m.binary_sha256 = "abc".into(); // too short
        assert!(!m.has_valid_hash_format());
        m.binary_sha256 = "g".repeat(64); // not hex
        assert!(!m.has_valid_hash_format());
    }

    #[test]
    fn verify_hash_matches_exact_value() {
        let mut m = SolverManifest::class_stub();
        m.binary_sha256 = real_hash();
        assert!(m.verify_hash(&real_hash()));
        assert!(!m.verify_hash(&different_hash()));
    }

    #[test]
    fn verify_hash_is_case_insensitive() {
        let mut m = SolverManifest::class_stub();
        m.binary_sha256 = "a".repeat(64);
        assert!(m.verify_hash(&"A".repeat(64)));
    }

    #[test]
    fn verify_hash_unpinned_always_passes() {
        let mut m = SolverManifest::class_stub();
        m.binary_sha256 = String::new();
        assert!(
            m.verify_hash(&different_hash()),
            "unpinned: any hash passes"
        );
    }

    #[test]
    fn serde_round_trip() {
        let m = SolverManifest::class_stub();
        let json = serde_json::to_string(&m).unwrap();
        let back: SolverManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(back, m);
    }

    // ---- InstrumentRiskKind + check_instrument_risk ----

    #[test]
    fn t0_score_when_t2_required_gives_tier_too_low() {
        let risk =
            check_instrument_risk(ForwardTier::T0Formula, ForwardTier::T2Boltzmann, None, None);
        assert!(matches!(
            risk,
            Some(InstrumentRiskKind::TierTooLow {
                required: ForwardTier::T2Boltzmann,
                actual: ForwardTier::T0Formula
            })
        ));
    }

    #[test]
    fn t2_score_without_manifest_gives_manifest_missing() {
        let risk = check_instrument_risk(
            ForwardTier::T2Boltzmann,
            ForwardTier::T2Boltzmann,
            None,
            None,
        );
        assert_eq!(risk, Some(InstrumentRiskKind::SolverManifestMissing));
    }

    #[test]
    fn t2_score_with_matching_hash_passes() {
        let mut m = SolverManifest::class_stub();
        m.binary_sha256 = real_hash();
        let risk = check_instrument_risk(
            ForwardTier::T2Boltzmann,
            ForwardTier::T2Boltzmann,
            Some(&m),
            Some(&real_hash()),
        );
        assert!(risk.is_none(), "matching hash must pass; got {risk:?}");
    }

    #[test]
    fn t2_score_with_mismatched_hash_gives_hash_mismatch() {
        let mut m = SolverManifest::class_stub();
        m.binary_sha256 = real_hash();
        let risk = check_instrument_risk(
            ForwardTier::T2Boltzmann,
            ForwardTier::T2Boltzmann,
            Some(&m),
            Some(&different_hash()),
        );
        assert!(
            matches!(risk, Some(InstrumentRiskKind::SolverHashMismatch { .. })),
            "hash mismatch must produce SolverHashMismatch; got {risk:?}"
        );
    }

    #[test]
    fn manifest_with_t1_tier_gives_manifest_tier_insufficient() {
        let mut m = SolverManifest::class_stub();
        m.tier = ForwardTier::T1Emulator;
        let risk = check_instrument_risk(
            ForwardTier::T2Boltzmann,
            ForwardTier::T2Boltzmann,
            Some(&m),
            None,
        );
        assert!(
            matches!(
                risk,
                Some(InstrumentRiskKind::ManifestTierInsufficient { .. })
            ),
            "T1 manifest for T2 claim must be rejected; got {risk:?}"
        );
    }

    #[test]
    fn t0_score_when_t0_required_with_no_manifest_passes() {
        // T0 scores don't need a solver manifest.
        let risk =
            check_instrument_risk(ForwardTier::T0Formula, ForwardTier::T0Formula, None, None);
        assert!(
            risk.is_none(),
            "T0/T0 with no manifest must pass; got {risk:?}"
        );
    }

    #[test]
    fn instrument_risk_display_is_informative() {
        let r = InstrumentRiskKind::TierTooLow {
            required: ForwardTier::T2Boltzmann,
            actual: ForwardTier::T0Formula,
        };
        let s = r.to_string();
        assert!(s.contains("tier_too_low"), "{s}");
    }
}
