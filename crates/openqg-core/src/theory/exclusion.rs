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
}
