//! V8 Phase 4 (#20): Audit cascade metrics.
//!
//! Converts the "exploit → regression test" cadence from a narrative into a field-standard
//! quantitative table. Each entry records: when an exploit first appeared, when the audit
//! detected it, how many campaigns it survived, the point/nat bounty, the fix commit, and
//! the regression test that now locks it permanently.
//!
//! The full V4–V7 audit trail is initialized from known history. New entries are added as
//! future exploits are discovered and regression-locked.
//!
//! Spec reference: S11 §"The exploit→regression-test audit cadence"

use serde::{Deserialize, Serialize};

/// Class of exploit the audit cascade discovered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExploitClass {
    /// Rubric-gaming: theory maximizes scorecard points without physical improvement.
    RubricGaming,
    /// Unpriced drift: free background parameter drifts without DOF charge.
    UnpricedDrift,
    /// Novelty laundering: score credit from a data-contaminated or instrument-biased witness.
    NoveltyLaundering,
    /// Missing generating term: a derived relation claims credit without an action term.
    MissingGeneratingTerm,
    /// Evaluator bias: the oracle itself has a calibration error exploited by the proposer.
    EvaluatorBias,
    /// Anti-laundering bypass: a certificate input was a fitted posterior value.
    AntiLaunderingBypass,
    /// Structural forgery: right-named term with wrong dimension or free index count.
    StructuralForgery,
    /// Tie credit: claiming model improvement when tied with ΛCDM at N sigma.
    TieCredit,
}

impl ExploitClass {
    pub fn label(&self) -> &'static str {
        match self {
            ExploitClass::RubricGaming => "rubric-gaming",
            ExploitClass::UnpricedDrift => "unpriced-drift",
            ExploitClass::NoveltyLaundering => "novelty-laundering",
            ExploitClass::MissingGeneratingTerm => "missing-generating-term",
            ExploitClass::EvaluatorBias => "evaluator-bias",
            ExploitClass::AntiLaunderingBypass => "anti-laundering-bypass",
            ExploitClass::StructuralForgery => "structural-forgery",
            ExploitClass::TieCredit => "tie-credit",
        }
    }
}

/// One entry in the audit cascade ledger.
///
/// Tracks an exploit from first appearance to regression lock.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditCascadeEntry {
    /// Unique exploit identifier (kebab-case).
    pub exploit_id: String,
    /// Version (V4/V5/V6/V6.1/V7/V8) when this exploit first appeared.
    pub first_appearance_version: String,
    /// Version when the audit detected this exploit.
    pub audit_detection_version: String,
    /// Number of campaigns the exploit survived before detection.
    pub campaigns_survived: u32,
    /// Score credit the exploit was extracting (scorecard points).
    pub bounty_scorecard_points: f64,
    /// Evidence credit the exploit was extracting (spurious lnZ units).
    pub bounty_ln_z: Option<f64>,
    pub exploit_class: ExploitClass,
    /// Human-readable description of what was being exploited.
    pub exploit_description: String,
    /// The fix (rule added, code changed, registry updated).
    pub fix_description: String,
    /// Git commit SHA of the fix (or branch/PR reference).
    pub fix_commit: String,
    /// Name of the regression test that locks this exploit.
    pub regression_test_name: String,
    /// True when the regression test is currently in the test suite.
    pub regression_locked: bool,
}

/// Time-to-invalidate metric for one exploit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeToInvalidate {
    pub exploit_id: String,
    pub first_appearance_version: String,
    pub audit_detection_version: String,
    pub campaigns_survived: u32,
    pub bounty_scorecard_points: f64,
    pub regression_test_name: String,
}

/// The full audit cascade report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditCascadeReport {
    pub entries: Vec<AuditCascadeEntry>,
    /// Version of the engine at report time.
    pub engine_version: String,
}

impl AuditCascadeReport {
    /// Median time-to-invalidate (campaigns survived) across all entries.
    pub fn median_time_to_invalidate(&self) -> Option<u32> {
        if self.entries.is_empty() {
            return None;
        }
        let mut times: Vec<u32> = self.entries.iter().map(|e| e.campaigns_survived).collect();
        times.sort_unstable();
        Some(times[times.len() / 2])
    }

    /// Total exploit bounty extracted before the audit caught each exploit.
    pub fn total_bounty_points(&self) -> f64 {
        self.entries.iter().map(|e| e.bounty_scorecard_points).sum()
    }

    /// Total spurious evidence before correction.
    pub fn total_bounty_ln_z(&self) -> f64 {
        self.entries.iter().filter_map(|e| e.bounty_ln_z).sum()
    }

    /// Entries by exploit class.
    pub fn by_class(&self, class: ExploitClass) -> Vec<&AuditCascadeEntry> {
        self.entries
            .iter()
            .filter(|e| e.exploit_class == class)
            .collect()
    }

    /// Fraction of entries that are regression-locked.
    pub fn regression_locked_fraction(&self) -> f64 {
        if self.entries.is_empty() {
            return 0.0;
        }
        let locked = self.entries.iter().filter(|e| e.regression_locked).count();
        locked as f64 / self.entries.len() as f64
    }

    /// All `TimeToInvalidate` metrics, sorted by campaigns_survived.
    pub fn time_to_invalidate_metrics(&self) -> Vec<TimeToInvalidate> {
        let mut metrics: Vec<TimeToInvalidate> = self
            .entries
            .iter()
            .map(|e| TimeToInvalidate {
                exploit_id: e.exploit_id.clone(),
                first_appearance_version: e.first_appearance_version.clone(),
                audit_detection_version: e.audit_detection_version.clone(),
                campaigns_survived: e.campaigns_survived,
                bounty_scorecard_points: e.bounty_scorecard_points,
                regression_test_name: e.regression_test_name.clone(),
            })
            .collect();
        metrics.sort_by_key(|m| m.campaigns_survived);
        metrics
    }
}

/// Return the known V4–V7 audit cascade history, initialized from campaign records.
///
/// This is the machine-readable version of the audit trail in `paper/main.tex` and
/// `paper/data/story.json`. New entries are added as future exploits are discovered.
pub fn v4_v7_audit_cascade() -> AuditCascadeReport {
    AuditCascadeReport {
        engine_version: "V8".into(),
        entries: vec![
            AuditCascadeEntry {
                exploit_id: "v4-rubric-gaming-87.5".into(),
                first_appearance_version: "V4".into(),
                audit_detection_version: "V4".into(),
                campaigns_survived: 0,
                bounty_scorecard_points: 87.5,
                bounty_ln_z: None,
                exploit_class: ExploitClass::RubricGaming,
                exploit_description: concat!(
                    "V4 champion reached 87.5 via rediscovery/tie-credit exploits ",
                    "without genuine evidence improvement over ΛCDM; rubric awarded ",
                    "novelty credit for fit-set improvement without trials correction."
                ).into(),
                fix_description: concat!(
                    "Added tie-with-ΛCDM-scores-zero rule; required trials correction ",
                    "for any BEATS_ΛCDM claim; novelty capped on fit-set data."
                ).into(),
                fix_commit: "feat/v5-anti-gaming".into(),
                regression_test_name: "decoy_fit_only_claim_is_vetoed".into(),
                regression_locked: true,
            },
            AuditCascadeEntry {
                exploit_id: "v5-unpriced-drift-55.0".into(),
                first_appearance_version: "V5".into(),
                audit_detection_version: "V5".into(),
                campaigns_survived: 0,
                bounty_scorecard_points: 55.0,
                bounty_ln_z: None,
                exploit_class: ExploitClass::UnpricedDrift,
                exploit_description: concat!(
                    "V5 champion used unpriced background drift: free w0/wa drifted ",
                    "without DOF charge, and a bare screening declaration bypassed the ",
                    "mechanism-derivation requirement."
                ).into(),
                fix_description: concat!(
                    "Added DOF charge for all background extensions; introduced FreeParameter ",
                    "veto for undeclared screening; introduced screening_recovery structural check."
                ).into(),
                fix_commit: "feat/v6-dof-pricing".into(),
                regression_test_name: "decoy_right_named_no_action_is_vetoed".into(),
                regression_locked: true,
            },
            AuditCascadeEntry {
                exploit_id: "v6-evaluator-bias-lA".into(),
                first_appearance_version: "V6".into(),
                audit_detection_version: "V6".into(),
                campaigns_survived: 0,
                bounty_scorecard_points: 77.0,
                bounty_ln_z: Some(35.0),
                exploit_class: ExploitClass::EvaluatorBias,
                exploit_description: concat!(
                    "V6 champion exploited a +0.755 systematic bias in the internal CMB ",
                    "acoustic-scale fitting formula (lA offset ≈ 8.4σ under Planck-prior σ=0.090). ",
                    "The apparent evidence valley along h–Ωm was an instrument artifact; ",
                    "after calibration the valley closed (~35 lnZ-units of spurious evidence removed)."
                ).into(),
                fix_description: concat!(
                    "Installed lA calibration correction; reduced fitting-formula CMB credit; ",
                    "required calibration-envelope residual for any CMB/lensing novelty claim."
                ).into(),
                fix_commit: "feat/v7-calibration-envelope".into(),
                regression_test_name: "calibration_envelope_covers_searched_space".into(),
                regression_locked: true,
            },
            AuditCascadeEntry {
                exploit_id: "v6.1-missing-generating-term".into(),
                first_appearance_version: "V6".into(),
                audit_detection_version: "V6".into(),
                campaigns_survived: 0,
                bounty_scorecard_points: 0.0,
                bounty_ln_z: None,
                exploit_class: ExploitClass::MissingGeneratingTerm,
                exploit_description: concat!(
                    "V6.1 survivor claimed a derived relation credit (nDGP-style G_eff/G) ",
                    "without a corresponding action term (BraneTerm::NormalDgp absent from ",
                    "the AlgebraTheory). The term registry was an allowlist, not a generative gate."
                ).into(),
                fix_description: concat!(
                    "Added openqg-algebra crate with structural generation gate: a DerivedCertificate ",
                    "relation must have a generating AlgebraTheory action term or is vetoed as ",
                    "StructurallyUngenerated. Regression-locked by theorem registry test."
                ).into(),
                fix_commit: "65560dc".into(),
                regression_test_name: "missing_ndgp_action_yields_structurally_ungenerated_veto".into(),
                regression_locked: true,
            },
        ],
    }
}

/// Baseline comparison entry for one AI-for-science system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineEntry {
    pub system_name: String,
    pub system_kind: BaselineKind,
    /// Paper / arXiv citation.
    pub citation: String,
    /// What they do that OpenQG does not.
    pub advantages_over_openqg: Vec<String>,
    /// What OpenQG does that they do not.
    pub openqg_advantages: Vec<String>,
    /// Specific head-to-head comparison (if run).
    pub head_to_head: Option<HeadToHeadResult>,
}

/// Kind of comparison baseline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BaselineKind {
    HumanMgScanPipeline,
    LlmProgramSearch,
    AutonomousLaboratory,
    DerivationCertificate,
    Other,
}

/// Result of a head-to-head comparison run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeadToHeadResult {
    /// Time to first invalidated champion (in campaigns).
    pub time_to_first_invalidated: Option<u32>,
    /// False-positive rate on decoy fixtures (proportion detected correctly).
    pub decoy_false_positive_rate: Option<f64>,
    /// Final evidence rank (rank 1 = best).
    pub final_evidence_rank: Option<u32>,
    /// Other metrics.
    pub notes: String,
}

/// Return the canonical baseline comparison table for the S11 paper section.
pub fn ai_for_science_baselines() -> Vec<BaselineEntry> {
    vec![
        BaselineEntry {
            system_name: "FunSearch".into(),
            system_kind: BaselineKind::LlmProgramSearch,
            citation: "Romera-Paredes et al. Nature 2024".into(),
            advantages_over_openqg: vec![
                "evaluator is closer to task ground truth (mathematical constructions)".into(),
                "demonstrated positive result (bin packing, cap set improvements)".into(),
            ],
            openqg_advantages: vec![
                "evaluator is a fallible scientific instrument — evaluator-bias discovery is the main result".into(),
                "deterministic host-owned oracle with versioned audit trail".into(),
                "ledgered regression tests prevent exploit recurrence".into(),
            ],
            head_to_head: None,
        },
        BaselineEntry {
            system_name: "AlphaEvolve".into(),
            system_kind: BaselineKind::LlmProgramSearch,
            citation: "Google DeepMind arXiv:2506.13131".into(),
            advantages_over_openqg: vec![
                "LLM-guided evolutionary search with verified algorithmic improvements".into(),
                "multiple scientific domains".into(),
            ],
            openqg_advantages: vec![
                "physical theory constraints and whitebox derivation certificates".into(),
                "anti-laundering gate prevents post-hoc rationalization of fit values".into(),
            ],
            head_to_head: None,
        },
        BaselineEntry {
            system_name: "AI-Hilbert".into(),
            system_kind: BaselineKind::DerivationCertificate,
            citation: "arXiv:2308.09474".into(),
            advantages_over_openqg: vec![
                "Positivstellensatz-style polynomial-law proof certificates".into(),
                "stronger value-level derivation guarantees for algebraic laws".into(),
            ],
            openqg_advantages: vec![
                "handles non-polynomial cosmological mechanisms and free-DOF pricing".into(),
                "adversarial audit loop finds evaluator biases, not just confirms known laws".into(),
            ],
            head_to_head: None,
        },
        BaselineEntry {
            system_name: "The AI Scientist".into(),
            system_kind: BaselineKind::LlmProgramSearch,
            citation: "arXiv:2408.06292 / v2 arXiv:2504.08066".into(),
            advantages_over_openqg: vec![
                "end-to-end idea-to-paper pipeline in ML domain".into(),
                "simulated peer review with LLM referees".into(),
            ],
            openqg_advantages: vec![
                "deterministic external oracle instead of LLM self-evaluation".into(),
                "the oracle's own bias discovery (V6 lA incident) is the claimed contribution".into(),
            ],
            head_to_head: None,
        },
        BaselineEntry {
            system_name: "Human ΛCDM/MG scan pipeline".into(),
            system_kind: BaselineKind::HumanMgScanPipeline,
            citation: "DESI/DES/KiDS Cobaya+CAMB/hi_class pipelines".into(),
            advantages_over_openqg: vec![
                "full Boltzmann likelihoods (CLASS, CAMB, hi_class, EFTofLSS)".into(),
                "mature covariance handling, blinding protocols, peer-reviewed analysis choices".into(),
                "full-shape/nonlinear/lensing/IA modeling".into(),
                "survey-specific selection functions and redshift distributions".into(),
            ],
            openqg_advantages: vec![
                "autonomous breadth over mechanism class space; not limited to pre-declared grids".into(),
                "adversarial regression corpus: exploit→regression-test ledger prevents reversion".into(),
                "explicit DOF pricing and anti-laundering gate built into scoring".into(),
                "evaluator-bias discovery is automatable; human pipelines rarely publish their own biases".into(),
            ],
            head_to_head: None,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_cascade_has_four_historical_exploits() {
        let report = v4_v7_audit_cascade();
        assert_eq!(
            report.entries.len(),
            4,
            "must have 4 historical exploit entries"
        );
        assert!(
            report.entries.iter().all(|e| e.regression_locked),
            "all historical exploits must be regression-locked"
        );
    }

    #[test]
    fn audit_cascade_has_evaluator_bias_entry() {
        let report = v4_v7_audit_cascade();
        let bias = report
            .by_class(ExploitClass::EvaluatorBias)
            .into_iter()
            .next()
            .expect("must have an EvaluatorBias entry (V6 lA incident)");
        assert!(bias.bounty_ln_z.unwrap() > 30.0);
        assert!(bias.exploit_description.contains("8.4σ"));
    }

    #[test]
    fn time_to_invalidate_metrics_sorted() {
        let report = v4_v7_audit_cascade();
        let metrics = report.time_to_invalidate_metrics();
        let times: Vec<u32> = metrics.iter().map(|m| m.campaigns_survived).collect();
        assert!(
            times.windows(2).all(|w| w[0] <= w[1]),
            "must be sorted ascending"
        );
    }

    #[test]
    fn total_bounty_includes_evaluator_bias_ln_z() {
        let report = v4_v7_audit_cascade();
        assert!(
            report.total_bounty_ln_z() >= 35.0,
            "total spurious lnZ must include the ~35 from the lA evaluator bias"
        );
        assert!(
            report.total_bounty_points() >= 87.5,
            "total bounty points must include V4's 87.5"
        );
    }

    #[test]
    fn regression_locked_fraction_is_100_percent() {
        let report = v4_v7_audit_cascade();
        assert_eq!(report.regression_locked_fraction(), 1.0);
    }

    #[test]
    fn baseline_table_covers_key_systems() {
        let baselines = ai_for_science_baselines();
        let names: Vec<&str> = baselines.iter().map(|b| b.system_name.as_str()).collect();
        assert!(names.iter().any(|n| n.contains("FunSearch")));
        assert!(names.iter().any(|n| n.contains("AI-Hilbert")));
        assert!(names.iter().any(|n| n.contains("Human ΛCDM")));
    }

    #[test]
    fn openqg_main_advantage_is_evaluator_bias_discovery() {
        let baselines = ai_for_science_baselines();
        let funsearch = baselines
            .iter()
            .find(|b| b.system_name == "FunSearch")
            .unwrap();
        assert!(
            funsearch
                .openqg_advantages
                .iter()
                .any(|a| a.contains("evaluator-bias")),
            "evaluator-bias discovery must be listed as OpenQG advantage over FunSearch"
        );
    }
}
