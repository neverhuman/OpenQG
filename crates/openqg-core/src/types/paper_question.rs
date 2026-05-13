use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaperIdentifier {
    pub kind: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaperSection {
    pub id: String,
    pub heading: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaperBodyRecord {
    pub publication_hash: String,
    pub body_hash: String,
    pub title: String,
    pub authors: Vec<String>,
    pub identifiers: Vec<PaperIdentifier>,
    pub license: String,
    pub oa_proof: String,
    pub source_urls: Vec<String>,
    pub extraction_receipts: Vec<String>,
    pub sections: Vec<PaperSection>,
    pub body_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RouteMetadata {
    pub request_id: Option<String>,
    pub route_mode: Option<String>,
    pub primary_model_id: Option<String>,
    pub backup_model_ids: Vec<String>,
    pub fusion_model_id: Option<String>,
    pub winner_model_id: Option<String>,
    pub confidence: Option<f64>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub agent_role: String,
    pub zyal_run_id: String,
    pub zyal_lane_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentAttemptRecord {
    pub agent_id: String,
    pub role: String,
    pub answer: Option<String>,
    pub score: Option<f64>,
    pub route_metadata: RouteMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContextPackSettings {
    pub strategy: String,
    pub target_fill_ratio: f64,
    pub output_reserve_tokens: u64,
    pub safe_window_tokens: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AcceptanceDecision {
    pub accepted: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChallengeRecord {
    pub challenge_hash: String,
    pub publication_hash: String,
    pub rubric_version: String,
    pub question: String,
    pub answer_key: String,
    pub support_sections: Vec<String>,
    pub context_pack: ContextPackSettings,
    pub generator_agents: Vec<AgentAttemptRecord>,
    pub blind_answer_attempts: Vec<AgentAttemptRecord>,
    pub critic_attempts: Vec<AgentAttemptRecord>,
    pub audit_attempts: Vec<AgentAttemptRecord>,
    pub acceptance: AcceptanceDecision,
}
