use openqg_domain::{agent_error, Result};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

pub fn sha256_digest(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("{digest:x}")
}

pub fn sha256_file(path: &Path) -> Result<String> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(err) => {
            return Err(agent_error(
                "read file",
                format!("failed to read {}", path.display()),
                vec![
                    "check file permissions".into(),
                    "verify the file exists".into(),
                ],
                "docs/testing.md",
                format!("fix the path or permissions: {err}"),
            ));
        }
    };
    Ok(sha256_digest(&bytes))
}

pub fn walk_yaml_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if !root.exists() {
        return Ok(files);
    }
    for entry in walkdir::WalkDir::new(root) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                return Err(agent_error(
                    "walk yaml files",
                    format!("failed to walk {}", root.display()),
                    vec![
                        "check the directory exists".into(),
                        "remove unreadable paths".into(),
                    ],
                    "docs/testing.md",
                    format!("rerun the walker after fixing the filesystem: {err}"),
                ));
            }
        };
        if entry.file_type().is_file() {
            let path = entry.path();
            if is_yaml_file(path) {
                files.push(path.to_path_buf());
            }
        }
    }
    files.sort();
    Ok(files)
}

fn is_yaml_file(path: &Path) -> bool {
    match path.extension().and_then(|s| s.to_str()) {
        Some("yml") | Some("yaml") => true,
        _ => false,
    }
}
