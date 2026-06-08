use super::*;

pub(crate) fn render_quality_gate_markdown(report: &Value) -> String {
    let metrics = report.get("metrics").cloned().unwrap_or_else(empty_object);
    let mut rows = vec![
        "# ZYAL Quality Gate".to_string(),
        String::new(),
        format!(
            "- Run: `{}`",
            report
                .get("run_id")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
        ),
        format!(
            "- Tier: `{}`",
            report
                .get("tier")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
        ),
        format!(
            "- Status: `{}`",
            report
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("failed")
        ),
        format!(
            "- Complete generation: `{}` / `{}`",
            metrics
                .get("complete_generation")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            metrics
                .get("target_generation")
                .and_then(Value::as_u64)
                .unwrap_or(0)
        ),
        format!(
            "- Live timeout rate: `{}`",
            metrics
                .get("live_timeout_rate")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
        ),
        format!(
            "- Degraded route count: `{}`",
            metrics
                .get("degraded_route_count")
                .and_then(Value::as_u64)
                .unwrap_or(0)
        ),
        format!(
            "- Regression rate: `{}`",
            metrics
                .get("regression_rate")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
        ),
        format!(
            "- Champion perfect rate: `{}`",
            metrics
                .get("champion_perfect_rate")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
        ),
        String::new(),
        "## Checks".to_string(),
        String::new(),
        "| Check | Status | Observed | Threshold |".to_string(),
        "| --- | --- | ---: | ---: |".to_string(),
    ];
    if let Some(checks) = report.get("checks").and_then(Value::as_array) {
        for check in checks {
            rows.push(format!(
                "| `{}` | `{}` | `{}` | `{}` |",
                check.get("name").and_then(Value::as_str).unwrap_or("check"),
                if check
                    .get("passed")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    "pass"
                } else {
                    "fail"
                },
                compact_json(check.get("observed").unwrap_or(&Value::Null)),
                compact_json(check.get("threshold").unwrap_or(&Value::Null)),
            ));
        }
    }
    rows.join("\n") + "\n"
}

pub(crate) fn compact_json(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        _ => serde_json::to_string(value).unwrap_or_else(|_| "null".to_string()),
    }
}

pub(crate) fn lineage_edges_are_acyclic(graph: &BTreeMap<String, Vec<String>>) -> bool {
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    for node in graph.keys() {
        if !dfs_acyclic(node, graph, &mut visiting, &mut visited) {
            return false;
        }
    }
    true
}

pub(crate) fn dfs_acyclic(
    node: &str,
    graph: &BTreeMap<String, Vec<String>>,
    visiting: &mut BTreeSet<String>,
    visited: &mut BTreeSet<String>,
) -> bool {
    if visited.contains(node) {
        return true;
    }
    if !visiting.insert(node.to_string()) {
        return false;
    }
    if let Some(parents) = graph.get(node) {
        for parent in parents {
            if !dfs_acyclic(parent, graph, visiting, visited) {
                return false;
            }
        }
    }
    visiting.remove(node);
    visited.insert(node.to_string());
    true
}

pub(crate) fn warnings_from_summary(summary: &Value) -> Vec<String> {
    let mut warnings = Vec::new();
    if summary
        .get("degraded_router")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        warnings.push("router degraded on at least one stage".to_string());
    }
    if summary
        .get("degraded_route_count")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        > 0
    {
        warnings.push("degraded route count is non-zero".to_string());
    }
    if summary
        .get("regression_rate")
        .and_then(Value::as_f64)
        .unwrap_or(0.0)
        > 0.25
    {
        warnings.push("regression rate exceeds 25%".to_string());
    }
    if summary
        .get("decoy_failures")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        > 0
    {
        warnings.push("decoy failures present in stage ledger".to_string());
    }
    warnings
}

pub(crate) fn render_run_markdown(offline_eval: &Value) -> String {
    let scorecard = offline_eval
        .get("scorecard")
        .cloned()
        .unwrap_or_else(empty_object);
    let mut rows = vec![
        "# ZYAL Offline Evaluation".to_string(),
        String::new(),
        format!(
            "- Run: `{}`",
            offline_eval
                .get("run_id")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
        ),
        format!(
            "- Variant: `{}`",
            offline_eval
                .get("variant")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
        ),
        format!(
            "- Best score seen: `{}`",
            scorecard
                .get("best_score_seen")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
        ),
        format!(
            "- Rolling 5 median: `{}`",
            scorecard
                .get("rolling_5_median")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
        ),
        format!(
            "- Best non-regressive delta: `{}`",
            scorecard
                .get("best_nonregressive_delta")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
        ),
        format!(
            "- Unique contributions: `{}`",
            scorecard
                .get("unique_contributions")
                .and_then(Value::as_u64)
                .unwrap_or(0)
        ),
        format!(
            "- Decoy failures: `{}`",
            scorecard
                .get("decoy_failures")
                .and_then(Value::as_u64)
                .unwrap_or(0)
        ),
        format!(
            "- Degraded routes: `{}`",
            scorecard
                .get("degraded_route_count")
                .and_then(Value::as_u64)
                .unwrap_or(0)
        ),
    ];
    if let Some(hybrid_baseline_score) = scorecard
        .get("hybrid_baseline_score")
        .and_then(Value::as_f64)
    {
        rows.extend([
            String::new(),
            "## Hybrid Highlights".to_string(),
            String::new(),
            format!("- Hybrid baseline: `{hybrid_baseline_score}`"),
            format!(
                "- Baseline delta: `{}`",
                scorecard
                    .get("hybrid_baseline_delta")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0)
            ),
        ]);
    }
    rows.extend([
        String::new(),
        "## Stage Rankings".to_string(),
        String::new(),
        "| Stage | Final Score | Delta | Route |".to_string(),
        "| --- | ---: | ---: | --- |".to_string(),
    ]);
    if let Some(stage_rankings) = offline_eval.get("stage_rankings").and_then(Value::as_array) {
        for row in stage_rankings {
            rows.push(format!(
                "| `{}` | `{}` | `{}` | `{}` |",
                row.get("stage_id")
                    .and_then(Value::as_str)
                    .unwrap_or("stage"),
                row.get("final_score")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0),
                row.get("delta_score")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0),
                row.get("route")
                    .and_then(Value::as_str)
                    .unwrap_or("jnoccio/standard")
            ));
        }
    }
    if let Some(warnings) = offline_eval.get("warnings").and_then(Value::as_array) {
        if !warnings.is_empty() {
            rows.extend([String::new(), "## Warnings".to_string(), String::new()]);
            for warning in warnings {
                rows.push(format!("- {}", warning.as_str().unwrap_or("warning")));
            }
        }
    }
    rows.join("\n") + "\n"
}

pub(crate) fn render_comparison_markdown(comparison: &Value) -> String {
    let mut rows = vec![
        "# ZYAL Genome Comparison".to_string(),
        String::new(),
        format!(
            "- Root: `{}`",
            comparison.get("root").and_then(Value::as_str).unwrap_or("")
        ),
        String::new(),
        "| Variant | Best Score | Rolling 5 Median |".to_string(),
        "| --- | ---: | ---: |".to_string(),
    ];
    if let Some(variants) = comparison.get("variants").and_then(Value::as_array) {
        for entry in variants {
            let scorecard = entry.get("scorecard").cloned().unwrap_or_else(empty_object);
            rows.push(format!(
                "| `{}` | `{}` | `{}` |",
                entry
                    .get("variant")
                    .and_then(Value::as_str)
                    .unwrap_or("variant"),
                scorecard
                    .get("best_score_seen")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0),
                scorecard
                    .get("rolling_5_median")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0)
            ));
        }
    }
    rows.join("\n") + "\n"
}

pub(crate) fn write_stage_summaries(
    run_dir: &Path,
    stage_registry: &[StagePackage],
    stage_ledgers: &[Value],
    stage_score_history: &BTreeMap<String, Vec<f64>>,
    variant: &str,
    run_id: &str,
) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for stage in stage_registry {
        let stage_dir = run_dir.join("stages").join(&stage.stage_id);
        fs::create_dir_all(&stage_dir)?;
        let scores = stage_score_history
            .get(&stage.stage_id)
            .cloned()
            .unwrap_or_default();
        let entries: Vec<&Value> = stage_ledgers
            .iter()
            .filter(|entry| {
                entry.get("stage_id").and_then(Value::as_str) == Some(stage.stage_id.as_str())
            })
            .collect();
        let summary = json!({
            "schema_version": SCHEMA_VERSION,
            "record_kind": "stage_summary",
            "run_id": run_id,
            "variant": variant,
            "stage_id": stage.stage_id,
            "stage_name": stage.name,
            "generations": scores.len(),
            "mean_final_score": if scores.is_empty() { 0.0 } else { scores.iter().sum::<f64>() / scores.len() as f64 },
            "best_final_score": scores.iter().copied().fold(0.0, f64::max),
            "pass_rate": if entries.is_empty() { 0.0 } else { entries.iter().map(|entry| entry.get("pass_rate").and_then(Value::as_f64).unwrap_or(0.0)).sum::<f64>() / entries.len() as f64 },
            "route_backends": entries.iter().filter_map(|entry| entry.get("route_backend").and_then(Value::as_str)).collect::<BTreeSet<_>>().into_iter().map(ToString::to_string).collect::<Vec<_>>(),
            "failure_modes": entries.iter().flat_map(|entry| entry.get("failure_modes").and_then(Value::as_array).cloned().unwrap_or_default()).filter_map(|mode| mode.as_str().map(ToString::to_string)).collect::<BTreeSet<_>>().into_iter().collect::<Vec<_>>(),
            "mutation_ops": entries.iter().filter_map(|entry| entry.get("mutation_op").and_then(Value::as_str).map(ToString::to_string)).collect::<BTreeSet<_>>().into_iter().collect::<Vec<_>>(),
        });
        let path = stage_dir.join("stage-summary.json");
        write_json(&path, &summary)?;
        paths.push(path);
    }
    Ok(paths)
}

pub(crate) fn render_lineage_markdown(
    lineage_edges: &[Value],
    generation_champions: &[Value],
) -> String {
    let mut rows = vec![
        "# ZYAL Genome Lineage".to_string(),
        String::new(),
        "| Generation | Champion | Score |".to_string(),
        "| --- | --- | ---: |".to_string(),
    ];
    for champion in generation_champions {
        rows.push(format!(
            "| `{}` | `{}` | `{}` |",
            champion
                .get("generation_id")
                .and_then(Value::as_str)
                .unwrap_or("g0000"),
            champion
                .get("candidate_id")
                .and_then(Value::as_str)
                .unwrap_or("candidate"),
            champion
                .get("final_score")
                .and_then(Value::as_f64)
                .unwrap_or(0.0),
        ));
    }
    rows.push(String::new());
    rows.push("## Edges".to_string());
    rows.push(String::new());
    for edge in lineage_edges {
        rows.push(format!(
            "- `{}` -> `{}` ({})",
            edge.get("parent_candidate_id")
                .and_then(Value::as_str)
                .unwrap_or("root"),
            edge.get("child_candidate_id")
                .and_then(Value::as_str)
                .unwrap_or("child"),
            edge.get("mutation_op")
                .and_then(Value::as_str)
                .unwrap_or("mutation"),
        ));
    }
    rows.join("\n") + "\n"
}
