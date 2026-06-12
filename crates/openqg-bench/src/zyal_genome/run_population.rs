//! V4 M5c: the real-engine run path.
//!
//! Drives the proven [`evolve_population`] engine end-to-end and writes a monitorable run directory:
//! `progress-ledger.jsonl` (per-generation new/reused claim-fingerprints), `champion.json` (the best
//! non-disqualified theory with its lineage + scorecard total), `run-summary.json`, and
//! `quality-gate.json` (the V4 hard gates — most importantly `population_progress`, which a V3-style
//! gen-1-only collapse FAILS). This is a NEW, self-contained CLI path; it does not touch the legacy
//! stage loop in `run_variant.rs`.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::{json, Value};

use openqg_core::ObservableRecord;

use super::proposer::{score_proposal, ProposalDoc, Proposer};
use super::theory_population::{
    evolve_population, population_progress_ok, EvolveConfig, Individual,
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

/// Run an external oracle command on the champion theory JSON (U1, Phase 34).
///
/// The champion theory is written to a temp file under `run_dir`; the oracle command is
/// invoked as `<cmd> <champion_theory_path>`. Exit 0 → passed; non-zero → rejected.
/// Returns `(passed, exit_code)`. Missing/non-executable commands are treated as failures.
fn run_oracle(cmd: &str, champion_json: &str, run_dir: &Path) -> (bool, i32) {
    let oracle_input = run_dir.join("oracle-input.json");
    if fs::write(&oracle_input, champion_json).is_err() {
        return (false, -1);
    }
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    let (binary, args) = match parts.split_first() {
        Some(split) => split,
        None => return (false, -1),
    };
    let status = std::process::Command::new(binary)
        .args(args)
        .arg(&oracle_input)
        .status();
    match status {
        Ok(s) => {
            let code = s.code().unwrap_or(-1);
            (s.success(), code)
        }
        Err(_) => (false, -1),
    }
}

/// Run the real-engine population search and write the run directory. Returns the run dir.
///
/// `oracle_command` — if Some, the champion theory is passed to this external command after
/// evolution completes. The command receives the champion JSON path as its last argument and
/// must exit 0 for the champion to be accepted. The oracle result is included in `quality-gate.json`.
pub(crate) fn run_population(
    observables_path: &Path,
    output_root: &Path,
    config: EvolveConfig,
    run_id: &str,
    covariance: &[PathBuf],
    proposer: Option<&dyn Proposer>,
    oracle_command: Option<&str>,
) -> Result<PathBuf> {
    let blocks = load_covariance_blocks(covariance)?;
    let observables = load_observables(observables_path)?;
    anyhow::ensure!(
        !observables.is_empty(),
        "no observables loaded from {}",
        observables_path.display()
    );

    let run_dir = output_root.join("runs").join(run_id);
    let mut sink = super::ledger_sink::RunDirSink::create(&run_dir, 25)?;
    let run = evolve_population(&config, &observables, &blocks, proposer, &mut sink);

    let run_dir = output_root.join("runs").join(run_id);
    fs::create_dir_all(&run_dir)
        .with_context(|| format!("create run dir {}", run_dir.display()))?;

    // (progress-ledger.jsonl and proposal-ledger.jsonl are STREAMED by the RunDirSink above.)

    // Champion.
    let champion_json = run
        .best
        .as_ref()
        .map(|best| {
            let val = champion_value(best);
            fs::write(
                run_dir.join("champion.json"),
                serde_json::to_string_pretty(&val).unwrap_or_default(),
            )
            .ok();
            serde_json::to_string_pretty(&val).unwrap_or_default()
        })
        .unwrap_or_default();

    // V4 hard gates.
    let progress_ok = population_progress_ok(&run.progress);
    let non_root_ok = run
        .champions
        .iter()
        .filter(|c| c.generation > 1)
        .all(|c| !c.parent_ids.is_empty());
    let has_survivor = run.best.as_ref().map(|b| !b.disqualified).unwrap_or(false);

    // U1: oracle gate — runs external command on champion theory; exit 0 = accepted.
    let oracle_result = oracle_command.map(|cmd| {
        let (passed, code) = run_oracle(cmd, &champion_json, &run_dir);
        json!({ "command": cmd, "passed": passed, "exit_code": code })
    });
    let oracle_passed = oracle_result
        .as_ref()
        .map(|r| r["passed"].as_bool().unwrap_or(false))
        .unwrap_or(true); // no oracle ⇒ trivially passed

    let gate_passed = progress_ok && non_root_ok && has_survivor && oracle_passed;
    let mut checks = json!({
        "population_progress": progress_ok,
        "non_root_lineage_after_gen1": non_root_ok,
        "has_non_disqualified_champion": has_survivor,
    });
    if let Some(ref or_) = oracle_result {
        checks["oracle"] = or_.clone();
    }
    let quality_gate = json!({
        "record_kind": "quality_gate",
        "checks": checks,
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
        "oracle_passed": oracle_passed,
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
/// Load covariance fixture files (single- or multi-block JSON) into likelihood blocks.
/// Content is hash-checked by `RegisteredCovariance`; a malformed file is a hard error, never a
/// silent diagonal fallback.
pub(crate) fn load_covariance_blocks(
    paths: &[PathBuf],
) -> Result<Vec<openqg_core::scoring::CovarianceBlock>> {
    use openqg_core::scoring::{CovarianceFixture, MultiBlockFixture};
    let mut blocks = Vec::new();
    for path in paths {
        let bytes = std::fs::read(path)
            .with_context(|| format!("read covariance fixture {}", path.display()))?;
        if let Ok(single) = serde_json::from_slice::<CovarianceFixture>(&bytes) {
            let reg = single
                .into_registered()
                .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
            anyhow::ensure!(
                reg.positive_definite,
                "{}: covariance is not positive definite",
                path.display()
            );
            blocks.push(reg.block);
        } else {
            let multi: MultiBlockFixture = serde_json::from_slice(&bytes)
                .with_context(|| format!("parse covariance fixture {}", path.display()))?;
            for reg in multi
                .into_registered()
                .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?
            {
                anyhow::ensure!(
                    reg.positive_definite,
                    "{}: block {} is not positive definite",
                    path.display(),
                    reg.id
                );
                blocks.push(reg.block);
            }
        }
    }
    Ok(blocks)
}

pub(crate) fn replay_ledger(
    ledger_path: &Path,
    observables_path: &Path,
    covariance: &[PathBuf],
) -> Result<(usize, usize)> {
    let blocks = load_covariance_blocks(covariance)?;
    let observables = load_observables(observables_path)?;
    anyhow::ensure!(!observables.is_empty(), "no observables loaded");
    let baseline_ll = super::physics_score::baseline_log_likelihood_cov(&observables, &blocks);
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
        let sc = score_proposal(&doc, &observables, &blocks, baseline_ll);
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
        // Five observables matching the theory_population integration test (seed-42 config).
        // Uniform spacing avoids asymmetric data-fit pressure that can kill all mutated theories
        // in early generations for certain RNG seeds.
        let lines = [
            r#"{"observable_id":"bao_dv_z038","kind":"cosmology","value":1.0,"uncertainty":0.05,"unit":"x"}"#,
            r#"{"observable_id":"bao_dv_z051","kind":"cosmology","value":1.1,"uncertainty":0.05,"unit":"x"}"#,
            r#"{"observable_id":"fsigma8_z038","kind":"cosmology","value":1.2,"uncertainty":0.05,"unit":"x"}"#,
            r#"{"observable_id":"fsigma8_z051","kind":"cosmology","value":1.3,"uncertainty":0.05,"unit":"x"}"#,
            r#"{"observable_id":"h0_riess","kind":"cosmology","value":1.4,"uncertainty":0.05,"unit":"x"}"#,
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
            max_generations: 6,
            seed: 1,
        };
        let run_dir = run_population(&obs, &tmp, cfg, "test-pop", &[], None, None).expect("run");

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
        assert_eq!(lines.len(), 6);
        for line in &lines[1..] {
            let g: Value = serde_json::from_str(line).unwrap();
            assert!(g["new_fingerprints"].as_u64().unwrap() >= 1);
        }

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn oracle_command_passing_exit0_keeps_gate_passed() {
        let tmp =
            std::env::temp_dir().join(format!("openqg-oracle-pass-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let obs = write_observables(&tmp);

        let cfg = EvolveConfig {
            population_size: 9,
            max_generations: 6,
            seed: 2,
        };
        // `true` (or `sh -c true`) exits 0 — oracle should pass.
        let run_dir =
            run_population(&obs, &tmp, cfg, "oracle-pass", &[], None, Some("true")).expect("run");

        let qg: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(run_dir.join("quality-gate.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            qg["checks"]["oracle"]["passed"],
            serde_json::Value::Bool(true),
            "oracle=true should pass: {qg}"
        );
        assert_eq!(qg["passed"], serde_json::Value::Bool(true));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn oracle_command_failing_exit1_fails_gate() {
        let tmp =
            std::env::temp_dir().join(format!("openqg-oracle-fail-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let obs = write_observables(&tmp);

        let cfg = EvolveConfig {
            population_size: 9,
            max_generations: 6,
            seed: 3,
        };
        // `false` exits 1 — oracle should reject.
        let run_dir =
            run_population(&obs, &tmp, cfg, "oracle-fail", &[], None, Some("false")).expect("run");

        let qg: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(run_dir.join("quality-gate.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            qg["checks"]["oracle"]["passed"],
            serde_json::Value::Bool(false),
            "oracle=false should fail: {qg}"
        );
        // Overall gate must also fail when oracle rejects.
        assert_eq!(qg["passed"], serde_json::Value::Bool(false));

        let _ = fs::remove_dir_all(&tmp);
    }
}
