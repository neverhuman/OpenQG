use super::*;

pub(crate) fn load_stage_registry(stage_root: &Path) -> Result<Vec<StagePackage>> {
    if !stage_root.exists() {
        bail!("missing stage registry root: {}", stage_root.display());
    }
    let mut stage_files = Vec::new();
    for entry in WalkDir::new(stage_root).min_depth(1).max_depth(2) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.into_path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if name == "stage.yml"
            || name == "stage.yaml"
            || (path.parent() == Some(stage_root)
                && matches!(
                    path.extension().and_then(|ext| ext.to_str()),
                    Some("yml" | "yaml")
                ))
        {
            stage_files.push(path);
        }
    }
    stage_files.sort();
    let mut stages = Vec::new();
    for path in stage_files {
        stages.push(load_stage_package(&path)?);
    }
    Ok(stages)
}

pub(crate) fn load_stage_package(path: &Path) -> Result<StagePackage> {
    let value = load_document(path)?;
    let stage_id = value
        .get("stage_id")
        .and_then(Value::as_str)
        .context("missing stage_id")?
        .to_string();
    let stage_dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
    let prompt_path = stage_dir.join("prompt.md");
    let memory_path = stage_dir.join("memory.yml");
    let score_path = stage_dir.join("score.yml");
    let prompt_hash = sha256_digest(
        fs::read(&prompt_path)
            .with_context(|| format!("read {}", prompt_path.display()))?
            .as_slice(),
    );
    Ok(StagePackage {
        stage_id,
        name: value
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("stage")
            .to_string(),
        family: value
            .get("family")
            .and_then(Value::as_str)
            .unwrap_or("standard")
            .to_string(),
        track: value
            .get("track")
            .and_then(Value::as_str)
            .unwrap_or("standard")
            .to_string(),
        purpose: value
            .get("purpose")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        inputs: value
            .get("inputs")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_else(Vec::new)
            .into_iter()
            .filter_map(|v| v.as_str().map(ToString::to_string))
            .collect(),
        outputs: value
            .get("outputs")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_else(Vec::new)
            .into_iter()
            .filter_map(|v| v.as_str().map(ToString::to_string))
            .collect(),
        required_evidence: value
            .get("required_evidence")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_else(Vec::new)
            .into_iter()
            .filter_map(|v| v.as_str().map(ToString::to_string))
            .collect(),
        validation_checks: value
            .get("validation_checks")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_else(Vec::new)
            .into_iter()
            .filter_map(|v| v.as_str().map(ToString::to_string))
            .collect(),
        mutation_op: value
            .get("mutation_op")
            .and_then(Value::as_str)
            .unwrap_or("emit_atlas")
            .to_string(),
        prompt_path,
        memory_path: memory_path.clone(),
        score_path: score_path.clone(),
        stage_dir,
        stage_file: path.to_path_buf(),
        prompt_hash,
        memory: load_optional_yaml(&memory_path)?,
        score_model: load_optional_yaml(&score_path)?,
    })
}

pub(crate) fn validate_stage_registry(stages: &[StagePackage]) -> Result<()> {
    if stages.is_empty() {
        bail!("stage registry is empty");
    }
    let mut seen = BTreeSet::new();
    for stage in stages {
        if !seen.insert(stage.stage_id.clone()) {
            bail!("duplicate stage id {}", stage.stage_id);
        }
        if stage.inputs.is_empty()
            || stage.outputs.is_empty()
            || stage.required_evidence.is_empty()
            || stage.validation_checks.is_empty()
        {
            bail!("stage {} is missing required lists", stage.stage_id);
        }
    }
    Ok(())
}

pub(crate) fn load_optional_yaml(path: &Path) -> Result<Value> {
    if !path.exists() {
        return Ok(json!({}));
    }
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let value: YamlValue =
        serde_yaml::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
    Ok(serde_json::to_value(value)?)
}

pub(crate) fn hard_stage_count(stages: &[StagePackage]) -> usize {
    stages.iter().filter(|stage| stage.family == "hard").count()
}

pub(crate) fn stage_package_prompt(stage: &StagePackage) -> String {
    if stage.prompt_path.is_file() {
        fs::read_to_string(&stage.prompt_path).unwrap_or_else(|_| stage.purpose.clone())
    } else {
        stage.purpose.clone()
    }
}
