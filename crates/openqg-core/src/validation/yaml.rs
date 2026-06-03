use openqg_domain::{agent_error, Result};
use serde_yaml::{Mapping, Sequence, Value};

pub(super) const TOP_LEVEL_KEYS: &[&str] = &[
    "version",
    "intent",
    "confirm",
    "id",
    "job",
    "loop",
    "stop",
    "context",
    "checkpoint",
    "tasks",
    "incubator",
    "agents",
    "mcp",
    "permissions",
    "ui",
    "on",
    "fan_out",
    "guardrails",
    "assertions",
    "retry",
    "hooks",
    "constraints",
    "workflow",
    "memory",
    "evidence",
    "approvals",
    "skills",
    "sandbox",
    "security",
    "observability",
    "arming",
    "capabilities",
    "quality",
    "experiments",
    "models",
    "budgets",
    "triggers",
    "rollback",
    "done",
    "repo_intelligence",
    "fleet",
    "research",
    "taint",
    "interaction",
    "interop",
    "runtime",
    "capability_negotiation",
    "memory_kernel",
    "evidence_graph",
    "trust",
    "requirements",
    "evaluation",
    "release",
    "roles",
    "channels",
    "imports",
    "reasoning_privacy",
    "unsupported_feature_policy",
];

pub(super) fn assert_keys(path: &str, value: &Mapping, allowed: &[&str]) -> Result<()> {
    for key in value.keys() {
        let Some(key) = key.as_str() else {
            continue;
        };
        if !allowed.contains(&key) {
            let rendered = if path == "ui" && key == "mode" {
                "Unknown ZYAL key: ui.mode".to_string()
            } else {
                format!("Unknown ZYAL key: {path}.{key}")
            };
            return Err(agent_error(
                "validate zyal document",
                rendered,
                vec![
                    "remove the unknown key".into(),
                    "compare against an existing runbook".into(),
                ],
                "docs/testing.md",
                "rerun just zyal-validate after removing the unsupported key",
            ));
        }
    }
    Ok(())
}

pub(super) fn expect_mapping<'a>(value: &'a Value, path: &str) -> Result<&'a Mapping> {
    match value.as_mapping() {
        Some(mapping) => Ok(mapping),
        None => Err(agent_error(
            "validate zyal document",
            format!("{path} must be a YAML mapping"),
            vec![
                "use a top-level mapping".into(),
                "restore the document structure".into(),
            ],
            "docs/testing.md",
            "reformat the document as a YAML mapping",
        )),
    }
}

pub(super) fn expect_mapping_field<'a>(mapping: &'a Mapping, key: &str) -> Result<&'a Mapping> {
    let value = require_field(mapping, key)?;
    expect_mapping(value, key)
}

pub(super) fn expect_sequence_field<'a>(mapping: &'a Mapping, key: &str) -> Result<&'a Sequence> {
    let value = require_field(mapping, key)?;
    expect_sequence(value, key)
}

pub(super) fn expect_sequence<'a>(value: &'a Value, path: &str) -> Result<&'a Sequence> {
    match value.as_sequence() {
        Some(sequence) => Ok(sequence),
        None => Err(agent_error(
            "validate zyal document",
            format!("{path} must be a YAML list"),
            vec!["use a YAML list at this path".into()],
            "docs/testing.md",
            "reformat the value as a YAML list",
        )),
    }
}

pub(super) fn expect_string_field(
    mapping: &Mapping,
    key: &str,
    expected: Option<&str>,
) -> Result<String> {
    let value = require_field(mapping, key)?;
    let value = expect_string(value, key)?.to_string();
    if let Some(expected) = expected {
        if value != expected {
            return Err(agent_error(
                "validate zyal document",
                format!("{key} must be {expected}"),
                vec![
                    format!("set {key} to {expected}"),
                    "compare against an existing runbook".into(),
                ],
                "docs/zyal-research-loops.md",
                format!("rewrite the value to {expected}"),
            ));
        }
    }
    Ok(value)
}

pub(super) fn expect_string<'a>(value: &'a Value, path: &str) -> Result<&'a str> {
    match value.as_str() {
        Some(text) => Ok(text),
        None => Err(agent_error(
            "validate zyal document",
            format!("{path} must be a string"),
            vec!["change the value to a string".into()],
            "docs/testing.md",
            "rewrite the value as a string",
        )),
    }
}

pub(super) fn scalar_to_string(value: &Value) -> String {
    match value {
        Value::Bool(v) => v.to_string(),
        Value::Number(v) => v.to_string(),
        Value::String(v) => v.clone(),
        Value::Null => "null".to_string(),
        other => serde_yaml::to_string(other)
            .map(|text| text.trim().to_string())
            .unwrap_or_else(|_| "(value)".to_string()),
    }
}

fn require_field<'a>(mapping: &'a Mapping, key: &str) -> Result<&'a Value> {
    match mapping.get(&Value::String(key.to_string())) {
        Some(value) => Ok(value),
        None => Err(agent_error(
            "validate zyal document",
            format!("ZYAL document missing required key: {key}"),
            vec![
                "add the missing key".into(),
                "compare against an existing runbook".into(),
            ],
            "docs/testing.md",
            "rerun just zyal-validate after restoring the required keys",
        )),
    }
}
