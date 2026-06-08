//! End-to-end proof that the background forward model produces *real, discriminating* scores
//! against external data: load the DESI DR1 BAO fixture, derive predictions from physical
//! parameters, and score them with the same `score_metrics` the evolution engine uses. A
//! wrong cosmology must fit measurably worse — something an identity `forward_map` can never do.

use openqg_core::cosmology::{BackgroundForwardModel, CosmologyParams, ForwardModel};
use openqg_core::{score_metrics, ObservableRecord};
use std::path::PathBuf;

fn load_desi() -> Vec<ObservableRecord> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/fixtures/cosmology/bao-desi-dr1.jsonl");
    let text = std::fs::read_to_string(&path).expect("read DESI BAO fixture");
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str::<ObservableRecord>(l).expect("parse observable"))
        .collect()
}

fn log_likelihood(params: &CosmologyParams, obs: &[ObservableRecord]) -> f64 {
    let model = BackgroundForwardModel;
    let ids: Vec<String> = obs.iter().map(|o| o.observable_id.clone()).collect();
    let preds = model.predict(params, &ids).expect("predict");
    let (metrics, findings) = score_metrics(obs, &preds, 6, 0.0);
    // The background model derives every Tier-0 observable in the fixture.
    assert!(
        (metrics.coverage - 1.0).abs() < 1e-9,
        "coverage {} (findings: {:?})",
        metrics.coverage,
        findings
    );
    metrics.log_likelihood
}

#[test]
fn lcdm_fits_desi_bao_and_beats_a_wrong_cosmology() {
    let obs = load_desi();
    assert!(obs.len() >= 13, "fixture loaded {} observables", obs.len());

    // Planck ΛCDM is broadly consistent with DESI DR1 BAO + the BBN helium anchor.
    let lcdm = CosmologyParams::planck_lcdm();
    let ll_lcdm = log_likelihood(&lcdm, &obs);
    let chi2 = -2.0 * ll_lcdm;
    assert!(chi2 < 45.0, "ΛCDM chi^2 = {chi2} unexpectedly high");

    // A clearly-wrong matter density shifts both the distances and the sound horizon, so it
    // must fit substantially worse — the forward model genuinely discriminates physics.
    let mut wrong = lcdm.clone();
    wrong.omega_m = 0.45;
    let ll_wrong = log_likelihood(&wrong, &obs);
    assert!(
        ll_wrong < ll_lcdm - 10.0,
        "wrong Omega_m=0.45 should fit much worse: ll_wrong={ll_wrong} vs ll_lcdm={ll_lcdm}"
    );

    // And a wrong Hubble rate likewise degrades the fit.
    let mut wrong_h = lcdm.clone();
    wrong_h.h = 0.60;
    let ll_wrong_h = log_likelihood(&wrong_h, &obs);
    assert!(
        ll_wrong_h < ll_lcdm,
        "wrong h=0.60 should fit worse: {ll_wrong_h} vs {ll_lcdm}"
    );
}
