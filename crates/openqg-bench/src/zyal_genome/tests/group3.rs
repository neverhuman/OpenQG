use super::super::*;
use super::*;

#[test]
fn strict_preflight_status_fails_when_jailgun_server_is_down_even_if_flagged_available() {
    let status = resolve_jailgun_status_with_config(
        true,
        true,
        JailgunHealthConfig {
            server_url: "http://127.0.0.1:9".to_string(),
            token: Some(JailgunToken {
                value: "token".to_string(),
                source: "env:JAILGUN_INGEST_TOKEN".to_string(),
            }),
            account_override_ids: Vec::new(),
        },
    );

    assert!(!status.available);
    assert_eq!(status.checks.get("server_health"), Some(&false));
}

#[test]
fn preflight_receipt_blocks_required_missing_hard_backend() {
    let stage_registry = load_stage_registry(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .unwrap()
            .join("ZYAL/stages"),
    )
    .expect("stage registry");
    let live_config =
        resolve_live_config(false, &json!({"evaluation": {"live": {"enabled": false}}}));
    let mut jailgun_status = JailgunStatus::server_authoritative(true);
    jailgun_status.set_check("server_health", false);
    jailgun_status.push_error("Jailgun server readiness was not proven");
    jailgun_status.finish();
    let receipt = preflight_receipt(
        "hybrid-v2-50",
        "hybrid",
        &json!({"evaluation": {"quality_gates": {"require_hard_backend": true}}}),
        Path::new("ZYAL/runs/run-hybrid-1000-v2.zyal"),
        Path::new("target/openqg/zyal-genome/hybrid/runs/hybrid-v2-50"),
        &stage_registry,
        &live_config,
        true,
        false,
        &jailgun_status,
    );
    assert_eq!(receipt["status"], json!("failed"));
    assert_eq!(
        receipt["routing_decision"],
        json!("blocked_missing_hard_backend")
    );
}

#[test]
fn ready_jailgun_account_ids_accepts_ready_or_authenticated_accounts() {
    let response = json!({
        "accounts": [
            {"id": "acct-a", "status": "ready"},
            {"id": "acct-b", "status": "auth-required"},
            {"account_id": "acct-c", "authenticated": true}
        ]
    });
    let accounts = jailgun_accounts_from_response(&response);
    let ready = ready_jailgun_account_ids(&accounts);

    assert_eq!(ready, vec!["acct-a".to_string(), "acct-c".to_string()]);
}

#[test]
fn strict_jailgun_health_succeeds_against_server_accounts_and_scheduler() {
    let server_url = spawn_fake_jailgun_server(
        json!({"accounts": [{"id": "acct-ready", "status": "ready"}]}),
        json!({"status": "ready", "available_slots": 1}),
    );

    let status = strict_jailgun_status_with_config(JailgunHealthConfig {
        server_url,
        token: Some(JailgunToken {
            value: "server-token".to_string(),
            source: "env:JAILGUN_INGEST_TOKEN".to_string(),
        }),
        account_override_ids: Vec::new(),
    });

    assert!(status.available, "{:?}", status.errors);
    assert_eq!(status.ready_account_ids, vec!["acct-ready".to_string()]);
    assert_eq!(
        status.account_source.as_deref(),
        Some("server:/api/browser/accounts")
    );
    assert_eq!(
        status.token_source.as_deref(),
        Some("env:JAILGUN_INGEST_TOKEN")
    );
    assert_eq!(status.checks.get("scheduler_capacity"), Some(&true));
}

#[test]
fn strict_jailgun_health_rejects_existing_scheduler_work() {
    let server_url = spawn_fake_jailgun_server(
        json!({"accounts": [{"id": "acct-ready", "status": "ready"}]}),
        json!({"status": "ready", "available_slots": 1, "running_jobs": 1}),
    );

    let status = strict_jailgun_status_with_config(JailgunHealthConfig {
        server_url,
        token: Some(JailgunToken {
            value: "server-token".to_string(),
            source: "env:JAILGUN_INGEST_TOKEN".to_string(),
        }),
        account_override_ids: Vec::new(),
    });

    assert!(!status.available);
    assert_eq!(status.checks.get("scheduler_capacity"), Some(&false));
}

#[test]
fn jailgun_artifact_smoke_receipt_succeeds_with_generic_json_target() {
    let server_url = spawn_fake_jailgun_server(
        json!({"accounts": [{"id": "acct-ready", "status": "ready"}]}),
        json!({"status": "ready", "available_slots": 1}),
    );
    let dir = tempdir().expect("tempdir");
    let run_dir = dir
        .path()
        .join("target/openqg/zyal-genome/hybrid/runs/smoke");
    fs::create_dir_all(&run_dir).expect("run dir");

    let receipt = run_jailgun_artifact_smoke_with_config(
        &run_dir,
        "hybrid-v2-100",
        "json",
        JailgunHealthConfig {
            server_url,
            token: Some(JailgunToken {
                value: "server-token".to_string(),
                source: "env:JAILGUN_INGEST_TOKEN".to_string(),
            }),
            account_override_ids: Vec::new(),
        },
        JailgunBridgeCommand {
            args: vec!["chrome-bridge".to_string()],
            source: "test".to_string(),
        },
        None,
    )
    .expect("smoke receipt");

    assert_eq!(receipt["status"], json!("ok"));
    assert_eq!(receipt["record_kind"], json!("jailgun_artifact_smoke"));
    assert_eq!(receipt["jailgun_account_count"], json!(1));
    assert_eq!(receipt["artifact_extension"], json!("json"));
    assert_eq!(receipt["attempt_count"], json!(1));
    assert!(receipt["download_target_name"]
        .as_str()
        .unwrap()
        .ends_with(".json"));
}

#[test]
fn jailgun_artifact_smoke_receipt_records_failure() {
    let dir = tempdir().expect("tempdir");
    let run_dir = dir
        .path()
        .join("target/openqg/zyal-genome/hybrid/runs/smoke");
    fs::create_dir_all(&run_dir).expect("run dir");

    let receipt = run_jailgun_artifact_smoke_with_config(
        &run_dir,
        "hybrid-v2-100",
        "md",
        JailgunHealthConfig {
            server_url: DEFAULT_JAILGUN_SERVER_URL.to_string(),
            token: None,
            account_override_ids: Vec::new(),
        },
        JailgunBridgeCommand {
            args: vec!["chrome-bridge".to_string()],
            source: "test".to_string(),
        },
        None,
    )
    .expect("smoke receipt");

    assert_eq!(receipt["status"], json!("failed"));
    assert_eq!(receipt["artifact_extension"], json!("md"));
    assert_eq!(receipt["jailgun_failure_kind"], json!("unknown"));
    assert_eq!(
        receipt["attempt_count"],
        json!(JAILGUN_ARTIFACT_SMOKE_ATTEMPTS)
    );
}

#[test]
fn strict_jailgun_health_fails_cleanly_without_token_or_ready_accounts() {
    let missing_token = strict_jailgun_status_with_config(JailgunHealthConfig {
        server_url: DEFAULT_JAILGUN_SERVER_URL.to_string(),
        token: None,
        account_override_ids: Vec::new(),
    });
    assert!(!missing_token.available);
    assert_eq!(missing_token.checks.get("token_configured"), Some(&false));
    assert!(missing_token
        .errors
        .iter()
        .any(|error| error.contains("token is not available")));

    let server_url = spawn_fake_jailgun_server(
        json!({"accounts": [{"id": "acct-a", "status": "auth-required"}]}),
        json!({"status": "ready", "available_slots": 1}),
    );
    let no_ready_accounts = strict_jailgun_status_with_config(JailgunHealthConfig {
        server_url,
        token: Some(JailgunToken {
            value: "server-token".to_string(),
            source: "env:JAILGUN_TOKEN".to_string(),
        }),
        account_override_ids: Vec::new(),
    });
    assert!(!no_ready_accounts.available);
    assert_eq!(no_ready_accounts.checks.get("ready_accounts"), Some(&false));
    assert!(no_ready_accounts
        .errors
        .iter()
        .any(|error| error.contains("no ready/authenticated Jailgun account")));
}

#[test]
fn strict_jailgun_receipts_record_only_token_source() {
    let stage_registry = load_stage_registry(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .unwrap()
            .join("ZYAL/stages"),
    )
    .expect("stage registry");
    let live_config =
        resolve_live_config(false, &json!({"evaluation": {"live": {"enabled": false}}}));
    let token_value = "test-ingest-token-fixture";
    let server_url = spawn_fake_jailgun_server(
        json!({"accounts": [{"id": "acct-a", "status": "auth-required"}]}),
        json!({"status": "ready", "available_slots": 1}),
    );
    let jailgun_status = strict_jailgun_status_with_config(JailgunHealthConfig {
        server_url,
        token: Some(JailgunToken {
            value: token_value.to_string(),
            source: "env:JAILGUN_INGEST_TOKEN".to_string(),
        }),
        account_override_ids: Vec::new(),
    });

    let receipt = preflight_receipt(
        "hybrid-v2-10",
        "hybrid",
        &json!({"evaluation": {"quality_gates": {"require_hard_backend": true}}}),
        Path::new("ZYAL/runs/run-hybrid-1000-v2.zyal"),
        Path::new("target/openqg/zyal-genome/hybrid/runs/hybrid-v2-10"),
        &stage_registry,
        &live_config,
        true,
        false,
        &jailgun_status,
    );
    let status_text = serde_json::to_string(&jailgun_status.as_json()).expect("status json");
    let receipt_text = serde_json::to_string(&receipt).expect("receipt json");

    assert_eq!(receipt["status"], json!("failed"));
    assert_eq!(receipt["backend_health"]["jailgun"]["strict"], json!(true));
    assert!(status_text.contains("env:JAILGUN_INGEST_TOKEN"));
    assert!(receipt_text.contains("env:JAILGUN_INGEST_TOKEN"));
    assert!(!status_text.contains(token_value));
    assert!(!receipt_text.contains(token_value));
    assert_eq!(
        receipt["backend_health"]["jailgun"]["checks"]["ready_accounts"],
        json!(false)
    );
}
