use super::*;

pub fn emit(run_dir: Option<PathBuf>, root: Option<PathBuf>) -> Result<()> {
    match (run_dir, root) {
        (Some(run_dir), None) => {
            emit_run_offline_eval(&run_dir)?;
            println!("emitted offline evaluation for {}", run_dir.display());
        }
        (None, Some(root)) => {
            emit_root_comparison(&root)?;
            println!("emitted comparison for {}", root.display());
        }
        _ => bail!("pass exactly one of --run-dir or --root"),
    }
    Ok(())
}

pub fn plot_index(run_dir: Option<PathBuf>, root: Option<PathBuf>) -> Result<()> {
    match (run_dir, root) {
        (Some(run_dir), None) => {
            let path = emit_run_plot_index(&run_dir)?;
            println!("wrote plot index to {}", path.display());
        }
        (None, Some(root)) => {
            let path = emit_root_plot_index(&root)?;
            println!("wrote plot index to {}", path.display());
        }
        _ => bail!("pass exactly one of --run-dir or --root"),
    }
    Ok(())
}

pub(crate) fn emit_run_offline_eval(run_dir: &Path) -> Result<Value> {
    let summary_path = run_dir.join("run-summary.json");
    let summary: Value = if summary_path.exists() {
        serde_json::from_str(&fs::read_to_string(&summary_path)?)
            .with_context(|| format!("parse {}", summary_path.display()))?
    } else {
        json!({})
    };
    let stage_rankings = read_stage_rankings(run_dir);
    let warnings = warnings_from_summary(&summary);
    let offline_eval = json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "offline_eval",
        "run_id": summary.get("run_id").cloned().unwrap_or_else(|| json!(infer_run_id_from_path(run_dir))),
        "variant": summary.get("variant").cloned().unwrap_or_else(|| json!(null)),
        "scorecard": summary,
        "stage_rankings": stage_rankings,
        "warnings": warnings,
    });
    write_json(&run_dir.join("offline-eval.json"), &offline_eval)?;
    write_markdown(
        &run_dir.join("offline-eval.md"),
        &render_run_markdown(&offline_eval),
    )?;
    Ok(offline_eval)
}

pub(crate) fn emit_root_comparison(root: &Path) -> Result<Value> {
    let mut variants = Vec::new();
    if root.exists() {
        for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if path.file_name().and_then(|name| name.to_str()) != Some("run-summary.json") {
                continue;
            }
            if path
                .components()
                .any(|component| component.as_os_str() == "latest")
            {
                continue;
            }
            let summary: Value = serde_json::from_str(&fs::read_to_string(path)?)
                .with_context(|| format!("parse {}", path.display()))?;
            variants.push(json!({
                "variant": summary.get("variant").cloned().unwrap_or_else(|| json!(null)),
                "scorecard": summary,
            }));
        }
    }
    let mut ranking = variants
        .iter()
        .map(|entry| {
            let scorecard = entry.get("scorecard").cloned().unwrap_or_else(empty_object);
            json!({
                "variant": entry.get("variant").cloned().unwrap_or_else(|| json!(null)),
                "final_score": scorecard.get("best_score_seen").and_then(Value::as_f64).unwrap_or(0.0),
            })
        })
        .collect::<Vec<_>>();
    ranking.sort_by(|a, b| {
        b.get("final_score")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
            .partial_cmp(&a.get("final_score").and_then(Value::as_f64).unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let comparison = json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "comparison",
        "root": root.display().to_string(),
        "variants": variants,
        "ranking": ranking,
    });
    write_json(&root.join("comparison.json"), &comparison)?;
    write_markdown(
        &root.join("comparison.md"),
        &render_comparison_markdown(&comparison),
    )?;
    emit_hybrid_report_copies(root)?;
    emit_root_plot_index(root)?;
    Ok(comparison)
}

pub(crate) fn emit_hybrid_report_copies(root: &Path) -> Result<()> {
    let hybrid_latest = root.join("hybrid").join("latest");
    for (source_name, target_name) in [
        ("novelty-archive.json", "hybrid-novelty-archive.json"),
        ("island-leaderboard.json", "hybrid-island-leaderboard.json"),
        ("lineage-graph.md", "hybrid-lineage-graph.md"),
    ] {
        let source = hybrid_latest.join(source_name);
        if source.exists() {
            fs::copy(&source, root.join(target_name))?;
        }
    }
    Ok(())
}

pub(crate) fn emit_run_plot_index(run_dir: &Path) -> Result<PathBuf> {
    let summary_path = run_dir.join("run-summary.json");
    let summary: Value = if summary_path.exists() {
        serde_json::from_str(&fs::read_to_string(&summary_path)?)
            .with_context(|| format!("parse {}", summary_path.display()))?
    } else {
        json!({})
    };
    let ledgers = json!({
        "run_events": run_dir.join("run-events.jsonl").display().to_string(),
        "stage_ledger": run_dir.join("stage-ledger.jsonl").display().to_string(),
        "population_ledger": run_dir.join("population-ledger.jsonl").display().to_string(),
        "lineage_graph": run_dir.join("lineage-graph.jsonl").display().to_string(),
        "metrics": run_dir.join("metrics-timeseries.jsonl").display().to_string(),
        "generation": run_dir.join("generation-ledger.jsonl").display().to_string(),
        "stage_variant": run_dir.join("stage-variant-ledger.jsonl").display().to_string(),
        "live_call": run_dir.join("live-call-ledger.jsonl").display().to_string(),
        "research": run_dir.join("research-ledger.jsonl").display().to_string(),
        "memory": run_dir.join("memory-ledger.jsonl").display().to_string(),
        "preflight": run_dir.join("preflight.json").display().to_string(),
        "quality_gate": run_dir.join("quality-gate.json").display().to_string(),
    });
    let series = json!({
        "champion": {"ledger": ledgers["generation"].clone(), "metric_name": "hybrid_champion_score"},
        "rollup": {"ledger": ledgers["generation"].clone(), "metric_name": "deterministic_rollup_score"},
        "stage": {"ledger": ledgers["metrics"].clone(), "metric_name": "stage_final_score"},
        "island": {"ledger": ledgers["population_ledger"].clone(), "field": "candidates[].island"},
        "live_call": {"ledger": ledgers["live_call"].clone(), "field": "status"},
        "research": {"ledger": ledgers["research"].clone(), "field": "accepted"},
    });
    let index = json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "plot_index",
        "run_id": summary.get("run_id").cloned().unwrap_or_else(|| json!(infer_run_id_from_path(run_dir))),
        "variant": summary.get("variant").cloned().unwrap_or_else(|| json!(null)),
        "run_dir": run_dir.display().to_string(),
        "generation_count": summary.get("generation_count").cloned(),
        "ledgers": ledgers,
        "series": series,
        "summary": {
            "best_score_seen": summary.get("best_score_seen").cloned(),
            "rolling_5_median": summary.get("rolling_5_median").cloned(),
            "candidate_count": summary.get("candidate_count").cloned(),
            "quality_gate": run_dir.join("quality-gate.json").display().to_string(),
        },
    });
    let path = run_dir.join("plot-index.json");
    write_json(&path, &index)?;
    Ok(path)
}

pub(crate) fn emit_root_plot_index(root: &Path) -> Result<PathBuf> {
    let mut runs = Vec::new();
    if root.exists() {
        for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if path.file_name().and_then(|name| name.to_str()) != Some("run-summary.json") {
                continue;
            }
            if path
                .components()
                .any(|component| component.as_os_str() == "latest")
            {
                continue;
            }
            let run_dir = path.parent().unwrap_or(root);
            emit_run_plot_index(run_dir)?;
            let summary: Value = serde_json::from_str(&fs::read_to_string(path)?)
                .with_context(|| format!("parse {}", path.display()))?;
            runs.push(json!({
                "run_id": summary.get("run_id").cloned(),
                "variant": summary.get("variant").cloned(),
                "run_dir": run_dir.display().to_string(),
                "plot_index": run_dir.join("plot-index.json").display().to_string(),
                "quality_gate": run_dir.join("quality-gate.json").display().to_string(),
                "best_score_seen": summary.get("best_score_seen").cloned(),
                "generation_count": summary.get("generation_count").cloned(),
            }));
        }
    }
    let index = json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "plot_index_root",
        "root": root.display().to_string(),
        "runs": runs,
    });
    let path = root.join("plot-index.json");
    write_json(&path, &index)?;
    Ok(path)
}

pub(crate) fn run_offline_eval_markdown_path(run_dir: &Path) -> PathBuf {
    run_dir.join("offline-eval.md")
}
