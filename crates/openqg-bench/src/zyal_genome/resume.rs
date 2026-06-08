use super::*;

pub(crate) fn empty_resume_state(stage_registry: &[StagePackage]) -> ResumeState {
    ResumeState {
        completed_generation: 0,
        stage_ledgers: Vec::new(),
        event_count: 0,
        generation_scores: Vec::new(),
        stage_score_history: stage_registry
            .iter()
            .map(|stage| (stage.stage_id.clone(), Vec::new()))
            .collect(),
        router_state: "nominal".to_string(),
        degraded_router: false,
        previous_generation_ids: Vec::new(),
    }
}

pub(crate) fn load_resume_state(
    run_dir: &Path,
    stage_registry: &[StagePackage],
) -> Result<ResumeState> {
    let mut state = empty_resume_state(stage_registry);
    state.completed_generation = read_completed_generation(run_dir)?;
    state.stage_ledgers =
        read_jsonl::<Value>(&run_dir.join("stage-ledger.jsonl")).unwrap_or_else(|_| Vec::new());
    state.event_count = read_jsonl::<Value>(&run_dir.join("run-events.jsonl"))
        .unwrap_or_else(|_| Vec::new())
        .len();
    state.generation_scores = read_generation_scores(&run_dir.join("generation-ledger.jsonl"));
    if state.generation_scores.is_empty() {
        state.generation_scores = generation_scores_from_events(&run_dir.join("run-events.jsonl"));
    }
    state.stage_score_history =
        stage_score_history_from_ledgers(stage_registry, &state.stage_ledgers);
    state.degraded_router = state.stage_ledgers.iter().any(|entry| {
        entry
            .get("failure_modes")
            .and_then(Value::as_array)
            .map(|modes| {
                modes
                    .iter()
                    .any(|mode| mode.as_str() == Some("degraded_router"))
            })
            .unwrap_or(false)
    });
    state.router_state = if state.degraded_router {
        "degraded_router".to_string()
    } else {
        latest_router_state(&run_dir.join("run-events.jsonl"))
    };
    state.previous_generation_ids =
        latest_population_candidates(&run_dir.join("population-ledger.jsonl"));
    Ok(state)
}

pub(crate) fn read_completed_generation(run_dir: &Path) -> Result<usize> {
    let checkpoint = run_dir.join("checkpoint.json");
    if checkpoint.exists() {
        let value: Value = serde_json::from_str(&fs::read_to_string(&checkpoint)?)
            .with_context(|| format!("parse {}", checkpoint.display()))?;
        return Ok(value
            .get("complete_generation")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize);
    }
    let mut generation_ids = Vec::new();
    for record in
        read_jsonl::<Value>(&run_dir.join("generation-ledger.jsonl")).unwrap_or_else(|_| Vec::new())
    {
        if record.get("metric_name").and_then(Value::as_str) == Some("deterministic_rollup_score") {
            if let Some(generation_id) = record.get("generation_id").and_then(Value::as_str) {
                generation_ids.push(generation_index_from_id(generation_id));
            }
        }
    }
    if !generation_ids.is_empty() {
        return Ok(*generation_ids.iter().max().unwrap());
    }
    for record in
        read_jsonl::<Value>(&run_dir.join("run-events.jsonl")).unwrap_or_else(|_| Vec::new())
    {
        if record.get("event_type").and_then(Value::as_str) == Some("generation_end") {
            if let Some(generation_id) = record.get("generation_id").and_then(Value::as_str) {
                generation_ids.push(generation_index_from_id(generation_id));
            }
        }
    }
    Ok(*generation_ids.iter().max().unwrap_or(&0))
}

pub(crate) fn generation_index_from_id(generation_id: &str) -> usize {
    generation_id
        .strip_prefix('g')
        .and_then(|suffix| suffix.parse::<usize>().ok())
        .unwrap_or(0)
}

pub(crate) fn read_generation_scores(path: &Path) -> Vec<f64> {
    let records = read_jsonl::<Value>(path).unwrap_or_else(|_| Vec::new());
    hybrid_quality_rollup_series(&records)
}

pub(crate) fn generation_scores_from_events(path: &Path) -> Vec<f64> {
    let mut records = read_jsonl::<Value>(path).unwrap_or_else(|_| Vec::new());
    records.sort_by_key(|record| {
        generation_index_from_id(
            record
                .get("generation_id")
                .and_then(Value::as_str)
                .unwrap_or("g0000"),
        )
    });
    records
        .into_iter()
        .filter(|record| record.get("event_type").and_then(Value::as_str) == Some("generation_end"))
        .filter_map(|record| record.get("final_score").and_then(Value::as_f64))
        .collect()
}

pub(crate) fn stage_score_history_from_ledgers(
    stage_registry: &[StagePackage],
    stage_ledgers: &[Value],
) -> BTreeMap<String, Vec<f64>> {
    let mut history: BTreeMap<String, Vec<f64>> = stage_registry
        .iter()
        .map(|stage| (stage.stage_id.clone(), Vec::new()))
        .collect();
    for entry in stage_ledgers {
        if let Some(stage_id) = entry.get("stage_id").and_then(Value::as_str) {
            history.entry(stage_id.to_string()).or_default().push(
                entry
                    .get("final_score")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0),
            );
        }
    }
    history
}

pub(crate) fn latest_router_state(path: &Path) -> String {
    let mut state = "nominal".to_string();
    for record in read_jsonl::<Value>(path).unwrap_or_else(|_| Vec::new()) {
        if let Some(router_state) = record.get("router_state").and_then(Value::as_str) {
            state = router_state.to_string();
        }
    }
    state
}

pub(crate) fn latest_population_candidates(path: &Path) -> Vec<String> {
    let mut candidates = Vec::new();
    for record in read_jsonl::<Value>(path).unwrap_or_else(|_| Vec::new()) {
        if let Some(candidate_id) = record.get("candidate_id").and_then(Value::as_str) {
            candidates.push(candidate_id.to_string());
        }
    }
    candidates
}
