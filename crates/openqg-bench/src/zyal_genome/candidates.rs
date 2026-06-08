use super::*;

pub(crate) fn candidate_parent_ids(
    previous_generation_ids: &[String],
    stage_index: usize,
) -> Vec<String> {
    if previous_generation_ids.is_empty() {
        return Vec::new();
    }
    vec![previous_generation_ids[stage_index % previous_generation_ids.len()].clone()]
}

pub(crate) fn ensure_present(record: &Value, field: &str) -> Result<()> {
    match record.get(field) {
        Some(Value::Null) | None => bail!("missing field: {field}"),
        Some(_) => Ok(()),
    }
}

pub(crate) fn adaptive_pressure_state(
    generation_scores: &[f64],
    previous_generation: &[Value],
    island_names: &[String],
) -> Value {
    let recent = median(&generation_scores[generation_scores.len().saturating_sub(3)..]);
    let previous_best = previous_generation
        .iter()
        .filter_map(|candidate| {
            candidate
                .get("scores")
                .and_then(|scores| scores.get("final_score"))
                .and_then(Value::as_f64)
        })
        .fold(0.0, f64::max);
    let island_count = island_names.len().max(1) as f64;
    json!({
        "recent_score_median": round6(recent),
        "previous_generation_best": round6(previous_best),
        "pressure_index": round6((recent + previous_best + island_names.len() as f64) / (island_count + 2.0)),
        "island_count": island_names.len(),
        "previous_generation_size": previous_generation.len(),
    })
}

pub(crate) fn choose_parent_ids(
    previous_generation: &[Value],
    mode: &str,
    island: &str,
    generation_index: usize,
    candidate_index: usize,
    seed: u64,
) -> Vec<String> {
    if previous_generation.is_empty() {
        return Vec::new();
    }
    let mut indices =
        vec![(generation_index + candidate_index + seed as usize) % previous_generation.len()];
    if previous_generation.len() > 1 && (mode == "exploitation" || island.contains("repair")) {
        indices.push(
            (generation_index + candidate_index + seed as usize + 1) % previous_generation.len(),
        );
    } else if previous_generation.len() > 2 && mode == "wildcard" {
        indices.push(
            (generation_index + candidate_index + seed as usize + 2) % previous_generation.len(),
        );
    }
    indices
        .into_iter()
        .filter_map(|index| {
            previous_generation
                .get(index)
                .and_then(|candidate| candidate.get("candidate_id"))
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .collect()
}

pub(crate) fn choose_stage_concepts(
    stage_registry: &[StagePackage],
    concepts: &[Value],
    generation_index: usize,
    candidate_index: usize,
    seed: u64,
) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    if concepts.is_empty() {
        return map;
    }
    for (index, stage) in stage_registry.iter().enumerate() {
        let pick = (index + generation_index + candidate_index + seed as usize) % concepts.len();
        let concept_id = concepts[pick]
            .get("concept_id")
            .and_then(Value::as_str)
            .unwrap_or(&stage.stage_id)
            .to_string();
        map.insert(stage.stage_id.clone(), concept_id);
    }
    map
}

pub(crate) fn choose_source_cards(
    accepted_cards: &[Value],
    fresh_cards: &[Value],
    mode: &str,
    generation_index: usize,
    candidate_index: usize,
    seed: u64,
) -> Vec<Value> {
    let mut pool = if fresh_cards.is_empty() {
        accepted_cards.to_vec()
    } else {
        fresh_cards.to_vec()
    };
    if pool.is_empty() {
        return Vec::new();
    }
    let start = (generation_index + candidate_index + mode.len() + seed as usize) % pool.len();
    pool.rotate_left(start);
    pool.into_iter().take(3).collect()
}

pub(crate) fn expected_candidate_failures(
    mode: &str,
    mutation_op: &str,
    router_state: &str,
) -> Vec<String> {
    let mut failures = Vec::new();
    if router_state != "nominal" {
        failures.push("degraded_router".to_string());
    }
    if mode == "wildcard" {
        failures.push("wildcard_risk".to_string());
    }
    if mutation_op.contains("repair") {
        failures.push("repair_drift".to_string());
    }
    failures
}

pub(crate) fn compute_candidate_scores(
    mode: &str,
    island: &str,
    mutation_op: &str,
    stage_concepts: &BTreeMap<String, String>,
    generation_index: usize,
    _generation_count: usize,
    candidate_index: usize,
    seed: u64,
    degraded_router: bool,
    degraded_router_penalty: f64,
    novelty_weight: f64,
) -> Value {
    let mode_factor = match mode {
        "exploitation" => 0.62,
        "novelty" => 0.55,
        _ => 0.48,
    };
    let island_factor = 0.03 * (island.len() as f64 % 5.0);
    let mutation_factor = 0.01 * (mutation_op.len() as f64 % 7.0);
    let stage_factor = 0.01 * stage_concepts.len() as f64;
    let jitter = hash_unit(&format!(
        "{mode}:{island}:{mutation_op}:{generation_index}:{candidate_index}:{seed}"
    ));
    let novelty = clamp(0.25 + 0.40 * novelty_weight + 0.15 * jitter, 0.0, 1.0);
    let generation_drift = 0.015 * (generation_index.saturating_sub(1).min(50) as f64 / 50.0);
    let final_score = clamp(
        mode_factor
            + island_factor
            + mutation_factor
            + stage_factor
            + 0.10 * jitter
            + novelty_weight * novelty
            + generation_drift
            - if degraded_router {
                degraded_router_penalty
            } else {
                0.0
            },
        0.0,
        1.0,
    );
    let interface = clamp(
        0.45 + 0.1 * jitter
            + if mutation_op.contains("contract") {
                0.08
            } else {
                0.0
            },
        0.0,
        1.0,
    );
    let macro_score = clamp(
        0.50 + 0.05 * stage_concepts.len() as f64 + 0.08 * jitter,
        0.0,
        1.0,
    );
    let innovation = clamp(0.35 + 0.22 * novelty + 0.03 * jitter, 0.0, 1.0);
    let failure_penalty = clamp(
        if degraded_router {
            degraded_router_penalty
        } else {
            0.04
        } + if mode == "wildcard" { 0.02 } else { 0.0 },
        0.0,
        0.35,
    );
    let final_score = apply_saturation_guard(
        final_score,
        &[interface, macro_score, innovation, novelty],
        failure_penalty,
        !degraded_router,
    );
    json!({
        "local_score": round6(clamp(0.45 + 0.10 * jitter, 0.0, 1.0)),
        "interface_score": round6(interface),
        "macro_score": round6(macro_score),
        "innovation_score": round6(innovation),
        "novelty_score": round6(novelty),
        "failure_penalty": round6(failure_penalty),
        "final_score": round6(final_score),
        "prompt_tokens": 640 + 12 * candidate_index + 8 * mutation_op.len(),
        "completion_tokens": 240 + 8 * candidate_index + 4 * island.len(),
        "time_seconds": round6(1.0 + 0.05 * candidate_index as f64 + if degraded_router { 0.2 } else { 0.1 }),
        "pass_rate": if final_score >= 0.45 { 1.0 } else { 0.0 },
        "failure_modes": expected_candidate_failures(mode, mutation_op, if degraded_router { "degraded_router" } else { "nominal" }),
        "failure_understanding": round6((1.0 - failure_penalty).clamp(0.0, 1.0)),
        "scoring_weights": json!({
            "local_score": 0.24,
            "interface_score": 0.16,
            "macro_score": 0.24,
            "innovation_score": 0.18,
            "novelty_score": novelty_weight,
            "failure_penalty": 0.05,
        }),
    })
}

pub(crate) fn align_candidate_score_with_stage_rollup(
    mut scores: Value,
    stage_rollup: Option<f64>,
    cap_enabled: bool,
) -> Value {
    if !cap_enabled {
        return scores;
    }
    let Some(stage_rollup) = stage_rollup else {
        return scores;
    };
    let ceiling = (stage_rollup + 0.12).min(0.995);
    if let Some(current) = scores.get("final_score").and_then(Value::as_f64) {
        if current > ceiling {
            scores["final_score"] = json!(round6(ceiling));
            scores["stage_rollup_score_ceiling"] = json!(round6(ceiling));
        }
    }
    scores
}

pub(crate) fn hybrid_candidate_record(
    generation_id: &str,
    stage: &StagePackage,
    candidate_id: &str,
    parent_ids: &[String],
    mode: &str,
    research_refs: &[String],
    stage_concepts: &BTreeMap<String, String>,
    route_policy: &Value,
    scores: &Value,
    score_breakdown: &Value,
) -> Value {
    let review = frontier_review_fields(
        candidate_id,
        mode,
        research_refs,
        stage_concepts,
        unwrap_or_value(
            scores
                .get("failure_modes")
                .and_then(Value::as_array)
                .cloned(),
            Vec::new(),
        ),
        scores,
    );
    json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "candidate",
        "candidate_id": candidate_id,
        "generation_id": generation_id,
        "island": stage.track,
        "mode": mode,
        "parent_candidate_ids": parent_ids,
        "lineage_depth": parent_ids.len() + 1,
        "mutation_ops": [stage.mutation_op],
        "source_card_ids": research_refs,
        "stage_concepts": stage_concepts,
        "route_policy": route_policy,
        "expected_failure_modes": field_or(scores, "failure_modes", empty_array_json),
        "adaptive_pressure": json!({}),
        "scoring_weights": field_or(scores, "scoring_weights", empty_object),
        "scores": scores,
        "score_breakdown": score_breakdown,
        "frontier_claim": review.frontier_claim,
        "falsifiable_tests": review.falsifiable_tests,
        "known_failure_modes": review.known_failure_modes,
        "review_priority": review.review_priority,
    })
}

pub(crate) fn frontier_review_fields(
    candidate_id: &str,
    mode: &str,
    source_card_ids: &[String],
    stage_concepts: &BTreeMap<String, String>,
    known_failure_modes: Vec<Value>,
    scores: &Value,
) -> FrontierReviewFields {
    let source_summary = if source_card_ids.is_empty() {
        "stage concepts".to_string()
    } else {
        source_card_ids
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    };
    let stage_targets = stage_concepts.keys().take(3).cloned().collect::<Vec<_>>();
    let mut falsifiable_tests = stage_targets
        .iter()
        .map(|stage_id| format!("Replay {stage_id} evidence with the proposed interface unchanged"))
        .collect::<Vec<_>>();
    if falsifiable_tests.is_empty() {
        falsifiable_tests
            .push("Replay the stage-local evidence bundle without target leakage".to_string());
    }
    let final_score = scores
        .get("final_score")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let novelty_score = scores
        .get("novelty_score")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let review_priority = if mode == "novelty" && final_score >= NOVELTY_CHAMPION_SCORE_FLOOR {
        "high"
    } else if final_score >= 0.80 || novelty_score >= 0.60 {
        "medium"
    } else {
        "low"
    }
    .to_string();
    FrontierReviewFields {
        frontier_claim: format!(
            "{candidate_id} proposes a {mode} frontier step grounded in {source_summary}"
        ),
        falsifiable_tests,
        known_failure_modes,
        review_priority,
    }
}
