use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReleaseManifest {
    pub version: String,
    pub benchmark_version: String,
    pub scorecard_hash: String,
    pub data_lock_hash: String,
    pub candidate_hashes: BTreeMap<String, String>,
    pub scorecard_path: String,
    pub approved: bool,
}
