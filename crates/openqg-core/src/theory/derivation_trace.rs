//! V8 Phase 2 (#10 + #11): DerivationTrace — the typed step-level derivation record.
//!
//! ## Problem closed
//! A `DerivedCertificate` currently only verifies that a *closed-form relation* holds given its
//! named inputs. The inputs themselves carry no provenance: a proposer can flow a posterior median
//! (a `ScoredDataEstimate`) into the inputs and earn derivation credit for what is functionally
//! a fit. This is the anti-laundering hole S01/S02/S09/S11 independently flagged.
//!
//! ## Design
//! Each input to a certificate now carries an `AssumptionStrength` that declares where its value
//! came from. The anti-laundering gate kills any certificate whose inputs include a
//! `ScoredDataEstimate` — a value extracted from fitting the scored data cannot earn derivation
//! credit, regardless of whether the relation is mathematically correct.
//!
//! Beyond the input gate, the full `DerivationTrace` is a sequence of typed `TraceStep`s that
//! represent the derivation at a granularity between "mechanism text" and a full formal proof:
//! just enough structure for the engine to compute rigor scores and audit assumption flow.

use serde::{Deserialize, Serialize};

/// Where an input value came from. This is the anti-laundering label.
///
/// The ordering matters: lower = stronger assumption (more trustworthy).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssumptionStrength {
    /// A mathematical axiom or definition (e.g. G_eff/G by definition from the action).
    /// Full derivation credit; no measurement required.
    Axiom,
    /// A result proven from axioms within the theory's own framework.
    TheoremFromAxiom,
    /// A value or relation cited from published, peer-reviewed literature that is NOT a
    /// measurement of the scored observables (e.g. a Planck prior cited for ω_b, not H0).
    PublishedLiterature,
    /// A value measured by an external experiment NOT in the scored dataset.
    /// Carries derivation credit, but caps rigor at 0.25 (S01 hard cap).
    ExternalMeasurement,
    /// A value extracted from the scored dataset — i.e. a posterior median or best-fit
    /// from the very data the engine is scoring against.
    /// **Hard kill**: any certificate input with this strength is an anti-laundering violation.
    ScoredDataEstimate,
    /// A phenomenological parametrization (e.g. a fitting formula for G_eff that has no
    /// first-principles derivation). Caps rigor at 0.45 (S01 hard cap).
    PhenomenologicalFit,
}

impl AssumptionStrength {
    /// True when this input strength triggers the anti-laundering hard kill.
    pub fn is_laundering(self) -> bool {
        self == AssumptionStrength::ScoredDataEstimate
    }

    /// The maximum rigor score a certificate whose worst input has this strength can earn.
    /// `None` means "killed" (score = 0.0, not just capped).
    pub fn rigor_cap(self) -> Option<f64> {
        match self {
            AssumptionStrength::Axiom => Some(1.0),
            AssumptionStrength::TheoremFromAxiom => Some(1.0),
            AssumptionStrength::PublishedLiterature => Some(0.75),
            AssumptionStrength::ExternalMeasurement => Some(0.25),
            AssumptionStrength::ScoredDataEstimate => None, // killed
            AssumptionStrength::PhenomenologicalFit => Some(0.45),
        }
    }
}

impl std::fmt::Display for AssumptionStrength {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssumptionStrength::Axiom => write!(f, "axiom"),
            AssumptionStrength::TheoremFromAxiom => write!(f, "theorem_from_axiom"),
            AssumptionStrength::PublishedLiterature => write!(f, "published_literature"),
            AssumptionStrength::ExternalMeasurement => write!(f, "external_measurement"),
            AssumptionStrength::ScoredDataEstimate => write!(f, "scored_data_estimate"),
            AssumptionStrength::PhenomenologicalFit => write!(f, "phenomenological_fit"),
        }
    }
}

/// A named input to a derivation certificate, with its assumption-strength label.
///
/// This replaces the bare `(String, f64)` input pairs in `DerivedCertificate.inputs`.
/// For backward compatibility, the plain `(String, f64)` form is still supported and
/// defaults to `AssumptionStrength::PublishedLiterature` (the least generous safe default).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputProvenance {
    /// Name of the input (matched case-insensitively against the relation's required inputs).
    pub name: String,
    /// Numeric value of the input.
    pub value: f64,
    /// Where this value came from.
    pub strength: AssumptionStrength,
}

impl InputProvenance {
    pub fn new(name: impl Into<String>, value: f64, strength: AssumptionStrength) -> Self {
        InputProvenance {
            name: name.into(),
            value,
            strength,
        }
    }

    /// Construct with the safe default strength (PublishedLiterature) when strength is unknown.
    pub fn unclassified(name: impl Into<String>, value: f64) -> Self {
        Self::new(name, value, AssumptionStrength::PublishedLiterature)
    }
}

/// What kind of reasoning a single step in a derivation represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceStepKind {
    /// State an assumption or import a result from literature/theory.
    Assumption,
    /// Algebraic manipulation (expand, factor, simplify).
    Algebraic,
    /// Substitute one expression into another (including taking a limit in a parameter).
    Substitution,
    /// Take a limit (e.g. GR limit as r_c → ∞, weak-field limit).
    Limit,
    /// Apply a known theorem (e.g. Euler–Lagrange, Bianchi identity).
    TheoremApplication,
    /// Numerical evaluation (e.g. evaluate at Planck best-fit parameters).
    Numerical,
}

/// One step in a derivation. Together, a sequence of steps forms a `DerivationTrace`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraceStep {
    /// What kind of reasoning this step uses.
    pub kind: TraceStepKind,
    /// The assumption strength of any value introduced at this step.
    /// For `Algebraic`/`Substitution`/`Limit` steps, this is `Axiom` (pure math).
    /// For `Assumption` steps, this records the strength of the cited value.
    pub assumption_strength: AssumptionStrength,
    /// Human-readable statement of what this step asserts.
    pub statement: String,
    /// Optional LaTeX rendering of the expression (for paper generation and review).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub latex: String,
}

impl TraceStep {
    /// Construct a pure algebraic step (no external assumption).
    pub fn algebraic(statement: impl Into<String>) -> Self {
        TraceStep {
            kind: TraceStepKind::Algebraic,
            assumption_strength: AssumptionStrength::Axiom,
            statement: statement.into(),
            latex: String::new(),
        }
    }

    /// Construct an assumption step, citing its strength.
    pub fn assumption(statement: impl Into<String>, strength: AssumptionStrength) -> Self {
        TraceStep {
            kind: TraceStepKind::Assumption,
            assumption_strength: strength,
            statement: statement.into(),
            latex: String::new(),
        }
    }

    /// True when this step introduces anti-laundering-killed content.
    pub fn is_laundering(&self) -> bool {
        self.assumption_strength.is_laundering()
    }
}

/// The verdict after running a trace through the anti-laundering gate and consistency checks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "verdict")]
pub enum TraceVerdict {
    /// All steps pass; the trace is internally consistent.
    Verified,
    /// Incomplete — steps were provided but they don't form a closed derivation.
    /// Reports as unverified but not killed.
    Incomplete,
    /// A step introduces a laundered (scored-data-estimate) value.
    /// Hard kill — the certificate loses all derivation credit.
    AntiLaunderingKill { step_index: usize, detail: String },
}

/// A typed, content-addressed sequence of derivation steps for a certificate.
///
/// The trace is the audit trail that replaces the unstructured `mechanism: String` text.
/// It carries enough structure for `trace_rigor()` to compute a score and for the
/// anti-laundering gate to kill certificates that launder scored-data estimates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DerivationTrace {
    /// Ordered derivation steps.
    pub steps: Vec<TraceStep>,
    /// SHA-256 of the canonical JSON of `steps`. Content-addresses the trace so two runs
    /// with the same derivation have the same receipt hash.
    pub content_hash: String,
}

impl DerivationTrace {
    /// Build a trace from steps, computing the content hash.
    pub fn new(steps: Vec<TraceStep>) -> Self {
        let content_hash = compute_trace_hash(&steps);
        DerivationTrace {
            steps,
            content_hash,
        }
    }

    /// Run the anti-laundering gate. Returns the verdict.
    pub fn verdict(&self) -> TraceVerdict {
        for (i, step) in self.steps.iter().enumerate() {
            if step.is_laundering() {
                return TraceVerdict::AntiLaunderingKill {
                    step_index: i,
                    detail: format!(
                        "step {} ({:?}) introduces a scored_data_estimate: \"{}\"",
                        i, step.kind, step.statement
                    ),
                };
            }
        }
        TraceVerdict::Verified
    }

    /// The weakest assumption strength across all steps (most restrictive for rigor capping).
    /// Returns `AssumptionStrength::Axiom` for an empty trace (no assumptions = perfect, by
    /// vacuous truth — the trace must be validated by `verdict()` first).
    pub fn weakest_assumption(&self) -> AssumptionStrength {
        self.steps
            .iter()
            .map(|s| s.assumption_strength)
            .max()
            .unwrap_or(AssumptionStrength::Axiom)
    }
}

/// Anti-laundering check on a set of `InputProvenance` inputs.
///
/// Returns `None` when clean, or `Some(detail)` with the violation description when
/// any input's strength is `ScoredDataEstimate`.
pub fn check_input_provenance(inputs: &[InputProvenance]) -> Option<String> {
    for inp in inputs {
        if inp.strength.is_laundering() {
            return Some(format!(
                "input '{}' = {:.6} has strength `scored_data_estimate`; \
                 a value extracted from the scored dataset cannot earn derivation credit \
                 (anti-laundering rule S01/S11)",
                inp.name, inp.value
            ));
        }
    }
    None
}

/// Policy parameters for `trace_rigor()`. Caller may use `TraceRigorPolicy::default()`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraceRigorPolicy {
    /// ln-normalization scale for the depth factor D.
    ///
    /// D = min(1.0, ln(n_steps + 1) / depth_ln_scale).
    /// Default: ln(10) ≈ 2.303 — a 10-step trace reaches D = 1.0.
    pub depth_ln_scale: f64,
}

impl Default for TraceRigorPolicy {
    fn default() -> Self {
        TraceRigorPolicy {
            depth_ln_scale: std::f64::consts::LN_10,
        }
    }
}

/// Compute the machine-derived rigor score for a derivation trace.
///
/// The score is a product of six bounded factors T·M·E·D·P, capped by the weakest
/// assumption's rigor ceiling (from SYNTHESIS #11 / S01 hard caps):
///
/// - **T** (theoremness): fraction of steps that are theorem-backed
///   (`TheoremApplication`, `Algebraic`, `Limit`) — pure-math steps need no assumptions.
/// - **M** (mechanistic chain): fraction of steps that manipulate expressions
///   (`Algebraic`, `Substitution`, `Limit`, `Numerical`).
/// - **E** (expression completeness): 0.3 base + 0.7 × (fraction of steps with non-empty LaTeX).
///   Traces without any LaTeX earn partial credit.
/// - **D** (derivation depth): ln(n+1) / policy.depth_ln_scale, capped at 1.0.
/// - **P** (provenance consistency): 1.0 if all assumption steps share the same strength class;
///   0.8 for two distinct classes; 0.6 for three or more.
///
/// The product is then capped by the weakest assumption's `rigor_cap()`:
/// - Axiom / TheoremFromAxiom: cap 1.0
/// - PublishedLiterature: cap 0.75
/// - ExternalMeasurement: cap 0.25
/// - PhenomenologicalFit: cap 0.45
/// - ScoredDataEstimate: 0.0 (hard kill, no score returned)
///
/// An empty trace returns 0.0. A trace killed by the anti-laundering gate returns 0.0.
pub fn trace_rigor(trace: &DerivationTrace, policy: &TraceRigorPolicy) -> f64 {
    if trace.steps.is_empty() {
        return 0.0;
    }

    let a_cap = match trace.weakest_assumption().rigor_cap() {
        None => return 0.0, // anti-laundering kill
        Some(cap) => cap,
    };

    let n = trace.steps.len() as f64;

    // T: theoremness — non-assumption steps that are mathematically grounded.
    let theorem_count = trace
        .steps
        .iter()
        .filter(|s| {
            matches!(
                s.kind,
                TraceStepKind::TheoremApplication | TraceStepKind::Algebraic | TraceStepKind::Limit
            )
        })
        .count() as f64;
    let t = theorem_count / n;

    // M: mechanistic chain — steps that actively transform or apply expressions.
    // TheoremApplication counts: applying a theorem is mechanistic work, not mere assertion.
    let mechanic_count = trace
        .steps
        .iter()
        .filter(|s| {
            matches!(
                s.kind,
                TraceStepKind::TheoremApplication
                    | TraceStepKind::Algebraic
                    | TraceStepKind::Substitution
                    | TraceStepKind::Limit
                    | TraceStepKind::Numerical
            )
        })
        .count() as f64;
    let m = mechanic_count / n;

    // E: expression completeness — fraction of steps with a non-empty LaTeX rendering.
    let latex_count = trace.steps.iter().filter(|s| !s.latex.is_empty()).count() as f64;
    let e = 0.3 + 0.7 * (latex_count / n);

    // D: derivation depth — log-normalised step count.
    let d = (f64::ln(n + 1.0) / policy.depth_ln_scale).min(1.0);

    // P: provenance consistency — penalty for mixing assumption strength classes.
    let distinct_strength_classes: std::collections::BTreeSet<u8> = trace
        .steps
        .iter()
        .filter(|s| s.kind == TraceStepKind::Assumption)
        .map(|s| s.assumption_strength as u8)
        .collect();
    let p = match distinct_strength_classes.len() {
        0 | 1 => 1.0,
        2 => 0.8,
        _ => 0.6,
    };

    (t * m * e * d * p).min(a_cap)
}

fn compute_trace_hash(steps: &[TraceStep]) -> String {
    let canonical = serde_json::to_string(steps).unwrap_or_default();
    crate::sha256_digest(canonical.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- AssumptionStrength ----

    #[test]
    fn scored_data_estimate_is_laundering() {
        assert!(AssumptionStrength::ScoredDataEstimate.is_laundering());
    }

    #[test]
    fn other_strengths_are_not_laundering() {
        for s in [
            AssumptionStrength::Axiom,
            AssumptionStrength::TheoremFromAxiom,
            AssumptionStrength::PublishedLiterature,
            AssumptionStrength::ExternalMeasurement,
            AssumptionStrength::PhenomenologicalFit,
        ] {
            assert!(!s.is_laundering(), "{s} must not be laundering");
        }
    }

    #[test]
    fn scored_data_estimate_has_no_rigor_cap() {
        assert!(AssumptionStrength::ScoredDataEstimate.rigor_cap().is_none());
    }

    #[test]
    fn external_measurement_capped_at_0_25() {
        assert_eq!(
            AssumptionStrength::ExternalMeasurement.rigor_cap(),
            Some(0.25)
        );
    }

    #[test]
    fn phenomenological_fit_capped_at_0_45() {
        assert_eq!(
            AssumptionStrength::PhenomenologicalFit.rigor_cap(),
            Some(0.45)
        );
    }

    // ---- check_input_provenance ----

    #[test]
    fn clean_inputs_pass() {
        let inputs = vec![
            InputProvenance::new("omega_m", 0.315, AssumptionStrength::PublishedLiterature),
            InputProvenance::new("h", 0.674, AssumptionStrength::PublishedLiterature),
        ];
        assert!(check_input_provenance(&inputs).is_none());
    }

    #[test]
    fn scored_data_estimate_input_fails() {
        let inputs = vec![
            InputProvenance::new("omega_m", 0.315, AssumptionStrength::PublishedLiterature),
            InputProvenance::new(
                "best_fit_mu0",
                -0.05,
                AssumptionStrength::ScoredDataEstimate,
            ),
        ];
        let result = check_input_provenance(&inputs);
        assert!(result.is_some());
        assert!(result.unwrap().contains("scored_data_estimate"));
    }

    // ---- DerivationTrace ----

    #[test]
    fn clean_trace_verifies() {
        let t = DerivationTrace::new(vec![
            TraceStep::assumption(
                "G_eff from nDGP action",
                AssumptionStrength::TheoremFromAxiom,
            ),
            TraceStep::algebraic("substitute r_c into G_eff formula"),
        ]);
        assert_eq!(t.verdict(), TraceVerdict::Verified);
    }

    #[test]
    fn laundering_step_kills() {
        let t = DerivationTrace::new(vec![
            TraceStep::assumption(
                "omega_m = 0.31 (fit)",
                AssumptionStrength::ScoredDataEstimate,
            ),
            TraceStep::algebraic("apply G_eff formula"),
        ]);
        assert!(matches!(
            t.verdict(),
            TraceVerdict::AntiLaunderingKill { step_index: 0, .. }
        ));
    }

    #[test]
    fn laundering_step_anywhere_kills() {
        let t = DerivationTrace::new(vec![
            TraceStep::algebraic("derive G_eff"),
            TraceStep::assumption("omega_m fitted", AssumptionStrength::ScoredDataEstimate),
            TraceStep::algebraic("evaluate"),
        ]);
        assert!(matches!(
            t.verdict(),
            TraceVerdict::AntiLaunderingKill { step_index: 1, .. }
        ));
    }

    #[test]
    fn weakest_assumption_is_most_restrictive() {
        let t = DerivationTrace::new(vec![
            TraceStep::assumption("axiom", AssumptionStrength::Axiom),
            TraceStep::assumption("from Planck fit", AssumptionStrength::ExternalMeasurement),
        ]);
        assert_eq!(
            t.weakest_assumption(),
            AssumptionStrength::ExternalMeasurement
        );
    }

    #[test]
    fn content_hash_is_stable_and_non_empty() {
        let t = DerivationTrace::new(vec![TraceStep::algebraic("G_eff")]);
        assert_eq!(t.content_hash.len(), 64);
        let t2 = DerivationTrace::new(vec![TraceStep::algebraic("G_eff")]);
        assert_eq!(t.content_hash, t2.content_hash);
    }

    #[test]
    fn different_traces_have_different_hashes() {
        let t1 = DerivationTrace::new(vec![TraceStep::algebraic("step A")]);
        let t2 = DerivationTrace::new(vec![TraceStep::algebraic("step B")]);
        assert_ne!(t1.content_hash, t2.content_hash);
    }

    #[test]
    fn empty_trace_weakest_assumption_is_axiom() {
        let t = DerivationTrace::new(vec![]);
        assert_eq!(t.weakest_assumption(), AssumptionStrength::Axiom);
    }

    // ---- trace_rigor ----

    fn policy() -> TraceRigorPolicy {
        TraceRigorPolicy::default()
    }

    #[test]
    fn empty_trace_rigor_is_zero() {
        let t = DerivationTrace::new(vec![]);
        assert_eq!(trace_rigor(&t, &policy()), 0.0);
    }

    #[test]
    fn laundering_trace_rigor_is_zero() {
        let t = DerivationTrace::new(vec![
            TraceStep::assumption("fitted mu0", AssumptionStrength::ScoredDataEstimate),
            TraceStep::algebraic("apply G_eff"),
        ]);
        assert_eq!(trace_rigor(&t, &policy()), 0.0);
    }

    #[test]
    fn external_measurement_caps_rigor_at_0_25() {
        let t = DerivationTrace::new(vec![
            TraceStep::assumption("omega_m from CMB", AssumptionStrength::ExternalMeasurement),
            TraceStep::algebraic("substitute into G_eff"),
            TraceStep::algebraic("simplify"),
        ]);
        let r = trace_rigor(&t, &policy());
        assert!(
            r <= 0.25 + 1e-12,
            "external measurement must cap at 0.25; got {r}"
        );
        assert!(r > 0.0, "positive rigor expected for a valid trace");
    }

    #[test]
    fn phenomenological_fit_caps_rigor_at_0_45() {
        let t = DerivationTrace::new(vec![
            TraceStep::assumption(
                "G_eff from fitting formula",
                AssumptionStrength::PhenomenologicalFit,
            ),
            TraceStep::algebraic("evaluate"),
        ]);
        let r = trace_rigor(&t, &policy());
        assert!(r <= 0.45 + 1e-12, "phenom cap 0.45; got {r}");
        assert!(r > 0.0);
    }

    #[test]
    fn axiom_only_trace_can_reach_high_rigor() {
        // Many steps, all algebraic + theorem, all with LaTeX → should be close to 1.0.
        let steps: Vec<TraceStep> = (0..10)
            .map(|i| {
                let mut s = TraceStep {
                    kind: TraceStepKind::TheoremApplication,
                    assumption_strength: AssumptionStrength::Axiom,
                    statement: format!("step {i}"),
                    latex: format!("\\alpha_{{{i}}}"),
                };
                // alternate algebraic and theorem steps for M factor
                if i % 2 == 0 {
                    s.kind = TraceStepKind::Algebraic;
                }
                s
            })
            .collect();
        let t = DerivationTrace::new(steps);
        let r = trace_rigor(&t, &policy());
        assert!(
            r > 0.5,
            "high-quality axiom trace should score > 0.5; got {r}"
        );
        assert!(r <= 1.0 + 1e-12);
    }

    #[test]
    fn mixed_assumption_classes_penalise_p_factor() {
        // Mix Axiom and ExternalMeasurement → two classes → P = 0.8;
        // cap from ExternalMeasurement (0.25) limits the ceiling anyway.
        let t = DerivationTrace::new(vec![
            TraceStep::assumption("axiom", AssumptionStrength::Axiom),
            TraceStep::assumption("measured H0", AssumptionStrength::ExternalMeasurement),
            TraceStep::algebraic("combine"),
        ]);
        let r = trace_rigor(&t, &policy());
        // Cap 0.25 applies; score positive but capped.
        assert!(r <= 0.25 + 1e-12);
        assert!(r >= 0.0);
    }

    #[test]
    fn policy_default_depth_scale_is_ln10() {
        let p = TraceRigorPolicy::default();
        assert!((p.depth_ln_scale - std::f64::consts::LN_10).abs() < 1e-12);
    }
}
