use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BenchmarkSuite {
    pub id: String,
    pub title: String,
    pub description: String,
    pub benchmark_version: String,
    pub fixture_observables: String,
    pub fixture_predictions: Option<String>,
    pub baseline_theory: String,
    pub dataset_ids: Vec<String>,
    pub observables: Vec<String>,
    pub weights: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScoreMetrics {
    pub log_likelihood: f64,
    pub delta_log_likelihood: f64,
    pub aic: f64,
    pub bic: f64,
    pub mdl: f64,
    pub coverage: f64,
    pub parameter_count_penalty: f64,
    pub invalid_prediction_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Scorecard {
    pub benchmark_version: String,
    pub suite_id: String,
    pub baseline_theory: String,
    pub candidate_theory: String,
    pub parameter_count: usize,
    pub observable_count: usize,
    pub matched_observable_count: usize,
    pub metrics: ScoreMetrics,
    pub findings: Vec<String>,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RepoScore {
    pub repo: String,
    pub benchmark_version: String,
    pub suite_id: String,
    pub score: u32,
    pub status: String,
    pub summary: String,
    pub metrics: ScoreMetrics,
    pub findings: Vec<String>,
}
