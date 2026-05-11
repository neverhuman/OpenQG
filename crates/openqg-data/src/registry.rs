use anyhow::{Context, Result};
use openqg_core::{sha256_file, validate_dataset_manifest, DatasetManifest};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DataLockEntry {
    pub path: String,
    pub sha256: String,
    pub dataset_id: String,
    pub suite: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DataLock {
    pub generated_at: String,
    pub entries: Vec<DataLockEntry>,
}

pub fn discover_dataset_manifests(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if !root.exists() {
        return Ok(files);
    }
    for entry in walkdir::WalkDir::new(root) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("yml")
                || path.extension().and_then(|s| s.to_str()) == Some("yaml")
            {
                files.push(path.to_path_buf());
            }
        }
    }
    files.sort();
    Ok(files)
}

pub fn load_dataset_manifest(path: &Path) -> Result<DatasetManifest> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let manifest: DatasetManifest =
        serde_yaml::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
    validate_dataset_manifest(&manifest)?;
    Ok(manifest)
}

pub fn validate_registry(root: &Path) -> Result<Vec<DatasetManifest>> {
    let mut manifests = Vec::new();
    for path in discover_dataset_manifests(root)? {
        manifests.push(load_dataset_manifest(&path)?);
    }
    Ok(manifests)
}

pub fn write_data_lock(root: &Path, output: &Path, generated_at: String) -> Result<DataLock> {
    let mut entries = Vec::new();
    for path in discover_dataset_manifests(root)? {
        let manifest = load_dataset_manifest(&path)?;
        entries.push(DataLockEntry {
            path: path.to_string_lossy().to_string(),
            sha256: sha256_file(&path)?,
            dataset_id: manifest.id,
            suite: manifest.suite,
        });
    }
    let lock = DataLock {
        generated_at,
        entries,
    };

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output, serde_json::to_vec_pretty(&lock)?)?;
    Ok(lock)
}
