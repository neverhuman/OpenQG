//! V8 Phase 22 (#13): ScreeningPlausibilityCheck — structural consistency gate for declared
//! screening mechanisms vs. the theory's α-basis parameters.
//!
//! ## The problem (SYNTHESIS #13, S03 §4)
//!
//! `Theory.screening_recovery` is a proposer-asserted float with no machine check. A proposer
//! can claim "Vainshtein screening, recovery = 1.0" for a theory with α_B ≈ 0 — but Vainshtein
//! screening is driven by braiding, so zero braiding means the mechanism cannot operate. Similarly,
//! a theory with large α_M but no declared screening fails the Cassini PPN γ bound (Bertotti,
//! Iess & Tortora 2003).
//!
//! ## What this gate checks
//!
//! | Declared mechanism | Required α-basis condition | Reason |
//! |---|---|---|
//! | `"vainshtein"` | \|α_B\| > min_braiding | Vainshtein is braiding-driven |
//! | `"chameleon"` | any (mass-dependent; structural check only) | OK as structural claim |
//! | `"symmetron"` | any (density-threshold; structural check only) | OK as structural claim |
//! | `"k-mouflage"` | α_K > min_kineticity | k-mouflage requires active kineticity |
//! | none declared | \|α_M\| < cassini_alpha_m_threshold | Large α_M without screening fails Cassini |
//!
//! The gate returns `ScreeningPlausibilityOutcome` with a pass/fail verdict and a reason.

use serde::{Deserialize, Serialize};

/// Minimum |α_B| for Vainshtein screening to be physically operative.
/// Below this braiding is negligible and Vainshtein suppression is exponentially small.
pub const VAINSHTEIN_MIN_BRAIDING: f64 = 0.01;

/// Minimum α_K for k-mouflage screening to be physically operative.
pub const KMOUFLAGE_MIN_KINETICITY: f64 = 0.01;

/// Maximum |α_M| without declared screening before the Cassini PPN γ bound is violated.
/// Cassini: |γ − 1| = (2.1 ± 2.3)×10⁻⁵. A running Planck mass with |α_M| > 0.1 and no
/// screening mechanism produces |γ − 1| ~ |α_M| >> 10⁻⁵ (fifth-force constraint).
pub const CASSINI_ALPHA_M_THRESHOLD: f64 = 0.1;

/// Outcome of the screening plausibility gate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreeningVerdict {
    /// The declared screening mechanism is consistent with the α-basis parameters.
    Plausible,
    /// The declared mechanism cannot operate given the theory's α-basis (or the Cassini bound
    /// is violated). The theory must be re-evaluated with a different screening claim.
    Implausible,
}

/// Full outcome from the screening plausibility gate, including diagnostics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScreeningPlausibilityOutcome {
    /// The declared screening mechanism string (e.g. `"vainshtein"`, `"chameleon"`), or `None`.
    pub declared_mechanism: Option<String>,
    /// α_M used for the Cassini check.
    pub alpha_m: f64,
    /// α_B used for the Vainshtein check.
    pub alpha_b: f64,
    /// α_K used for the k-mouflage check.
    pub alpha_k: f64,
    /// Gate verdict.
    pub verdict: ScreeningVerdict,
    /// Human-readable explanation of the verdict.
    pub reason: String,
}

impl ScreeningPlausibilityOutcome {
    /// True when the screening claim is physically plausible.
    pub fn is_plausible(&self) -> bool {
        self.verdict == ScreeningVerdict::Plausible
    }

    /// True when the gate blocks the theory.
    pub fn is_implausible(&self) -> bool {
        self.verdict == ScreeningVerdict::Implausible
    }
}

/// Run the screening plausibility gate.
///
/// # Arguments
/// - `declared_mechanism`: the `Theory.screening` field (e.g. `Some("vainshtein")`).
/// - `alpha_m`: Planck-mass run rate α_M from the theory's `AlphaBasis`.
/// - `alpha_b`: braiding coefficient α_B.
/// - `alpha_k`: kineticity α_K.
pub fn check_screening_plausibility(
    declared_mechanism: Option<&str>,
    alpha_m: f64,
    alpha_b: f64,
    alpha_k: f64,
) -> ScreeningPlausibilityOutcome {
    let mechanism_lower = declared_mechanism.map(|s| s.to_ascii_lowercase());
    let (verdict, reason) = match mechanism_lower.as_deref() {
        Some(m) if m.contains("vainshtein") => {
            if alpha_b.abs() >= VAINSHTEIN_MIN_BRAIDING {
                (
                    ScreeningVerdict::Plausible,
                    format!(
                        "Vainshtein screening is braiding-driven: |α_B| = {:.4} ≥ {VAINSHTEIN_MIN_BRAIDING} — mechanism can operate",
                        alpha_b.abs()
                    ),
                )
            } else {
                (
                    ScreeningVerdict::Implausible,
                    format!(
                        "Vainshtein screening requires |α_B| ≥ {VAINSHTEIN_MIN_BRAIDING}, \
                         but |α_B| = {:.4}; braiding is negligible and Vainshtein suppression \
                         cannot operate",
                        alpha_b.abs()
                    ),
                )
            }
        }
        Some(m) if m.contains("kmouflage") || m.contains("k-mouflage") => {
            if alpha_k >= KMOUFLAGE_MIN_KINETICITY {
                (
                    ScreeningVerdict::Plausible,
                    format!(
                        "k-mouflage screening is kineticity-driven: α_K = {alpha_k:.4} ≥ {KMOUFLAGE_MIN_KINETICITY}"
                    ),
                )
            } else {
                (
                    ScreeningVerdict::Implausible,
                    format!(
                        "k-mouflage requires α_K ≥ {KMOUFLAGE_MIN_KINETICITY}, \
                         but α_K = {alpha_k:.4}; kineticity is negligible"
                    ),
                )
            }
        }
        Some(m) if m.contains("chameleon") || m.contains("symmetron") => {
            // Structural claim only — chameleon/symmetron depend on local density threshold,
            // not on the α-basis. Accept as structurally plausible; value-level check requires
            // the Boltzmann lane.
            (
                ScreeningVerdict::Plausible,
                format!(
                    "'{m}' is a density-threshold mechanism; α-basis structural check passes \
                     (value-level verification requires Boltzmann escalation)"
                ),
            )
        }
        Some(m) => {
            // Unknown mechanism — accept at the structural level; flag for operator review.
            (
                ScreeningVerdict::Plausible,
                format!(
                    "unrecognised screening mechanism '{m}'; accepting as structural claim \
                     pending Boltzmann-tier verification"
                ),
            )
        }
        None => {
            // No declared screening. If α_M is large, the Cassini bound is likely violated.
            if alpha_m.abs() >= CASSINI_ALPHA_M_THRESHOLD {
                (
                    ScreeningVerdict::Implausible,
                    format!(
                        "no screening mechanism declared but |α_M| = {:.4} ≥ {CASSINI_ALPHA_M_THRESHOLD}; \
                         a running Planck mass at this level produces |γ − 1| ~ |α_M| ≫ 10⁻⁵ \
                         (Cassini PPN γ bound). Declare a screening mechanism or set α_M ≈ 0.",
                        alpha_m.abs()
                    ),
                )
            } else {
                (
                    ScreeningVerdict::Plausible,
                    format!(
                        "no screening declared; |α_M| = {:.4} < {CASSINI_ALPHA_M_THRESHOLD} — \
                         Cassini bound not threatened at this modification level",
                        alpha_m.abs()
                    ),
                )
            }
        }
    };

    ScreeningPlausibilityOutcome {
        declared_mechanism: declared_mechanism.map(|s| s.to_string()),
        alpha_m,
        alpha_b,
        alpha_k,
        verdict,
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Vainshtein ----

    #[test]
    fn vainshtein_with_sufficient_braiding_passes() {
        let o = check_screening_plausibility(Some("vainshtein"), 0.3, 0.5, 0.1);
        assert_eq!(o.verdict, ScreeningVerdict::Plausible);
        assert!(o.is_plausible());
    }

    #[test]
    fn vainshtein_without_braiding_fails() {
        let o = check_screening_plausibility(Some("vainshtein"), 0.3, 0.0, 0.1);
        assert_eq!(o.verdict, ScreeningVerdict::Implausible);
        assert!(o.is_implausible());
        assert!(o.reason.contains("braiding"));
    }

    #[test]
    fn vainshtein_at_min_braiding_boundary_passes() {
        let o = check_screening_plausibility(Some("Vainshtein"), 0.3, VAINSHTEIN_MIN_BRAIDING, 0.0);
        assert_eq!(o.verdict, ScreeningVerdict::Plausible);
    }

    // ---- k-mouflage ----

    #[test]
    fn kmouflage_with_kineticity_passes() {
        let o = check_screening_plausibility(Some("k-mouflage"), 0.2, 0.0, 0.5);
        assert_eq!(o.verdict, ScreeningVerdict::Plausible);
    }

    #[test]
    fn kmouflage_without_kineticity_fails() {
        let o = check_screening_plausibility(Some("kmouflage"), 0.2, 0.3, 0.0);
        assert_eq!(o.verdict, ScreeningVerdict::Implausible);
        assert!(o.reason.contains("kineticity"));
    }

    // ---- chameleon / symmetron ----

    #[test]
    fn chameleon_always_passes_structural_check() {
        let o = check_screening_plausibility(Some("chameleon"), 0.5, 0.0, 0.0);
        assert_eq!(o.verdict, ScreeningVerdict::Plausible);
        assert!(o.reason.contains("density-threshold"));
    }

    #[test]
    fn symmetron_always_passes_structural_check() {
        let o = check_screening_plausibility(Some("symmetron"), 0.3, 0.0, 0.0);
        assert_eq!(o.verdict, ScreeningVerdict::Plausible);
    }

    // ---- no screening declared ----

    #[test]
    fn no_screening_small_alpha_m_passes() {
        let o = check_screening_plausibility(None, 0.05, 0.0, 0.0);
        assert_eq!(o.verdict, ScreeningVerdict::Plausible);
    }

    #[test]
    fn no_screening_large_alpha_m_fails_cassini() {
        let o = check_screening_plausibility(None, 0.2, 0.0, 0.0);
        assert_eq!(o.verdict, ScreeningVerdict::Implausible);
        assert!(o.reason.contains("Cassini"));
    }

    #[test]
    fn no_screening_gr_limit_passes() {
        let o = check_screening_plausibility(None, 0.0, 0.0, 0.0);
        assert_eq!(o.verdict, ScreeningVerdict::Plausible);
    }

    #[test]
    fn negative_alpha_m_magnitude_checked() {
        let o = check_screening_plausibility(None, -0.15, 0.0, 0.0);
        assert_eq!(
            o.verdict,
            ScreeningVerdict::Implausible,
            "|α_M| = 0.15 should fail"
        );
    }

    // ---- unknown mechanism ----

    #[test]
    fn unknown_mechanism_passes_with_note() {
        let o = check_screening_plausibility(Some("novel-screening"), 0.3, 0.0, 0.0);
        assert_eq!(o.verdict, ScreeningVerdict::Plausible);
        assert!(o.reason.contains("unrecognised"));
    }

    // ---- serde ----

    #[test]
    fn outcome_serde_round_trip() {
        let o = check_screening_plausibility(Some("vainshtein"), 0.3, 0.5, 0.1);
        let json = serde_json::to_string(&o).unwrap();
        let back: ScreeningPlausibilityOutcome = serde_json::from_str(&json).unwrap();
        assert_eq!(back, o);
    }

    #[test]
    fn declared_mechanism_stored_in_outcome() {
        let o = check_screening_plausibility(Some("chameleon"), 0.0, 0.0, 0.0);
        assert_eq!(o.declared_mechanism.as_deref(), Some("chameleon"));
    }
}
