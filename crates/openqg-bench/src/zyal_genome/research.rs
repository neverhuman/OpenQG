use super::*;

pub(crate) fn write_memory_cards(
    writer: &mut JsonlWriter,
    stage_registry: &[StagePackage],
    run_id: &str,
) -> Result<()> {
    for stage in stage_registry {
        let record = json!({
            "schema_version": SCHEMA_VERSION,
            "record_kind": "memory_card",
            "run_id": run_id,
            "memory_card_id": format!("mem-{}-{}", stage.stage_id, short_hash(&stage.prompt_hash, 12)),
            "stage_id": stage.stage_id,
            "memory_refs": memory_refs_for_stage(stage),
            "content_hash": stage.prompt_hash,
            "summary": stage.purpose,
            "source_path": stage.memory_path.display().to_string(),
        });
        writer.write(&record)?;
    }
    Ok(())
}

pub(crate) fn write_research_cards(
    writer: &mut JsonlWriter,
    research_cards: &[Value],
) -> Result<()> {
    for card in research_cards {
        writer.write(card)?;
    }
    Ok(())
}

pub(crate) fn load_research_cache_cards(path: Option<&Path>, run_id: &str) -> Result<Vec<Value>> {
    if let Some(path) = path {
        if !path.exists() {
            return Ok(Vec::new());
        }
        let mut entries = Vec::new();
        if path.is_dir() {
            let mut files = Vec::new();
            for entry in WalkDir::new(path).min_depth(1) {
                let entry = entry?;
                if entry.file_type().is_file() {
                    let ext = entry
                        .path()
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .unwrap_or("");
                    if matches!(ext, "json" | "jsonl" | "yml" | "yaml") {
                        files.push(entry.into_path());
                    }
                }
            }
            files.sort();
            for source_path in files {
                entries.extend(load_research_cache_entries(&source_path)?);
            }
        } else {
            entries.extend(load_research_cache_entries(path)?);
        }
        let mut cards = Vec::new();
        for (index, entry) in entries.into_iter().enumerate() {
            let source_text = format!(
                "{} {} {} {} {} {}",
                string_or_default(&entry, "url"),
                string_or_default(&entry, "title"),
                string_or_default(&entry, "date"),
                string_or_default(&entry, "citation"),
                string_or_default(&entry, "summary"),
                string_or_default(&entry, "claim"),
            );
            let rejection_reason = reject_information_text(
                string_or_default(&entry, "url").as_str(),
                source_text.as_str(),
            );
            let source_hash = unwrap_or_value(
                entry
                    .get("hash")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
                stable_hash(&source_text),
            );
            cards.push(json!({
                "schema_version": SCHEMA_VERSION,
                "record_kind": "research_card",
                "run_id": run_id,
                "research_card_id": unwrap_or_value(entry.get("research_card_id").and_then(Value::as_str).map(ToString::to_string), format!("research-{}-{index:03}", short_hash(&source_hash, 12))),
                "source_type": string_or_default(&entry, "source_type"),
                "url": string_or_default(&entry, "url"),
                "title": entry.get("title").and_then(Value::as_str).unwrap_or("source"),
                "date": string_or_default(&entry, "date"),
                "source_hash": source_hash,
                "citation": or_alt(entry.get("citation").and_then(Value::as_str), entry.get("title").and_then(Value::as_str)).unwrap_or("source"),
                "summary": or_alt(entry.get("summary").and_then(Value::as_str), entry.get("claim").and_then(Value::as_str)).unwrap_or("").chars().take(1000).collect::<String>(),
                "cache_path": path.display().to_string(),
                "accepted": rejection_reason.is_none(),
                "rejection_reason": rejection_reason,
            }));
        }
        return Ok(cards);
    }
    Ok(Vec::new())
}

pub(crate) fn load_research_cache_entries(path: &Path) -> Result<Vec<Value>> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    if path.extension().and_then(|ext| ext.to_str()) == Some("jsonl") {
        Ok(text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| value_or(serde_json::from_str::<Value>(line).ok(), empty_object))
            .collect())
    } else if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
        let value: Value =
            serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
        Ok(match value {
            Value::Array(items) => items
                .into_iter()
                .filter(|entry| entry.is_object())
                .collect(),
            Value::Object(map) => unwrap_or_value(
                or_alt(
                    or_alt(
                        map.get("cards").and_then(Value::as_array).cloned(),
                        map.get("sources").and_then(Value::as_array).cloned(),
                    ),
                    map.get("research").and_then(Value::as_array).cloned(),
                ),
                vec![Value::Object(map.clone())],
            ),
            _ => Vec::new(),
        })
    } else {
        let value: YamlValue =
            serde_yaml::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
        let value = serde_json::to_value(value)?;
        Ok(match value {
            Value::Array(items) => items
                .into_iter()
                .filter(|entry| entry.is_object())
                .collect(),
            Value::Object(map) => unwrap_or_value(
                or_alt(
                    or_alt(
                        map.get("cards").and_then(Value::as_array).cloned(),
                        map.get("sources").and_then(Value::as_array).cloned(),
                    ),
                    map.get("research").and_then(Value::as_array).cloned(),
                ),
                vec![Value::Object(map.clone())],
            ),
            _ => Vec::new(),
        })
    }
}

pub(crate) fn synthesize_information_cards(
    stage_registry: &[StagePackage],
    run_id: &str,
) -> Vec<Value> {
    stage_registry
        .iter()
        .map(|stage| {
            let summary = if stage.purpose.is_empty() {
                stage.name.clone()
            } else {
                stage.purpose.clone()
            };
            json!({
                "schema_version": SCHEMA_VERSION,
                "record_kind": "information_card",
                "run_id": run_id,
                "information_card_id": format!("info-{}", short_hash(&stage.prompt_hash, 12)),
                "source_path": stage.stage_file.display().to_string(),
                "domain": stage.family,
                "claim": first_sentence(&summary),
                "method": "stage_synthesis",
                "constraint": "cached stage metadata only",
                "failure_risk": "stage metadata may be incomplete",
                "stage_concept_hint": stage.stage_id,
                "provenance_hash": stage.prompt_hash,
                "novelty_terms": novelty_terms_from_text(&summary),
            })
        })
        .collect()
}

pub(crate) fn synthesize_additional_information_cards(
    stage_registry: &[StagePackage],
    generation_id: &str,
    run_id: &str,
    count: usize,
) -> Vec<Value> {
    stage_registry
        .iter()
        .take(count)
        .map(|stage| {
            json!({
                "schema_version": SCHEMA_VERSION,
                "record_kind": "information_card",
                "run_id": run_id,
                "information_card_id": format!("info-{generation_id}-{}", short_hash(&stage.prompt_hash, 12)),
                "source_path": stage.stage_file.display().to_string(),
                "domain": stage.family,
                "claim": first_sentence(&stage.purpose),
                "method": "cached_research_synthesis",
                "constraint": "cached research only; no uncached web state inside scoring",
                "failure_risk": "cached source may be outdated or too broad for the target stage",
                "stage_concept_hint": stage.stage_id,
                "provenance_hash": stage.prompt_hash,
                "novelty_terms": novelty_terms_from_text(&stage.purpose),
            })
        })
        .collect()
}

pub(crate) fn select_research_refs_for_stage(
    research_cards: &[Value],
    stage: &StagePackage,
) -> Vec<String> {
    let accepted: Vec<&Value> = research_cards
        .iter()
        .filter(|card| {
            card.get("accepted")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .collect();
    if accepted.is_empty() {
        return Vec::new();
    }
    let stage_text = format!(
        "{} {} {} {} {}",
        stage.stage_id, stage.track, stage.family, stage.purpose, stage.name
    )
    .to_lowercase();
    let mut matched = Vec::new();
    for card in &accepted {
        let haystack = format!(
            "{} {} {}",
            string_or_default(card, "title"),
            string_or_default(card, "summary"),
            string_or_default(card, "source_type"),
        )
        .to_lowercase();
        if haystack.contains(&stage_text) {
            if let Some(id) = card.get("research_card_id").and_then(Value::as_str) {
                matched.push(id.to_string());
            }
        }
    }
    if matched.is_empty() {
        accepted
            .into_iter()
            .filter_map(|card| {
                card.get("research_card_id")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
            })
            .take(3)
            .collect()
    } else {
        matched.into_iter().take(3).collect()
    }
}

pub(crate) fn select_information_sources(
    stage_registry: &[StagePackage],
    generation_index: usize,
    seed: u64,
    count: usize,
) -> Vec<PathBuf> {
    if stage_registry.is_empty() {
        return Vec::new();
    }
    let mut paths = Vec::new();
    for index in 0..count {
        let stage =
            &stage_registry[(generation_index + index + seed as usize) % stage_registry.len()];
        paths.push(stage.stage_file.clone());
    }
    paths
}

pub(crate) fn extract_information_card(
    source_path: &Path,
    generation_id: &str,
    run_id: &str,
) -> Option<Value> {
    let text = fs::read_to_string(source_path).ok()?;
    let prompt_hash = stable_hash(&text);
    let stage_id = source_path
        .parent()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("source");
    Some(json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "information_card",
        "run_id": run_id,
        "information_card_id": format!("info-{generation_id}-{}", short_hash(&prompt_hash, 12)),
        "source_path": source_path.display().to_string(),
        "domain": "sources",
        "claim": first_sentence(&text),
        "method": "file_synthesis",
        "constraint": "cached research only",
        "failure_risk": "source may be outdated or broad",
        "stage_concept_hint": stage_id,
        "provenance_hash": prompt_hash,
        "novelty_terms": novelty_terms_from_text(&text),
    }))
}

pub(crate) fn memory_refs_for_stage(stage: &StagePackage) -> Vec<String> {
    unwrap_or_value(
        or_alt(
            stage
                .memory
                .get("memory_refs")
                .and_then(Value::as_array)
                .cloned(),
            stage.memory.get("refs").and_then(Value::as_array).cloned(),
        ),
        Vec::new(),
    )
    .into_iter()
    .filter_map(|value| value.as_str().map(ToString::to_string))
    .collect()
}

pub(crate) fn novelty_terms_from_text(text: &str) -> Vec<String> {
    let mut terms = BTreeSet::new();
    for word in text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| word.len() >= 4)
    {
        terms.insert(word.to_lowercase());
    }
    terms.into_iter().collect()
}

pub(crate) fn reject_information_text(_source: &str, text: &str) -> Option<String> {
    let lowered = text.to_lowercase();
    if lowered.trim().is_empty() {
        return Some("empty_provenance".to_string());
    }
    for (pattern, label) in [
        ("fixture leakage", "fixture_leakage"),
        ("copied benchmark", "copied_benchmark_values"),
        ("empirical fitting", "empirical_fitting"),
        ("hidden free parameters", "hidden_free_parameters"),
        (
            "unsupported numeric prediction",
            "unsupported_numeric_prediction",
        ),
    ] {
        if lowered.contains(pattern) {
            return Some(label.to_string());
        }
    }
    None
}

pub(crate) fn concept_gene_from_card(card: &Value) -> Value {
    let source_hash = card
        .get("provenance_hash")
        .and_then(Value::as_str)
        .unwrap_or("selftest")
        .to_string();
    json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "concept_gene",
        "gene_id": format!("seed-{}", short_hash(&source_hash, 12)),
        "concept_id": card.get("stage_concept_hint").and_then(Value::as_str).unwrap_or("concept").to_string(),
        "family": card.get("domain").and_then(Value::as_str).unwrap_or("foundations").to_string(),
        "domain": card.get("domain").and_then(Value::as_str).unwrap_or("foundations").to_string(),
        "claim": first_sentence(card.get("claim").and_then(Value::as_str).unwrap_or("")),
        "method": "cached_research_synthesis",
        "constraint": "cached research only; no uncached web state inside scoring",
        "failure_risk": "cached source may be outdated or too broad for the target stage",
        "stage_concept_hint": card.get("stage_concept_hint").and_then(Value::as_str).unwrap_or("concept").to_string(),
        "source_card_ids": [field_or(card, "research_card_id", empty_string_json)],
        "source_path": field_or(card, "source_path", empty_string_json),
        "provenance_hash": source_hash,
        "novelty_terms": novelty_terms_from_text(card.get("summary").and_then(Value::as_str).unwrap_or("")),
    })
}

pub(crate) fn research_cards_from_cache(
    research_cards: &[Value],
    run_id: &str,
) -> Result<Vec<Value>> {
    Ok(research_cards
        .iter()
        .map(|card| {
            let mut card = card.clone();
            if card.get("record_kind").and_then(Value::as_str) != Some("research_card") {
                card["schema_version"] = json!(SCHEMA_VERSION);
                card["record_kind"] = json!("research_card");
                card["run_id"] = json!(run_id);
            }
            card
        })
        .collect())
}

pub(crate) fn build_genesis_card(stage: &StagePackage, run_id: &str) -> Value {
    json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "research_card",
        "run_id": run_id,
        "research_card_id": format!("research-{}", short_hash(&stage.prompt_hash, 12)),
        "source_type": "cached_stage",
        "url": "",
        "title": stage.name,
        "date": "",
        "source_hash": stage.prompt_hash,
        "citation": stage.name,
        "accepted": true,
        "rejection_reason": null,
    })
}
