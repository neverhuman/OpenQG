//! `openqg data audit` (v3.0.0 M6): verify every observational fixture against the sha256 recorded
//! in `data/manifest.json`, so a referee can prove the committed data has not drifted under the
//! reproducible-replay package. Output is deterministic (manifest order, no wall-clock/RNG) so it
//! can itself be diffed.

use anyhow::{bail, Context, Result};
use openqg_core::sha256_file;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// One fixture entry in `data/manifest.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ManifestFixture {
    pub path: String,
    pub sha256: String,
    pub kind: String,
    /// Covariance availability: `diagonal_only`, `block_available`, `is_covariance`,
    /// `not_applicable`, …
    pub covariance: String,
    #[serde(default)]
    pub covariance_note: String,
}

/// The data manifest.
#[derive(Debug, Clone, Deserialize)]
pub struct DataManifest {
    pub schema: String,
    #[serde(default)]
    pub description: String,
    pub fixtures: Vec<ManifestFixture>,
}

/// The per-fixture audit outcome.
#[derive(Debug, Clone, PartialEq)]
pub enum FixtureStatus {
    Ok { sha256: String },
    Drift { expected: String, got: String },
    Missing,
}

/// One audited fixture row (deterministic, manifest order).
#[derive(Debug, Clone)]
pub struct AuditRow {
    pub path: String,
    pub covariance: String,
    pub status: FixtureStatus,
}

/// Audit every fixture in `manifest`, resolving paths relative to `base`. Pure + deterministic so
/// it is unit-testable without touching the real tree.
pub fn audit_manifest(manifest: &DataManifest, base: &Path) -> Result<Vec<AuditRow>> {
    let mut rows = Vec::with_capacity(manifest.fixtures.len());
    for f in &manifest.fixtures {
        let p = base.join(&f.path);
        let status = if !p.exists() {
            FixtureStatus::Missing
        } else {
            let got = sha256_file(&p).with_context(|| format!("hashing {}", p.display()))?;
            if got == f.sha256 {
                FixtureStatus::Ok { sha256: got }
            } else {
                FixtureStatus::Drift {
                    expected: f.sha256.clone(),
                    got,
                }
            }
        };
        rows.push(AuditRow {
            path: f.path.clone(),
            covariance: f.covariance.clone(),
            status,
        });
    }
    Ok(rows)
}

/// CLI entry: load `data/manifest.json` (or a given path), audit, print, and fail on any
/// drift/missing fixture.
pub fn audit(manifest_path: &Path) -> Result<()> {
    let text = std::fs::read_to_string(manifest_path)
        .with_context(|| format!("read manifest {}", manifest_path.display()))?;
    let manifest: DataManifest = serde_json::from_str(&text)
        .with_context(|| format!("parse manifest {}", manifest_path.display()))?;
    // Manifest paths are repo-relative; resolve from the current working directory.
    let base = Path::new(".");
    let rows = audit_manifest(&manifest, base)?;

    println!(
        "data audit: {} ({} fixtures)",
        manifest.schema,
        rows.len()
    );
    let mut drift = 0usize;
    let mut missing = 0usize;
    for r in &rows {
        match &r.status {
            FixtureStatus::Ok { sha256 } => {
                println!("  ok     {}  {:48}  cov={}", &sha256[..12], r.path, r.covariance)
            }
            FixtureStatus::Drift { expected, got } => {
                drift += 1;
                println!(
                    "  DRIFT  {}  expected {}.. got {}..",
                    r.path,
                    &expected[..12.min(expected.len())],
                    &got[..12.min(got.len())]
                );
            }
            FixtureStatus::Missing => {
                missing += 1;
                println!("  MISSING  {}", r.path);
            }
        }
    }
    if drift > 0 || missing > 0 {
        bail!("data audit FAILED: {drift} drifted, {missing} missing");
    }
    println!("data audit OK: all {} fixtures match the manifest", rows.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write(dir: &Path, rel: &str, content: &str) -> String {
        let p = dir.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, content).unwrap();
        sha256_file(&p).unwrap()
    }

    #[test]
    fn audit_passes_when_hashes_match() {
        let dir = tempfile::tempdir().unwrap();
        let h = write(dir.path(), "data/x.jsonl", "{\"a\":1}\n");
        let manifest = DataManifest {
            schema: "test.v1".into(),
            description: String::new(),
            fixtures: vec![ManifestFixture {
                path: "data/x.jsonl".into(),
                sha256: h.clone(),
                kind: "test".into(),
                covariance: "none".into(),
                covariance_note: String::new(),
            }],
        };
        let rows = audit_manifest(&manifest, dir.path()).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].status, FixtureStatus::Ok { sha256: h });
    }

    #[test]
    fn audit_detects_drift_and_missing() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "data/x.jsonl", "{\"a\":1}\n");
        let manifest = DataManifest {
            schema: "test.v1".into(),
            description: String::new(),
            fixtures: vec![
                ManifestFixture {
                    path: "data/x.jsonl".into(),
                    sha256: "deadbeef".repeat(8), // wrong hash
                    kind: "test".into(),
                    covariance: "none".into(),
                    covariance_note: String::new(),
                },
                ManifestFixture {
                    path: "data/gone.jsonl".into(),
                    sha256: "0".repeat(64),
                    kind: "test".into(),
                    covariance: "none".into(),
                    covariance_note: String::new(),
                },
            ],
        };
        let rows = audit_manifest(&manifest, dir.path()).unwrap();
        assert!(matches!(rows[0].status, FixtureStatus::Drift { .. }));
        assert_eq!(rows[1].status, FixtureStatus::Missing);
    }

    #[test]
    fn the_real_repo_manifest_audits_clean() {
        // The committed data/manifest.json must match the committed fixtures (the reproducibility
        // contract). Resolve paths from the crate's repo root.
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let text = fs::read_to_string(repo.join("data/manifest.json")).unwrap();
        let manifest: DataManifest = serde_json::from_str(&text).unwrap();
        let rows = audit_manifest(&manifest, &repo).unwrap();
        for r in &rows {
            assert!(
                matches!(r.status, FixtureStatus::Ok { .. }),
                "fixture {} failed audit: {:?}",
                r.path,
                r.status
            );
        }
        assert!(rows.len() >= 8, "manifest should list the cosmology fixtures");
    }
}
