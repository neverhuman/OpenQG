use super::*;

pub(crate) fn read_required_json(path: &Path, checks: &mut Vec<Value>) -> Value {
    match fs::read_to_string(path)
        .with_context(|| format!("read {}", path.display()))
        .and_then(|text| {
            serde_json::from_str::<Value>(&text)
                .with_context(|| format!("parse {}", path.display()))
        }) {
        Ok(value) => {
            let valid = validate_record(&value).is_ok()
                || path.file_name().and_then(|name| name.to_str()) == Some("checkpoint.json");
            checks
                .push(json!({"path": path.display().to_string(), "passed": valid, "kind": "json"}));
            value
        }
        Err(err) => {
            checks.push(json!({"path": path.display().to_string(), "passed": false, "kind": "json", "error": err.to_string()}));
            json!({})
        }
    }
}

pub(crate) fn read_optional_json(path: &Path) -> Value {
    value_or(
        fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok()),
        empty_object,
    )
}

pub(crate) fn read_required_jsonl(path: &Path, checks: &mut Vec<Value>) -> Vec<Value> {
    match read_jsonl::<Value>(path) {
        Ok(records) => {
            let valid = records.iter().all(|record| validate_record(record).is_ok());
            checks.push(json!({"path": path.display().to_string(), "passed": valid, "kind": "jsonl", "records": records.len()}));
            records
        }
        Err(err) => {
            checks.push(json!({"path": path.display().to_string(), "passed": false, "kind": "jsonl", "error": err.to_string()}));
            Vec::new()
        }
    }
}

pub(crate) fn read_optional_jsonl(path: &Path) -> Vec<Value> {
    ok_or_value(read_jsonl::<Value>(path), Vec::new())
}

pub(crate) fn add_check(
    checks: &mut Vec<Value>,
    name: &str,
    passed: bool,
    observed: Value,
    threshold: Value,
) {
    checks.push(json!({
        "name": name,
        "passed": passed,
        "observed": observed,
        "threshold": threshold,
    }));
}

pub(crate) fn timeout_overrun_count(record: &Value) -> usize {
    if let Some(attempts) = record.get("attempts").and_then(Value::as_array) {
        return attempts
            .iter()
            .filter(|attempt| {
                let elapsed = attempt
                    .get("elapsed_seconds")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                let timeout = attempt
                    .get("timeout_seconds")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                timeout > 0.0 && elapsed > timeout + 5.0
            })
            .count();
    }
    let elapsed = record
        .get("elapsed_seconds")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let timeout = or_alt(
        record.get("configured_timeout_seconds"),
        record.get("timeout_seconds"),
    )
    .and_then(Value::as_f64)
    .unwrap_or(0.0);
    usize::from(timeout > 0.0 && elapsed > timeout + 5.0)
}

pub(crate) fn degraded_route_count_from_ledgers(
    stage_ledgers: &[Value],
    run_events: &[Value],
) -> usize {
    if !stage_ledgers.is_empty() {
        return stage_ledgers
            .iter()
            .filter(|entry| {
                entry.get("router_state").and_then(Value::as_str) == Some("degraded_router")
            })
            .count();
    }
    let mut seen = BTreeSet::new();
    for event in run_events {
        if event.get("event_type").and_then(Value::as_str) == Some("router_decision")
            && event.get("router_state").and_then(Value::as_str) == Some("degraded_router")
        {
            let key = format!(
                "{}:{}",
                event
                    .get("generation_id")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                event.get("stage_id").and_then(Value::as_str).unwrap_or("")
            );
            seen.insert(key);
        }
    }
    seen.len()
}

pub(crate) fn metric_values(records: &[Value], metric_name: &str) -> Vec<f64> {
    let mut values = records
        .iter()
        .filter(|record| record.get("metric_name").and_then(Value::as_str) == Some(metric_name))
        .filter_map(|record| {
            Some((
                generation_index_from_id(
                    record
                        .get("generation_id")
                        .and_then(Value::as_str)
                        .unwrap_or("g0000"),
                ),
                record.get("metric_value").and_then(Value::as_f64)?,
            ))
        })
        .collect::<Vec<_>>();
    values.sort_by_key(|(generation, _)| *generation);
    values.into_iter().map(|(_, value)| value).collect()
}

pub(crate) fn metric_values_by_generation(
    records: &[Value],
    metric_name: &str,
) -> BTreeMap<usize, f64> {
    let mut values = BTreeMap::new();
    for record in records
        .iter()
        .filter(|record| record.get("metric_name").and_then(Value::as_str) == Some(metric_name))
    {
        let Some(value) = record.get("metric_value").and_then(Value::as_f64) else {
            continue;
        };
        let generation = generation_index_from_id(
            record
                .get("generation_id")
                .and_then(Value::as_str)
                .unwrap_or("g0000"),
        );
        values.insert(generation, value);
    }
    values
}

pub(crate) fn hybrid_quality_rollup_series(records: &[Value]) -> Vec<f64> {
    let deterministic = metric_values_by_generation(records, "deterministic_rollup_score");
    let champions = metric_values_by_generation(records, "hybrid_champion_score");
    let generations = deterministic
        .keys()
        .chain(champions.keys())
        .copied()
        .collect::<BTreeSet<_>>();
    generations
        .into_iter()
        .filter_map(|generation| {
            or_alt(champions.get(&generation), deterministic.get(&generation)).copied()
        })
        .collect()
}

pub(crate) fn champion_scores(summary: &Value, generation_records: &[Value]) -> Vec<f64> {
    let from_summary = unwrap_or_value(
        summary
            .get("generation_champions")
            .and_then(Value::as_array)
            .map(|champions| {
                champions
                    .iter()
                    .filter_map(|champion| champion.get("final_score").and_then(Value::as_f64))
                    .collect::<Vec<_>>()
            }),
        Vec::new(),
    );
    if from_summary.is_empty() {
        metric_values(generation_records, "hybrid_champion_score")
    } else {
        from_summary
    }
}

pub(crate) fn regression_rate(scores: &[f64]) -> f64 {
    let deltas = scores
        .windows(2)
        .map(|window| window[1] - window[0])
        .collect::<Vec<_>>();
    if deltas.is_empty() {
        0.0
    } else {
        deltas
            .iter()
            .filter(|delta| **delta < -QUALITY_REGRESSION_TOLERANCE)
            .count() as f64
            / deltas.len() as f64
    }
}

pub(crate) fn is_hard_stage_repair_live_call(record: &Value) -> bool {
    record.get("purpose").and_then(Value::as_str) == Some("hard_stage_repair")
}

pub(crate) fn jailgun_live_call_has_proof(record: &Value) -> bool {
    is_hard_stage_repair_live_call(record)
        && record.get("route_backend").and_then(Value::as_str) == Some("jailgun")
        && record.get("execution_backend").and_then(Value::as_str) == Some("jailgun_mcp")
        && record
            .get("jailgun_run_id")
            .and_then(Value::as_str)
            .map(|run_id| !run_id.trim().is_empty())
            .unwrap_or(false)
        && record.get("status").and_then(Value::as_str) == Some("ok")
        && unwrap_or_value(
            record
                .get("jailgun_summary_status")
                .and_then(Value::as_str)
                .map(|status| status == "succeeded"),
            record.get("jailgun_summary").is_some(),
        )
}

pub(crate) fn count_string_field(records: &[Value], field: &str) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for record in records {
        if let Some(value) = record.get(field).and_then(Value::as_str) {
            *counts.entry(value.to_string()).or_default() += 1;
        }
    }
    counts
}
