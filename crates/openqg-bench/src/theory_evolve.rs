//! CLI handler: run the `openqg-core` symbolic-theory evolution engine against real data and
//! emit a champion report. This is the additive first integration of the rebuilt engine into the
//! binary — it drives `Theory -> veto -> real forward model -> epsilon/beta -> unification ->
//! evolved champion` end-to-end, without touching the legacy `zyal_*` paths.

use anyhow::{Context, Result};
use openqg_core::cosmology::BackgroundForwardModel;
use openqg_core::theory::{evolve, perturbation_robustness, Champion, Provenance, Theory};
use openqg_core::ObservableRecord;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

fn provenance_label(p: &Provenance) -> String {
    match p {
        Provenance::Fundamental => "fundamental".into(),
        Provenance::Derived { mechanism } => format!("derived: {mechanism}"),
        Provenance::Free => "free".into(),
    }
}

fn champion_json(c: &Champion, robustness: f64) -> Value {
    let a = &c.assessment;
    json!({
        "id": c.theory.id,
        "final_fitness": a.final_fitness,
        "credible": a.is_credible(),
        "perturbation_robustness": robustness,
        "fit": {
            "vetoed": a.evaluation.vetoed,
            "epsilon_delta_log_likelihood": a.evaluation.epsilon_delta_log_likelihood,
            "log_likelihood": a.evaluation.log_likelihood,
            "beta": a.evaluation.beta,
            "coverage": a.evaluation.coverage,
        },
        "unification": {
            "score": a.unification.score,
            "is_unified": a.unification.is_unified(),
            "domains": a.unification.checks.iter().map(|d| json!({
                "domain": d.domain, "score": d.score, "note": d.note,
            })).collect::<Vec<_>>(),
        },
        "theory": {
            "alpha": {
                "alpha_m": c.theory.alpha.alpha_m,
                "alpha_b": c.theory.alpha.alpha_b,
                "alpha_k": c.theory.alpha.alpha_k,
                "alpha_t": c.theory.alpha.alpha_t,
            },
            "background": {
                "h": c.theory.background.h,
                "omega_m": c.theory.background.omega_m,
                "omega_b_h2": c.theory.background.omega_b_h2,
                "n_eff": c.theory.background.n_eff,
                "sum_mnu": c.theory.background.sum_mnu,
                "w0": c.theory.background.w0,
                "wa": c.theory.background.wa,
            },
            "screening": c.theory.screening,
            "parameters": c.theory.parameters.iter().map(|p| json!({
                "symbol": p.symbol,
                "value": p.value,
                "physical_meaning": p.physical_meaning,
                "provenance": provenance_label(&p.provenance),
            })).collect::<Vec<_>>(),
        },
    })
}

fn load_observables(path: &Path) -> Result<Vec<ObservableRecord>> {
    let text =
        fs::read_to_string(path).with_context(|| format!("read observables {}", path.display()))?;
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).with_context(|| format!("parse observable: {l}")))
        .collect()
}

/// Run the evolution loop from the GR/ΛCDM baseline seed and write the champion report.
pub fn run_evolve(
    observables_path: &Path,
    output: &Path,
    generations: usize,
    population: usize,
    seed: u64,
) -> Result<()> {
    let observables = load_observables(observables_path)?;
    let seeds = vec![Theory::baseline_lcdm()];
    let model = BackgroundForwardModel;
    let result = evolve(
        &seeds,
        &observables,
        &model,
        0.0,
        generations,
        population,
        seed,
    );

    let report = json!({
        "engine": "openqg-core/theory-evolve",
        "observables": observables_path.display().to_string(),
        "observable_count": observables.len(),
        "generations": generations,
        "population": population,
        "seed": seed,
        "qd_score": result.qd_score,
        "archive_cells": result.archive.len(),
        "champion": result.champion.as_ref().map(|c| {
            // Robustness-under-perturbation of the champion (structural stability).
            let robustness =
                perturbation_robustness(&c.theory, &observables, &model, 0.0, seed, 48);
            champion_json(c, robustness)
        }),
    });

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    fs::write(output, serde_json::to_string_pretty(&report)?)
        .with_context(|| format!("write {}", output.display()))?;

    match &result.champion {
        Some(c) => println!(
            "champion {} final_fitness={:.4} credible={} unification={:.3} | qd={:.3} cells={} -> {}",
            c.theory.id,
            c.assessment.final_fitness,
            c.assessment.is_credible(),
            c.assessment.unification.score,
            result.qd_score,
            result.archive.len(),
            output.display(),
        ),
        None => println!(
            "no credible champion | qd={:.3} cells={} -> {}",
            result.qd_score,
            result.archive.len(),
            output.display(),
        ),
    }
    Ok(())
}
