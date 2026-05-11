use anyhow::{Context, Result};
use openqg_core::{validate_theory_manifest, BenchmarkSuite, TheoryManifest};
use std::fs;
use std::path::Path;

pub fn suite(path: &Path) -> Result<BenchmarkSuite> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    serde_yaml::from_str::<BenchmarkSuite>(&text)
        .with_context(|| format!("parse {}", path.display()))
}

pub fn theory(path: &Path) -> Result<TheoryManifest> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let theory = serde_yaml::from_str::<TheoryManifest>(&text)
        .with_context(|| format!("parse {}", path.display()))?;
    validate_theory_manifest(&theory)?;
    Ok(theory)
}
