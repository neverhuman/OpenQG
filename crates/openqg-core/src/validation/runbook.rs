mod envelope;
mod preview;
mod schema;
mod yaml;

use envelope::{parse_zyal_envelope, ParsedEnvelope};
use openqg_domain::{agent_error, Result};
use preview::build_preview;
use serde_yaml::Value;
use std::fs;
use std::path::Path;

use crate::ZyalPreview;

pub use schema::validate_zyal_value;

pub fn parse_zyal_document(path: &Path) -> Result<Value> {
    let (_, value) = read_zyal_document(path)?;
    Ok(value)
}

pub fn preview_zyal_document(path: &Path) -> Result<ZyalPreview> {
    let (envelope, value) = read_zyal_document(path)?;
    let warnings = validate_zyal_value(&value)?;
    Ok(build_preview(path, &envelope, &value, warnings))
}

fn read_zyal_document(path: &Path) -> Result<(ParsedEnvelope, Value)> {
    let text = fs::read_to_string(path).map_err(|err| {
        agent_error(
            "read zyal document",
            format!("failed to read {}", path.display()),
            vec![
                "check file permissions".into(),
                "verify the runbook exists".into(),
            ],
            "docs/testing.md",
            format!("fix the runbook path or permissions: {err}"),
        )
    })?;
    let envelope = parse_zyal_envelope(&text, path)?;
    let value = serde_yaml::from_str(&envelope.body).map_err(|err| {
        agent_error(
            "parse zyal document",
            format!("failed to parse {}", path.display()),
            vec![
                "check the YAML syntax".into(),
                "verify the ZYAL sentinels".into(),
            ],
            "docs/testing.md",
            format!("fix the YAML and rerun the validator: {err}"),
        )
    })?;
    Ok((envelope, value))
}
