//! V8 Phase 1 (#4): EvidenceReceipt — the structured output of a nested-sampling run.
//!
//! The `EvidenceReceipt` captures the Bayesian evidence estimate (ln Z) and its uncertainty
//! from a completed nested-sampling run (UltraNest, dynesty, MultiNest …). It is the typed
//! replacement for raw log-files: downstream code can verify the run completed, check the
//! uncertainty budget, and compare cross-solver consistency without string parsing.
//!
//! **External solver integration**: this module defines the Rust interface types. The actual
//! UltraNest / dynesty subprocess integration lives in `cosmology/subprocess.rs` and requires
//! the external solver to be installed. These types are usable without the solver.

use serde::{Deserialize, Serialize};

/// Which nested-sampling backend produced this receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NestingSolver {
    /// UltraNest — reactive nested sampling with reliable uncertainty estimates.
    UltraNest,
    /// dynesty — dynamic nested sampling.
    Dynesty,
    /// MultiNest — multinest (legacy; lower accuracy for multimodal posteriors).
    MultiNest,
    /// Internal — deterministic quadrature approximation (for unit tests and CI; not
    /// promotion-grade; maps to T1Emulator instrument tier).
    Internal,
}

impl NestingSolver {
    /// True when this solver is promotion-grade (usable for ΔlnZ > 2 claims).
    /// Only UltraNest and dynesty are considered promotion-grade.
    pub fn is_promotion_grade(self) -> bool {
        matches!(self, NestingSolver::UltraNest | NestingSolver::Dynesty)
    }
}

impl std::fmt::Display for NestingSolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NestingSolver::UltraNest => write!(f, "ultranest"),
            NestingSolver::Dynesty => write!(f, "dynesty"),
            NestingSolver::MultiNest => write!(f, "multinest"),
            NestingSolver::Internal => write!(f, "internal"),
        }
    }
}

/// The full output receipt from one completed nested-sampling run.
///
/// Produced by running the external solver subprocess and parsing its output.
/// The `ln_z` and `ln_z_err` fields are the primary evidence estimate; all other fields
/// are diagnostics that help validate the run quality before the receipt is accepted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceReceipt {
    /// Which solver produced this receipt.
    pub solver: NestingSolver,

    /// The estimated log-evidence ln Z for this theory on this dataset.
    pub ln_z: f64,

    /// 1-sigma uncertainty on ln Z (from the nested-sampling run itself).
    /// A run is considered trustworthy when `ln_z_err < 0.5`.
    pub ln_z_err: f64,

    /// Number of live points used. More live points → lower `ln_z_err`.
    pub n_live: u32,

    /// Number of iterations (dead points) at termination.
    pub n_iter: u64,

    /// Effective sample size of the posterior — a diagnostic for run quality.
    /// `None` if the solver does not report it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective_n_samples: Option<f64>,

    /// SHA-256 hash of the prior registry entry used for this run.
    /// Locks the prior to the receipt: two receipts with the same `prior_hash`
    /// used the same prior bounds and parameterization.
    pub prior_hash: String,

    /// SHA-256 hash of the dataset used. Locks the data.
    pub data_hash: String,

    /// V8 Phase 8: Laplace approximation cross-check against the nested-sampling result.
    ///
    /// When present, `disagreement_with_nested` in the diagnostic is filled with
    /// `|ln_z_laplace − ln_z_nested|`. Values > 0.5 ln-units indicate the Laplace
    /// approximation is unreliable for this candidate — the nested result stands.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub laplace_diagnostic: Option<crate::scoring::LaplaceDiagnostic>,
}

impl EvidenceReceipt {
    /// True when this receipt is trustworthy enough to report a ΔlnZ claim.
    ///
    /// Conditions: solver is promotion-grade AND `ln_z_err < 0.5` AND
    /// prior/data hashes are non-empty (run was against a real prior + data).
    pub fn is_trustworthy(&self) -> bool {
        self.solver.is_promotion_grade()
            && self.ln_z_err < 0.5
            && !self.prior_hash.is_empty()
            && !self.data_hash.is_empty()
    }

    /// The signal-to-noise ratio of the evidence estimate: |ln Z| / ln_z_err.
    /// A value > 4 means the evidence is well above the noise floor.
    pub fn ln_z_snr(&self) -> f64 {
        if self.ln_z_err == 0.0 {
            f64::INFINITY
        } else {
            self.ln_z.abs() / self.ln_z_err
        }
    }

    /// Approximate global significance in units of σ for a positive ΔlnZ.
    ///
    /// This uses the relation σ ≈ ΔlnZ / ln_z_err, which holds when the evidence is large
    /// compared to its uncertainty. Returns 0.0 when `ln_z ≤ 0` (no positive evidence) or
    /// when `ln_z_err = 0` (degenerate run). This is an approximation; the rigorous route
    /// is `SearchNullDistribution.p_value_post_search` converted via `erfinv`.
    pub fn sigma_equivalent(&self) -> f64 {
        if self.ln_z <= 0.0 || self.ln_z_err == 0.0 {
            return 0.0;
        }
        self.ln_z / self.ln_z_err
    }

    /// Attach a Laplace diagnostic, filling in the `disagreement_with_nested` field
    /// from the difference between the Laplace estimate and this receipt's `ln_z`.
    ///
    /// This is the standard way to attach a Laplace cross-check to a nested-sampling
    /// receipt: compute the Laplace estimate, then call this method to record the comparison.
    pub fn with_laplace_diagnostic(mut self, mut diag: crate::scoring::LaplaceDiagnostic) -> Self {
        let disagreement = (diag.ln_z_estimate - self.ln_z).abs();
        diag.disagreement_with_nested = Some(disagreement);
        self.laplace_diagnostic = Some(diag);
        self
    }
}

/// Registry entry for a prior used in nested sampling.
///
/// The prior registry is the machine-checkable answer to "what prior was used for this run?"
/// Each entry is hashed and the hash is embedded in `EvidenceReceipt.prior_hash` so the
/// exact prior can be recovered and re-run later.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PriorEntry {
    /// Unique identifier for this prior specification.
    pub entry_id: String,

    /// Human-readable description of the prior.
    pub description: String,

    /// Per-parameter bounds: (parameter_name, lower_bound, upper_bound).
    /// All priors are uniform; log-uniform priors should be expressed in log-space.
    pub bounds: Vec<(String, f64, f64)>,
}

impl PriorEntry {
    /// Compute the SHA-256 hash of this prior entry (deterministic JSON serialization).
    /// This is the value that goes into `EvidenceReceipt.prior_hash`.
    pub fn content_hash(&self) -> String {
        let canonical = serde_json::to_string(self).unwrap_or_default();
        crate::sha256_digest(canonical.as_bytes())
    }
}

/// Transform applied to a parameter before it enters the sampler.
///
/// Controls how the unit hypercube from the sampler maps to physical parameter space.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PriorTransform {
    /// Linear mapping: physical = lo + (hi - lo) * u.
    Linear,
    /// Log-10 absolute-value mapping: physical = 10^(lo_log + (hi_log - lo_log) * u).
    Log10Abs,
    /// Logit unit mapping: physical = logit(u) scaled to (lo, hi).
    LogitUnit,
    /// Fixed derived value — not sampled; passed through as a constant.
    DerivedFixed { value: f64 },
}

/// Prior density assigned to a parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PriorDensity {
    /// Flat prior over the declared support.
    Uniform,
    /// Log-uniform prior (Jeffreys for scale parameters).
    LogUniform,
    /// Gaussian informative prior from an external experiment (not the scored dataset).
    GaussianExternal { mean: f64, sigma: f64 },
}

/// Per-parameter prior specification for a nested-sampling run.
///
/// The prior registry is machine-owned: the LLM proposer may not set prior bounds.
/// Bounds must come from the physics registry or an explicit analyst decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorSpec {
    pub name: String,
    pub transform: PriorTransform,
    pub support: [f64; 2],
    pub density: PriorDensity,
}

/// Which backend sampler to invoke.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SamplerBackend {
    UltraNest,
    Dynesty,
    PolyChord,
    /// Internal deterministic quadrature — CI / unit tests only; not promotion-grade.
    Internal,
}

/// Sampler-level configuration for one nested-sampling run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplerSpec {
    pub backend: SamplerBackend,
    pub n_live: u32,
    pub rng_seed: u64,
    pub max_likelihood_calls: u64,
    pub walltime_seconds: u64,
}

/// Request sent by Rust to the external nested-sampling subprocess.
///
/// Rust writes this JSON file; the child process reads it, runs the sampler against
/// the deterministic likelihood service, and returns an [`EvidenceReceipt`].
/// Rust validates hashes and GoF gates before accepting the receipt.
///
/// The process boundary isolates Rust policy from Python/Fortran sampler implementations.
/// All hashes must match before a receipt is accepted by the Rust side.
///
/// Reference: S07 §"Evidence backbone: nested sampling by process boundary".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceRequest {
    pub run_id: String,
    pub model_id: String,
    /// SHA-256 of the serialized theory / model specification.
    pub model_fingerprint: String,
    /// SHA-256 of the data manifest used to select observables.
    pub data_manifest_hash: String,
    /// SHA-256 of the likelihood function specification (covariance blocks + observable list).
    pub likelihood_manifest_hash: String,
    /// SHA-256 of the prior registry entry — locks the priors to this receipt.
    pub prior_registry_hash: String,
    /// Per-parameter prior specifications. Identical to the prior registry entry at this hash.
    pub parameters: Vec<PriorSpec>,
    pub sampler: SamplerSpec,
}

impl EvidenceRequest {
    /// Content hash of this request, for binding a receipt to its exact run specification.
    pub fn content_hash(&self) -> String {
        let canonical = serde_json::to_string(self).unwrap_or_default();
        crate::sha256_digest(canonical.as_bytes())
    }

    /// True when the request specifies a promotion-grade backend.
    pub fn is_promotion_grade(&self) -> bool {
        matches!(
            self.sampler.backend,
            SamplerBackend::UltraNest | SamplerBackend::Dynesty | SamplerBackend::PolyChord
        )
    }
}

/// Consistency check between two EvidenceReceipts from different solvers on the same
/// theory + dataset. Returns `None` if the two receipts agree within combined uncertainty.
pub fn cross_solver_tension(a: &EvidenceReceipt, b: &EvidenceReceipt) -> Option<f64> {
    let combined_err = (a.ln_z_err.powi(2) + b.ln_z_err.powi(2)).sqrt();
    if combined_err == 0.0 {
        return None;
    }
    let tension = (a.ln_z - b.ln_z).abs() / combined_err;
    if tension > 2.0 {
        Some(tension)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good_receipt(solver: NestingSolver) -> EvidenceReceipt {
        EvidenceReceipt {
            solver,
            ln_z: -5.2,
            ln_z_err: 0.12,
            n_live: 500,
            n_iter: 12_000,
            effective_n_samples: Some(480.0),
            prior_hash: "a".repeat(64),
            data_hash: "b".repeat(64),
            laplace_diagnostic: None,
        }
    }

    #[test]
    fn ultranest_is_promotion_grade() {
        assert!(NestingSolver::UltraNest.is_promotion_grade());
        assert!(NestingSolver::Dynesty.is_promotion_grade());
        assert!(!NestingSolver::MultiNest.is_promotion_grade());
        assert!(!NestingSolver::Internal.is_promotion_grade());
    }

    #[test]
    fn trustworthy_receipt_passes_all_checks() {
        let r = good_receipt(NestingSolver::UltraNest);
        assert!(r.is_trustworthy());
    }

    #[test]
    fn internal_solver_is_not_trustworthy() {
        let r = good_receipt(NestingSolver::Internal);
        assert!(!r.is_trustworthy());
    }

    #[test]
    fn high_ln_z_err_is_not_trustworthy() {
        let mut r = good_receipt(NestingSolver::UltraNest);
        r.ln_z_err = 0.5;
        assert!(!r.is_trustworthy());
    }

    #[test]
    fn empty_hashes_are_not_trustworthy() {
        let mut r = good_receipt(NestingSolver::UltraNest);
        r.prior_hash = String::new();
        assert!(!r.is_trustworthy());
    }

    #[test]
    fn snr_computed_correctly() {
        let r = good_receipt(NestingSolver::UltraNest);
        let expected = r.ln_z.abs() / r.ln_z_err;
        assert!((r.ln_z_snr() - expected).abs() < 1e-12);
    }

    #[test]
    fn cross_solver_no_tension_within_2_sigma() {
        let mut a = good_receipt(NestingSolver::UltraNest);
        let mut b = good_receipt(NestingSolver::Dynesty);
        a.ln_z = -5.2;
        b.ln_z = -5.3; // 0.1 apart, combined_err = sqrt(0.12^2 + 0.12^2) ≈ 0.17; tension ≈ 0.59
        assert!(cross_solver_tension(&a, &b).is_none());
    }

    #[test]
    fn cross_solver_tension_detected_above_2_sigma() {
        let mut a = good_receipt(NestingSolver::UltraNest);
        let mut b = good_receipt(NestingSolver::Dynesty);
        a.ln_z = -5.2;
        b.ln_z = -5.2 - 1.5; // combined_err ≈ 0.17; tension = 1.5/0.17 ≈ 8.8 > 2
        let t = cross_solver_tension(&a, &b);
        assert!(t.is_some());
        assert!(t.unwrap() > 2.0);
    }

    #[test]
    fn prior_entry_hash_is_stable() {
        let e = PriorEntry {
            entry_id: "ndgp-v1".into(),
            description: "nDGP prior".into(),
            bounds: vec![("omega_m".into(), 0.2, 0.4), ("h".into(), 0.6, 0.8)],
        };
        let h1 = e.content_hash();
        let h2 = e.content_hash();
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    // ---- V8 Phase 5: EvidenceRequest process contract ----

    fn sample_request() -> EvidenceRequest {
        EvidenceRequest {
            run_id: "run-001".into(),
            model_id: "planck-mu0-v1".into(),
            model_fingerprint: "a".repeat(64),
            data_manifest_hash: "b".repeat(64),
            likelihood_manifest_hash: "c".repeat(64),
            prior_registry_hash: "d".repeat(64),
            parameters: vec![PriorSpec {
                name: "mu0".into(),
                transform: PriorTransform::Linear,
                support: [-0.30, 0.00],
                density: PriorDensity::Uniform,
            }],
            sampler: SamplerSpec {
                backend: SamplerBackend::UltraNest,
                n_live: 400,
                rng_seed: 42,
                max_likelihood_calls: 100_000,
                walltime_seconds: 3600,
            },
        }
    }

    #[test]
    fn sigma_equivalent_positive_evidence() {
        // ln_z = 6.0, ln_z_err = 1.2 → sigma = 5.0
        let mut r = good_receipt(NestingSolver::UltraNest);
        r.ln_z = 6.0;
        r.ln_z_err = 1.2;
        assert!((r.sigma_equivalent() - 5.0).abs() < 1e-9);
    }

    #[test]
    fn sigma_equivalent_zero_when_evidence_negative() {
        let mut r = good_receipt(NestingSolver::UltraNest);
        r.ln_z = -3.0;
        assert_eq!(r.sigma_equivalent(), 0.0);
    }

    #[test]
    fn sigma_equivalent_zero_when_err_is_zero() {
        let mut r = good_receipt(NestingSolver::UltraNest);
        r.ln_z = 5.0;
        r.ln_z_err = 0.0;
        assert_eq!(r.sigma_equivalent(), 0.0);
    }

    #[test]
    fn with_laplace_diagnostic_fills_disagreement() {
        use crate::scoring::{LaplaceDiagnostic, LaplaceValidity};
        let r = good_receipt(NestingSolver::UltraNest); // ln_z = -5.2
        let diag = LaplaceDiagnostic {
            ln_z_estimate: -4.8,
            validity: LaplaceValidity::Valid,
            disagreement_with_nested: None,
        };
        let enriched = r.with_laplace_diagnostic(diag);
        let d = enriched.laplace_diagnostic.unwrap();
        // |(-4.8) - (-5.2)| = 0.4
        assert!((d.disagreement_with_nested.unwrap() - 0.4).abs() < 1e-9);
        assert!(d.is_reliable()); // 0.4 <= 0.5 and Valid
    }

    #[test]
    fn with_laplace_diagnostic_large_disagreement_is_unreliable() {
        use crate::scoring::{LaplaceDiagnostic, LaplaceValidity};
        let mut r = good_receipt(NestingSolver::UltraNest);
        r.ln_z = -10.0;
        let diag = LaplaceDiagnostic {
            ln_z_estimate: -9.0,
            validity: LaplaceValidity::Valid,
            disagreement_with_nested: None,
        };
        let enriched = r.with_laplace_diagnostic(diag);
        let d = enriched.laplace_diagnostic.unwrap();
        // |(-9.0) - (-10.0)| = 1.0 > 0.5 → unreliable
        assert!(!d.is_reliable());
    }

    #[test]
    fn laplace_diagnostic_field_roundtrips_through_json() {
        use crate::scoring::{LaplaceDiagnostic, LaplaceValidity};
        let r = good_receipt(NestingSolver::UltraNest);
        let diag = LaplaceDiagnostic {
            ln_z_estimate: -5.0,
            validity: LaplaceValidity::Valid,
            disagreement_with_nested: Some(0.2),
        };
        let enriched = r.with_laplace_diagnostic(diag);
        let json = serde_json::to_string(&enriched).unwrap();
        let back: EvidenceReceipt = serde_json::from_str(&json).unwrap();
        assert!(back.laplace_diagnostic.is_some());
    }

    #[test]
    fn receipt_without_laplace_still_roundtrips() {
        let r = good_receipt(NestingSolver::Dynesty);
        let json = serde_json::to_string(&r).unwrap();
        let back: EvidenceReceipt = serde_json::from_str(&json).unwrap();
        assert!(back.laplace_diagnostic.is_none());
    }

    #[test]
    fn evidence_request_content_hash_stable() {
        let r = sample_request();
        let h1 = r.content_hash();
        let h2 = r.content_hash();
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn evidence_request_promotion_grade_backends() {
        let mut r = sample_request();
        assert!(r.is_promotion_grade());
        r.sampler.backend = SamplerBackend::Internal;
        assert!(!r.is_promotion_grade());
    }

    #[test]
    fn evidence_request_hash_changes_with_model() {
        let r1 = sample_request();
        let mut r2 = sample_request();
        r2.model_id = "dark-scattering-v1".into();
        assert_ne!(r1.content_hash(), r2.content_hash());
    }
}
