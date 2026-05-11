use crate::util::{read_generated_json, write_generated_json, write_generated_text};
use anyhow::Result;
use openqg_core::{repo_score_from_scorecard, RepoScore, Scorecard};
use std::path::Path;

fn render_markdown(repo_score: &RepoScore) -> String {
    format!(
        "# Repo Score\n\n- repo: OpenQG\n- score: {}\n- status: {}\n- suite: {}\n- coverage: {:.3}\n- log_likelihood: {:.3}\n- delta_log_likelihood: {:.3}\n- aic: {:.3}\n- bic: {:.3}\n- mdl: {:.3}\n- findings: {}\n",
        repo_score.score,
        repo_score.status,
        repo_score.suite_id,
        repo_score.metrics.coverage,
        repo_score.metrics.log_likelihood,
        repo_score.metrics.delta_log_likelihood,
        repo_score.metrics.aic,
        repo_score.metrics.bic,
        repo_score.metrics.mdl,
        repo_score.findings.len()
    )
}

pub fn compare(scorecard_path: &Path, json_output: &Path, md_output: &Path) -> Result<()> {
    let scorecard: Scorecard = read_generated_json(scorecard_path)?;
    let (score, status) = repo_score_from_scorecard(&scorecard);
    let repo_score = RepoScore {
        repo: "OpenQG".into(),
        benchmark_version: scorecard.benchmark_version.clone(),
        suite_id: scorecard.suite_id.clone(),
        score,
        status,
        summary: if score >= 85 {
            "benchmark lane green".into()
        } else {
            "benchmark lane needs work".into()
        },
        metrics: scorecard.metrics.clone(),
        findings: scorecard.findings.clone(),
    };
    write_generated_json(json_output, "openqg-bench", "just score", &repo_score)?;
    if json_output.ends_with("target/jankurai/repo-score.json") {
        write_generated_json(
            Path::new("agent/repo-score.json"),
            "openqg-bench",
            "just score",
            &repo_score,
        )?;
    }
    write_generated_text(
        md_output,
        "openqg-bench",
        "just score",
        &render_markdown(&repo_score),
    )?;
    if md_output.ends_with("target/jankurai/repo-score.md") {
        write_generated_text(
            Path::new("agent/repo-score.md"),
            "openqg-bench",
            "just score",
            &render_markdown(&repo_score),
        )?;
    }
    println!("repo score {}", repo_score.score);
    Ok(())
}
