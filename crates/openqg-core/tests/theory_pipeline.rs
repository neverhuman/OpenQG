//! End-to-end candidate-evaluation pipeline: the deterministic veto cascade GATES the forward
//! model, so a structurally-broken theory is killed for free and never reaches (expensive)
//! scoring, while a clean theory flows to genuine data-driven scoring. This is the shape the
//! rebuilt evolution engine evaluates every candidate through (veto → forward → score).

use openqg_core::cosmology::{BackgroundForwardModel, ForwardModel};
use openqg_core::theory::{run_veto_cascade, Parameter, Provenance, Theory};
use openqg_core::{score_metrics, ObservableRecord};
use std::path::PathBuf;

fn load_desi() -> Vec<ObservableRecord> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/fixtures/cosmology/bao-desi-dr1.jsonl");
    std::fs::read_to_string(&path)
        .expect("read DESI fixture")
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str::<ObservableRecord>(l).expect("parse observable"))
        .collect()
}

/// The candidate-evaluation pipeline: returns `None` if the theory is vetoed (never scored),
/// otherwise the data log-likelihood from the real forward model.
fn evaluate(theory: &Theory, obs: &[ObservableRecord]) -> Option<f64> {
    if !run_veto_cascade(theory).is_empty() {
        return None; // hard kill — no expensive scoring spent on a broken theory
    }
    let model = BackgroundForwardModel;
    let ids: Vec<String> = obs.iter().map(|o| o.observable_id.clone()).collect();
    let preds = model.predict(&theory.background, &ids).expect("predict");
    let (metrics, _) = score_metrics(obs, &preds, theory.parameters.len().max(1), 0.0);
    Some(metrics.log_likelihood)
}

#[test]
fn a_free_parameter_theory_is_vetoed_and_never_scored() {
    let obs = load_desi();
    let mut gray_box = Theory::baseline_lcdm();
    gray_box.parameters.push(Parameter {
        symbol: "f_ede".into(),
        value: 0.07,
        physical_meaning: "early dark energy fraction tuned to raise H0".into(),
        provenance: Provenance::Free,
    });
    assert!(
        evaluate(&gray_box, &obs).is_none(),
        "a gray-box free-parameter theory must be killed before scoring"
    );
}

#[test]
fn a_clean_theory_flows_through_to_real_scoring() {
    let obs = load_desi();
    let baseline = Theory::baseline_lcdm();
    let ll = evaluate(&baseline, &obs).expect("clean theory should be scored");
    // It produced a real, finite likelihood against external DESI data.
    assert!(ll.is_finite() && ll < 0.0, "log-likelihood = {ll}");
    assert!(-2.0 * ll < 45.0, "baseline chi^2 = {}", -2.0 * ll);
}
