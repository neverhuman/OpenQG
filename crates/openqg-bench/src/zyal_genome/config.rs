use super::*;

pub(crate) fn resolve_generation_count(
    generations: Option<usize>,
    max_generations: Option<usize>,
    evaluation: &Value,
) -> Result<usize> {
    let count = or_alt(
        generations.or(max_generations),
        evaluation
            .get("max_generations_default")
            .and_then(Value::as_u64)
            .map(|v| v as usize),
    )
    .unwrap_or(1);
    if count < 1 {
        bail!("--max-generations/--generations must be at least 1");
    }
    Ok(count)
}

pub(crate) fn resolve_population_config(
    population_size: Option<usize>,
    islands: Option<usize>,
    novelty_weight: Option<f64>,
    new_info_refresh: Option<usize>,
    evaluation: &Value,
) -> Result<PopulationConfig> {
    let evolution = field_or(evaluation, "evolution", empty_object);
    let population_size = or_alt(
        population_size,
        evolution
            .get("population_size")
            .and_then(Value::as_u64)
            .map(|v| v as usize),
    )
    .unwrap_or(24);
    let islands = or_alt(
        islands,
        evolution
            .get("islands")
            .and_then(Value::as_u64)
            .map(|v| v as usize),
    )
    .unwrap_or(6);
    let novelty_weight = or_alt(
        novelty_weight,
        evolution.get("novelty_weight").and_then(Value::as_f64),
    )
    .unwrap_or(0.13);
    let new_info_refresh = or_alt(
        new_info_refresh,
        evolution
            .get("new_info_refresh")
            .and_then(Value::as_u64)
            .map(|v| v as usize),
    )
    .unwrap_or(4);
    if population_size < 1 {
        bail!("--population-size must be at least 1");
    }
    if islands < 1 {
        bail!("--islands must be at least 1");
    }
    if new_info_refresh < 1 {
        bail!("--new-info-refresh must be at least 1");
    }
    let mut island_names = unwrap_or_value(
        evolution
            .get("island_names")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
            }),
        DEFAULT_ISLANDS.iter().map(|s| s.to_string()).collect(),
    );
    while island_names.len() < islands {
        island_names.push(format!("island-{}", island_names.len() + 1));
    }
    island_names.truncate(islands);
    Ok(PopulationConfig {
        population_size,
        islands,
        island_names,
        new_info_refresh,
        novelty_weight,
        diversity_targets: unwrap_or_value(
            evolution.get("diversity_targets").cloned(),
            json!({
                "concept_entropy_min": 2.2,
                "island_balance_min": 0.70,
                "source_diversity_min": 0.35
            }),
        ),
        promotion_gates: unwrap_or_value(
            evolution
                .get("promotion_gates")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .map(ToString::to_string)
                        .collect()
                }),
            vec![
                "final_score".to_string(),
                "novelty_score".to_string(),
                "interface_score".to_string(),
                "failure_understanding".to_string(),
            ],
        ),
        degraded_penalties: field_or(&evolution, "degraded_penalties", default_degraded_penalties),
    })
}

pub(crate) fn resolve_live_config(live_selective: bool, runbook: &Value) -> LiveConfig {
    let evaluation = field_or(runbook, "evaluation", empty_object);
    let merged = deep_merge_values(
        &field_or(&evaluation, "live", empty_object),
        &field_or(runbook, "live", empty_object),
    );
    let enabled = live_selective
        || merged
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        || evaluation
            .get("live_selective")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    LiveConfig {
        enabled,
        timeout_seconds: merged
            .get("timeout_seconds")
            .and_then(Value::as_u64)
            .unwrap_or(45),
        timeout_by_purpose: resolve_live_timeouts(&merged),
        retry_count: merged
            .get("retry_count")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize,
        champion_audit_every: merged
            .get("champion_audit_every")
            .and_then(Value::as_u64)
            .unwrap_or(25) as usize,
        hard_stage_every: merged
            .get("hard_stage_every")
            .and_then(Value::as_u64)
            .unwrap_or(10) as usize,
        promotion_every: merged
            .get("promotion_every")
            .and_then(Value::as_u64)
            .unwrap_or(10) as usize,
        research_synthesis: merged
            .get("research_synthesis")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        hard_stage_repair: merged
            .get("hard_stage_repair")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        promotion_judging: merged
            .get("promotion_judging")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        command: unwrap_or_value(
            merged
                .get("command")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .map(ToString::to_string)
                        .collect()
                }),
            JEKKO_LIVE_COMMAND.iter().map(|s| s.to_string()).collect(),
        ),
    }
}

pub(crate) fn resolve_live_timeouts(merged: &Value) -> BTreeMap<String, u64> {
    let default = merged
        .get("timeout_seconds")
        .and_then(Value::as_u64)
        .unwrap_or(45);
    let mut timeouts = BTreeMap::from([
        ("hard_stage_repair".to_string(), default),
        ("promotion_judging".to_string(), default),
        ("champion_audit".to_string(), default),
        ("research_synthesis".to_string(), default),
    ]);
    for key in [
        "timeouts",
        "timeout_seconds_by_purpose",
        "per_purpose_timeouts",
    ] {
        if let Some(map) = merged.get(key).and_then(Value::as_object) {
            for (purpose, value) in map {
                if let Some(seconds) = value.as_u64() {
                    timeouts.insert(purpose.clone(), seconds);
                }
            }
        }
    }
    timeouts
}

pub(crate) fn resolve_checkpoint_every(
    checkpoint_every: Option<usize>,
    evaluation: &Value,
) -> Result<usize> {
    let value = or_alt(
        checkpoint_every,
        evaluation
            .get("checkpoint_every")
            .and_then(Value::as_u64)
            .map(|v| v as usize),
    )
    .unwrap_or(0);
    Ok(value)
}

pub(crate) fn resolve_research_cache_path(
    research_cache: Option<PathBuf>,
    runbook: &Value,
) -> Option<PathBuf> {
    if research_cache.is_some() {
        return research_cache;
    }
    let configured = or_alt(
        runbook
            .get("evaluation")
            .and_then(|evaluation| evaluation.get("research_cache"))
            .and_then(Value::as_str),
        runbook
            .get("context")
            .and_then(|context| context.get("research_cache"))
            .and_then(Value::as_str),
    );
    if let Some(configured) = configured {
        return Some(PathBuf::from(configured));
    }
    let default = PathBuf::from(DEFAULT_RESEARCH_CACHE);
    if default.exists() {
        Some(default)
    } else {
        None
    }
}
