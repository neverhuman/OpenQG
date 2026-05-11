use openqg_domain::{agent_error, Result};
use serde_yaml::Value;
use std::fs;
use std::path::Path;

fn zyal_body<'a>(text: &'a str, path: &Path) -> Result<&'a str> {
    match text.find("<<<ZYAL") {
        Some(start) => {
            let end = match text.find("<<<END_ZYAL") {
                Some(end) => end,
                None => {
                    return Err(agent_error(
                        "parse zyal document",
                        format!("missing end sentinel in {}", path.display()),
                        vec![
                            "add the end sentinel".into(),
                            "compare against an existing runbook".into(),
                        ],
                        "docs/testing.md",
                        "rerun just zyal-validate after restoring the sentinel",
                    ));
                }
            };
            let sentinel_end = match text[start..].find(">>>") {
                Some(offset) => start + offset + 3,
                None => {
                    return Err(agent_error(
                        "parse zyal document",
                        format!("missing sentinel terminator in {}", path.display()),
                        vec![
                            "close the opening sentinel".into(),
                            "compare against an existing runbook".into(),
                        ],
                        "docs/testing.md",
                        "rerun just zyal-validate after restoring the terminator",
                    ));
                }
            };
            Ok(&text[sentinel_end..end])
        }
        None => Ok(text),
    }
}

pub fn parse_zyal_document(path: &Path) -> Result<Value> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) => {
            return Err(agent_error(
                "read zyal document",
                format!("failed to read {}", path.display()),
                vec![
                    "check file permissions".into(),
                    "verify the runbook exists".into(),
                ],
                "docs/testing.md",
                format!("fix the runbook path or permissions: {err}"),
            ));
        }
    };
    let body = zyal_body(&text, path)?;
    match serde_yaml::from_str(body) {
        Ok(value) => Ok(value),
        Err(err) => Err(agent_error(
            "parse zyal document",
            format!("failed to parse {}", path.display()),
            vec![
                "check the YAML syntax".into(),
                "verify the ZYAL sentinels".into(),
            ],
            "docs/testing.md",
            format!("fix the YAML and rerun the validator: {err}"),
        )),
    }
}

pub fn validate_zyal_value(value: &Value) -> Result<Vec<String>> {
    let mapping = match value.as_mapping() {
        Some(mapping) => mapping,
        None => {
            return Err(agent_error(
                "validate zyal document",
                "ZYAL document must be a mapping",
                vec![
                    "use a top-level mapping".into(),
                    "restore the document structure".into(),
                ],
                "docs/testing.md",
                "reformat the document as a YAML mapping",
            ));
        }
    };

    let required = ["version", "intent", "confirm", "job", "stop"];
    for key in required {
        let key_value = Value::String(key.to_string());
        if !mapping.contains_key(&key_value) {
            return Err(agent_error(
                "validate zyal document",
                format!("ZYAL document missing required key: {key}"),
                vec![
                    "add the missing key".into(),
                    "compare against an existing runbook".into(),
                ],
                "docs/testing.md",
                "rerun just zyal-validate after restoring the required keys",
            ));
        }
    }

    let mut warnings = Vec::new();
    if !mapping.contains_key(&Value::String("checkpoint".to_string())) {
        warnings.push("checkpoint block absent".to_string());
    }
    if !mapping.contains_key(&Value::String("permissions".to_string())) {
        warnings.push("permissions block absent".to_string());
    }
    Ok(warnings)
}
