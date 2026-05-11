use crate::util::generated_at;
use anyhow::{Context, Result};
use openqg_core::sha256_file;
use serde::Serialize;
use std::fs;
use std::path::Path;

#[path = "findings_helpers.rs"]
mod findings_helpers;

const SECURITY_DIR: &str = "target/jankurai/security";
const LANE_LOG_PATH: &str = "target/jankurai/security/lane.log";
const LANE_STATUS_PATH: &str = "target/jankurai/security/lane-status.txt";
const CARGO_LOCK_PATH: &str = "Cargo.lock";

#[derive(Debug, Serialize)]
pub struct SecurityScan {
    pub generated_at: String,
    pub status: String,
    pub lane_log_path: String,
    pub lane_status_path: String,
    pub provenance: SecurityProvenance,
    pub policy: SecurityPolicyEvidence,
    pub commands: Vec<SecurityCommandResult>,
    pub findings: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SecurityProvenance {
    pub repo_root: String,
    pub git_head: String,
    pub git_status: String,
    pub cargo_lock_path: Option<String>,
    pub cargo_lock_sha256: Option<String>,
    pub node_manifests: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SecurityPolicyEvidence {
    pub schema_version: String,
    pub path: String,
    pub sha256: String,
    pub profile: String,
    pub enabled_tools: Vec<String>,
    pub required_tools: Vec<String>,
    pub advisory_tools: Vec<String>,
    pub fail_lane_on: String,
}

#[derive(Debug, Serialize)]
pub struct SecurityCommandResult {
    pub name: String,
    pub tool: String,
    pub command: String,
    pub status: String,
    pub exit_code: Option<i32>,
    pub required: bool,
    pub advisory: bool,
    pub log_path: String,
    pub log_sha256: String,
    pub log_bytes: u64,
    pub duration_ms: u128,
    pub inputs: Vec<String>,
}

pub fn collect(root: &Path) -> Result<SecurityScan> {
    fs::create_dir_all(SECURITY_DIR).with_context(|| format!("create {}", SECURITY_DIR))?;

    let git_head = findings_helpers::git_head(root);
    let git_status = findings_helpers::git_status(root)?;
    let cargo_lock_path = root.join(CARGO_LOCK_PATH);
    let cargo_lock = cargo_lock_path.exists().then_some(cargo_lock_path.clone());
    let cargo_lock_sha256 = match cargo_lock.as_ref() {
        Some(path) => Some(sha256_file(path)?),
        None => None,
    };
    let node_manifests = findings_helpers::find_node_manifests(root)?;
    let policy = findings_helpers::read_policy(root)?;

    let mut commands = Vec::new();
    let mut findings = Vec::new();

    commands.push(findings_helpers::run_command(
        "gitleaks",
        "gitleaks",
        "gitleaks detect --source . --no-git --no-banner --redact",
        &[
            "detect",
            "--source",
            ".",
            "--no-git",
            "--no-banner",
            "--redact",
        ],
        root,
        Path::new(SECURITY_DIR).join("gitleaks.log"),
        true,
        false,
        &["."],
    )?);

    if cargo_lock.is_some() {
        commands.push(findings_helpers::run_command(
            "cargo-audit",
            "cargo",
            "cargo audit --json --no-fetch",
            &["audit", "--json", "--no-fetch"],
            root,
            Path::new(SECURITY_DIR).join("cargo-audit.log"),
            false,
            true,
            &[CARGO_LOCK_PATH],
        )?);
    }

    if !node_manifests.is_empty() {
        let node_inputs = node_manifests
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        commands.push(findings_helpers::run_command(
            "npm-audit",
            "npm",
            "npm audit --offline --audit-level=high --json",
            &["audit", "--offline", "--audit-level=high", "--json"],
            root,
            Path::new(SECURITY_DIR).join("npm-audit.log"),
            false,
            true,
            &node_inputs,
        )?);
    }

    let workflow_files = findings_helpers::find_workflow_files(root)?;
    if !workflow_files.is_empty() {
        let workflow_inputs = workflow_files
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        commands.push(findings_helpers::run_command(
            "workflow-lint",
            "actionlint",
            &format!("actionlint {}", workflow_inputs.join(" ")),
            &workflow_inputs,
            root,
            Path::new(SECURITY_DIR).join("workflow-lint.log"),
            false,
            true,
            &workflow_inputs,
        )?);
    }

    commands.push(findings_helpers::run_command(
        "sbom",
        "syft",
        "syft dir:. -o json",
        &["dir:.", "-o", "json"],
        root,
        Path::new(SECURITY_DIR).join("syft.log"),
        false,
        true,
        &["."],
    )?);

    for command in &commands {
        if command.status == "failed" {
            findings.push(format!(
                "{} failed with exit code {:?} (log: {})",
                command.name, command.exit_code, command.log_path
            ));
        }
        if command.status == "missing_tool" && command.required {
            findings.push(format!("{} is required but not installed", command.name));
        }
    }

    let status = if commands.iter().any(|command| command.status == "failed") {
        String::from("fail")
    } else if commands
        .iter()
        .any(|command| command.status == "missing_tool" && command.required)
    {
        String::from("warn")
    } else {
        String::from("pass")
    };

    Ok(SecurityScan {
        generated_at: generated_at(),
        status,
        lane_log_path: String::from(LANE_LOG_PATH),
        lane_status_path: String::from(LANE_STATUS_PATH),
        provenance: SecurityProvenance {
            repo_root: root.display().to_string(),
            git_head,
            git_status,
            cargo_lock_path: cargo_lock.as_ref().map(|path| path.display().to_string()),
            cargo_lock_sha256,
            node_manifests,
        },
        policy,
        commands,
        findings,
    })
}
