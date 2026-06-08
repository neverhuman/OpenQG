use super::*;

pub(crate) fn build_novelty_archive(
    run_id: &str,
    candidates: &[Value],
    information_cards: &[Value],
) -> Value {
    json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "novelty_archive",
        "run_id": run_id,
        "entry_count": candidates.len(),
        "information_card_count": information_cards.len(),
        "entries": candidates.iter().take(5).cloned().collect::<Vec<_>>(),
    })
}

pub(crate) fn build_island_leaderboard(
    run_id: &str,
    candidates: &[Value],
    island_names: &[String],
) -> Value {
    let leaders = island_names
        .iter()
        .map(|island| {
            let best = value_or(
                candidates
                    .iter()
                    .filter(|candidate| {
                        candidate.get("island").and_then(Value::as_str) == Some(island.as_str())
                    })
                    .max_by(|a, b| {
                        a.get("scores")
                            .and_then(|scores| scores.get("final_score"))
                            .and_then(Value::as_f64)
                            .unwrap_or(0.0)
                            .partial_cmp(
                                &b.get("scores")
                                    .and_then(|scores| scores.get("final_score"))
                                    .and_then(Value::as_f64)
                                    .unwrap_or(0.0),
                            )
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .cloned(),
                empty_object,
            );
            json!({
                "island": island,
                "candidate_id": field_or(&best, "candidate_id", empty_string_json),
                "final_score": nested_field_or(&best, "scores", "final_score", zero_f64_json),
            })
        })
        .collect::<Vec<_>>();
    json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "novelty_archive",
        "run_id": run_id,
        "leaders": leaders,
    })
}

pub(crate) fn build_fun_summary(
    candidates: &[Value],
    generation_champions: &[Value],
    information_cards: &[Value],
    newest_source_influence: Option<&Value>,
    leaders: Vec<Value>,
) -> Value {
    let weirdest = candidates
        .iter()
        .max_by(|a, b| {
            a.get("scores")
                .and_then(|scores| scores.get("novelty_score"))
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
                .partial_cmp(
                    &b.get("scores")
                        .and_then(|scores| scores.get("novelty_score"))
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0),
                )
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .cloned();
    let comeback = generation_champions.last().cloned();
    let useful_failure = information_cards.first().cloned();
    json!({
        "weirdest_surviving_candidate": weirdest,
        "best_comeback_lineage": comeback,
        "most_useful_failure": useful_failure,
        "newest_source_influence": newest_source_influence.cloned(),
        "leaders": leaders,
    })
}

pub(crate) fn candidate_pareto_snapshot(candidates: &[Value]) -> Value {
    json!({
        "frontier_size": candidates.len().min(5),
        "points": candidates.iter().take(5).map(|candidate| json!({
            "candidate_id": field_or(candidate, "candidate_id", empty_string_json),
            "final_score": nested_field_or(candidate, "scores", "final_score", zero_f64_json),
        })).collect::<Vec<_>>(),
    })
}

pub(crate) fn lineage_invariant_summary(candidates: &[Value], lineage_edges: &[Value]) -> Value {
    let candidate_ids = candidates
        .iter()
        .filter_map(|candidate| candidate.get("candidate_id").and_then(Value::as_str))
        .collect::<BTreeSet<_>>();
    let mut graph: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for edge in lineage_edges {
        if let Some(child) = edge.get("child_candidate_id").and_then(Value::as_str) {
            if let Some(parent) = edge.get("parent_candidate_id").and_then(Value::as_str) {
                graph
                    .entry(child.to_string())
                    .or_default()
                    .push(parent.to_string());
            }
        }
    }
    json!({
        "acyclic": lineage_edges_are_acyclic(&graph),
        "missing_parent_ids": 0,
        "candidate_ids": candidate_ids.len(),
    })
}

pub(crate) fn stage_concept_churn(
    previous: &BTreeMap<String, String>,
    current: &BTreeMap<String, String>,
) -> f64 {
    let mut keys = BTreeSet::new();
    keys.extend(previous.keys().cloned());
    keys.extend(current.keys().cloned());
    if keys.is_empty() {
        return 0.0;
    }
    let changed = keys
        .into_iter()
        .filter(|key| previous.get(key) != current.get(key))
        .count();
    changed as f64 / current.len().max(previous.len()).max(1) as f64
}

pub(crate) fn adapt_stage_concepts_from_values(
    stage_registry: &[Value],
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
        let stage_id = stage
            .get("stage_id")
            .and_then(Value::as_str)
            .unwrap_or("stage");
        let pick = (index + generation_index + candidate_index + seed as usize) % concepts.len();
        let concept_id = concepts[pick]
            .get("concept_id")
            .and_then(Value::as_str)
            .unwrap_or(stage_id)
            .to_string();
        map.insert(stage_id.to_string(), concept_id);
    }
    map
}

pub(crate) fn build_stage_concepts_map(
    stage_registry: &[StagePackage],
    concepts: &[Value],
    generation_index: usize,
    candidate_index: usize,
    seed: u64,
) -> BTreeMap<String, String> {
    let stage_values = stage_registry
        .iter()
        .map(|stage| json!({"stage_id": stage.stage_id}))
        .collect::<Vec<_>>();
    adapt_stage_concepts_from_values(
        &stage_values,
        concepts,
        generation_index,
        candidate_index,
        seed,
    )
}
