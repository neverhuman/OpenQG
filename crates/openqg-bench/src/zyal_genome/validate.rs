use super::*;

pub fn validate(root: &Path, schema: &Path) -> Result<()> {
    let _ = fs::read_to_string(schema).with_context(|| format!("read {}", schema.display()))?;
    let artifact_paths = collect_artifact_files(root)?;
    if artifact_paths.is_empty() {
        bail!("no genome artifacts found under {}", root.display());
    }
    for path in &artifact_paths {
        if path.extension().and_then(|ext| ext.to_str()) == Some("jsonl") {
            for record in read_jsonl::<Value>(path)? {
                validate_record(&record).with_context(|| format!("validate {}", path.display()))?;
            }
        } else {
            let record: Value = serde_json::from_str(&fs::read_to_string(path)?)
                .with_context(|| format!("parse {}", path.display()))?;
            validate_record(&record).with_context(|| format!("validate {}", path.display()))?;
        }
    }
    validate_hybrid_invariants(root)?;
    println!(
        "validated {} genome artifact files under {}",
        artifact_paths.len(),
        root.display()
    );
    Ok(())
}

pub(crate) fn validate_record(record: &Value) -> Result<()> {
    let kind = record
        .get("record_kind")
        .and_then(Value::as_str)
        .context("missing record_kind")?;
    let known = [
        "run_event",
        "stage_ledger",
        "stage_summary",
        "run_summary",
        "offline_eval",
        "comparison",
        "concept_gene",
        "stage_concept",
        "population_snapshot",
        "novelty_archive",
        "lineage_edge",
        "information_card",
        "stage_variant",
        "live_call",
        "research_card",
        "memory_card",
        "promotion_decision",
        "metrics_point",
        "preflight",
        "quality_gate",
    ];
    if !known.contains(&kind) {
        bail!("unknown record_kind: {kind}");
    }
    ensure_present(record, "schema_version")?;
    match kind {
        "run_event" => {
            for field in [
                "event_type",
                "run_id",
                "variant",
                "generation_id",
                "stage_id",
                "candidate_id",
                "artifact_paths",
                "judge",
            ] {
                ensure_present(record, field)?;
            }
        }
        "stage_ledger" => {
            for field in [
                "run_id",
                "variant",
                "generation_id",
                "stage_id",
                "candidate_id",
                "stage_name",
                "route_backend",
                "route_tier",
                "router_state",
                "mutation_op",
                "parent_generation_id",
                "delta_score",
                "pass_rate",
                "time_seconds",
                "failure_modes",
                "artifact_paths",
            ] {
                ensure_present(record, field)?;
            }
        }
        "stage_summary" => {
            for field in [
                "run_id",
                "variant",
                "stage_id",
                "stage_name",
                "generations",
                "mean_final_score",
                "best_final_score",
                "pass_rate",
                "route_backends",
                "failure_modes",
                "mutation_ops",
            ] {
                ensure_present(record, field)?;
            }
        }
        "run_summary" => {
            for field in [
                "run_id",
                "variant",
                "generation_count",
                "stage_count",
                "candidate_count",
                "best_score_seen",
                "rolling_5_median",
                "best_nonregressive_delta",
                "unique_contributions",
                "decoy_failures",
                "regression_rate",
                "throughput",
                "fail_stop_rate",
                "score_blend",
                "router_state",
                "degraded_router",
                "lineage",
                "pareto_snapshot",
            ] {
                ensure_present(record, field)?;
            }
        }
        "offline_eval" => {
            for field in [
                "run_id",
                "variant",
                "scorecard",
                "stage_rankings",
                "warnings",
            ] {
                ensure_present(record, field)?;
            }
        }
        "comparison" => {
            for field in ["root", "variants", "ranking"] {
                ensure_present(record, field)?;
            }
        }
        "concept_gene" => {
            for field in [
                "gene_id",
                "concept_id",
                "family",
                "domain",
                "claim",
                "method",
                "constraint",
                "failure_risk",
                "stage_concept_hint",
                "source_card_ids",
                "source_path",
                "provenance_hash",
                "novelty_terms",
            ] {
                ensure_present(record, field)?;
            }
        }
        "stage_concept" => {
            for field in [
                "run_id",
                "generation_id",
                "stage_id",
                "candidate_id",
                "concept_id",
                "family",
                "source_card_ids",
                "mutation_op",
            ] {
                ensure_present(record, field)?;
            }
        }
        "population_snapshot" => {
            for field in [
                "run_id",
                "generation_id",
                "population_size",
                "islands",
                "mode_counts",
                "candidate_ids",
                "promoted_candidate_ids",
                "champion_candidate_id",
                "best_final_score",
                "diversity_metrics",
                "candidates",
            ] {
                ensure_present(record, field)?;
            }
        }
        "novelty_archive" => {
            for field in ["run_id", "entry_count", "information_card_count", "entries"] {
                ensure_present(record, field)?;
            }
        }
        "lineage_edge" => {
            for field in [
                "run_id",
                "generation_id",
                "parent_candidate_id",
                "child_candidate_id",
                "mutation_op",
                "island",
            ] {
                ensure_present(record, field)?;
            }
        }
        "information_card" => {
            for field in [
                "information_card_id",
                "source_path",
                "domain",
                "claim",
                "method",
                "constraint",
                "failure_risk",
                "stage_concept_hint",
                "provenance_hash",
                "novelty_terms",
            ] {
                ensure_present(record, field)?;
            }
        }
        "stage_variant" => {
            for field in [
                "run_id",
                "variant",
                "generation_id",
                "stage_id",
                "candidate_id",
                "stage_variant_id",
                "parent_stage_variant_ids",
                "algorithm_summary",
                "prompt_hash",
                "memory_refs",
                "research_refs",
                "evidence_bundle_path",
                "score_breakdown",
            ] {
                ensure_present(record, field)?;
            }
        }
        "live_call" => {
            for field in [
                "run_id",
                "generation_id",
                "stage_id",
                "candidate_id",
                "call_id",
                "purpose",
                "status",
                "prompt_path",
                "retrieval_packet_path",
                "raw_output_path",
                "parsed_summary_path",
                "receipt_path",
                "timeout_seconds",
                "command",
                "token_usage",
            ] {
                ensure_present(record, field)?;
            }
        }
        "research_card" => {
            for field in [
                "run_id",
                "research_card_id",
                "source_type",
                "url",
                "title",
                "date",
                "source_hash",
                "citation",
                "accepted",
            ] {
                ensure_present(record, field)?;
            }
        }
        "memory_card" => {
            for field in [
                "run_id",
                "memory_card_id",
                "stage_id",
                "memory_refs",
                "content_hash",
                "summary",
            ] {
                ensure_present(record, field)?;
            }
        }
        "promotion_decision" => {
            for field in [
                "run_id",
                "generation_id",
                "candidate_id",
                "promoted_candidate_ids",
                "champion_candidate_id",
                "decision_basis",
                "score_breakdown",
                "evidence_bundle_path",
            ] {
                ensure_present(record, field)?;
            }
        }
        "metrics_point" => {
            for field in [
                "run_id",
                "variant",
                "generation_id",
                "metric_name",
                "metric_value",
                "series",
            ] {
                ensure_present(record, field)?;
            }
        }
        "preflight" => {
            for field in [
                "run_id",
                "variant",
                "runbook_path",
                "run_dir",
                "status",
                "backend_health",
                "live_command",
                "timeout_config",
                "routing_decision",
            ] {
                ensure_present(record, field)?;
            }
        }
        "quality_gate" => {
            for field in [
                "run_id",
                "run_dir",
                "tier",
                "passed",
                "status",
                "metrics",
                "checks",
                "artifacts",
            ] {
                ensure_present(record, field)?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub(crate) fn collect_artifact_files(root: &Path) -> Result<Vec<PathBuf>> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    let allowed = BTreeSet::from([
        "run-events.jsonl",
        "stage-ledger.jsonl",
        "run-summary.json",
        "offline-eval.json",
        "comparison.json",
        "stage-summary.json",
        "population-snapshot.json",
        "population-ledger.jsonl",
        "novelty-archive.json",
        "information-ledger.jsonl",
        "lineage-graph.jsonl",
        "concept-gene-ledger.jsonl",
        "stage-concept-ledger.jsonl",
        "metrics-timeseries.jsonl",
        "generation-ledger.jsonl",
        "stage-variant-ledger.jsonl",
        "live-call-ledger.jsonl",
        "research-ledger.jsonl",
        "memory-ledger.jsonl",
        "promotion-ledger.jsonl",
        "preflight.json",
        "quality-gate.json",
    ]);
    let mut paths = Vec::new();
    for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let filename = entry.file_name().to_string_lossy().to_string();
        if allowed.contains(filename.as_str()) {
            paths.push(entry.into_path());
        }
    }
    paths.sort();
    Ok(paths)
}

pub(crate) fn validate_hybrid_invariants(root: &Path) -> Result<()> {
    let population_paths: Vec<PathBuf> = if root.exists() {
        WalkDir::new(root)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry.file_type().is_file()
                    && entry.file_name().to_string_lossy() == "population-snapshot.json"
            })
            .map(|entry| entry.into_path())
            .collect()
    } else {
        Vec::new()
    };
    if population_paths.is_empty() {
        return Ok(());
    }
    let mut candidate_ids_by_run: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut promoted_ids_by_run: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut parent_ids_by_run: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut child_to_parents_by_run: BTreeMap<String, BTreeMap<String, Vec<String>>> =
        BTreeMap::new();
    let mut information_counts: BTreeMap<String, usize> = BTreeMap::new();
    for path in population_paths {
        let snapshot: Value = serde_json::from_str(&fs::read_to_string(&path)?)?;
        let run_id = snapshot
            .get("run_id")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let candidate_ids = snapshot
            .get("candidate_ids")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let ids: BTreeSet<String> = candidate_ids
            .iter()
            .filter_map(Value::as_str)
            .map(ToString::to_string)
            .collect();
        if ids.is_empty() {
            bail!("empty population snapshot in {}", path.display());
        }
        if ids.len() != candidate_ids.len() {
            bail!("duplicate candidate ids in {}", path.display());
        }
        candidate_ids_by_run
            .entry(run_id.clone())
            .or_default()
            .extend(ids);
        promoted_ids_by_run
            .entry(run_id.clone())
            .or_default()
            .extend(
                snapshot
                    .get("promoted_candidate_ids")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(ToString::to_string),
            );
        if let Some(candidates) = snapshot.get("candidates").and_then(Value::as_array) {
            for candidate in candidates {
                let child_id = candidate
                    .get("candidate_id")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let parent_ids = candidate
                    .get("parent_candidate_ids")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                for parent_id in parent_ids {
                    let parent_id = parent_id.as_str().unwrap_or("").to_string();
                    parent_ids_by_run
                        .entry(run_id.clone())
                        .or_default()
                        .insert(parent_id.clone());
                    child_to_parents_by_run
                        .entry(run_id.clone())
                        .or_default()
                        .entry(child_id.clone())
                        .or_default()
                        .push(parent_id);
                }
            }
        }
    }
    for path in WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.file_type().is_file()
                && entry.file_name().to_string_lossy() == "information-ledger.jsonl"
        })
        .map(|entry| entry.into_path())
    {
        for card in read_jsonl::<Value>(&path).unwrap_or_default() {
            let run_id = card
                .get("run_id")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            if card
                .get("provenance_hash")
                .and_then(Value::as_str)
                .is_none()
            {
                bail!(
                    "information card missing provenance hash in {}",
                    path.display()
                );
            }
            *information_counts.entry(run_id).or_default() += 1;
        }
    }
    for (run_id, candidate_ids) in candidate_ids_by_run {
        let missing_parents = parent_ids_by_run
            .get(&run_id)
            .cloned()
            .unwrap_or_default()
            .difference(&candidate_ids)
            .cloned()
            .collect::<Vec<_>>();
        if !missing_parents.is_empty() {
            bail!(
                "missing parent candidate ids for {}: {:?}",
                run_id,
                &missing_parents[..missing_parents.len().min(5)]
            );
        }
        let orphan_promoted = promoted_ids_by_run
            .get(&run_id)
            .cloned()
            .unwrap_or_default()
            .difference(&candidate_ids)
            .cloned()
            .collect::<Vec<_>>();
        if !orphan_promoted.is_empty() {
            bail!(
                "orphan promoted candidate ids for {}: {:?}",
                run_id,
                &orphan_promoted[..orphan_promoted.len().min(5)]
            );
        }
        let empty_graph = BTreeMap::new();
        let graph = child_to_parents_by_run.get(&run_id).unwrap_or(&empty_graph);
        if !lineage_edges_are_acyclic(graph) {
            bail!("lineage cycle detected for {}", run_id);
        }
        if information_counts.get(&run_id).copied().unwrap_or(0) == 0 {
            bail!("empty information ledger for {}", run_id);
        }
    }
    Ok(())
}

pub(crate) fn validate_record_kind(record: &Value) -> Result<()> {
    validate_record(record)
}
