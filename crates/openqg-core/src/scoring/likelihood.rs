use crate::{ObservableRecord, PredictionRecord, ScoreMetrics};
use std::collections::BTreeMap;

pub fn gaussian_log_likelihood(observed: f64, predicted: f64, sigma: f64) -> f64 {
    if !sigma.is_finite() || sigma <= 0.0 {
        return f64::NEG_INFINITY;
    }
    let residual = observed - predicted;
    -0.5 * (residual * residual) / (sigma * sigma)
}

pub fn score_metrics(
    observables: &[ObservableRecord],
    predictions: &[PredictionRecord],
    parameter_count: usize,
    baseline_log_likelihood: f64,
) -> (ScoreMetrics, Vec<String>) {
    let mut prediction_map: BTreeMap<&str, &PredictionRecord> = BTreeMap::new();
    for prediction in predictions {
        prediction_map.insert(prediction.observable_id.as_str(), prediction);
    }

    let mut log_likelihood = 0.0;
    let mut matched = 0usize;
    let mut findings = Vec::new();

    for observable in observables {
        match prediction_map.get(observable.observable_id.as_str()) {
            Some(prediction) => {
                matched += 1;
                let sigma = observable.uncertainty.max(prediction.uncertainty);
                log_likelihood +=
                    gaussian_log_likelihood(observable.value, prediction.value, sigma);
                if observable.unit.trim() != prediction.unit.trim() {
                    findings.push(format!(
                        "unit mismatch for {}: {} vs {}",
                        observable.observable_id, observable.unit, prediction.unit
                    ));
                }
            }
            None => findings.push(format!(
                "missing prediction for {}",
                observable.observable_id
            )),
        }
    }

    let observable_count = observables.len().max(1) as f64;
    let coverage = matched as f64 / observable_count;
    let k = parameter_count.max(1) as f64;
    let n = observables.len().max(1) as f64;
    let aic = 2.0 * k - 2.0 * log_likelihood;
    let bic = k * n.ln() - 2.0 * log_likelihood;
    let mdl = bic + (1.0 - coverage) * 10.0 + findings.len() as f64;
    let parameter_count_penalty = k.log10().max(0.0);

    let metrics = ScoreMetrics {
        log_likelihood,
        delta_log_likelihood: log_likelihood - baseline_log_likelihood,
        aic,
        bic,
        mdl,
        coverage,
        parameter_count_penalty,
        invalid_prediction_count: findings.len(),
    };

    (metrics, findings)
}
