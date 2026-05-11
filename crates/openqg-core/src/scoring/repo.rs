use crate::Scorecard;

pub fn repo_score_from_scorecard(scorecard: &Scorecard) -> (u32, String) {
    let mut raw = 100.0 * scorecard.metrics.coverage;
    raw += (scorecard.metrics.delta_log_likelihood.max(0.0) * 5.0).min(10.0);
    raw -= scorecard.metrics.parameter_count_penalty * 2.0;
    raw -= scorecard.metrics.invalid_prediction_count as f64 * 8.0;

    let score = raw.clamp(0.0, 100.0).round() as u32;
    let status = if score >= 85 {
        "pass".to_string()
    } else if score >= 65 {
        "warning".to_string()
    } else {
        "fail".to_string()
    };

    (score, status)
}
