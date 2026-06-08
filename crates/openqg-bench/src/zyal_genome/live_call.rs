use super::*;

pub(crate) fn live_call_purpose(
    live_config: &LiveConfig,
    stage: &StagePackage,
    generation_index: usize,
    stage_index: usize,
    stage_count: usize,
) -> Option<String> {
    if !live_config.enabled {
        return None;
    }
    if generation_index == 1 && stage_index == 0 && live_config.research_synthesis {
        return Some("research_synthesis".to_string());
    }
    if stage.family == "hard"
        && live_config.hard_stage_repair
        && (generation_index == 1 || generation_index % live_config.hard_stage_every == 0)
    {
        return Some("hard_stage_repair".to_string());
    }
    if stage.stage_id.ends_with("promotion")
        && live_config.promotion_judging
        && (generation_index == 1 || generation_index % live_config.promotion_every == 0)
    {
        return Some("promotion_judging".to_string());
    }
    if live_config.champion_audit_every > 0
        && generation_index % live_config.champion_audit_every == 0
        && stage_index + 1 == stage_count
    {
        return Some("champion_audit".to_string());
    }
    None
}

pub(crate) fn run_live_call(
    run_dir: &Path,
    stage: &StagePackage,
    route: &RoutePolicy,
    generation_id: &str,
    candidate_id: &str,
    purpose: &str,
    live_config: &LiveConfig,
    research_refs: &[String],
    output_guard: Option<&OutputPathGuard>,
) -> Result<Value> {
    if let Some(guard) = output_guard {
        guard.check()?;
    }
    let call_id = format!("live-{generation_id}-{}-{purpose}", stage.stage_id);
    let call_dir = run_dir
        .join("stages")
        .join(&stage.stage_id)
        .join("generations")
        .join(generation_id)
        .join("live-calls")
        .join(&call_id);
    fs::create_dir_all(&call_dir)?;
    let run_id = infer_run_id_from_path(run_dir);
    let execution_backend = live_execution_backend(route);
    let stage_prompt = stage_prompt_text(stage)?;
    let jailgun_download_target_name = (execution_backend == "jailgun_mcp")
        .then(|| jailgun_download_target_name(&run_id, &call_id, stage, purpose, &stage_prompt));
    let retrieval_packet = json!({
        "schema_version": SCHEMA_VERSION,
        "run_dir": run_dir.display().to_string(),
        "run_id": run_id.as_str(),
        "generation_id": generation_id,
        "stage_id": stage.stage_id,
        "candidate_id": candidate_id,
        "purpose": purpose,
        "route_backend": route.route_backend,
        "route_tier": route.route_tier,
        "router_state": route.router_state,
        "stage_path": stage.stage_file.display().to_string(),
        "prompt_hash": stage.prompt_hash,
        "memory_refs": memory_refs_for_stage(stage),
        "research_refs": research_refs,
        "required_evidence": stage.required_evidence,
        "validation_checks": stage.validation_checks,
        "jailgun_download_target_name": jailgun_download_target_name.as_deref(),
    });
    let prompt = live_prompt(
        stage,
        &stage_prompt,
        &retrieval_packet,
        jailgun_download_target_name.as_deref(),
    )?;
    let prompt_path = call_dir.join("prompt.md");
    let retrieval_path = call_dir.join("retrieval-packet.json");
    let raw_output_path = call_dir.join("raw-output.txt");
    let parsed_summary_path = call_dir.join("parsed-summary.json");
    let receipt_path = call_dir.join("receipt.json");
    if let Some(guard) = output_guard {
        guard.check()?;
    }
    write_markdown(&prompt_path, &prompt)?;
    write_json(&retrieval_path, &retrieval_packet)?;

    let command = if execution_backend == "jailgun_mcp" {
        Vec::new()
    } else {
        live_command(live_config)
    };
    let timeout_seconds = live_timeout_for_purpose(live_config, purpose);
    let max_attempts = live_max_attempts(live_config, purpose, execution_backend);
    let mut attempts = Vec::new();
    let mut final_stdout = String::new();
    let mut final_stderr = String::new();
    let mut exit_code = None;
    let mut status = "failed".to_string();
    let mut error = None;
    let mut started_at = now_iso8601();
    let mut final_metadata = json!({});
    let overall_start = Instant::now();
    for attempt_index in 1..=max_attempts {
        let attempt_started_at = now_iso8601();
        if attempt_index == 1 {
            started_at = attempt_started_at.clone();
        }
        let attempt_stdout_path = call_dir.join(format!("raw-stdout-attempt-{attempt_index}.txt"));
        let attempt_stderr_path = call_dir.join(format!("raw-stderr-attempt-{attempt_index}.txt"));
        if let Some(guard) = output_guard {
            guard.check()?;
        }
        let attempt = if execution_backend == "jailgun_mcp" {
            run_jailgun_live_call_attempt(
                &call_id,
                &prompt_path,
                jailgun_download_target_name
                    .as_deref()
                    .expect("jailgun target name"),
                timeout_seconds,
                attempt_index,
                &attempt_started_at,
            )
        } else {
            run_live_call_attempt(
                &command,
                &prompt,
                timeout_seconds,
                attempt_index,
                &attempt_started_at,
            )?
        };
        fs::write(&attempt_stdout_path, &attempt.stdout)?;
        fs::write(&attempt_stderr_path, &attempt.stderr)?;
        final_stdout = attempt.stdout.clone();
        final_stderr = attempt.stderr.clone();
        exit_code = attempt.exit_code;
        status = attempt.status.clone();
        error = attempt.error.clone();
        final_metadata = attempt.metadata.clone();
        let should_rate_limit_backoff = status != "ok"
            && attempt_index < max_attempts
            && live_attempt_failure_kind(&attempt) == Some("rate-limit");
        let should_rate_limit_cooldown =
            status == "ok" && live_attempt_warning_kind(&attempt) == Some("rate-limit");
        let mut attempt_record = json!({
            "attempt": attempt_index,
            "status": attempt.status,
            "exit_code": attempt.exit_code,
            "started_at": attempt_started_at,
            "elapsed_seconds": round6(attempt.elapsed_seconds),
            "timeout_seconds": timeout_seconds,
            "raw_stdout_path": attempt_stdout_path.display().to_string(),
            "raw_stderr_path": attempt_stderr_path.display().to_string(),
            "error": attempt.error,
        });
        merge_object(&mut attempt_record, &attempt.metadata);
        if should_rate_limit_backoff {
            attempt_record["retry_backoff_seconds"] = json!(JAILGUN_RATE_LIMIT_BACKOFF_SECONDS);
        }
        if should_rate_limit_cooldown {
            attempt_record["rate_limit_cooldown_seconds"] =
                json!(JAILGUN_RATE_LIMIT_BACKOFF_SECONDS);
            final_metadata["jailgun_rate_limit_cooldown_seconds"] =
                json!(JAILGUN_RATE_LIMIT_BACKOFF_SECONDS);
        }
        attempts.push(attempt_record);
        if status == "ok" {
            if should_rate_limit_cooldown {
                thread::sleep(Duration::from_secs(JAILGUN_RATE_LIMIT_BACKOFF_SECONDS));
            }
            break;
        }
        if should_rate_limit_backoff {
            thread::sleep(Duration::from_secs(JAILGUN_RATE_LIMIT_BACKOFF_SECONDS));
        }
    }

    let raw = if final_stderr.is_empty() {
        final_stdout.clone()
    } else {
        format!("{final_stdout}\n[stderr]\n{final_stderr}")
    };
    fs::write(&raw_output_path, raw)?;
    let token_usage = json!({
        "prompt": estimate_tokens(&prompt),
        "completion": estimate_tokens(&final_stdout),
        "total": estimate_tokens(&prompt) + estimate_tokens(&final_stdout),
    });
    let parsed = live_summary(&final_stdout, &status, error.as_deref());
    write_json(&parsed_summary_path, &parsed)?;
    let mut record = json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "live_call",
        "run_id": run_id,
        "generation_id": generation_id,
        "stage_id": stage.stage_id,
        "candidate_id": candidate_id,
        "call_id": call_id,
        "purpose": purpose,
        "route_backend": route.route_backend,
        "route_tier": route.route_tier,
        "router_state": route.router_state,
        "execution_backend": execution_backend,
        "status": status,
        "exit_code": exit_code,
        "started_at": started_at,
        "elapsed_seconds": round6(overall_start.elapsed().as_secs_f64()),
        "timeout_seconds": timeout_seconds,
        "configured_timeout_seconds": timeout_seconds,
        "retry_count": live_config.retry_count,
        "attempt_count": attempts.len(),
        "attempts": attempts,
        "command": command,
        "prompt_path": prompt_path.display().to_string(),
        "retrieval_packet_path": retrieval_path.display().to_string(),
        "raw_output_path": raw_output_path.display().to_string(),
        "parsed_summary_path": parsed_summary_path.display().to_string(),
        "receipt_path": receipt_path.display().to_string(),
        "token_usage": token_usage,
        "summary": field_or(&parsed, "summary", empty_string_json),
        "error": error,
    });
    merge_object(&mut record, &final_metadata);
    write_json(&receipt_path, &record)?;
    Ok(record)
}

pub(crate) fn live_execution_backend(route: &RoutePolicy) -> &'static str {
    if route.route_backend == "jailgun" {
        "jailgun_mcp"
    } else {
        "jekko_command"
    }
}

pub(crate) fn live_command(live_config: &LiveConfig) -> Vec<String> {
    if live_config.command.is_empty() {
        JEKKO_LIVE_COMMAND.iter().map(|s| s.to_string()).collect()
    } else {
        live_config.command.clone()
    }
}

pub(crate) fn live_max_attempts(
    live_config: &LiveConfig,
    purpose: &str,
    execution_backend: &str,
) -> usize {
    let configured_attempts = live_config.retry_count.saturating_add(1).max(1);
    if purpose == "hard_stage_repair" && execution_backend == "jailgun_mcp" {
        configured_attempts.saturating_add(JAILGUN_RATE_LIMIT_EXTRA_ATTEMPTS)
    } else {
        configured_attempts
    }
}

pub(crate) fn live_timeout_for_purpose(live_config: &LiveConfig, purpose: &str) -> u64 {
    live_config
        .timeout_by_purpose
        .get(purpose)
        .copied()
        .unwrap_or(live_config.timeout_seconds)
}

pub(crate) fn spawn_pipe_reader<R>(mut reader: R) -> mpsc::Receiver<String>
where
    R: Read + Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut buffer = String::new();
        let _ = reader.read_to_string(&mut buffer);
        let _ = tx.send(buffer);
    });
    rx
}

pub(crate) fn collect_pipe(
    rx: Option<mpsc::Receiver<String>>,
    timeout: Duration,
) -> Option<String> {
    rx.and_then(|rx| rx.recv_timeout(timeout).ok())
}

pub(crate) fn kill_live_process_tree(child_pid: u32) {
    #[cfg(unix)]
    // SAFETY: kill(2) is async-signal-safe and takes only integer arguments; sending SIGKILL to
    // the process group and pid cannot create memory-safety hazards in this process.
    unsafe {
        let pid = child_pid as i32;
        let _ = kill(-pid, SIGKILL_NUM);
        let _ = kill(pid, SIGKILL_NUM);
    }
    #[cfg(not(unix))]
    {
        let _ = child_pid;
    }
}

pub(crate) fn live_prompt(
    stage: &StagePackage,
    stage_prompt: &str,
    retrieval_packet: &Value,
    jailgun_download_target_name: Option<&str>,
) -> Result<String> {
    let artifact_instruction = jailgun_download_target_name
        .map(|name| {
            format!(
                "Create a fresh downloadable artifact named exactly `{name}` now. The filename and extension are authoritative. Do not answer with a plan, acknowledgement, or prose outside the artifact. Do not recover or reuse an artifact from another conversation. Keep the content concise and match the file type requested by the filename and stage prompt."
            )
        })
        .unwrap_or_else(|| {
            "Return concise JSON-like notes with risks, repairs, and audit concerns.".to_string()
        });
    Ok([
        "# ZYAL Selective Live Call",
        "",
        &format!(
            "Purpose: {}",
            retrieval_packet
                .get("purpose")
                .and_then(Value::as_str)
                .unwrap_or("")
        ),
        &format!("Stage: {} - {}", stage.stage_id, stage.name),
        "",
        "Use the retrieval packet as the only evidence context.",
        "Return frontier-value review material: a falsifiable claim, source-card grounding, concrete tests, known failure modes, and a review priority.",
        "Do not rely on raw provider logs, target artifacts, or unstated external evidence.",
        artifact_instruction.as_str(),
        "",
        "## Stage Prompt",
        stage_prompt,
        "",
        "## Retrieval Packet",
        &serde_json::to_string_pretty(retrieval_packet)?,
    ]
    .join("\n"))
}

pub(crate) fn stage_prompt_text(stage: &StagePackage) -> Result<String> {
    if stage.prompt_path.is_file() {
        fs::read_to_string(&stage.prompt_path)
            .with_context(|| format!("read {}", stage.prompt_path.display()))
    } else {
        Ok(stage.purpose.clone())
    }
}

pub(crate) fn live_summary_record(stdout: &str, status: &str, error: Option<&str>) -> Value {
    let summary = if stdout.trim().is_empty() {
        error.unwrap_or(status).to_string()
    } else {
        first_sentence(stdout)
    };
    json!({
        "status": status,
        "summary": summary,
        "evidence_terms": novelty_terms_from_text(stdout).into_iter().take(8).collect::<Vec<_>>(),
        "reasoning_quality": if status == "ok" && !stdout.trim().is_empty() { 0.70 } else if status == "failed" { 0.20 } else { 0.10 },
    })
}

pub(crate) fn live_summary(stdout: &str, status: &str, error: Option<&str>) -> Value {
    let summary = if stdout.trim().is_empty() {
        error.unwrap_or(status).to_string()
    } else {
        first_sentence(stdout)
    };
    json!({
        "status": status,
        "summary": summary,
        "evidence_terms": novelty_terms_from_text(stdout).into_iter().take(8).collect::<Vec<_>>(),
        "reasoning_quality": if status == "ok" && !stdout.trim().is_empty() { 0.70 } else if status == "failed" { 0.20 } else { 0.10 },
    })
}

pub(crate) fn load_live_call_record_text(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok()
}
