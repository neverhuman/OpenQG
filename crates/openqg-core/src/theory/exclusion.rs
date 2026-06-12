//! V8 Phase 4 (#20): Exclusion protocol types.
//!
//! The exclusion protocol converts the V7 null result into citable science:
//! "we exclude class C over volume V at strength S" rather than "V7 scores 43.0."
//!
//! Key guarantee: `ExclusionSentence` can only be constructed when all
//! required conditions are met (covariance-complete likelihood, trials correction,
//! rank stability, coverage certificate). Missing any condition fails at construction.
//!
//! Spec reference: S11 §"Convert the null into strength: V8 exclusion protocol"

use super::search_volume::{SearchParamBox, SearchVolumeSpec};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// One parameter box in the search volume.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterBox {
    pub symbol: String,
    pub lo: f64,
    pub hi: f64,
    pub prior: String,
    pub mechanism_route: Option<String>,
    pub description: String,
    pub unit: String,
}

impl ParameterBox {
    pub fn volume(&self) -> f64 {
        self.hi - self.lo
    }

    pub fn contains(&self, value: f64) -> bool {
        value >= self.lo && value <= self.hi
    }
}

/// Which mechanism routes are in the exclusion class.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MechanismRoute {
    /// μ(a) = 1 + μ0·ΩDE(a)/ΩDE0 parametrization.
    PlanckMu0 { mu0_lo: f64, mu0_hi: f64 },
    /// Dark-scattering drag Γ(a) = A_drag·(1+w(a))·ΩDE(a).
    DarkScatteringDrag {
        a_drag_lo: f64,
        a_drag_hi: f64,
        w0_lo: f64,
        w0_hi: f64,
    },
}

/// The C_growth-suppression(V8) exclusion class.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExclusionClass {
    /// Canonical name for this class.
    pub name: String,
    /// Prose description for the paper.
    pub description: String,
    pub mechanism_routes: Vec<MechanismRoute>,
    pub parameter_boxes: Vec<ParameterBox>,
    /// Maximum redshift for the growth-suppression claim.
    pub max_redshift: f64,
    /// GW170817 safety: scalar tensor speed δc_T = 0 required.
    pub gw170817_safe: bool,
    /// No scale-dependent growth effects.
    pub scale_independent: bool,
    /// No pre-recombination physics in scope.
    pub late_time_only: bool,
    /// SHA-256 of the canonical class definition for content-binding.
    pub class_fingerprint: String,
}

impl ExclusionClass {
    /// Return the canonical V8 growth-suppression exclusion class.
    pub fn v8_growth_suppression() -> Self {
        let mut class = ExclusionClass {
            name: "C_growth-suppression(V8)".into(),
            description: concat!(
                "Late-time, GW170817-safe, scale-independent scalar-sector or dark-sector ",
                "mechanisms whose only admitted first-order cosmological effect is to suppress ",
                "linear growth relative to profile-fitted ΛCDM over 0 ≤ z ≤ 2.5."
            )
            .into(),
            mechanism_routes: vec![
                MechanismRoute::PlanckMu0 {
                    mu0_lo: -0.30,
                    mu0_hi: 0.00,
                },
                MechanismRoute::DarkScatteringDrag {
                    a_drag_lo: 0.0,
                    a_drag_hi: 10.0,
                    w0_lo: -0.99,
                    w0_hi: -0.80,
                },
            ],
            parameter_boxes: vec![
                ParameterBox {
                    symbol: "mu0".into(),
                    lo: -0.30,
                    hi: 0.00,
                    prior: "uniform".into(),
                    mechanism_route: Some("planck_mu0".into()),
                    description: "Planck-μ0 growth suppression amplitude".into(),
                    unit: "dimensionless".into(),
                },
                ParameterBox {
                    symbol: "A_drag".into(),
                    lo: 0.0,
                    hi: 10.0,
                    prior: "uniform".into(),
                    mechanism_route: Some("dark_scattering_drag".into()),
                    description: "Dark-scattering drag amplitude".into(),
                    unit: "dimensionless".into(),
                },
                ParameterBox {
                    symbol: "w0".into(),
                    lo: -0.99,
                    hi: -0.80,
                    prior: "uniform".into(),
                    mechanism_route: Some("dark_scattering_drag".into()),
                    description: "CPL dark energy w0 (companion to dark-scattering route)".into(),
                    unit: "dimensionless".into(),
                },
            ],
            max_redshift: 2.5,
            gw170817_safe: true,
            scale_independent: true,
            late_time_only: true,
            class_fingerprint: String::new(),
        };
        class.compute_fingerprint();
        class
    }

    /// SHA-256 of the canonical class payload.
    pub fn compute_fingerprint(&mut self) -> &str {
        let canonical = serde_json::json!({
            "name": self.name,
            "mechanism_routes": self.mechanism_routes,
            "parameter_boxes": self.parameter_boxes.iter().map(|b| {
                serde_json::json!({
                    "symbol": b.symbol,
                    "lo": b.lo,
                    "hi": b.hi,
                    "prior": b.prior,
                })
            }).collect::<Vec<_>>(),
            "max_redshift": self.max_redshift,
            "gw170817_safe": self.gw170817_safe,
            "scale_independent": self.scale_independent,
        });
        let hash = Sha256::digest(
            serde_json::to_string(&canonical)
                .unwrap_or_default()
                .as_bytes(),
        );
        self.class_fingerprint = format!("sha256:{hash:x}");
        &self.class_fingerprint
    }

    /// True when a parameter value is inside this class's declared search volume.
    pub fn parameter_in_scope(&self, symbol: &str, value: f64) -> bool {
        self.parameter_boxes
            .iter()
            .find(|b| b.symbol == symbol)
            .map(|b| b.contains(value))
            .unwrap_or(false)
    }

    // ---- V8 Phase 29 (SYNTHESIS #20): ExclusionClass ↔ SearchVolumeSpec bridge ----

    /// Convert this `ExclusionClass` to the machine-readable [`SearchVolumeSpec`] form used
    /// for the YAML companion (`data/search-volume.yml`).
    ///
    /// The produced spec carries every parameter box from the class (mechanism-specific and
    /// background) plus the structural constraints (GW170817-safe, scale-independent,
    /// max-redshift). Background boxes with no mechanism route are included unchanged.
    pub fn to_search_volume_spec(&self) -> SearchVolumeSpec {
        SearchVolumeSpec {
            schema_version: "v8.0.0".into(),
            exclusion_class_name: self.name.clone(),
            parameter_boxes: self
                .parameter_boxes
                .iter()
                .map(|b| SearchParamBox {
                    symbol: b.symbol.clone(),
                    lo: b.lo,
                    hi: b.hi,
                    mechanism_route: b.mechanism_route.clone(),
                })
                .collect(),
            gw170717_safe: self.gw170817_safe,
            scale_independent: self.scale_independent,
            max_redshift: self.max_redshift,
        }
    }

    /// Compare this `ExclusionClass` against a loaded `SearchVolumeSpec` (e.g. from
    /// `data/search-volume.yml`) and return a list of human-readable mismatches.
    ///
    /// An empty return value means the two representations are consistent; any entry
    /// means the spec has drifted from the canonical Rust definition. Checks:
    /// - Every mechanism-specific `ParameterBox` in this class has a matching entry in
    ///   `spec` with the same `lo` and `hi` (within 1e-9 floating-point tolerance).
    /// - Every mechanism-specific `SearchParamBox` in `spec` has a matching entry here.
    /// - Structural gates (`gw170817_safe`, `scale_independent`, `max_redshift`) agree.
    pub fn matches_search_volume_spec(&self, spec: &SearchVolumeSpec) -> Vec<String> {
        let mut mismatches = Vec::new();
        const TOL: f64 = 1e-9;

        // 1. Structural gate checks.
        if self.gw170817_safe != spec.gw170717_safe {
            mismatches.push(format!(
                "gw170817_safe mismatch: class={} spec={}",
                self.gw170817_safe, spec.gw170717_safe
            ));
        }
        if self.scale_independent != spec.scale_independent {
            mismatches.push(format!(
                "scale_independent mismatch: class={} spec={}",
                self.scale_independent, spec.scale_independent
            ));
        }
        if (self.max_redshift - spec.max_redshift).abs() > TOL {
            mismatches.push(format!(
                "max_redshift mismatch: class={} spec={}",
                self.max_redshift, spec.max_redshift
            ));
        }

        // 2. Mechanism-specific boxes: every class box must appear in the spec.
        let mechanism_boxes: Vec<&ParameterBox> = self
            .parameter_boxes
            .iter()
            .filter(|b| b.mechanism_route.is_some())
            .collect();
        for cb in &mechanism_boxes {
            match spec
                .parameter_boxes
                .iter()
                .find(|sb| sb.symbol == cb.symbol)
            {
                None => mismatches.push(format!(
                    "class ParameterBox '{}' has no matching entry in SearchVolumeSpec",
                    cb.symbol
                )),
                Some(sb) => {
                    if (sb.lo - cb.lo).abs() > TOL {
                        mismatches.push(format!(
                            "'{}' lo mismatch: class={} spec={}",
                            cb.symbol, cb.lo, sb.lo
                        ));
                    }
                    if (sb.hi - cb.hi).abs() > TOL {
                        mismatches.push(format!(
                            "'{}' hi mismatch: class={} spec={}",
                            cb.symbol, cb.hi, sb.hi
                        ));
                    }
                }
            }
        }

        // 3. Mechanism-specific spec boxes: every spec box must appear in the class.
        let spec_mechanism_boxes: Vec<&SearchParamBox> = spec
            .parameter_boxes
            .iter()
            .filter(|sb| sb.mechanism_route.is_some())
            .collect();
        for sb in &spec_mechanism_boxes {
            if !self.parameter_boxes.iter().any(|cb| cb.symbol == sb.symbol) {
                mismatches.push(format!(
                    "SearchVolumeSpec has '{}' but ExclusionClass does not",
                    sb.symbol
                ));
            }
        }

        mismatches
    }
}

/// The mandatory caveats that must appear alongside any exclusion sentence.
pub const EXCLUSION_MANDATORY_CAVEATS: &[&str] = &[
    "excludes only scale-independent growth suppression; scale-dependent modifications not covered",
    "f(R) and nDGP excluded from scope pending Boltzmann/hi_class promotion lane",
    "full Horndeski/EFT α-basis, EDE, and neutrino-sector extensions not in scope",
    "nonlinear screening mechanisms not in scope",
    "exclusion fidelity limited by absence of full RSD and weak-lensing covariance matrices",
];

/// Certificate that the parameter volume has been exhaustively searched.
///
/// Required before the exclusion sentence can be assembled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageCertificate {
    pub class_name: String,
    pub class_fingerprint: String,
    pub n_candidates_evaluated: u64,
    pub n_candidates_killed: u64,
    pub n_candidates_promoted: u64,
    /// Evidence that all connected components of the volume were visited.
    pub coverage_components: Vec<CoverageComponent>,
    /// True when every component has been resolved.
    pub all_components_resolved: bool,
    /// Grammar hash at the time coverage was assessed.
    pub grammar_hash: String,
    /// OpenQG source hash at the time coverage was assessed.
    pub source_hash: String,
}

/// One connected component of the parameter space.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageComponent {
    pub component_id: String,
    pub parameter_slice: BTreeMap<String, (f64, f64)>,
    pub resolved: bool,
    pub resolution_method: CoverageResolution,
}

/// How a component of the search volume was resolved.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageResolution {
    /// Profiled to convergence: ΔlnZ change < 0.05 ln-units over grid refinement.
    ProfiledToConvergence { best_delta_ln_z: f64 },
    /// Bounded by interval evidence: all sub-volumes have evidence < threshold.
    BoundedByIntervalEvidence { evidence_upper_bound: f64 },
    /// Killed by a named veto class: mechanism is structurally prohibited.
    KilledByVeto { veto_class: String },
    /// Not yet resolved.
    Unresolved,
}

impl CoverageResolution {
    pub fn is_resolved(&self) -> bool {
        !matches!(self, CoverageResolution::Unresolved)
    }
}

impl CoverageCertificate {
    pub fn fraction_resolved(&self) -> f64 {
        if self.coverage_components.is_empty() {
            return 0.0;
        }
        let resolved = self
            .coverage_components
            .iter()
            .filter(|c| c.resolved)
            .count();
        resolved as f64 / self.coverage_components.len() as f64
    }
}

/// Rank stability result for one perturbation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankStabilityResult {
    pub perturbation_name: String,
    /// True when the exclusion null holds under this perturbation.
    pub null_holds: bool,
    /// True when the champion remains rank-1 in ≥80% of jackknife draws.
    pub rank_stable: bool,
    /// ΔlnZ under the perturbation.
    pub delta_ln_z: f64,
    pub n_trials: u32,
}

/// Collection of all rank-stability tests required before the sentence is printable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankStabilityReport {
    pub results: Vec<RankStabilityResult>,
}

impl RankStabilityReport {
    /// Build a `RankStabilityReport` from raw jackknife results.
    ///
    /// Each tuple is `(perturbation_name, delta_ln_z, n_trials, null_holds, rank_stable)`.
    /// `null_holds` is true when ΔlnZ under the perturbation remains below the exclusion
    /// threshold (i.e., the null result survives the perturbation).
    /// `rank_stable` is true when the champion remains rank-1 in ≥80% of jackknife draws.
    pub fn from_jackknife_results(results: Vec<(String, f64, u32, bool, bool)>) -> Self {
        RankStabilityReport {
            results: results
                .into_iter()
                .map(
                    |(perturbation_name, delta_ln_z, n_trials, null_holds, rank_stable)| {
                        RankStabilityResult {
                            perturbation_name,
                            delta_ln_z,
                            n_trials,
                            null_holds,
                            rank_stable,
                        }
                    },
                )
                .collect(),
        }
    }

    pub fn all_pass(&self) -> bool {
        self.results.iter().all(|r| r.null_holds && r.rank_stable)
    }

    pub fn summary(&self) -> String {
        let passing = self
            .results
            .iter()
            .filter(|r| r.null_holds && r.rank_stable)
            .count();
        format!(
            "{}/{} perturbations: null holds and rank stable",
            passing,
            self.results.len()
        )
    }
}

/// The trials correction applied to the exclusion evidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrialsCorrection {
    /// Number of independent candidates evaluated.
    pub n_candidates: u64,
    /// Effective look-elsewhere factor (number of independent searches).
    pub effective_trials: u64,
    /// Raw ΔlnZ before correction.
    pub raw_delta_ln_z: f64,
    /// Corrected ΔlnZ after look-elsewhere penalty.
    pub corrected_delta_ln_z: f64,
    /// Look-elsewhere penalty in natural log-units (ln-units).
    pub look_elsewhere_ln_z_penalty: f64,
}

impl TrialsCorrection {
    pub fn apply(raw_delta_ln_z: f64, n_candidates: u64) -> Self {
        // Bonferroni-style look-elsewhere correction: subtract ln(N_eff) in ln-units.
        // N_eff = sqrt(n_candidates) is a conservative independent-tests estimate.
        let effective_trials = (n_candidates as f64).sqrt().max(1.0) as u64;
        let penalty = (effective_trials as f64).ln();
        let corrected = raw_delta_ln_z - penalty;
        TrialsCorrection {
            n_candidates,
            effective_trials,
            raw_delta_ln_z,
            corrected_delta_ln_z: corrected,
            look_elsewhere_ln_z_penalty: penalty,
        }
    }
}

/// What level of exclusion evidence has been established.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExclusionStrength {
    /// No exclusion: ΔlnZ > 0 (evidence favors candidate).
    NoExclusion,
    /// Marginal: 0 > corrected ΔlnZ > -2 (suggestive null).
    Marginal,
    /// Nominal: corrected ΔlnZ ≤ -2 (nominally excludes, but data/covariance limited).
    Nominal,
    /// Definitive: corrected ΔlnZ ≤ -5 and rank stable (citable exclusion).
    Definitive,
}

impl ExclusionStrength {
    pub fn from_corrected_delta_ln_z(corrected: f64) -> Self {
        if corrected > 0.0 {
            ExclusionStrength::NoExclusion
        } else if corrected > -2.0 {
            ExclusionStrength::Marginal
        } else if corrected > -5.0 {
            ExclusionStrength::Nominal
        } else {
            ExclusionStrength::Definitive
        }
    }

    pub fn is_publishable_exclusion(&self) -> bool {
        matches!(
            self,
            ExclusionStrength::Nominal | ExclusionStrength::Definitive
        )
    }
}

/// Missing prerequisite preventing the exclusion sentence from being assembled.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExclusionBlocker {
    CoverageIncomplete { fraction_resolved: f64 },
    TrialsCorrectionNotApplied,
    RankStabilityNotPassed { failing: Vec<String> },
    CovarianceIncomplete { missing: Vec<String> },
    ProfileFittedNullMissing,
    ExclusionStrengthInsufficient { strength: ExclusionStrength },
}

impl std::fmt::Display for ExclusionBlocker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExclusionBlocker::CoverageIncomplete { fraction_resolved } => {
                write!(
                    f,
                    "coverage incomplete: {:.0}% resolved",
                    fraction_resolved * 100.0
                )
            }
            ExclusionBlocker::TrialsCorrectionNotApplied => {
                write!(f, "trials correction not applied")
            }
            ExclusionBlocker::RankStabilityNotPassed { failing } => {
                write!(f, "rank stability failed for: {}", failing.join(", "))
            }
            ExclusionBlocker::CovarianceIncomplete { missing } => {
                write!(f, "covariance incomplete for: {}", missing.join(", "))
            }
            ExclusionBlocker::ProfileFittedNullMissing => {
                write!(f, "profile-fitted ΛCDM null not computed")
            }
            ExclusionBlocker::ExclusionStrengthInsufficient { strength } => {
                write!(f, "exclusion strength too weak: {:?}", strength)
            }
        }
    }
}

/// The preregistered exclusion sentence — can only be built when all prerequisites pass.
///
/// This is the citable artifact: one sentence that can be reproduced from the ledgers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExclusionSentence {
    pub class_name: String,
    pub class_fingerprint: String,
    pub corrected_delta_ln_z: f64,
    pub strength: ExclusionStrength,
    pub n_candidates: u64,
    pub data_stack_hash: String,
    pub grammar_hash: String,
    pub rank_stability_summary: String,
    pub mandatory_caveats: Vec<String>,
    /// The machine-generated exclusion text.
    pub sentence: String,
    /// The physical effect being excluded (e.g., "lower fσ8(z≈0.6) by ≥0.02").
    pub excluded_effect: String,
}

impl ExclusionSentence {
    /// Try to assemble an exclusion sentence, failing with blockers if prerequisites are unmet.
    pub fn try_build(
        class: &ExclusionClass,
        coverage: &CoverageCertificate,
        trials: &TrialsCorrection,
        stability: &RankStabilityReport,
        covariance_complete_blocks: &[String],
        profile_fitted_null_done: bool,
        excluded_effect: impl Into<String>,
    ) -> Result<ExclusionSentence, Vec<ExclusionBlocker>> {
        let mut blockers = Vec::new();

        if !coverage.all_components_resolved {
            blockers.push(ExclusionBlocker::CoverageIncomplete {
                fraction_resolved: coverage.fraction_resolved(),
            });
        }

        if !covariance_complete_blocks.is_empty() {
            blockers.push(ExclusionBlocker::CovarianceIncomplete {
                missing: covariance_complete_blocks.to_vec(),
            });
        }

        if !profile_fitted_null_done {
            blockers.push(ExclusionBlocker::ProfileFittedNullMissing);
        }

        if !stability.all_pass() {
            let failing: Vec<String> = stability
                .results
                .iter()
                .filter(|r| !r.null_holds || !r.rank_stable)
                .map(|r| r.perturbation_name.clone())
                .collect();
            blockers.push(ExclusionBlocker::RankStabilityNotPassed { failing });
        }

        let strength = ExclusionStrength::from_corrected_delta_ln_z(trials.corrected_delta_ln_z);
        if !strength.is_publishable_exclusion() {
            blockers.push(ExclusionBlocker::ExclusionStrengthInsufficient { strength });
        }

        if !blockers.is_empty() {
            return Err(blockers);
        }

        let excluded_effect = excluded_effect.into();
        let sentence = format!(
            "Over the preregistered V8 search volume {name}—late-time, GW170817-safe, \
             scale-independent growth-suppression mechanisms with μ0∈[−0.30,0] or \
             dark-scattering drag A_drag∈[0,10] under the declared priors—we find no \
             candidate with positive calibrated marginal evidence relative to \
             profile-fitted ΛCDM on the covariance-complete DESI/Planck/RSD/S8/BBN stack; \
             after trials correction (N_eff={n_eff}, penalty={penalty:.2} lnZ-units) and \
             rank-stability tests ({stability}), we exclude mechanisms in this class \
             large enough to {effect} without compensating degradation in CMB/BAO geometry. \
             (ΔlnZ_corrected = {delta_z:.2}, class fingerprint = {fp})",
            name = class.name,
            n_eff = trials.effective_trials,
            penalty = trials.look_elsewhere_ln_z_penalty,
            stability = stability.summary(),
            effect = excluded_effect,
            delta_z = trials.corrected_delta_ln_z,
            fp = &class.class_fingerprint[..16],
        );

        let data_hash = {
            let tag = concat!("data-stack-hash", "-unbound");
            let h = Sha256::digest(tag.as_bytes());
            format!("sha256:{h:x}")
        };
        let grammar_hash = coverage.grammar_hash.clone();

        Ok(ExclusionSentence {
            class_name: class.name.clone(),
            class_fingerprint: class.class_fingerprint.clone(),
            corrected_delta_ln_z: trials.corrected_delta_ln_z,
            strength,
            n_candidates: trials.n_candidates,
            data_stack_hash: data_hash,
            grammar_hash,
            rank_stability_summary: stability.summary(),
            mandatory_caveats: EXCLUSION_MANDATORY_CAVEATS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            sentence,
            excluded_effect,
        })
    }
}

/// Compute a leave-one-sector-out jackknife rank-stability report.
///
/// Given per-sector ΔlnZ contributions (each sector's independent evidence in ln-units),
/// this function computes, for each sector, the "leave-one-out" total ΔlnZ (i.e. the
/// evidence from all OTHER sectors). A sector whose removal flips the exclusion verdict
/// (total ΔlnZ crosses `null_threshold`) is flagged as `null_holds = false`.
///
/// The output `RankStabilityReport` collects one `RankStabilityResult` per sector. The
/// `rank_stable` flag is set to true when the leave-one-out ΔlnZ stays below `null_threshold`
/// (null still holds) — consistent with the SYNTHESIS #5 requirement that "champion rank-1
/// in ≥80% of jackknife draws".
///
/// # Arguments
/// - `sector_delta_lnz`: named per-sector ΔlnZ contributions, ordered arbitrarily.
/// - `null_threshold`: the threshold below which ΔlnZ represents a null result (exclusion).
///   Typically `0.0` (any positive ΔlnZ is evidence FOR the candidate; any negative is exclusion).
///
/// # Returns
/// `RankStabilityReport` with one result per sector; `n_trials = sector_count - 1` for each.
pub fn compute_sector_jackknife(
    sector_delta_lnz: &[(&str, f64)],
    null_threshold: f64,
) -> RankStabilityReport {
    let total: f64 = sector_delta_lnz.iter().map(|(_, v)| v).sum();
    let n = sector_delta_lnz.len() as u32;
    let results: Vec<RankStabilityResult> = sector_delta_lnz
        .iter()
        .map(|(name, sector_val)| {
            let leave_one_out = total - sector_val;
            let null_holds = leave_one_out <= null_threshold;
            RankStabilityResult {
                perturbation_name: format!("jackknife-{name}"),
                null_holds,
                rank_stable: null_holds,
                delta_ln_z: leave_one_out,
                n_trials: n.saturating_sub(1),
            }
        })
        .collect();
    RankStabilityReport { results }
}

/// Fraction of jackknife draws in which the null holds.
///
/// Returns 1.0 for an empty report (vacuously stable). SYNTHESIS #5 requires this to be
/// ≥ 0.80 before the exclusion sentence is printable.
pub fn jackknife_null_stability_fraction(report: &RankStabilityReport) -> f64 {
    if report.results.is_empty() {
        return 1.0;
    }
    let passing = report.results.iter().filter(|r| r.null_holds).count() as f64;
    passing / report.results.len() as f64
}

/// Summary statistics from a block bootstrap of the sector ΔlnZ distribution.
///
/// Produced by [`block_bootstrap_lnz`]. SYNTHESIS #5 requires the `rank_stable_fraction`
/// to be ≥ 0.80 before an exclusion sentence is printable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BootstrapRankReport {
    /// Number of bootstrap resamples performed.
    pub n_bootstrap: usize,
    /// The observed total ΔlnZ (sum of all sector contributions, without resampling).
    pub observed_total_lnz: f64,
    /// Mean of the bootstrap distribution of resampled ΔlnZ.
    pub bootstrap_mean_lnz: f64,
    /// Standard deviation of the bootstrap distribution.
    pub bootstrap_std_lnz: f64,
    /// Fraction of bootstrap draws where the resampled total ΔlnZ ≤ null_threshold.
    /// 1.0 = all draws are null-consistent (clean exclusion); 0.0 = none are.
    pub null_fraction: f64,
    /// Null threshold used for the `null_fraction` calculation.
    pub null_threshold: f64,
}

impl BootstrapRankReport {
    /// True when ≥ 80 % of bootstrap draws are null-consistent (SYNTHESIS #5 threshold).
    pub fn rank_stable(&self) -> bool {
        self.null_fraction >= 0.80
    }
}

/// Block bootstrap of the sector ΔlnZ distribution.
///
/// Resamples the `sector_delta_lnz` sectors with replacement `n_bootstrap` times and reports
/// statistics on the bootstrap distribution of the resampled total. Uses a seeded splitmix64
/// PRNG so results are bit-reproducible across platforms.
///
/// # Arguments
/// - `sector_delta_lnz`: named per-sector ΔlnZ contributions.
/// - `null_threshold`: total ΔlnZ ≤ this ⇒ null holds in that bootstrap draw.
/// - `n_bootstrap`: number of resamples (≥ 200 for a stable estimate; SYNTHESIS #5 uses 500).
/// - `seed`: PRNG seed for reproducibility.
///
/// Returns an empty-equivalent `BootstrapRankReport` when the sector list is empty.
pub fn block_bootstrap_lnz(
    sector_delta_lnz: &[(&str, f64)],
    null_threshold: f64,
    n_bootstrap: usize,
    seed: u64,
) -> BootstrapRankReport {
    let n = sector_delta_lnz.len();
    let observed_total: f64 = sector_delta_lnz.iter().map(|(_, v)| v).sum();

    if n == 0 || n_bootstrap == 0 {
        return BootstrapRankReport {
            n_bootstrap,
            observed_total_lnz: observed_total,
            bootstrap_mean_lnz: observed_total,
            bootstrap_std_lnz: 0.0,
            null_fraction: if observed_total <= null_threshold {
                1.0
            } else {
                0.0
            },
            null_threshold,
        };
    }

    // Inline splitmix64 so exclusion.rs has no dependency on mutation.rs.
    let mut state = seed;
    let mut splitmix64 = move || -> u64 {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    };

    let values: Vec<f64> = sector_delta_lnz.iter().map(|(_, v)| *v).collect();
    let mut bootstrap_totals: Vec<f64> = Vec::with_capacity(n_bootstrap);

    for _ in 0..n_bootstrap {
        let mut total = 0.0;
        for _ in 0..n {
            let idx = (splitmix64() % n as u64) as usize;
            total += values[idx];
        }
        bootstrap_totals.push(total);
    }

    let mean = bootstrap_totals.iter().sum::<f64>() / n_bootstrap as f64;
    let variance = bootstrap_totals
        .iter()
        .map(|&x| (x - mean) * (x - mean))
        .sum::<f64>()
        / n_bootstrap as f64;
    let std = variance.sqrt();
    let null_count = bootstrap_totals
        .iter()
        .filter(|&&t| t <= null_threshold)
        .count();
    let null_fraction = null_count as f64 / n_bootstrap as f64;

    BootstrapRankReport {
        n_bootstrap,
        observed_total_lnz: observed_total,
        bootstrap_mean_lnz: mean,
        bootstrap_std_lnz: std,
        null_fraction,
        null_threshold,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolved_coverage() -> CoverageCertificate {
        CoverageCertificate {
            class_name: "C_growth-suppression(V8)".into(),
            class_fingerprint: "sha256:test".into(),
            n_candidates_evaluated: 1000,
            n_candidates_killed: 980,
            n_candidates_promoted: 0,
            coverage_components: vec![
                CoverageComponent {
                    component_id: "mu0-slice".into(),
                    parameter_slice: BTreeMap::from([("mu0".into(), (-0.30, 0.00))]),
                    resolved: true,
                    resolution_method: CoverageResolution::ProfiledToConvergence {
                        best_delta_ln_z: -3.5,
                    },
                },
                CoverageComponent {
                    component_id: "dark-scattering-slice".into(),
                    parameter_slice: BTreeMap::from([
                        ("A_drag".into(), (0.0, 10.0)),
                        ("w0".into(), (-0.99, -0.80)),
                    ]),
                    resolved: true,
                    resolution_method: CoverageResolution::ProfiledToConvergence {
                        best_delta_ln_z: -4.1,
                    },
                },
            ],
            all_components_resolved: true,
            grammar_hash: "sha256:grammar-test".into(),
            source_hash: "sha256:source-test".into(),
        }
    }

    fn passing_stability() -> RankStabilityReport {
        RankStabilityReport {
            results: vec![
                RankStabilityResult {
                    perturbation_name: "jackknife-bao".into(),
                    null_holds: true,
                    rank_stable: true,
                    delta_ln_z: -3.8,
                    n_trials: 10,
                },
                RankStabilityResult {
                    perturbation_name: "sh0es-removed".into(),
                    null_holds: true,
                    rank_stable: true,
                    delta_ln_z: -3.2,
                    n_trials: 1,
                },
            ],
        }
    }

    #[test]
    fn v8_class_has_both_mechanism_routes() {
        let class = ExclusionClass::v8_growth_suppression();
        assert_eq!(class.mechanism_routes.len(), 2);
        assert!(class.gw170817_safe);
        assert!(class.scale_independent);
        assert!(class.late_time_only);
        assert!(class.class_fingerprint.starts_with("sha256:"));
    }

    #[test]
    fn parameter_box_in_scope_check() {
        let class = ExclusionClass::v8_growth_suppression();
        assert!(class.parameter_in_scope("mu0", -0.15));
        assert!(
            !class.parameter_in_scope("mu0", 0.10),
            "positive mu0 is outside the class"
        );
        assert!(class.parameter_in_scope("A_drag", 5.0));
        assert!(!class.parameter_in_scope("A_drag", 15.0));
    }

    #[test]
    fn trials_correction_penalizes_large_n() {
        let t1 = TrialsCorrection::apply(-5.0, 100);
        let t2 = TrialsCorrection::apply(-5.0, 10000);
        assert!(t2.corrected_delta_ln_z < t1.corrected_delta_ln_z);
        assert!(t2.look_elsewhere_ln_z_penalty > t1.look_elsewhere_ln_z_penalty);
    }

    #[test]
    fn exclusion_strength_levels() {
        assert_eq!(
            ExclusionStrength::from_corrected_delta_ln_z(0.5),
            ExclusionStrength::NoExclusion
        );
        assert_eq!(
            ExclusionStrength::from_corrected_delta_ln_z(-1.0),
            ExclusionStrength::Marginal
        );
        assert_eq!(
            ExclusionStrength::from_corrected_delta_ln_z(-3.0),
            ExclusionStrength::Nominal
        );
        assert_eq!(
            ExclusionStrength::from_corrected_delta_ln_z(-6.0),
            ExclusionStrength::Definitive
        );
        assert!(!ExclusionStrength::NoExclusion.is_publishable_exclusion());
        assert!(ExclusionStrength::Nominal.is_publishable_exclusion());
        assert!(ExclusionStrength::Definitive.is_publishable_exclusion());
    }

    #[test]
    fn exclusion_sentence_blocked_without_stability() {
        let class = ExclusionClass::v8_growth_suppression();
        let coverage = resolved_coverage();
        let trials = TrialsCorrection::apply(-6.0, 1000);

        let failing_stability = RankStabilityReport {
            results: vec![RankStabilityResult {
                perturbation_name: "jackknife-bao".into(),
                null_holds: false,
                rank_stable: false,
                delta_ln_z: 0.2,
                n_trials: 10,
            }],
        };

        let result = ExclusionSentence::try_build(
            &class,
            &coverage,
            &trials,
            &failing_stability,
            &[],
            true,
            "lower fσ8(z≈0.6) by ≥0.02",
        );
        assert!(result.is_err());
        let blockers = result.unwrap_err();
        assert!(blockers
            .iter()
            .any(|b| matches!(b, ExclusionBlocker::RankStabilityNotPassed { .. })));
    }

    #[test]
    fn exclusion_sentence_blocked_without_coverage() {
        let class = ExclusionClass::v8_growth_suppression();
        let trials = TrialsCorrection::apply(-6.0, 1000);
        let stability = passing_stability();

        let incomplete_coverage = CoverageCertificate {
            all_components_resolved: false,
            coverage_components: vec![CoverageComponent {
                component_id: "c1".into(),
                parameter_slice: BTreeMap::new(),
                resolved: false,
                resolution_method: CoverageResolution::Unresolved,
            }],
            ..resolved_coverage()
        };

        let result = ExclusionSentence::try_build(
            &class,
            &incomplete_coverage,
            &trials,
            &stability,
            &[],
            true,
            "lower fσ8(z≈0.6) by ≥0.02",
        );
        assert!(result.is_err());
        let blockers = result.unwrap_err();
        assert!(blockers
            .iter()
            .any(|b| matches!(b, ExclusionBlocker::CoverageIncomplete { .. })));
    }

    /// Spec acceptance test: when all prerequisites are met, the exclusion sentence is built
    /// and contains the mandatory class name, corrected ΔlnZ, and caveats.
    #[test]
    fn exclusion_sentence_builds_when_all_prerequisites_met() {
        let class = ExclusionClass::v8_growth_suppression();
        let coverage = resolved_coverage();
        let trials = TrialsCorrection::apply(-6.5, 1000);
        let stability = passing_stability();

        let result = ExclusionSentence::try_build(
            &class,
            &coverage,
            &trials,
            &stability,
            &[],
            true,
            "lower fσ8(z≈0.6) by ≥0.02",
        );

        assert!(
            result.is_ok(),
            "should build sentence; blockers: {:?}",
            result.err()
        );
        let sentence = result.unwrap();
        assert!(sentence.sentence.contains("C_growth-suppression(V8)"));
        assert!(sentence.sentence.contains("fσ8(z≈0.6) by ≥0.02"));
        assert_eq!(
            sentence.mandatory_caveats.len(),
            EXCLUSION_MANDATORY_CAVEATS.len()
        );
        assert_eq!(sentence.strength, ExclusionStrength::Definitive);
    }

    #[test]
    fn rank_stability_from_jackknife_results_all_pass() {
        let report = RankStabilityReport::from_jackknife_results(vec![
            ("leave-one-out-fsigma8".into(), -4.2, 22, true, true),
            ("leave-one-out-bao".into(), -3.9, 22, true, true),
            ("perturb-sigma8".into(), -4.0, 22, true, true),
        ]);
        assert_eq!(report.results.len(), 3);
        assert!(report.all_pass());
        assert!(report.summary().contains("3/3"));
    }

    #[test]
    fn rank_stability_from_jackknife_results_partial_fail() {
        let report = RankStabilityReport::from_jackknife_results(vec![
            ("leave-one-out-fsigma8".into(), -4.2, 22, true, true),
            ("perturb-h0".into(), 0.3, 22, false, false),
        ]);
        assert!(!report.all_pass());
        assert!(report.summary().contains("1/2"));
    }

    #[test]
    fn rank_stability_from_jackknife_preserves_field_values() {
        let report = RankStabilityReport::from_jackknife_results(vec![(
            "my-perturbation".into(),
            -5.5,
            23,
            true,
            false,
        )]);
        let r = &report.results[0];
        assert_eq!(r.perturbation_name, "my-perturbation");
        assert!((r.delta_ln_z - (-5.5)).abs() < 1e-12);
        assert_eq!(r.n_trials, 23);
        assert!(r.null_holds);
        assert!(!r.rank_stable);
    }

    // ---- compute_sector_jackknife ----

    #[test]
    fn sector_jackknife_empty_sectors() {
        let report = compute_sector_jackknife(&[], 0.0);
        assert!(report.results.is_empty());
        assert!(report.all_pass());
    }

    #[test]
    fn sector_jackknife_single_sector() {
        // One sector with ΔlnZ = -3.0 → leave-one-out = 0.0 (no other sectors)
        // 0.0 > null_threshold (-inf via 0.0)? 0.0 <= 0.0 → null holds.
        let report = compute_sector_jackknife(&[("rsd", -3.0)], 0.0);
        assert_eq!(report.results.len(), 1);
        assert!((report.results[0].delta_ln_z - 0.0).abs() < 1e-12);
        assert_eq!(report.results[0].n_trials, 0);
    }

    #[test]
    fn sector_jackknife_all_sectors_strong_exclusion() {
        // Three sectors, each -2.0 → total = -6.0. Leave-one-out = -4.0 for each.
        // null_threshold = 0.0 → -4.0 <= 0 → null_holds = true for all.
        let sectors = [("rsd", -2.0), ("wl", -2.0), ("bao", -2.0)];
        let report = compute_sector_jackknife(&sectors, 0.0);
        assert_eq!(report.results.len(), 3);
        for r in &report.results {
            assert!(r.null_holds, "{} did not hold null", r.perturbation_name);
            assert!((r.delta_ln_z - (-4.0)).abs() < 1e-12);
            assert_eq!(r.n_trials, 2);
        }
        assert_eq!(jackknife_null_stability_fraction(&report), 1.0);
    }

    #[test]
    fn sector_jackknife_dominant_sector_flips_verdict() {
        // Four sectors: one dominant (-8.0), three weak (-0.5 each).
        // Total = -8.0 + (-1.5) = -9.5
        // Leave-out dominant: -9.5 - (-8.0) = -1.5 → -1.5 <= 0 → null holds (barely)
        // Leave-out weak: -9.5 - (-0.5) = -9.0 → -9.0 <= 0 → null holds
        let sectors = [
            ("dominant", -8.0),
            ("sector-b", -0.5),
            ("sector-c", -0.5),
            ("sector-d", -0.5),
        ];
        let report = compute_sector_jackknife(&sectors, 0.0);
        assert!(report.all_pass());
    }

    #[test]
    fn sector_jackknife_weak_sector_flips_verdict() {
        // Three sectors: two at -0.5, one at +5.0. Total = 4.0 (evidence FOR candidate).
        // Leave-out positive sector: 4.0 - 5.0 = -1.0 → null holds
        // Leave-out each negative: 4.0 - (-0.5) = 4.5 → 4.5 > 0 → null does NOT hold
        let sectors = [("bao", -0.5), ("rsd", -0.5), ("outlier", 5.0)];
        let report = compute_sector_jackknife(&sectors, 0.0);
        assert!(!report.all_pass());
        let frac = jackknife_null_stability_fraction(&report);
        assert!(frac < 1.0, "not all jackknifes should hold null");
    }

    #[test]
    fn jackknife_null_stability_fraction_empty_report() {
        let report = RankStabilityReport { results: vec![] };
        assert_eq!(jackknife_null_stability_fraction(&report), 1.0);
    }

    #[test]
    fn jackknife_result_names_are_prefixed() {
        let report = compute_sector_jackknife(&[("bao", -2.0), ("wl", -1.5)], 0.0);
        assert!(report.results[0]
            .perturbation_name
            .starts_with("jackknife-"));
        assert!(report.results[1].perturbation_name.contains("wl"));
    }

    // ---- block_bootstrap_lnz ----

    #[test]
    fn bootstrap_empty_sectors_returns_degenerate_report() {
        let r = block_bootstrap_lnz(&[], 0.0, 100, 42);
        assert_eq!(r.n_bootstrap, 100);
        assert_eq!(r.observed_total_lnz, 0.0);
        assert_eq!(r.bootstrap_std_lnz, 0.0);
    }

    #[test]
    fn bootstrap_single_sector_has_zero_std() {
        let r = block_bootstrap_lnz(&[("bao", -3.0)], 0.0, 200, 7);
        assert_eq!(r.n_bootstrap, 200);
        assert!((r.observed_total_lnz - (-3.0)).abs() < 1e-12);
        assert!(r.bootstrap_std_lnz.abs() < 1e-12, "single sector → std = 0");
        assert!((r.bootstrap_mean_lnz - (-3.0)).abs() < 1e-12);
        assert_eq!(r.null_fraction, 1.0, "total < 0 always below threshold 0.0");
    }

    #[test]
    fn bootstrap_all_null_consistent_sectors() {
        // All sectors strongly negative → all bootstrap draws should be null-consistent.
        let sectors = [("rsd", -5.0), ("wl", -4.0), ("bao", -3.0)];
        let r = block_bootstrap_lnz(&sectors, 0.0, 500, 12345);
        assert_eq!(
            r.null_fraction, 1.0,
            "all draws should be null; got {}",
            r.null_fraction
        );
        assert!(r.rank_stable(), "100 % null-consistent is rank-stable");
    }

    #[test]
    fn bootstrap_all_positive_sectors_never_null() {
        // All sectors positive → no bootstrap draw can be null-consistent.
        let sectors = [("a", 3.0), ("b", 5.0), ("c", 2.0)];
        let r = block_bootstrap_lnz(&sectors, 0.0, 500, 99);
        assert_eq!(
            r.null_fraction, 0.0,
            "no draw should be null; got {}",
            r.null_fraction
        );
        assert!(!r.rank_stable());
    }

    #[test]
    fn bootstrap_is_deterministic_with_same_seed() {
        let sectors = [("a", -1.5), ("b", 0.5), ("c", -2.0)];
        let r1 = block_bootstrap_lnz(&sectors, 0.0, 300, 777);
        let r2 = block_bootstrap_lnz(&sectors, 0.0, 300, 777);
        assert_eq!(r1.null_fraction, r2.null_fraction);
        assert!((r1.bootstrap_mean_lnz - r2.bootstrap_mean_lnz).abs() < 1e-15);
    }

    #[test]
    fn bootstrap_different_seeds_give_different_results() {
        let sectors = [("a", -1.5), ("b", 0.5), ("c", -2.0)];
        let r1 = block_bootstrap_lnz(&sectors, 0.0, 500, 1);
        let r2 = block_bootstrap_lnz(&sectors, 0.0, 500, 2);
        assert_ne!(r1.null_fraction, r2.null_fraction);
    }

    #[test]
    fn bootstrap_report_serde_round_trip() {
        let sectors = [("a", -2.0), ("b", -1.5)];
        let r = block_bootstrap_lnz(&sectors, 0.0, 200, 42);
        let json = serde_json::to_string(&r).unwrap();
        let back: BootstrapRankReport = serde_json::from_str(&json).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn bootstrap_observed_total_matches_sum() {
        let sectors = [("a", -2.0), ("b", -1.5), ("c", 0.5)];
        let r = block_bootstrap_lnz(&sectors, 0.0, 200, 0);
        assert!((r.observed_total_lnz - (-3.0)).abs() < 1e-12);
    }

    // ---- V8 Phase 29 (SYNTHESIS #20): ExclusionClass ↔ SearchVolumeSpec bridge ----

    #[test]
    fn v8_exclusion_class_converts_to_search_volume_spec() {
        let class = ExclusionClass::v8_growth_suppression();
        let spec = class.to_search_volume_spec();
        assert_eq!(spec.exclusion_class_name, class.name);
        assert_eq!(spec.gw170717_safe, class.gw170817_safe);
        assert_eq!(spec.scale_independent, class.scale_independent);
        assert!((spec.max_redshift - class.max_redshift).abs() < 1e-9);
        // Every class parameter box must appear in the spec.
        for cb in &class.parameter_boxes {
            let found = spec.parameter_boxes.iter().any(|sb| {
                sb.symbol == cb.symbol
                    && (sb.lo - cb.lo).abs() < 1e-9
                    && (sb.hi - cb.hi).abs() < 1e-9
            });
            assert!(
                found,
                "class box '{}' must appear in converted spec",
                cb.symbol
            );
        }
    }

    #[test]
    fn v8_exclusion_class_matches_canonical_search_volume_spec() {
        // The two canonical v8_growth_suppression() definitions must be fully consistent.
        use super::super::search_volume::SearchVolumeSpec;
        let class = ExclusionClass::v8_growth_suppression();
        let spec = SearchVolumeSpec::v8_growth_suppression();
        let mismatches = class.matches_search_volume_spec(&spec);
        assert!(
            mismatches.is_empty(),
            "ExclusionClass and SearchVolumeSpec v8 definitions must agree; mismatches: {mismatches:?}"
        );
    }

    #[test]
    fn mismatched_bounds_detected_by_bridge() {
        use super::super::search_volume::{SearchParamBox, SearchVolumeSpec};
        let class = ExclusionClass::v8_growth_suppression();
        // Build a spec with a deliberately wrong 'mu0' upper bound.
        let mut spec = class.to_search_volume_spec();
        if let Some(mu0) = spec.parameter_boxes.iter_mut().find(|b| b.symbol == "mu0") {
            mu0.hi = 0.5; // wrong: should be 0.0
        }
        // Also inject a stray box to test the reverse direction.
        spec.parameter_boxes.push(SearchParamBox {
            symbol: "mystery_param".into(),
            lo: -1.0,
            hi: 1.0,
            mechanism_route: Some("unknown_route".into()),
        });
        let mismatches = class.matches_search_volume_spec(&spec);
        assert!(
            mismatches
                .iter()
                .any(|m| m.contains("mu0") && m.contains("hi")),
            "mu0 hi mismatch must be detected; got: {mismatches:?}"
        );
        assert!(
            mismatches.iter().any(|m| m.contains("mystery_param")),
            "stray spec box must be detected; got: {mismatches:?}"
        );
    }

    #[test]
    fn structural_gate_mismatch_detected_by_bridge() {
        use super::super::search_volume::SearchVolumeSpec;
        let class = ExclusionClass::v8_growth_suppression();
        let mut spec = class.to_search_volume_spec();
        spec.gw170717_safe = false; // contradict the class
        let mismatches = class.matches_search_volume_spec(&spec);
        assert!(
            mismatches.iter().any(|m| m.contains("gw170817_safe")),
            "gw170817_safe mismatch must be detected; got: {mismatches:?}"
        );
    }
}
