use crate::util::generated_at;
use anyhow::{Context, Result};
use openqg_core::sha256_file;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

const SECURITY_DIR: &str = "target/jankurai/security";
const LANE_LOG_PATH: &str = "target/jankurai/security/lane.log";
const LANE_STATUS_PATH: &str = "target/jankurai/security/lane-status.txt";
const POLICY_PATH: &str = "agent/security-policy.toml";
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

    let git_head = git_head(root);
    let git_status = git_status(root)?;
    let cargo_lock_path = root.join(CARGO_LOCK_PATH);
    let cargo_lock = cargo_lock_path.exists().then_some(cargo_lock_path.clone());
    let cargo_lock_sha256 = match cargo_lock.as_ref() {
        Some(path) => Some(sha256_file(path)?),
        None => None,
    };
    let node_manifests = find_node_manifests(root)?;
    let policy = read_policy(root)?;

    let mut commands = Vec::new();
    let mut findings = Vec::new();

    commands.push(run_command(
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
        commands.push(run_command(
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
        commands.push(run_command(
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

fn read_policy(root: &Path) -> Result<SecurityPolicyEvidence> {
    let path = root.join(POLICY_PATH);
    let text = fs::read_to_string(&path)
        .with_context(|| format!("read security policy {}", path.display()))?;
    let sha256 = sha256_file(&path)?;
    let policy: SecurityPolicyToml = toml::from_str(&text)
        .with_context(|| format!("parse security policy {}", path.display()))?;
    let profile = if std::env::var("CI").is_ok() {
        "ci"
    } else {
        "local"
    }
    .to_string();

    Ok(SecurityPolicyEvidence {
        schema_version: policy.schema_version,
        path: path.display().to_string(),
        sha256,
        profile,
        enabled_tools: policy.enabled_tools,
        required_tools: policy.required_tools,
        advisory_tools: policy.advisory_tools,
        fail_lane_on: policy.severity_thresholds.fail_lane_on,
    })
}

#[derive(Debug, Deserialize)]
struct SecurityPolicyToml {
    schema_version: String,
    enabled_tools: Vec<String>,
    required_tools: Vec<String>,
    advisory_tools: Vec<String>,
    severity_thresholds: SeverityThresholds,
}

#[derive(Debug, Deserialize)]
struct SeverityThresholds {
    fail_lane_on: String,
}

fn run_command(
    name: &str,
    tool: &str,
    command: &str,
    args: &[&str],
    root: &Path,
    log_path: PathBuf,
    required: bool,
    advisory: bool,
    inputs: &[&str],
) -> Result<SecurityCommandResult> {
    let started = Instant::now();
    let result = Command::new(tool).args(args).current_dir(root).output();
    let (mut status, mut exit_code, stdout, stderr) = match result {
        Ok(output) => {
            let status = if output.status.success() {
                "success"
            } else {
                "failed"
            };
            (
                status.to_string(),
                output.status.code(),
                String::from_utf8_lossy(&output.stdout).to_string(),
                String::from_utf8_lossy(&output.stderr).to_string(),
            )
        }
        Err(err) if err.kind() == ErrorKind::NotFound => (
            String::from("missing_tool"),
            None,
            String::new(),
            format!("{tool} is not installed"),
        ),
        Err(err) => (
            String::from("failed"),
            None,
            String::new(),
            format!("failed to run {tool}: {err}"),
        ),
    };

    if matches_missing_cargo_audit(tool, command, &stderr) {
        status = String::from("missing_tool");
        exit_code = None;
    }

    let mut log = String::new();
    log.push_str(&format!("tool: {tool}\n"));
    log.push_str(&format!("command: {command}\n"));
    log.push_str(&format!("status: {status}\n"));
    if let Some(code) = exit_code {
        log.push_str(&format!("exit_code: {code}\n"));
    }
    if !inputs.is_empty() {
        log.push_str("inputs:\n");
        for input in inputs {
            log.push_str(&format!("- {input}\n"));
        }
    }
    log.push('\n');
    if !stdout.is_empty() {
        log.push_str("stdout:\n");
        log.push_str(&stdout);
        if !stdout.ends_with('\n') {
            log.push('\n');
        }
        log.push('\n');
    }
    if !stderr.is_empty() {
        log.push_str("stderr:\n");
        log.push_str(&stderr);
        if !stderr.ends_with('\n') {
            log.push('\n');
        }
        log.push('\n');
    }

    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    fs::write(&log_path, log).with_context(|| format!("write {}", log_path.display()))?;
    let log_sha256 = sha256_file(&log_path)?;
    let log_bytes = fs::metadata(&log_path)
        .with_context(|| format!("stat {}", log_path.display()))?
        .len();

    Ok(SecurityCommandResult {
        name: name.to_string(),
        tool: tool.to_string(),
        command: command.to_string(),
        status,
        exit_code,
        required,
        advisory,
        log_path: log_path.display().to_string(),
        log_sha256,
        log_bytes,
        duration_ms: started.elapsed().as_millis(),
        inputs: inputs.iter().map(|input| input.to_string()).collect(),
    })
}

fn matches_missing_cargo_audit(tool: &str, command: &str, stderr: &str) -> bool {
    tool == "cargo" && command.starts_with("cargo audit") && stderr.contains("no such command")
}

fn git_head(root: &Path) -> String {
    let output = match Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()
    {
        Ok(output) if output.status.success() => output,
        _ => return String::from("unknown"),
    };
    match String::from_utf8(output.stdout) {
        Ok(value) => value.trim().to_string(),
        Err(_) => String::from("unknown"),
    }
}

fn git_status(root: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["status", "--short", "--untracked-files=no"])
        .current_dir(root)
        .output()
        .with_context(|| "run git status")?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn find_node_manifests(root: &Path) -> Result<Vec<String>> {
    let mut manifests = Vec::new();
    for entry in walkdir::WalkDir::new(root) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        match entry.path().file_name().and_then(|name| name.to_str()) {
            Some("package.json")
            | Some("package-lock.json")
            | Some("pnpm-lock.yaml")
            | Some("yarn.lock")
            | Some("npm-shrinkwrap.json") => {
                manifests.push(entry.path().display().to_string());
            }
            _ => {}
        }
    }
    manifests.sort();
    Ok(manifests)
}
