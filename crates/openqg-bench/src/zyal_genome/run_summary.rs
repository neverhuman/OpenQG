use super::*;

pub(crate) fn run_event(
    run_id: &str,
    variant: &str,
    event_type: &str,
    generation_id: &str,
    stage_id: &str,
    parent_generation_id: Option<String>,
    mutation_op: Option<String>,
    stage_family: &str,
    route_backend: &str,
    route_tier: &str,
    router_state: &str,
    artifact_paths: Value,
    judge: Value,
    scores: Value,
) -> Value {
    let candidate_id = if stage_id == "run" || stage_id == "generation" {
        generation_id.to_string()
    } else {
        format!("{generation_id}-{stage_id}")
    };
    json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "run_event",
        "event_type": event_type,
        "run_id": run_id,
        "variant": variant,
        "generation_id": generation_id,
        "stage_id": stage_id,
        "candidate_id": candidate_id,
        "parent_generation_id": parent_generation_id,
        "mutation_op": mutation_op,
        "stage_family": stage_family,
        "route_backend": route_backend,
        "route_tier": route_tier,
        "router_state": router_state,
        "artifact_paths": artifact_paths,
        "judge": judge,
        "local_score": field_or(&scores, "local_score", zero_f64_json),
        "interface_score": field_or(&scores, "interface_score", zero_f64_json),
        "macro_score": field_or(&scores, "macro_score", zero_f64_json),
        "innovation_score": field_or(&scores, "innovation_score", zero_f64_json),
        "novelty_score": field_or(&scores, "novelty_score", zero_f64_json),
        "failure_penalty": field_or(&scores, "failure_penalty", zero_f64_json),
        "final_score": field_or(&scores, "final_score", zero_f64_json),
    })
}

pub(crate) fn build_run_summary(
    run_id: &str,
    variant: &str,
    runbook: &Value,
    runbook_path: &Path,
    max_generations: usize,
    seed: u64,
    dry_run: bool,
    jailgun_available: bool,
    generation_scores: &[f64],
    stage_ledgers: &[Value],
    stage_registry: &[StagePackage],
    router_state: &str,
    degraded_router: bool,
    stage_summary_paths: &[PathBuf],
    run_dir: &Path,
    hybrid_evolution: Option<&Value>,
    population: &PopulationConfig,
) -> Value {
    let quality_scores = hybrid_evolution
        .and_then(|value| value.get("generation_scores").and_then(Value::as_array))
        .map(|scores| scores.iter().filter_map(Value::as_f64).collect::<Vec<_>>())
        .filter(|scores| !scores.is_empty())
        .unwrap_or_else(|| generation_scores.to_vec());
    let hybrid_best = hybrid_evolution
        .and_then(|value| value.get("best_score_seen").and_then(Value::as_f64))
        .unwrap_or(0.0);
    let best_score_seen = generation_scores
        .iter()
        .copied()
        .fold(0.0, f64::max)
        .max(hybrid_best);
    let rolling_5_median = median(&quality_scores[quality_scores.len().saturating_sub(5)..]);
    let deltas: Vec<f64> = quality_scores
        .windows(2)
        .map(|window| window[1] - window[0])
        .collect();
    let best_nonregressive_delta = deltas
        .iter()
        .copied()
        .filter(|delta| *delta >= 0.0)
        .fold(0.0, f64::max);
    let regression_rate = regression_rate(&quality_scores);
    let total_stage_runs = stage_ledgers.len();
    let degraded_route_count = stage_ledgers
        .iter()
        .filter(|entry| {
            entry.get("router_state").and_then(Value::as_str) == Some("degraded_router")
                || entry
                    .get("failure_modes")
                    .and_then(Value::as_array)
                    .map(|modes| {
                        modes
                            .iter()
                            .any(|mode| mode.as_str() == Some("degraded_router"))
                    })
                    .unwrap_or(false)
        })
        .count();
    let decoy_failures = stage_ledgers
        .iter()
        .filter(|entry| {
            entry
                .get("failure_modes")
                .and_then(Value::as_array)
                .map(|modes| {
                    modes.iter().any(|mode| {
                        matches!(
                            mode.as_str(),
                            Some(
                                "decoy_failure"
                                    | "decoy_failed"
                                    | "failed_decoy"
                                    | "test_decoy_failure"
                            )
                        )
                    })
                })
                .unwrap_or(false)
        })
        .count();
    let unique_contributions = stage_ledgers
        .iter()
        .filter(|entry| {
            entry
                .get("innovation_score")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
                >= 0.45
                && entry
                    .get("final_score")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0)
                    >= 0.55
        })
        .filter_map(|entry| entry.get("stage_id").and_then(Value::as_str))
        .collect::<BTreeSet<_>>()
        .len();
    let total_time = stage_ledgers
        .iter()
        .map(|entry| {
            entry
                .get("time_seconds")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
        })
        .sum::<f64>()
        .max(1.0);
    let throughput = max_generations as f64 / total_time;
    let fail_stop_rate = if total_stage_runs == 0 {
        0.0
    } else {
        stage_ledgers
            .iter()
            .filter(|entry| {
                entry
                    .get("pass_rate")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0)
                    < 1.0
            })
            .count() as f64
            / total_stage_runs as f64
    };
    let lineage_missing = hybrid_evolution
        .and_then(|value| value.get("lineage"))
        .and_then(|value| value.get("missing_parent_ids"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let stage_scores = stage_registry
        .iter()
        .map(|stage| {
            let scores = stage_ledgers
                .iter()
                .filter(|entry| entry.get("stage_id").and_then(Value::as_str) == Some(stage.stage_id.as_str()))
                .filter_map(|entry| entry.get("final_score").and_then(Value::as_f64))
                .collect::<Vec<_>>();
            json!({
                "stage_id": stage.stage_id,
                "final_score": if scores.is_empty() { 0.0 } else { scores.iter().sum::<f64>() / scores.len() as f64 },
            })
        })
        .collect::<Vec<_>>();
    let pareto = hybrid_evolution
        .and_then(|value| value.get("pareto_snapshot").cloned())
        .unwrap_or_else(|| pareto_snapshot(generation_scores, &stage_scores));
    let score_blend = score_blend(runbook, Some(population));
    let mut summary = json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "run_summary",
        "run_id": run_id,
        "variant": variant,
        "runbook_path": runbook_path.display().to_string(),
        "output_root": run_dir.parent().and_then(|p| p.parent()).map(|p| p.display().to_string()).unwrap_or_else(|| DEFAULT_OUTPUT_ROOT.to_string()),
        "run_dir": run_dir.display().to_string(),
        "generation_count": max_generations,
        "stage_count": stage_registry.len(),
        "candidate_count": hybrid_evolution.and_then(|value| value.get("candidate_count").and_then(Value::as_u64)).map(|v| v as usize).unwrap_or(stage_ledgers.len()),
        "best_score_seen": round6(best_score_seen),
        "rolling_5_median": round6(rolling_5_median),
        "best_nonregressive_delta": round6(best_nonregressive_delta),
        "unique_contributions": unique_contributions,
        "decoy_failures": decoy_failures,
        "degraded_route_count": degraded_route_count,
        "regression_rate": round6(regression_rate),
        "throughput": round6(throughput),
        "fail_stop_rate": round6(fail_stop_rate),
        "score_blend": score_blend,
        "router_state": router_state,
        "degraded_router": degraded_router,
        "lineage": {
            "acyclic": hybrid_evolution.and_then(|value| value.get("lineage")).and_then(|value| value.get("acyclic")).and_then(Value::as_bool).unwrap_or(lineage_missing == 0),
            "missing_parent_ids": lineage_missing,
        },
        "pareto_snapshot": pareto,
        "stage_summaries": stage_summary_paths.iter().map(|path| path.display().to_string()).collect::<Vec<_>>(),
        "validation": {
            "stage_count": stage_registry.len(),
            "stage_ids": stage_registry.iter().map(|stage| stage.stage_id.clone()).collect::<Vec<_>>(),
        },
        "seed": seed,
        "dry_run": dry_run,
        "jailgun_available": jailgun_available,
    });
    if let Some(hybrid_evolution) = hybrid_evolution {
        merge_object(
            &mut summary,
            &json!({
                "hybrid_baseline_score": HYBRID_BASELINE_SCORE,
                "hybrid_baseline_delta": round6(best_score_seen - HYBRID_BASELINE_SCORE),
                "population": {
                    "population_size": population.population_size,
                    "islands": population.islands,
                    "island_names": population.island_names,
                    "new_info_refresh": population.new_info_refresh,
                    "novelty_weight": population.novelty_weight,
                    "diversity_targets": population.diversity_targets,
                    "promotion_gates": population.promotion_gates,
                    "degraded_penalties": population.degraded_penalties,
                },
                "diversity_metrics": field_or(hybrid_evolution, "diversity_metrics", empty_object),
                "novelty_archive": field_or(hybrid_evolution, "novelty_archive", empty_string_json),
                "island_leaderboard": field_or(hybrid_evolution, "island_leaderboard", empty_object),
                "fun_summary": field_or(hybrid_evolution, "fun_summary", empty_object),
                "generation_champions": field_or(hybrid_evolution, "generation_champions", empty_array_json),
                "lineage": field_or(hybrid_evolution, "lineage", default_lineage_summary),
            }),
        );
    }
    summary
}

pub(crate) fn score_blend(runbook: &Value, population: Option<&PopulationConfig>) -> Value {
    let novelty_default = population
        .map(|population| population.novelty_weight)
        .unwrap_or(0.0);
    let evaluation = field_or(runbook, "evaluation", empty_object);
    let score_blend = field_or(&evaluation, "score_blend", empty_object);
    json!({
        "local_score": score_blend.get("local_score").and_then(Value::as_f64).unwrap_or(0.30),
        "interface_score": score_blend.get("interface_score").and_then(Value::as_f64).unwrap_or(0.20),
        "macro_score": score_blend.get("macro_score").and_then(Value::as_f64).unwrap_or(0.30),
        "innovation_score": score_blend.get("innovation_score").and_then(Value::as_f64).unwrap_or(0.15),
        "novelty_score": score_blend.get("novelty_score").and_then(Value::as_f64).unwrap_or(novelty_default),
        "failure_penalty": score_blend.get("failure_penalty").and_then(Value::as_f64).unwrap_or(0.05),
    })
}

pub(crate) fn pareto_snapshot(generation_scores: &[f64], stage_scores: &[Value]) -> Value {
    json!({
        "frontier_size": generation_scores.len().min(stage_scores.len()),
        "points": generation_scores.iter().enumerate().map(|(index, score)| json!({
            "generation_index": index + 1,
            "score": round6(*score),
        })).collect::<Vec<_>>(),
    })
}
