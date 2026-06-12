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
}
