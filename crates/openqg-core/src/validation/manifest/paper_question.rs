use crate::{ChallengeRecord, PaperBodyRecord, RouteMetadata};
use anyhow::{bail, Result};

use super::{ensure_http_url, ensure_non_empty};

pub fn validate_paper_body_record(record: &PaperBodyRecord) -> Result<()> {
    ensure_hash("publication_hash", &record.publication_hash)?;
    ensure_hash("body_hash", &record.body_hash)?;
    ensure_non_empty("title", &record.title)?;
    if record.authors.is_empty() {
        bail!("authors must not be empty");
    }
    ensure_non_empty("license", &record.license)?;
    ensure_non_empty("oa_proof", &record.oa_proof)?;
    if record.source_urls.is_empty() {
        bail!("source_urls must not be empty");
    }
    for url in &record.source_urls {
        ensure_http_url("source_urls[]", url)?;
    }
    if record.sections.is_empty() {
        bail!("sections must not be empty");
    }
    ensure_non_empty("body_text", &record.body_text)?;
    Ok(())
}

pub fn validate_challenge_record(record: &ChallengeRecord) -> Result<()> {
    ensure_hash("challenge_hash", &record.challenge_hash)?;
    ensure_hash("publication_hash", &record.publication_hash)?;
    ensure_non_empty("rubric_version", &record.rubric_version)?;
    ensure_non_empty("question", &record.question)?;
    ensure_non_empty("answer_key", &record.answer_key)?;
    if record.support_sections.is_empty() {
        bail!("support_sections must not be empty");
    }
    if !(record.context_pack.target_fill_ratio > 0.0
        && record.context_pack.target_fill_ratio <= 1.0)
    {
        bail!("context_pack.target_fill_ratio must be > 0 and <= 1");
    }
    if record.context_pack.output_reserve_tokens == 0 {
        bail!("context_pack.output_reserve_tokens must be positive");
    }
    for attempt in record
        .generator_agents
        .iter()
        .chain(record.blind_answer_attempts.iter())
        .chain(record.critic_attempts.iter())
        .chain(record.audit_attempts.iter())
    {
        ensure_non_empty("agent_attempt.agent_id", &attempt.agent_id)?;
        ensure_non_empty("agent_attempt.role", &attempt.role)?;
        validate_route_metadata(&attempt.route_metadata)?;
    }
    ensure_non_empty("acceptance.reason", &record.acceptance.reason)?;
    Ok(())
}

pub fn validate_route_metadata(metadata: &RouteMetadata) -> Result<()> {
    ensure_non_empty("route_metadata.agent_role", &metadata.agent_role)?;
    ensure_non_empty("route_metadata.zyal_run_id", &metadata.zyal_run_id)?;
    ensure_non_empty("route_metadata.zyal_lane_id", &metadata.zyal_lane_id)?;
    if let Some(confidence) = metadata.confidence {
        if !(0.0..=1.0).contains(&confidence) {
            bail!("route_metadata.confidence must be between 0 and 1");
        }
    }
    Ok(())
}

fn ensure_hash(label: &str, value: &str) -> Result<()> {
    ensure_non_empty(label, value)?;
    if value.len() != 64 || !value.chars().all(|ch| ch.is_ascii_hexdigit()) {
        bail!("{label} must be a 64-character SHA-256 hex digest");
    }
    Ok(())
}
