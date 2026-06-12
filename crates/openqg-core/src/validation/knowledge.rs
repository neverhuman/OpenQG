//! V8 Phase 3 (#16): Knowledge Lesson store.
//!
//! A `KnowledgeLesson` is a deterministic, cross-campaign distillation: "after K observed kills
//! matching pattern P, the proposer should always / never try X". Lessons are mined from the
//! kill ledger after each campaign, persisted in JSONL, and injected into GP run manifests
//! as retrieval context.
//!
//! Lesson IDs are SHA-256 hashes of the canonical predicate tuple so identical patterns from
//! different campaigns produce the same lesson (dedup-stable).
//!
//! Spec reference: S09 §5-6.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{BufRead, Write};

/// Scope of a lesson — what kind of thing it applies to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LessonScope {
    /// Applies to a specific relation (e.g. `dark_scattering_growth_drag`).
    Relation { relation: String },
    /// Applies to any theory targeting a specific observable set.
    ObservableSet { observable_ids: Vec<String> },
    /// Applies to a mechanism class (e.g. screening, running_coupling, brane_gravity).
    MechanismClass { class: String },
    /// Applies globally across all campaigns (very high support only).
    Global,
}

/// What conditions must hold in an incoming context for this lesson to fire.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LessonTrigger {
    /// The proposer is attempting the named relation.
    RelationAttempted { relation: String },
    /// A specific parameter is in a value range (exclusive bounds).
    ParameterInRange { symbol: String, lo: f64, hi: f64 },
    /// Kill count for this relation in the current campaign exceeds threshold.
    KillCountAbove { relation: String, threshold: u32 },
    /// The observable pull for a named observable is above a threshold.
    PullAbove {
        observable_id: String,
        pull_sigma: f64,
    },
    /// Composite AND of inner triggers.
    All { triggers: Vec<LessonTrigger> },
    /// Composite OR of inner triggers.
    Any { triggers: Vec<LessonTrigger> },
}

/// The machine-authored conclusion of a lesson.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LessonConclusion {
    /// Avoid this relation in future proposals.
    AvoidRelation { relation: String, reason: String },
    /// Clamp the named parameter to a range in proposals.
    ClampParameter {
        symbol: String,
        lo: f64,
        hi: f64,
        reason: String,
    },
    /// Increase search time on the named observable.
    FocusObservable {
        observable_id: String,
        priority_boost: f64,
    },
    /// Prefer this prior family for the named parameter.
    PreferPrior {
        symbol: String,
        prior_family: String,
        reason: String,
    },
    /// Informational: record what pattern was observed.
    Informational { summary: String },
}

/// Confidence level of a lesson (gated by support and lift).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LessonConfidence {
    Tentative,  // support < 5 or lift < 1.2
    Moderate,   // support >= 5 and lift >= 1.2
    Strong,     // support >= 20 and lift >= 2.0
    Definitive, // support >= 50 and lift >= 3.0 and no counter-examples
}

impl LessonConfidence {
    pub fn from_support_lift(support: u32, lift: f64) -> Self {
        if support >= 50 && lift >= 3.0 {
            LessonConfidence::Definitive
        } else if support >= 20 && lift >= 2.0 {
            LessonConfidence::Strong
        } else if support >= 5 && lift >= 1.2 {
            LessonConfidence::Moderate
        } else {
            LessonConfidence::Tentative
        }
    }
}

/// Whether a lesson's numeric values are free of contamination from sealed data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RedactionState {
    Clean,
    Redacted { sealed_entries_removed: u32 },
    ContaminationSuspected,
}

/// A single predicate in a lesson's trigger/evidence tuple.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Predicate {
    pub key: String,
    pub operator: String,
    pub value: serde_json::Value,
}

/// The origin of a numeric literal in a lesson or retrieval packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NumericOrigin {
    /// A coefficient that appears in a derived equation (safe for certificates).
    EquationCoefficient,
    /// A definition constant (e.g. `G_N = 1` in Planck units).
    DefinitionConstant,
    /// A physical constant from the constant registry (e.g. `c`, `h_bar`).
    PhysicalConstantRegistry,
    /// The posterior mean/mode of a fitted parameter.
    PosteriorValue,
    /// A value from the observed-data vector (never allowed in certificate inputs).
    ObservedDataValue,
    /// An entry in a covariance matrix.
    CovarianceEntry,
    /// The best-fit value from the champion run.
    ChampionFitValue,
    /// Origin unknown or not yet classified.
    UnknownNumeric,
}

impl NumericOrigin {
    /// True when this origin is allowed to appear in a `DerivedCertificate`.
    pub fn allowed_in_certificate(&self) -> bool {
        matches!(
            self,
            NumericOrigin::EquationCoefficient
                | NumericOrigin::DefinitionConstant
                | NumericOrigin::PhysicalConstantRegistry
        )
    }

    /// True when this origin is safe to include in a proposer prompt (contamination firewall).
    pub fn allowed_in_prompt(&self) -> bool {
        matches!(
            self,
            NumericOrigin::EquationCoefficient
                | NumericOrigin::DefinitionConstant
                | NumericOrigin::PhysicalConstantRegistry
        )
    }
}

/// Contamination audit for one numeric literal that appears in a retrieval packet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueOriginAudit {
    pub numeric_literal: f64,
    pub origin: NumericOrigin,
    /// Which chunk or retrieval source provided this literal.
    pub source_chunk_id: String,
    pub allowed_in_certificate: bool,
    pub allowed_in_prompt: bool,
    /// Non-empty when the value was redacted from the packet.
    pub redaction_id: String,
}

impl ValueOriginAudit {
    pub fn new(literal: f64, origin: NumericOrigin, source_chunk_id: impl Into<String>) -> Self {
        ValueOriginAudit {
            numeric_literal: literal,
            allowed_in_certificate: origin.allowed_in_certificate(),
            allowed_in_prompt: origin.allowed_in_prompt(),
            origin,
            source_chunk_id: source_chunk_id.into(),
            redaction_id: String::new(),
        }
    }

    pub fn needs_redaction(&self) -> bool {
        !self.allowed_in_prompt
    }
}

/// A persisted knowledge lesson distilled from multiple campaign kill records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeLesson {
    /// SHA-256 of the canonical predicate tuple (dedup-stable across campaigns).
    pub lesson_id: String,
    /// Run IDs this lesson was distilled from.
    pub created_from_runs: Vec<String>,
    pub scope: LessonScope,
    pub trigger: LessonTrigger,
    pub predicates: Vec<Predicate>,
    pub conclusion: LessonConclusion,
    /// Host-authored template text safe for injection into proposer prompts.
    pub host_template_text: String,
    /// Number of kill events matching this lesson's pattern.
    pub support: u32,
    /// Rate ratio vs background rate (support/total relative to baseline rate).
    pub lift_vs_background: f64,
    /// Median score delta (lessons that kill high-scoring theories earn more credit).
    pub median_score_delta: f64,
    /// Which observables are most affected.
    pub affected_observables: Vec<String>,
    /// Evidence attempt IDs that contributed to this lesson.
    pub evidence_attempt_ids: Vec<String>,
    /// Retrieval IDs (chunks) pulled during distillation.
    pub retrieval_ids: Vec<String>,
    pub confidence: LessonConfidence,
    /// If set: invalidate this lesson when the snapshot hash changes.
    pub valid_until_snapshot: Option<String>,
    /// Lesson IDs superseded by this one.
    pub supersedes: Vec<String>,
    pub contamination_state: RedactionState,
}

impl KnowledgeLesson {
    /// Compute the canonical lesson ID from the stable predicate tuple.
    ///
    /// The ID is a SHA-256 of (scope, trigger) — the conclusion is derived from these and
    /// must not include support-count or per-campaign text, so the same pattern from any
    /// number of campaigns produces the same ID (dedup-stable).
    pub fn compute_lesson_id(scope: &LessonScope, trigger: &LessonTrigger) -> String {
        let key = serde_json::json!({
            "scope": scope,
            "trigger": trigger,
        });
        let canonical = serde_json::to_string(&key).unwrap_or_default();
        let hash = Sha256::digest(canonical.as_bytes());
        format!("lesson:{hash:x}")
    }

    /// True when this lesson is strong enough to be injected into a GP run manifest.
    pub fn is_promotable(&self) -> bool {
        self.confidence >= LessonConfidence::Moderate
            && !matches!(
                self.contamination_state,
                RedactionState::ContaminationSuspected
            )
    }
}

/// A batch of kill events from which lessons can be distilled.
#[derive(Debug, Clone)]
pub struct KillRecord {
    pub run_id: String,
    pub relation: String,
    pub veto_reason: String,
    pub score_at_kill: f64,
    pub affected_observables: Vec<String>,
}

/// Distill lessons from a set of kill records.
///
/// Groups by (relation, veto_reason_class), emits a lesson when the group's support
/// meets the minimum threshold.
pub fn distill_lessons(kills: &[KillRecord], min_support: u32) -> Vec<KnowledgeLesson> {
    use std::collections::HashMap;
    let mut groups: HashMap<(String, String), Vec<&KillRecord>> = HashMap::new();

    for kill in kills {
        let key = (kill.relation.clone(), veto_class(&kill.veto_reason));
        groups.entry(key).or_default().push(kill);
    }

    let total = kills.len() as f64;
    let mut lessons = Vec::new();

    for ((relation, veto_class), records) in groups {
        let support = records.len() as u32;
        if support < min_support {
            continue;
        }
        let background_rate = 0.1_f64.max(1.0 / total.max(1.0));
        let lift = (support as f64 / total) / background_rate;

        let run_ids: Vec<String> = records.iter().map(|r| r.run_id.clone()).collect();
        let obs: Vec<String> = records
            .iter()
            .flat_map(|r| r.affected_observables.iter().cloned())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        let score_deltas: Vec<f64> = records.iter().map(|r| r.score_at_kill).collect();
        let median = {
            let mut s = score_deltas.clone();
            s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            s[s.len() / 2]
        };

        let scope = LessonScope::Relation {
            relation: relation.clone(),
        };
        let trigger = LessonTrigger::RelationAttempted {
            relation: relation.clone(),
        };
        let conclusion = LessonConclusion::AvoidRelation {
            relation: relation.clone(),
            reason: format!(
                "killed {support} times with veto class '{veto_class}'; median score at kill = {median:.1}"
            ),
        };

        let lesson_id = KnowledgeLesson::compute_lesson_id(&scope, &trigger);
        let confidence = LessonConfidence::from_support_lift(support, lift);

        let host_template = format!(
            "NOTE: relation '{relation}' has been killed {support} times by '{veto_class}'. \
             Unless the proposal explicitly addresses this veto, avoid '{relation}'."
        );

        lessons.push(KnowledgeLesson {
            lesson_id,
            created_from_runs: run_ids,
            scope,
            trigger,
            predicates: vec![],
            conclusion,
            host_template_text: host_template,
            support,
            lift_vs_background: lift,
            median_score_delta: median,
            affected_observables: obs,
            evidence_attempt_ids: vec![],
            retrieval_ids: vec![],
            confidence,
            valid_until_snapshot: None,
            supersedes: vec![],
            contamination_state: RedactionState::Clean,
        });
    }

    lessons
}

fn veto_class(reason: &str) -> String {
    if reason.contains("anti-laundering") {
        "anti_laundering".into()
    } else if reason.contains("free parameter") || reason.contains("FreeParameter") {
        "free_parameter".into()
    } else if reason.contains("ungenerated") || reason.contains("StructurallyUngenerated") {
        "structurally_ungenerated".into()
    } else if reason.contains("rigor") || reason.contains("Rigor") {
        "low_rigor".into()
    } else {
        "other".into()
    }
}

/// Append lessons to a JSONL file (one JSON line per lesson).
pub fn append_lessons(path: &std::path::Path, lessons: &[KnowledgeLesson]) -> std::io::Result<()> {
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    let mut writer = std::io::BufWriter::new(file);
    for lesson in lessons {
        let line = serde_json::to_string(lesson)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        writeln!(writer, "{line}")?;
    }
    Ok(())
}

/// Read all lessons from a JSONL file.
pub fn read_lessons(path: &std::path::Path) -> std::io::Result<Vec<KnowledgeLesson>> {
    if !path.exists() {
        return Ok(vec![]);
    }
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let mut lessons = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let lesson: KnowledgeLesson = serde_json::from_str(&line)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        lessons.push(lesson);
    }
    Ok(lessons)
}

/// Retrieval packet injected into the proposer context — contamination-filtered.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalPacket {
    pub packet_id: String,
    pub lessons: Vec<KnowledgeLesson>,
    pub value_audits: Vec<ValueOriginAudit>,
    /// Snapshot hash at retrieval time.
    pub snapshot_hash: String,
}

impl RetrievalPacket {
    /// True when any value in the packet is contaminated (not safe for prompt injection).
    pub fn has_contamination(&self) -> bool {
        self.value_audits.iter().any(|a| a.needs_redaction())
    }

    /// Lessons safe to inject into a GP run manifest.
    pub fn promotable_lessons(&self) -> Vec<&KnowledgeLesson> {
        self.lessons.iter().filter(|l| l.is_promotable()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_kills(relation: &str, count: usize, reason: &str) -> Vec<KillRecord> {
        (0..count)
            .map(|i| KillRecord {
                run_id: format!("run-{i:03}"),
                relation: relation.into(),
                veto_reason: reason.into(),
                score_at_kill: 45.0 + i as f64 * 0.1,
                affected_observables: vec!["fsigma8".into()],
            })
            .collect()
    }

    /// Spec acceptance test (S09 rank 5): ≥20 constant dark-scattering kills → deterministic lesson.
    #[test]
    fn dark_scattering_kills_produce_lesson() {
        let kills = make_kills(
            "dark_scattering_growth_drag",
            22,
            "anti-laundering: A_drag is a free numerical coefficient",
        );
        let lessons = distill_lessons(&kills, 5);
        assert_eq!(lessons.len(), 1, "should produce exactly one lesson");
        let l = &lessons[0];
        assert_eq!(l.support, 22);
        assert!(l.lift_vs_background > 1.0);
        assert!(l.lesson_id.starts_with("lesson:"));
        assert!(
            l.confidence >= LessonConfidence::Strong,
            "22 kills should yield Strong confidence; got {:?}",
            l.confidence
        );
        assert!(l.is_promotable());
    }

    /// Lesson IDs are deterministic: same pattern from two different campaigns produces same ID.
    #[test]
    fn lesson_id_is_deterministic() {
        let kills1 = make_kills("ndgp_geff_over_g", 8, "structurally_ungenerated");
        let kills2 = make_kills("ndgp_geff_over_g", 12, "structurally_ungenerated");
        let l1 = distill_lessons(&kills1, 5);
        let l2 = distill_lessons(&kills2, 5);
        assert_eq!(
            l1[0].lesson_id, l2[0].lesson_id,
            "same scope+trigger+conclusion must produce same lesson ID"
        );
    }

    #[test]
    fn lesson_below_min_support_not_emitted() {
        let kills = make_kills("fr_alpha_m", 3, "low_rigor");
        let lessons = distill_lessons(&kills, 5);
        assert!(
            lessons.is_empty(),
            "support=3 < min_support=5 must not emit a lesson"
        );
    }

    #[test]
    fn lesson_roundtrips_jsonl() {
        let kills = make_kills(
            "coupled_de_geff_over_g",
            10,
            "FreeParameter: beta not derived",
        );
        let lessons = distill_lessons(&kills, 5);
        assert_eq!(lessons.len(), 1);

        let tmp = std::env::temp_dir().join("openqg-lesson-test.jsonl");
        append_lessons(&tmp, &lessons).unwrap();
        let loaded = read_lessons(&tmp).unwrap();
        std::fs::remove_file(&tmp).ok();

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].lesson_id, lessons[0].lesson_id);
        assert_eq!(loaded[0].support, 10);
    }

    #[test]
    fn numeric_origin_firewall_blocks_observed_data() {
        let audit = ValueOriginAudit::new(67.4, NumericOrigin::ObservedDataValue, "chunk:h0-obs");
        assert!(!audit.allowed_in_certificate);
        assert!(!audit.allowed_in_prompt);
        assert!(audit.needs_redaction());
    }

    #[test]
    fn numeric_origin_firewall_allows_equation_coefficients() {
        let audit = ValueOriginAudit::new(
            1.0 / 3.0,
            NumericOrigin::EquationCoefficient,
            "chunk:ndgp-derivation",
        );
        assert!(audit.allowed_in_certificate);
        assert!(audit.allowed_in_prompt);
        assert!(!audit.needs_redaction());
    }

    #[test]
    fn lesson_confidence_levels_ordered_correctly() {
        assert!(LessonConfidence::Definitive > LessonConfidence::Strong);
        assert!(LessonConfidence::Strong > LessonConfidence::Moderate);
        assert!(LessonConfidence::Moderate > LessonConfidence::Tentative);
        assert_eq!(
            LessonConfidence::from_support_lift(50, 3.5),
            LessonConfidence::Definitive
        );
        assert_eq!(
            LessonConfidence::from_support_lift(20, 2.0),
            LessonConfidence::Strong
        );
        assert_eq!(
            LessonConfidence::from_support_lift(5, 1.5),
            LessonConfidence::Moderate
        );
        assert_eq!(
            LessonConfidence::from_support_lift(2, 0.8),
            LessonConfidence::Tentative
        );
    }
}
