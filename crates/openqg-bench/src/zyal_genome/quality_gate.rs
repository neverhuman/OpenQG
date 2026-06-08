use super::*;

pub fn quality_gate(run_dir: &Path) -> Result<()> {
    let _ = emit_run_plot_index(run_dir);
    let report = build_quality_gate_report(run_dir)?;
    write_json(&run_dir.join("quality-gate.json"), &report)?;
    write_markdown(
        &run_dir.join("quality-gate.md"),
        &render_quality_gate_markdown(&report),
    )?;
    emit_run_plot_index(run_dir)?;
    let passed = report
        .get("passed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !passed {
        bail!(
            "quality gate failed for {}; see {}",
            report
                .get("run_id")
                .and_then(Value::as_str)
                .unwrap_or("unknown"),
            run_dir.join("quality-gate.json").display()
        );
    }
    println!("quality gate passed for {}", run_dir.display());
    Ok(())
}

pub(crate) fn build_quality_gate_report(run_dir: &Path) -> Result<Value> {
    let checkpoint_path = run_dir.join("checkpoint.json");
    let summary_path = run_dir.join("run-summary.json");
    let offline_eval_path = run_dir.join("offline-eval.json");
    let live_ledger_path = run_dir.join("live-call-ledger.jsonl");
    let generation_ledger_path = run_dir.join("generation-ledger.jsonl");
    let run_events_path = run_dir.join("run-events.jsonl");
    let stage_ledger_path = run_dir.join("stage-ledger.jsonl");
    let plot_index_path = run_dir.join("plot-index.json");
    let preflight_path = run_dir.join("preflight.json");

    let mut artifact_checks = Vec::new();
    let checkpoint = read_required_json(&checkpoint_path, &mut artifact_checks);
    let summary = read_required_json(&summary_path, &mut artifact_checks);
    let offline_eval = read_required_json(&offline_eval_path, &mut artifact_checks);
    let live_records = read_required_jsonl(&live_ledger_path, &mut artifact_checks);
    let generation_records = read_required_jsonl(&generation_ledger_path, &mut artifact_checks);
    let run_events = read_required_jsonl(&run_events_path, &mut artifact_checks);
    let stage_ledgers = read_optional_jsonl(&stage_ledger_path);
    let preflight = read_optional_json(&preflight_path);

    let artifact_validity = artifact_checks.iter().all(|check| {
        check
            .get("passed")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    });
    let complete_generation = checkpoint
        .get("complete_generation")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let target_generation = checkpoint
        .get("target_generation")
        .and_then(Value::as_u64)
        .or_else(|| summary.get("generation_count").and_then(Value::as_u64))
        .unwrap_or(complete_generation as u64) as usize;
    let tier = if target_generation >= 1000 {
        "full"
    } else if target_generation >= 50 {
        "qualification"
    } else {
        "smoke"
    };
    let report_run_id = summary
        .get("run_id")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .unwrap_or_else(|| infer_run_id_from_path(run_dir));
    let live_total = live_records.len();
    let live_timeout_count = live_records
        .iter()
        .filter(|record| record.get("status").and_then(Value::as_str) == Some("timeout"))
        .count();
    let failed_live_calls = live_records
        .iter()
        .filter(|record| {
            record
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("failed")
                != "ok"
        })
        .count();
    let timeout_rate = if live_total == 0 {
        0.0
    } else {
        live_timeout_count as f64 / live_total as f64
    };
    let timeout_overrun_count = live_records
        .iter()
        .map(timeout_overrun_count)
        .sum::<usize>();
    let degraded_route_count = summary
        .get("degraded_route_count")
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or_else(|| degraded_route_count_from_ledgers(&stage_ledgers, &run_events));
    let decoy_failures = summary
        .get("decoy_failures")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let quality_rollups = hybrid_quality_rollup_series(&generation_records);
    let champion_scores = champion_scores(&summary, &generation_records);
    let stage_rollup_median = median(&quality_rollups);
    let rolling_5_median = median(&quality_rollups[quality_rollups.len().saturating_sub(5)..]);
    let regression_rate = regression_rate(&quality_rollups);
    let perfect_champions = champion_scores
        .iter()
        .filter(|score| **score >= 0.999999)
        .count();
    let champion_perfect_rate = if champion_scores.is_empty() {
        0.0
    } else {
        perfect_champions as f64 / champion_scores.len() as f64
    };
    let champions = summary
        .get("generation_champions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_else(Vec::new);
    let champion_islands = champions
        .iter()
        .filter_map(|champion| champion.get("island").and_then(Value::as_str))
        .map(ToString::to_string)
        .collect::<BTreeSet<_>>();
    let novelty_champions = champions
        .iter()
        .filter(|champion| champion.get("mode").and_then(Value::as_str) == Some("novelty"))
        .count();
    let novelty_champion_rate = if champions.is_empty() {
        0.0
    } else {
        novelty_champions as f64 / champions.len() as f64
    };
    let island_champion_counts = count_string_field(&champions, "island");
    let mode_champion_counts = count_string_field(&champions, "mode");
    let max_island_share = if champions.is_empty() {
        0.0
    } else {
        island_champion_counts.values().copied().max().unwrap_or(0) as f64 / champions.len() as f64
    };
    let run_plot_index_written = plot_index_path.is_file();
    let root_plot_index_path = genome_root_from_run_dir(run_dir).join("plot-index.json");
    let root_plot_index_written = root_plot_index_path.is_file();
    let route_nominal = degraded_route_count == 0
        && !summary
            .get("degraded_router")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    let preflight_ok = preflight
        .get("status")
        .and_then(Value::as_str)
        .map(|status| status == "ok")
        .unwrap_or(true);
    let hard_backend_required = preflight
        .get("backend_health")
        .and_then(|health| health.get("require_hard_backend"))
        .and_then(Value::as_bool)
        .unwrap_or_else(|| {
            summary.get("variant").and_then(Value::as_str) == Some("hybrid")
                && report_run_id.starts_with("hybrid-v2")
                && target_generation >= 10
        });
    let jailgun_proof_required = hard_backend_required
        && summary.get("variant").and_then(Value::as_str) == Some("hybrid")
        && target_generation >= 10;
    let hard_stage_live_calls = live_records
        .iter()
        .filter(|record| is_hard_stage_repair_live_call(record))
        .count();
    let hard_stage_jailgun_live_calls = live_records
        .iter()
        .filter(|record| is_hard_stage_repair_live_call(record))
        .filter(|record| {
            record.get("execution_backend").and_then(Value::as_str) == Some("jailgun_mcp")
        })
        .count();
    let failed_jailgun_live_calls = live_records
        .iter()
        .filter(|record| {
            record.get("execution_backend").and_then(Value::as_str) == Some("jailgun_mcp")
        })
        .filter(|record| record.get("status").and_then(Value::as_str) != Some("ok"))
        .count();
    let hard_stage_jailgun_proof_missing = live_records
        .iter()
        .filter(|record| is_hard_stage_repair_live_call(record))
        .filter(|record| !jailgun_live_call_has_proof(record))
        .count();

    let mut checks = Vec::new();
    add_check(
        &mut checks,
        "artifact_validity",
        artifact_validity,
        json!(artifact_checks),
        json!("all required artifacts parse"),
    );
    add_check(
        &mut checks,
        "preflight_status",
        preflight_ok,
        field_or(&preflight, "status", missing_allowed_status),
        json!("ok"),
    );
    add_check(
        &mut checks,
        "complete_generation",
        complete_generation >= target_generation,
        json!(complete_generation),
        json!(target_generation),
    );
    add_check(
        &mut checks,
        "failed_live_calls",
        failed_live_calls == 0,
        json!(failed_live_calls),
        json!(0),
    );
    add_check(
        &mut checks,
        "timeout_overrun_count",
        timeout_overrun_count == 0,
        json!(timeout_overrun_count),
        json!(0),
    );
    add_check(
        &mut checks,
        "degraded_route_count",
        degraded_route_count == 0 && route_nominal,
        json!(degraded_route_count),
        json!(0),
    );
    add_check(
        &mut checks,
        "decoy_failures",
        decoy_failures == 0,
        json!(decoy_failures),
        json!(0),
    );
    if jailgun_proof_required {
        add_check(
            &mut checks,
            "hard_stage_jailgun_live_calls",
            hard_stage_jailgun_live_calls > 0,
            json!(hard_stage_jailgun_live_calls),
            json!(">0"),
        );
        add_check(
            &mut checks,
            "failed_jailgun_live_calls",
            failed_jailgun_live_calls == 0,
            json!(failed_jailgun_live_calls),
            json!(0),
        );
        add_check(
            &mut checks,
            "hard_stage_jailgun_receipt_proof",
            hard_stage_jailgun_proof_missing == 0,
            json!(hard_stage_jailgun_proof_missing),
            json!(0),
        );
    }
    add_check(
        &mut checks,
        "run_plot_index",
        run_plot_index_written,
        json!(plot_index_path.display().to_string()),
        json!("exists"),
    );

    match tier {
        "full" => {
            add_check(
                &mut checks,
                "live_timeout_rate",
                timeout_rate <= 0.01,
                json!(round6(timeout_rate)),
                json!(0.01),
            );
            add_check(
                &mut checks,
                "regression_rate",
                regression_rate <= 0.25,
                json!(round6(regression_rate)),
                json!(0.25),
            );
            add_check(
                &mut checks,
                "champion_perfect_rate",
                champion_perfect_rate <= 0.30,
                json!(round6(champion_perfect_rate)),
                json!(0.30),
            );
            add_check(
                &mut checks,
                "rolling_5_median",
                rolling_5_median >= 0.76,
                json!(round6(rolling_5_median)),
                json!(0.76),
            );
            add_check(
                &mut checks,
                "root_plot_index",
                root_plot_index_written,
                json!(root_plot_index_path.display().to_string()),
                json!("exists"),
            );
        }
        "qualification" => {
            add_check(
                &mut checks,
                "live_timeout_rate",
                timeout_rate <= 0.01,
                json!(round6(timeout_rate)),
                json!(0.01),
            );
            add_check(
                &mut checks,
                "regression_rate",
                regression_rate <= 0.25,
                json!(round6(regression_rate)),
                json!(0.25),
            );
            add_check(
                &mut checks,
                "champion_perfect_rate",
                champion_perfect_rate <= 0.20,
                json!(round6(champion_perfect_rate)),
                json!(0.20),
            );
            add_check(
                &mut checks,
                "stage_rollup_median",
                stage_rollup_median >= 0.75,
                json!(round6(stage_rollup_median)),
                json!(0.75),
            );
            add_check(
                &mut checks,
                "champion_island_coverage",
                champion_islands.len() >= 4,
                json!(champion_islands.len()),
                json!(4),
            );
            add_check(
                &mut checks,
                "novelty_champion_rate",
                novelty_champion_rate >= 0.05,
                json!(round6(novelty_champion_rate)),
                json!(0.05),
            );
            add_check(
                &mut checks,
                "max_island_share",
                max_island_share <= 0.60,
                json!(round6(max_island_share)),
                json!(0.60),
            );
        }
        _ => {
            add_check(
                &mut checks,
                "live_timeout_rate",
                timeout_rate <= 0.10,
                json!(round6(timeout_rate)),
                json!(0.10),
            );
        }
    }

    let passed = checks.iter().all(|check| {
        check
            .get("passed")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    });
    Ok(json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "quality_gate",
        "run_id": report_run_id,
        "variant": field_or(&summary, "variant", null_json),
        "run_dir": run_dir.display().to_string(),
        "generated_at": now_iso8601(),
        "tier": tier,
        "passed": passed,
        "status": if passed { "passed" } else { "failed" },
        "metrics": {
            "complete_generation": complete_generation,
            "target_generation": target_generation,
            "live_total": live_total,
            "failed_live_calls": failed_live_calls,
            "live_timeout_count": live_timeout_count,
            "live_timeout_rate": round6(timeout_rate),
            "timeout_overrun_count": timeout_overrun_count,
            "degraded_route_count": degraded_route_count,
            "decoy_failures": decoy_failures,
            "regression_rate": round6(regression_rate),
            "stage_rollup_median": round6(stage_rollup_median),
            "rolling_5_median": round6(rolling_5_median),
            "quality_rollup_count": quality_rollups.len(),
            "champion_count": champion_scores.len(),
            "perfect_champions": perfect_champions,
            "champion_perfect_rate": round6(champion_perfect_rate),
            "hard_backend_required": hard_backend_required,
            "jailgun_proof_required": jailgun_proof_required,
            "hard_stage_live_calls": hard_stage_live_calls,
            "hard_stage_jailgun_live_calls": hard_stage_jailgun_live_calls,
            "failed_jailgun_live_calls": failed_jailgun_live_calls,
            "hard_stage_jailgun_proof_missing": hard_stage_jailgun_proof_missing,
            "champion_island_coverage": champion_islands.len(),
            "novelty_champions": novelty_champions,
            "novelty_champion_rate": round6(novelty_champion_rate),
            "max_island_share": round6(max_island_share),
            "island_champion_counts": island_champion_counts,
            "mode_champion_counts": mode_champion_counts,
            "route_health": if route_nominal { "nominal" } else { "degraded" },
        },
        "artifacts": {
            "checkpoint": checkpoint_path.display().to_string(),
            "run_summary": summary_path.display().to_string(),
            "offline_eval": offline_eval_path.display().to_string(),
            "live_call_ledger": live_ledger_path.display().to_string(),
            "generation_ledger": generation_ledger_path.display().to_string(),
            "run_events": run_events_path.display().to_string(),
            "run_plot_index": plot_index_path.display().to_string(),
            "root_plot_index": root_plot_index_path.display().to_string(),
            "preflight": preflight_path.display().to_string(),
        },
        "checks": checks,
        "artifact_checks": artifact_checks,
        "offline_eval_warnings": field_or(&offline_eval, "warnings", empty_array_json),
    }))
}

pub(crate) fn genome_root_from_run_dir(run_dir: &Path) -> PathBuf {
    run_dir
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_OUTPUT_ROOT))
}
