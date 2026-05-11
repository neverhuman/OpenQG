use crate::ReleaseManifest;
use anyhow::{bail, Result};

use super::ensure_non_empty;

pub fn validate_release_manifest(manifest: &ReleaseManifest) -> Result<()> {
    ensure_non_empty("version", &manifest.version)?;
    ensure_non_empty("benchmark_version", &manifest.benchmark_version)?;
    ensure_non_empty("scorecard_hash", &manifest.scorecard_hash)?;
    ensure_non_empty("data_lock_hash", &manifest.data_lock_hash)?;
    ensure_non_empty("scorecard_path", &manifest.scorecard_path)?;
    if manifest.candidate_hashes.is_empty() {
        bail!("candidate_hashes must not be empty");
    }
    Ok(())
}
