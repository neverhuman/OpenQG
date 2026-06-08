use super::*;

pub(crate) fn champion_summary_record(
    generation_id: &str,
    champion: &Value,
    promotion_reason: PromotionReason,
) -> Value {
    json!({
        "generation_id": generation_id,
        "candidate_id": field_or(champion, "candidate_id", empty_string_json),
        "island": field_or(champion, "island", empty_string_json),
        "mode": field_or(champion, "mode", empty_string_json),
        "final_score": nested_field_or(champion, "scores", "final_score", zero_f64_json),
        "novelty_score": nested_field_or(champion, "scores", "novelty_score", zero_f64_json),
        "promotion_reason": promotion_reason.as_str(),
        "source_card_ids": field_or(champion, "source_card_ids", empty_array_json),
        "frontier_claim": field_or(champion, "frontier_claim", empty_string_json),
        "falsifiable_tests": field_or(champion, "falsifiable_tests", empty_array_json),
        "known_failure_modes": field_or(champion, "known_failure_modes", empty_array_json),
        "review_priority": field_or(champion, "review_priority", empty_string_json),
        "retained_from_generation_id": field_or(champion, "retained_from_generation_id", null_json),
    })
}

pub(crate) fn population_snapshot(
    run_id: &str,
    generation_id: &str,
    population_size: usize,
    island_names: &[String],
    mode_counts: &BTreeMap<String, usize>,
    generation_candidates: &[Value],
    promoted: &[Value],
    champion_candidate_id: &Value,
    best_final_score: f64,
    diversity_metrics: Value,
) -> Value {
    json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "population_snapshot",
        "run_id": run_id,
        "generation_id": generation_id,
        "population_size": population_size,
        "islands": island_names,
        "mode_counts": mode_counts,
        "candidate_ids": generation_candidates.iter().filter_map(|candidate| candidate.get("candidate_id").and_then(Value::as_str)).map(ToString::to_string).collect::<Vec<_>>(),
        "promoted_candidate_ids": promoted.iter().filter_map(|candidate| candidate.get("candidate_id").and_then(Value::as_str)).map(ToString::to_string).collect::<Vec<_>>(),
        "champion_candidate_id": champion_candidate_id,
        "best_final_score": round6(best_final_score),
        "diversity_metrics": diversity_metrics,
        "candidates": generation_candidates,
    })
}

pub(crate) fn stage_variant_record(
    run_id: &str,
    variant: &str,
    generation_id: &str,
    candidate_id: &str,
    stage: &StagePackage,
    parent_stage_variant_ids: &[String],
    score_breakdown: &Value,
    research_refs: &[String],
) -> Value {
    let prompt_hash = if stage.prompt_hash.is_empty() {
        stable_hash(&stage.purpose)
    } else {
        stage.prompt_hash.clone()
    };
    let variant_hash = short_hash(
        &format!(
            "{}:{}:{}:{:?}",
            stage.stage_id, generation_id, prompt_hash, parent_stage_variant_ids
        ),
        12,
    );
    json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "stage_variant",
        "run_id": run_id,
        "variant": variant,
        "generation_id": generation_id,
        "stage_id": stage.stage_id,
        "candidate_id": candidate_id,
        "stage_variant_id": format!("sv-{}-{}", stage.stage_id, variant_hash),
        "parent_stage_variant_ids": parent_stage_variant_ids,
        "algorithm_summary": format!("{}: {} mutation={}", stage.name, stage.purpose, stage.mutation_op),
        "prompt_hash": prompt_hash,
        "memory_refs": memory_refs_for_stage(stage),
        "research_refs": research_refs,
        "evidence_bundle_path": format!("stages/{}/generations/{}/evidence-bundle.json", stage.stage_id, generation_id),
        "score_breakdown": score_breakdown,
    })
}

pub(crate) fn stage_ledger_record(
    run_id: &str,
    variant: &str,
    generation_id: &str,
    stage_id: &str,
    candidate_id: &str,
    stage_name: &str,
    route: &RoutePolicy,
    mutation_op: Option<String>,
    parent_generation_id: Option<String>,
    delta_score: f64,
    pass_rate: f64,
    time_seconds: f64,
    failure_modes: Vec<Value>,
    artifact_paths: Value,
    scores: &Value,
) -> Value {
    let parent_generation_id = unwrap_or_value(parent_generation_id, "root".to_string());
    json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "stage_ledger",
        "run_id": run_id,
        "variant": variant,
        "generation_id": generation_id,
        "stage_id": stage_id,
        "candidate_id": candidate_id,
        "stage_name": stage_name,
        "route_backend": route.route_backend,
        "route_tier": route.route_tier,
        "router_state": route.router_state,
        "mutation_op": mutation_op,
        "parent_generation_id": parent_generation_id,
        "delta_score": round6(delta_score),
        "pass_rate": round6(pass_rate),
        "time_seconds": round6(time_seconds),
        "failure_modes": failure_modes,
        "artifact_paths": artifact_paths,
        "local_score": field_or(scores, "local_score", zero_f64_json),
        "interface_score": field_or(scores, "interface_score", zero_f64_json),
        "macro_score": field_or(scores, "macro_score", zero_f64_json),
        "innovation_score": field_or(scores, "innovation_score", zero_f64_json),
        "novelty_score": field_or(scores, "novelty_score", zero_f64_json),
        "failure_penalty": field_or(scores, "failure_penalty", zero_f64_json),
        "final_score": field_or(scores, "final_score", zero_f64_json),
    })
}

pub(crate) fn metrics_point(
    run_id: &str,
    variant: &str,
    generation_id: &str,
    metric_name: &str,
    metric_value: f64,
    series: &str,
    stage_id: Option<&str>,
    candidate_id: Option<&str>,
    extra: Value,
) -> Value {
    let mut record = json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "metrics_point",
        "run_id": run_id,
        "variant": variant,
        "generation_id": generation_id,
        "stage_id": stage_id,
        "candidate_id": candidate_id,
        "metric_name": metric_name,
        "metric_value": round6(metric_value),
        "series": series,
    });
    merge_object(&mut record, &extra);
    record
}

pub(crate) fn promotion_decision_record(
    run_id: &str,
    generation_id: &str,
    champion: &Value,
    promoted: &[Value],
    evidence_bundle_path: &Path,
    promotion_reason: PromotionReason,
    score_breakdown: Value,
) -> Value {
    json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "promotion_decision",
        "run_id": run_id,
        "generation_id": generation_id,
        "candidate_id": field_or(champion, "candidate_id", empty_string_json),
        "promoted_candidate_ids": promoted.iter().filter_map(|candidate| candidate.get("candidate_id").and_then(Value::as_str)).map(ToString::to_string).collect::<Vec<_>>(),
        "champion_candidate_id": field_or(champion, "candidate_id", empty_string_json),
        "promotion_reason": promotion_reason.as_str(),
        "decision_basis": ["final_score", "novelty_score", "interface_score", "failure_understanding"],
        "score_breakdown": score_breakdown,
        "evidence_bundle_path": evidence_bundle_path.display().to_string(),
    })
}

pub(crate) fn lineage_edge_record(
    run_id: &str,
    generation_id: &str,
    parent_candidate_id: Value,
    child_candidate_id: Value,
    mutation_op: String,
    island: String,
) -> Value {
    let parent_candidate_id = match parent_candidate_id {
        Value::Null => json!("root"),
        value => value,
    };
    json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "lineage_edge",
        "run_id": run_id,
        "generation_id": generation_id,
        "parent_candidate_id": parent_candidate_id,
        "child_candidate_id": child_candidate_id,
        "mutation_op": mutation_op,
        "island": island,
    })
}

pub(crate) fn constant_score_block(value: f64) -> Value {
    let rounded = round6(value);
    json!({
        "local_score": rounded,
        "interface_score": rounded,
        "macro_score": rounded,
        "innovation_score": rounded,
        "novelty_score": rounded,
        "failure_penalty": 0.0,
        "final_score": rounded,
    })
}

pub(crate) fn judge_block(
    family: &str,
    provenance: &str,
    route_tier: &str,
    prompt: usize,
    completion: usize,
) -> Value {
    let total = prompt + completion;
    json!({
        "family": family,
        "provenance": provenance,
        "route_tier": route_tier,
        "token_usage": {
            "prompt": prompt,
            "completion": completion,
            "total": total,
        }
    })
}
