pub mod audit_cascade;
pub mod calibration_envelope;
pub mod claims_linter;
pub mod data_tier;
pub mod engine_kpis;
pub mod evidence_receipt;
pub mod forecast_registry;
pub mod h0_panel;
pub mod hash;
pub mod knowledge;
pub mod manifest;
pub mod rigor;
pub mod search_statistics;
#[path = "runbook.rs"]
pub mod zyal;

pub use audit_cascade::{
    ai_for_science_baselines, v4_v7_audit_cascade, AuditCascadeEntry, AuditCascadeReport,
    BaselineEntry, BaselineKind, ExploitClass, HeadToHeadResult, TimeToInvalidate,
};
pub use calibration_envelope::{CalibrationEnvelope, PredictionResidual};
pub use claims_linter::{lint_claims, ClaimFinding, ClaimLintReport};
pub use data_tier::{
    check_value_firewall, DataTier, DataTierManifest, FirewallReport, FirewallViolation,
};
pub use engine_kpis::{EngineKpis, TimeToInvalidateStats};
pub use evidence_receipt::{
    cross_solver_tension, EvidenceReceipt, EvidenceRequest, NestingSolver, PriorDensity,
    PriorEntry, PriorSpec, PriorTransform, SamplerBackend, SamplerSpec,
};
pub use forecast_registry::{ForecastEntry, ForecastStatus};
pub use h0_panel::{
    canonical_h0_panel, load_h0_panel, score_h0_panel, ExternalPlausibilityReport,
    H0CalibratorFamily, H0FamilyResidual, H0PanelEntry, H0PanelScore,
};
pub use hash::*;
pub use knowledge::{
    append_lessons, distill_lessons, read_lessons, KillRecord, KnowledgeLesson, LessonConclusion,
    LessonConfidence, LessonScope, LessonTrigger, NumericOrigin, Predicate, RedactionState,
    RetrievalPacket, ValueOriginAudit,
};
pub use manifest::*;
pub use rigor::{text_rigor, trace_rigor, RigorFactor, RigorScore};
pub use search_statistics::{
    NullReplication, PostSearchVerdict, SearchNullDistribution, DISCOVERY_P_VALUE_THRESHOLD,
    MIN_NULL_REPLICATIONS,
};
pub use zyal::*;
