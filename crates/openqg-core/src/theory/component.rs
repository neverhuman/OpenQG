//! V8 Phase 3 (#14): Component Graph and Component Ledger.
//!
//! Gap-directed decomposition: every scored theory gets a machine-authored `ComponentLedger`
//! that records which relation/parameter/term/claim moved which observable by how much, so
//! focused re-proposal can target the weakest component rather than proposing whole theories.
//!
//! The ledger is content-bound and path-stable: component IDs are deterministic SHA-256 hashes
//! of their canonical key strings, and the ledger digest covers all component IDs so inserting
//! a fake component changes the digest.
//!
//! Spec reference: S04 §1-3.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Which part of a theory a component represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentKind {
    /// A certificate registry relation (e.g. `ndgp_geff_over_g`).
    Relation,
    /// A named physical parameter (e.g. `geff_over_g`).
    Parameter,
    /// A bound background field (e.g. `bg.mu0`).
    BackgroundField,
    /// An action-level term entry (e.g. `planck_mu_parametrization`).
    Term,
    /// A theory-level claim (evidence assertion).
    Claim,
    /// A forward-model oracle approximation that may be sensitivity-limited.
    Oracle,
    /// A nuisance/calibration degree of freedom (not a mechanism DOF).
    NuisanceCalibration,
    /// A certificate input value feeding a relation.
    CertificateInput,
    /// A scored observable contributing to data fit.
    Observable,
}

/// Stable, content-bound identity for one component.
///
/// `stable_key` is a human-readable, canonical string (see the module-level examples).
/// `content_digest` is SHA-256 of the canonical payload so its contents are bound.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ComponentId {
    pub kind: ComponentKind,
    /// Canonical key such as `relation:planck_mu0_geff:param:geff_over_g`.
    pub stable_key: String,
    /// SHA-256 over the canonical component payload (inputs, outputs, owner_path).
    pub content_digest: String,
}

impl ComponentId {
    pub fn new(kind: ComponentKind, stable_key: impl Into<String>, payload: &str) -> Self {
        let sk = stable_key.into();
        let hash = Sha256::digest(format!("{sk}|{payload}").as_bytes());
        ComponentId {
            kind,
            stable_key: sk,
            content_digest: format!("sha256:{hash:x}"),
        }
    }
}

impl std::fmt::Display for ComponentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.kind as u8, self.stable_key)
    }
}

/// How a component can be neutralized for ablation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum NeutralizerSpec {
    /// Remove the physical influence (set relation output to GR value) but keep cost, claims,
    /// and bookkeeping. Answers: "does the mechanism move evidence?"
    MechanismOffKeepCost {
        /// Name of the relation whose output is zeroed/GR-restored.
        relation: String,
        /// The GR-limit value for the output (e.g. 1.0 for G_eff/G).
        gr_value: f64,
    },
    /// Delete the component and recompute all downstream credit (rigor, novelty, parsimony).
    /// Answers: "was the component earning credit or adding complexity cost?"
    RemoveComponentReprice {
        /// Identifiers of the components to delete in this pass.
        component_keys: Vec<String>,
    },
    /// No neutralizer available for this component type.
    Unsupported { reason: String },
}

impl NeutralizerSpec {
    pub fn is_supported(&self) -> bool {
        !matches!(self, NeutralizerSpec::Unsupported { .. })
    }
}

/// One component of a theory's causal accounting graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TheoryComponent {
    pub id: ComponentId,
    /// JSON-pointer path into the Theory / ClaimGraph / obligations where this lives.
    pub owner_path: String,
    /// Cosmology sector (growth, lensing, background, cmb, etc.) when applicable.
    pub sector: Option<String>,
    /// Input identifiers: relation input keys, upstream component keys, or bound field names.
    pub inputs: Vec<String>,
    /// Output identifiers: bound field names, observable IDs, score dimension names.
    pub outputs: Vec<String>,
    /// Components this one depends on (upstream in the causal graph).
    pub upstream: Vec<ComponentId>,
    /// Components that depend on this one (downstream).
    pub downstream: Vec<ComponentId>,
    /// Claim IDs this component participates in.
    pub claim_ids: Vec<String>,
    /// Obligation IDs this component participates in.
    pub obligation_ids: Vec<String>,
    /// Free DOF cost charged against parsimony for this component.
    pub costed_dof: f64,
    /// How to neutralize this component during ablation.
    pub neutralizer: NeutralizerSpec,
}

/// Per-observable accounting for one evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservableTrace {
    pub observable_id: String,
    pub kind: String,
    pub observed: f64,
    pub uncertainty: f64,
    pub predicted: Option<f64>,
    pub baseline_predicted: Option<f64>,
    /// `predicted - observed`
    pub residual: Option<f64>,
    /// Whitened pull: `(predicted - observed) / sigma`, or covariance-weighted.
    pub pull_sigma: Option<f64>,
    pub loglike_contribution: Option<f64>,
    pub covariance_block_id: Option<String>,
    /// Which components contributed to this observable's prediction.
    pub upstream_components: Vec<ComponentId>,
}

/// Per-likelihood-block accounting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LikelihoodBlockTrace {
    pub block_id: String,
    pub observable_ids: Vec<String>,
    pub delta_loglike: f64,
    pub n_observables: u32,
    pub chi2_per_dof: Option<f64>,
    pub covariance_used: bool,
}

/// Full per-observable accounting for one theory evaluation.
///
/// Invariants (spec S04 §2):
/// - sum of `LikelihoodBlockTrace::delta_loglike` + Occam penalty == `DataFitOutcome::delta_lnz`
/// - sum of `dimension_points` == `ScorecardV4::total`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationTrace {
    pub theory_id: String,
    pub theory_digest: String,
    pub forward_manifest: String,
    pub observables: Vec<ObservableTrace>,
    pub likelihood_blocks: Vec<LikelihoodBlockTrace>,
    /// Per-scorecard-dimension contribution in points.
    pub dimension_points: BTreeMap<String, f64>,
    pub free_dof: u32,
}

impl EvaluationTrace {
    /// Sum of all likelihood block contributions.
    pub fn total_delta_loglike(&self) -> f64 {
        self.likelihood_blocks.iter().map(|b| b.delta_loglike).sum()
    }

    /// Sum of all dimension-point contributions.
    pub fn total_score_points(&self) -> f64 {
        self.dimension_points.values().sum()
    }

    /// Mean absolute pull across scored observables.
    pub fn mean_abs_pull(&self) -> Option<f64> {
        let pulls: Vec<f64> = self
            .observables
            .iter()
            .filter_map(|o| o.pull_sigma.map(|p| p.abs()))
            .collect();
        if pulls.is_empty() {
            None
        } else {
            Some(pulls.iter().sum::<f64>() / pulls.len() as f64)
        }
    }
}

/// Attribution score for one component after ablation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentAttribution {
    pub component_id: ComponentId,
    pub neutralizer_mode: String,
    /// `full_score - ablated_score`; positive = component helps total.
    pub score_delta: f64,
    /// Per-dimension score deltas.
    pub dimension_deltas: BTreeMap<String, f64>,
    /// Per-observable pull deltas: `|pull_ablated| - |pull_full|`; positive = component reduced pull.
    pub pull_help: BTreeMap<String, f64>,
    /// Per-observable prediction effect: `pred_full - pred_ablated`.
    pub prediction_effect: BTreeMap<String, f64>,
}

/// Pairwise interaction effect between two components.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairwiseInteraction {
    pub component_a: ComponentId,
    pub component_b: ComponentId,
    /// `score_delta({a,b}) - score_delta(a) - score_delta(b)`.
    pub interaction_effect: f64,
    /// True when |interaction_effect| exceeds the threshold (>0.5 pts or >20% of max single delta).
    pub interaction_risk: bool,
}

/// Shapley estimate for a group of components.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapleyEstimate {
    pub group_id: String,
    pub component_ids: Vec<ComponentId>,
    pub shapley_values: Vec<f64>,
    pub n_permutation_samples: u32,
}

/// Class of a detected gap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GapClass {
    Evidence,
    Rigor,
    Novelty,
    Oracle,
    Interaction,
    AntiGaming,
}

/// A ranked gap: one weak component targeted for focused re-proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapRecord {
    pub gap_id: String,
    pub component_id: ComponentId,
    pub gap_class: GapClass,
    /// Higher = more urgent. Combines evidence shortfall, rigor gap, and interaction risk.
    pub priority: f64,
    pub evidence_shortfall: f64,
    pub rigor_shortfall: f64,
    pub novelty_shortfall: f64,
    pub interaction_risk: f64,
    /// Short, host-authored description of why this component is weak.
    pub description: String,
    /// Suggested focus prompt hint for the proposer (value-safe).
    pub focus_hint: String,
}

/// An oracle approximation sensitivity entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleSensitivity {
    pub oracle_id: String,
    pub description: String,
    /// Maximum score change under the oracle's uncertainty.
    pub score_leverage: f64,
    /// True when the sign of evidence flips under the oracle's uncertainty.
    pub evidence_sign_flip: bool,
    /// Whether this oracle sensitivity should block promotion.
    pub blocks_promotion: bool,
}

/// A reference to an external receipt artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptRef {
    pub receipt_kind: String,
    pub receipt_id: String,
    pub content_digest: String,
}

/// The full component ledger for one champion theory evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentLedger {
    pub schema_version: u32,
    pub run_id: String,
    pub champion_id: String,
    pub champion_digest: String,
    pub source_tree_digest: String,
    pub full_trace: EvaluationTrace,
    pub components: Vec<TheoryComponent>,
    pub ablations: Vec<ComponentAttribution>,
    pub pairwise_interactions: Vec<PairwiseInteraction>,
    pub shapley_estimates: Vec<ShapleyEstimate>,
    pub gaps: Vec<GapRecord>,
    pub oracle_sensitivities: Vec<OracleSensitivity>,
    pub receipts: Vec<ReceiptRef>,
}

impl ComponentLedger {
    /// SHA-256 digest of the canonical (sorted by component stable_key) ledger header.
    ///
    /// Covers: champion_id, champion_digest, component stable_keys, and component content_digests.
    /// Reordering components changes this digest.
    pub fn ledger_digest(&self) -> String {
        let mut keys: Vec<(&str, &str)> = self
            .components
            .iter()
            .map(|c| (c.id.stable_key.as_str(), c.id.content_digest.as_str()))
            .collect();
        keys.sort_unstable();
        let payload = format!(
            "champion_id={}\nchampion_digest={}\ncomponents=[{}]",
            self.champion_id,
            self.champion_digest,
            keys.iter()
                .map(|(k, d)| format!("{k}:{d}"))
                .collect::<Vec<_>>()
                .join(",")
        );
        let hash = Sha256::digest(payload.as_bytes());
        format!("sha256:{hash:x}")
    }

    /// Find a component by its stable_key.
    pub fn component(&self, stable_key: &str) -> Option<&TheoryComponent> {
        self.components
            .iter()
            .find(|c| c.id.stable_key == stable_key)
    }

    /// Return all gaps in priority order (highest first).
    pub fn ranked_gaps(&self) -> Vec<&GapRecord> {
        let mut gaps: Vec<&GapRecord> = self.gaps.iter().collect();
        gaps.sort_by(|a, b| {
            b.priority
                .partial_cmp(&a.priority)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        gaps
    }
}

/// Build a minimal `ComponentLedger` from a theory's parameters and claims.
///
/// This is the lightweight builder used in normal scoring; the full ablation engine
/// with parallel neutralization is in `crates/openqg-bench/src/ablation.rs`.
pub fn build_component_graph(
    run_id: &str,
    theory_id: &str,
    theory_digest: &str,
    parameters: &[(String, String, String)], // (symbol, relation, sector)
    claims: &[String],
    free_dof: u32,
    dimension_points: BTreeMap<String, f64>,
) -> ComponentLedger {
    let mut components = Vec::new();

    // Build relation + parameter component pairs from certified parameters.
    for (symbol, relation, sector) in parameters {
        if !relation.is_empty() {
            let rel_key = format!("relation:{relation}:param:{symbol}");
            let rel_id = ComponentId::new(
                ComponentKind::Relation,
                &rel_key,
                &format!("relation={relation},param={symbol}"),
            );
            let param_key = format!("param:{symbol}");
            let param_id = ComponentId::new(
                ComponentKind::Parameter,
                &param_key,
                &format!("symbol={symbol},relation={relation}"),
            );
            let bg_key = format!("bg:{symbol}");
            let bg_id = ComponentId::new(
                ComponentKind::BackgroundField,
                &bg_key,
                &format!("field={symbol}"),
            );

            let gr_value = gr_limit_for_relation(relation);
            let neutralizer = NeutralizerSpec::MechanismOffKeepCost {
                relation: relation.clone(),
                gr_value,
            };

            components.push(TheoryComponent {
                id: rel_id.clone(),
                owner_path: format!("/parameters/{symbol}/provenance/certificate/relation"),
                sector: Some(sector.clone()),
                inputs: vec![format!("cert:inputs:{symbol}")],
                outputs: vec![param_key.clone(), bg_key.clone()],
                upstream: vec![],
                downstream: vec![param_id.clone(), bg_id.clone()],
                claim_ids: vec![],
                obligation_ids: vec![],
                costed_dof: 0.0,
                neutralizer,
            });

            components.push(TheoryComponent {
                id: param_id.clone(),
                owner_path: format!("/parameters/{symbol}"),
                sector: Some(sector.clone()),
                inputs: vec![rel_key.clone()],
                outputs: vec![bg_key.clone()],
                upstream: vec![rel_id],
                downstream: vec![bg_id],
                claim_ids: vec![],
                obligation_ids: vec![],
                costed_dof: 0.0,
                neutralizer: NeutralizerSpec::Unsupported {
                    reason: "parameter component is neutralized via its relation".into(),
                },
            });
        }
    }

    // Build claim components.
    for claim_id in claims {
        let claim_key = format!("claim:{claim_id}");
        let claim_comp_id = ComponentId::new(ComponentKind::Claim, &claim_key, claim_id);
        components.push(TheoryComponent {
            id: claim_comp_id,
            owner_path: format!("/claims/{claim_id}"),
            sector: None,
            inputs: vec![],
            outputs: vec!["rigor".into(), "novelty".into()],
            upstream: vec![],
            downstream: vec![],
            claim_ids: vec![claim_id.clone()],
            obligation_ids: vec![],
            costed_dof: 0.0,
            neutralizer: NeutralizerSpec::RemoveComponentReprice {
                component_keys: vec![claim_key],
            },
        });
    }

    let eval_trace = EvaluationTrace {
        theory_id: theory_id.into(),
        theory_digest: theory_digest.into(),
        forward_manifest: String::new(),
        observables: vec![],
        likelihood_blocks: vec![],
        dimension_points,
        free_dof,
    };

    ComponentLedger {
        schema_version: 1,
        run_id: run_id.into(),
        champion_id: theory_id.into(),
        champion_digest: theory_digest.into(),
        source_tree_digest: String::new(),
        full_trace: eval_trace,
        components,
        ablations: vec![],
        pairwise_interactions: vec![],
        shapley_estimates: vec![],
        gaps: vec![],
        oracle_sensitivities: vec![],
        receipts: vec![],
    }
}

/// Discounted-UCB priority for a gap in the bandit router (SYNTHESIS #14).
///
/// Extends `GapRecord` with a time-discount factor (effort decays on unaddressed gaps to
/// encourage diverse exploration) and a UCB exploration bonus (arms with fewer attempts
/// receive a higher bonus). The bandit router picks the gap with the highest `total_priority`.
///
/// Formula: `total_priority = base_priority × discount_factor + ucb_bonus`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapPriority {
    /// Stable key of the gap (matches `GapRecord.gap_id`).
    pub gap_id: String,
    /// The component this gap targets.
    pub component_id: ComponentId,
    /// Semantic class of the gap.
    pub gap_class: GapClass,
    /// Raw priority from the component ledger.
    pub base_priority: f64,
    /// Exponential discount applied per attempt: `exp(-decay_rate × attempts)`.
    pub discount_factor: f64,
    /// UCB exploration bonus: `exploration_c × sqrt(ln(total_campaign_calls + 1) / (attempts + 1))`.
    pub ucb_bonus: f64,
    /// Final routable priority: `base_priority × discount_factor + ucb_bonus`.
    pub total_priority: f64,
    /// Number of focused re-proposal attempts targeting this gap.
    pub attempts: u32,
    /// Number of attempts that improved the score.
    pub successes: u32,
}

impl GapPriority {
    /// True when at least one attempt has succeeded for this gap.
    pub fn has_any_success(&self) -> bool {
        self.successes > 0
    }

    /// Empirical success rate (0.0 when no attempts).
    pub fn success_rate(&self) -> f64 {
        if self.attempts == 0 {
            0.0
        } else {
            self.successes as f64 / self.attempts as f64
        }
    }
}

/// Kill reason for a `FocusedPatchSketch` that violates anti-laundering rules (SYNTHESIS #14).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum FocusedPatchKillReason {
    /// The target component has a frozen digest; patching it directly is prohibited.
    /// Changes must go through the derivation sandbox.
    FrozenDigestMutation {
        component_id: String,
        frozen_digest: String,
        patch_digest: String,
    },
    /// The patch shifts a DOF to a sibling component without re-pricing — a disguised
    /// relabeling that launders complexity.
    DofLaunderingShift {
        original_symbol: String,
        shifted_to_component: String,
        detail: String,
    },
}

/// A patch proposal targeting one weak component (SYNTHESIS #14).
///
/// `FocusedPatchSketch` is the structured interface between the gap-routing bandit and the
/// proposer: the router identifies the gap, the proposer fills in the patch details, and
/// `validate()` applies the anti-laundering gate before the patch enters the scoring loop.
///
/// A sketch that fails `validate()` is killed before physics scoring; its `kill_reason`
/// maps directly to a `VetoReason::DofLaunderingKill` or `FrozenDigestMutationKill` in the
/// downstream veto cascade.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusedPatchSketch {
    /// Unique ID for this patch sketch.
    pub sketch_id: String,
    /// The component being patched.
    pub target_component: ComponentId,
    /// The gap this patch addresses.
    pub target_gap_id: String,
    /// Human-readable description of the proposed change.
    pub proposed_change: String,
    /// If the target component has a frozen digest (set by the derivation sandbox), patching
    /// it directly is prohibited. Set `Some(digest)` to enable the mutation kill.
    pub frozen_digest: Option<String>,
    /// Content digest of the component in the CURRENT ledger.
    pub current_digest: String,
    /// Content digest of the component AFTER the proposed change.
    pub patch_digest: String,
    /// True when the patch shifts a DOF from one component to a sibling without repricing.
    pub is_dof_shift_to_sibling: bool,
    /// If DOF shift: the original symbol that is being relabeled.
    pub original_dof_symbol: Option<String>,
    /// Anti-laundering verdict. `None` = clean (sketch may proceed); `Some(_)` = killed.
    pub kill_reason: Option<FocusedPatchKillReason>,
}

impl FocusedPatchSketch {
    /// True when the sketch has passed the anti-laundering gate.
    pub fn is_safe(&self) -> bool {
        self.kill_reason.is_none()
    }

    /// Run the anti-laundering gate and set `kill_reason` if any violation is found.
    ///
    /// Call this before entering the sketch into the scoring loop. Once killed, the sketch
    /// is immutable — re-calling `validate()` on an already-killed sketch is a no-op.
    pub fn validate(&mut self) {
        if self.kill_reason.is_some() {
            return; // already killed
        }
        // Gate 1: frozen-digest mutation — target must not have a frozen digest.
        if let Some(ref frozen) = self.frozen_digest {
            if *frozen != self.patch_digest {
                self.kill_reason = Some(FocusedPatchKillReason::FrozenDigestMutation {
                    component_id: self.target_component.stable_key.clone(),
                    frozen_digest: frozen.clone(),
                    patch_digest: self.patch_digest.clone(),
                });
                return;
            }
        }
        // Gate 2: DOF-laundering shift — DOFs must not be silently moved between components.
        if self.is_dof_shift_to_sibling {
            let sym = self
                .original_dof_symbol
                .clone()
                .unwrap_or_else(|| "unknown".into());
            self.kill_reason = Some(FocusedPatchKillReason::DofLaunderingShift {
                original_symbol: sym,
                shifted_to_component: self.target_component.stable_key.clone(),
                detail: "DOF relabeled to sibling during focused patch — re-price required".into(),
            });
        }
    }
}

/// Compute discounted-UCB gap priorities from a component ledger (SYNTHESIS #14).
///
/// # Parameters
/// - `ledger`: the champion's component ledger containing `gaps`.
/// - `attempts_per_gap`: map from `gap_id` → `(attempts, successes)` from campaign history.
/// - `total_campaign_calls`: total number of focused-patch attempts across ALL gaps this campaign.
/// - `exploration_c`: UCB exploration constant (S04 recommends 0.3–1.0; default 0.5).
/// - `decay_rate`: exponential discount per attempt (default 0.1 → 10-step gap halves to ~0.37).
///
/// Returns gaps sorted by `total_priority` descending.
pub fn compute_gap_priorities(
    ledger: &ComponentLedger,
    attempts_per_gap: &BTreeMap<String, (u32, u32)>,
    total_campaign_calls: u64,
    exploration_c: f64,
    decay_rate: f64,
) -> Vec<GapPriority> {
    let mut priorities: Vec<GapPriority> = ledger
        .gaps
        .iter()
        .map(|gap| {
            let (attempts, successes) =
                attempts_per_gap.get(&gap.gap_id).copied().unwrap_or((0, 0));
            let discount_factor = (-decay_rate * attempts as f64).exp();
            let ucb_bonus = exploration_c
                * ((total_campaign_calls as f64 + 1.0).ln() / (attempts as f64 + 1.0)).sqrt();
            let total_priority = gap.priority * discount_factor + ucb_bonus;
            GapPriority {
                gap_id: gap.gap_id.clone(),
                component_id: gap.component_id.clone(),
                gap_class: gap.gap_class,
                base_priority: gap.priority,
                discount_factor,
                ucb_bonus,
                total_priority,
                attempts,
                successes,
            }
        })
        .collect();
    priorities.sort_by(|a, b| {
        b.total_priority
            .partial_cmp(&a.total_priority)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    priorities
}

/// Return the GR-limit value for the output of a relation (for MechanismOffKeepCost).
fn gr_limit_for_relation(relation: &str) -> f64 {
    match relation {
        "ndgp_geff_over_g"
        | "coupled_de_geff_over_g"
        | "fr_largescale_geff_over_g"
        | "planck_mu0_geff" => 1.0, // G_eff/G = 1 in GR
        "dark_scattering_growth_drag" => 0.0, // drag = 0 in GR
        "fr_alpha_m" => 0.0,                  // alpha_M = 0 in GR (f_R -> 0)
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn planck_mu0_ledger() -> ComponentLedger {
        let mut points = BTreeMap::new();
        points.insert("data_fit".into(), 12.0);
        points.insert("derivation_rigor".into(), 8.0);
        points.insert("novelty".into(), 6.0);
        build_component_graph(
            "v8-test-run",
            "planck_mu0-suppressed-growth",
            "sha256:aaabbb",
            &[
                (
                    "geff_over_g".into(),
                    "planck_mu0_geff".into(),
                    "growth".into(),
                ),
                ("mu0".into(), "planck_mu0_geff".into(), "growth".into()),
            ],
            &["claim-suppressed-growth".into()],
            2,
            points,
        )
    }

    /// Spec acceptance test (S04 rank 1): stable component IDs for planck_mu0 fixture.
    #[test]
    fn component_ids_for_planck_mu0_fixture() {
        let ledger = planck_mu0_ledger();

        // Can locate by stable_key patterns.
        assert!(
            ledger
                .component("relation:planck_mu0_geff:param:geff_over_g")
                .is_some(),
            "relation:planck_mu0_geff:param:geff_over_g must be findable"
        );
        assert!(
            ledger.component("param:geff_over_g").is_some(),
            "param:geff_over_g must be findable"
        );
        assert!(
            ledger.component("claim:claim-suppressed-growth").is_some(),
            "claim:claim-suppressed-growth must be findable"
        );
    }

    /// Spec acceptance test (S04 rank 1): ledger digest is deterministic and stable.
    #[test]
    fn ledger_digest_is_deterministic() {
        let l1 = planck_mu0_ledger();
        let l2 = planck_mu0_ledger();
        assert_eq!(l1.ledger_digest(), l2.ledger_digest());
        assert!(!l1.ledger_digest().is_empty());
        assert!(l1.ledger_digest().starts_with("sha256:"));
    }

    /// Different champions produce different digests.
    #[test]
    fn different_theories_different_digests() {
        let l1 = planck_mu0_ledger();
        let mut points = BTreeMap::new();
        points.insert("data_fit".into(), 5.0);
        let l2 = build_component_graph(
            "v8-test-run",
            "other-theory",
            "sha256:cccddd",
            &[(
                "beta".into(),
                "ndgp_beta_from_omega_rc".into(),
                "background".into(),
            )],
            &[],
            1,
            points,
        );
        assert_ne!(l1.ledger_digest(), l2.ledger_digest());
    }

    #[test]
    fn mechanism_off_neutralizer_has_gr_value() {
        let ledger = planck_mu0_ledger();
        let rel = ledger
            .component("relation:planck_mu0_geff:param:geff_over_g")
            .unwrap();
        assert!(rel.neutralizer.is_supported());
        if let NeutralizerSpec::MechanismOffKeepCost { gr_value, .. } = &rel.neutralizer {
            assert!(
                (*gr_value - 1.0).abs() < 1e-9,
                "planck_mu0_geff GR value must be 1.0"
            );
        } else {
            panic!("expected MechanismOffKeepCost neutralizer");
        }
    }

    #[test]
    fn evaluation_trace_totals_work() {
        let ledger = planck_mu0_ledger();
        assert_eq!(ledger.full_trace.total_score_points(), 26.0);
        assert!(ledger.full_trace.mean_abs_pull().is_none()); // no observables in minimal build
    }

    // ---- GapPriority + compute_gap_priorities (SYNTHESIS #14) ----

    fn ledger_with_gaps() -> ComponentLedger {
        let mut l = planck_mu0_ledger();
        let cid = ComponentId::new(ComponentKind::Relation, "rel:planck_mu0_geff", "payload");
        l.gaps.push(GapRecord {
            gap_id: "gap-rigor-mu0".into(),
            component_id: cid.clone(),
            gap_class: GapClass::Rigor,
            priority: 0.8,
            evidence_shortfall: 0.0,
            rigor_shortfall: 0.3,
            novelty_shortfall: 0.0,
            interaction_risk: 0.0,
            description: "mu0 cert rigor low".into(),
            focus_hint: "upgrade cert to T2".into(),
        });
        l.gaps.push(GapRecord {
            gap_id: "gap-evidence-fs8".into(),
            component_id: cid,
            gap_class: GapClass::Evidence,
            priority: 0.5,
            evidence_shortfall: 0.4,
            rigor_shortfall: 0.0,
            novelty_shortfall: 0.0,
            interaction_risk: 0.0,
            description: "fσ8 evidence thin".into(),
            focus_hint: "add eBOSS ref".into(),
        });
        l
    }

    #[test]
    fn compute_gap_priorities_sorts_by_total_priority() {
        let ledger = ledger_with_gaps();
        let attempts = BTreeMap::new();
        let prios = compute_gap_priorities(&ledger, &attempts, 10, 0.5, 0.1);
        assert_eq!(prios.len(), 2);
        // Without prior attempts, higher base_priority + same UCB bonus → top gap first
        assert!(prios[0].total_priority >= prios[1].total_priority);
    }

    #[test]
    fn ucb_bonus_increases_for_unattempted_gaps() {
        let ledger = ledger_with_gaps();
        let mut attempts = BTreeMap::new();
        // Mark the top gap as heavily attempted
        attempts.insert("gap-rigor-mu0".to_string(), (20u32, 5u32));
        let prios = compute_gap_priorities(&ledger, &attempts, 100, 1.0, 0.0);
        // With decay_rate=0, discount=1.0; UCB bonus drives unattempted gap up.
        // gap-evidence-fs8 has 0 attempts → higher UCB than gap-rigor-mu0 (20 attempts)
        let rigor_prio = prios.iter().find(|p| p.gap_id == "gap-rigor-mu0").unwrap();
        let ev_prio = prios
            .iter()
            .find(|p| p.gap_id == "gap-evidence-fs8")
            .unwrap();
        assert!(
            ev_prio.ucb_bonus > rigor_prio.ucb_bonus,
            "unattempted gap must have higher UCB; rigor={} ev={}",
            rigor_prio.ucb_bonus,
            ev_prio.ucb_bonus
        );
    }

    #[test]
    fn success_rate_correct() {
        let gp = GapPriority {
            gap_id: "g1".into(),
            component_id: ComponentId::new(ComponentKind::Relation, "r", "p"),
            gap_class: GapClass::Rigor,
            base_priority: 0.5,
            discount_factor: 1.0,
            ucb_bonus: 0.2,
            total_priority: 0.7,
            attempts: 4,
            successes: 1,
        };
        assert!((gp.success_rate() - 0.25).abs() < 1e-10);
        assert!(gp.has_any_success());
    }

    // ---- FocusedPatchSketch (SYNTHESIS #14) ----

    fn clean_sketch() -> FocusedPatchSketch {
        let cid = ComponentId::new(ComponentKind::Parameter, "param:mu0", "p");
        FocusedPatchSketch {
            sketch_id: "s1".into(),
            target_component: cid,
            target_gap_id: "gap-rigor-mu0".into(),
            proposed_change: "upgrade mu0 cert to T2Boltzmann".into(),
            frozen_digest: None,
            current_digest: "digest-a".into(),
            patch_digest: "digest-b".into(),
            is_dof_shift_to_sibling: false,
            original_dof_symbol: None,
            kill_reason: None,
        }
    }

    #[test]
    fn clean_sketch_is_safe_after_validate() {
        let mut s = clean_sketch();
        s.validate();
        assert!(s.is_safe());
        assert!(s.kill_reason.is_none());
    }

    #[test]
    fn frozen_digest_mutation_kills_sketch() {
        let mut s = clean_sketch();
        s.frozen_digest = Some("frozen-digest-X".into()); // patch_digest = "digest-b" ≠ frozen
        s.validate();
        assert!(!s.is_safe());
        assert!(matches!(
            s.kill_reason,
            Some(FocusedPatchKillReason::FrozenDigestMutation { .. })
        ));
    }

    #[test]
    fn frozen_digest_matching_patch_digest_is_safe() {
        let mut s = clean_sketch();
        s.frozen_digest = Some("digest-b".into()); // matches patch_digest
        s.validate();
        assert!(s.is_safe(), "matching digest must not kill");
    }

    #[test]
    fn dof_laundering_shift_kills_sketch() {
        let mut s = clean_sketch();
        s.is_dof_shift_to_sibling = true;
        s.original_dof_symbol = Some("mu0".into());
        s.validate();
        assert!(!s.is_safe());
        assert!(matches!(
            s.kill_reason,
            Some(FocusedPatchKillReason::DofLaunderingShift { .. })
        ));
    }

    #[test]
    fn validate_is_idempotent_on_killed_sketch() {
        let mut s = clean_sketch();
        s.is_dof_shift_to_sibling = true;
        s.original_dof_symbol = Some("mu0".into());
        s.validate();
        let first_kill = s.kill_reason.clone();
        s.validate(); // second call must not change anything
        assert_eq!(s.kill_reason, first_kill);
    }
}
