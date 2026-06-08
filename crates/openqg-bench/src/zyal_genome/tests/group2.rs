use super::super::*;
use super::*;

#[test]
fn jailgun_token_resolution_prefers_env_then_proc() {
    let mut env_values = BTreeMap::new();
    env_values.insert("JAILGUN_TOKEN".to_string(), "fallback-token".to_string());
    let token = jailgun_token_from_env_lookup(|name| env_values.get(name).cloned()).expect("token");
    assert_eq!(token.value, "fallback-token");
    assert_eq!(token.source, "env:JAILGUN_TOKEN");

    env_values.insert(
        "JAILGUN_INGEST_TOKEN".to_string(),
        "ingest-token".to_string(),
    );
    let token = jailgun_token_from_env_lookup(|name| env_values.get(name).cloned()).expect("token");
    assert_eq!(token.value, "ingest-token");
    assert_eq!(token.source, "env:JAILGUN_INGEST_TOKEN");

    let entries = vec![
        JailgunProcEntry {
            cmdline: vec![
                "/home/ubuntu/jailgun/target/debug/jailgun".to_string(),
                "serve".to_string(),
                "--addr".to_string(),
                "127.0.0.1:8797".to_string(),
            ],
            environ: vec!["JAILGUN_INGEST_TOKEN=proc-token".to_string()],
        },
        JailgunProcEntry {
            cmdline: vec![
                "/home/ubuntu/jailgun/target/debug/jailgun".to_string(),
                "serve".to_string(),
                "--addr".to_string(),
                "127.0.0.1:9999".to_string(),
            ],
            environ: vec!["JAILGUN_INGEST_TOKEN=wrong-proc-token".to_string()],
        },
    ];
    let token =
        jailgun_token_from_proc_entries(&entries, DEFAULT_JAILGUN_SERVER_URL).expect("token");
    assert_eq!(token.value, "proc-token");
    assert_eq!(token.source, "proc:JAILGUN_INGEST_TOKEN");
}

#[test]
fn jailgun_proc_cmdline_and_environ_parsing_is_nul_safe() {
    let cmdline = proc_nul_strings(
        b"/home/ubuntu/jailgun/target/debug/jailgun\0serve\0--addr=127.0.0.1:8797\0",
    );
    let environ = proc_nul_strings(b"PATH=/bin\0JAILGUN_INGEST_TOKEN=proc-secret\0");
    assert!(jailgun_process_matches_server(
        &cmdline,
        DEFAULT_JAILGUN_SERVER_URL
    ));
    let token = jailgun_token_from_proc_entries(
        &[JailgunProcEntry { cmdline, environ }],
        DEFAULT_JAILGUN_SERVER_URL,
    )
    .expect("token");
    assert_eq!(token.value, "proc-secret");
    assert_eq!(token.source, "proc:JAILGUN_INGEST_TOKEN");
}

#[test]
fn jailgun_bridge_command_prefers_env_then_chrome_bridge_default() {
    let default = jailgun_bridge_command_from_env_lookup_with_default(|_| None, &["chrome-bridge"]);
    assert_eq!(default.args, vec!["chrome-bridge"]);
    assert_eq!(default.source, "default:chrome-bridge");

    let configured = jailgun_bridge_command_from_env_lookup_with_default(
        |name| {
            (name == "JAILGUN_BRIDGE_CMD")
                .then(|| "codex exec --sandbox danger-full-access -".to_string())
        },
        &["codex", "exec", "-"],
    );
    assert_eq!(
        configured.args,
        vec!["codex", "exec", "--sandbox", "danger-full-access", "-"]
    );
    assert_eq!(configured.source, "env:JAILGUN_BRIDGE_CMD");
}

#[test]
fn jailgun_run_arguments_use_one_account_fresh_artifact_and_no_source_archive() {
    let bridge_cmd = JailgunBridgeCommand {
        args: vec!["/home/ubuntu/jailgun/apps/chrome-bridge/bin/chrome-bridge.mjs".to_string()],
        source: "default:chrome-bridge".to_string(),
    };
    let account_ids = vec!["acct-a".to_string(), "acct-b".to_string()];
    let call_id = "live-g0001-02-decompose-failed-hard_stage_repair";
    let selected_account_ids = jailgun_single_account_ids(&account_ids, call_id, 1);
    let download_target_name =
        jailgun_download_target_name_with_extension("hybrid-v2-10", call_id, "json");
    let args = jailgun_run_arguments(
        "run-1",
        call_id,
        Path::new("prompt.md"),
        120,
        &selected_account_ids,
        &bridge_cmd,
        &download_target_name,
    );

    assert_eq!(
        download_target_name,
        "openqg-hybrid-v2-10-live-g0001-02-decompose-failed-hard_stage_repair.json"
    );
    assert_eq!(selected_account_ids.len(), 1);
    assert!(account_ids.contains(&selected_account_ids[0]));
    let retry_account_ids = jailgun_single_account_ids(&account_ids, call_id, 2);
    assert_eq!(retry_account_ids.len(), 1);
    assert!(account_ids.contains(&retry_account_ids[0]));
    assert_eq!(selected_account_ids, retry_account_ids);
    assert_eq!(
        args["prompt_file"],
        json!(env::current_dir()
            .unwrap()
            .join("prompt.md")
            .display()
            .to_string())
    );
    assert_eq!(args["tabs"], json!(1));
    assert_eq!(args["source_archive"]["enabled"], json!(false));
    assert_eq!(args["browser"]["account_ids"], json!(selected_account_ids));
    assert_eq!(
        args["browser"]["queue_timeout_seconds"],
        json!(JAILGUN_QUEUE_TIMEOUT_SECONDS)
    );
    assert_eq!(
        args["browser"]["bridge_cmd"],
        json!(["/home/ubuntu/jailgun/apps/chrome-bridge/bin/chrome-bridge.mjs"])
    );
    assert_eq!(
        args["browser"]["bridge_env"]["JAILGUN_ARTIFACT_CONVERSATION_RECOVERY_LIMIT"],
        json!("0")
    );
    assert_eq!(
        args["browser"]["bridge_env"]["JAILGUN_ARTIFACT_REPAIR_ATTEMPTS"],
        json!("1")
    );
    assert_eq!(
        args["browser"]["download_target_name"],
        json!(download_target_name)
    );
    assert_eq!(args.get("bridge_cmd"), None);
}

#[test]
fn jailgun_artifact_target_name_is_extension_agnostic() {
    let call_id = "live-g0001-02-decompose-failed-hard_stage_repair";
    assert_eq!(
        jailgun_download_target_name_with_extension("hybrid-v2-10", call_id, "md"),
        "openqg-hybrid-v2-10-live-g0001-02-decompose-failed-hard_stage_repair.md"
    );
    assert_eq!(
        jailgun_download_target_name_with_extension("hybrid-v2-10", call_id, ".csv"),
        "openqg-hybrid-v2-10-live-g0001-02-decompose-failed-hard_stage_repair.csv"
    );
    let stage = test_stage(
        "02-decompose-failed",
        "hard",
        vec!["repair.jsonl".to_string()],
    );
    assert_eq!(
        jailgun_download_target_name(
            "hybrid-v2-10",
            call_id,
            &stage,
            "hard_stage_repair",
            "stage prompt"
        ),
        "openqg-hybrid-v2-10-live-g0001-02-decompose-failed-hard_stage_repair.jsonl"
    );
    let stage = test_stage("02-decompose-failed", "hard", Vec::new());
    assert_eq!(
        jailgun_download_target_name(
            "hybrid-v2-10",
            call_id,
            &stage,
            "hard_stage_repair",
            "Please write `repair-notes.md`."
        ),
        "openqg-hybrid-v2-10-live-g0001-02-decompose-failed-hard_stage_repair.md"
    );
}

#[test]
fn jailgun_live_prompt_requires_exact_fresh_artifact_name() {
    let stage = test_stage("02-decompose-failed", "hard", Vec::new());
    let prompt = live_prompt(
        &stage,
        "Repair hard failures.",
        &json!({"purpose": "hard_stage_repair"}),
        Some("openqg-hybrid-v2-10-live-g0001.md"),
    )
    .expect("live prompt");

    assert!(prompt.contains(
            "Create a fresh downloadable artifact named exactly `openqg-hybrid-v2-10-live-g0001.md` now"
        ));
    assert!(prompt.contains("The filename and extension are authoritative"));
    assert!(!prompt.contains(".tex artifact"));
    assert!(prompt
        .contains("Do not answer with a plan, acknowledgement, or prose outside the artifact"));
    assert!(prompt.contains("Do not recover or reuse an artifact from another conversation"));
}

#[test]
fn live_retry_count_applies_to_promotion_judging() {
    let mut config = test_live_config();
    config.retry_count = 1;

    assert_eq!(
        live_max_attempts(&config, "promotion_judging", "jekko_command"),
        2
    );
    assert_eq!(
        live_max_attempts(&config, "hard_stage_repair", "jailgun_mcp"),
        3
    );
}

#[test]
fn jailgun_failure_classifier_labels_known_failure_modes() {
    let cases = [
        ("tar-validation failed for source archive", "tar-validation"),
        (
            "invalid gzip header while reading payload",
            "invalid-archive-header",
        ),
        (
            "run timed out after max_runtime_seconds elapsed",
            "runtime-timeout",
        ),
        (
            "artifact conversation recovery returned a recovered artifact",
            "outdated-artifact-recovery",
        ),
        ("HTTP 429 Too Many Requests from provider", "rate-limit"),
    ];

    for (message, expected) in cases {
        assert_eq!(classify_jailgun_failure(message), expected);
    }
    assert_eq!(
        classify_jailgun_summary_failure(&json!({
            "status": "failed",
            "failures": [{"kind": "tar-validation"}],
        })),
        Some("tar-validation")
    );
    assert_eq!(
        classify_jailgun_summary_failure(&json!({"status": "timed-out"})),
        Some("runtime-timeout")
    );
    assert_eq!(
        classify_jailgun_status_failure(&json!({
            "status": "failed",
            "tabs": [{"status": "error"}],
        })),
        Some("browser-tab-error")
    );
    assert_eq!(
        jailgun_effective_status(
            Some(&json!({"status": "running"})),
            &json!({"status": "failed", "tabs": [{"status": "error"}]})
        ),
        "failed"
    );
    assert_eq!(
        classify_jailgun_attempt_failure(
            Some(&json!({"status": "running"})),
            &json!({"status": "failed", "tabs": [{"status": "error"}]}),
            &[]
        ),
        Some("browser-tab-error")
    );
    assert_eq!(
        classify_jailgun_events_text(
            r#"{"kind":"rate-limit-detected","message":"Too many requests"}"#
        ),
        Some("rate-limit")
    );
    assert_eq!(
        classify_jailgun_events_text(
            r#"{"kind":"error","message":"assistant finished but no .tex artifact download candidate was found"}"#
        ),
        Some("artifact-download-missing")
    );
}

#[test]
fn jailgun_rate_limit_events_are_warnings_on_success_and_failures_on_failure() {
    let dir = tempdir().expect("tempdir");
    let events_path = dir.path().join("agent-events.jsonl");
    fs::write(
        &events_path,
        r#"{"kind":"rate-limit-detected","message":"HTTP 429 Too Many Requests"}"#,
    )
    .expect("events");

    let success = jailgun_attempt_metadata(
        "http://127.0.0.1:8797",
        "run-success",
        &["acct-a".to_string()],
        &json!({"summary_json": null, "events_jsonl": events_path.display().to_string()}),
        &json!({"status": "succeeded"}),
        &[],
        Some(&json!({
            "status": "succeeded",
            "events_jsonl": events_path.display().to_string(),
            "receipt_paths": []
        })),
        "token",
    );
    assert_eq!(success["jailgun_status"], json!("succeeded"));
    assert_eq!(success["jailgun_warning_kind"], json!("rate-limit"));
    assert_eq!(success["jailgun_failure_kind"], json!("none"));

    let failed = jailgun_attempt_metadata(
        "http://127.0.0.1:8797",
        "run-failed",
        &["acct-a".to_string()],
        &json!({"summary_json": null, "events_jsonl": events_path.display().to_string()}),
        &json!({"status": "failed"}),
        &[],
        Some(&json!({
            "status": "failed",
            "events_jsonl": events_path.display().to_string(),
            "receipt_paths": []
        })),
        "token",
    );
    assert_eq!(failed["jailgun_status"], json!("failed"));
    assert_eq!(failed["jailgun_warning_kind"], json!("none"));
    assert_eq!(failed["jailgun_failure_kind"], json!("rate-limit"));
}
