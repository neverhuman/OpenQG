//! V8 Phase 3 (#15): GP dual-path run contract types.
//!
//! A GP midpoint run has two parallel paths:
//!
//! 1. **Whitebox path** — generates from a structured action algebra; algebraic output feeds
//!    the term-algebra gate. Only this path is allowed to earn mechanism credit.
//!
//! 2. **Phenomenology-only path** — GP-fitted curve; quarantined to a separate `BehaviorDescriptor`
//!    that cannot be promoted to a scored theory. Exists to map observational landscape only.
//!
//! The run boundary enforces this split: a `GpRunManifest` names which primitives are whitebox
//! and which are phenom-only. The `GpRunReceipt` records the Pareto front, the quarantine map,
//! and any bounty claims for each candidate.
//!
//! Spec reference: S02 §1-3.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Primitive function primitives allowed in whitebox expressions.
///
/// Phenomenological-only primitives (`Polynomial`, `FreeSpline`) may appear in the phenom-only
/// path but are quarantined and cannot produce mechanism credit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Primitive {
    /// `f(x, n)` = x^n with integer n.
    Power,
    /// `exp(a*x)`.
    Exponential,
    /// `tanh(x/w)` — screened-to-unscreened transition kernel.
    ScreeningKernel,
    /// `1/(1 + (x/x0)^n)` — Hu-Sawicki profile kernel.
    HuSawickiProfile,
    /// `sin(omega*x + phi)`.
    Oscillatory,
    /// Polynomial basis — **phenomenology-only path**.
    Polynomial,
    /// Free cubic/quintic spline — **phenomenology-only path**.
    FreeSpline,
}

impl Primitive {
    /// True when this primitive is restricted to the phenom-only quarantined path.
    pub fn is_phenom_only(&self) -> bool {
        matches!(self, Primitive::Polynomial | Primitive::FreeSpline)
    }
}

/// A midpoint-form expression: a formula string plus its complexity budget.
///
/// Expressions are validated for dimensional consistency before entering the structural gate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidpointForm {
    /// The expression string in the Horndeski basis (e.g. `"G2 = X - m^2 phi^2 / 2"`).
    pub expression: String,
    /// Number of primitive operations (tree nodes), used in complexity scoring.
    pub complexity: u32,
    /// Whether all primitives are whitebox (false ⟹ quarantined to phenom-only path).
    pub is_whitebox: bool,
    /// SHA-256 of the expression string for content binding.
    pub content_hash: String,
    /// Which GP run iteration produced this form.
    pub iteration: u32,
}

impl MidpointForm {
    pub fn new(
        expression: impl Into<String>,
        complexity: u32,
        is_whitebox: bool,
        iteration: u32,
    ) -> Self {
        let expr = expression.into();
        let hash = Sha256::digest(expr.as_bytes());
        MidpointForm {
            expression: expr,
            complexity,
            is_whitebox,
            content_hash: format!("sha256:{hash:x}"),
            iteration,
        }
    }
}

/// A descriptive summary of a GP candidate's behavior across the observable space.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorDescriptor {
    pub candidate_id: String,
    /// Pull bins by observable (whitened residuals bucketed to [-3,-1,0,+1,+3] sigma).
    pub observable_pull_bins: std::collections::BTreeMap<String, f64>,
    /// Mechanism bins: what fraction of the fsigma8 / shear / H0 signal is mechanistic vs phenom.
    pub mechanism_fraction: std::collections::BTreeMap<String, f64>,
    /// True when the candidate is quarantined to the phenom-only path.
    pub quarantined: bool,
    /// Reason for quarantine (empty when `quarantined = false`).
    pub quarantine_reason: String,
}

/// An outstanding bounty: a pre-committed score bonus for a theory that achieves something
/// specific (e.g. resolving the H0 tension with a whitebox mechanism).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bounty {
    pub bounty_id: String,
    pub description: String,
    pub target_observable: String,
    /// Required reduction in pull_sigma to claim the bounty.
    pub required_pull_reduction: f64,
    /// Extra score points awarded when claimed.
    pub score_award: f64,
    pub status: BountyStatus,
}

/// Current state of a bounty claim.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BountyStatus {
    Open,
    Claimed { by_candidate_id: String },
    Expired { reason: String },
}

/// Input hashes for a GP run: seals what data the run consumed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunInputHashes {
    pub observable_registry_hash: String,
    pub action_algebra_hash: String,
    pub primitive_whitelist_hash: String,
    pub knowledge_snapshot_hash: Option<String>,
}

/// The run contract: what a GP run is allowed to do and what constraints it must respect.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpRunManifest {
    pub run_id: String,
    pub schema_version: String,
    pub parent_run_id: Option<String>,
    pub input_hashes: RunInputHashes,
    /// Whitebox-allowed primitives (phenom-only primitives are allowed but quarantined).
    pub primitive_whitelist: Vec<Primitive>,
    /// Maximum complexity budget for whitebox path expressions.
    pub max_complexity: u32,
    /// Maximum number of GP iterations per path.
    pub max_iterations: u32,
    /// Maximum wall-clock seconds per run.
    pub max_wallclock_seconds: u32,
    /// Active bounties this run should attempt.
    pub active_bounties: Vec<Bounty>,
    /// Knowledge lessons injected into the proposer context for this run.
    pub lesson_ids: Vec<String>,
    /// Which observables are frozen (sealed) vs open-fit for this run.
    pub sealed_observable_ids: Vec<String>,
}

impl GpRunManifest {
    /// SHA-256 of the canonical manifest payload (for content-binding).
    pub fn manifest_hash(&self) -> String {
        let payload = serde_json::to_string(self).unwrap_or_default();
        let hash = Sha256::digest(payload.as_bytes());
        format!("sha256:{hash:x}")
    }
}

/// A single scored candidate from a GP run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpCandidate {
    pub candidate_id: String,
    pub run_id: String,
    pub midpoint_form: MidpointForm,
    pub behavior: BehaviorDescriptor,
    /// Score on the whitebox scorecard (0.0 when quarantined).
    pub whitebox_score: f64,
    /// Score on the phenom-only scorecard (separate, not promotable).
    pub phenom_score: f64,
    /// Bounties claimed by this candidate.
    pub claimed_bounty_ids: Vec<String>,
    /// Whether the theory passed the structural generation gate.
    pub structural_gate_passed: bool,
}

/// Pareto front entry: trade-off between whitebox_score and complexity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParetoEntry {
    pub candidate_id: String,
    pub whitebox_score: f64,
    pub complexity: u32,
    /// True when this entry dominates all others at its complexity level.
    pub pareto_dominant: bool,
}

/// Receipt recording the outcome of one GP run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpRunReceipt {
    pub run_id: String,
    pub manifest_hash: String,
    pub schema_version: String,
    pub elapsed_seconds: f64,
    pub iterations_completed: u32,
    pub total_candidates: u32,
    pub whitebox_candidates: u32,
    pub quarantined_candidates: u32,
    pub pareto_front: Vec<ParetoEntry>,
    pub candidates: Vec<GpCandidate>,
    pub claimed_bounties: Vec<String>,
    pub lessons_applied: Vec<String>,
}

impl GpRunReceipt {
    /// The best (highest whitebox_score) non-quarantined candidate.
    pub fn best_whitebox(&self) -> Option<&GpCandidate> {
        self.candidates
            .iter()
            .filter(|c| !c.behavior.quarantined && c.structural_gate_passed)
            .max_by(|a, b| {
                a.whitebox_score
                    .partial_cmp(&b.whitebox_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn midpoint_form_content_hash_is_stable() {
        let f = MidpointForm::new("G2 = X - m^2 phi^2 / 2", 4, true, 0);
        let f2 = MidpointForm::new("G2 = X - m^2 phi^2 / 2", 4, true, 0);
        assert_eq!(f.content_hash, f2.content_hash);
        assert!(f.content_hash.starts_with("sha256:"));
    }

    #[test]
    fn phenom_only_primitives_flagged_correctly() {
        assert!(Primitive::Polynomial.is_phenom_only());
        assert!(Primitive::FreeSpline.is_phenom_only());
        assert!(!Primitive::Exponential.is_phenom_only());
        assert!(!Primitive::HuSawickiProfile.is_phenom_only());
    }

    #[test]
    fn bounty_status_transitions() {
        let b = Bounty {
            bounty_id: "h0-tension-resolution".into(),
            description: "Resolve H0 tension with whitebox mechanism".into(),
            target_observable: "h0".into(),
            required_pull_reduction: 1.5,
            score_award: 20.0,
            status: BountyStatus::Open,
        };
        assert_eq!(b.status, BountyStatus::Open);
    }

    #[test]
    fn gp_run_receipt_best_whitebox_finds_top() {
        let receipt = GpRunReceipt {
            run_id: "run-01".into(),
            manifest_hash: "sha256:abc".into(),
            schema_version: "v8.0.0".into(),
            elapsed_seconds: 12.0,
            iterations_completed: 50,
            total_candidates: 3,
            whitebox_candidates: 2,
            quarantined_candidates: 1,
            pareto_front: vec![],
            candidates: vec![
                GpCandidate {
                    candidate_id: "cand-a".into(),
                    run_id: "run-01".into(),
                    midpoint_form: MidpointForm::new("G4 = 1/2", 1, true, 1),
                    behavior: BehaviorDescriptor {
                        candidate_id: "cand-a".into(),
                        observable_pull_bins: Default::default(),
                        mechanism_fraction: Default::default(),
                        quarantined: false,
                        quarantine_reason: String::new(),
                    },
                    whitebox_score: 72.0,
                    phenom_score: 0.0,
                    claimed_bounty_ids: vec![],
                    structural_gate_passed: true,
                },
                GpCandidate {
                    candidate_id: "cand-b".into(),
                    run_id: "run-01".into(),
                    midpoint_form: MidpointForm::new("G2 = X", 1, false, 2),
                    behavior: BehaviorDescriptor {
                        candidate_id: "cand-b".into(),
                        observable_pull_bins: Default::default(),
                        mechanism_fraction: Default::default(),
                        quarantined: true,
                        quarantine_reason: "phenom-only primitive".into(),
                    },
                    whitebox_score: 0.0,
                    phenom_score: 85.0,
                    claimed_bounty_ids: vec![],
                    structural_gate_passed: false,
                },
            ],
            claimed_bounties: vec![],
            lessons_applied: vec![],
        };
        let best = receipt.best_whitebox().unwrap();
        assert_eq!(best.candidate_id, "cand-a");
        assert!(best.structural_gate_passed);
    }
}
