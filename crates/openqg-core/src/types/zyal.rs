use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZyalPreview {
    pub file: String,
    pub id: String,
    pub name: String,
    pub job_name: String,
    pub armed: bool,
    pub research_enabled: bool,
    pub permission_summary: String,
    pub stop_summary: String,
    pub valid: bool,
    pub strict_valid: bool,
    pub warnings: Vec<String>,
}
