use super::*;

pub(crate) fn jailgun_run_arguments(
    jailgun_run_id: &str,
    call_id: &str,
    prompt_path: &Path,
    timeout_seconds: u64,
    account_ids: &[String],
    bridge_cmd: &JailgunBridgeCommand,
    download_target_name: &str,
) -> Value {
    let mut bridge_env = serde_json::Map::new();
    bridge_env.insert(
        "JAILGUN_ARTIFACT_CONVERSATION_RECOVERY_LIMIT".to_string(),
        json!("0"),
    );
    bridge_env.insert("JAILGUN_ARTIFACT_REPAIR_ATTEMPTS".to_string(), json!("1"));
    // Reuse a provisioned X display (export DISPLAY before launching) instead of spawning a
    // fresh Xvfb per call. Per-call spawning exhausted the display pool and caused ~73% of the
    // last run's live calls to fail with "could not find a free Xvfb display number".
    if let Ok(display) = std::env::var("DISPLAY") {
        if !display.trim().is_empty() {
            bridge_env.insert("DISPLAY".to_string(), json!(display));
        }
    }
    json!({
        "version": 1,
        "run_id": jailgun_run_id,
        "prompt_ref": format!("openqg://zyal-genome/{call_id}"),
        "prompt_file": jailgun_prompt_file_path(prompt_path),
        "tabs": 1,
        "max_runtime_seconds": timeout_seconds.max(1),
        "browser": {
            "account_ids": account_ids,
            "allow_queueing": true,
            "queue_timeout_seconds": JAILGUN_QUEUE_TIMEOUT_SECONDS,
            "bridge_cmd": &bridge_cmd.args,
            "bridge_env": Value::Object(bridge_env),
            "download_target_name": download_target_name,
        },
        "source_archive": {
            "enabled": false,
        },
        "deploy": {
            "enabled": false,
            "dry_run": true,
        },
        "ci": {
            "enabled": false,
        },
        "github": {
            "allow_write_prompts": false,
            "allow_info_prompts": true,
        },
    })
}

pub(crate) fn jailgun_prompt_file_path(prompt_path: &Path) -> String {
    if prompt_path.is_absolute() {
        return prompt_path.display().to_string();
    }
    env::current_dir()
        .map(|cwd| cwd.join(prompt_path))
        .unwrap_or_else(|_| prompt_path.to_path_buf())
        .display()
        .to_string()
}

pub(crate) fn run_live_call_attempt(
    command: &[String],
    prompt: &str,
    timeout_seconds: u64,
    attempt: usize,
    _started_at: &str,
) -> Result<LiveAttempt> {
    if command.is_empty() {
        bail!("live command is empty");
    }
    let start = Instant::now();
    let mut cmd = Command::new(&command[0]);
    cmd.args(&command[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    // SAFETY: pre_exec runs in the forked child before exec; setsid() is async-signal-safe and
    // the closure performs no allocation or non-reentrant work beyond the single libc call.
    unsafe {
        cmd.pre_exec(|| {
            if setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = cmd.spawn().map_err(|err| anyhow::anyhow!(err))?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(prompt.as_bytes());
    }
    let stdout_rx = child.stdout.take().map(spawn_pipe_reader);
    let stderr_rx = child.stderr.take().map(spawn_pipe_reader);
    let mut exit_code = None;
    let status;
    let mut error = None;
    let timed_out;
    loop {
        if let Some(exit) = child.try_wait()? {
            exit_code = exit.code();
            status = if exit.success() { "ok" } else { "failed" }.to_string();
            timed_out = false;
            break;
        }
        if start.elapsed() >= Duration::from_secs(timeout_seconds) {
            kill_live_process_tree(child.id());
            let _ = child.wait();
            status = "timeout".to_string();
            error = Some(format!(
                "timeout after {timeout_seconds}s on attempt {attempt}"
            ));
            timed_out = true;
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }
    let pipe_wait = if timed_out {
        Duration::from_secs(1)
    } else {
        Duration::from_secs(5)
    };
    let stdout = collect_pipe(stdout_rx, pipe_wait)
        .unwrap_or_else(|| "[stdout unavailable after timeout]\n".to_string());
    let stderr = collect_pipe(stderr_rx, pipe_wait)
        .unwrap_or_else(|| "[stderr unavailable after timeout]\n".to_string());
    Ok(LiveAttempt {
        status,
        exit_code,
        elapsed_seconds: start.elapsed().as_secs_f64(),
        stdout,
        stderr,
        error,
        metadata: json!({}),
    })
}

pub(crate) fn run_jailgun_live_call_attempt(
    call_id: &str,
    prompt_path: &Path,
    download_target_name: &str,
    timeout_seconds: u64,
    attempt: usize,
    started_at: &str,
) -> LiveAttempt {
    run_jailgun_live_call_attempt_with_config(
        call_id,
        prompt_path,
        download_target_name,
        timeout_seconds,
        attempt,
        started_at,
        JailgunHealthConfig::from_env(),
        jailgun_bridge_command(),
    )
}

pub(crate) fn run_jailgun_live_call_attempt_with_config(
    call_id: &str,
    prompt_path: &Path,
    download_target_name: &str,
    timeout_seconds: u64,
    attempt: usize,
    started_at: &str,
    config: JailgunHealthConfig,
    bridge_cmd: JailgunBridgeCommand,
) -> LiveAttempt {
    let start = Instant::now();
    let server_url = config.server_url.clone();
    let token = match config.token.as_ref() {
        Some(value) => value,
        None => {
            let mut metadata = jailgun_transport_metadata(&config, None);
            metadata["bridge_cmd_source"] = json!(bridge_cmd.source.as_str());
            return failed_jailgun_attempt(
                start,
                Some(server_url),
                None,
                Vec::new(),
                "Jailgun token is not available from env or matching local jailgun serve process"
                    .to_string(),
                metadata,
            );
        }
    };
    let jailgun_status = strict_jailgun_status_with_config(config.clone());
    let account_ids = jailgun_status.ready_account_ids.clone();
    if !jailgun_status.available {
        let error = if jailgun_status.errors.is_empty() {
            "Jailgun server is not ready".to_string()
        } else {
            format!(
                "Jailgun server is not ready: {}",
                jailgun_status.errors.join("; ")
            )
        };
        let mut metadata = jailgun_transport_metadata(&config, Some(&jailgun_status));
        metadata["bridge_cmd_source"] = json!(bridge_cmd.source.as_str());
        return failed_jailgun_attempt(start, Some(server_url), None, account_ids, error, metadata);
    }
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(JAILGUN_MCP_HTTP_TIMEOUT_SECONDS))
        .build()
    {
        Ok(client) => client,
        Err(error) => {
            let mut metadata = jailgun_transport_metadata(&config, Some(&jailgun_status));
            metadata["bridge_cmd_source"] = json!(bridge_cmd.source.as_str());
            return failed_jailgun_attempt(
                start,
                Some(server_url),
                None,
                account_ids,
                format!("failed to build Jailgun HTTP client: {error}"),
                metadata,
            );
        }
    };
    let jailgun_run_id = jailgun_live_run_id(call_id, attempt, started_at);
    let account_ids = jailgun_single_account_ids(&account_ids, call_id, attempt);
    let run_args = jailgun_run_arguments(
        &jailgun_run_id,
        call_id,
        prompt_path,
        timeout_seconds,
        &account_ids,
        &bridge_cmd,
        download_target_name,
    );
    let accepted = match jailgun_mcp_tool_call(
        &client,
        &server_url,
        token.value.as_str(),
        &format!("{jailgun_run_id}-run"),
        "jailgun.run",
        run_args,
    ) {
        Ok(value) => value,
        Err(error) => {
            let mut metadata = jailgun_transport_metadata(&config, Some(&jailgun_status));
            metadata["bridge_cmd_source"] = json!(bridge_cmd.source.as_str());
            return failed_jailgun_attempt(
                start,
                Some(server_url),
                Some(jailgun_run_id),
                account_ids,
                error,
                metadata,
            );
        }
    };

    let deadline = start
        + Duration::from_secs(
            timeout_seconds + JAILGUN_MCP_SUMMARY_GRACE_SECONDS + JAILGUN_QUEUE_TIMEOUT_SECONDS,
        );
    let mut status_snapshots = Vec::new();
    let mut final_status = json!({});
    while Instant::now() < deadline {
        match jailgun_mcp_tool_call(
            &client,
            &server_url,
            token.value.as_str(),
            &format!("{jailgun_run_id}-status"),
            "jailgun.run_status",
            json!({ "run_id": &jailgun_run_id }),
        ) {
            Ok(status) => {
                final_status = status.clone();
                status_snapshots.push(status.clone());
                match status.get("status").and_then(Value::as_str).unwrap_or("") {
                    "succeeded" | "failed" | "timed-out" => break,
                    _ => {}
                }
            }
            Err(error) => {
                return failed_jailgun_attempt(
                    start,
                    Some(server_url),
                    Some(jailgun_run_id),
                    account_ids,
                    error,
                    json!({
                        "token_source": config.token_source(),
                        "account_source": jailgun_status.account_source.as_deref(),
                        "bridge_cmd_source": bridge_cmd.source.as_str(),
                        "jailgun_health": jailgun_status.as_json(),
                        "jailgun_started_response": accepted,
                        "jailgun_status_snapshots": status_snapshots,
                    }),
                );
            }
        }
        thread::sleep(Duration::from_millis(JAILGUN_MCP_POLL_INTERVAL_MILLIS));
    }

    let poll_deadline_elapsed = Instant::now() >= deadline;
    let summary = match jailgun_mcp_tool_call(
        &client,
        &server_url,
        token.value.as_str(),
        &format!("{jailgun_run_id}-summary"),
        "jailgun.run_summary",
        json!({ "run_id": &jailgun_run_id }),
    ) {
        Ok(value) => value,
        Err(error) => {
            return failed_jailgun_attempt(
                start,
                Some(server_url),
                Some(jailgun_run_id),
                account_ids,
                error,
                json!({
                        "token_source": config.token_source(),
                        "account_source": jailgun_status.account_source.as_deref(),
                        "bridge_cmd_source": bridge_cmd.source.as_str(),
                        "jailgun_health": jailgun_status.as_json(),
                        "jailgun_started_response": accepted,
                        "jailgun_status_snapshot": final_status,
                        "jailgun_status_snapshots": status_snapshots,
                }),
            );
        }
    };
    let summary_status = summary
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("missing");
    let effective_status = jailgun_effective_status(Some(&summary), &final_status);
    let mut metadata = jailgun_attempt_metadata(
        &server_url,
        &jailgun_run_id,
        &account_ids,
        &accepted,
        &final_status,
        &status_snapshots,
        Some(&summary),
        token.value.as_str(),
    );
    metadata["token_source"] = json!(config.token_source());
    metadata["account_source"] = json!(jailgun_status.account_source.as_deref());
    metadata["bridge_cmd_source"] = json!(bridge_cmd.source.as_str());
    metadata["jailgun_health"] = jailgun_status.as_json();
    metadata["jailgun_summary_status"] = json!(summary_status);
    metadata["jailgun_effective_status"] = json!(effective_status);
    let redacted_summary = redact_jailgun_token_in_value(&summary, token.value.as_str());
    let stdout = serde_json::to_string_pretty(&redacted_summary)
        .unwrap_or_else(|_| redacted_summary.to_string());
    let status = match effective_status.as_str() {
        "succeeded" => "ok",
        "timed-out" => "timeout",
        "running" | "accepted" if poll_deadline_elapsed => "timeout",
        _ => "failed",
    }
    .to_string();
    let error = if status == "ok" {
        None
    } else {
        Some(format!(
            "Jailgun run {jailgun_run_id} finished with status {effective_status}"
        ))
    };
    LiveAttempt {
        exit_code: if status == "ok" { Some(0) } else { Some(1) },
        status,
        elapsed_seconds: start.elapsed().as_secs_f64(),
        stdout,
        stderr: String::new(),
        error,
        metadata,
    }
}

pub(crate) fn failed_jailgun_attempt(
    start: Instant,
    server_url: Option<String>,
    jailgun_run_id: Option<String>,
    account_ids: Vec<String>,
    error: String,
    extra: Value,
) -> LiveAttempt {
    let mut metadata = json!({
        "jailgun_server_url": server_url,
        "jailgun_run_id": jailgun_run_id,
        "jailgun_account_count": account_ids.len(),
        "jailgun_status": "failed",
        "jailgun_failure_kind": classify_jailgun_failure(&error),
        "jailgun_warning_kind": "none",
        "jailgun_error": error,
    });
    merge_object(&mut metadata, &extra);
    LiveAttempt {
        status: "failed".to_string(),
        exit_code: None,
        elapsed_seconds: start.elapsed().as_secs_f64(),
        stdout: String::new(),
        stderr: String::new(),
        error: metadata
            .get("jailgun_error")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        metadata,
    }
}

pub(crate) fn run_jailgun_artifact_smoke(
    run_dir: &Path,
    run_id: &str,
    extension: &str,
    output_guard: Option<&OutputPathGuard>,
) -> Result<Value> {
    run_jailgun_artifact_smoke_with_config(
        run_dir,
        run_id,
        extension,
        JailgunHealthConfig::from_env(),
        jailgun_bridge_command(),
        output_guard,
    )
}

pub(crate) fn run_jailgun_artifact_smoke_with_config(
    run_dir: &Path,
    run_id: &str,
    extension: &str,
    config: JailgunHealthConfig,
    bridge_cmd: JailgunBridgeCommand,
    output_guard: Option<&OutputPathGuard>,
) -> Result<Value> {
    if let Some(guard) = output_guard {
        guard.check()?;
    }
    let call_id = "preflight-jailgun-artifact-smoke";
    let call_dir = run_dir.join("preflight").join(call_id);
    fs::create_dir_all(&call_dir)?;
    let download_target_name =
        jailgun_download_target_name_with_extension(run_id, call_id, extension);
    let prompt_path = call_dir.join("prompt.md");
    let receipt_path = call_dir.join("receipt.json");
    let stdout_path = call_dir.join("raw-stdout.txt");
    let stderr_path = call_dir.join("raw-stderr.txt");
    let prompt = format!(
        "# Jailgun Artifact Smoke\n\nCreate a fresh downloadable artifact named exactly `{download_target_name}` now. The filename and extension are authoritative. Do not answer with prose outside the artifact. Do not recover or reuse an artifact from another conversation. Keep the content short and valid for the requested file type.\n"
    );
    write_markdown(&prompt_path, &prompt)?;
    if let Some(guard) = output_guard {
        guard.check()?;
    }
    let started_at = now_iso8601();
    let mut attempts = Vec::new();
    let mut final_attempt = None;
    for attempt_index in 1..=JAILGUN_ARTIFACT_SMOKE_ATTEMPTS {
        let attempt = run_jailgun_live_call_attempt_with_config(
            call_id,
            &prompt_path,
            &download_target_name,
            JAILGUN_ARTIFACT_SMOKE_TIMEOUT_SECONDS,
            attempt_index,
            &started_at,
            config.clone(),
            bridge_cmd.clone(),
        );
        let attempt_stdout_path = call_dir.join(format!("raw-stdout-attempt-{attempt_index}.txt"));
        let attempt_stderr_path = call_dir.join(format!("raw-stderr-attempt-{attempt_index}.txt"));
        fs::write(&attempt_stdout_path, &attempt.stdout)?;
        fs::write(&attempt_stderr_path, &attempt.stderr)?;
        let attempt_record = json!({
            "attempt": attempt_index,
            "status": attempt.status,
            "exit_code": attempt.exit_code,
            "elapsed_seconds": round6(attempt.elapsed_seconds),
            "timeout_seconds": JAILGUN_ARTIFACT_SMOKE_TIMEOUT_SECONDS,
            "raw_stdout_path": attempt_stdout_path.display().to_string(),
            "raw_stderr_path": attempt_stderr_path.display().to_string(),
            "error": attempt.error,
        });
        attempts.push(attempt_record);
        let status = attempt.status.clone();
        final_attempt = Some(attempt);
        if status == "ok" {
            break;
        }
    }
    let attempt = final_attempt.expect("artifact smoke attempts must be configured");
    if let Some(guard) = output_guard {
        guard.check()?;
    }
    fs::write(&stdout_path, &attempt.stdout)?;
    fs::write(&stderr_path, &attempt.stderr)?;
    let mut receipt = json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "jailgun_artifact_smoke",
        "run_id": run_id,
        "call_id": call_id,
        "status": attempt.status,
        "exit_code": attempt.exit_code,
        "started_at": started_at,
        "elapsed_seconds": round6(attempt.elapsed_seconds),
        "timeout_seconds": JAILGUN_ARTIFACT_SMOKE_TIMEOUT_SECONDS,
        "attempt_count": attempts.len(),
        "attempts": attempts,
        "prompt_path": prompt_path.display().to_string(),
        "raw_stdout_path": stdout_path.display().to_string(),
        "raw_stderr_path": stderr_path.display().to_string(),
        "receipt_path": receipt_path.display().to_string(),
        "download_target_name": download_target_name,
        "artifact_extension": safe_artifact_extension(extension).unwrap_or_else(|| "json".to_string()),
        "error": attempt.error,
    });
    merge_object(&mut receipt, &attempt.metadata);
    write_json(&receipt_path, &receipt)?;
    Ok(receipt)
}

pub(crate) fn jailgun_transport_metadata(
    config: &JailgunHealthConfig,
    status: Option<&JailgunStatus>,
) -> Value {
    let mut metadata = json!({
        "token_source": config.token_source(),
    });
    if let Some(status) = status {
        metadata["account_source"] = json!(status.account_source.as_deref());
        metadata["jailgun_health"] = status.as_json();
    }
    metadata
}

pub(crate) fn jailgun_live_run_id(call_id: &str, attempt: usize, started_at: &str) -> String {
    let suffix = short_hash(started_at, 12);
    let candidate = format!("openqg-{call_id}-a{attempt}-{suffix}");
    if candidate.len() <= 128 {
        candidate
    } else {
        format!("openqg-{}-a{attempt}-{suffix}", short_hash(call_id, 32))
    }
}

pub(crate) fn jailgun_download_target_name(
    run_id: &str,
    call_id: &str,
    stage: &StagePackage,
    purpose: &str,
    prompt: &str,
) -> String {
    let extension = jailgun_artifact_extension(stage, purpose, prompt);
    jailgun_download_target_name_with_extension(run_id, call_id, &extension)
}

pub(crate) fn jailgun_download_target_name_with_extension(
    run_id: &str,
    call_id: &str,
    extension: &str,
) -> String {
    let extension = safe_artifact_extension(extension).unwrap_or_else(|| "json".to_string());
    let stem = format!("openqg-{run_id}-{call_id}");
    let safe_stem = stem
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>();
    let candidate = format!("{safe_stem}.{extension}");
    if candidate.len() <= 128 {
        candidate
    } else {
        format!(
            "openqg-{}-{}.{}",
            short_hash(run_id, 12),
            short_hash(call_id, 32),
            extension
        )
    }
}

pub(crate) fn jailgun_artifact_extension(
    stage: &StagePackage,
    purpose: &str,
    prompt: &str,
) -> String {
    artifact_extension_from_items(&stage.outputs)
        .or_else(|| artifact_extension_from_text(prompt))
        .unwrap_or_else(|| default_jailgun_artifact_extension(purpose).to_string())
}

pub(crate) fn artifact_extension_from_items(items: &[String]) -> Option<String> {
    items
        .iter()
        .find_map(|item| safe_artifact_extension_from_path(item))
}

pub(crate) fn artifact_extension_from_text(text: &str) -> Option<String> {
    text.split(|ch: char| {
        ch.is_whitespace()
            || matches!(
                ch,
                '`' | '"' | '\'' | '(' | ')' | '[' | ']' | '{' | '}' | ',' | ';' | ':'
            )
    })
    .find_map(safe_artifact_extension_from_path)
}

pub(crate) fn safe_artifact_extension_from_path(value: &str) -> Option<String> {
    let trimmed = value.trim_matches(|ch: char| {
        matches!(
            ch,
            '`' | '"' | '\'' | '.' | ',' | ';' | ':' | ')' | ']' | '}'
        )
    });
    let extension = Path::new(trimmed).extension()?.to_str()?;
    safe_artifact_extension(extension)
}

pub(crate) fn safe_artifact_extension(extension: &str) -> Option<String> {
    let extension = extension
        .trim()
        .trim_start_matches('.')
        .to_ascii_lowercase();
    (!extension.is_empty()
        && extension.len() <= 16
        && extension.chars().all(|ch| ch.is_ascii_alphanumeric()))
    .then_some(extension)
}

pub(crate) fn default_jailgun_artifact_extension(_purpose: &str) -> &'static str {
    "json"
}

pub(crate) fn jailgun_single_account_ids(
    account_ids: &[String],
    call_id: &str,
    attempt: usize,
) -> Vec<String> {
    let _ = (call_id, attempt);
    account_ids.iter().take(1).cloned().collect()
}

pub(crate) fn live_attempt_failure_kind(attempt: &LiveAttempt) -> Option<&str> {
    attempt
        .metadata
        .get("jailgun_failure_kind")
        .and_then(Value::as_str)
}

pub(crate) fn live_attempt_warning_kind(attempt: &LiveAttempt) -> Option<&str> {
    attempt
        .metadata
        .get("jailgun_warning_kind")
        .and_then(Value::as_str)
        .filter(|kind| *kind != "none")
}

pub(crate) fn jailgun_attempt_metadata(
    server_url: &str,
    jailgun_run_id: &str,
    account_ids: &[String],
    accepted: &Value,
    final_status: &Value,
    status_snapshots: &[Value],
    summary: Option<&Value>,
    token: &str,
) -> Value {
    let accepted = redact_jailgun_token_in_value(accepted, token);
    let final_status = redact_jailgun_token_in_value(final_status, token);
    let status_snapshots = status_snapshots
        .iter()
        .map(|snapshot| redact_jailgun_token_in_value(snapshot, token))
        .collect::<Vec<_>>();
    let summary = summary.map(|value| redact_jailgun_token_in_value(value, token));
    let summary_path = summary
        .as_ref()
        .and_then(|value| value.get("summary_json").and_then(Value::as_str))
        .or_else(|| accepted.get("summary_json").and_then(Value::as_str))
        .map(ToString::to_string);
    let events_path = summary
        .as_ref()
        .and_then(|value| value.get("events_jsonl").and_then(Value::as_str))
        .or_else(|| accepted.get("events_jsonl").and_then(Value::as_str))
        .map(ToString::to_string);
    let jailgun_status = jailgun_effective_status(summary.as_ref(), &final_status);
    let event_failure_kind = events_path
        .as_deref()
        .and_then(classify_jailgun_events_failure);
    let jailgun_warning_kind =
        if jailgun_status == "succeeded" && event_failure_kind == Some("rate-limit") {
            "rate-limit"
        } else {
            "none"
        };
    let classified_failure_kind = if event_failure_kind == Some("rate-limit") {
        event_failure_kind
    } else {
        classify_jailgun_attempt_failure(summary.as_ref(), &final_status, &status_snapshots)
            .or(event_failure_kind)
    };
    let jailgun_failure_kind = if jailgun_status == "succeeded" {
        "none"
    } else {
        classified_failure_kind.unwrap_or("none")
    };
    let receipt_paths = summary
        .as_ref()
        .and_then(|value| value.get("receipt_paths").cloned())
        .unwrap_or_else(|| json!([]));
    json!({
        "jailgun_server_url": server_url,
        "jailgun_run_id": jailgun_run_id,
        "jailgun_status": jailgun_status,
        "jailgun_account_count": account_ids.len(),
        "jailgun_failure_kind": jailgun_failure_kind,
        "jailgun_warning_kind": jailgun_warning_kind,
        "jailgun_started_response": accepted,
        "jailgun_status_snapshot": final_status,
        "jailgun_status_snapshots": status_snapshots,
        "jailgun_summary_path": summary_path,
        "jailgun_events_path": events_path,
        "jailgun_event_failure_kind": event_failure_kind,
        "jailgun_receipt_paths": receipt_paths,
        "jailgun_summary": summary,
    })
}

pub(crate) fn jailgun_effective_status(summary: Option<&Value>, final_status: &Value) -> String {
    let summary_status = summary.and_then(|value| value.get("status").and_then(Value::as_str));
    let final_status = final_status.get("status").and_then(Value::as_str);
    summary_status
        .filter(|status| jailgun_terminal_status(status))
        .or_else(|| final_status.filter(|status| jailgun_terminal_status(status)))
        .or(summary_status)
        .or(final_status)
        .unwrap_or("unknown")
        .to_string()
}

pub(crate) fn jailgun_terminal_status(status: &str) -> bool {
    matches!(status, "succeeded" | "failed" | "timed-out")
}
