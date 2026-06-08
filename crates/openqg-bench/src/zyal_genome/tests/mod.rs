use super::*;
use std::net::{TcpListener, TcpStream};
use tempfile::tempdir;

fn spawn_fake_jailgun_server(accounts: Value, scheduler: Value) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake jailgun");
    listener
        .set_nonblocking(true)
        .expect("fake jailgun nonblocking");
    let url = format!("http://{}", listener.local_addr().expect("local addr"));
    thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            match listener.accept() {
                Ok((stream, _)) => handle_fake_jailgun_connection(stream, &accounts, &scheduler),
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(_) => break,
            }
        }
    });
    url
}

fn handle_fake_jailgun_connection(mut stream: TcpStream, accounts: &Value, scheduler: &Value) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
    let mut data = Vec::new();
    let mut buffer = [0u8; 4096];
    while !data.windows(4).any(|window| window == b"\r\n\r\n") {
        match stream.read(&mut buffer) {
            Ok(0) => return,
            Ok(read) => data.extend_from_slice(&buffer[..read]),
            Err(_) => return,
        }
    }
    let Some(header_end) = data.windows(4).position(|window| window == b"\r\n\r\n") else {
        return;
    };
    let header_text = String::from_utf8_lossy(&data[..header_end]).to_string();
    let content_length = header_text
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0);
    let body_start = header_end + 4;
    while data.len() < body_start + content_length {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => data.extend_from_slice(&buffer[..read]),
            Err(_) => break,
        }
    }
    let body =
        String::from_utf8_lossy(&data[body_start..data.len().min(body_start + content_length)]);
    let first_line = header_text.lines().next().unwrap_or_default();
    let path = first_line.split_whitespace().nth(1).unwrap_or("/");
    let response = fake_jailgun_response(path, &body, accounts, scheduler);
    let response_text = serde_json::to_string(&response).expect("serialize response");
    let http = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            response_text.len(),
            response_text
        );
    let _ = stream.write_all(http.as_bytes());
}

fn fake_jailgun_response(path: &str, body: &str, accounts: &Value, scheduler: &Value) -> Value {
    if path == "/api/health" {
        return json!({"status": "ok"});
    }
    if path == "/api/browser/accounts" {
        return accounts.clone();
    }
    if path != "/mcp" {
        return json!({"error": "not found"});
    }
    let request: Value = serde_json::from_str(body).unwrap_or_else(|_| json!({}));
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    match request.get("method").and_then(Value::as_str).unwrap_or("") {
        "initialize" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {"serverInfo": {"name": "jailgun"}}
        }),
        "tools/list" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": [
                    {"name": "jailgun.auth_status"},
                    {"name": "jailgun.scheduler_status"},
                    {"name": "jailgun.run"},
                    {"name": "jailgun.run_status"},
                    {"name": "jailgun.run_summary"}
                ]
            }
        }),
        "tools/call" => {
            let name = request
                .get("params")
                .and_then(|params| params.get("name"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let content = match name {
                "jailgun.auth_status" => json!({"status": "ready"}),
                "jailgun.scheduler_status" => scheduler.clone(),
                "jailgun.run" => {
                    let run_id = request
                        .get("params")
                        .and_then(|params| params.get("arguments"))
                        .and_then(|args| args.get("run_id"))
                        .cloned()
                        .unwrap_or_else(|| json!("fake-run"));
                    json!({"status": "accepted", "run_id": run_id})
                }
                "jailgun.run_status" => json!({"status": "succeeded"}),
                "jailgun.run_summary" => json!({
                    "status": "succeeded",
                    "summary_json": null,
                    "events_jsonl": null,
                    "receipt_paths": ["artifact-smoke.json"]
                }),
                _ => json!({"status": "ok"}),
            };
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {"structuredContent": content}
            })
        }
        _ => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {"message": "unknown method"}
        }),
    }
}

fn write_jsonl_records(path: &Path, records: &[Value]) {
    let mut text = String::new();
    for record in records {
        text.push_str(&serde_json::to_string(record).expect("serialize jsonl record"));
        text.push('\n');
    }
    fs::write(path, text).expect("write jsonl");
}

fn test_stage(stage_id: &str, family: &str, outputs: Vec<String>) -> StagePackage {
    StagePackage {
        stage_id: stage_id.to_string(),
        name: "Test stage".to_string(),
        track: "test".to_string(),
        family: family.to_string(),
        purpose: "test purpose".to_string(),
        inputs: Vec::new(),
        outputs,
        required_evidence: Vec::new(),
        validation_checks: Vec::new(),
        mutation_op: "test_mutation".to_string(),
        prompt_path: PathBuf::from("missing-prompt.md"),
        memory_path: PathBuf::from("memory.yml"),
        score_path: PathBuf::from("score.yml"),
        stage_dir: PathBuf::from("stage"),
        stage_file: PathBuf::from("stage/stage.yml"),
        prompt_hash: "hash".to_string(),
        memory: json!({}),
        score_model: json!({}),
    }
}

fn test_live_config() -> LiveConfig {
    LiveConfig {
        enabled: true,
        timeout_seconds: 1,
        timeout_by_purpose: BTreeMap::new(),
        retry_count: 0,
        champion_audit_every: 1,
        hard_stage_every: 1,
        promotion_every: 1,
        research_synthesis: true,
        hard_stage_repair: true,
        promotion_judging: true,
        command: vec!["false".to_string()],
    }
}

fn selection_candidate(id: &str, mode: &str, island: &str, final_score: f64) -> Value {
    json!({
        "candidate_id": id,
        "generation_id": "g0001",
        "island": island,
        "mode": mode,
        "parent_candidate_ids": ["parent-a"],
        "lineage_depth": 2,
        "source_card_ids": ["info-a"],
        "stage_concepts": {"03-generate-genes": "concept-a"},
        "scores": {
            "final_score": final_score,
            "novelty_score": if mode == "novelty" { 0.72 } else { 0.42 },
        },
        "frontier_claim": format!("{id} claim"),
        "falsifiable_tests": ["Replay 03-generate-genes evidence"],
        "known_failure_modes": ["test_failure"],
        "review_priority": if mode == "novelty" { "high" } else { "medium" },
    })
}

fn previous_champions(count: usize, novelty_count: usize, last_score: f64) -> Vec<Value> {
    let islands = [
        "foundations",
        "coefficients",
        "observables",
        "failure-repair",
        "interface-contracts",
        "wildcards",
    ];
    (0..count)
        .map(|index| {
            let mode = if index < novelty_count {
                "novelty"
            } else {
                "exploitation"
            };
            let score = if index + 1 == count { last_score } else { 0.80 };
            let mut candidate = selection_candidate(
                &format!("prev-{index:03}"),
                mode,
                islands[index % islands.len()],
                score,
            );
            candidate["generation_id"] = json!(format!("g{:04}", index + 1));
            candidate
        })
        .collect()
}

fn write_healthy_gate_fixture(run_dir: &Path, target_generation: usize) {
    fs::create_dir_all(run_dir).expect("create run dir");
    write_json(
        &run_dir.join("checkpoint.json"),
        &json!({
            "schema_version": SCHEMA_VERSION,
            "run_id": "healthy",
            "variant": "hybrid",
            "complete_generation": target_generation,
            "target_generation": target_generation,
        }),
    )
    .expect("checkpoint");
    let champions = (1..=target_generation)
        .map(|generation| {
            json!({
                "generation_id": format!("g{generation:04}"),
                "candidate_id": format!("c{generation:04}"),
                "island": DEFAULT_ISLANDS[(generation - 1) % DEFAULT_ISLANDS.len()],
                "mode": if generation % 10 == 0 { "novelty" } else { "exploitation" },
                "final_score": 0.82,
                "novelty_score": 0.62,
            })
        })
        .collect::<Vec<_>>();
    write_json(
            &run_dir.join("run-summary.json"),
            &json!({
                "schema_version": SCHEMA_VERSION,
                "record_kind": "run_summary",
                "run_id": "healthy",
                "variant": "hybrid",
                "generation_count": target_generation,
                "stage_count": 1,
                "candidate_count": target_generation,
                "best_score_seen": 0.82,
                "rolling_5_median": 0.82,
                "best_nonregressive_delta": 0.0,
                "unique_contributions": 1,
                "decoy_failures": 0,
                "degraded_route_count": 0,
                "regression_rate": 0.0,
                "throughput": 1.0,
                "fail_stop_rate": 0.0,
                "score_blend": {"local_score":0.2,"interface_score":0.2,"macro_score":0.2,"innovation_score":0.2,"novelty_score":0.1,"failure_penalty":0.1},
                "router_state": "nominal",
                "degraded_router": false,
                "lineage": {"acyclic": true, "missing_parent_ids": 0},
                "pareto_snapshot": {"frontier_size": 1, "points": []},
                "generation_champions": champions,
            }),
        )
        .expect("summary");
    write_json(
        &run_dir.join("offline-eval.json"),
        &json!({
            "schema_version": SCHEMA_VERSION,
            "record_kind": "offline_eval",
            "run_id": "healthy",
            "variant": "hybrid",
            "scorecard": {"best_score_seen": 0.82},
            "stage_rankings": [],
            "warnings": [],
        }),
    )
    .expect("offline eval");
    write_json(
        &run_dir.join("preflight.json"),
        &json!({
            "schema_version": SCHEMA_VERSION,
            "record_kind": "preflight",
            "run_id": "healthy",
            "variant": "hybrid",
            "status": "ok",
            "backend_health": {
                "hard_backend": "jailgun",
                "hard_stage_count": 1,
                "require_hard_backend": true,
                "jailgun_available": true,
            },
            "routing_decision": "nominal",
        }),
    )
    .expect("preflight");
    write_jsonl_records(
        &run_dir.join("live-call-ledger.jsonl"),
        &[json!({
            "schema_version": SCHEMA_VERSION,
            "record_kind": "live_call",
            "run_id": "healthy",
            "generation_id": "g0001",
            "stage_id": "02-decompose-failed",
            "candidate_id": "g0001-02-decompose-failed",
            "call_id": "live-g0001-02-decompose-failed-hard_stage_repair",
            "purpose": "hard_stage_repair",
            "route_backend": "jailgun",
            "route_tier": "top20_pct",
            "router_state": "nominal",
            "execution_backend": "jailgun_mcp",
            "status": "ok",
            "exit_code": 0,
            "started_at": "0",
            "elapsed_seconds": 0.1,
            "timeout_seconds": 120,
            "configured_timeout_seconds": 120,
            "retry_count": 1,
            "attempt_count": 1,
            "attempts": [{"attempt":1,"status":"ok","elapsed_seconds":0.1,"timeout_seconds":120}],
            "command": ["sh","-c","cat"],
            "jailgun_server_url": "http://127.0.0.1:8797",
            "jailgun_run_id": "openqg-live-g0001-02-decompose-failed-hard_stage_repair-a1-test",
            "jailgun_status": "succeeded",
            "jailgun_summary_status": "succeeded",
            "jailgun_account_count": 1,
            "jailgun_summary_path": "agent-summary.json",
            "jailgun_events_path": "agent-events.jsonl",
            "jailgun_receipt_paths": [],
            "prompt_path": "prompt.md",
            "retrieval_packet_path": "retrieval.json",
            "raw_output_path": "raw.txt",
            "parsed_summary_path": "summary.json",
            "receipt_path": "receipt.json",
            "token_usage": {"prompt": 1, "completion": 1, "total": 2},
            "summary": "ok",
            "error": null,
        })],
    );
    let mut generation_records = Vec::new();
    for generation in 1..=target_generation {
        generation_records.push(json!({
                "schema_version": SCHEMA_VERSION,
                "record_kind": "metrics_point",
                "run_id": "healthy",
                "variant": "hybrid",
                "generation_id": format!("g{generation:04}"),
                "metric_name": "deterministic_rollup_score",
                "metric_value": 0.40,
                "series": "generation_rollup",
        }));
        generation_records.push(json!({
                "schema_version": SCHEMA_VERSION,
                "record_kind": "metrics_point",
                "run_id": "healthy",
                "variant": "hybrid",
                "generation_id": format!("g{generation:04}"),
                "metric_name": "hybrid_champion_score",
                "metric_value": 0.82,
                "series": "hybrid_champion",
        }));
    }
    write_jsonl_records(
        &run_dir.join("generation-ledger.jsonl"),
        &generation_records,
    );
    write_jsonl_records(&run_dir.join("run-events.jsonl"), &[]);
    write_jsonl_records(&run_dir.join("stage-ledger.jsonl"), &[]);
}

mod group1;
mod group2;
mod group3;
mod group4;
