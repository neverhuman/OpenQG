use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Citation {
    pub title: String,
    pub authors: Vec<String>,
    pub year: u16,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParameterSpec {
    pub name: String,
    pub symbol: String,
    pub unit: String,
    pub physical_meaning: String,
    pub prior_or_fixed_value: String,
    pub source_or_free_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TheoryAdapter {
    pub command: String,
    #[serde(default)]
    pub predictions_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TheoryManifest {
    pub id: String,
    pub name: String,
    pub status: String,
    pub benchmark_suite: String,
    pub description: String,
    pub citations: Vec<Citation>,
    pub observables: Vec<String>,
    pub parameters: Vec<ParameterSpec>,
    pub adapter: TheoryAdapter,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DatasetManifest {
    pub id: String,
    pub title: String,
    pub source_url: String,
    pub license: String,
    pub citation: String,
    pub version: String,
    pub checksum_strategy: String,
    pub access_method: String,
    pub expected_columns: Vec<String>,
    pub unit_map: std::collections::BTreeMap<String, String>,
    pub suite: String,
    #[serde(default)]
    pub local_cache_path: Option<String>,
}
