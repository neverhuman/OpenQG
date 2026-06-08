//! CLI handler: run the `openqg-core` symbolic-theory evolution engine against real data and
//! emit a champion report. This is the additive first integration of the rebuilt engine into the
//! binary — it drives `Theory -> veto -> real forward model -> epsilon/beta -> unification ->
//! evolved champion` end-to-end, without touching the legacy `zyal_*` paths.

use anyhow::{Context, Result};
use openqg_core::cosmology::{BackgroundForwardModel, CosmologyParams, ForwardModel};
use openqg_core::theory::{
    evolve_run, miscalibrated, perturbation_robustness, proposal_to_theory, Champion,
    GenerationReport, Provenance, Theory,
};
use openqg_core::{score_metrics, ObservableRecord};
use serde_json::{json, Value};
use std::fs;
use std::io::Write;
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

/// Parse JSONL proposal lines, each through the derivation checker into `(Theory, demoted)`.
fn proposals_from_lines<'a>(
    lines: impl Iterator<Item = &'a str>,
) -> Result<Vec<(Theory, Vec<String>)>> {
    lines
        .filter(|l| !l.trim().is_empty())
        .map(|l| proposal_to_theory(l).with_context(|| format!("parse proposal: {l}")))
        .collect()
}

/// Load theory proposals from a JSONL file.
fn load_proposals(path: &Path) -> Result<Vec<(Theory, Vec<String>)>> {
    let text =
        fs::read_to_string(path).with_context(|| format!("read proposals {}", path.display()))?;
    proposals_from_lines(text.lines())
}

/// Run an LLM-proposer command and parse its stdout as JSONL proposals. A non-zero exit or a
/// command that cannot run is an error (a failed proposer is not silently ignored).
fn run_proposer(cmd: &str) -> Result<Vec<(Theory, Vec<String>)>> {
    let out = std::process::Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output()
        .with_context(|| format!("run proposer command: {cmd}"))?;
    if !out.status.success() {
        anyhow::bail!(
            "proposer command failed ({}): {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    proposals_from_lines(stdout.lines())
}

/// One per-generation telemetry line (legacy-monitor-compatible metrics_point shape).
fn metrics_point(gr: &GenerationReport, run_id: &str) -> Value {
    json!({
        "schema_version": "theory-evolve.v1",
        "record_kind": "metrics_point",
        "series": "diversity",
        "run_id": run_id,
        "generation_id": format!("g{:04}", gr.generation),
        "generation": gr.generation,
        "qd_score": gr.qd_score,
        "archive_cells": gr.archive_cells,
        "frontier_margin": gr.frontier_margin,
        "anchor_health": gr.anchor_health,
        "calibration_honest": gr.calibration_honest,
        "champion_id": gr.champion_id,
        "champion_fitness": gr.champion_fitness,
        "champion_pressured_fitness": gr.champion_pressured_fitness,
        "champion_credible": gr.champion_credible,
    })
}

fn write_json(path: &Path, value: &Value) -> Result<()> {
    fs::write(path, serde_json::to_string_pretty(value)?)
        .with_context(|| format!("write {}", path.display()))
}

/// The GR/ΛCDM baseline's log-likelihood on the observables. Passed as the run's baseline so that
/// ε (`delta_log_likelihood`) measures *improvement over the GR baseline* — centering the fitness
/// near 0.5 for the baseline rather than collapsing all candidates near 0 (which happens if ε is an
/// absolute multi-observable log-likelihood). This also calibrates the adversary's escalation.
fn baseline_log_likelihood(observables: &[ObservableRecord]) -> f64 {
    let model = BackgroundForwardModel;
    let ids: Vec<String> = observables
        .iter()
        .map(|o| o.observable_id.clone())
        .collect();
    match model.predict(&CosmologyParams::planck_lcdm(), &ids) {
        Ok(preds) => score_metrics(observables, &preds, 3, 0.0).0.log_likelihood,
        Err(_) => 0.0,
    }
}

/// Run the adversarial, observed evolution loop from the GR/ΛCDM baseline (plus any proposals) and
/// write a fully-logged, monitorable run directory under `<output_root>/runs/<run_id>/`.
pub fn run_evolve(
    observables_path: &Path,
    proposals_path: Option<&Path>,
    proposer_cmd: Option<&str>,
    output_root: &Path,
    run_id: &str,
    checkpoint_every: usize,
    generations: usize,
    population: usize,
    seed: u64,
) -> Result<()> {
    let observables = load_observables(observables_path)?;
    let mut seeds = vec![Theory::baseline_lcdm()];
    let mut demotions: Vec<Value> = Vec::new();
    let mut proposed = Vec::new();
    if let Some(pp) = proposals_path {
        proposed.extend(load_proposals(pp)?);
    }
    if let Some(cmd) = proposer_cmd {
        proposed.extend(run_proposer(cmd)?);
    }
    for (theory, demoted) in proposed {
        if !demoted.is_empty() {
            demotions.push(json!({"proposal": theory.id, "demoted_parameters": demoted}));
        }
        seeds.push(theory);
    }

    let run_dir = output_root.join("runs").join(run_id);
    fs::create_dir_all(&run_dir).with_context(|| format!("create {}", run_dir.display()))?;
    let model = BackgroundForwardModel;
    // ε is measured as improvement over the GR baseline (not absolute log-likelihood).
    let baseline_ll = baseline_log_likelihood(&observables);

    write_json(
        &run_dir.join("run-config.json"),
        &json!({
            "engine": "openqg-core/theory-evolve",
            "run_id": run_id,
            "observables": observables_path.display().to_string(),
            "observable_count": observables.len(),
            "generations": generations,
            "population": population,
            "seed": seed,
            "seeds": seeds.len(),
            "proposal_demotions": demotions.clone(),
        }),
    )?;

    // Per-generation telemetry, appended live so the run is monitorable via `tail -f`.
    let ts_path = run_dir.join("metrics-timeseries.jsonl");
    let mut ts =
        fs::File::create(&ts_path).with_context(|| format!("create {}", ts_path.display()))?;
    let flush_every = checkpoint_every.max(1);
    let mut last_report: Option<GenerationReport> = None;

    let result = evolve_run(
        &seeds,
        &observables,
        &model,
        baseline_ll,
        generations,
        population,
        seed,
        |gr| {
            let line = serde_json::to_string(&metrics_point(gr, run_id)).unwrap_or_default();
            let _ = writeln!(ts, "{line}");
            if gr.generation % flush_every == 0 {
                let _ = ts.flush();
            }
            last_report = Some(gr.clone());
        },
    );
    ts.flush().ok();

    let calibration = miscalibrated(&observables, &model, baseline_ll);
    let final_frontier = last_report
        .as_ref()
        .map(|g| g.frontier_margin)
        .unwrap_or(0.0);
    let final_anchor_health = last_report.as_ref().map(|g| g.anchor_health).unwrap_or(1.0);
    let champion_credible = result
        .champion
        .as_ref()
        .map(|c| c.assessment.is_credible())
        .unwrap_or(false);
    let champion_unified = result
        .champion
        .as_ref()
        .map(|c| c.assessment.unification.is_unified())
        .unwrap_or(false);

    // MAP-Elites archive (cells → serialized theory + assessment summary).
    let cells: Vec<Value> = result
        .archive
        .iter()
        .map(|(cell, champ)| {
            json!({
                "cell": [cell.0, cell.1, cell.2],
                "final_fitness": champ.assessment.final_fitness,
                "credible": champ.assessment.is_credible(),
                "theory": serde_json::to_value(&champ.theory).unwrap_or(Value::Null),
            })
        })
        .collect();
    write_json(
        &run_dir.join("map-elites-archive.json"),
        &json!({ "qd_score": result.qd_score, "cells": cells }),
    )?;

    // Champion report.
    let champion_value = result.champion.as_ref().map(|c| {
        let robustness =
            perturbation_robustness(&c.theory, &observables, &model, baseline_ll, seed, 48);
        champion_json(c, robustness)
    });
    write_json(
        &run_dir.join("champion.json"),
        &json!({
            "engine": "openqg-core/theory-evolve",
            "run_id": run_id,
            "generations": generations,
            "population": population,
            "seed": seed,
            "qd_score": result.qd_score,
            "archive_cells": result.archive.len(),
            "seeds": seeds.len(),
            "proposal_demotions": demotions.clone(),
            "final_frontier_margin": final_frontier,
            "final_anchor_health": final_anchor_health,
            "calibration": {
                "honest": calibration.is_empty(),
                "miscalibrated_anchors": calibration.clone(),
            },
            "champion": champion_value,
        }),
    )?;

    // Quality gate (final honesty + champion verdict).
    let distinct_fitness_ratio = {
        use std::collections::BTreeSet;
        let s: BTreeSet<u64> = result
            .archive
            .values()
            .map(|c| (c.assessment.final_fitness * 1e6) as u64)
            .collect();
        if result.archive.is_empty() {
            0.0
        } else {
            s.len() as f64 / result.archive.len() as f64
        }
    };
    let passed = calibration.is_empty() && champion_credible && champion_unified;
    write_json(
        &run_dir.join("quality-gate.json"),
        &json!({
            "passed": passed,
            "calibration_honest": calibration.is_empty(),
            "miscalibrated_anchors": calibration,
            "champion_credible": champion_credible,
            "champion_unified": champion_unified,
            "qd_score": result.qd_score,
            "archive_cells": result.archive.len(),
            "distinct_fitness_ratio": distinct_fitness_ratio,
            "final_frontier_margin": final_frontier,
        }),
    )?;

    // Run summary.
    write_json(
        &run_dir.join("run-summary.json"),
        &json!({
            "run_id": run_id,
            "generations": generations,
            "population": population,
            "seed": seed,
            "observable_count": observables.len(),
            "qd_score": result.qd_score,
            "archive_cells": result.archive.len(),
            "proposal_demotions": demotions,
            "final_frontier_margin": final_frontier,
            "final_anchor_health": final_anchor_health,
            "calibration_honest": passed,
            "champion": result.champion.as_ref().map(|c| json!({
                "id": c.theory.id,
                "final_fitness": c.assessment.final_fitness,
                "credible": c.assessment.is_credible(),
                "unification": c.assessment.unification.score,
            })),
        }),
    )?;

    match &result.champion {
        Some(c) => println!(
            "[{run_id}] champion {} fitness={:.4} credible={} unified={} | qd={:.3} cells={} frontier={:.3} honest={} -> {}",
            c.theory.id,
            c.assessment.final_fitness,
            c.assessment.is_credible(),
            champion_unified,
            result.qd_score,
            result.archive.len(),
            final_frontier,
            calibration.is_empty(),
            run_dir.display(),
        ),
        None => println!(
            "[{run_id}] no credible champion | qd={:.3} cells={} -> {}",
            result.qd_score,
            result.archive.len(),
            run_dir.display(),
        ),
    }
    Ok(())
}
