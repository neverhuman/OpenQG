use super::*;

pub(crate) fn write_json(path: &Path, value: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

pub(crate) fn write_markdown(path: &Path, text: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, text)?;
    Ok(())
}

pub(crate) fn touch_latest(output_root: &Path, run_dir: &Path) -> Result<()> {
    let latest = output_root.join("latest");
    if let Ok(metadata) = fs::symlink_metadata(&latest) {
        if metadata.is_dir() && !metadata.file_type().is_symlink() {
            fs::remove_dir_all(&latest)?;
        } else {
            fs::remove_file(&latest)?;
        }
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(run_dir, &latest)?;
    #[cfg(not(unix))]
    {
        fs::create_dir_all(&latest)?;
    }
    Ok(())
}

pub(crate) fn write_checkpoint(
    run_dir: &Path,
    run_id: &str,
    variant: &str,
    generation_index: usize,
    max_generations: usize,
    output_guard: Option<&OutputPathGuard>,
) -> Result<()> {
    if let Some(guard) = output_guard {
        guard.check()?;
    }
    let checkpoint = json!({
        "schema_version": SCHEMA_VERSION,
        "run_id": run_id,
        "variant": variant,
        "complete_generation": generation_index,
        "complete_generation_id": format!("g{:04}", generation_index),
        "target_generation": max_generations,
        "updated_at": now_iso8601(),
    });
    if let Some(guard) = output_guard {
        guard.check()?;
    }
    write_json(&run_dir.join("checkpoint.json"), &checkpoint)?;
    if let Some(guard) = output_guard {
        guard.check()?;
    }
    write_json(
        &run_dir
            .join("checkpoints")
            .join(format!("g{:04}.json", generation_index)),
        &checkpoint,
    )?;
    Ok(())
}

pub(crate) fn artifact_paths(
    run_dir: &Path,
    stage_dir: &Path,
    inputs: &[String],
    outputs: &[String],
) -> Value {
    json!({
        "run_dir": run_dir.display().to_string(),
        "stage_dir": stage_dir.display().to_string(),
        "inputs": inputs,
        "outputs": outputs,
    })
}

pub(crate) fn read_stage_rankings(run_dir: &Path) -> Vec<Value> {
    let summary_paths = WalkDir::new(run_dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.file_type().is_file()
                && entry.file_name().to_string_lossy() == "stage-summary.json"
        })
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();
    let mut rows = Vec::new();
    for path in summary_paths {
        if let Ok(text) = fs::read_to_string(&path) {
            if let Ok(summary) = serde_json::from_str::<Value>(&text) {
                rows.push(json!({
                    "stage_id": field_or(&summary, "stage_id", empty_string_json),
                    "final_score": field_or(&summary, "best_final_score", zero_f64_json),
                    "delta_score": field_or(&summary, "mean_final_score", zero_f64_json),
                    "route": "jnoccio/standard",
                }));
            }
        }
    }
    rows
}

pub(crate) fn write_generated_text(path: &Path, body: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, body)?;
    Ok(())
}

pub(crate) fn parse_zyal_header(header: &str, path: &Path) -> Result<String> {
    let prefix = "<<<ZYAL v1:daemon id=";
    let suffix = ">>>";
    let trimmed = header.trim();
    if !trimmed.starts_with(prefix) || !trimmed.ends_with(suffix) {
        bail!("invalid ZYAL envelope in {}", path.display());
    }
    Ok(trimmed[prefix.len()..trimmed.len() - suffix.len()].to_string())
}

pub(crate) fn load_zyal_document(text: &str, path: &Path) -> Result<Value> {
    let mut lines = text.lines();
    let Some(header) = lines.next() else {
        anyhow::bail!("empty ZYAL document: {}", path.display());
    };
    let id = parse_zyal_header(header, path)?;
    let end_line = format!("<<<END_ZYAL id={id}>>>");
    let arm_line = format!("ZYAL_ARM RUN_FOREVER id={id}");
    let mut body = Vec::new();
    let mut closed = false;
    for line in lines.by_ref() {
        if line.trim() == end_line {
            closed = true;
            break;
        }
        body.push(line);
    }
    if !closed {
        bail!("missing closing sentinel in {}", path.display());
    }
    let trailing: Vec<_> = lines.filter(|line| !line.trim().is_empty()).collect();
    if trailing != vec![arm_line.as_str()] {
        bail!("invalid ZYAL closing arm in {}", path.display());
    }
    let yaml_body = body.join("\n");
    let value: YamlValue =
        serde_yaml::from_str(&yaml_body).with_context(|| format!("parse {}", path.display()))?;
    Ok(serde_json::to_value(value)?)
}

pub(crate) fn load_document(path: &Path) -> Result<Value> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    if path.extension().and_then(|ext| ext.to_str()) == Some("zyal") {
        load_zyal_document(&text, path)
    } else {
        let value: YamlValue =
            serde_yaml::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
        Ok(serde_json::to_value(value)?)
    }
}

pub(crate) fn load_runbook(path: &Path) -> Result<Value> {
    resolve_imports(path, &mut BTreeSet::new())
}

pub(crate) fn variant_from_runbook(runbook: &Value) -> Option<GenomeVariant> {
    match runbook
        .get("evaluation")
        .and_then(|evaluation| evaluation.get("variant"))
        .and_then(Value::as_str)
    {
        Some("pure-jnoccio") => Some(GenomeVariant::PureJnoccio),
        Some("hybrid") => Some(GenomeVariant::Hybrid),
        Some("jailgun-only") => Some(GenomeVariant::JailgunOnly),
        _ => None,
    }
}

pub(crate) fn load_population_snapshot(path: &Path) -> Option<Value> {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
}

pub(crate) fn load_stage_scores_from_json(
    stage_registry: &[StagePackage],
    stage_ledgers: &[Value],
) -> BTreeMap<String, Vec<f64>> {
    stage_score_history_from_ledgers(stage_registry, stage_ledgers)
}

pub(crate) fn read_json_value(path: &Path) -> Result<Value> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    Ok(serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))?)
}
