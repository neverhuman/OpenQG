use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZyalPreview {
    pub file: String,
    pub name: String,
    pub valid: bool,
    pub warnings: Vec<String>,
}
