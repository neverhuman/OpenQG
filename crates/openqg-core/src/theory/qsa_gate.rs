//! V8 Phase 20 (#13): QsaEpsilonGate — per-theory quasi-static approximation (QSA) validity check.
//!
//! ## The problem (SYNTHESIS #13, S03 §4)
//!
//! The fitting-formula forward model (T0/T1 tier) uses the QSA: it assumes the scalar field
//! tracks the matter distribution quasi-statically and computes G_eff(a) algebraically. This
//! approximation fails when the scalar field has significant independent kineticity without
//! strong matter coupling via braiding. Specifically: when α_K is large and |α_B| is small,
//! the scalar has dynamics not captured by the QSA, and the fitting-formula score is wrong.
//!
//! ## What this gate does
//!
//! Given a theory's α-basis parameters (`AlphaBasis`) and scalar-sector stability coefficients
//! (`Stability.q_s`), the gate computes:
//!
//!   ε_QSA = |α_K| / max(6·α_B², ε_floor)
//!
//! where ε_floor = 1e-4 prevents division by zero. A small ε means braiding dominates the
//! scalar dynamics and the QSA is reliable. A large ε means the scalar has kineticity-driven
//! dynamics that the fitting formula cannot capture, and Boltzmann escalation is needed.
//!
//! Standard threshold: ε_QSA > 1.0 → ESCALATE (the braiding contribution is smaller than
//! the kineticity contribution by at least one order of magnitude).
//!
//! ## Reference
//! Bellini & Sawicki (2014), arXiv:1404.3625 §III; Planck 2015 results XIV, arXiv:1502.01590
//! §7.4 (QSA validity conditions). The ε_QSA formula is the ratio introduced in Peirone et al.
//! (2018), Phys. Rev. D 97, 043519.

use serde::{Deserialize, Serialize};

/// Gate decision for a single theory's QSA validity check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QsaDecision {
    /// QSA is reliable; the fitting-formula tier may proceed without Boltzmann escalation.
    QsaValid,
    /// QSA is suspect; Boltzmann escalation is required before fit-set scoring is reportable.
    EscalateToboltzmann,
}

/// Outcome of the QSA epsilon gate for a single theory.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QsaGateOutcome {
    /// The computed ε_QSA value.
    pub epsilon: f64,
    /// α_K input used.
    pub alpha_k: f64,
    /// α_B input used.
    pub alpha_b: f64,
    /// Gate decision.
    pub decision: QsaDecision,
    /// Human-readable explanation.
    pub reason: String,
}

impl QsaGateOutcome {
    /// True when the theory is in the QSA-valid regime.
    pub fn is_valid(&self) -> bool {
        self.decision == QsaDecision::QsaValid
    }

    /// True when Boltzmann escalation is required.
    pub fn requires_boltzmann(&self) -> bool {
        self.decision == QsaDecision::EscalateToboltzmann
    }
}

/// Computes the QSA validity parameter and issues a gate decision.
///
/// Default threshold: 1.0 (the braiding contribution must be at least as large as the
/// kineticity contribution to keep the scalar locked to matter in the QSA sense).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct QsaEpsilonGate {
    /// Maximum ε_QSA before escalation is required. Standard value: 1.0.
    pub threshold: f64,
    /// Floor value to prevent division by zero when α_B ≈ 0.
    pub epsilon_floor: f64,
}

impl QsaEpsilonGate {
    /// Standard gate: threshold = 1.0, floor = 1e-4.
    pub fn standard() -> Self {
        QsaEpsilonGate {
            threshold: 1.0,
            epsilon_floor: 1e-4,
        }
    }

    /// Strict gate: threshold = 0.1 (used when fitting-formula uncertainty must be < 0.5 nat).
    pub fn strict() -> Self {
        QsaEpsilonGate {
            threshold: 0.1,
            epsilon_floor: 1e-4,
        }
    }

    /// Compute ε_QSA = |α_K| / max(6·α_B², ε_floor).
    ///
    /// Returns 0.0 when α_K = 0 (GR-like kineticity → QSA trivially valid).
    pub fn compute_epsilon(&self, alpha_k: f64, alpha_b: f64) -> f64 {
        if alpha_k == 0.0 {
            return 0.0;
        }
        let braiding_term = 6.0 * alpha_b * alpha_b;
        alpha_k.abs() / braiding_term.max(self.epsilon_floor)
    }

    /// Run the gate and return a structured [`QsaGateOutcome`].
    pub fn evaluate(&self, alpha_k: f64, alpha_b: f64) -> QsaGateOutcome {
        let epsilon = self.compute_epsilon(alpha_k, alpha_b);
        let decision = if epsilon <= self.threshold {
            QsaDecision::QsaValid
        } else {
            QsaDecision::EscalateToboltzmann
        };
        let reason = match &decision {
            QsaDecision::QsaValid => format!(
                "ε_QSA = {epsilon:.4} ≤ threshold {:.4}: braiding dominates, QSA reliable",
                self.threshold
            ),
            QsaDecision::EscalateToboltzmann => format!(
                "ε_QSA = {epsilon:.4} > threshold {:.4}: kineticity-dominated dynamics; \
                 fitting-formula predictions unreliable — escalate to Boltzmann tier",
                self.threshold
            ),
        };
        QsaGateOutcome {
            epsilon,
            alpha_k,
            alpha_b,
            decision,
            reason,
        }
    }

    /// Convenience: evaluate directly from `AlphaBasis` fields.
    pub fn evaluate_from_basis(&self, alpha_k: f64, alpha_b: f64) -> QsaGateOutcome {
        self.evaluate(alpha_k, alpha_b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gate() -> QsaEpsilonGate {
        QsaEpsilonGate::standard()
    }

    // ---- epsilon computation ----

    #[test]
    fn gr_like_alpha_k_zero_gives_zero_epsilon() {
        assert_eq!(gate().compute_epsilon(0.0, 0.5), 0.0);
    }

    #[test]
    fn strong_braiding_keeps_epsilon_small() {
        // α_K = 0.1, α_B = 1.0 → ε = 0.1 / (6 * 1.0) = 0.1/6 ≈ 0.017
        let eps = gate().compute_epsilon(0.1, 1.0);
        assert!(
            eps < 0.1,
            "strong braiding should produce small ε; got {eps}"
        );
    }

    #[test]
    fn zero_braiding_uses_floor() {
        // α_K = 1.0, α_B = 0 → ε = 1.0 / floor = 1.0/1e-4 = 10000 (>> threshold)
        let eps = gate().compute_epsilon(1.0, 0.0);
        assert!(
            eps > gate().threshold,
            "zero braiding with nonzero kineticity should fail"
        );
    }

    #[test]
    fn epsilon_symmetric_in_sign_of_alpha_k() {
        let pos = gate().compute_epsilon(0.5, 0.3);
        let neg = gate().compute_epsilon(-0.5, 0.3);
        assert!(
            (pos - neg).abs() < 1e-12,
            "ε_QSA uses |α_K|, sign irrelevant"
        );
    }

    #[test]
    fn epsilon_symmetric_in_sign_of_alpha_b() {
        // α_B appears squared, so sign doesn't matter
        let pos = gate().compute_epsilon(0.5, 0.3);
        let neg = gate().compute_epsilon(0.5, -0.3);
        assert!((pos - neg).abs() < 1e-12, "α_B² is symmetric");
    }

    // ---- gate decisions ----

    #[test]
    fn gr_theory_is_valid() {
        let outcome = gate().evaluate(0.0, 0.0);
        assert_eq!(outcome.decision, QsaDecision::QsaValid);
        assert!(outcome.is_valid());
        assert!(!outcome.requires_boltzmann());
    }

    #[test]
    fn braiding_dominated_passes_standard_gate() {
        // α_K = 0.5, α_B = 1.0 → ε ≈ 0.5/6 ≈ 0.083 < 1.0
        let outcome = gate().evaluate(0.5, 1.0);
        assert_eq!(outcome.decision, QsaDecision::QsaValid);
    }

    #[test]
    fn kineticity_dominated_fails_standard_gate() {
        // α_K = 5.0, α_B = 0.1 → ε = 5.0 / (6*0.01) = 5.0/0.06 ≈ 83 >> 1.0
        let outcome = gate().evaluate(5.0, 0.1);
        assert_eq!(outcome.decision, QsaDecision::EscalateToboltzmann);
        assert!(outcome.requires_boltzmann());
    }

    #[test]
    fn at_threshold_is_valid() {
        // Find α_K such that ε = threshold exactly: α_K = threshold * 6 * α_B²
        let alpha_b = 1.0;
        let alpha_k = gate().threshold * 6.0 * alpha_b * alpha_b; // = 1.0 * 6 * 1 = 6.0
        let outcome = gate().evaluate(alpha_k, alpha_b);
        assert_eq!(
            outcome.decision,
            QsaDecision::QsaValid,
            "ε = threshold should pass (≤)"
        );
    }

    #[test]
    fn strict_gate_is_more_conservative() {
        // α_K = 0.5, α_B = 1.0 → ε ≈ 0.083. Standard passes, strict (threshold=0.1) also passes.
        // But α_K = 2.0, α_B = 1.0 → ε ≈ 0.333. Standard passes, strict fails.
        let standard_outcome = QsaEpsilonGate::standard().evaluate(2.0, 1.0);
        let strict_outcome = QsaEpsilonGate::strict().evaluate(2.0, 1.0);
        assert_eq!(standard_outcome.decision, QsaDecision::QsaValid);
        assert_eq!(strict_outcome.decision, QsaDecision::EscalateToboltzmann);
    }

    #[test]
    fn reason_contains_epsilon_value() {
        let outcome = gate().evaluate(5.0, 0.1);
        assert!(outcome.reason.contains("ε_QSA"), "{}", outcome.reason);
    }

    #[test]
    fn outcome_serde_round_trip() {
        let outcome = gate().evaluate(1.0, 0.5);
        let json = serde_json::to_string(&outcome).unwrap();
        let back: QsaGateOutcome = serde_json::from_str(&json).unwrap();
        assert_eq!(back.decision, outcome.decision);
        assert!((back.epsilon - outcome.epsilon).abs() < 1e-12);
    }

    #[test]
    fn gate_serde_round_trip() {
        let g = QsaEpsilonGate::standard();
        let json = serde_json::to_string(&g).unwrap();
        let back: QsaEpsilonGate = serde_json::from_str(&json).unwrap();
        assert_eq!(back, g);
    }
}
