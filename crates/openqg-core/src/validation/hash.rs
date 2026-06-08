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
    walk_files(root, "walk yaml files", is_yaml_file)
}

pub fn walk_zyal_files(root: &Path) -> Result<Vec<PathBuf>> {
    walk_files(root, "walk zyal files", is_zyal_file)
}

pub fn validate_zyal_layout(repo_root: &Path) -> Result<()> {
    let canonical_root = Path::new("agent/zyal");
    if !repo_root.exists() {
        return Ok(());
    }
    for entry in walkdir::WalkDir::new(repo_root)
        .into_iter()
        .filter_entry(|entry| !is_skipped_source_tree(entry.path()))
    {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                return Err(agent_error(
                    "validate zyal layout",
                    format!("failed to walk {}", repo_root.display()),
                    vec![
                        "check the directory exists".into(),
                        "remove unreadable paths".into(),
                    ],
                    "docs/zyal-research-loops.md",
                    format!("rerun the validator after fixing the filesystem: {err}"),
                ));
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let relative = path.strip_prefix(repo_root).unwrap_or(path);
        let inside_canonical_root = relative.starts_with(canonical_root);
        if is_retired_zyal_file(path) {
            return Err(agent_error(
                "validate zyal layout",
                format!(
                    "retired ZYAL filename is not allowed: {}",
                    relative.display()
                ),
                vec![
                    "rename .zyal.yml and .zyal.yaml files to .zyal".into(),
                    "keep runbooks under agent/zyal/".into(),
                ],
                "docs/zyal-research-loops.md",
                "move the runbook to agent/zyal/<name>.zyal and rerun just zyal-validate",
            ));
        }
        if inside_canonical_root && !is_zyal_file(path) {
            return Err(agent_error(
                "validate zyal layout",
                format!(
                    "non-ZYAL file is not allowed under agent/zyal/: {}",
                    relative.display()
                ),
                vec![
                    "keep only .zyal runbooks under agent/zyal/".into(),
                    "move ownership notes to the parent agent policy files".into(),
                ],
                "docs/zyal-research-loops.md",
                "remove or relocate the non-runbook file and rerun just zyal-validate",
            ));
        }
        if is_zyal_file(path) && !inside_canonical_root {
            return Err(agent_error(
                "validate zyal layout",
                format!(
                    "ZYAL runbook is outside agent/zyal/: {}",
                    relative.display()
                ),
                vec![
                    "move the runbook under agent/zyal/".into(),
                    "do not keep compatibility aliases outside the canonical root".into(),
                ],
                "docs/zyal-research-loops.md",
                "move the runbook to agent/zyal/ and rerun just zyal-validate",
            ));
        }
    }
    Ok(())
}

fn walk_files(
    root: &Path,
    operation: &'static str,
    predicate: fn(&Path) -> bool,
) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if !root.exists() {
        return Ok(files);
    }
    for entry in walkdir::WalkDir::new(root) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                return Err(agent_error(
                    operation,
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
            if predicate(path) {
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

fn is_zyal_file(path: &Path) -> bool {
    path.extension().and_then(|s| s.to_str()) == Some("zyal")
}

fn is_retired_zyal_file(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|s| s.to_str()) else {
        return false;
    };
    file_name.ends_with(".zyal.yml") || file_name.ends_with(".zyal.yaml")
}

fn is_skipped_source_tree(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|s| s.to_str()) else {
        return false;
    };
    // `ZYAL/` is the genome-evolution engine's own input tree (runbooks with a distinct DSL
    // schema, read by the engine and validated by it + `just zyal-test`). It is a separate
    // subsystem from the canonical agent control-plane runbooks under `agent/zyal/`, so this
    // layout validator skips it rather than rejecting engine inputs as misplaced agent tools.
    matches!(file_name, ".git" | "target" | ".jekko" | "ZYAL")
}
