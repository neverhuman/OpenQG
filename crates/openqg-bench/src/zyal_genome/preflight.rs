use super::*;

pub fn preflight(
    variant: Option<GenomeVariant>,
    runbook_path: PathBuf,
    output_root: PathBuf,
    run_id: Option<String>,
    stage_root: Option<PathBuf>,
    require_hard_backend: bool,
    live_smoke: bool,
    live_selective: bool,
    jailgun_available: bool,
    jailgun_artifact_smoke: bool,
    jailgun_artifact_smoke_extension: String,
) -> Result<()> {
    let runbook = load_runbook(&runbook_path)?;
    let variant = variant
        .or_else(|| variant_from_runbook(&runbook))
        .unwrap_or(GenomeVariant::Hybrid);
    let evaluation = field_or(&runbook, "evaluation", empty_object);
    let output_root = if output_root != PathBuf::from(DEFAULT_OUTPUT_ROOT) {
        output_root
    } else {
        evaluation
            .get("output_root")
            .and_then(Value::as_str)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_OUTPUT_ROOT))
    };
    let max_generations = resolve_generation_count(None, None, &evaluation)?;
    let run_id = run_id
        .or_else(|| {
            evaluation
                .get("run_id")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| default_run_id(variant.as_str(), DEFAULT_SEED, max_generations));
    let run_dir = output_root.join("runs").join(&run_id);
    fs::create_dir_all(&run_dir)?;
    let output_guard = OutputPathGuard::new(&output_root)?;
    let stage_root = stage_root
        .or_else(|| {
            runbook
                .get("context")
                .and_then(|context| context.get("stage_root"))
                .and_then(Value::as_str)
                .map(PathBuf::from)
        })
        .unwrap_or_else(|| PathBuf::from(DEFAULT_STAGE_ROOT));
    let stage_registry = load_stage_registry(&stage_root)?;
    validate_stage_registry(&stage_registry)?;
    let live_config = resolve_live_config(live_selective, &runbook);
    let require_hard_backend =
        require_hard_backend || hard_backend_required(&runbook, &variant, &run_id, max_generations);
    let jailgun_status = resolve_jailgun_status(
        strict_jailgun_required(
            &variant,
            &run_id,
            require_hard_backend,
            hard_stage_count(&stage_registry),
        ),
        jailgun_available,
    );
    let jailgun_available = jailgun_status.available;
    output_guard.check()?;
    let mut receipt = preflight_receipt(
        &run_id,
        variant.as_str(),
        &runbook,
        &runbook_path,
        &run_dir,
        &stage_registry,
        &live_config,
        require_hard_backend,
        live_smoke,
        &jailgun_status,
    );
    let artifact_smoke_receipt = if jailgun_artifact_smoke {
        let smoke = run_jailgun_artifact_smoke(
            &run_dir,
            &run_id,
            &jailgun_artifact_smoke_extension,
            Some(&output_guard),
        )?;
        receipt["backend_health"]["jailgun"]["artifact_smoke"] = smoke.clone();
        Some(smoke)
    } else {
        None
    };
    output_guard.check()?;
    write_json(&run_dir.join("preflight.json"), &receipt)?;
    let hard_backend_blocked = matches!(variant, GenomeVariant::Hybrid)
        && require_hard_backend
        && hard_stage_count(&stage_registry) > 0
        && !jailgun_available;
    let live_command_blocked = live_smoke
        && !receipt
            .get("live_command")
            .and_then(|value| value.get("executable_found"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
    let artifact_smoke_blocked = artifact_smoke_receipt
        .as_ref()
        .map(|smoke| smoke.get("status").and_then(Value::as_str) != Some("ok"))
        .unwrap_or(false);
    if hard_backend_blocked || live_command_blocked || artifact_smoke_blocked {
        bail!(
            "preflight failed for {} (hard_backend_available={}, live_command_found={}, jailgun_artifact_smoke_ok={})",
            run_id,
            jailgun_available,
            !live_command_blocked,
            !artifact_smoke_blocked
        );
    }
    println!(
        "wrote preflight receipt to {}",
        run_dir.join("preflight.json").display()
    );
    Ok(())
}

pub(crate) fn preflight_receipt(
    run_id: &str,
    variant: &str,
    runbook: &Value,
    runbook_path: &Path,
    run_dir: &Path,
    stage_registry: &[StagePackage],
    live_config: &LiveConfig,
    require_hard_backend: bool,
    live_smoke: bool,
    jailgun_status: &JailgunStatus,
) -> Value {
    let command = live_command(live_config);
    let executable_found = command
        .first()
        .map(|executable| command_exists(executable))
        .unwrap_or(false);
    let (provider, model) = live_provider_model(&command);
    let hard_stages = hard_stage_count(stage_registry);
    let hybrid_requires_hard_backend =
        variant == "hybrid" && require_hard_backend && hard_stages > 0;
    let jailgun_available = jailgun_status.available;
    let routing_decision = if hybrid_requires_hard_backend && !jailgun_available {
        "blocked_missing_hard_backend"
    } else if variant == "hybrid" && hard_stages > 0 && !jailgun_available {
        "degraded_fallback_available"
    } else {
        "nominal"
    };
    json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "preflight",
        "run_id": run_id,
        "variant": variant,
        "runbook_path": runbook_path.display().to_string(),
        "run_dir": run_dir.display().to_string(),
        "created_at": now_iso8601(),
        "status": if routing_decision == "blocked_missing_hard_backend" || (live_smoke && !executable_found) { "failed" } else { "ok" },
        "backend_health": {
            "hard_backend": "jailgun",
            "hard_stage_count": hard_stages,
            "require_hard_backend": require_hard_backend,
            "jailgun_available": jailgun_available,
            "jailgun_evidence": jailgun_status.evidence.as_str(),
            "jailgun": jailgun_status.as_json(),
        },
        "live_command": {
            "enabled": live_config.enabled,
            "smoke_requested": live_smoke,
            "command": command,
            "executable_found": executable_found,
            "provider": provider,
            "model": model,
        },
        "timeout_config": {
            "default_seconds": live_config.timeout_seconds,
            "by_purpose": live_config.timeout_by_purpose.clone(),
            "retry_count": live_config.retry_count,
        },
        "routing_decision": routing_decision,
        "quality_gates": nested_field_or(runbook, "evaluation", "quality_gates", empty_object),
    })
}

pub(crate) fn live_provider_model(command: &[String]) -> (Option<String>, Option<String>) {
    let mut provider = None;
    let mut model = None;
    let mut iter = command.iter();
    while let Some(arg) = iter.next() {
        if arg == "--provider" {
            provider = iter.next().cloned();
        } else if arg == "--model" {
            model = iter.next().cloned();
        }
    }
    (provider, model)
}

pub(crate) fn command_exists(command: &str) -> bool {
    if command.contains('/') {
        return Path::new(command).is_file();
    }
    let Some(path_var) = env::var_os("PATH") else {
        return false;
    };
    env::split_paths(&path_var).any(|path| path.join(command).is_file())
}

pub(crate) fn resolve_imports(path: &Path, seen: &mut BTreeSet<PathBuf>) -> Result<Value> {
    let path = path
        .canonicalize()
        .with_context(|| format!("resolve {}", path.display()))?;
    if !seen.insert(path.clone()) {
        bail!("cyclic import detected for {}", path.display());
    }
    let doc = load_document(&path)?;
    let mut merged = json!({});
    if let Some(imports) = doc.get("imports").and_then(Value::as_array) {
        for import_entry in imports {
            if let Some(import) = import_entry.as_str() {
                let imported =
                    resolve_imports(&path.parent().unwrap_or(Path::new(".")).join(import), seen)?;
                deep_merge(&mut merged, &imported);
            }
        }
    }
    let mut doc = doc;
    if let Some(map) = doc.as_object_mut() {
        map.remove("imports");
    }
    deep_merge(&mut merged, &doc);
    Ok(merged)
}

pub(crate) fn default_run_id(variant: &str, seed: u64, generations: usize) -> String {
    format!("{variant}-{seed}-{generations}")
}

pub(crate) fn live_call_to_receipt(record: &Value) -> Value {
    record.clone()
}
