//! V4 M1: the **derivation obligation oracle**.
//!
//! A `DerivedCertificate` ([`super::certificate`]) binds the *value* of one parameter to a cited
//! closed form. A theory-level *claim*, though, is bigger than a single number: "this term is
//! dimensionally consistent", "this model recovers GR in the appropriate limit", "this identity
//! holds symbolically", "this matches a published result". This module records each such claim as a
//! [`DerivationObligation`] and discharges it through a deterministic oracle ([`DerivationObligation::check`]).
//!
//! The oracle is honest about what it can and cannot machine-check. Numeric/symbolic claims are
//! witnessed by a [`DerivedCertificate`] (real verification). Limit claims are witnessed by a
//! [`LimitWitness`] (a residual that must sit under a finite, non-negative bound). Dimensional claims
//! defer to [`dimensional_consistency`] over the action terms. Literature-equivalence is a *low
//! rigor* textual attestation (a citation), and the two formal-proof kinds ([`DerivationObligationKind::Positivstellensatz`],
//! [`DerivationObligationKind::LeanSketch`]) are recorded but **not** machine-checked in v4: they
//! return [`ObligationOutcome::Unsupported`] and therefore earn no rigor weight. This keeps the
//! engine from rewarding a proof it never actually ran.

use super::certificate::{CertificateOutcome, DerivedCertificate};
use serde::{Deserialize, Serialize};

/// The kind of derivation obligation a claim raises, and (via [`Self::rigor_weight`]) how much
/// rigor discharging it actually buys. The formal-proof kinds are stubs in v4 — see the module docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DerivationObligationKind {
    /// The term/Lagrangian-density has the right mass dimension and Lorentz structure.
    Dimensional,
    /// A quantity tends to its reference value at a physical limit (witnessed by a residual bound).
    Limit,
    /// A symbolic identity reduces to a recomputable closed-form value (witnessed by a certificate).
    SymbolicIdentity,
    /// A numeric value is recomputed from cited inputs (witnessed by a certificate).
    NumericWitness,
    /// A polynomial positivity (sum-of-squares) certificate. **Unsupported stub in v4.**
    Positivstellensatz,
    /// A sketch of a Lean/formal proof. **Unsupported stub in v4.**
    LeanSketch,
    /// A textual attestation that the claim matches a published result (low rigor: just a citation).
    /// Rigor weight demoted to 0.15 in V8 — prefer [`Self::LiteratureEquationMatch`].
    LiteratureEquivalence,
    /// An equation-anchored literature match: the proposer copies a specific equation (by ID and
    /// DOI) from a published paper and attests that the theory's expression reduces to that form.
    /// Stronger than [`Self::LiteratureEquivalence`] because it commits to a specific equation,
    /// not just a paper, making auditing machine-assisted.
    LiteratureEquationMatch,
    /// A *falsifiable novel prediction* distinguishing the theory from the ΛCDM/SM baseline: a named
    /// observable whose predicted value departs from the baseline by a testable amount, plus the
    /// experiment that would refute it. Feeds the **novelty** dimension, not derivation rigor.
    NovelPrediction,
}

impl DerivationObligationKind {
    /// Rigor weight in `[0, 1]`: how much a *verified* obligation of this kind contributes to a
    /// claim's rigor. Machine-checkable kinds are strong; the formal-proof stubs earn nothing here
    /// (they are [`ObligationOutcome::Unsupported`] until a checker is wired in); a bare citation is
    /// weak. The weight is a property of the *kind*, not of any particular outcome — multiply by
    /// whether the obligation actually verified at the call site.
    pub fn rigor_weight(self) -> f64 {
        match self {
            DerivationObligationKind::NumericWitness => 1.0,
            DerivationObligationKind::SymbolicIdentity => 0.9,
            DerivationObligationKind::Dimensional => 0.9,
            DerivationObligationKind::Limit => 0.9,
            // Recorded but not machine-checked in v4 ⇒ no rigor earned.
            DerivationObligationKind::Positivstellensatz => 0.0,
            DerivationObligationKind::LeanSketch => 0.0,
            // A citation is an attestation, not a derivation. Demoted in V8.
            DerivationObligationKind::LiteratureEquivalence => 0.15,
            // An equation-anchored match: pinned to a specific equation + DOI.
            DerivationObligationKind::LiteratureEquationMatch => 0.7,
            // A prediction is novelty evidence, not rigor — it earns nothing on the rigor axis.
            DerivationObligationKind::NovelPrediction => 0.0,
        }
    }
}

/// A witness that the theory makes a *falsifiable novel prediction*: a named observable whose
/// predicted value departs from the baseline (ΛCDM/SM) by at least the cited experiment's detectable
/// resolution, with the measurement that would refute it. The oracle checks the *form* of a testable,
/// non-degenerate prediction (finite numbers, a real gap, a named falsifier) — **not** that the
/// predicted number is physically correct (that is the data layer's job).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NovelPredictionWitness {
    /// The observable the prediction concerns (e.g. "fsigma8_z051").
    pub observable: String,
    /// The theory's predicted value for that observable.
    pub predicted: f64,
    /// The baseline (ΛCDM) value for the same observable.
    pub baseline: f64,
    /// V6.1: true when the engine (re-clothe refresh) authored the declared numbers — the
    /// honesty attestation is then vacuous and the witness earns at most the engine-attested
    /// tier, never full proposer-honesty credit.
    #[serde(default)]
    pub refreshed_by_engine: bool,
    /// The smallest deviation the cited experiment can resolve (must be finite and `> 0`).
    pub min_detectable: f64,
    /// The measurement/experiment that would falsify the prediction.
    pub falsifier: String,
}

impl NovelPredictionWitness {
    /// True iff this is a well-formed, testable, non-degenerate prediction: all numbers finite, a
    /// positive detectable resolution, a non-empty observable + falsifier, and a predicted deviation
    /// from baseline that meets or exceeds that resolution.
    pub fn verifies(&self) -> bool {
        !self.observable.trim().is_empty()
            && !self.falsifier.trim().is_empty()
            && self.predicted.is_finite()
            && self.baseline.is_finite()
            && self.min_detectable.is_finite()
            && self.min_detectable > 0.0
            && (self.predicted - self.baseline).abs() >= self.min_detectable
    }
}

/// Witness that a theory expression matches a specific equation from a published paper.
/// Stronger than a bare citation: the proposer copies the equation by ID and DOI, which
/// makes auditing targeted — a reviewer can look up the exact equation rather than scanning the
/// paper. All three fields must be non-empty for [`EquationMatchWitness::verifies`] to hold.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EquationMatchWitness {
    /// Equation identifier in the source (e.g., "Eq. 15", "Eq. B12", "(3.4)").
    pub equation_id: String,
    /// DOI or persistent URL of the source paper (e.g., "10.1103/PhysRevD.62.043511").
    pub doi_anchor: String,
    /// LaTeX formula transcribed from the paper (the proposer copies it literally from the source).
    pub formula_latex: String,
}

impl EquationMatchWitness {
    /// True iff all three identifying fields are non-empty (after trimming).
    pub fn verifies(&self) -> bool {
        !self.equation_id.trim().is_empty()
            && !self.doi_anchor.trim().is_empty()
            && !self.formula_latex.trim().is_empty()
    }
}

/// Outcome of discharging an obligation through the oracle.
#[derive(Debug, Clone, PartialEq)]
pub enum ObligationOutcome {
    /// The obligation was machine-checked and holds.
    Verified { detail: String },
    /// The obligation was machine-checked and does **not** hold. `counterexample` carries a short
    /// falsifier when one is available (see [`falsifier`]).
    Failed {
        detail: String,
        counterexample: Option<String>,
    },
    /// The obligation cannot be machine-checked in this version (honest stub). Earns no rigor.
    Unsupported { detail: String },
}

impl ObligationOutcome {
    /// True iff the obligation was machine-checked and holds.
    pub fn is_verified(&self) -> bool {
        matches!(self, ObligationOutcome::Verified { .. })
    }
}

/// A numeric witness that some quantity has converged to its reference value at a physical limit:
/// the residual `|quantity − reference|` must sit under a finite, non-negative `bound`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LimitWitness {
    /// What limit/quantity this witnesses (used in diagnostics and counterexamples).
    pub name: String,
    /// The residual `quantity − reference` at the limit. Only its magnitude matters.
    pub residual: f64,
    /// Absolute bound the residual magnitude must respect. A non-finite or negative bound never
    /// verifies (guards against a "bound = ∞" cheat, mirroring [`DerivedCertificate`]'s tolerance).
    pub bound: f64,
}

impl LimitWitness {
    /// True iff `bound` is finite and non-negative and `|residual| <= bound`. A `NaN` residual or
    /// bound never verifies (the comparison is false).
    pub fn verifies(&self) -> bool {
        self.bound.is_finite() && self.bound >= 0.0 && self.residual.abs() <= self.bound
    }
}

/// One recorded derivation obligation: a claim plus whatever witness discharges it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DerivationObligation {
    /// Identifier of the claim this obligation belongs to.
    pub claim_id: String,
    /// The kind of obligation (selects the witness the oracle requires).
    pub kind: DerivationObligationKind,
    /// Human-readable description of the claim.
    pub detail: String,
    /// Value/identity witness for [`DerivationObligationKind::NumericWitness`],
    /// [`DerivationObligationKind::SymbolicIdentity`], and (optionally) [`DerivationObligationKind::Dimensional`].
    #[serde(default)]
    pub certificate: Option<DerivedCertificate>,
    /// Residual-bound witness for [`DerivationObligationKind::Limit`].
    #[serde(default)]
    pub limit: Option<LimitWitness>,
    /// Citation backing a [`DerivationObligationKind::LiteratureEquivalence`] attestation.
    #[serde(default)]
    pub citation: Option<String>,
    /// Falsifiable-prediction witness for [`DerivationObligationKind::NovelPrediction`].
    #[serde(default)]
    pub novel: Option<NovelPredictionWitness>,
    /// Equation-anchor witness for [`DerivationObligationKind::LiteratureEquationMatch`].
    #[serde(default)]
    pub equation_match: Option<EquationMatchWitness>,
}

impl DerivationObligation {
    /// Discharge this obligation through the deterministic oracle. Dispatches on [`Self::kind`].
    pub fn check(&self) -> ObligationOutcome {
        match self.kind {
            DerivationObligationKind::NumericWitness | DerivationObligationKind::SymbolicIdentity => {
                match &self.certificate {
                    Some(cert) => self.check_certificate(cert),
                    None => ObligationOutcome::Failed {
                        detail: format!(
                            "{:?} obligation '{}' requires a certificate witness but none is attached",
                            self.kind, self.claim_id
                        ),
                        counterexample: None,
                    },
                }
            }
            DerivationObligationKind::Dimensional => match &self.certificate {
                // A dimensional claim can be witnessed by a certificate; delegate when one exists.
                Some(cert) => self.check_certificate(cert),
                None => ObligationOutcome::Unsupported {
                    detail:
                        "dimensional obligation needs a witness; use dimensional_consistency() on the theory terms"
                            .into(),
                },
            },
            DerivationObligationKind::Limit => match &self.limit {
                Some(limit) => {
                    if limit.verifies() {
                        ObligationOutcome::Verified {
                            detail: format!(
                                "limit '{}' converged: |residual| {} <= bound {}",
                                limit.name,
                                limit.residual.abs(),
                                limit.bound
                            ),
                        }
                    } else {
                        ObligationOutcome::Failed {
                            detail: format!("limit '{}' did not converge", limit.name),
                            counterexample: Some(limit_counterexample(limit)),
                        }
                    }
                }
                None => ObligationOutcome::Failed {
                    detail: format!(
                        "limit obligation '{}' requires a LimitWitness but none is attached",
                        self.claim_id
                    ),
                    counterexample: None,
                },
            },
            DerivationObligationKind::LiteratureEquivalence => {
                match self.citation.as_ref().map(|c| c.trim()) {
                    Some(c) if !c.is_empty() => ObligationOutcome::Verified {
                        detail: format!(
                            "literature-equivalence attested by citation: {c} (low rigor)"
                        ),
                    },
                    _ => ObligationOutcome::Failed {
                        detail: format!(
                            "literature-equivalence claim '{}' has no citation",
                            self.claim_id
                        ),
                        counterexample: Some(
                            "no citation supplied for literature-equivalence attestation".into(),
                        ),
                    },
                }
            }
            DerivationObligationKind::LiteratureEquationMatch => {
                match &self.equation_match {
                    Some(w) if w.verifies() => ObligationOutcome::Verified {
                        detail: format!(
                            "equation match: {} in {} — formula: {}",
                            w.equation_id, w.doi_anchor, w.formula_latex
                        ),
                    },
                    Some(_) => ObligationOutcome::Failed {
                        detail: format!(
                            "equation-match witness for '{}' is incomplete (equation_id, doi_anchor, or formula_latex is empty)",
                            self.claim_id
                        ),
                        counterexample: Some(
                            "equation_id, doi_anchor, and formula_latex must all be non-empty".into(),
                        ),
                    },
                    None => ObligationOutcome::Failed {
                        detail: format!(
                            "literature-equation-match obligation '{}' requires an EquationMatchWitness",
                            self.claim_id
                        ),
                        counterexample: None,
                    },
                }
            }
            DerivationObligationKind::Positivstellensatz | DerivationObligationKind::LeanSketch => {
                ObligationOutcome::Unsupported {
                    detail: "formal-proof obligation recorded but not yet machine-checked in v4"
                        .into(),
                }
            }
            DerivationObligationKind::NovelPrediction => match &self.novel {
                Some(w) if w.verifies() => ObligationOutcome::Verified {
                    detail: format!(
                        "novel falsifiable prediction on '{}': |Δ| {} >= resolution {}; falsifier: {}",
                        w.observable,
                        (w.predicted - w.baseline).abs(),
                        w.min_detectable,
                        w.falsifier
                    ),
                },
                Some(_) => ObligationOutcome::Failed {
                    detail: format!(
                        "novel-prediction witness for '{}' is degenerate (Δ below detectable / missing fields)",
                        self.claim_id
                    ),
                    counterexample: Some(
                        "predicted deviation does not exceed the stated detectable resolution".into(),
                    ),
                },
                None => ObligationOutcome::Failed {
                    detail: format!(
                        "novel-prediction obligation '{}' requires a NovelPredictionWitness",
                        self.claim_id
                    ),
                    counterexample: None,
                },
            },
        }
    }

    /// The rigor this obligation *actually* earns: the kind weight scaled by the depth of the
    /// relation it is witnessed on. A `NumericWitness`/`SymbolicIdentity`/witnessed-`Dimensional` on a
    /// trivial identity (e.g. `h0_from_h`, `flat_universe_omega_lambda`) earns **0**; on a genuine
    /// modified-gravity relation it earns the kind weight. `Limit`/`LiteratureEquivalence`/stub kinds
    /// are independent of the certificate registry and keep their kind weight. This closes the
    /// "recompute a definition and call it a derivation" hole — see
    /// [`super::certificate::relation_rigor_weight`].
    pub fn effective_rigor_weight(&self) -> f64 {
        use DerivationObligationKind::*;
        match self.kind {
            NumericWitness | SymbolicIdentity => {
                self.kind.rigor_weight()
                    * self
                        .certificate
                        .as_ref()
                        .map(|c| super::certificate::relation_rigor_weight(&c.relation))
                        .unwrap_or(0.0)
            }
            Dimensional => match &self.certificate {
                Some(c) => {
                    self.kind.rigor_weight()
                        * super::certificate::relation_rigor_weight(&c.relation)
                }
                None => self.kind.rigor_weight(),
            },
            _ => self.kind.rigor_weight(),
        }
    }

    /// Map a certificate's outcome onto an [`ObligationOutcome`], attaching a falsifier on failure.
    fn check_certificate(&self, cert: &DerivedCertificate) -> ObligationOutcome {
        match cert.check() {
            CertificateOutcome::Verified { computed, residual } => ObligationOutcome::Verified {
                detail: format!(
                    "{}: relation '{}' recomputed {} (residual {} <= tol {})",
                    self.claim_id, cert.relation, computed, residual, cert.tolerance
                ),
            },
            _ => ObligationOutcome::Failed {
                detail: format!(
                    "{}: certificate on relation '{}' failed verification",
                    self.claim_id, cert.relation
                ),
                counterexample: Some(certificate_counterexample(cert)),
            },
        }
    }
}

/// Build a short falsifier string for a certificate that did not verify.
fn certificate_counterexample(cert: &DerivedCertificate) -> String {
    match cert.check() {
        CertificateOutcome::ValueMismatch { computed, residual } => format!(
            "relation '{}' recomputes {} but theory claims {} (residual {} > tol {})",
            cert.relation, computed, cert.expected, residual, cert.tolerance
        ),
        CertificateOutcome::UnknownRelation => {
            format!(
                "relation '{}' is not in the certificate registry",
                cert.relation
            )
        }
        CertificateOutcome::MissingOrInvalidInput { detail } => {
            format!("relation '{}' has invalid input: {detail}", cert.relation)
        }
        CertificateOutcome::InvalidTolerance => format!(
            "relation '{}' declares an unusable tolerance {}",
            cert.relation, cert.tolerance
        ),
        CertificateOutcome::AntiLaunderingKill { detail } => format!(
            "relation '{}' killed by anti-laundering gate: {detail}",
            cert.relation
        ),
        // Verified certificates have no counterexample; report neutrally for completeness.
        CertificateOutcome::Verified { .. } => {
            format!("relation '{}' verifies (no counterexample)", cert.relation)
        }
    }
}

/// Build a short falsifier string for a limit witness that did not verify.
fn limit_counterexample(limit: &LimitWitness) -> String {
    format!(
        "residual {} exceeds bound {} at {}",
        limit.residual.abs(),
        limit.bound,
        limit.name
    )
}

/// Structural dimensional-consistency oracle over a theory's action terms: every 4D Lagrangian
/// density term must have mass dimension 4 and zero uncontracted Lorentz indices. Verified iff all
/// terms satisfy both; otherwise Failed, listing the offending term(s) with a counterexample.
pub fn dimensional_consistency(terms: &[super::Term]) -> ObligationOutcome {
    let offenders: Vec<String> = terms
        .iter()
        .filter(|t| t.mass_dimension != 4 || t.free_lorentz_indices != 0)
        .map(|t| {
            format!(
                "{} (mass_dimension={}, free_lorentz_indices={})",
                t.name, t.mass_dimension, t.free_lorentz_indices
            )
        })
        .collect();

    if offenders.is_empty() {
        ObligationOutcome::Verified {
            detail: format!(
                "all {} action term(s) are dimension-4 Lorentz scalars",
                terms.len()
            ),
        }
    } else {
        ObligationOutcome::Failed {
            detail: format!(
                "{} action term(s) are not dimension-4 Lorentz scalars: {}",
                offenders.len(),
                offenders.join("; ")
            ),
            counterexample: Some(format!("offending term(s): {}", offenders.join("; "))),
        }
    }
}

/// Structural "recovers GR/ΛCDM at the appropriate limit" oracle. Verified iff the linear gravity
/// modification, the tensor-speed excess, and the screening recovery all sit within `tol` of GR:
/// `modification_scale() <= tol`, `|alpha_t| <= tol`, and either no screening is declared or its
/// recovery efficiency is within `tol` of 1.0. Otherwise Failed, naming the surviving deviation.
///
/// NOTE: this is a *structural* limit check. The residual it implies (how far from GR the model sits)
/// should itself be evidence-bound at the claim layer — a small residual is necessary, not
/// sufficient, for a credible GR-limit claim.
pub fn limit_recovers_gr(theory: &super::Theory, tol: f64) -> ObligationOutcome {
    if !(tol.is_finite() && tol >= 0.0) {
        return ObligationOutcome::Failed {
            detail: format!("GR-limit tolerance {tol} is not finite and non-negative"),
            counterexample: Some(format!("unusable tolerance {tol}")),
        };
    }

    let mut deviations: Vec<String> = Vec::new();

    let mod_scale = theory.alpha.modification_scale();
    if mod_scale > tol {
        deviations.push(format!("linear modification scale {mod_scale} > tol {tol}"));
    }

    let alpha_t = theory.alpha.alpha_t.abs();
    if alpha_t > tol {
        deviations.push(format!("|alpha_t| {alpha_t} > tol {tol}"));
    }

    if theory.screening.is_some() {
        match theory.screening_recovery {
            Some(r) if (1.0 - r).abs() <= tol => {}
            Some(r) => deviations.push(format!(
                "declared screening '{}' recovery {} is {} from full GR recovery (> tol {})",
                theory.screening.as_deref().unwrap_or(""),
                r,
                (1.0 - r).abs(),
                tol
            )),
            None => deviations.push(format!(
                "declared screening '{}' has no numeric recovery efficiency",
                theory.screening.as_deref().unwrap_or("")
            )),
        }
    }

    if deviations.is_empty() {
        ObligationOutcome::Verified {
            detail: format!("theory '{}' recovers GR within tol {}", theory.id, tol),
        }
    } else {
        ObligationOutcome::Failed {
            detail: format!(
                "theory '{}' does not recover GR within tol {}",
                theory.id, tol
            ),
            counterexample: Some(format!("surviving deviation(s): {}", deviations.join("; "))),
        }
    }
}

/// Return a short counterexample/falsifier string for an obligation whose [`DerivationObligation::check`]
/// is `Failed`; `None` if it verifies or is unsupported. When the failure carries no specific
/// counterexample (e.g. a missing witness), the failure detail is returned as the falsifier.
pub fn falsifier(o: &DerivationObligation) -> Option<String> {
    match o.check() {
        ObligationOutcome::Failed {
            counterexample,
            detail,
        } => Some(counterexample.unwrap_or(detail)),
        ObligationOutcome::Verified { .. } | ObligationOutcome::Unsupported { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::{Term, Theory};

    fn ndgp_cert(expected: f64, tol: f64) -> DerivedCertificate {
        DerivedCertificate {
            relation: "ndgp_geff_over_g".into(),
            inputs: vec![("beta".into(), 2.0)],
            expected,
            tolerance: tol,
            ..Default::default()
        }
    }

    fn obligation(kind: DerivationObligationKind) -> DerivationObligation {
        DerivationObligation {
            claim_id: "claim-1".into(),
            kind,
            detail: "test obligation".into(),
            certificate: None,
            limit: None,
            citation: None,
            novel: None,
            equation_match: None,
        }
    }

    #[test]
    fn rigor_weights_rank_as_expected() {
        assert_eq!(DerivationObligationKind::NumericWitness.rigor_weight(), 1.0);
        assert!(DerivationObligationKind::SymbolicIdentity.rigor_weight() > 0.0);
        // Formal-proof stubs earn nothing until a checker is wired in.
        assert_eq!(
            DerivationObligationKind::Positivstellensatz.rigor_weight(),
            0.0
        );
        assert_eq!(DerivationObligationKind::LeanSketch.rigor_weight(), 0.0);
        // A citation is weaker than any machine check.
        assert!(
            DerivationObligationKind::LiteratureEquivalence.rigor_weight()
                < DerivationObligationKind::Dimensional.rigor_weight()
        );
        // A novel prediction earns no rigor (it feeds the novelty dimension instead).
        assert_eq!(
            DerivationObligationKind::NovelPrediction.rigor_weight(),
            0.0
        );
    }

    #[test]
    fn effective_rigor_zeros_trivial_relations_keeps_real_ones() {
        // A NumericWitness on a trivial identity earns ZERO effective rigor (definition, not derivation).
        let mut trivial = obligation(DerivationObligationKind::NumericWitness);
        trivial.certificate = Some(DerivedCertificate {
            relation: "h0_from_h".into(),
            inputs: vec![("h".into(), 0.674)],
            expected: 67.4,
            tolerance: 1e-6,
            ..Default::default()
        });
        assert!(trivial.check().is_verified(), "the cert still verifies");
        assert_eq!(trivial.effective_rigor_weight(), 0.0, "but earns no rigor");

        // A NumericWitness on a real modified-gravity relation keeps full effective rigor.
        let mut real = obligation(DerivationObligationKind::NumericWitness);
        real.certificate = Some(ndgp_cert(1.0 + 1.0 / 6.0, 1e-9));
        assert!(real.check().is_verified());
        assert_eq!(real.effective_rigor_weight(), 1.0);

        // A GR-recovery Limit obligation is registry-independent and keeps its kind weight.
        let mut lim = obligation(DerivationObligationKind::Limit);
        lim.limit = Some(LimitWitness {
            name: "gr".into(),
            residual: 0.0,
            bound: 1e-6,
        });
        assert_eq!(lim.effective_rigor_weight(), 0.9);
    }

    #[test]
    fn novel_prediction_verifies_and_fails() {
        let mut o = obligation(DerivationObligationKind::NovelPrediction);
        // A testable deviation beyond resolution verifies.
        o.novel = Some(NovelPredictionWitness {
            refreshed_by_engine: false,
            observable: "fsigma8_z051".into(),
            predicted: 0.42,
            baseline: 0.46,
            min_detectable: 0.01,
            falsifier: "DESI/Euclid fσ8 at z=0.51".into(),
        });
        assert!(o.check().is_verified(), "{:?}", o.check());

        // A deviation below the detectable resolution is degenerate ⇒ Failed.
        o.novel = Some(NovelPredictionWitness {
            refreshed_by_engine: false,
            observable: "fsigma8_z051".into(),
            predicted: 0.4601,
            baseline: 0.46,
            min_detectable: 0.01,
            falsifier: "DESI/Euclid".into(),
        });
        assert!(!o.check().is_verified());

        // No witness ⇒ Failed.
        o.novel = None;
        assert!(!o.check().is_verified());
    }

    #[test]
    fn numeric_witness_passes_with_correct_certificate() {
        // β = 2 ⇒ G_eff/G = 1 + 1/6 = 1.16666...
        let mut o = obligation(DerivationObligationKind::NumericWitness);
        o.certificate = Some(ndgp_cert(1.0 + 1.0 / 6.0, 1e-9));
        let outcome = o.check();
        assert!(outcome.is_verified(), "{outcome:?}");
        assert!(falsifier(&o).is_none());
    }

    #[test]
    fn numeric_witness_fails_with_wrong_value() {
        let mut o = obligation(DerivationObligationKind::NumericWitness);
        o.certificate = Some(ndgp_cert(1.0, 1e-9)); // claims GR, but β=2 gives 1.1666...
        let outcome = o.check();
        assert!(!outcome.is_verified());
        let f = falsifier(&o).expect("failed numeric witness has a falsifier");
        assert!(f.contains("recomputes") && f.contains("residual"), "{f}");
    }

    #[test]
    fn numeric_witness_without_certificate_fails() {
        let o = obligation(DerivationObligationKind::NumericWitness);
        assert!(matches!(o.check(), ObligationOutcome::Failed { .. }));
    }

    #[test]
    fn symbolic_identity_passes_and_fails() {
        let mut pass = obligation(DerivationObligationKind::SymbolicIdentity);
        pass.certificate = Some(DerivedCertificate {
            relation: "coupled_de_geff_over_g".into(),
            inputs: vec![("beta".into(), 0.1)],
            expected: 1.02,
            tolerance: 1e-12,
            ..Default::default()
        });
        assert!(pass.check().is_verified(), "{:?}", pass.check());

        let mut fail = obligation(DerivationObligationKind::SymbolicIdentity);
        fail.certificate = Some(DerivedCertificate {
            relation: "coupled_de_geff_over_g".into(),
            inputs: vec![("beta".into(), 0.1)],
            expected: 1.5,
            tolerance: 1e-3,
            ..Default::default()
        });
        assert!(!fail.check().is_verified());
    }

    #[test]
    fn dimensional_obligation_delegates_or_is_unsupported() {
        // No payload ⇒ Unsupported (defer to dimensional_consistency on terms).
        let bare = obligation(DerivationObligationKind::Dimensional);
        assert!(matches!(
            bare.check(),
            ObligationOutcome::Unsupported { .. }
        ));
        assert!(falsifier(&bare).is_none());

        // With a witnessing certificate ⇒ delegates.
        let mut witnessed = obligation(DerivationObligationKind::Dimensional);
        witnessed.certificate = Some(ndgp_cert(1.0 + 1.0 / 6.0, 1e-9));
        assert!(witnessed.check().is_verified());
    }

    #[test]
    fn limit_obligation_passes_and_fails() {
        let mut pass = obligation(DerivationObligationKind::Limit);
        pass.limit = Some(LimitWitness {
            name: "c_GW -> c".into(),
            residual: 0.0,
            bound: 1e-6,
        });
        assert!(pass.check().is_verified());
        assert!(falsifier(&pass).is_none());

        let mut fail = obligation(DerivationObligationKind::Limit);
        fail.limit = Some(LimitWitness {
            name: "c_GW -> c".into(),
            residual: 1.0,
            bound: 1e-6,
        });
        assert!(!fail.check().is_verified());
        let f = falsifier(&fail).expect("failed limit has a falsifier");
        assert!(
            f.contains("exceeds bound") && f.contains("c_GW -> c"),
            "{f}"
        );
    }

    #[test]
    fn limit_witness_rejects_nonfinite_bound() {
        let w = LimitWitness {
            name: "x".into(),
            residual: 0.0,
            bound: f64::INFINITY,
        };
        assert!(!w.verifies());
    }

    #[test]
    fn literature_equivalence_passes_with_citation_and_fails_without() {
        let mut pass = obligation(DerivationObligationKind::LiteratureEquivalence);
        pass.citation = Some("Amendola, PRD 62, 043511 (2000)".into());
        assert!(pass.check().is_verified());

        let mut empty = obligation(DerivationObligationKind::LiteratureEquivalence);
        empty.citation = Some("   ".into());
        assert!(!empty.check().is_verified());

        let none = obligation(DerivationObligationKind::LiteratureEquivalence);
        assert!(!none.check().is_verified());
    }

    #[test]
    fn formal_proof_kinds_are_unsupported() {
        let pos = obligation(DerivationObligationKind::Positivstellensatz);
        assert!(matches!(pos.check(), ObligationOutcome::Unsupported { .. }));
        assert!(falsifier(&pos).is_none());

        let lean = obligation(DerivationObligationKind::LeanSketch);
        assert!(matches!(
            lean.check(),
            ObligationOutcome::Unsupported { .. }
        ));
    }

    #[test]
    fn dimensional_consistency_passes_on_baseline_terms() {
        let t = Theory::baseline_lcdm();
        assert!(dimensional_consistency(&t.terms).is_verified());
    }

    #[test]
    fn dimensional_consistency_fails_on_dim5_term() {
        let terms = vec![Term {
            name: "dim5_operator".into(),
            mass_dimension: 5,
            free_lorentz_indices: 0,
        }];
        let outcome = dimensional_consistency(&terms);
        assert!(!outcome.is_verified());
        match outcome {
            ObligationOutcome::Failed {
                counterexample: Some(c),
                ..
            } => assert!(c.contains("dim5_operator"), "{c}"),
            other => panic!("expected Failed with counterexample, got {other:?}"),
        }
    }

    #[test]
    fn limit_recovers_gr_passes_on_baseline() {
        let t = Theory::baseline_lcdm();
        assert!(limit_recovers_gr(&t, 1e-6).is_verified());
    }

    #[test]
    fn limit_recovers_gr_fails_when_gravity_modified() {
        let mut t = Theory::baseline_lcdm();
        t.alpha.alpha_m = 0.1;
        let outcome = limit_recovers_gr(&t, 1e-6);
        assert!(!outcome.is_verified());
        match outcome {
            ObligationOutcome::Failed {
                counterexample: Some(c),
                ..
            } => assert!(c.contains("modification scale"), "{c}"),
            other => panic!("expected Failed with counterexample, got {other:?}"),
        }
    }

    #[test]
    fn limit_recovers_gr_fails_when_screening_unquantified() {
        let mut t = Theory::baseline_lcdm();
        t.screening = Some("chameleon".into());
        t.screening_recovery = None;
        assert!(!limit_recovers_gr(&t, 1e-6).is_verified());
    }

    #[test]
    fn obligation_serde_round_trips() {
        let o = DerivationObligation {
            claim_id: "ndgp-geff".into(),
            kind: DerivationObligationKind::NumericWitness,
            detail: "nDGP effective gravitational coupling at β=2".into(),
            certificate: Some(ndgp_cert(1.0 + 1.0 / 6.0, 1e-9)),
            limit: Some(LimitWitness {
                name: "c_GW -> c".into(),
                residual: 0.0,
                bound: 1e-6,
            }),
            citation: Some("Koyama & Maartens, JCAP 0601:016 (2006)".into()),
            novel: None,
            equation_match: None,
        };
        let json = serde_json::to_string(&o).expect("serialize");
        let back: DerivationObligation = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(o, back);
        assert!(back.check().is_verified());
    }

    #[test]
    fn kind_serializes_snake_case() {
        let j = serde_json::to_string(&DerivationObligationKind::LiteratureEquivalence).unwrap();
        assert_eq!(j, "\"literature_equivalence\"");
    }

    #[test]
    fn obligation_deserializes_without_optional_witness_keys() {
        // Back-compat / minimal payload: only the required keys present.
        let minimal = r#"{"claim_id":"c","kind":"lean_sketch","detail":"d"}"#;
        let o: DerivationObligation = serde_json::from_str(minimal).expect("minimal deserializes");
        assert!(o.certificate.is_none() && o.limit.is_none() && o.citation.is_none());
        assert!(matches!(o.check(), ObligationOutcome::Unsupported { .. }));
    }

    // --- Phase 14 / SYNTHESIS #16: LiteratureEquationMatch tests ---

    #[test]
    fn literature_equation_match_rigor_weight_is_higher_than_equivalence() {
        assert!(
            DerivationObligationKind::LiteratureEquationMatch.rigor_weight()
                > DerivationObligationKind::LiteratureEquivalence.rigor_weight()
        );
        assert_eq!(
            DerivationObligationKind::LiteratureEquationMatch.rigor_weight(),
            0.7
        );
    }

    #[test]
    fn literature_equivalence_weight_demoted_to_0_15() {
        assert_eq!(
            DerivationObligationKind::LiteratureEquivalence.rigor_weight(),
            0.15
        );
    }

    #[test]
    fn equation_match_witness_verifies_when_all_fields_set() {
        let w = EquationMatchWitness {
            equation_id: "Eq. 15".into(),
            doi_anchor: "10.1103/PhysRevD.62.043511".into(),
            formula_latex: r"\mu(a) = 1 + \mu_0 \Omega_{\rm DE}(a)".into(),
        };
        assert!(w.verifies());
    }

    #[test]
    fn equation_match_witness_fails_when_any_field_empty() {
        let base = EquationMatchWitness {
            equation_id: "Eq. 15".into(),
            doi_anchor: "10.1103/PhysRevD.62.043511".into(),
            formula_latex: r"G_{\rm eff}/G = 1 + 1/3\beta".into(),
        };
        let mut missing_eq = base.clone();
        missing_eq.equation_id = "".into();
        assert!(!missing_eq.verifies());

        let mut missing_doi = base.clone();
        missing_doi.doi_anchor = "  ".into();
        assert!(!missing_doi.verifies());

        let mut missing_latex = base.clone();
        missing_latex.formula_latex = "".into();
        assert!(!missing_latex.verifies());
    }

    #[test]
    fn literature_equation_match_verifies_with_full_witness() {
        let mut o = obligation(DerivationObligationKind::LiteratureEquationMatch);
        o.equation_match = Some(EquationMatchWitness {
            equation_id: "Eq. 3".into(),
            doi_anchor: "10.1088/1475-7516/2006/01/016".into(),
            formula_latex: r"G_{\rm eff}/G = 1 + 1/(3\beta^2)".into(),
        });
        assert!(o.check().is_verified(), "{:?}", o.check());
        assert!(falsifier(&o).is_none());
    }

    #[test]
    fn literature_equation_match_fails_with_incomplete_witness() {
        let mut o = obligation(DerivationObligationKind::LiteratureEquationMatch);
        o.equation_match = Some(EquationMatchWitness {
            equation_id: "Eq. 3".into(),
            doi_anchor: "".into(),
            formula_latex: r"G_{\rm eff}/G".into(),
        });
        assert!(!o.check().is_verified());
        let f = falsifier(&o).expect("incomplete witness has a falsifier");
        assert!(f.contains("non-empty"), "{f}");
    }

    #[test]
    fn literature_equation_match_fails_without_witness() {
        let o = obligation(DerivationObligationKind::LiteratureEquationMatch);
        assert!(matches!(o.check(), ObligationOutcome::Failed { .. }));
        assert!(falsifier(&o).is_some());
    }

    #[test]
    fn literature_equation_match_effective_rigor_weight() {
        // LiteratureEquationMatch falls through to `_ => self.kind.rigor_weight()` — always 0.7.
        let mut o = obligation(DerivationObligationKind::LiteratureEquationMatch);
        o.equation_match = Some(EquationMatchWitness {
            equation_id: "Eq. 1".into(),
            doi_anchor: "10.1234/test".into(),
            formula_latex: r"\alpha_T = 0".into(),
        });
        assert_eq!(o.effective_rigor_weight(), 0.7);
    }

    #[test]
    fn literature_equation_match_serializes_snake_case() {
        let j = serde_json::to_string(&DerivationObligationKind::LiteratureEquationMatch).unwrap();
        assert_eq!(j, "\"literature_equation_match\"");
    }

    #[test]
    fn equation_match_obligation_serde_round_trip() {
        let o = DerivationObligation {
            claim_id: "coupled-de-geff".into(),
            kind: DerivationObligationKind::LiteratureEquationMatch,
            detail: "G_eff/G expression matches Amendola 2000 Eq. 15".into(),
            certificate: None,
            limit: None,
            citation: Some("Amendola 2000".into()),
            novel: None,
            equation_match: Some(EquationMatchWitness {
                equation_id: "Eq. 15".into(),
                doi_anchor: "10.1103/PhysRevD.62.043511".into(),
                formula_latex: r"G_{\rm eff}/G = 1 + \frac{2\beta^2}{1 + m^2/k^2}".into(),
            }),
        };
        let json = serde_json::to_string(&o).expect("serialize");
        let back: DerivationObligation = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(o, back);
        assert!(back.check().is_verified());
    }

    #[test]
    fn minimal_json_without_equation_match_back_compat() {
        // Old obligations without the equation_match key must still deserialize cleanly.
        let minimal = r#"{"claim_id":"c","kind":"literature_equation_match","detail":"d"}"#;
        let o: DerivationObligation = serde_json::from_str(minimal).expect("deserializes");
        assert!(o.equation_match.is_none());
        assert!(matches!(o.check(), ObligationOutcome::Failed { .. }));
    }
}
