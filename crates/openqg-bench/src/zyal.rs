use crate::util::write_generated_json;
use anyhow::Result;
use openqg_core::{parse_zyal_document, validate_zyal_value, ZyalPreview};
use serde_yaml::Value;
use std::path::Path;

pub fn validate(root: &Path, output: &Path) -> Result<()> {
    let mut previews = Vec::new();
    for path in openqg_core::walk_yaml_files(root)? {
        let value = parse_zyal_document(&path)?;
        let warnings = validate_zyal_value(&value)?;
        let name = if let Some(mapping) = value.as_mapping() {
            if let Some(job) = mapping.get(&Value::String("job".into())) {
                if let Some(job_mapping) = job.as_mapping() {
                    if let Some(name) = job_mapping.get(&Value::String("name".into())) {
                        if let Some(name) = name.as_str() {
                            name.to_owned()
                        } else {
                            preview_name_from_path(&path)
                        }
                    } else {
                        preview_name_from_path(&path)
                    }
                } else {
                    preview_name_from_path(&path)
                }
            } else {
                preview_name_from_path(&path)
            }
        } else {
            preview_name_from_path(&path)
        };
        previews.push(ZyalPreview {
            file: path.to_string_lossy().to_string(),
            name,
            valid: true,
            warnings,
        });
    }
    write_generated_json(output, "openqg-bench", "just zyal-validate", &previews)?;
    println!("validated {} zyal runbooks", previews.len());
    Ok(())
}

fn preview_name_from_path(path: &Path) -> String {
    match path.file_stem() {
        Some(stem) => match stem.to_str() {
            Some(stem) => stem.to_owned(),
            None => String::from("unknown"),
        },
        None => String::from("unknown"),
    }
}
