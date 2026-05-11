use super::envelope::ParsedEnvelope;
use super::yaml::{expect_mapping, expect_mapping_field, expect_sequence, expect_string_field};
use crate::ZyalPreview;
use serde_yaml::Value;
use std::path::Path;

pub(super) fn build_preview(
    path: &Path,
    envelope: &ParsedEnvelope,
    value: &Value,
    warnings: Vec<String>,
) -> ZyalPreview {
    let mapping =
        expect_mapping(value, "ZYAL document").expect("validated document must be a mapping");
    let job = expect_mapping_field(mapping, "job").expect("validated document must contain job");
    let job_name =
        expect_string_field(job, "name", None).expect("validated document must contain job.name");
    let permission_summary =
        summarize_permissions(mapping.get(&Value::String("permissions".into())));
    let stop_summary = summarize_stop(mapping.get(&Value::String("stop".into())));
    let research_enabled = mapping.contains_key(&Value::String("research".into()));

    ZyalPreview {
        file: path.to_string_lossy().to_string(),
        id: envelope.id.clone(),
        name: job_name.to_string(),
        job_name: job_name.to_string(),
        armed: envelope.armed,
        research_enabled,
        permission_summary,
        stop_summary,
        valid: true,
        strict_valid: true,
        warnings,
    }
}

fn summarize_permissions(value: Option<&Value>) -> String {
    let Some(value) = value else {
        return "(none)".to_string();
    };
    let Ok(mapping) = expect_mapping(value, "permissions") else {
        return "(invalid)".to_string();
    };
    let mut entries = Vec::new();
    for (key, permission) in mapping {
        if let Some(key) = key.as_str() {
            entries.push(format!(
                "{key}:{}",
                super::yaml::scalar_to_string(permission)
            ));
        }
    }
    if entries.is_empty() {
        "(none)".to_string()
    } else {
        entries.join(", ")
    }
}

fn summarize_stop(value: Option<&Value>) -> String {
    let Some(value) = value else {
        return "(missing stop)".to_string();
    };
    let Ok(mapping) = expect_mapping(value, "stop") else {
        return "(invalid stop)".to_string();
    };
    let Some(all) = mapping.get(&Value::String("all".into())) else {
        return "(missing stop.all)".to_string();
    };
    let Ok(all) = expect_sequence(all, "stop.all") else {
        return "(invalid stop.all)".to_string();
    };
    let mut entries = Vec::new();
    for condition in all {
        if let Some(summary) = stop_condition_summary(condition) {
            entries.push(summary);
        }
    }
    if entries.is_empty() {
        "(empty)".to_string()
    } else {
        entries.join(" && ")
    }
}

fn stop_condition_summary(value: &Value) -> Option<String> {
    let mapping = expect_mapping(value, "stop.condition").ok()?;
    if let Some(shell) = mapping.get(&Value::String("shell".into())) {
        let shell = expect_mapping(shell, "stop.condition.shell").ok()?;
        let command = shell
            .get(&Value::String("command".into()))?
            .as_str()?
            .to_string();
        return Some(format!("shell:{command}"));
    }
    if let Some(git_clean) = mapping.get(&Value::String("git_clean".into())) {
        let git_clean = expect_mapping(git_clean, "stop.condition.git_clean").ok()?;
        let allow_untracked = git_clean
            .get(&Value::String("allow_untracked".into()))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        return Some(format!("git_clean:allow_untracked={allow_untracked}"));
    }
    None
}
