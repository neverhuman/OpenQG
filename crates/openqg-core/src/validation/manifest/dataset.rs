use crate::DatasetManifest;
use anyhow::{bail, Result};

use super::{ensure_http_url, ensure_non_empty, reject_raw_path};

pub fn validate_dataset_manifest(manifest: &DatasetManifest) -> Result<()> {
    ensure_non_empty("dataset id", &manifest.id)?;
    ensure_non_empty("dataset title", &manifest.title)?;
    ensure_http_url("source_url", &manifest.source_url)?;
    ensure_non_empty("license", &manifest.license)?;
    ensure_non_empty("citation", &manifest.citation)?;
    ensure_non_empty("version", &manifest.version)?;
    ensure_non_empty("checksum_strategy", &manifest.checksum_strategy)?;
    ensure_non_empty("access_method", &manifest.access_method)?;
    ensure_non_empty("suite", &manifest.suite)?;
    if manifest.expected_columns.is_empty() {
        bail!("expected_columns must not be empty");
    }
    if manifest.unit_map.is_empty() {
        bail!("unit_map must not be empty");
    }
    if let Some(path) = &manifest.local_cache_path {
        reject_raw_path("local_cache_path", path)?;
    }
    Ok(())
}
