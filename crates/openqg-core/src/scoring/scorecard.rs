use crate::{ObservableRecord, PredictionRecord, Scorecard};

use super::score_metrics;

pub fn scorecard_from_predictions(
    benchmark_version: String,
    suite_id: String,
    baseline_theory: String,
    candidate_theory: String,
    parameter_count: usize,
    observables: &[ObservableRecord],
    predictions: &[PredictionRecord],
    baseline_log_likelihood: f64,
    generated_at: String,
) -> Scorecard {
    let (metrics, findings) = score_metrics(
        observables,
        predictions,
        parameter_count,
        baseline_log_likelihood,
    );

    Scorecard {
        benchmark_version,
        suite_id,
        baseline_theory,
        candidate_theory,
        parameter_count,
        observable_count: observables.len(),
        matched_observable_count: observables
            .len()
            .saturating_sub(metrics.invalid_prediction_count),
        metrics,
        findings,
        generated_at,
    }
}
