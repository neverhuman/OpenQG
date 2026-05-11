use crate::util::{read_jsonl, write_generated_json};
use anyhow::Result;
use openqg_core::{BenchmarkSuite, ObservableRecord, PredictionRecord, Scorecard, TheoryManifest};
use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, serde::Deserialize)]
pub struct BenchmarkArtifact {
    pub suite: BenchmarkSuite,
    pub theory: TheoryManifest,
    pub observables: Vec<ObservableRecord>,
    pub predictions: Vec<PredictionRecord>,
    pub scorecard: Scorecard,
}

pub fn read_records(path: &Path) -> Result<Vec<ObservableRecord>> {
    read_jsonl(path)
}

pub fn load_predictions(path: Option<&Path>) -> Result<Vec<PredictionRecord>> {
    if let Some(path) = path {
        read_jsonl(path)
    } else {
        Ok(Vec::new())
    }
}

pub fn write_scorecard(output: &Path, scorecard: &Scorecard) -> Result<()> {
    write_generated_json(output, "openqg-bench", "just bench-smoke", scorecard)
}

pub fn write_predictions(parent: &Path, predictions: &[PredictionRecord]) -> Result<()> {
    let predictions_output = parent.join("predictions.jsonl");
    let payload = predictions
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<Vec<_>, _>>()?
        .join("\n");
    fs::write(predictions_output, format!("{payload}\n"))?;
    Ok(())
}
