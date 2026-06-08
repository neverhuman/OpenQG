use super::*;

pub(crate) fn classify_jailgun_attempt_failure(
    summary: Option<&Value>,
    final_status: &Value,
    status_snapshots: &[Value],
) -> Option<&'static str> {
    summary
        .and_then(classify_jailgun_summary_failure)
        .or_else(|| classify_jailgun_status_failure(final_status))
        .or_else(|| {
            status_snapshots
                .iter()
                .rev()
                .find_map(classify_jailgun_status_failure)
        })
}

pub(crate) fn classify_jailgun_summary_failure(summary: &Value) -> Option<&'static str> {
    if summary.get("status").and_then(Value::as_str) == Some("timed-out") {
        return Some("runtime-timeout");
    }
    for key in ["failure_kind", "error", "message", "status_detail"] {
        if let Some(message) = summary.get(key).and_then(Value::as_str) {
            let class = classify_jailgun_failure(message);
            if class != "unknown" {
                return Some(class);
            }
        }
    }
    let failures = summary.get("failures").and_then(Value::as_array)?;
    for failure in failures {
        for key in ["kind", "code", "phase", "error", "message"] {
            if let Some(message) = failure.get(key).and_then(Value::as_str) {
                let class = classify_jailgun_failure(message);
                if class != "unknown" {
                    return Some(class);
                }
            }
        }
        let class = classify_jailgun_failure(&failure.to_string());
        if class != "unknown" {
            return Some(class);
        }
    }
    None
}

pub(crate) fn classify_jailgun_events_failure(path: &str) -> Option<&'static str> {
    fs::read_to_string(Path::new(path))
        .ok()
        .and_then(|text| classify_jailgun_events_text(&text))
}

pub(crate) fn classify_jailgun_events_text(text: &str) -> Option<&'static str> {
    let mut first_known = None;
    for line in text.lines() {
        let class = classify_jailgun_failure(line);
        if class == "rate-limit" {
            return Some("rate-limit");
        }
        if class != "unknown" && first_known.is_none() {
            first_known = Some(class);
        }
    }
    first_known
}

pub(crate) fn classify_jailgun_status_failure(status: &Value) -> Option<&'static str> {
    if status.get("status").and_then(Value::as_str) == Some("timed-out") {
        return Some("runtime-timeout");
    }
    for key in ["failure_kind", "error", "message", "status_detail"] {
        if let Some(message) = status.get(key).and_then(Value::as_str) {
            let class = classify_jailgun_failure(message);
            if class != "unknown" {
                return Some(class);
            }
        }
    }
    if let Some(tabs) = status.get("tabs").and_then(Value::as_array) {
        for tab in tabs {
            for key in [
                "status",
                "deploy_status",
                "prompt_policy_decision",
                "error",
                "message",
                "status_detail",
            ] {
                if let Some(message) = tab.get(key).and_then(Value::as_str) {
                    let class = classify_jailgun_failure(message);
                    if class != "unknown" {
                        return Some(class);
                    }
                }
            }
            if tab.get("status").and_then(Value::as_str) == Some("error") {
                return Some("browser-tab-error");
            }
        }
    }
    let class = classify_jailgun_failure(&status.to_string());
    if class != "unknown" {
        return Some(class);
    }
    if status.get("status").and_then(Value::as_str) == Some("failed") {
        return Some("browser-run-failed");
    }
    None
}

pub(crate) fn classify_jailgun_failure(message: &str) -> &'static str {
    let lower = message.to_ascii_lowercase();
    if lower.contains("browser account queue timed out") || lower.contains("browser-lease-timeout")
    {
        "queue-timeout"
    } else if lower.contains("rate limit")
        || lower.contains("rate-limit")
        || lower.contains("too many requests")
        || lower.contains("http 429")
        || lower.contains(" 429")
        || lower.contains("quota exceeded")
    {
        "rate-limit"
    } else if lower.contains("browser lease failed")
        || lower.contains("browser-account")
        || lower.contains("browser lease")
    {
        "browser-lease"
    } else if lower.contains("auth-required") || lower.contains("code-requested") {
        "auth-required"
    } else if lower.contains("session-expired") || lower.contains("session expired") {
        "session-expired"
    } else if lower.contains("manual-browser-required") {
        "manual-browser-required"
    } else if lower.contains("browser tab error")
        || lower.contains("tab status error")
        || lower.contains("\"status\":\"error\"")
        || lower.contains("\"status\": \"error\"")
    {
        "browser-tab-error"
    } else if lower.contains("no .tex artifact download candidate")
        || lower.contains("no artifact download candidate")
        || lower.contains("done-no-artifact")
    {
        "artifact-download-missing"
    } else if lower.contains("tar-validation")
        || lower.contains("tar validation")
        || lower.contains("tarball validation")
        || lower.contains("archive validation")
    {
        "tar-validation"
    } else if lower.contains("invalid gzip")
        || lower.contains("gzip header")
        || lower.contains("not in gzip format")
        || lower.contains("incorrect header check")
        || lower.contains("invalid tar header")
        || lower.contains("invalid header")
    {
        "invalid-archive-header"
    } else if lower.contains("artifact conversation recovery")
        || lower.contains("artifact_conversation")
        || lower.contains("recovered artifact")
        || lower.contains("recovered-target")
        || lower.contains("recovered_from")
        || lower.contains("abandoned run tab")
    {
        "outdated-artifact-recovery"
    } else if lower.contains("runtime timeout")
        || lower.contains("run timed out")
        || lower.contains("timed-out")
        || lower.contains("timed out")
        || lower.contains("deadline elapsed")
        || lower.contains("max_runtime_seconds")
        || lower.contains("max runtime")
    {
        "runtime-timeout"
    } else {
        "unknown"
    }
}
