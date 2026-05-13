use super::yaml::{
    assert_keys, expect_mapping, expect_mapping_field, expect_sequence, expect_sequence_field,
    expect_string, expect_string_field, TOP_LEVEL_KEYS,
};
use openqg_domain::{agent_error, Result};
use serde_yaml::Value;

pub fn validate_zyal_value(value: &Value) -> Result<Vec<String>> {
    let mapping = expect_mapping(value, "ZYAL document")?;
    assert_keys("ZYAL document", mapping, TOP_LEVEL_KEYS)?;

    expect_string_field(mapping, "version", Some("v1"))?;
    expect_string_field(mapping, "intent", Some("daemon"))?;
    expect_string_field(mapping, "confirm", Some("RUN_FOREVER"))?;

    let job = expect_mapping_field(mapping, "job")?;
    assert_keys("job", job, &["name", "objective", "risk"])?;
    expect_string_field(job, "name", None)?;
    expect_string_field(job, "objective", None)?;

    let stop = expect_mapping_field(mapping, "stop")?;
    assert_keys("stop", stop, &["all", "any"])?;
    if stop.contains_key(&Value::String("any".into())) {
        return Err(agent_error(
            "validate zyal document",
            "stop.any is not supported by the OpenQG validator",
            vec![
                "move every stop condition into stop.all".into(),
                "remove the stop.any block".into(),
            ],
            "docs/zyal-research-loops.md",
            "rewrite the runbook so the stop logic lives under stop.all",
        ));
    }

    let all = expect_sequence_field(stop, "all")?;
    if all.is_empty() {
        return Err(agent_error(
            "validate zyal document",
            "stop.all must contain at least one condition",
            vec![
                "add a shell or git_clean stop condition".into(),
                "compare against an existing runbook".into(),
            ],
            "docs/zyal-research-loops.md",
            "add a stop.all condition before rerunning just zyal-validate",
        ));
    }
    for (index, condition) in all.iter().enumerate() {
        validate_stop_condition(condition, &format!("stop.all[{index}]"))?;
    }

    if let Some(checkpoint) = mapping.get(&Value::String("checkpoint".into())) {
        let checkpoint = expect_mapping(checkpoint, "checkpoint")?;
        assert_keys(
            "checkpoint",
            checkpoint,
            &["when", "noop_if_clean", "verify", "git"],
        )?;
        if let Some(when) = checkpoint.get(&Value::String("when".into())) {
            let when = expect_string(when, "checkpoint.when")?;
            if !matches!(when, "after_verified_change" | "manual" | "on_error") {
                return Err(agent_error(
                    "validate zyal document",
                    format!("unsupported checkpoint.when value: {when}"),
                    vec![
                        "use after_verified_change, manual, or on_error".into(),
                        "remove checkpoint.when if the runbook does not need it".into(),
                    ],
                    "docs/zyal-research-loops.md",
                    "replace checkpoint.when with a Jekko-compatible enum value",
                ));
            }
        }
        if let Some(verify) = checkpoint.get(&Value::String("verify".into())) {
            let verify = expect_sequence(verify, "checkpoint.verify")?;
            for (index, item) in verify.iter().enumerate() {
                validate_shell_check(item, &format!("checkpoint.verify[{index}]"))?;
            }
        }
        if let Some(git) = checkpoint.get(&Value::String("git".into())) {
            let git = expect_mapping(git, "checkpoint.git")?;
            assert_keys("checkpoint.git", git, &["add", "commit_message", "push"])?;
            if let Some(push) = git.get(&Value::String("push".into())) {
                let push = expect_string(push, "checkpoint.git.push")?;
                if !matches!(push, "ask" | "allow" | "deny") {
                    return Err(agent_error(
                        "validate zyal document",
                        format!("checkpoint.git.push must be ask, allow, or deny; got {push}"),
                        vec!["set checkpoint.git.push to ask, allow, or deny".into()],
                        "docs/zyal-research-loops.md",
                        "fix the checkpoint git policy and rerun just zyal-validate",
                    ));
                }
            }
        }
    }

    if let Some(ui) = mapping.get(&Value::String("ui".into())) {
        let ui = expect_mapping(ui, "ui")?;
        assert_keys("ui", ui, &["theme", "banner"])?;
    }

    if let Some(research) = mapping.get(&Value::String("research".into())) {
        let research = expect_mapping(research, "research")?;
        assert_keys(
            "research",
            research,
            &[
                "version",
                "mode",
                "autonomy",
                "max_parallel",
                "timeout_seconds",
                "provider_policy",
                "extraction",
                "evidence",
                "safety",
                "budgets",
                "paper_scan",
                "full_text",
                "dedupe",
                "context_packing",
                "question_bank",
                "agent_trials",
                "audit",
            ],
        )?;
        expect_string_field(research, "version", Some("v1"))?;
    }

    let mut warnings = Vec::new();
    if !mapping.contains_key(&Value::String("checkpoint".into())) {
        warnings.push("checkpoint block absent".to_string());
    }
    if !mapping.contains_key(&Value::String("permissions".into())) {
        warnings.push("permissions block absent".to_string());
    }
    Ok(warnings)
}

fn validate_stop_condition(value: &Value, path: &str) -> Result<()> {
    let mapping = expect_mapping(value, path)?;
    assert_keys(path, mapping, &["shell", "git_clean"])?;
    let shell = mapping.get(&Value::String("shell".into()));
    let git_clean = mapping.get(&Value::String("git_clean".into()));
    if shell.is_some() == git_clean.is_some() {
        return Err(agent_error(
            "validate zyal document",
            format!("{path} must contain exactly one of shell or git_clean"),
            vec!["use a single stop condition shape".into()],
            "docs/zyal-research-loops.md",
            "fix the stop condition shape and rerun just zyal-validate",
        ));
    }
    if let Some(shell) = shell {
        validate_shell_check(shell, &format!("{path}.shell"))?;
    }
    if let Some(git_clean) = git_clean {
        let git_clean = expect_mapping(git_clean, &format!("{path}.git_clean"))?;
        assert_keys(
            &format!("{path}.git_clean"),
            git_clean,
            &["allow_untracked"],
        )?;
    }
    Ok(())
}

fn validate_shell_check(value: &Value, path: &str) -> Result<()> {
    let mapping = expect_mapping(value, path)?;
    assert_keys(path, mapping, &["command", "timeout", "cwd", "assert"])?;
    let command = expect_string_field(mapping, "command", None)?;
    if command.trim().is_empty() {
        return Err(agent_error(
            "validate zyal document",
            format!("{path}.command must not be empty"),
            vec!["supply a real shell command".into()],
            "docs/zyal-research-loops.md",
            "give the shell check a concrete command",
        ));
    }
    if let Some(assert) = mapping.get(&Value::String("assert".into())) {
        let assert = expect_mapping(assert, &format!("{path}.assert"))?;
        assert_keys(
            &format!("{path}.assert"),
            assert,
            &["exit_code", "stdout_contains", "stdout_regex", "json"],
        )?;
    }
    Ok(())
}
