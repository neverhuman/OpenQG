use crate::util::{read_generated_json, sha256_digest, write_generated_json, write_generated_text};
use anyhow::{Context, Result};
use openqg_core::{validate_release_manifest, ReleaseManifest, Scorecard};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

pub fn pack(scorecard_path: &Path, output: &Path) -> Result<()> {
    let text = fs::read_to_string(scorecard_path)
        .with_context(|| format!("read {}", scorecard_path.display()))?;
    let scorecard: Scorecard = read_generated_json(scorecard_path)?;

    let scorecard_hash = sha256_digest(text.as_bytes());
    let data_lock_path = PathBuf::from("target/openqg/data/locks/data-lock.json");
    let data_lock_hash = if data_lock_path.exists() {
        let bytes = fs::read(&data_lock_path)?;
        sha256_digest(&bytes)
    } else {
        String::from("missing")
    };

    let mut candidate_hashes = BTreeMap::new();
    let theory_manifest = PathBuf::from(format!(
        "theories/{}/manifest.yml",
        scorecard.candidate_theory
    ));
    if theory_manifest.exists() {
        candidate_hashes.insert(
            scorecard.candidate_theory.clone(),
            sha256_digest(&fs::read(&theory_manifest)?),
        );
    }

    let release = ReleaseManifest {
        version: format!("{}-release", scorecard.benchmark_version),
        benchmark_version: scorecard.benchmark_version.clone(),
        scorecard_hash,
        data_lock_hash,
        candidate_hashes,
        scorecard_path: scorecard_path.to_string_lossy().to_string(),
        approved: false,
    };
    validate_release_manifest(&release)?;

    let json_path = output.join("release-manifest.json");
    let md_path = output.join("release-manifest.md");
    write_generated_json(&json_path, "openqg-bench", "just release-pack", &release)?;
    fs::create_dir_all(output)?;
    write_generated_text(
        &md_path,
        "openqg-bench",
        "just release-pack",
        &format!(
            "# Release Pack\n\n- benchmark: {}\n- scorecard: {}\n- approved: {}\n- scorecard hash: {}\n- data lock hash: {}\n",
            release.benchmark_version,
            release.scorecard_path,
            release.approved,
            release.scorecard_hash,
            release.data_lock_hash
        ),
    )?;
    println!("wrote release pack to {}", output.display());
    Ok(())
}
