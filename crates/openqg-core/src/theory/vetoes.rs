//! Deterministic physics-veto cascade: cheap, structural *necessary conditions* every theory
//! must satisfy. Failing any one is a hard kill regardless of how well the theory fits data —
//! this is what makes the engine chase derivable physics instead of better curve fits.
//!
//! Ordered cheapest-first (each is O(1)–O(terms)), so the vast majority of degenerate candidates
//! die here for almost no cost and never reach the forward model or the LLM critic. The checks
//! follow `docs/research/automated-theory-discovery.md` §4 and
//! `docs/research/forward-model-and-unification.md` §4:
//! dimensional homogeneity → Lorentz invariance → parameter provenance (whitebox gate) →
//! GW170817 tensor speed → ghost/Ostrogradsky → gradient stability → PPN screening.

use super::{Provenance, Theory};

/// GW170817 bound: c_GW = c forces the tensor-speed excess α_T to ~0. We allow a 1% structural
/// tolerance (the measured bound is ~1e-15; an evolved candidate with |α_T| above a percent is
/// unambiguously ruled out and its G4(X)/G5 sector must be removed).
const ALPHA_T_TOLERANCE: f64 = 1e-2;
/// Above this linear-modification scale a theory measurably alters gravity and MUST declare a
/// screening mechanism to survive solar-system (PPN) tests.
const SCREENING_REQUIRED_SCALE: f64 = 1e-3;

/// A reason a theory was vetoed. `Vec<VetoReason>` empty ⇒ the theory passes the cascade.
#[derive(Debug, Clone, PartialEq)]
pub enum VetoReason {
    /// A Lagrangian-density term is not mass-dimension 4 (action not dimensionless).
    DimensionalInhomogeneity { term: String, mass_dimension: i32 },
    /// A term carries uncontracted Lorentz indices (not a scalar under the Lorentz group).
    UncontractedLorentzIndex { term: String, free_indices: u32 },
    /// A free fitting parameter (the gray-box trap).
    FreeParameter { symbol: String },
    /// A `Derived` parameter whose mechanism note is empty (provenance not machine-checkable).
    UnprovenancedParameter { symbol: String },
    /// A `Derived` parameter that carries a value-level [`DerivedCertificate`](super::DerivedCertificate)
    /// which FAILED verification (the claimed number does not equal the cited closed form). The
    /// parameter is treated as `Free` and killed — this is the M1 "derived, not fit" value-level gate.
    FailedDerivationCertificate {
        symbol: String,
        relation: String,
        detail: String,
    },
    /// A `Derived` parameter with NO certificate: it still passes (text-checked, today's behavior)
    /// but is flagged so the critic/ledger knows the value was never machine-verified. This variant
    /// is a *diagnostic*, not a kill — `is_vetoed` ignores it (see [`VetoReason::is_kill`]).
    UncertifiedDerivedParameter { symbol: String },
    /// Tensor speed excess incompatible with GW170817.
    GravitationalWaveSpeed { alpha_t: f64 },
    /// Wrong-sign kinetic term or non-positive no-ghost determinant (negative-norm ghost).
    Ghost { kinetic_coefficient: f64, q_s: f64 },
    /// Non-degenerate higher time derivatives ⇒ Ostrogradsky instability.
    OstrogradskyGhost,
    /// Negative scalar sound speed squared ⇒ gradient instability.
    GradientInstability { sound_speed_sq: f64 },
    /// Modifies gravity on linear scales but declares no screening to recover GR at the solar
    /// system (would violate the Cassini PPN γ bound).
    MissingScreening { modification_scale: f64 },
}

impl VetoReason {
    /// Whether this reason is a hard kill. All structural violations are kills; the M1
    /// `UncertifiedDerivedParameter` is a non-fatal *diagnostic* (today's text-only behavior is
    /// preserved) and is the sole non-kill reason.
    pub fn is_kill(&self) -> bool {
        !matches!(self, VetoReason::UncertifiedDerivedParameter { .. })
    }
}

/// Run the full deterministic veto cascade. Returns every **kill** reason found (empty ⇒ passes);
/// any non-empty result is a hard kill. Non-fatal diagnostics (the M1 "uncertified derived"
/// flag) are filtered out here to preserve the historical `.is_empty()` ⇒ "passes" contract — use
/// [`run_veto_cascade_full`] to also see diagnostics.
pub fn run_veto_cascade(theory: &Theory) -> Vec<VetoReason> {
    run_veto_cascade_full(theory)
        .into_iter()
        .filter(VetoReason::is_kill)
        .collect()
}

/// Run the cascade and return every reason, *including* non-fatal diagnostics (e.g. an uncertified
/// derived parameter). Useful for the critic/ledger; use [`run_veto_cascade`] for the kill verdict.
pub fn run_veto_cascade_full(theory: &Theory) -> Vec<VetoReason> {
    let mut reasons = Vec::new();

    // 1. Dimensional homogeneity: every Lagrangian-density term must be mass-dimension 4.
    for term in &theory.terms {
        if term.mass_dimension != 4 {
            reasons.push(VetoReason::DimensionalInhomogeneity {
                term: term.name.clone(),
                mass_dimension: term.mass_dimension,
            });
        }
    }

    // 2. Lorentz invariance: a scalar action term has no uncontracted indices.
    for term in &theory.terms {
        if term.free_lorentz_indices != 0 {
            reasons.push(VetoReason::UncontractedLorentzIndex {
                term: term.name.clone(),
                free_indices: term.free_lorentz_indices,
            });
        }
    }

    // 3. Whitebox provenance: no free / unprovenanced parameters (machine-checkable, not text), and
    //    — M1 — a `Derived` value with a FAILING certificate is treated as `Free` (killed); a
    //    `Derived` with no certificate keeps today's text-only behavior but emits a diagnostic.
    for p in &theory.parameters {
        match &p.provenance {
            Provenance::Free => reasons.push(VetoReason::FreeParameter {
                symbol: p.symbol.clone(),
            }),
            Provenance::Derived { mechanism, .. } if mechanism.trim().is_empty() => {
                reasons.push(VetoReason::UnprovenancedParameter {
                    symbol: p.symbol.clone(),
                })
            }
            Provenance::Derived {
                certificate: Some(cert),
                ..
            } => {
                // Value-level gate: recompute the value from cited closed-form inputs.
                match cert.check() {
                    super::certificate::CertificateOutcome::Verified { .. } => {}
                    other => reasons.push(VetoReason::FailedDerivationCertificate {
                        symbol: p.symbol.clone(),
                        relation: cert.relation.clone(),
                        detail: format!("{other:?}"),
                    }),
                }
            }
            Provenance::Derived {
                certificate: None, ..
            } => {
                // Text-checked but never value-verified: diagnostic only (not a kill).
                reasons.push(VetoReason::UncertifiedDerivedParameter {
                    symbol: p.symbol.clone(),
                })
            }
            _ => {}
        }
    }

    // 4. GW170817: tensor speed excess must vanish.
    if theory.alpha.alpha_t.abs() > ALPHA_T_TOLERANCE {
        reasons.push(VetoReason::GravitationalWaveSpeed {
            alpha_t: theory.alpha.alpha_t,
        });
    }

    // 5. Ghosts: kinetic term and no-ghost determinant must be positive.
    let s = &theory.stability;
    if s.kinetic_coefficient <= 0.0 || s.q_s <= 0.0 {
        reasons.push(VetoReason::Ghost {
            kinetic_coefficient: s.kinetic_coefficient,
            q_s: s.q_s,
        });
    }
    if s.has_nondegenerate_higher_derivatives {
        reasons.push(VetoReason::OstrogradskyGhost);
    }

    // 6. Gradient stability: non-negative sound speed squared.
    if s.sound_speed_sq < 0.0 {
        reasons.push(VetoReason::GradientInstability {
            sound_speed_sq: s.sound_speed_sq,
        });
    }

    // 7. PPN screening: a gravity-modifying theory must screen at solar-system scales.
    let scale = theory.alpha.modification_scale();
    if scale > SCREENING_REQUIRED_SCALE && theory.screening.is_none() {
        reasons.push(VetoReason::MissingScreening {
            modification_scale: scale,
        });
    }

    reasons
}

/// Convenience: true if the theory is killed by any veto.
pub fn is_vetoed(theory: &Theory) -> bool {
    !run_veto_cascade(theory).is_empty()
}

#[cfg(test)]
mod tests {
    use super::super::{AlphaBasis, Parameter, Provenance, Stability, Term, Theory};
    use super::*;

    fn baseline() -> Theory {
        Theory::baseline_lcdm()
    }

    #[test]
    fn baseline_passes_every_veto() {
        assert!(run_veto_cascade(&baseline()).is_empty());
        assert!(!is_vetoed(&baseline()));
    }

    #[test]
    fn free_parameter_is_killed() {
        let mut t = baseline();
        t.parameters.push(Parameter {
            symbol: "f_ede".into(),
            value: 0.07,
            physical_meaning: "early dark energy fraction".into(),
            provenance: Provenance::Free,
        });
        assert!(run_veto_cascade(&t)
            .iter()
            .any(|r| matches!(r, VetoReason::FreeParameter { symbol } if symbol == "f_ede")));
    }

    #[test]
    fn unprovenanced_derived_parameter_is_killed() {
        let mut t = baseline();
        t.parameters.push(Parameter {
            symbol: "xi_dm".into(),
            value: 0.1,
            physical_meaning: "dark-sector coupling".into(),
            provenance: Provenance::Derived {
                mechanism: "   ".into(),
                certificate: None,
            },
        });
        assert!(run_veto_cascade(&t)
            .iter()
            .any(|r| matches!(r, VetoReason::UnprovenancedParameter { .. })));
    }

    #[test]
    fn gw170817_kills_tensor_speed_excess() {
        let mut t = baseline();
        t.alpha.alpha_t = 0.3;
        assert!(run_veto_cascade(&t)
            .iter()
            .any(|r| matches!(r, VetoReason::GravitationalWaveSpeed { .. })));
    }

    #[test]
    fn ghost_and_gradient_instabilities_are_killed() {
        let mut ghost = baseline();
        ghost.stability.kinetic_coefficient = -1.0;
        assert!(run_veto_cascade(&ghost)
            .iter()
            .any(|r| matches!(r, VetoReason::Ghost { .. })));

        let mut grad = baseline();
        grad.stability.sound_speed_sq = -0.2;
        assert!(run_veto_cascade(&grad)
            .iter()
            .any(|r| matches!(r, VetoReason::GradientInstability { .. })));
    }

    #[test]
    fn ostrogradsky_higher_derivatives_are_killed() {
        let mut t = baseline();
        t.stability.has_nondegenerate_higher_derivatives = true;
        assert!(run_veto_cascade(&t)
            .iter()
            .any(|r| matches!(r, VetoReason::OstrogradskyGhost)));
    }

    #[test]
    fn modifying_gravity_without_screening_is_killed_but_with_screening_passes() {
        let mut no_screen = baseline();
        no_screen.alpha = AlphaBasis {
            alpha_m: 0.2,
            alpha_b: 0.1,
            alpha_k: 0.05,
            alpha_t: 0.0,
        };
        assert!(run_veto_cascade(&no_screen)
            .iter()
            .any(|r| matches!(r, VetoReason::MissingScreening { .. })));

        // Same modification, but with a declared screening mechanism, survives the PPN veto.
        let mut screened = no_screen.clone();
        screened.screening = Some("vainshtein".into());
        assert!(!run_veto_cascade(&screened)
            .iter()
            .any(|r| matches!(r, VetoReason::MissingScreening { .. })));
    }

    #[test]
    fn dimensional_and_lorentz_violations_are_killed() {
        let mut dim = baseline();
        dim.terms.push(Term {
            name: "bad_dim".into(),
            mass_dimension: 5,
            free_lorentz_indices: 0,
        });
        assert!(run_veto_cascade(&dim)
            .iter()
            .any(|r| matches!(r, VetoReason::DimensionalInhomogeneity { .. })));

        let mut lor = baseline();
        lor.terms.push(Term {
            name: "dangling_index".into(),
            mass_dimension: 4,
            free_lorentz_indices: 1,
        });
        assert!(run_veto_cascade(&lor)
            .iter()
            .any(|r| matches!(r, VetoReason::UncontractedLorentzIndex { .. })));
    }

    #[test]
    fn a_realistic_alpha_t_zero_screened_modified_gravity_survives() {
        // A GW170817-safe (α_T=0), screened, ghost-free modified-gravity theory with provenanced
        // parameters is exactly the kind of non-degenerate candidate the engine should keep.
        let mut t = Theory::baseline_lcdm();
        t.id = "screened-mg".into();
        t.alpha = AlphaBasis {
            alpha_m: 0.05,
            alpha_b: -0.03,
            alpha_k: 0.1,
            alpha_t: 0.0,
        };
        t.screening = Some("chameleon".into());
        t.stability = Stability {
            kinetic_coefficient: 0.8,
            q_s: 0.5,
            sound_speed_sq: 0.4,
            has_nondegenerate_higher_derivatives: false,
        };
        t.parameters.push(Parameter {
            symbol: "alpha_M0".into(),
            value: 0.05,
            physical_meaning: "Planck-mass run amplitude".into(),
            provenance: Provenance::Derived {
                mechanism: "conformal coupling beta in the chameleon potential".into(),
                certificate: None,
            },
        });
        assert!(
            run_veto_cascade(&t).is_empty(),
            "{:?}",
            run_veto_cascade(&t)
        );
    }

    use super::super::DerivedCertificate;

    /// Helper: a screened, GW-safe, ghost-free MG theory carrying one `Derived` α_M parameter whose
    /// provenance we vary per test.
    fn mg_with_alpha_m(provenance: Provenance) -> Theory {
        let mut t = Theory::baseline_lcdm();
        t.id = "mg-cert-test".into();
        t.alpha = AlphaBasis {
            alpha_m: 0.05,
            alpha_b: -0.03,
            alpha_k: 0.1,
            alpha_t: 0.0,
        };
        t.screening = Some("chameleon".into());
        t.stability = Stability {
            kinetic_coefficient: 0.8,
            q_s: 0.5,
            sound_speed_sq: 0.4,
            has_nondegenerate_higher_derivatives: false,
        };
        t.parameters.push(Parameter {
            symbol: "Geff_over_G".into(),
            value: 1.02,
            physical_meaning: "coupled-DE effective gravitational coupling".into(),
            provenance,
        });
        t
    }

    #[test]
    fn derived_with_passing_certificate_survives() {
        // β = 0.1 ⇒ coupled-DE G_eff/G = 1.02 — the certificate verifies, so the value-level gate
        // passes and the theory is not killed.
        let cert = DerivedCertificate {
            relation: "coupled_de_geff_over_g".into(),
            inputs: vec![("beta".into(), 0.1)],
            expected: 1.02,
            tolerance: 1e-9,
        };
        let t = mg_with_alpha_m(Provenance::derived_certified(
            "coupled-DE fifth force",
            cert,
        ));
        assert!(
            run_veto_cascade(&t).is_empty(),
            "{:?}",
            run_veto_cascade(&t)
        );
    }

    #[test]
    fn derived_with_failing_certificate_is_demoted_and_killed() {
        // Same relation/inputs, but the claimed value (1.5) is NOT 1.02 — the certificate fails, so
        // the parameter is treated as Free and the theory is killed.
        let cert = DerivedCertificate {
            relation: "coupled_de_geff_over_g".into(),
            inputs: vec![("beta".into(), 0.1)],
            expected: 1.5,
            tolerance: 1e-3,
        };
        let t = mg_with_alpha_m(Provenance::derived_certified(
            "coupled-DE fifth force",
            cert,
        ));
        let reasons = run_veto_cascade(&t);
        assert!(
            reasons.iter().any(|r| matches!(
                r,
                VetoReason::FailedDerivationCertificate { symbol, .. } if symbol == "Geff_over_G"
            )),
            "expected a failed-certificate kill, got {reasons:?}"
        );
        assert!(is_vetoed(&t));
    }

    #[test]
    fn derived_without_certificate_passes_but_emits_uncertified_diagnostic() {
        // Today's behavior is preserved: a text-only Derived param does NOT kill the theory...
        let t = mg_with_alpha_m(Provenance::derived("a stated mechanism"));
        assert!(
            run_veto_cascade(&t).is_empty(),
            "{:?}",
            run_veto_cascade(&t)
        );
        assert!(!is_vetoed(&t));
        // ...but the full cascade flags it as uncertified for the critic/ledger.
        assert!(run_veto_cascade_full(&t).iter().any(|r| matches!(
            r,
            VetoReason::UncertifiedDerivedParameter { symbol } if symbol == "Geff_over_G"
        )));
    }

    #[test]
    fn uncertified_diagnostic_is_not_a_kill() {
        let only_diag = VetoReason::UncertifiedDerivedParameter { symbol: "x".into() };
        assert!(!only_diag.is_kill());
        assert!(VetoReason::FreeParameter { symbol: "x".into() }.is_kill());
    }
}
