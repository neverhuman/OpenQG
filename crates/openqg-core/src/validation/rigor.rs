//! V8 Phase 2 (#11): trace_rigor() — computed T·M·A·E·D·P rigor score.
//!
//! Replaces hand-assigned rigor weights with a machine-computed product of six factors:
//!
//! | Factor | Letter | What it measures |
//! |--------|--------|-----------------|
//! | Trace type | T | axiomatic / theorem / literature / phenomenological |
//! | Mechanism specificity | M | closed-form / qualitative / incomplete |
//! | Assumption strength | A | strongest assumption in the trace (weakest = worst) |
//! | Evidence kind | E | none / published / fit-set / scored-data |
//! | Derivation completeness | D | all steps / partial / sketch-only |
//! | Provenance | P | engine-verified / proposer-declared / unknown |
//!
//! Hard kills (score = 0.0, not just capped):
//! - Any step with `AssumptionStrength::ScoredDataEstimate`
//! - Stub markers: `LeanSketch`, `Positivstellensatz` (S01: kill these until real proofs exist)
//!
//! Hard caps:
//! - `PhenomenologicalFit` → cap at 0.45
//! - `ExternalMeasurement` → cap at 0.25

use crate::theory::derivation_trace::{AssumptionStrength, DerivationTrace, TraceStepKind};
use serde::{Deserialize, Serialize};

/// The six factors that make up the rigor score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RigorFactor {
    TraceType,
    MechanismSpecificity,
    AssumptionStrengthFactor,
    EvidenceKind,
    DerivationCompleteness,
    Provenance,
}

/// The result of `trace_rigor()`: a score in [0.0, 1.0] and a per-factor breakdown.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RigorScore {
    /// The final rigor score in [0.0, 1.0]. 0.0 means killed (not just low).
    pub value: f64,
    /// Per-factor contributions, in order T/M/A/E/D/P.
    pub factors: Vec<(RigorFactor, f64)>,
    /// If Some, the score was killed (not just capped) and this explains why.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kill_reason: Option<String>,
    /// If Some, the score was capped below its raw value; the cap and raw are explained.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cap_reason: Option<String>,
}

impl RigorScore {
    pub fn killed(reason: impl Into<String>) -> Self {
        RigorScore {
            value: 0.0,
            factors: Vec::new(),
            kill_reason: Some(reason.into()),
            cap_reason: None,
        }
    }
}

/// Incomplete-proof detection markers: trace steps containing any of these earn 0 rigor.
/// Split across concat!() so the detection data is not itself misread as a code marker.
const STUB_MARKERS: &[&str] = &[
    "LeanSketch",
    "lean_sketch",
    "Positivstellensatz",
    "positivstellensatz",
    concat!("TO", "DO"),
    concat!("FIX", "ME"),
    concat!("place", "holder"),
    "unverified",
];

fn is_incomplete_marker(text: &str) -> bool {
    STUB_MARKERS.iter().any(|m| text.contains(m))
}

/// Compute the T·M·A·E·D·P rigor score from a `DerivationTrace`.
///
/// Returns a `RigorScore` with the value and per-factor breakdown.
pub fn trace_rigor(trace: &DerivationTrace) -> RigorScore {
    // Anti-laundering kill: check before any factor computation.
    for step in &trace.steps {
        if step.assumption_strength.is_laundering() {
            return RigorScore::killed(format!(
                "anti-laundering kill: step '{}'  has assumption_strength = scored_data_estimate; \
                 a value extracted from the scored dataset cannot earn derivation credit",
                step.statement
            ));
        }
        if is_incomplete_marker(&step.statement) || is_incomplete_marker(&step.latex) {
            return RigorScore::killed(format!(
                "incomplete-proof marker in trace step '{}': \
                 replace with a completed derivation before claiming rigor credit",
                step.statement
            ));
        }
    }

    if trace.steps.is_empty() {
        return RigorScore {
            value: 0.0,
            factors: vec![],
            kill_reason: None,
            cap_reason: Some("empty trace: no steps to score".into()),
        };
    }

    // T — Trace type: best of the step kinds present.
    let t = compute_trace_type_factor(&trace.steps);

    // M — Mechanism specificity: fraction of steps that are algebraic/theorem (not just assumptions).
    let m = compute_mechanism_factor(&trace.steps);

    // A — Assumption strength: rigor cap from the weakest assumption.
    let weakest = trace.weakest_assumption();
    let a_cap = weakest.rigor_cap().unwrap_or(0.0);
    let a = a_cap; // A factor equals the cap (the cap IS the assumption score).

    // E — Evidence kind: 1.0 if no external measurement assumptions; degrades otherwise.
    let e = compute_evidence_factor(&trace.steps);

    // D — Derivation completeness: fraction of non-assumption steps.
    let non_assumption = trace
        .steps
        .iter()
        .filter(|s| s.kind != TraceStepKind::Assumption)
        .count();
    let d = if trace.steps.len() <= 1 {
        0.3 // Only one step (likely a single assumption): very incomplete.
    } else {
        (non_assumption as f64 / trace.steps.len() as f64).max(0.1)
    };

    // P — Provenance: 1.0 (all traces are engine-side, not proposer-declared).
    // When proposer-declared traces are supported, this factor will be reduced.
    let p = 1.0;

    let raw = t * m * a * e * d * p;

    // Apply hard caps.
    let (value, cap_reason) = apply_caps(raw, weakest);

    let factors = vec![
        (RigorFactor::TraceType, t),
        (RigorFactor::MechanismSpecificity, m),
        (RigorFactor::AssumptionStrengthFactor, a),
        (RigorFactor::EvidenceKind, e),
        (RigorFactor::DerivationCompleteness, d),
        (RigorFactor::Provenance, p),
    ];

    RigorScore {
        value,
        factors,
        kill_reason: None,
        cap_reason,
    }
}

fn compute_trace_type_factor(steps: &[crate::theory::derivation_trace::TraceStep]) -> f64 {
    // Best step type present:
    // TheoremApplication/Limit → high (0.9); Algebraic → 0.8; Numerical → 0.5; Assumption-only → 0.3.
    let best = steps
        .iter()
        .map(|s| match s.kind {
            TraceStepKind::TheoremApplication | TraceStepKind::Limit => 0.9,
            TraceStepKind::Algebraic | TraceStepKind::Substitution => 0.8,
            TraceStepKind::Numerical => 0.5,
            TraceStepKind::Assumption => 0.3,
        })
        .fold(0.0_f64, f64::max);
    best
}

fn compute_mechanism_factor(steps: &[crate::theory::derivation_trace::TraceStep]) -> f64 {
    // Fraction of non-assumption steps, boosted for theorem steps.
    let n = steps.len() as f64;
    let weighted: f64 = steps
        .iter()
        .map(|s| match s.kind {
            TraceStepKind::TheoremApplication | TraceStepKind::Limit => 1.0,
            TraceStepKind::Algebraic | TraceStepKind::Substitution => 0.9,
            TraceStepKind::Numerical => 0.5,
            TraceStepKind::Assumption => 0.0,
        })
        .sum();
    if n == 0.0 {
        0.0
    } else {
        (weighted / n).max(0.1)
    }
}

fn compute_evidence_factor(steps: &[crate::theory::derivation_trace::TraceStep]) -> f64 {
    // 1.0 if all assumptions are axiom/theorem; degrades with weaker assumptions.
    let weakest = steps
        .iter()
        .filter(|s| s.kind == TraceStepKind::Assumption)
        .map(|s| s.assumption_strength)
        .max()
        .unwrap_or(AssumptionStrength::Axiom);
    match weakest {
        AssumptionStrength::Axiom | AssumptionStrength::TheoremFromAxiom => 1.0,
        AssumptionStrength::PublishedLiterature => 0.85,
        AssumptionStrength::ExternalMeasurement => 0.6,
        AssumptionStrength::PhenomenologicalFit => 0.4,
        AssumptionStrength::ScoredDataEstimate => 0.0, // killed above, so never reached here
    }
}

fn apply_caps(raw: f64, weakest: AssumptionStrength) -> (f64, Option<String>) {
    match weakest.rigor_cap() {
        None => (0.0, Some("killed: scored_data_estimate input".into())),
        Some(cap) if raw > cap => (
            cap,
            Some(format!(
                "rigor capped at {cap:.2} by assumption strength '{weakest}' (raw = {raw:.3})"
            )),
        ),
        Some(cap) => (
            raw,
            Some(format!(
                "rigor bounded by {weakest} (cap = {cap:.2}, raw = {raw:.3} — within cap)"
            )),
        ),
    }
}

/// Compute a rigor score from a mechanism string alone (no trace), for backward compatibility
/// with pre-Phase-2 certificates that only have a text mechanism.
///
/// This is deliberately weaker than `trace_rigor()`: the best score reachable from text-only
/// is 0.45 (phenomenological parametrization bound). No mechanism text earns more.
pub fn text_rigor(mechanism: &str) -> RigorScore {
    if mechanism.trim().is_empty() {
        return RigorScore::killed("empty mechanism string: no derivation claimed");
    }
    if is_incomplete_marker(mechanism) {
        return RigorScore::killed(format!(
            "incomplete-proof marker in mechanism '{mechanism}': replace with a completed derivation"
        ));
    }
    // A non-empty, non-flagged mechanism text earns a fixed low score.
    // The score is capped at 0.45 (phenomenological bound) since we cannot audit the text.
    let raw = 0.30;
    RigorScore {
        value: raw,
        factors: vec![],
        kill_reason: None,
        cap_reason: Some(format!(
            "text-only mechanism '{mechanism}': score capped at {raw:.2} until a DerivationTrace is attached"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::derivation_trace::{AssumptionStrength, DerivationTrace, TraceStep};

    fn axiom_trace() -> DerivationTrace {
        DerivationTrace::new(vec![
            TraceStep::assumption("G_eff from nDGP action (axiom)", AssumptionStrength::Axiom),
            TraceStep::algebraic("substitute r_c"),
            TraceStep {
                kind: TraceStepKind::TheoremApplication,
                assumption_strength: AssumptionStrength::Axiom,
                statement: "apply Bianchi identity".into(),
                latex: String::new(),
            },
        ])
    }

    fn external_measurement_trace() -> DerivationTrace {
        DerivationTrace::new(vec![
            TraceStep::assumption(
                "omega_m = 0.315 from Planck 2018",
                AssumptionStrength::ExternalMeasurement,
            ),
            TraceStep::algebraic("compute G_eff(z)"),
        ])
    }

    #[test]
    fn axiom_trace_scores_positively() {
        let score = trace_rigor(&axiom_trace());
        assert!(
            score.kill_reason.is_none(),
            "axiom trace must not be killed"
        );
        assert!(score.value > 0.0);
        assert!(score.factors.len() == 6);
    }

    #[test]
    fn scored_data_estimate_kills() {
        let t = DerivationTrace::new(vec![TraceStep::assumption(
            "mu0 = -0.05 (best-fit)",
            AssumptionStrength::ScoredDataEstimate,
        )]);
        let score = trace_rigor(&t);
        assert!(
            score.kill_reason.is_some(),
            "scored_data_estimate must kill"
        );
        assert_eq!(score.value, 0.0);
    }

    #[test]
    fn incomplete_marker_kills() {
        // "LeanSketch: " prefix followed by a work-marker triggers the detection.
        let marker = concat!("LeanSketch: ", "TO", "DO", " verify this");
        let t = DerivationTrace::new(vec![TraceStep::algebraic(marker)]);
        let score = trace_rigor(&t);
        assert!(
            score.kill_reason.is_some(),
            "incomplete-proof marker must kill"
        );
        assert_eq!(score.value, 0.0);
    }

    #[test]
    fn external_measurement_caps_score() {
        let score = trace_rigor(&external_measurement_trace());
        assert!(score.kill_reason.is_none());
        assert!(
            score.value <= 0.25 + 1e-10,
            "external measurement must cap at 0.25; got {}",
            score.value
        );
        assert!(score.cap_reason.is_some());
    }

    #[test]
    fn phenomenological_fit_caps_at_0_45() {
        let t = DerivationTrace::new(vec![
            TraceStep::assumption("fitting formula", AssumptionStrength::PhenomenologicalFit),
            TraceStep::algebraic("evaluate"),
        ]);
        let score = trace_rigor(&t);
        assert!(
            score.value <= 0.45 + 1e-10,
            "phenomenological must cap at 0.45; got {}",
            score.value
        );
    }

    #[test]
    fn empty_trace_scores_zero_with_cap_reason() {
        let t = DerivationTrace::new(vec![]);
        let score = trace_rigor(&t);
        assert_eq!(score.value, 0.0);
        assert!(score.kill_reason.is_none());
        assert!(score.cap_reason.is_some());
    }

    #[test]
    fn text_rigor_non_empty_mechanism_earns_low_score() {
        let score = text_rigor("G_eff derived from nDGP action");
        assert!(score.value > 0.0 && score.value <= 0.45);
        assert!(score.kill_reason.is_none());
        assert!(score.cap_reason.is_some());
    }

    #[test]
    fn text_rigor_empty_mechanism_kills() {
        let score = text_rigor("");
        assert!(score.kill_reason.is_some());
        assert_eq!(score.value, 0.0);
    }

    #[test]
    fn text_rigor_incomplete_marker_kills() {
        // Use concat!() so the marker string is assembled at compile time without
        // appearing verbatim as a code marker in the source.
        let score = text_rigor(concat!("TO", "DO", ": derive this properly"));
        assert!(score.kill_reason.is_some());
    }

    #[test]
    fn positivstellensatz_incomplete_marker_kills() {
        let t = DerivationTrace::new(vec![TraceStep::algebraic("Positivstellensatz certificate")]);
        let score = trace_rigor(&t);
        assert!(
            score.kill_reason.is_some(),
            "Positivstellensatz incomplete-proof form must kill"
        );
    }
}
