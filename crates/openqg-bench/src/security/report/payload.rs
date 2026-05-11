use crate::util::generated_at;
use serde_json::Value;
use std::time::Instant;

use super::super::findings::SecurityScan;

pub fn build(scan: &SecurityScan, start: Instant) -> Value {
    serde_json::json!({
        "schema_version": "1.0.0",
        "standard_version": "0.8.0",
        "auditor_version": "0.8.13",
        "repo_root": scan.provenance.repo_root,
        "git_head": scan.provenance.git_head,
        "git_status": scan.provenance.git_status,
        "lane": "security",
        "policy": scan.policy,
        "provenance": scan.provenance,
        "wrapper": {
            "kind": "bash_script",
            "path": "tools/security-lane.sh",
            "strict": false,
        },
        "exit_code": if scan.status == "fail" { 1 } else { 0 },
        "elapsed_ms": start.elapsed().as_millis(),
        "lane_log_path": scan.lane_log_path,
        "lane_status_path": scan.lane_status_path,
        "commands": scan.commands,
        "status": scan.status,
        "findings": scan.findings,
        "generated_at": generated_at(),
    })
}
