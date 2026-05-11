use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ObservableRecord {
    pub observable_id: String,
    pub kind: String,
    pub value: f64,
    pub uncertainty: f64,
    pub unit: String,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PredictionRecord {
    pub observable_id: String,
    pub value: f64,
    pub uncertainty: f64,
    pub unit: String,
    #[serde(default)]
    pub theory_id: Option<String>,
}
