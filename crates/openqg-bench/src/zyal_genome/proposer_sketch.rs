//! V6: the **ProposalSketch** — a flat, LLM-designed proposal schema + its deterministic expander.
//!
//! V5 exposed the raw serde shapes (`Provenance` string-or-object enums, tuple-array certificate
//! inputs, dynamic evidence maps) to LLMs and paid for it in parse failures and repair loops. The
//! sketch is the designed-for-LLM input contract: every field required, `additionalProperties:
//! false` everywhere, typed sentinels (`""`, `0.0`, `[]`) instead of optionals. The jnoccio router
//! validates responses against [`proposal_sketch_schema`] **server-side** (strict structured
//! output with its own repair loop), and [`expand_sketch`] — pure, deterministic, unit-tested —
//! turns a sketch into the [`ProposalDoc`] the oracle scores.
//!
//! Anti-drift locks (tests below): the schema's enum strings are generated from
//! [`registered_relations`] and the actual serde serializations, and `fixture_sketch()` must
//! expand to a doc that scores EXACTLY like `fixture_proposal()`.

use std::collections::BTreeMap;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use openqg_core::theory::{
    registered_relations, Claim, ClaimKind, DerivationObligation, DerivationObligationKind,
    DerivedCertificate, LimitWitness, NovelPredictionWitness, Parameter, Provenance, Sector,
    SharedParam, Theory, UnificationClaim,
};
use openqg_core::{sha256_digest, EvidenceRef};

use super::proposer::ProposalDoc;

// ---------------------------------------------------------------------------
// Sketch types — flat, sentinel-based, strict-schema friendly.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ProposalSketch {
    pub theory_id: String,
    /// 1–3 sentence mechanism summary (also the anti-monoculture signature text).
    pub summary: String,
    /// Echo of the engine-assigned lane (audit only; the ledgered value is the engine's).
    pub mechanism_lane: String,
    pub parameters: Vec<SketchParam>,
    pub background: SketchBackground,
    pub claims: Vec<SketchClaim>,
    pub obligations: Vec<SketchObligation>,
    pub shared_params: Vec<SketchShared>,
    pub evidence: Vec<SketchEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SketchParam {
    pub symbol: String,
    pub value: f64,
    pub meaning: String,
    /// "fundamental" | "derived_certified" | "derived_textual"
    pub provenance_kind: String,
    /// Mechanism text for derived provenance ("" when fundamental).
    pub mechanism: String,
    /// Registry relation name for "derived_certified" ("" otherwise). Schema-enum-pinned, so an
    /// invented relation is a ROUTER-side rejection, not an oracle kill.
    pub relation: String,
    pub inputs: Vec<SketchInput>,
    pub expected: f64,
    pub tolerance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SketchInput {
    pub name: String,
    pub value: f64,
}

/// The flat modified-gravity surface; every other background coordinate stays pinned to
/// Planck-ΛCDM (background drift is a costed dof — propose it as a derived parameter instead).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SketchBackground {
    /// "none" | "fr_hu_sawicki" | "ndgp"
    pub mg_family: String,
    pub mu0: f64,
    pub fr_n: f64,
    pub fr_log10_fr0: f64,
    pub ndgp_omega_rc: f64,
    pub w0: f64,
    pub wa: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SketchClaim {
    pub id: String,
    pub sector: String,
    /// "physics" | "engineering"
    pub kind: String,
    pub statement: String,
    pub evidence_paths: Vec<String>,
    pub obligations: Vec<String>,
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SketchObligation {
    pub claim_id: String,
    pub kind: String,
    pub detail: String,
    // numeric_witness / symbolic_identity / dimensional:
    pub relation: String,
    pub inputs: Vec<SketchInput>,
    pub expected: f64,
    pub tolerance: f64,
    // limit:
    pub limit_name: String,
    pub residual: f64,
    pub bound: f64,
    // literature_equivalence:
    pub citation: String,
    // novel_prediction:
    pub observable: String,
    pub predicted: f64,
    pub baseline: f64,
    pub min_detectable: f64,
    pub falsifier: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SketchShared {
    pub symbol: String,
    pub value: f64,
    pub sectors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SketchEvidence {
    pub path: String,
    pub content: String,
}

// ---------------------------------------------------------------------------
// Enum string sources — generated from the serde truth, never hand-listed twice.
// ---------------------------------------------------------------------------

fn sector_strings() -> Vec<String> {
    [
        Sector::Background,
        Sector::Growth,
        Sector::TensorSector,
        Sector::ScreeningPpn,
        Sector::Bbn,
        Sector::Particle,
        Sector::Quantum,
    ]
    .iter()
    .map(|s| serde_enum_string(s))
    .collect()
}

fn obligation_kind_strings() -> Vec<String> {
    // The supported kinds only — positivstellensatz/lean_sketch are machine-unverified and earn
    // nothing; excluding them from the schema steers the model away at generation time.
    [
        DerivationObligationKind::Dimensional,
        DerivationObligationKind::Limit,
        DerivationObligationKind::SymbolicIdentity,
        DerivationObligationKind::NumericWitness,
        DerivationObligationKind::LiteratureEquivalence,
        DerivationObligationKind::NovelPrediction,
    ]
    .iter()
    .map(|k| serde_enum_string(k))
    .collect()
}

fn mg_family_strings() -> Vec<String> {
    vec!["none".into(), "fr_hu_sawicki".into(), "ndgp".into()]
}

fn serde_enum_string<T: Serialize>(v: &T) -> String {
    serde_json::to_value(v)
        .ok()
        .and_then(|x| x.as_str().map(str::to_string))
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// The strict JSON schema (router-validated).
// ---------------------------------------------------------------------------

fn strict_obj(properties: Value) -> Value {
    let required: Vec<String> = properties
        .as_object()
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default();
    json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false,
    })
}

fn arr(items: Value) -> Value {
    json!({"type": "array", "items": items})
}

fn s() -> Value {
    json!({"type": "string"})
}
fn n() -> Value {
    json!({"type": "number"})
}
fn s_enum(values: Vec<String>) -> Value {
    json!({"type": "string", "enum": values})
}

/// The complete strict schema for [`ProposalSketch`], with relation/sector/kind enums injected
/// from the registry + serde truth.
pub(crate) fn proposal_sketch_schema() -> Value {
    let mut relations: Vec<String> = registered_relations()
        .into_iter()
        .map(str::to_string)
        .collect();
    relations.push(String::new()); // "" sentinel = no relation
    let input = strict_obj(json!({"name": s(), "value": n()}));
    strict_obj(json!({
        "theory_id": s(),
        "summary": s(),
        "mechanism_lane": s(),
        "parameters": arr(strict_obj(json!({
            "symbol": s(),
            "value": n(),
            "meaning": s(),
            "provenance_kind": s_enum(vec!["fundamental".into(), "derived_certified".into(), "derived_textual".into()]),
            "mechanism": s(),
            "relation": s_enum(relations.clone()),
            "inputs": arr(input.clone()),
            "expected": n(),
            "tolerance": n(),
        }))),
        "background": strict_obj(json!({
            "mg_family": s_enum(mg_family_strings()),
            "mu0": n(),
            "fr_n": n(),
            "fr_log10_fr0": n(),
            "ndgp_omega_rc": n(),
            "w0": n(),
            "wa": n(),
        })),
        "claims": arr(strict_obj(json!({
            "id": s(),
            "sector": s_enum(sector_strings()),
            "kind": s_enum(vec!["physics".into(), "engineering".into()]),
            "statement": s(),
            "evidence_paths": arr(s()),
            "obligations": arr(s()),
            "depends_on": arr(s()),
        }))),
        "obligations": arr(strict_obj(json!({
            "claim_id": s(),
            "kind": s_enum(obligation_kind_strings()),
            "detail": s(),
            "relation": s_enum(relations),
            "inputs": arr(input),
            "expected": n(),
            "tolerance": n(),
            "limit_name": s(),
            "residual": n(),
            "bound": n(),
            "citation": s(),
            "observable": s(),
            "predicted": n(),
            "baseline": n(),
            "min_detectable": n(),
            "falsifier": s(),
        }))),
        "shared_params": arr(strict_obj(json!({
            "symbol": s(),
            "value": n(),
            "sectors": arr(s_enum(sector_strings())),
        }))),
        "evidence": arr(strict_obj(json!({"path": s(), "content": s()}))),
    }))
}

// ---------------------------------------------------------------------------
// The expander — pure, deterministic, total over valid sketches.
// ---------------------------------------------------------------------------

fn parse_enum<T: serde::de::DeserializeOwned>(label: &str, raw: &str) -> Result<T> {
    serde_json::from_value(Value::String(raw.to_string()))
        .with_context(|| format!("{label}: `{raw}` is not a recognized value"))
}

fn sketch_inputs(inputs: &[SketchInput]) -> Vec<(String, f64)> {
    inputs.iter().map(|i| (i.name.clone(), i.value)).collect()
}

/// Expand a sketch into the [`ProposalDoc`] the oracle scores. Two calls are byte-identical;
/// out-of-enum strings are hard errors (never silent defaults).
pub(crate) fn expand_sketch(sketch: &ProposalSketch) -> Result<ProposalDoc> {
    if sketch.theory_id.trim().is_empty() {
        bail!("theory_id is empty");
    }
    // Theory: Planck-ΛCDM scaffolding + the sketch's MG surface + its parameters.
    let mut theory = Theory::baseline_lcdm();
    theory.id = sketch.theory_id.clone();
    let bg = &sketch.background;
    theory.background.mg_family = parse_enum("mg_family", &bg.mg_family)?;
    theory.background.mu0 = bg.mu0;
    if bg.fr_n != 0.0 {
        theory.background.fr_n = bg.fr_n;
    }
    if bg.fr_log10_fr0 != 0.0 {
        theory.background.fr_log10_fr0 = bg.fr_log10_fr0;
    }
    theory.background.ndgp_omega_rc = bg.ndgp_omega_rc;
    theory.background.w0 = bg.w0;
    theory.background.wa = bg.wa;

    for p in &sketch.parameters {
        let provenance = match p.provenance_kind.as_str() {
            "fundamental" => Provenance::Fundamental,
            "derived_textual" => Provenance::derived(&p.mechanism),
            "derived_certified" => {
                if p.relation.is_empty() {
                    bail!("parameter {}: derived_certified needs a relation", p.symbol);
                }
                Provenance::derived_certified(
                    &p.mechanism,
                    DerivedCertificate {
                        relation: p.relation.clone(),
                        inputs: sketch_inputs(&p.inputs),
                        expected: p.expected,
                        tolerance: p.tolerance,
                        ..Default::default()
                    },
                )
            }
            other => bail!("parameter {}: unknown provenance_kind `{other}`", p.symbol),
        };
        theory.parameters.push(Parameter {
            symbol: p.symbol.clone(),
            value: p.value,
            physical_meaning: p.meaning.clone(),
            provenance,
        });
    }

    // V7: structure follows the dial — the generating term for every certified MG relation
    // and turned background dial travels with the theory (StructurallyUngenerated otherwise).
    {
        let mut ensure = |name: &str| {
            if !theory.terms.iter().any(|t| t.name == name) {
                theory.terms.push(openqg_core::theory::Term {
                    name: name.into(),
                    mass_dimension: 4,
                    free_lorentz_indices: 0,
                });
            }
        };
        for p in &sketch.parameters {
            match p.relation.as_str() {
                "ndgp_geff_over_g" | "ndgp_beta_from_omega_rc" => ensure("dgp_brane"),
                "fr_alpha_m" | "fr_largescale_geff_over_g" => ensure("f_r_correction"),
                "planck_mu0_geff" => ensure("planck_mu_parametrization"),
                "dark_scattering_growth_drag" => ensure("dark_scattering_coupling"),
                "coupled_de_geff_over_g" => ensure("quintessence_scalar"),
                _ => {}
            }
        }
        if (bg.w0 + 1.0).abs() > 1e-9 || bg.wa.abs() > 1e-9 {
            ensure("quintessence_scalar");
        }
        if bg.mu0.abs() > 1e-12 {
            ensure("planck_mu_parametrization");
        }
        if bg.ndgp_omega_rc > 0.0 {
            ensure("dgp_brane");
        }
    }

    // Evidence: path → content, with refs content-bound by sha256 at expansion time.
    let evidence: BTreeMap<String, String> = sketch
        .evidence
        .iter()
        .map(|e| (e.path.clone(), e.content.clone()))
        .collect();
    let evidence_ref = |path: &str| -> Result<EvidenceRef> {
        let content = evidence
            .get(path)
            .with_context(|| format!("claim cites evidence path `{path}` not in evidence[]"))?;
        Ok(EvidenceRef {
            path: path.to_string(),
            sha256: sha256_digest(content.as_bytes()),
            json_pointer: None,
            byte_range: None,
            event_id: None,
            tier: openqg_core::EvidenceTier::T2,
        })
    };

    let mut claims = Vec::with_capacity(sketch.claims.len());
    for c in &sketch.claims {
        let mut refs = Vec::with_capacity(c.evidence_paths.len());
        for path in &c.evidence_paths {
            refs.push(evidence_ref(path)?);
        }
        claims.push(Claim {
            id: c.id.clone(),
            sector: parse_enum("sector", &c.sector)?,
            kind: parse_enum::<ClaimKind>("claim kind", &c.kind)?,
            statement: c.statement.clone(),
            evidence: refs,
            obligations: c.obligations.clone(),
            depends_on: c.depends_on.clone(),
        });
    }

    let mut obligations = Vec::with_capacity(sketch.obligations.len());
    for o in &sketch.obligations {
        let kind: DerivationObligationKind = parse_enum("obligation kind", &o.kind)?;
        let certificate = matches!(
            kind,
            DerivationObligationKind::NumericWitness
                | DerivationObligationKind::SymbolicIdentity
                | DerivationObligationKind::Dimensional
        )
        .then(|| DerivedCertificate {
            relation: o.relation.clone(),
            inputs: sketch_inputs(&o.inputs),
            expected: o.expected,
            tolerance: o.tolerance,
            ..Default::default()
        })
        .filter(|c| !c.relation.is_empty());
        let limit = (kind == DerivationObligationKind::Limit).then(|| LimitWitness {
            name: o.limit_name.clone(),
            residual: o.residual,
            bound: o.bound,
        });
        let citation = (!o.citation.is_empty()).then(|| o.citation.clone());
        let novel =
            (kind == DerivationObligationKind::NovelPrediction).then(|| NovelPredictionWitness {
                refreshed_by_engine: false,
                // V7 (P1.2): canonicalize at expansion — V6 chunks 1-3 lost 20 points each to
                // spellings like "fsigma8_z0.61" rejected only later at audit time.
                observable: openqg_core::cosmology::canonicalize_observable_id(&o.observable)
                    .map(|c| c.to_id())
                    .unwrap_or_else(|| o.observable.clone()),
                predicted: o.predicted,
                baseline: o.baseline,
                min_detectable: o.min_detectable,
                falsifier: o.falsifier.clone(),
            });
        obligations.push(DerivationObligation {
            claim_id: o.claim_id.clone(),
            kind,
            detail: o.detail.clone(),
            certificate,
            limit,
            citation,
            novel,
            equation_match: None,
        });
    }

    let mut shared = Vec::with_capacity(sketch.shared_params.len());
    for sp in &sketch.shared_params {
        let mut sectors = Vec::with_capacity(sp.sectors.len());
        for sec in &sp.sectors {
            sectors.push(parse_enum("shared sector", sec)?);
        }
        shared.push(SharedParam {
            symbol: sp.symbol.clone(),
            value: sp.value,
            sectors,
        });
    }

    Ok(ProposalDoc {
        theory,
        claims,
        obligations,
        unification: UnificationClaim { shared },
        evidence,
    })
}

/// Parse a router response body (canonical JSON when the router validated it; tolerant of fences
/// otherwise) into a sketch.
pub(crate) fn parse_sketch_response(raw: &str) -> Result<ProposalSketch> {
    let trimmed = raw.trim();
    if let Ok(sk) = serde_json::from_str::<ProposalSketch>(trimmed) {
        return Ok(sk);
    }
    // Fallback: strip markdown fences / take the outermost braces.
    let start = trimmed.find('{').context("no JSON object in response")?;
    let end = trimmed.rfind('}').context("no closing brace in response")?;
    serde_json::from_str(&trimmed[start..=end]).context("parse ProposalSketch")
}

// ---------------------------------------------------------------------------
// Mechanism lanes (per-sample diversity assignments).
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Lane {
    PlanckMu0,
    DarkScattering,
    Free,
    NullDiagnostic,
}

pub(crate) const LANES: [Lane; 4] = [
    Lane::PlanckMu0,
    Lane::DarkScattering,
    Lane::Free,
    Lane::NullDiagnostic,
];

impl Lane {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Lane::PlanckMu0 => "planck_mu0",
            Lane::DarkScattering => "dark_scattering",
            Lane::Free => "free",
            Lane::NullDiagnostic => "null_diagnostic",
        }
    }

    fn brief(self) -> &'static str {
        match self {
            Lane::PlanckMu0 => {
                "Build a suppressed-growth theory in the Planck-μ0 parametrization: mg_family \
                 \"none\", background mu0 < 0, and a parameter certified via `planck_mu0_geff` \
                 whose certificate input mu0 EQUALS the background mu0. VERBATIM-PASSING \
                 example for background mu0 = -0.1 (relation: G_eff/G = 1 + mu0): parameter \
                 {\"symbol\": \"geff_over_g\", \"value\": 0.9, \"provenance_kind\": \
                 \"derived_certified\", \"relation\": \"planck_mu0_geff\", \"inputs\": \
                 [{\"name\": \"mu0\", \"value\": -0.1}], \"expected\": 0.9, \
                 \"tolerance\": 1e-6}. Aim at the negative fσ8/S8 pulls in the DATA BRIEF."
            }
            Lane::DarkScattering => {
                "Build a dark-sector interaction theory (DE–DM momentum exchange, Simpson 2010; \
                 Pourtsidou+ 2013) via the REAL mechanism relation `dark_scattering_growth_drag` \
                 with inputs {a_drag ≥ 0, w0 > −1, omega_de0}: the engine integrates the friction \
                 Γ(a)=A_drag·(1+w(a))·Ω_de(a) in the growth ODE (suppresses growth). Set the \
                 sketch background w0 to the SAME w0 you certify. VERBATIM-PASSING certificate \
                 example (relation Γ₀ = a_drag·(1+w0)·omega_de0): {\"relation\": \
                 \"dark_scattering_growth_drag\", \"inputs\": [{\"name\": \"a_drag\", \
                 \"value\": 2.0}, {\"name\": \"w0\", \"value\": -0.9}, {\"name\": \
                 \"omega_de0\", \"value\": 0.685}], \"expected\": 0.137, \"tolerance\": \
                 1e-6} — your background w0 must then be -0.9 (include term \
                 quintessence_scalar) and you must include term dark_scattering_coupling."
            }
            Lane::Free => {
                "Choose the mechanism YOU judge most promising — any registry relation, any \
                 family. Do not repeat your sibling samples' lanes (planck_mu0, dark_scattering)."
            }
            Lane::NullDiagnostic => {
                "Adversarial control: build the most ΛCDM-degenerate theory that still makes ONE \
                 honestly falsifiable novel prediction at the edge of detectability. Do NOT \
                 fabricate deviations — declared witness numbers are machine-checked against the \
                 engine's computed truth and fabrication is a kill."
            }
        }
    }
}

/// The router prompt: oracle rules + the sketch contract (schema enforced server-side, example
/// included) + the lane assignment + the computed DATA BRIEF / MEMORY sections.
pub(crate) fn build_router_prompt(lane: Lane, sample_index: usize, extra_sections: &str) -> String {
    let example = serde_json::to_string_pretty(&fixture_sketch()).unwrap_or_default();
    let relations = registered_relations().join(", ");
    format!(
        "You are a theoretical cosmologist proposing ONE falsifiable modified-cosmology theory \
         to a deterministic physics oracle (\"LLM proposes, the oracle disposes\").\n\
         \n\
         ## THE ORACLE'S RULES (violations are machine-detected kills)\n\
         1. The engine COMPUTES all physics from your bound background — declared numbers are \
         honesty attestations checked against the computed truth. Fabrication (>3× tolerance) \
         is a kill; honest-but-wrong is a demotion.\n\
         2. Only these registry relations verify: {relations}. EXACT input keys AND \
         expected-value formulas (wrong 'expected' = FailedDerivationCertificate kill):\n\
         - planck_mu0_geff: inputs=[mu0]; expected = 1.0 + mu0\n\
         - ndgp_geff_over_g: inputs=[beta]; expected = 1.0 + 1.0/(3.0*beta); beta>0 suppresses growth; beta<0 enhances\n\
         - ndgp_beta_from_omega_rc: inputs=[omega_rc, omega_m, (opt)omega_r, omega_k, w0=-1]; \
         expected = 1 + (1/sqrt(omega_rc)) * (1 + D1/3) where D1 = -0.5*(3*Om + 4*Or + 2*Ok + 3*(1+w0)*(1-Om-Or-Ok))\n\
         - coupled_de_geff_over_g: inputs=[beta]; expected = 1.0 + 2.0*beta^2\n\
         - dark_scattering_growth_drag: inputs=[a_drag, w0, omega_de0]; expected = a_drag*(1+w0)*omega_de0\n\
         - fr_alpha_m: inputs=[f_R, a_f_R_prime]; expected = a_f_R_prime/(1.0 + f_R)\n\
         - fr_largescale_geff_over_g: inputs=[regime]; expected = 4/3 if regime=1.0, 1.0 if regime=0.0\n\
         To set a background MG dial (e.g. mu0) you MUST include a \
         derived_certified parameter whose certificate input equals that background value — \
         an uncertified background modification is an automatic kill.\n\
         3. A certified modified-gravity claim is BOUND into the background the model integrates; \
         unexplained or conflicting modifications are kills.\n\
         4. Every physics claim needs ≥1 obligation; every cited evidence path needs content in \
         `evidence`.\n\
         5. Novelty credit requires a computed, honest, DISTINCT prediction — no witness = 0.\n\
         6. Background dials (w0, wa) moved off ΛCDM cost parsimony; prefer certified parameters.\n\
         7. Numbers must be JSON numbers, never strings. Omitted optional fields carry sentinels: \"\" / 0.0 \
         / [].\n\
         \n\
         ## MECHANISM LANE — sample {sample_index}: {lane_name}\n\
         {lane_brief}\n\
         \n\
         ## OUTPUT CONTRACT\n\
         Return EXACTLY one JSON object matching the ProposalSketch schema (it is machine-\
         validated server-side; no prose, no fences). Example of a VALID sketch:\n\
         ```json\n{example}\n```\n\
         {extra_sections}",
        relations = relations,
        sample_index = sample_index,
        lane_name = lane.name(),
        lane_brief = lane.brief(),
        example = example,
        extra_sections = extra_sections,
    )
}

// ---------------------------------------------------------------------------
// The canonical fixture sketch (round-trip-locked against fixture_proposal()).
// ---------------------------------------------------------------------------

/// A hand-written sketch expressing the same physics as `fixture_proposal()` — nDGP with a
/// certified Ω_rc→β chain and an engine-computed witness.
pub(crate) fn fixture_sketch() -> ProposalSketch {
    let fixture = super::proposer::fixture_proposal();
    sketch_from_doc(&fixture)
}

/// Project a ProposalDoc back into sketch form (used for the fixture + future re-clothe ops).
pub(crate) fn sketch_from_doc(doc: &ProposalDoc) -> ProposalSketch {
    let t = &doc.theory;
    let base = Theory::baseline_lcdm();
    let base_symbols: Vec<&str> = base.parameters.iter().map(|p| p.symbol.as_str()).collect();
    let parameters = t
        .parameters
        .iter()
        .filter(|p| !base_symbols.contains(&p.symbol.as_str()))
        .map(|p| {
            let (provenance_kind, mechanism, relation, inputs, expected, tolerance) =
                match &p.provenance {
                    Provenance::Fundamental => (
                        "fundamental".to_string(),
                        String::new(),
                        String::new(),
                        vec![],
                        0.0,
                        0.0,
                    ),
                    Provenance::Free => {
                        // Free params are vetoed anyway; round-trip them as textual-derived.
                        (
                            "derived_textual".to_string(),
                            String::new(),
                            String::new(),
                            vec![],
                            0.0,
                            0.0,
                        )
                    }
                    Provenance::Derived {
                        mechanism,
                        certificate,
                    } => match certificate {
                        Some(c) => (
                            "derived_certified".to_string(),
                            mechanism.clone(),
                            c.relation.clone(),
                            c.inputs
                                .iter()
                                .map(|(name, value)| SketchInput {
                                    name: name.clone(),
                                    value: *value,
                                })
                                .collect(),
                            c.expected,
                            c.tolerance,
                        ),
                        None => (
                            "derived_textual".to_string(),
                            mechanism.clone(),
                            String::new(),
                            vec![],
                            0.0,
                            0.0,
                        ),
                    },
                };
            SketchParam {
                symbol: p.symbol.clone(),
                value: p.value,
                meaning: p.physical_meaning.clone(),
                provenance_kind,
                mechanism,
                relation,
                inputs,
                expected,
                tolerance,
            }
        })
        .collect();
    ProposalSketch {
        theory_id: t.id.clone(),
        summary: "fixture: certified nDGP crossover with an engine-computed growth witness".into(),
        mechanism_lane: "free".into(),
        parameters,
        background: SketchBackground {
            mg_family: serde_enum_string(&t.background.mg_family),
            mu0: t.background.mu0,
            fr_n: if t.background.fr_n == base.background.fr_n {
                0.0
            } else {
                t.background.fr_n
            },
            fr_log10_fr0: if t.background.fr_log10_fr0 == base.background.fr_log10_fr0 {
                0.0
            } else {
                t.background.fr_log10_fr0
            },
            ndgp_omega_rc: t.background.ndgp_omega_rc,
            w0: t.background.w0,
            wa: t.background.wa,
        },
        claims: doc
            .claims
            .iter()
            .map(|c| SketchClaim {
                id: c.id.clone(),
                sector: serde_enum_string(&c.sector),
                kind: serde_enum_string(&c.kind),
                statement: c.statement.clone(),
                evidence_paths: c.evidence.iter().map(|e| e.path.clone()).collect(),
                obligations: c.obligations.clone(),
                depends_on: c.depends_on.clone(),
            })
            .collect(),
        obligations: doc
            .obligations
            .iter()
            .map(|o| SketchObligation {
                claim_id: o.claim_id.clone(),
                kind: serde_enum_string(&o.kind),
                detail: o.detail.clone(),
                relation: o
                    .certificate
                    .as_ref()
                    .map(|c| c.relation.clone())
                    .unwrap_or_default(),
                inputs: o
                    .certificate
                    .as_ref()
                    .map(|c| {
                        c.inputs
                            .iter()
                            .map(|(name, value)| SketchInput {
                                name: name.clone(),
                                value: *value,
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
                expected: o.certificate.as_ref().map(|c| c.expected).unwrap_or(0.0),
                tolerance: o.certificate.as_ref().map(|c| c.tolerance).unwrap_or(0.0),
                limit_name: o.limit.as_ref().map(|l| l.name.clone()).unwrap_or_default(),
                residual: o.limit.as_ref().map(|l| l.residual).unwrap_or(0.0),
                bound: o.limit.as_ref().map(|l| l.bound).unwrap_or(0.0),
                citation: o.citation.clone().unwrap_or_default(),
                observable: o
                    .novel
                    .as_ref()
                    .map(|w| w.observable.clone())
                    .unwrap_or_default(),
                predicted: o.novel.as_ref().map(|w| w.predicted).unwrap_or(0.0),
                baseline: o.novel.as_ref().map(|w| w.baseline).unwrap_or(0.0),
                min_detectable: o.novel.as_ref().map(|w| w.min_detectable).unwrap_or(0.0),
                falsifier: o
                    .novel
                    .as_ref()
                    .map(|w| w.falsifier.clone())
                    .unwrap_or_default(),
            })
            .collect(),
        shared_params: doc
            .unification
            .shared
            .iter()
            .map(|sp| SketchShared {
                symbol: sp.symbol.clone(),
                value: sp.value,
                sectors: sp.sectors.iter().map(serde_enum_string).collect(),
            })
            .collect(),
        evidence: doc
            .evidence
            .iter()
            .map(|(path, content)| SketchEvidence {
                path: path.clone(),
                content: content.clone(),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zyal_genome::physics_score::baseline_log_likelihood;
    use crate::zyal_genome::proposer::{fixture_proposal, score_proposal};
    use openqg_core::ObservableRecord;

    fn obs() -> Vec<ObservableRecord> {
        ["bao_dv_z038", "bao_dv_z051", "fsigma8_z038", "fsigma8_z051"]
            .iter()
            .enumerate()
            .map(|(i, id)| ObservableRecord {
                observable_id: id.to_string(),
                kind: "bao".into(),
                value: 10.0 + i as f64,
                uncertainty: 0.5,
                unit: "dimensionless".into(),
                source: None,
            })
            .collect()
    }

    /// THE round-trip contract: the fixture sketch expands to a doc that scores EXACTLY like
    /// fixture_proposal() — total, DQ, and distinctness.
    #[test]
    fn fixture_sketch_expands_to_the_fixture_score() {
        let direct = fixture_proposal();
        let expanded = expand_sketch(&fixture_sketch()).expect("fixture sketch expands");
        let observables = obs();
        let bll = baseline_log_likelihood(&observables);
        let a = score_proposal(&direct, &observables, &[], bll);
        let b = score_proposal(&expanded, &observables, &[], bll);
        assert_eq!(a.disqualified, b.disqualified, "{:?}", b.kill_reasons);
        assert_eq!(a.total, b.total);
        assert_eq!(a.distinct_from_baseline, b.distinct_from_baseline);
    }

    #[test]
    fn expansion_is_deterministic() {
        let a = expand_sketch(&fixture_sketch()).unwrap();
        let b = expand_sketch(&fixture_sketch()).unwrap();
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap()
        );
    }

    /// The anti-drift lock: every schema enum string IS the serde serialization / registry truth.
    #[test]
    fn schema_enums_match_the_serde_truth() {
        let schema = proposal_sketch_schema();
        let rels: Vec<String> = schema["properties"]["parameters"]["items"]["properties"]
            ["relation"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        for r in registered_relations() {
            assert!(rels.contains(&r.to_string()), "missing relation {r}");
        }
        let sectors: Vec<String> = schema["properties"]["claims"]["items"]["properties"]["sector"]
            ["enum"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        assert!(sectors.contains(&serde_enum_string(&Sector::Growth)));
        assert!(sectors.contains(&serde_enum_string(&Sector::ScreeningPpn)));
        assert_eq!(sectors.len(), 7);
    }

    /// Strict-mode discipline: every object schema is closed and fully required.
    #[test]
    fn schema_is_strict_and_closed() {
        fn walk(v: &Value) {
            if let Some(obj) = v.as_object() {
                if obj.get("type").and_then(Value::as_str) == Some("object") {
                    assert_eq!(
                        obj.get("additionalProperties"),
                        Some(&Value::Bool(false)),
                        "open object found"
                    );
                    let props: Vec<String> = obj["properties"]
                        .as_object()
                        .unwrap()
                        .keys()
                        .cloned()
                        .collect();
                    let reqd: Vec<String> = obj["required"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|x| x.as_str().unwrap().to_string())
                        .collect();
                    assert_eq!(props, reqd, "required != properties");
                }
                for val in obj.values() {
                    walk(val);
                }
            } else if let Some(items) = v.as_array() {
                for val in items {
                    walk(val);
                }
            }
        }
        walk(&proposal_sketch_schema());
    }

    #[test]
    fn out_of_enum_strings_are_hard_errors() {
        let mut sk = fixture_sketch();
        sk.claims[0].sector = "gravity".into(); // not a Sector
        assert!(expand_sketch(&sk).is_err());
        let mut sk2 = fixture_sketch();
        sk2.background.mg_family = "planck_mu0".into(); // the V5 invented-family bug
        assert!(expand_sketch(&sk2).is_err());
    }

    #[test]
    fn lane_prompts_only_name_registry_relations_and_assemble() {
        for (i, lane) in LANES.iter().enumerate() {
            let p = build_router_prompt(*lane, i, "## DATA BRIEF\n(test)");
            assert!(p.contains(lane.name()));
            assert!(p.contains("planck_mu0_geff") || !p.contains("_geff{"));
            assert!(p.contains("DATA BRIEF"));
        }
    }
}
