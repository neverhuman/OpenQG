use crate::util::{generated_at, sha256_digest, write_json};
use anyhow::{bail, Context, Result};
use openqg_core::{
    validate_challenge_record, validate_paper_body_record, ChallengeRecord, PaperBodyRecord,
};
use serde::Serialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
struct ResearchValidationReport {
    generated_at: String,
    root: String,
    papers: usize,
    accepted_challenges: usize,
    rejected_challenges: usize,
    duplicates: usize,
}

pub fn validate(root: &Path, output: &Path) -> Result<()> {
    let papers = load_papers(&root.join("papers"))?;
    let accepted = load_challenges(&root.join("challenges"))?;
    let rejected = load_challenges(&root.join("rejected"))?;
    let duplicates = duplicate_count(&papers);
    if duplicates > 0 {
        bail!("duplicate publication_hash records found: {duplicates}");
    }
    for paper in &papers {
        validate_paper_body_record(paper)?;
    }
    for challenge in accepted.iter().chain(rejected.iter()) {
        validate_challenge_record(challenge)?;
    }
    write_json(
        output,
        &ResearchValidationReport {
            generated_at: generated_at(),
            root: root.display().to_string(),
            papers: papers.len(),
            accepted_challenges: accepted.len(),
            rejected_challenges: rejected.len(),
            duplicates,
        },
    )?;
    println!(
        "validated paper question bank: papers={} accepted={} rejected={}",
        papers.len(),
        accepted.len(),
        rejected.len()
    );
    Ok(())
}

pub fn dedupe_check(root: &Path) -> Result<()> {
    let papers = load_papers(&root.join("papers"))?;
    let duplicates = duplicate_count(&papers);
    if duplicates > 0 {
        bail!("duplicate publication_hash records found: {duplicates}");
    }
    println!("dedupe check passed for {} papers", papers.len());
    Ok(())
}

pub fn smoke_fixture(root: &Path) -> Result<()> {
    fs::create_dir_all(root.join("papers"))?;
    fs::create_dir_all(root.join("challenges"))?;
    fs::create_dir_all(root.join("rejected"))?;
    let body_text = "Open-access smoke paper body. The invariant answer is alpha equals one.";
    let publication_hash = sha256_digest(b"openqg-smoke-paper");
    let body_hash = sha256_digest(body_text.as_bytes());
    let paper = serde_json::json!({
        "publication_hash": publication_hash,
        "body_hash": body_hash,
        "title": "OpenQG smoke paper",
        "authors": ["OpenQG"],
        "identifiers": [{"kind": "url", "value": "https://example.org/openqg-smoke"}],
        "license": "CC-BY-4.0",
        "oa_proof": "smoke fixture; public example URL",
        "source_urls": ["https://example.org/openqg-smoke"],
        "extraction_receipts": ["target/openqg/research/smoke/latest/raw/paper.txt"],
        "sections": [{"id": "s1", "heading": "Result", "text": body_text}],
        "body_text": body_text
    });
    let challenge_hash = sha256_digest(format!("{publication_hash}What is alpha?onev1").as_bytes());
    let challenge = serde_json::json!({
        "challenge_hash": challenge_hash,
        "publication_hash": publication_hash,
        "rubric_version": "v1",
        "question": "According to the smoke paper, what value is alpha?",
        "answer_key": "alpha equals one",
        "support_sections": ["s1"],
        "context_pack": {
            "strategy": "hard",
            "target_fill_ratio": 0.82,
            "output_reserve_tokens": 4096,
            "safe_window_tokens": 8192
        },
        "generator_agents": [],
        "blind_answer_attempts": [],
        "critic_attempts": [],
        "audit_attempts": [{
            "agent_id": "smoke-auditor",
            "role": "auditor",
            "answer": null,
            "score": 1.0,
            "route_metadata": {
                "request_id": "smoke-request",
                "route_mode": "fast",
                "primary_model_id": "smoke-primary",
                "backup_model_ids": [],
                "fusion_model_id": null,
                "winner_model_id": "smoke-primary",
                "confidence": 1.0,
                "provider": "smoke",
                "model": "smoke-model",
                "agent_role": "auditor",
                "zyal_run_id": "smoke-run",
                "zyal_lane_id": "smoke-lane"
            }
        }],
        "acceptance": {"accepted": true, "reason": "smoke fixture"}
    });
    fs::write(
        root.join("papers").join(format!("{publication_hash}.json")),
        serde_json::to_vec_pretty(&paper)?,
    )?;
    fs::write(
        root.join("challenges")
            .join(format!("{challenge_hash}.json")),
        serde_json::to_vec_pretty(&challenge)?,
    )?;
    println!("wrote smoke fixture to {}", root.display());
    Ok(())
}

fn load_papers(root: &Path) -> Result<Vec<PaperBodyRecord>> {
    load_json_files(root)
}

fn load_challenges(root: &Path) -> Result<Vec<ChallengeRecord>> {
    load_json_files(root)
}

fn load_json_files<T: serde::de::DeserializeOwned>(root: &Path) -> Result<Vec<T>> {
    let mut out = Vec::new();
    for path in collect_json_files(root)? {
        let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        out.push(serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))?);
    }
    Ok(out)
}

fn collect_json_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if !root.exists() {
        return Ok(files);
    }
    for entry in walkdir::WalkDir::new(root) {
        let entry = entry?;
        if entry.file_type().is_file()
            && entry.path().extension().and_then(|ext| ext.to_str()) == Some("json")
        {
            files.push(entry.path().to_path_buf());
        }
    }
    files.sort();
    Ok(files)
}

fn duplicate_count(papers: &[PaperBodyRecord]) -> usize {
    let mut seen = BTreeSet::new();
    let mut duplicates = 0;
    for paper in papers {
        if !seen.insert(paper.publication_hash.as_str()) {
            duplicates += 1;
        }
    }
    duplicates
}
