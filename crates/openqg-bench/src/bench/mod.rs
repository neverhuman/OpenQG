mod artifact;
mod load;

use crate::util::generated_at;
use anyhow::Result;
use openqg_core::{
    scorecard_from_predictions, validate_observable_record, validate_prediction_record,
    ObservableRecord, PredictionRecord,
};
use std::path::{Path, PathBuf};

pub fn run(
    suite_path: &Path,
    theory_path: &Path,
    output: &Path,
    observables_override: Option<&Path>,
    predictions_override: Option<&Path>,
) -> Result<()> {
    let suite = load::suite(suite_path)?;
    let theory = load::theory(theory_path)?;

    let observables_path = if let Some(path) = observables_override {
        PathBuf::from(path)
    } else {
        PathBuf::from(&suite.fixture_observables)
    };
    let observables: Vec<ObservableRecord> = artifact::read_records(&observables_path)?;
    for observable in &observables {
        validate_observable_record(observable)?;
    }

    let predictions_path = if let Some(path) = predictions_override {
        Some(PathBuf::from(path))
    } else if let Some(path) = theory.adapter.predictions_path.as_deref() {
        Some(PathBuf::from(path))
    } else {
        suite.fixture_predictions.as_deref().map(PathBuf::from)
    };

    let predictions: Vec<PredictionRecord> =
        artifact::load_predictions(predictions_path.as_deref())?;
    for prediction in &predictions {
        validate_prediction_record(prediction)?;
    }

    let scorecard = scorecard_from_predictions(
        suite.benchmark_version.clone(),
        suite.id.clone(),
        suite.baseline_theory.clone(),
        theory.id.clone(),
        theory.parameters.len(),
        &observables,
        &predictions,
        0.0,
        generated_at(),
    );

    let artifact = artifact::BenchmarkArtifact {
        suite,
        theory,
        observables,
        predictions,
        scorecard,
    };
    artifact::write_scorecard(output, &artifact.scorecard)?;
    if let Some(parent) = output.parent() {
        artifact::write_predictions(parent, &artifact.predictions)?;
    }
    println!(
        "wrote scorecard for {} against {}",
        artifact.scorecard.candidate_theory, artifact.scorecard.suite_id
    );
    Ok(())
}
