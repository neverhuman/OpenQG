//! M4 core: construct a symbolic [`Theory`] from an external (LLM-proposed) JSON proposal, with a
//! deterministic **derivation-checker** that demotes any parameter whose claimed derivation cannot
//! be verified to [`Provenance::Free`] — which the whitebox veto then kills. This is the structural
//! enforcement that keeps an LLM proposer honest: *nothing it asserts is trusted until it clears the
//! deterministic oracle* (see `docs/m4-llm-proposer-design.md` and
//! `docs/research/automated-theory-discovery.md` §5: "never let an LLM be the final judge").
//!
//! This module is pure and testable with a mocked JSON string — the live jnoccio subprocess that
//! produces the JSON is wired separately (it reuses the existing genome live-call machinery).

use super::{AlphaBasis, Parameter, Provenance, Stability, Theory};
use crate::cosmology::CosmologyParams;
use anyhow::{Context, Result};
use serde::Deserialize;

/// A proposed parameter. `provenance` is `"fundamental" | "derived" | "free"`; a `derived`
/// parameter must carry a non-empty `mechanism` and may only depend (via `derived_from`) on
/// symbols that actually exist in the proposal or the background.
#[derive(Debug, Clone, Deserialize)]
pub struct ParameterProposal {
    pub symbol: String,
    pub value: f64,
    #[serde(default)]
    pub physical_meaning: String,
    pub provenance: String,
    #[serde(default)]
    pub mechanism: String,
    #[serde(default)]
    pub derived_from: Vec<String>,
}

/// Proposed α-basis deviations (omitted ⇒ GR / 0).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AlphaProposal {
    #[serde(default)]
    pub alpha_m: f64,
    #[serde(default)]
    pub alpha_b: f64,
    #[serde(default)]
    pub alpha_k: f64,
    #[serde(default)]
    pub alpha_t: f64,
}

/// Proposed scalar-sector stability coefficients (omitted ⇒ healthy GR-like sector).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct StabilityProposal {
    pub kinetic_coefficient: Option<f64>,
    pub q_s: Option<f64>,
    pub sound_speed_sq: Option<f64>,
    #[serde(default)]
    pub has_nondegenerate_higher_derivatives: bool,
}

/// Proposed background cosmology overrides (omitted fields ⇒ Planck-ΛCDM baseline value).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct BackgroundProposal {
    pub h: Option<f64>,
    pub omega_m: Option<f64>,
    pub omega_b_h2: Option<f64>,
    pub n_eff: Option<f64>,
    pub sum_mnu: Option<f64>,
    pub w0: Option<f64>,
    pub wa: Option<f64>,
}

/// A full theory proposal as emitted by the LLM proposer.
#[derive(Debug, Clone, Deserialize)]
pub struct TheoryProposal {
    pub id: String,
    #[serde(default)]
    pub parameters: Vec<ParameterProposal>,
    #[serde(default)]
    pub alpha: AlphaProposal,
    #[serde(default)]
    pub screening: Option<String>,
    #[serde(default)]
    pub stability: StabilityProposal,
    #[serde(default)]
    pub background: BackgroundProposal,
}

/// Parse a strict-JSON proposal. A parse failure is a lethal proposal (return Err).
pub fn parse_proposal(json: &str) -> Result<TheoryProposal> {
    serde_json::from_str(json).context("parse theory proposal JSON")
}

/// Symbols the background always provides, available as derivation dependencies.
const BACKGROUND_SYMBOLS: &[&str] = &[
    "H0",
    "Omega_m",
    "omega_b_h2",
    "n_eff",
    "sum_mnu",
    "w0",
    "wa",
];

/// Convert a proposal into a [`Theory`], running the deterministic derivation-checker. A parameter
/// claiming `derived` is **demoted to `Free`** (which the whitebox veto then kills) if its mechanism
/// is empty or any `derived_from` symbol is unknown. Returns the theory and the list of demoted
/// symbols (diagnostics for the critic/ledger).
pub fn proposal_into_theory(p: &TheoryProposal) -> (Theory, Vec<String>) {
    let known: Vec<String> = BACKGROUND_SYMBOLS
        .iter()
        .map(|s| s.to_string())
        .chain(p.parameters.iter().map(|q| q.symbol.clone()))
        .collect();

    let mut demoted = Vec::new();
    let parameters = p
        .parameters
        .iter()
        .map(|q| {
            let provenance = match q.provenance.trim().to_ascii_lowercase().as_str() {
                "fundamental" => Provenance::Fundamental,
                "derived" => {
                    let mechanism_ok = !q.mechanism.trim().is_empty();
                    let deps_ok = q.derived_from.iter().all(|d| known.iter().any(|k| k == d));
                    if mechanism_ok && deps_ok {
                        // Text-level provenance only; a value-level certificate is attached by
                        // model-family code in a later milestone (M1 certificate.rs / vetoes.rs).
                        Provenance::derived(q.mechanism.clone())
                    } else {
                        // Unverifiable derivation ⇒ demote to a free parameter (veto will kill it).
                        demoted.push(q.symbol.clone());
                        Provenance::Free
                    }
                }
                // Anything else (including an explicit "free") is a free parameter.
                _ => Provenance::Free,
            };
            Parameter {
                symbol: q.symbol.clone(),
                value: q.value,
                physical_meaning: q.physical_meaning.clone(),
                provenance,
            }
        })
        .collect();

    let base = Theory::baseline_lcdm();
    let mut background = CosmologyParams::planck_lcdm();
    let b = &p.background;
    if let Some(v) = b.h {
        background.h = v;
    }
    if let Some(v) = b.omega_m {
        background.omega_m = v;
    }
    if let Some(v) = b.omega_b_h2 {
        background.omega_b_h2 = v;
    }
    if let Some(v) = b.n_eff {
        background.n_eff = v;
    }
    if let Some(v) = b.sum_mnu {
        background.sum_mnu = v;
    }
    if let Some(v) = b.w0 {
        background.w0 = v;
    }
    if let Some(v) = b.wa {
        background.wa = v;
    }

    let mut stability = Stability::healthy();
    if let Some(v) = p.stability.kinetic_coefficient {
        stability.kinetic_coefficient = v;
    }
    if let Some(v) = p.stability.q_s {
        stability.q_s = v;
    }
    if let Some(v) = p.stability.sound_speed_sq {
        stability.sound_speed_sq = v;
    }
    stability.has_nondegenerate_higher_derivatives =
        p.stability.has_nondegenerate_higher_derivatives;

    let theory = Theory {
        id: p.id.clone(),
        parameters,
        // Keep the GR action terms (dimensional/Lorentz structure is correct by construction;
        // a richer symbolic term grammar is future work).
        terms: base.terms.clone(),
        alpha: AlphaBasis {
            alpha_m: p.alpha.alpha_m,
            alpha_b: p.alpha.alpha_b,
            alpha_k: p.alpha.alpha_k,
            alpha_t: p.alpha.alpha_t,
        },
        stability,
        screening: p.screening.clone(),
        background,
    };
    (theory, demoted)
}

/// Parse + convert a proposal in one step. Errors on malformed JSON.
pub fn proposal_to_theory(json: &str) -> Result<(Theory, Vec<String>)> {
    let proposal = parse_proposal(json)?;
    Ok(proposal_into_theory(&proposal))
}

#[cfg(test)]
mod tests {
    use super::super::run_veto_cascade;
    use super::*;

    #[test]
    fn a_fundamental_proposal_builds_a_viable_theory() {
        let json = r#"{
            "id": "prop-lcdm",
            "parameters": [
                {"symbol":"H0","value":67.4,"provenance":"fundamental","physical_meaning":"expansion rate"}
            ],
            "background": {"h": 0.674, "omega_m": 0.315}
        }"#;
        let (theory, demoted) = proposal_to_theory(json).unwrap();
        assert!(demoted.is_empty());
        assert!(
            run_veto_cascade(&theory).is_empty(),
            "{:?}",
            run_veto_cascade(&theory)
        );
    }

    #[test]
    fn a_free_parameter_proposal_is_vetoed() {
        let json = r#"{
            "id": "prop-graybox",
            "parameters": [
                {"symbol":"f_ede","value":0.07,"provenance":"free","physical_meaning":"tuned"}
            ]
        }"#;
        let (theory, _) = proposal_to_theory(json).unwrap();
        assert!(!run_veto_cascade(&theory).is_empty());
    }

    #[test]
    fn a_derived_param_with_empty_mechanism_is_demoted_and_killed() {
        let json = r#"{
            "id": "prop-handwave",
            "parameters": [
                {"symbol":"xi","value":0.1,"provenance":"derived","mechanism":"  "}
            ]
        }"#;
        let (theory, demoted) = proposal_to_theory(json).unwrap();
        assert_eq!(demoted, vec!["xi".to_string()]);
        assert!(!run_veto_cascade(&theory).is_empty());
    }

    #[test]
    fn a_derived_param_referencing_an_unknown_symbol_is_demoted() {
        let json = r#"{
            "id": "prop-fakedep",
            "parameters": [
                {"symbol":"alpha_M0","value":0.05,"provenance":"derived",
                 "mechanism":"conformal coupling beta","derived_from":["nonexistent_symbol"]}
            ]
        }"#;
        let (_theory, demoted) = proposal_to_theory(json).unwrap();
        assert_eq!(demoted, vec!["alpha_M0".to_string()]);
    }

    #[test]
    fn a_well_provenanced_derived_param_survives() {
        let json = r#"{
            "id": "prop-screened-mg",
            "parameters": [
                {"symbol":"alpha_M0","value":0.05,"provenance":"derived",
                 "mechanism":"Planck-mass run from the conformal coupling",
                 "derived_from":["Omega_m"]}
            ],
            "alpha": {"alpha_m": 0.05, "alpha_b": -0.02},
            "screening": "chameleon",
            "stability": {"q_s": 0.6, "sound_speed_sq": 0.4, "kinetic_coefficient": 0.8}
        }"#;
        let (theory, demoted) = proposal_to_theory(json).unwrap();
        assert!(demoted.is_empty());
        assert!(
            run_veto_cascade(&theory).is_empty(),
            "{:?}",
            run_veto_cascade(&theory)
        );
        assert!(theory.screening.is_some());
        assert!(theory.modifies_gravity());
    }

    #[test]
    fn malformed_json_is_a_lethal_proposal() {
        assert!(proposal_to_theory("{not valid json").is_err());
    }
}
