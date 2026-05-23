use openqg_core::{
    parse_zyal_document, preview_zyal_document, validate_zyal_layout, validate_zyal_value,
};
use serde_yaml::Value;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn preview_reports_identity_and_summaries() {
    let path = write_temp_zyal(
        "preview",
        r#"
version: v1
intent: daemon
confirm: RUN_FOREVER
job:
  name: literature-radar
  objective: find papers
stop:
  all:
    - shell:
        command: "true"
        timeout: 1s
permissions:
  research: allow
  websearch: deny
  webfetch: ask
"#,
    );

    let preview = preview_zyal_document(&path).expect("preview should validate");
    assert_eq!(preview.id, "preview");
    assert_eq!(preview.job_name, "literature-radar");
    assert!(preview.armed);
    assert!(preview.strict_valid);
    assert!(preview.permission_summary.contains("research:allow"));
    assert!(preview.stop_summary.contains("shell:true"));
}

#[test]
fn parse_rejects_vv1_sentinels() {
    let path = write_temp_zyal(
        "vv1",
        r#"
version: v1
intent: daemon
confirm: RUN_FOREVER
job:
  name: test
  objective: test
stop:
  all:
    - git_clean: {}
"#,
    );

    let text = fs::read_to_string(&path)
        .expect("temp zyal should be readable")
        .replace("v1:daemon", "vv1:daemon");
    fs::write(&path, text).expect("temp zyal should be writable");

    assert!(parse_zyal_document(&path).is_err());
}

#[test]
fn validator_rejects_stop_any() {
    let value: Value = serde_yaml::from_str(
        r#"
version: v1
intent: daemon
confirm: RUN_FOREVER
job:
  name: test
  objective: test
stop:
  all:
    - git_clean: {}
  any:
    - shell:
        command: "true"
"#,
    )
    .expect("yaml should parse");

    let err = validate_zyal_value(&value).expect_err("validator should reject stop.any");
    let message = format!("{err}");
    assert!(message.contains("stop.any"));
}

#[test]
fn validator_rejects_ui_mode() {
    let value: Value = serde_yaml::from_str(
        r#"
version: v1
intent: daemon
confirm: RUN_FOREVER
job:
  name: test
  objective: test
stop:
  all:
    - git_clean: {}
ui:
  mode: gold
"#,
    )
    .expect("yaml should parse");

    let err = validate_zyal_value(&value).expect_err("validator should reject ui.mode");
    let message = format!("{err}");
    assert!(message.contains("ui.mode"));
}

#[test]
fn validator_accepts_hero_judge_block() {
    let value: Value = serde_yaml::from_str(
        r#"
version: v1
intent: daemon
confirm: RUN_FOREVER
job:
  name: hero-judge
  objective: evolve prompts
stop:
  all:
    - git_clean: {}
hero_judge:
  generations: 1
  population:
    hero_lanes: 2
    judge_lanes: 1
    verifier_lanes: 1
    literature_lanes: 1
    red_team_lanes: 1
    max_parallel: 2
  budgets:
    model_calls: 12
    search_queries: 1
    search_pages: 2
  research:
    enabled: true
    missing_provider: skip_with_receipt
  evidence:
    - id: loops
      role: workflow_doc
      path: docs/zyal-research-loops.md
  promotion:
    min_score: 0.75
    canary_replay: true
    anti_leak: true
"#,
    )
    .expect("yaml should parse");

    validate_zyal_value(&value).expect("hero_judge block should validate");
}

#[test]
fn validator_rejects_after_new_card_checkpoint() {
    let value: Value = serde_yaml::from_str(
        r#"
version: v1
intent: daemon
confirm: RUN_FOREVER
job:
  name: test
  objective: test
stop:
  all:
    - git_clean: {}
checkpoint:
  when: after_new_card
"#,
    )
    .expect("yaml should parse");

    let err = validate_zyal_value(&value).expect_err("validator should reject after_new_card");
    let message = format!("{err}");
    assert!(message.contains("unsupported checkpoint.when value"));
}

#[test]
fn layout_rejects_retired_zyal_yaml_extension() {
    let root = unique_temp_root("retired-extension");
    let dir = root.join("agent/zyal");
    fs::create_dir_all(&dir).expect("temp zyal dir should be created");
    fs::write(dir.join("bad.zyal.yml"), "not parsed").expect("temp zyal should be written");

    let err = validate_zyal_layout(&root).expect_err("retired extension should be rejected");
    let message = format!("{err}");
    assert!(message.contains("retired ZYAL filename"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn layout_rejects_zyal_outside_agent_zyal() {
    let root = unique_temp_root("outside-root");
    let dir = root.join("retired-runbooks");
    fs::create_dir_all(&dir).expect("temp zyal dir should be created");
    fs::write(dir.join("bad.zyal"), "not parsed").expect("temp zyal should be written");

    let err = validate_zyal_layout(&root).expect_err("misplaced zyal should be rejected");
    let message = format!("{err}");
    assert!(message.contains("outside agent/zyal"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn layout_rejects_non_zyal_file_under_agent_zyal() {
    let root = unique_temp_root("non-zyal-file");
    let dir = root.join("agent/zyal");
    fs::create_dir_all(&dir).expect("temp zyal dir should be created");
    fs::write(dir.join("README.md"), "not a runbook").expect("temp file should be written");

    let err = validate_zyal_layout(&root).expect_err("non-runbook file should be rejected");
    let message = format!("{err}");
    assert!(message.contains("non-ZYAL file"));

    let _ = fs::remove_dir_all(root);
}

fn write_temp_zyal(name: &str, body: &str) -> PathBuf {
    let path = unique_temp_root(name).with_extension("zyal");
    let text = format!(
        "<<<ZYAL v1:daemon id={name}>>>\n{body}\n<<<END_ZYAL id={name}>>>\nZYAL_ARM RUN_FOREVER id={name}\n"
    );
    fs::write(&path, text).expect("temp zyal should be writable");
    path
}

fn unique_temp_root(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic")
        .as_nanos();
    std::env::temp_dir().join(format!("openqg-zyal-{name}-{unique}"))
}
