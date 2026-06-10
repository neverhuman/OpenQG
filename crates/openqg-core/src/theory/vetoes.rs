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

use super::obligation::{DerivationObligation, DerivationObligationKind, ObligationOutcome};
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

    // --- Adjudication-only reasons (computed physics, not metadata labels) ---
    /// ADJUDICATION: the tensor speed *recomputed at the GW170817 source epoch* violates the real
    /// multi-messenger bound |c_T/c − 1| ≲ 3×10⁻¹⁵ (Abbott et al. 2017). This is the value-level
    /// check that replaces the 1e-2 metadata triage gate — a candidate can carry `alpha_t = 0`
    /// *today* yet still violate the bound once α_T is evolved to the source redshift.
    TensorSpeedAtSource {
        /// Redshift of the GW170817 host (NGC 4993).
        z_source: f64,
        /// |c_T/c − 1| evaluated at `z_source`.
        ct_excess: f64,
        /// The bound it exceeded.
        bound: f64,
    },
    /// ADJUDICATION: the theory *declares* a screening mechanism (non-empty `screening` string)
    /// but supplies no numeric `screening_recovery`, so its solar-system PPN deviation cannot be
    /// computed. A bare string is not a physics claim — adjudication rejects it.
    ScreeningRecoveryUnquantified { mechanism: String },
    /// ADJUDICATION: the numeric screening recovery is out of the physical `[0, 1]` range, so it
    /// does not describe a real suppression efficiency.
    ScreeningRecoveryUnphysical { recovery: f64 },
    /// ADJUDICATION: the *recomputed* residual solar-system PPN deviation `(γ−1)_pred =
    /// modification_scale · (1 − recovery)` exceeds the Cassini bound |γ−1| ≲ 2.3×10⁻⁵ — the
    /// declared screening does not actually recover GR tightly enough.
    ScreeningInsufficientPpn {
        /// Predicted residual |γ−1| at the solar system.
        predicted_gamma_minus_one: f64,
        /// The Cassini 1σ bound it exceeded.
        bound: f64,
    },
    /// ADJUDICATION: the theory's own background is unphysical (the Friedmann sum E(z)² went
    /// negative at some probed redshift) — recomputed, not clamped.
    UnphysicalBackground { z: f64, e_squared: f64 },

    // --- V4 derivation-obligation reasons (the typed "derived, not asserted" gate) ---
    /// A derivation obligation FAILED verification — the claimed derivation does not check out
    /// (e.g. a numeric/symbolic witness recomputes a different value, or a literature attestation
    /// is missing). Carries the owning claim/obligation id, the obligation kind, and a detail.
    UnverifiedDerivation {
        obligation: String,
        kind: String,
        detail: String,
    },
    /// A required physical-*limit* obligation FAILED — the theory does not recover its reference
    /// (GR/QM/QFT/SM/ΛCDM) in the stated limit to within the bound.
    LimitFailure { obligation: String, detail: String },

    // --- V5 truth-binding reasons (claims must have computable consequences) ---
    /// The theory is distinct from ΛCDM via verified certificates, but the bound background still
    /// computes GR growth (an unbindable relation or an inversion domain error) — the claimed
    /// modification has no computable consequence, which is exactly the credit-without-risk
    /// arbitrage V5 closes.
    UnimplementedModification {
        relation: String,
        symbol: String,
        detail: String,
    },
    /// A non-GR modified-gravity background field is supported by NO verified MG certificate — an
    /// uncertified modification is a fitting knob in disguise (FreeParameter severity).
    UnexplainedModification { field: String, value: f64 },
    /// Relation-derived and declared values for the same background field disagree — the theory
    /// asserts two inconsistent values of one physical constant.
    ConflictingModification {
        field: String,
        certificate_value: f64,
        declared_value: f64,
    },
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

/// V4 derivation-obligation gate: run each [`DerivationObligation`] through its oracle and turn any
/// FAILED obligation into a kill veto ([`VetoReason::LimitFailure`] for a `Limit` obligation, else
/// [`VetoReason::UnverifiedDerivation`]). `Unsupported` obligations (e.g. a recorded-but-unchecked
/// Positivstellensatz/Lean stub) are NOT a kill here — they simply earn no rigor credit; the
/// separate "every physics claim must carry ≥1 obligation" rule is enforced at the ClaimGraph /
/// scorecard layer ([`super::ClaimGraph::unobligated_physics_claims`]). Empty input ⇒ no vetoes.
pub fn obligation_vetoes(obligations: &[DerivationObligation]) -> Vec<VetoReason> {
    let mut reasons = Vec::new();
    for o in obligations {
        if let ObligationOutcome::Failed { detail, .. } = o.check() {
            match o.kind {
                DerivationObligationKind::Limit => reasons.push(VetoReason::LimitFailure {
                    obligation: o.claim_id.clone(),
                    detail,
                }),
                _ => reasons.push(VetoReason::UnverifiedDerivation {
                    obligation: o.claim_id.clone(),
                    kind: format!("{:?}", o.kind),
                    detail,
                }),
            }
        }
    }
    reasons
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

    // 8. V5 truth-binding: certified modifications must bind into a background the forward model
    //    can compute (unbindable / conflicting / unexplained modifications are kills). Pure
    //    algebra — no ODE solve — cheap enough for triage.
    reasons.extend(super::binding::bind_modified_background(theory).vetoes);

    reasons
}

/// Convenience: true if the theory is killed by any veto.
pub fn is_vetoed(theory: &Theory) -> bool {
    !run_veto_cascade(theory).is_empty()
}

// =====================================================================================
// Triage vs adjudication (M3).
//
// `run_veto_cascade` above is **triage**: O(1) checks on candidate-supplied *metadata* (labels,
// declared stability, a declared screening *string*), cheap enough to run on every proposal in the
// search loop. Its tensor-speed gate uses a deliberately loose 1e-2 tolerance — a *search
// warning*, not the physics bound — so promising candidates are not pruned before the expensive
// stage.
//
// `adjudicate` below **recomputes physics** instead of trusting those labels. It is the gate a
// candidate must clear to be promotable: the real ~10⁻¹⁵ GW170817 tensor-speed bound evaluated at
// the source epoch, and a real solar-system PPN check derived from a *numeric* screening-recovery
// field (a declared `screening` string with no number is rejected). All bounds are cited inline.
// =====================================================================================

/// Redshift of the GW170817 host galaxy NGC 4993 (heliocentric z_helio = 0.009783 ± 0.000023;
/// cosmic z ≈ 0.0099). Source: Hjorth et al. 2017, ApJL 848 L31 (arXiv:1710.05856). The tensor
/// speed must satisfy the multi-messenger bound *at the epoch the signal was emitted*.
const GW170817_SOURCE_REDSHIFT: f64 = 0.0099;

/// GW170817 + GRB 170817A tensor-speed bound: from the (+1.74 ± 0.05) s GW–GRB time lag,
/// −3×10⁻¹⁵ ≤ (c_gw − c)/c ≤ +7×10⁻¹⁶ (Abbott et al. 2017, ApJL 848 L13, arXiv:1710.05834). We use
/// the larger (negative-side) magnitude, 3×10⁻¹⁵, as a symmetric |c_T/c − 1| ceiling.
const GW170817_CT_EXCESS_BOUND: f64 = 3.0e-15;

/// Cassini solar-system PPN bound: γ − 1 = (2.1 ± 2.3)×10⁻⁵ (Bertotti, Iess & Tortora 2003,
/// Nature 425, 374). A screened modified-gravity theory must keep its residual |γ−1| below the 1σ
/// uncertainty, 2.3×10⁻⁵, at the solar system.
const CASSINI_GAMMA_MINUS_ONE_BOUND: f64 = 2.3e-5;

/// Redshifts at which the adjudicator probes the theory's own background for the unphysical
/// (negative Friedmann sum) failure mode — late-time through recombination.
const BACKGROUND_PROBE_REDSHIFTS: [f64; 6] = [0.0, 0.5, 1.0, 2.0, 10.0, 1100.0];

/// Tensor-speed excess |c_T/c − 1| of a theory **recomputed at a given redshift**, from the
/// α-basis relation c_T² = 1 + α_T (Bellini & Sawicki 2014, JCAP 07 (2014) 050, arXiv:1404.3713;
/// see also Ezquiaga & Zumalacárregui 2017, PRL 119, 251304, arXiv:1710.05901). α_T is evolved to
/// the source epoch with the standard " propto Ω_DE" α-function tracking ansatz used in hi_class
/// (Zumalacárregui et al. 2017, JCAP 08 (2017) 019, arXiv:1605.06102): α_T(a) = α_T0 ·
/// Ω_DE(a)/Ω_DE(today). At GW170817's low source redshift this ratio is ≈ 1, so the check is
/// essentially the present-day α_T — but it is computed from the background, not read off a label,
/// and it is the structural hook for redshift-dependent α-functions.
pub fn tensor_speed_excess_at(theory: &Theory, z: f64) -> f64 {
    let bg = &theory.background;
    let omega_de_today = bg.omega_de();
    // Ω_DE(a)/Ω_DE(today) = [ρ_DE(z)/ρ_DE0] / E(z)²  (= 1 at z = 0).
    let e2 = bg.e_squared_unclamped(z);
    let de_tracking = if omega_de_today.abs() < 1e-30 || e2 <= 0.0 {
        1.0
    } else {
        bg.de_density_ratio(z) / e2
    };
    let alpha_t_at_z = theory.alpha.alpha_t * de_tracking;
    // c_T = sqrt(1 + α_T); |c_T/c − 1| = |sqrt(1 + α_T) − 1| (exact, no small-α approximation).
    ((1.0 + alpha_t_at_z).max(0.0).sqrt() - 1.0).abs()
}

/// **Adjudication**: recompute physics and return every hard failure (empty ⇒ promotable on
/// physics). Unlike [`run_veto_cascade`] (triage), this trusts no candidate-supplied label:
///
/// 1. Tensor speed at the GW170817 source epoch vs the real ~10⁻¹⁵ bound (Abbott 2017).
/// 2. A theory declaring `screening` must carry a numeric `screening_recovery` whose *recomputed*
///    residual solar-system PPN γ−1 clears the Cassini bound (Bertotti 2003) — a bare string fails.
/// 3. The theory's own background must be physical (E² ≥ 0) on the probe grid.
///
/// Adjudication is *additional* to triage: a candidate must pass both. We do not re-run the cheap
/// metadata checks here (the search loop already did), so the two paths compose without overlap.
/// V6 unified physics gate: triage cascade kills PLUS promotability adjudication.
/// The V5 exploit was a champion that passed triage while failing the stricter checks the code
/// itself said a promotable candidate must clear (quantified screening, the real GW170817 bound,
/// physical background at every probe). No candidate may score while this returns reasons.
pub fn physics_kills(theory: &Theory) -> Vec<VetoReason> {
    let mut reasons = run_veto_cascade(theory);
    reasons.extend(adjudicate(theory));
    reasons
}

pub fn adjudicate(theory: &Theory) -> Vec<VetoReason> {
    let mut reasons = Vec::new();

    // 1. Redshift-aware tensor speed at the GW170817 source epoch vs the real ~10⁻¹⁵ bound.
    let ct_excess = tensor_speed_excess_at(theory, GW170817_SOURCE_REDSHIFT);
    if ct_excess > GW170817_CT_EXCESS_BOUND {
        reasons.push(VetoReason::TensorSpeedAtSource {
            z_source: GW170817_SOURCE_REDSHIFT,
            ct_excess,
            bound: GW170817_CT_EXCESS_BOUND,
        });
    }

    // 2. Screening must be a *quantified* physics claim that actually passes the Cassini PPN bound.
    //    Only required when the theory modifies gravity on linear scales (otherwise GR screens
    //    itself trivially).
    let modification_scale = theory.alpha.modification_scale();
    let modifies_linear_gravity = modification_scale > SCREENING_REQUIRED_SCALE;
    if let Some(mechanism) = &theory.screening {
        match theory.screening_recovery {
            None => {
                // Declared screening with no number: not a physics claim. (Only meaningful if it
                // actually modifies gravity — a screening label on a GR theory is harmless.)
                if modifies_linear_gravity {
                    reasons.push(VetoReason::ScreeningRecoveryUnquantified {
                        mechanism: mechanism.clone(),
                    });
                }
            }
            Some(recovery) => {
                if !(0.0..=1.0).contains(&recovery) {
                    reasons.push(VetoReason::ScreeningRecoveryUnphysical { recovery });
                } else if modifies_linear_gravity {
                    // Residual unscreened PPN deviation: a fraction (1 − recovery) of the linear
                    // gravity modification leaks into the solar system. This is the schematic
                    // chameleon thin-shell / Vainshtein suppression: full recovery (1.0) ⇒ γ → 1.
                    // The exact thin-shell factor (Khoury & Weltman 2004, PRD 69, 044026) / the
                    // Vainshtein (r_*/r)^{3/2} suppression (Vainshtein 1972, Phys.Lett.B39, 393)
                    // is a v3.1 derivation; here recovery is the candidate's *derived* suppression
                    // efficiency and we test the residual against Cassini. // VERIFY: replace the
                    // linear (1−recovery) leakage with the mechanism-specific thin-shell/Vainshtein
                    // formula once the f(R)/nDGP sectors land (M4).
                    let predicted = modification_scale * (1.0 - recovery);
                    if predicted > CASSINI_GAMMA_MINUS_ONE_BOUND {
                        reasons.push(VetoReason::ScreeningInsufficientPpn {
                            predicted_gamma_minus_one: predicted,
                            bound: CASSINI_GAMMA_MINUS_ONE_BOUND,
                        });
                    }
                }
            }
        }
    }

    // 3. The theory's own background must be physical at every probe redshift (recomputed, not
    //    clamped to E = 0).
    for &z in &BACKGROUND_PROBE_REDSHIFTS {
        if let Err(crate::cosmology::BackgroundError::NegativeESquared { z, e_squared }) =
            theory.background.try_e_of_z(z)
        {
            reasons.push(VetoReason::UnphysicalBackground { z, e_squared });
        }
    }

    reasons
}

/// True if the theory is killed by adjudication (the promotability gate).
pub fn is_adjudicated_out(theory: &Theory) -> bool {
    !adjudicate(theory).is_empty()
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

    // ----------------------------------------------------------------------------------------
    // M3 adjudication tests: recomputed physics must reject what triage labels let through.
    // ----------------------------------------------------------------------------------------

    /// Build the adversary's "declare-good-metadata" decoy: a CPL theory that passes every cheap
    /// triage check by *asserting* good metadata (α_T = 0 today, a bare `screening` string, hand
    /// healthy stability, dimension-4 terms, an arbitrary w0) but has no quantified screening.
    fn metadata_decoy() -> Theory {
        let mut t = Theory::baseline_lcdm();
        t.id = "metadata-decoy-cpl-vainshtein".into();
        // Modifies gravity on linear scales (so screening is genuinely required)…
        t.alpha = AlphaBasis {
            alpha_m: 0.1,
            alpha_b: 0.05,
            alpha_k: 0.2,
            alpha_t: 0.0, // …but declares the tensor sector safe today.
        };
        // A bare screening *string* — the decoy's whole trick — with NO numeric recovery.
        t.screening = Some("vainshtein".into());
        t.screening_recovery = None;
        // Hand-set healthy stability (asserted, not derived).
        t.stability = Stability::healthy();
        // An arbitrary CPL background (dimension-4 terms inherited from the baseline).
        t.background.w0 = -0.85;
        t.background.wa = 0.2;
        // A provenanced parameter so the whitebox triage gate is satisfied.
        t.parameters.push(Parameter {
            symbol: "alpha_M0".into(),
            value: 0.1,
            physical_meaning: "Planck-mass run amplitude".into(),
            provenance: Provenance::Derived {
                mechanism: "conformal coupling in the assumed scalar potential".into(),
                certificate: None,
            },
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

    #[test]
    fn metadata_decoy_passes_triage_but_is_killed_by_adjudication() {
        let decoy = metadata_decoy();
        // It sails through the cheap metadata triage: declared screening string ⇒ no
        // MissingScreening, α_T = 0 ⇒ no GW gate, healthy stability, dim-4 terms, whitebox params.
        assert!(
            run_veto_cascade(&decoy).is_empty(),
            "decoy must PASS triage, got {:?}",
            run_veto_cascade(&decoy)
        );
        // But adjudication recomputes physics and kills it with a receipt that names the failing
        // computed quantity: the screening is an unquantified label, not a real recovery.
        let verdict = adjudicate(&decoy);
        assert!(
            verdict.iter().any(|r| matches!(
                r,
                VetoReason::ScreeningRecoveryUnquantified { mechanism } if mechanism == "vainshtein"
            )),
            "adjudication must KILL the decoy naming the unquantified screening, got {verdict:?}"
        );
        assert!(is_adjudicated_out(&decoy));
    }

    #[test]
    fn a_quantified_but_insufficient_screening_is_killed_by_adjudication() {
        // Same decoy, now with a *number* — but a recovery too weak to pass Cassini: with
        // modification_scale = 0.1 and recovery = 0.9, residual γ−1 ≈ 0.01 ≫ 2.3e-5.
        let mut t = metadata_decoy();
        t.screening_recovery = Some(0.9);
        let verdict = adjudicate(&t);
        assert!(
            verdict
                .iter()
                .any(|r| matches!(r, VetoReason::ScreeningInsufficientPpn { .. })),
            "weak recovery must fail the Cassini PPN bound, got {verdict:?}"
        );
    }

    #[test]
    fn a_sufficiently_screened_modified_gravity_clears_adjudication() {
        // modification_scale = max(|α_m|,|α_b|,|α_k|) = 0.2 for this decoy. To suppress it below
        // Cassini's 2.3e-5 we need (1 − recovery) < 1.15e-4, i.e. recovery ≳ 0.999885.
        // Vainshtein/chameleon screening in the solar system is far deeper than this, so a
        // genuinely-screened theory clears the bound.
        let mut t = metadata_decoy();
        t.screening_recovery = Some(0.999_9);
        assert!(
            adjudicate(&t).is_empty(),
            "a deeply-screened theory must clear adjudication, got {:?}",
            adjudicate(&t)
        );
    }

    #[test]
    fn redshift_aware_tensor_speed_kills_a_today_safe_but_evolving_alpha_t() {
        // A candidate can hide an α_T violation behind "α_T = 0 *today*" only if α_T does not
        // evolve. Here we hand it a non-zero present-day α_T well above the 10⁻¹⁵ bound: the
        // recomputed tensor speed at the GW170817 source epoch kills it (triage's 1e-2 gate would
        // also catch this large a value — the point is that adjudication uses the *real* bound, so
        // even an α_T of 1e-10, which triage waves through, dies here).
        let mut t = Theory::baseline_lcdm();
        t.alpha.alpha_t = 1.0e-10; // passes triage (|α_T| < 1e-2) …
        assert!(
            !run_veto_cascade(&t)
                .iter()
                .any(|r| matches!(r, VetoReason::GravitationalWaveSpeed { .. })),
            "α_T = 1e-10 must pass the loose triage gate"
        );
        // … but |c_T/c − 1| ≈ 5e-11 ≫ 3e-15, so adjudication kills it at the source epoch.
        let verdict = adjudicate(&t);
        assert!(
            verdict.iter().any(|r| matches!(
                r,
                VetoReason::TensorSpeedAtSource { ct_excess, bound, .. }
                    if *ct_excess > *bound
            )),
            "adjudication must apply the real 10⁻¹⁵ GW170817 bound, got {verdict:?}"
        );
    }

    #[test]
    fn the_gr_baseline_and_a_truly_screened_theory_pass_both_paths() {
        // GR ΛCDM passes triage and adjudication (α_T = 0, no screening claim, physical bg).
        assert!(run_veto_cascade(&Theory::baseline_lcdm()).is_empty());
        assert!(adjudicate(&Theory::baseline_lcdm()).is_empty());
        // tensor-speed excess at the source epoch is exactly 0 for GR.
        assert_eq!(
            tensor_speed_excess_at(&Theory::baseline_lcdm(), GW170817_SOURCE_REDSHIFT),
            0.0
        );
    }

    #[test]
    fn an_out_of_range_screening_recovery_is_rejected() {
        let mut t = metadata_decoy();
        t.screening_recovery = Some(1.5); // not a fraction
        assert!(adjudicate(&t)
            .iter()
            .any(|r| matches!(r, VetoReason::ScreeningRecoveryUnphysical { .. })));
    }

    #[test]
    fn an_unphysical_background_is_caught_by_adjudication() {
        // A runaway phantom with large wa drives the DE density — and the Friedmann sum — negative
        // at high z, which `e_of_z` would silently clamp to 0. Adjudication reports it instead.
        let mut t = Theory::baseline_lcdm();
        t.background.w0 = 5.0; // strongly positive w ⇒ ρ_DE blows up toward early times…
        t.background.wa = 0.0;
        // Make ΩDE negative so the sum can go negative: push Ωm above 1 so ΩDE = 1−Ωm−Ωr < 0.
        t.background.omega_m = 1.4;
        let verdict = adjudicate(&t);
        assert!(
            verdict
                .iter()
                .any(|r| matches!(r, VetoReason::UnphysicalBackground { .. })),
            "a negative Friedmann sum must be reported, got {verdict:?}"
        );
    }
}
