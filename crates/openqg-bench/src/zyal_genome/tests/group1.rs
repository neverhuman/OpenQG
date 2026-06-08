use super::super::*;
use super::*;

#[test]
fn discovers_stage_packages() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .join("ZYAL/stages");
    let stages = load_stage_registry(&root).expect("load stage registry");
    assert_eq!(stages.len(), 11);
    assert_eq!(stages.first().unwrap().stage_id, "00-atlas");
    assert_eq!(stages.last().unwrap().stage_id, "10-promotion");
}

#[test]
fn resume_state_recovers_checkpoint_and_scores() {
    let dir = tempdir().expect("tempdir");
    let run_dir = dir.path().join("run");
    fs::create_dir_all(&run_dir).expect("create run dir");
    write_json(
        &run_dir.join("checkpoint.json"),
        &json!({
            "schema_version": SCHEMA_VERSION,
            "run_id": "run-1",
            "variant": "hybrid",
            "complete_generation": 2,
        }),
    )
    .expect("checkpoint");
    write_json(
            &run_dir.join("generation-ledger.jsonl"),
            &json!({"schema_version": SCHEMA_VERSION, "record_kind": "metrics_point", "generation_id": "g0001", "metric_name": "deterministic_rollup_score", "metric_value": 0.4, "run_id": "run-1", "variant": "hybrid", "series": "generation_rollup"}),
        )
        .expect("generation ledger json");
    let stage_registry = load_stage_registry(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .unwrap()
            .join("ZYAL/stages"),
    )
    .expect("stage registry");
    let state = load_resume_state(&run_dir, &stage_registry).expect("resume state");
    assert_eq!(state.completed_generation, 2);
    assert_eq!(
        read_completed_generation(&run_dir).expect("completed generation"),
        2
    );
}

#[test]
fn live_call_receipt_serializes() {
    let record = json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "live_call",
        "run_id": "run-1",
        "generation_id": "g0001",
        "stage_id": "03-generate-genes",
        "candidate_id": "g0001-03-generate-genes",
        "call_id": "live-g0001-03-generate-genes-research_synthesis",
        "purpose": "research_synthesis",
        "status": "ok",
        "exit_code": 0,
        "started_at": "0",
        "elapsed_seconds": 0.1,
        "timeout_seconds": 45,
        "retry_count": 0,
        "command": ["sh","-c","echo hi"],
        "prompt_path": "prompt.md",
        "retrieval_packet_path": "retrieval.json",
        "raw_output_path": "raw.txt",
        "parsed_summary_path": "summary.json",
        "receipt_path": "receipt.json",
        "token_usage": {"prompt": 1, "completion": 1, "total": 2},
        "summary": "hi",
        "error": null,
    });
    let serialized = live_call_to_receipt(&record);
    assert_eq!(serialized["record_kind"], "live_call");
    assert_eq!(serialized["token_usage"]["total"], 2);
}

#[test]
fn lineage_root_edge_uses_non_null_parent() {
    let record = lineage_edge_record(
        "run-1",
        "g0001",
        Value::Null,
        json!("g0001-00-atlas"),
        "emit_atlas".to_string(),
        "routing".to_string(),
    );
    assert_eq!(record["parent_candidate_id"], json!("root"));
    validate_record_kind(&record).expect("root lineage edge should validate");
}

#[test]
#[cfg(unix)]
fn touch_latest_replaces_broken_symlink() {
    let dir = tempdir().expect("tempdir");
    let output_root = dir.path().join("hybrid");
    let run_dir = output_root.join("runs").join("run-1");
    fs::create_dir_all(&run_dir).expect("create run dir");
    std::os::unix::fs::symlink(
        output_root.join("runs").join("missing"),
        output_root.join("latest"),
    )
    .expect("create broken latest symlink");

    touch_latest(&output_root, &run_dir).expect("replace latest symlink");

    assert_eq!(
        fs::read_link(output_root.join("latest")).expect("read latest symlink"),
        run_dir
    );
}

#[test]
fn output_guard_detects_deleted_and_recreated_root() {
    let dir = tempdir().expect("tempdir");
    let output_root = dir.path().join("target/openqg/zyal-genome/hybrid");
    fs::create_dir_all(&output_root).expect("create output root");
    let guard = OutputPathGuard::new(&output_root).expect("guard");

    fs::remove_dir_all(&output_root).expect("remove output root");
    let deleted = guard.check().expect_err("deleted root should fail");
    assert!(deleted.to_string().contains("output-root-deleted"));

    fs::create_dir_all(&output_root).expect("recreate output root");
    let stale = guard.check().expect_err("recreated root should fail");
    assert!(stale.to_string().contains("output-root-changed"));
}

#[test]
fn jsonl_writer_detects_deleted_visible_ledger_path() {
    let dir = tempdir().expect("tempdir");
    let output_root = dir.path().join("target/openqg/zyal-genome/hybrid");
    fs::create_dir_all(&output_root).expect("create output root");
    let guard = OutputPathGuard::new(&output_root).expect("guard");
    let path = output_root.join("runs/run-1/live-call-ledger.jsonl");
    let mut writer = JsonlWriter::open_guarded(&path, false, &guard).expect("writer");

    writer.write(&json!({"ok": true})).expect("first write");
    fs::remove_file(&path).expect("remove visible ledger");

    let error = writer
        .write(&json!({"ok": false}))
        .expect_err("stale ledger");
    assert!(error.to_string().contains("output-root-changed"));
}

#[test]
fn live_call_prechecks_output_guard_before_provider_spend() {
    let dir = tempdir().expect("tempdir");
    let output_root = dir.path().join("target/openqg/zyal-genome/hybrid");
    let run_dir = output_root.join("runs/run-1");
    fs::create_dir_all(&run_dir).expect("create run dir");
    let guard = OutputPathGuard::new(&output_root).expect("guard");
    fs::remove_dir_all(&output_root).expect("remove output root");

    let stage = test_stage("02-decompose-failed", "hard", Vec::new());
    let route = RoutePolicy {
        route_backend: "jailgun".to_string(),
        route_tier: "top20_pct".to_string(),
        router_state: "nominal".to_string(),
        judge_family: "jailgun".to_string(),
        provenance: "test".to_string(),
        route_policy: json!({}),
    };
    let error = run_live_call(
        &run_dir,
        &stage,
        &route,
        "g0001",
        "g0001-02-decompose-failed",
        "hard_stage_repair",
        &test_live_config(),
        &[],
        Some(&guard),
    )
    .expect_err("stale output root should fail before provider attempt");

    assert!(error.to_string().contains("output-root-deleted"));
    assert!(!run_dir
        .join("stages/02-decompose-failed/generations/g0001/live-calls")
        .exists());
}

#[test]
fn stage_ledger_root_parent_uses_non_null_generation() {
    let route = RoutePolicy {
        route_backend: "jnoccio".to_string(),
        route_tier: "standard".to_string(),
        router_state: "nominal".to_string(),
        judge_family: "jnoccio".to_string(),
        provenance: "test".to_string(),
        route_policy: json!({}),
    };
    let record = stage_ledger_record(
        "run-1",
        "hybrid",
        "g0001",
        "00-atlas",
        "g0001-00-atlas",
        "Atlas intake",
        &route,
        Some("emit_atlas".to_string()),
        None,
        0.1,
        1.0,
        0.1,
        Vec::new(),
        json!({}),
        &json!({}),
    );
    assert_eq!(record["parent_generation_id"], json!("root"));
    validate_record_kind(&record).expect("root stage ledger should validate");
}

#[test]
fn plot_index_is_written_for_run_dir() {
    let dir = tempdir().expect("tempdir");
    let run_dir = dir.path().join("run");
    fs::create_dir_all(&run_dir).expect("create run dir");
    write_json(
            &run_dir.join("run-summary.json"),
            &json!({
                "schema_version": SCHEMA_VERSION,
                "record_kind": "run_summary",
                "run_id": "run-1",
                "variant": "hybrid",
                "generation_count": 1,
                "stage_count": 1,
                "candidate_count": 1,
                "best_score_seen": 0.5,
                "rolling_5_median": 0.5,
                "best_nonregressive_delta": 0.1,
                "unique_contributions": 1,
                "decoy_failures": 0,
                "regression_rate": 0.0,
                "throughput": 1.0,
                "fail_stop_rate": 0.0,
                "score_blend": {"local_score":0.2,"interface_score":0.2,"macro_score":0.2,"innovation_score":0.2,"novelty_score":0.1,"failure_penalty":0.1},
                "router_state": "nominal",
                "degraded_router": false,
                "lineage": {"acyclic": true, "missing_parent_ids": 0},
                "pareto_snapshot": {"frontier_size": 1, "points": []},
            }),
        )
        .expect("summary");
    let path = emit_run_plot_index(&run_dir).expect("plot index");
    assert!(path.exists());
    let index: Value = serde_json::from_str(&fs::read_to_string(path).expect("read plot index"))
        .expect("parse plot index");
    assert_eq!(index["record_kind"], "plot_index");
    assert_eq!(index["summary"]["best_score_seen"], 0.5);
}

#[test]
fn quality_gate_passes_on_healthy_smoke_fixture() {
    let dir = tempdir().expect("tempdir");
    let run_dir = dir
        .path()
        .join("target/openqg/zyal-genome/hybrid/runs/healthy");
    write_healthy_gate_fixture(&run_dir, 10);
    quality_gate(&run_dir).expect("quality gate");
    let report: Value = serde_json::from_str(
        &fs::read_to_string(run_dir.join("quality-gate.json")).expect("read quality gate"),
    )
    .expect("parse quality gate");
    assert_eq!(report["passed"], json!(true));
}

#[test]
fn hard_stage_hybrid_live_route_uses_jailgun_mcp() {
    let stage = StagePackage {
        stage_id: "02-decompose-failed".to_string(),
        name: "Decompose failed stage".to_string(),
        track: "failure-repair".to_string(),
        family: "hard".to_string(),
        purpose: "repair hard failures".to_string(),
        inputs: vec!["input".to_string()],
        outputs: vec!["output".to_string()],
        required_evidence: vec!["evidence".to_string()],
        validation_checks: vec!["check".to_string()],
        mutation_op: "failure_mode_invert".to_string(),
        prompt_path: PathBuf::from("prompt.md"),
        memory_path: PathBuf::from("memory.yml"),
        score_path: PathBuf::from("score.yml"),
        stage_dir: PathBuf::from("stage"),
        stage_file: PathBuf::from("stage/stage.yml"),
        prompt_hash: "hash".to_string(),
        memory: json!({}),
        score_model: json!({}),
    };

    let route = route_for_variant(&GenomeVariant::Hybrid, &stage, true, true);

    assert_eq!(route.route_backend, "jailgun");
    assert_eq!(live_execution_backend(&route), "jailgun_mcp");
}

#[test]
fn quality_gate_fails_required_hard_stage_without_jailgun_proof() {
    let dir = tempdir().expect("tempdir");
    let run_dir = dir
        .path()
        .join("target/openqg/zyal-genome/hybrid/runs/missing-proof");
    write_healthy_gate_fixture(&run_dir, 100);
    let mut records =
        read_jsonl::<Value>(&run_dir.join("live-call-ledger.jsonl")).expect("live records");
    records[0]["execution_backend"] = json!("jekko_command");
    records[0]["jailgun_run_id"] = json!(null);
    write_jsonl_records(&run_dir.join("live-call-ledger.jsonl"), &records);

    let report = build_quality_gate_report(&run_dir).expect("quality report");

    assert_eq!(report["passed"], json!(false));
    assert_eq!(
        report["metrics"]["hard_stage_jailgun_proof_missing"],
        json!(1)
    );
}

#[test]
fn quality_gate_passes_healthy_hybrid_qualification_on_champion_series() {
    let dir = tempdir().expect("tempdir");
    let run_dir = dir
        .path()
        .join("target/openqg/zyal-genome/hybrid/runs/healthy-qualification");
    write_healthy_gate_fixture(&run_dir, 100);

    quality_gate(&run_dir).expect("quality gate");
    let report: Value = serde_json::from_str(
        &fs::read_to_string(run_dir.join("quality-gate.json")).expect("read quality gate"),
    )
    .expect("parse quality gate");

    assert_eq!(report["passed"], json!(true));
    assert_eq!(report["metrics"]["stage_rollup_median"], json!(0.82));
    assert_eq!(report["metrics"]["hard_stage_jailgun_live_calls"], json!(1));
}

#[test]
fn quality_gate_prefers_champion_series_for_mixed_rollup_regression() {
    let dir = tempdir().expect("tempdir");
    let run_dir = dir
        .path()
        .join("target/openqg/zyal-genome/hybrid/runs/hybrid-v2-50");
    write_healthy_gate_fixture(&run_dir, 50);
    let mut records =
        read_jsonl::<Value>(&run_dir.join("generation-ledger.jsonl")).expect("ledger");
    for record in &mut records {
        if record.get("metric_name").and_then(Value::as_str) == Some("deterministic_rollup_score") {
            let generation = generation_index_from_id(
                record
                    .get("generation_id")
                    .and_then(Value::as_str)
                    .unwrap_or("g0000"),
            );
            record["metric_value"] = if generation % 2 == 0 {
                json!(0.90)
            } else {
                json!(0.40)
            };
        }
    }
    write_jsonl_records(&run_dir.join("generation-ledger.jsonl"), &records);

    quality_gate(&run_dir).expect("quality gate");
    let report: Value = serde_json::from_str(
        &fs::read_to_string(run_dir.join("quality-gate.json")).expect("read quality gate"),
    )
    .expect("parse quality gate");

    assert_eq!(report["tier"], json!("qualification"));
    assert_eq!(report["passed"], json!(true));
    assert_eq!(report["metrics"]["stage_rollup_median"], json!(0.82));
    assert_eq!(report["metrics"]["regression_rate"], json!(0.0));
}

#[test]
fn quality_gate_fails_degraded_route() {
    let dir = tempdir().expect("tempdir");
    let run_dir = dir
        .path()
        .join("target/openqg/zyal-genome/hybrid/runs/degraded");
    write_healthy_gate_fixture(&run_dir, 10);
    let mut summary: Value = serde_json::from_str(
        &fs::read_to_string(run_dir.join("run-summary.json")).expect("read summary"),
    )
    .expect("parse summary");
    summary["degraded_router"] = json!(true);
    summary["degraded_route_count"] = json!(1);
    write_json(&run_dir.join("run-summary.json"), &summary).expect("write summary");
    let report = build_quality_gate_report(&run_dir).expect("quality report");
    assert_eq!(report["passed"], json!(false));
    assert_eq!(report["metrics"]["degraded_route_count"], json!(1));
}
