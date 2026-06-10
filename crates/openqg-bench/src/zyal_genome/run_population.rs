//! V4 M5c: the real-engine run path.
//!
//! Drives the proven [`evolve_population`] engine end-to-end and writes a monitorable run directory:
//! `progress-ledger.jsonl` (per-generation new/reused claim-fingerprints), `champion.json` (the best
//! non-disqualified theory with its lineage + scorecard total), `run-summary.json`, and
//! `quality-gate.json` (the V4 hard gates — most importantly `population_progress`, which a V3-style
//! gen-1-only collapse FAILS). This is a NEW, self-contained CLI path; it does not touch the legacy
//! stage loop in `run_variant.rs`.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::{json, Value};

use openqg_core::ObservableRecord;

use super::physics_score::baseline_log_likelihood;
use super::proposer::{score_proposal, ProposalDoc, Proposer};
use super::theory_population::{
    evolve_population, population_progress_ok, EvolveConfig, GenerationProgress, Individual,
};

/// Read JSONL observables (one [`ObservableRecord`] per non-empty line).
pub(crate) fn load_observables(path: &Path) -> Result<Vec<ObservableRecord>> {
    let text =
        fs::read_to_string(path).with_context(|| format!("read observables {}", path.display()))?;
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).with_context(|| format!("parse observable: {l}")))
        .collect()
}

fn progress_value(g: &GenerationProgress) -> Value {
    json!({
        "record_kind": "generation_progress",
        "generation": g.generation,
        "new_fingerprints": g.new_fingerprints,
        "reused_fingerprints": g.reused_fingerprints,
        "distinct_lineages": g.distinct_lineages,
        "promote_lineage_only": g.promote_lineage_only,
    })
}

fn champion_value(i: &Individual) -> Value {
    json!({
        "record_kind": "champion",
        "id": i.id,
        "generation": i.generation,
        "island": i.island,
        "parent_ids": i.parent_ids,
        "claim_fingerprint": i.fingerprint,
        "final_score": i.final_score,
        "disqualified": i.disqualified,
        "theory": serde_json::to_value(&i.theory).unwrap_or(Value::Null),
    })
}

/// Run the real-engine population search and write the run directory. Returns the run dir.
pub(crate) fn run_population(
    observables_path: &Path,
    output_root: &Path,
    config: EvolveConfig,
    run_id: &str,
    proposer: Option<&dyn Proposer>,
) -> Result<PathBuf> {
    let observables = load_observables(observables_path)?;
    anyhow::ensure!(
        !observables.is_empty(),
        "no observables loaded from {}",
        observables_path.display()
    );

    let run = evolve_population(&config, &observables, proposer);

    let run_dir = output_root.join("runs").join(run_id);
    fs::create_dir_all(&run_dir)
        .with_context(|| format!("create run dir {}", run_dir.display()))?;

    // Progress ledger — one line per generation.
    let mut pl = fs::File::create(run_dir.join("progress-ledger.jsonl"))?;
    for g in &run.progress {
        writeln!(pl, "{}", serde_json::to_string(&progress_value(g))?)?;
    }

    // Proposal ledger — content-pinned audit/replay trail of every proposal that entered the run.
    let mut led = fs::File::create(run_dir.join("proposal-ledger.jsonl"))?;
    for rec in &run.live_proposals {
        writeln!(led, "{}", serde_json::to_string(rec)?)?;
    }

    // Champion.
    if let Some(best) = &run.best {
        fs::write(
            run_dir.join("champion.json"),
            serde_json::to_string_pretty(&champion_value(best))?,
        )?;
    }

    // V4 hard gates.
    let progress_ok = population_progress_ok(&run.progress);
    let non_root_ok = run
        .champions
        .iter()
        .filter(|c| c.generation > 1)
        .all(|c| !c.parent_ids.is_empty());
    let has_survivor = run.best.as_ref().map(|b| !b.disqualified).unwrap_or(false);
    let gate_passed = progress_ok && non_root_ok && has_survivor;
    let quality_gate = json!({
        "record_kind": "quality_gate",
        "checks": {
            "population_progress": progress_ok,
            "non_root_lineage_after_gen1": non_root_ok,
            "has_non_disqualified_champion": has_survivor,
        },
        "passed": gate_passed,
    });
    fs::write(
        run_dir.join("quality-gate.json"),
        serde_json::to_string_pretty(&quality_gate)?,
    )?;

    // Run summary.
    let summary = json!({
        "record_kind": "run_summary",
        "run_id": run_id,
        "engine": "theory_population.v5",
        "generation_count": config.max_generations,
        "population_size": config.population_size,
        "seed": config.seed,
        "observables": observables.len(),
        "champion_count": run.champions.len(),
        "best_score": run.best.as_ref().map(|b| b.final_score),
        "best_fingerprint": run.best.as_ref().map(|b| b.fingerprint.clone()),
        "progress_ok": progress_ok,
        "quality_gate_passed": gate_passed,
    });
    fs::write(
        run_dir.join("run-summary.json"),
        serde_json::to_string_pretty(&summary)?,
    )?;

    Ok(run_dir)
}

/// Replay a proposal ledger **without the LLM**: deserialize each recorded `ProposalDoc` and re-score
/// it deterministically, confirming it reproduces the recorded total. This is what makes the live
/// run's "replayable" claim true — the proposals are pinned in the ledger, so the verdict is
/// reproducible from artifacts alone. Returns `(checked, mismatches)`.
pub(crate) fn replay_ledger(ledger_path: &Path, observables_path: &Path) -> Result<(usize, usize)> {
    let observables = load_observables(observables_path)?;
    anyhow::ensure!(!observables.is_empty(), "no observables loaded");
    let baseline_ll = baseline_log_likelihood(&observables);
    let text = fs::read_to_string(ledger_path)
        .with_context(|| format!("read ledger {}", ledger_path.display()))?;
    let mut checked = 0usize;
    let mut mismatches = 0usize;
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let rec: Value = serde_json::from_str(line).context("parse ledger line")?;
        let doc_val = rec.get("doc").cloned().unwrap_or(Value::Null);
        let doc: ProposalDoc = match serde_json::from_value(doc_val) {
            Ok(d) => d,
            Err(e) => {
                println!("  UNPARSEABLE ledger doc: {e}");
                mismatches += 1;
                continue;
            }
        };
        let sc = score_proposal(&doc, &observables, baseline_ll);
        let recorded = rec.get("total").and_then(Value::as_f64).unwrap_or(f64::NAN);
        checked += 1;
        if (sc.total - recorded).abs() > 1e-6 {
            mismatches += 1;
            println!(
                "  MISMATCH gen {}: recorded {recorded} vs replay {}",
                rec.get("generation").and_then(Value::as_u64).unwrap_or(0),
                sc.total
            );
        }
    }
    Ok((checked, mismatches))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_observables(dir: &Path) -> PathBuf {
        let path = dir.join("obs.jsonl");
        let lines = [
            r#"{"observable_id":"bao_dv_z038","kind":"cosmology","value":1.0,"uncertainty":0.05,"unit":"x"}"#,
            r#"{"observable_id":"bao_dv_z051","kind":"cosmology","value":1.1,"uncertainty":0.05,"unit":"x"}"#,
            r#"{"observable_id":"fsigma8_z038","kind":"cosmology","value":0.45,"uncertainty":0.03,"unit":"x"}"#,
            r#"{"observable_id":"fsigma8_z051","kind":"cosmology","value":0.46,"uncertainty":0.03,"unit":"x"}"#,
        ];
        fs::write(&path, lines.join("\n")).unwrap();
        path
    }

    #[test]
    fn run_population_writes_artifacts_and_passes_progress_gate() {
        let tmp = std::env::temp_dir().join(format!("openqg-v4-runpop-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let obs = write_observables(&tmp);

        let cfg = EvolveConfig {
            population_size: 9,
            max_generations: 5,
            seed: 123,
        };
        let run_dir = run_population(&obs, &tmp, cfg, "test-pop", None).expect("run");

        // Artifacts exist.
        for f in [
            "progress-ledger.jsonl",
            "champion.json",
            "run-summary.json",
            "quality-gate.json",
        ] {
            assert!(run_dir.join(f).exists(), "missing artifact {f}");
        }

        // The quality gate passed (real per-generation progress).
        let qg: Value =
            serde_json::from_str(&fs::read_to_string(run_dir.join("quality-gate.json")).unwrap())
                .unwrap();
        assert_eq!(
            qg["passed"],
            Value::Bool(true),
            "quality gate must pass: {qg}"
        );

        // The progress ledger has one line per generation, with new fingerprints after gen 1.
        let ledger = fs::read_to_string(run_dir.join("progress-ledger.jsonl")).unwrap();
        let lines: Vec<&str> = ledger.lines().filter(|l| !l.trim().is_empty()).collect();
        assert_eq!(lines.len(), 5);
        for line in &lines[1..] {
            let g: Value = serde_json::from_str(line).unwrap();
            assert!(g["new_fingerprints"].as_u64().unwrap() >= 1);
        }

        let _ = fs::remove_dir_all(&tmp);
    }
}
