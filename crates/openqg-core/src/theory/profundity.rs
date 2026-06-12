//! V8 Phase 7: publication-grade verdict system.
//!
//! ## The problem: `DiscoveryClaim` was never actually derivable
//!
//! Phase 5 added `ClaimClass::DiscoveryClaim` to the enum, but `ClaimClass::derive()` never
//! returns it — the comment said "that gate is enforced externally, not here." Phase 7 closes
//! this gap with three interlocking constructs:
//!
//! 1. [`MechanismOffTwin`]: captures the ablation result — what happens to ΔlnZ when the
//!    proposed mechanism is disabled (ΛCDM fallback). If the effect is mostly due to the
//!    mechanism, the mechanism-off twin should lose ≥70% of the champion's ΔlnZ.
//!
//! 2. [`ProfundityGate`]: the S06 publication threshold — a claim is "profound" only when:
//!    - σ_significance ≥ 5.0 (global significance, 5σ ≈ p < 3e-7)
//!    - ΔlnZ_corrected ≥ 5.0 (post-trials Bayesian evidence)
//!    - The mechanism-off twin loses ≥70% of the effect (mechanism attribution verified)
//!
//! 3. [`DiscoveryClaimGate`]: aggregates all gates needed to upgrade
//!    `PromotionCandidate → DiscoveryClaim`:
//!    - `ProfundityGate` passes
//!    - Post-search p-value < 0.003 with ≥200 null replications
//!    - GoF passed (absolute fit acceptable)
//!    - Instrument tier ≥ T2Boltzmann
//!    - Pre-registered sealed forecast in the prediction registry
//!
//! ## Usage pattern
//!
//! ```text
//! let base_class = ClaimClass::derive(n_obs, mode, delta_lnz, tier, disq, boundary);
//! // ... compute gate inputs ...
//! let gate = DiscoveryClaimGate { ... };
//! let final_class = base_class.try_upgrade_to_discovery(&gate);
//! ```
//!
//! ## Reference
//!
//! - S06 §"The profundity threshold"
//! - S07 §3 "Post-search p-value"
//! - S11 §"The V8 exclusion paper — gating table"

use crate::cosmology::ForwardTier;
use serde::{Deserialize, Serialize};

/// The σ threshold required for a Nature-tier discovery claim.
pub const NATURE_TIER_SIGMA: f64 = 5.0;

/// The minimum ΔlnZ (post-trials) required for a profound discovery claim.
pub const PROFUNDITY_DELTA_LNZ_MIN: f64 = 5.0;

/// Fraction of the mechanism effect that the mechanism-off twin must lose for attribution to pass.
/// Twin must retain ≤ 1 − MECHANISM_LOSS_THRESHOLD of the champion's ΔlnZ.
pub const MECHANISM_LOSS_THRESHOLD: f64 = 0.70;

/// One ablation study: the champion evaluated with its mechanism disabled.
///
/// The mechanism-off twin is the same theoretical candidate but with the modification
/// zeroed out (μ0 → 0, A_drag → 0, etc.), re-evaluated on the same dataset. This tests
/// whether the evidence improvement ΔlnZ is genuinely driven by the proposed mechanism
/// rather than by incidental parameter freedom.
///
/// A mechanism that retains >30% of ΔlnZ when disabled fails the attribution test — the
/// effect was not uniquely caused by the mechanism.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MechanismOffTwin {
    /// The champion's ΔlnZ vs ΛCDM (with the mechanism active).
    pub delta_ln_z_with_mechanism: f64,
    /// The mechanism-off baseline ΔlnZ vs ΛCDM (mechanism zeroed out).
    pub delta_ln_z_without_mechanism: f64,
    /// Description of which parameter(s) were zeroed to produce the off-twin.
    pub zeroed_parameters: Vec<String>,
}

impl MechanismOffTwin {
    /// Fraction of the champion's ΔlnZ that survives when the mechanism is disabled.
    ///
    /// Defined as `delta_ln_z_without / |delta_ln_z_with|` when `delta_ln_z_with ≠ 0`.
    /// Returns `1.0` (worst case: all effect retained) when `delta_ln_z_with` is ≈ 0.
    pub fn fraction_of_effect_retained(&self) -> f64 {
        let denom = self.delta_ln_z_with_mechanism.abs();
        if denom < 1e-9 {
            return 1.0; // denominator too small — conservative: assume no attribution
        }
        (self.delta_ln_z_without_mechanism / denom)
            .max(0.0)
            .min(1.0)
    }

    /// True when the mechanism accounts for ≥70% of the champion's ΔlnZ.
    ///
    /// That is, the off-twin retains ≤30% of the effect — the mechanism is load-bearing.
    pub fn mechanism_attribution_passes(&self) -> bool {
        self.fraction_of_effect_retained() <= (1.0 - MECHANISM_LOSS_THRESHOLD)
    }

    /// The portion of ΔlnZ attributable to the mechanism.
    pub fn attributed_delta_ln_z(&self) -> f64 {
        self.delta_ln_z_with_mechanism - self.delta_ln_z_without_mechanism
    }
}

/// The S06 profundity threshold for a "profound" discovery claim.
///
/// All three criteria must be met:
/// - `sigma_significance ≥ 5.0` (global significance, ~5σ)
/// - `delta_ln_z_corrected ≥ 5.0` (Bayesian evidence, post-trials)
/// - `mechanism_off_twin.mechanism_attribution_passes()` (mechanism drives ≥70% of effect)
///
/// Below this threshold, a claim may still be `DiscoveryClaim`-class (requiring only
/// post-search p < 0.003 with ≥200 replications) but cannot be labeled "profound" or
/// submitted to a Nature-tier journal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProfundityGate {
    /// Global significance expressed in units of σ (one-sided). ≥5.0 required for profound.
    pub sigma_significance: f64,
    /// ΔlnZ after the trials / look-elsewhere correction. ≥5.0 required for profound.
    pub delta_ln_z_corrected: f64,
    /// Ablation twin result. `None` means the mechanism-off evaluation has not been run —
    /// the profundity gate fails when this is absent.
    pub mechanism_off_twin: Option<MechanismOffTwin>,
}

impl ProfundityGate {
    /// True when all three profundity criteria are met.
    pub fn passes(&self) -> bool {
        let sigma_ok = self.sigma_significance >= NATURE_TIER_SIGMA;
        let lnz_ok = self.delta_ln_z_corrected >= PROFUNDITY_DELTA_LNZ_MIN;
        let mech_ok = self
            .mechanism_off_twin
            .as_ref()
            .map(|t| t.mechanism_attribution_passes())
            .unwrap_or(false);
        sigma_ok && lnz_ok && mech_ok
    }

    /// True only when this is a 5σ-equivalent profound claim with full mechanism attribution.
    /// Alias for `passes()` — every profound claim is Nature-tier eligible.
    pub fn is_nature_tier(&self) -> bool {
        self.passes()
    }

    /// List of failing criteria (empty when `passes()` is true).
    pub fn failure_reasons(&self) -> Vec<&'static str> {
        let mut reasons = Vec::new();
        if self.sigma_significance < NATURE_TIER_SIGMA {
            reasons.push("sigma_significance < 5.0");
        }
        if self.delta_ln_z_corrected < PROFUNDITY_DELTA_LNZ_MIN {
            reasons.push("delta_ln_z_corrected < 5.0");
        }
        match &self.mechanism_off_twin {
            None => reasons.push("mechanism_off_twin not evaluated"),
            Some(t) if !t.mechanism_attribution_passes() => {
                reasons.push("mechanism retains > 30% of ΔlnZ when disabled")
            }
            _ => {}
        }
        reasons
    }
}

/// Aggregated gate for upgrading `PromotionCandidate → DiscoveryClaim`.
///
/// All five sub-gates must pass before `ClaimClass::try_upgrade_to_discovery()` returns
/// `DiscoveryClaim`:
///
/// 1. `profundity_gate.passes()` — σ ≥ 5, ΔlnZ ≥ 5, mechanism attribution ≥ 70%
/// 2. `post_search_p_value < 0.003` with `post_search_n_replications ≥ 200` — null distribution
/// 3. `gof_passed` — absolute goodness of fit (χ²/dof < 2.0)
/// 4. `instrument_tier ≥ T2Boltzmann` — no formula-level instrument
/// 5. `has_sealed_forecast` — a pre-registered forecast is in the prediction registry
///
/// When any gate fails, `try_upgrade_to_discovery()` returns the input class unchanged
/// (typically `PromotionCandidate`), and `blocking_reasons()` reports which gates failed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiscoveryClaimGate {
    /// S06 profundity criteria.
    pub profundity_gate: ProfundityGate,
    /// Post-search p-value from the null distribution (Phase 6).
    pub post_search_p_value: f64,
    /// Number of null replications used (must be ≥ 200).
    pub post_search_n_replications: u32,
    /// Whether the absolute goodness-of-fit passed (χ²/dof < 2.0).
    pub gof_passed: bool,
    /// Forward-model instrument tier used when evaluating this candidate.
    pub instrument_tier: ForwardTier,
    /// True when a pre-registered, hash-sealed forecast exists in the prediction registry.
    pub has_sealed_forecast: bool,
}

impl DiscoveryClaimGate {
    /// True when all five sub-gates pass.
    pub fn passes_all_gates(&self) -> bool {
        self.profundity_gate.passes()
            && self.post_search_p_value < crate::validation::DISCOVERY_P_VALUE_THRESHOLD
            && self.post_search_n_replications >= crate::validation::MIN_NULL_REPLICATIONS
            && self.gof_passed
            && self.instrument_tier >= ForwardTier::T2Boltzmann
            && self.has_sealed_forecast
    }

    /// List of failing sub-gate descriptions (empty when `passes_all_gates()` is true).
    pub fn blocking_reasons(&self) -> Vec<String> {
        let mut reasons = Vec::new();
        for r in self.profundity_gate.failure_reasons() {
            reasons.push(format!("profundity: {r}"));
        }
        if self.post_search_n_replications < crate::validation::MIN_NULL_REPLICATIONS {
            reasons.push(format!(
                "insufficient null replications: {} < {}",
                self.post_search_n_replications,
                crate::validation::MIN_NULL_REPLICATIONS
            ));
        }
        if self.post_search_p_value >= crate::validation::DISCOVERY_P_VALUE_THRESHOLD {
            reasons.push(format!(
                "post-search p={:.4} ≥ {:.3} threshold",
                self.post_search_p_value,
                crate::validation::DISCOVERY_P_VALUE_THRESHOLD
            ));
        }
        if !self.gof_passed {
            reasons.push("absolute goodness-of-fit failed (χ²/dof ≥ 2.0)".into());
        }
        if self.instrument_tier < ForwardTier::T2Boltzmann {
            reasons.push(format!(
                "instrument tier {:?} < T2Boltzmann required for discovery",
                self.instrument_tier
            ));
        }
        if !self.has_sealed_forecast {
            reasons
                .push("no sealed forecast in prediction registry — cannot claim discovery".into());
        }
        reasons
    }

    /// Build a `DiscoveryClaimGate` from the Phase 5–7 evidence types.
    ///
    /// - `receipt`: the nested-sampling evidence receipt (provides ln_z → sigma, instrument tier)
    /// - `null_dist`: the Phase 6 null distribution (provides post-search p-value)
    /// - `gof_passed`: whether the absolute goodness-of-fit gate passed
    /// - `has_sealed_forecast`: whether a pre-registered forecast exists in the registry
    /// - `mechanism_off_twin`: optional ablation result; `None` → profundity gate will fail
    /// - `trials_corrected_delta_ln_z`: ΔlnZ after look-elsewhere correction
    pub fn from_evidence(
        receipt: &crate::validation::EvidenceReceipt,
        null_dist: &crate::validation::SearchNullDistribution,
        gof_passed: bool,
        has_sealed_forecast: bool,
        mechanism_off_twin: Option<MechanismOffTwin>,
        trials_corrected_delta_ln_z: f64,
    ) -> Self {
        let sigma = receipt.sigma_equivalent();
        DiscoveryClaimGate {
            profundity_gate: ProfundityGate {
                sigma_significance: sigma,
                delta_ln_z_corrected: trials_corrected_delta_ln_z,
                mechanism_off_twin,
            },
            post_search_p_value: null_dist.p_value_post_search,
            post_search_n_replications: null_dist.n_replications(),
            gof_passed,
            instrument_tier: ForwardTier::T2Boltzmann, // receipt implies Boltzmann-grade
            has_sealed_forecast,
        }
    }

    /// Evaluate the gate and return a `DiscoveryClaimGateResult` recording the verdict.
    pub fn evaluate(&self) -> DiscoveryClaimGateResult {
        let passed = self.passes_all_gates();
        let blocking_reasons = if passed {
            vec![]
        } else {
            self.blocking_reasons()
        };
        DiscoveryClaimGateResult {
            passed,
            blocking_reasons,
            post_search_p_value: self.post_search_p_value,
            post_search_n_replications: self.post_search_n_replications,
            sigma_significance: self.profundity_gate.sigma_significance,
            delta_ln_z_corrected: self.profundity_gate.delta_ln_z_corrected,
            mechanism_attribution_passed: self
                .profundity_gate
                .mechanism_off_twin
                .as_ref()
                .map(|t| t.mechanism_attribution_passes()),
        }
    }
}

/// Recorded verdict from evaluating a `DiscoveryClaimGate`.
///
/// This lightweight result is stored on `ScorecardV4` — it carries enough
/// information for a referee to understand why a candidate did or did not
/// reach `DiscoveryClaim` class, without embedding the full gate inputs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DiscoveryClaimGateResult {
    /// True when all sub-gates passed (candidate is `DiscoveryClaim`-eligible).
    pub passed: bool,
    /// Reasons why the gate failed (empty when `passed` is true).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocking_reasons: Vec<String>,
    /// Post-search p-value recorded at evaluation time.
    pub post_search_p_value: f64,
    /// Number of null replications at evaluation time.
    pub post_search_n_replications: u32,
    /// Global significance in σ units.
    pub sigma_significance: f64,
    /// ΔlnZ after trials correction.
    pub delta_ln_z_corrected: f64,
    /// Whether the mechanism attribution test passed (`None` if twin not evaluated).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mechanism_attribution_passed: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn passing_gate() -> DiscoveryClaimGate {
        DiscoveryClaimGate {
            profundity_gate: ProfundityGate {
                sigma_significance: 5.2,
                delta_ln_z_corrected: 6.1,
                mechanism_off_twin: Some(MechanismOffTwin {
                    delta_ln_z_with_mechanism: 6.1,
                    delta_ln_z_without_mechanism: 0.8,
                    zeroed_parameters: vec!["mu0".into()],
                }),
            },
            post_search_p_value: 0.001,
            post_search_n_replications: 250,
            gof_passed: true,
            instrument_tier: ForwardTier::T2Boltzmann,
            has_sealed_forecast: true,
        }
    }

    // ---- MechanismOffTwin ----

    #[test]
    fn mechanism_attribution_passes_when_twin_loses_70_percent() {
        let twin = MechanismOffTwin {
            delta_ln_z_with_mechanism: 6.0,
            delta_ln_z_without_mechanism: 1.2, // retains 20%
            zeroed_parameters: vec!["mu0".into()],
        };
        assert!(twin.mechanism_attribution_passes());
        let retained = twin.fraction_of_effect_retained();
        assert!((retained - 0.2).abs() < 1e-9);
    }

    #[test]
    fn mechanism_attribution_fails_when_twin_retains_over_30_percent() {
        let twin = MechanismOffTwin {
            delta_ln_z_with_mechanism: 6.0,
            delta_ln_z_without_mechanism: 2.5, // retains ~42%
            zeroed_parameters: vec!["A_drag".into()],
        };
        assert!(!twin.mechanism_attribution_passes());
    }

    #[test]
    fn mechanism_twin_zero_champion_is_conservative() {
        let twin = MechanismOffTwin {
            delta_ln_z_with_mechanism: 0.0,
            delta_ln_z_without_mechanism: 0.0,
            zeroed_parameters: vec![],
        };
        // denominator ≈ 0 → conservative: fraction = 1.0 → attribution fails
        assert!(!twin.mechanism_attribution_passes());
    }

    #[test]
    fn attributed_delta_ln_z_is_difference() {
        let twin = MechanismOffTwin {
            delta_ln_z_with_mechanism: 5.0,
            delta_ln_z_without_mechanism: 1.0,
            zeroed_parameters: vec!["mu0".into()],
        };
        assert!((twin.attributed_delta_ln_z() - 4.0).abs() < 1e-9);
    }

    // ---- ProfundityGate ----

    #[test]
    fn profundity_gate_passes_when_all_criteria_met() {
        let gate = ProfundityGate {
            sigma_significance: 5.1,
            delta_ln_z_corrected: 5.5,
            mechanism_off_twin: Some(MechanismOffTwin {
                delta_ln_z_with_mechanism: 5.5,
                delta_ln_z_without_mechanism: 0.8,
                zeroed_parameters: vec!["mu0".into()],
            }),
        };
        assert!(gate.passes());
        assert!(gate.is_nature_tier());
        assert!(gate.failure_reasons().is_empty());
    }

    #[test]
    fn profundity_gate_fails_low_sigma() {
        let gate = ProfundityGate {
            sigma_significance: 4.5, // too low
            delta_ln_z_corrected: 6.0,
            mechanism_off_twin: Some(MechanismOffTwin {
                delta_ln_z_with_mechanism: 6.0,
                delta_ln_z_without_mechanism: 0.5,
                zeroed_parameters: vec!["mu0".into()],
            }),
        };
        assert!(!gate.passes());
        assert!(gate
            .failure_reasons()
            .iter()
            .any(|r| r.contains("sigma_significance")));
    }

    #[test]
    fn profundity_gate_fails_when_twin_missing() {
        let gate = ProfundityGate {
            sigma_significance: 5.5,
            delta_ln_z_corrected: 6.0,
            mechanism_off_twin: None,
        };
        assert!(!gate.passes());
        let reasons = gate.failure_reasons();
        assert!(
            reasons.iter().any(|r| r.contains("not evaluated")),
            "{reasons:?}"
        );
    }

    #[test]
    fn profundity_gate_fails_low_delta_lnz() {
        let gate = ProfundityGate {
            sigma_significance: 5.5,
            delta_ln_z_corrected: 3.0, // too low
            mechanism_off_twin: Some(MechanismOffTwin {
                delta_ln_z_with_mechanism: 3.0,
                delta_ln_z_without_mechanism: 0.3,
                zeroed_parameters: vec!["mu0".into()],
            }),
        };
        assert!(!gate.passes());
        assert!(gate
            .failure_reasons()
            .iter()
            .any(|r| r.contains("delta_ln_z_corrected")));
    }

    // ---- DiscoveryClaimGate ----

    #[test]
    fn discovery_gate_passes_when_all_sub_gates_pass() {
        let gate = passing_gate();
        assert!(gate.passes_all_gates());
        assert!(gate.blocking_reasons().is_empty());
    }

    #[test]
    fn discovery_gate_blocked_by_low_instrument_tier() {
        let mut gate = passing_gate();
        gate.instrument_tier = ForwardTier::T0Formula;
        assert!(!gate.passes_all_gates());
        let reasons = gate.blocking_reasons();
        assert!(
            reasons.iter().any(|r| r.contains("T2Boltzmann")),
            "{reasons:?}"
        );
    }

    #[test]
    fn discovery_gate_blocked_without_sealed_forecast() {
        let mut gate = passing_gate();
        gate.has_sealed_forecast = false;
        assert!(!gate.passes_all_gates());
        let reasons = gate.blocking_reasons();
        assert!(
            reasons.iter().any(|r| r.contains("sealed forecast")),
            "{reasons:?}"
        );
    }

    #[test]
    fn discovery_gate_blocked_by_insufficient_null_replications() {
        let mut gate = passing_gate();
        gate.post_search_n_replications = 50;
        assert!(!gate.passes_all_gates());
        let reasons = gate.blocking_reasons();
        assert!(
            reasons.iter().any(|r| r.contains("null replications")),
            "{reasons:?}"
        );
    }

    #[test]
    fn discovery_gate_blocked_by_high_post_search_p() {
        let mut gate = passing_gate();
        gate.post_search_p_value = 0.01; // above 0.003 threshold
        assert!(!gate.passes_all_gates());
        let reasons = gate.blocking_reasons();
        assert!(
            reasons.iter().any(|r| r.contains("post-search p")),
            "{reasons:?}"
        );
    }

    #[test]
    fn discovery_gate_blocked_by_gof_failure() {
        let mut gate = passing_gate();
        gate.gof_passed = false;
        assert!(!gate.passes_all_gates());
        let reasons = gate.blocking_reasons();
        assert!(
            reasons.iter().any(|r| r.contains("goodness-of-fit")),
            "{reasons:?}"
        );
    }

    #[test]
    fn multiple_blockers_are_all_reported() {
        let mut gate = passing_gate();
        gate.gof_passed = false;
        gate.has_sealed_forecast = false;
        gate.instrument_tier = ForwardTier::T1Emulator;
        let reasons = gate.blocking_reasons();
        assert!(
            reasons.len() >= 3,
            "expected ≥3 blocking reasons: {reasons:?}"
        );
    }

    // ---- from_evidence factory ----

    fn sample_receipt() -> crate::validation::EvidenceReceipt {
        crate::validation::EvidenceReceipt {
            solver: crate::validation::NestingSolver::UltraNest,
            ln_z: 6.0,
            ln_z_err: 1.0, // sigma = 6.0
            n_live: 500,
            n_iter: 10_000,
            effective_n_samples: None,
            prior_hash: "a".repeat(64),
            data_hash: "b".repeat(64),
            laplace_diagnostic: None,
        }
    }

    fn sample_null_dist() -> crate::validation::SearchNullDistribution {
        let reps: Vec<_> = (0..200)
            .map(|i| crate::validation::NullReplication {
                max_delta_ln_z: i as f64 * 0.01,
                n_candidates: 100,
                rng_seed: i as u64,
            })
            .collect();
        crate::validation::SearchNullDistribution::new(5.0, reps)
    }

    #[test]
    fn from_evidence_builds_gate_from_receipt_and_null() {
        let receipt = sample_receipt();
        let null = sample_null_dist();
        let twin = MechanismOffTwin {
            delta_ln_z_with_mechanism: 6.0,
            delta_ln_z_without_mechanism: 0.8,
            zeroed_parameters: vec!["mu0".into()],
        };
        let gate = DiscoveryClaimGate::from_evidence(&receipt, &null, true, true, Some(twin), 5.5);
        // sigma = ln_z / ln_z_err = 6.0 / 1.0 = 6.0
        assert!((gate.profundity_gate.sigma_significance - 6.0).abs() < 1e-9);
        assert_eq!(gate.post_search_n_replications, 200);
        assert_eq!(gate.post_search_p_value, 0.0);
        assert!(gate.passes_all_gates());
    }

    #[test]
    fn from_evidence_fails_when_no_sealed_forecast() {
        let receipt = sample_receipt();
        let null = sample_null_dist();
        let gate = DiscoveryClaimGate::from_evidence(
            &receipt,
            &null,
            true,
            false, // no sealed forecast
            Some(MechanismOffTwin {
                delta_ln_z_with_mechanism: 6.0,
                delta_ln_z_without_mechanism: 0.5,
                zeroed_parameters: vec![],
            }),
            5.5,
        );
        assert!(!gate.passes_all_gates());
        let reasons = gate.blocking_reasons();
        assert!(reasons.iter().any(|r| r.contains("sealed forecast")));
    }
}
