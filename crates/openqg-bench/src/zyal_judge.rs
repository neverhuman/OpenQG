//! Phases 2-4 of the adversarial-robustness rebuild: the judging layer, the co-evolving
//! adversary, hyper-focus allocation, MAP-Elites diversity, and the robustness quality gate.
//!
//! Design (matches the approved plan):
//! - **Critic abstraction.** A candidate is attacked by a panel. The default `Critic` is the
//!   *deterministic* one below (cheap-first, fully reproducible, no live stack). A live-LLM
//!   critic is a drop-in upgrade behind the same `Verdict` contract; it stays *subordinate*
//!   to the physics veto + structural anchors, which can kill a candidate regardless.
//! - **Co-evolving adversary.** The `AttackArchive` accumulates falsification pressure and an
//!   escalating "frontier margin" across generations, so a fixed candidate's survival is a
//!   *moving target* — this is what structurally prevents the 0.995-style saturation.
//! - **Honesty meta-loop.** Escalation only proceeds while the frozen `survive` anchors still
//!   clear a survival floor; if escalation would start killing good anchors, it rolls back.
//! - Scoring/aggregation is FROZEN within a run; only the adversary escalates. (Rubric/anchor
//!   adaptation happens between runs.)

use crate::zyal_robustness::{RobustnessOutcome, TheoryArtifact};
use openqg_core::ObservableRecord;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::Path;

const FRONTIER_STEEPNESS: f64 = 0.12;
const PARSIMONY_WEIGHT: f64 = 2.0;
const ESCALATION_STEP: f64 = 0.6;
const ESCALATION_CAP: f64 = 12.0;
/// A frozen `survive` anchor must keep at least this survival as the adversary escalates,
/// otherwise the escalation is unfair and is rolled back.
pub const ANCHOR_SURVIVAL_FLOOR: f64 = 0.20;

fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}
fn round6(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}

// ---------------------------------------------------------------------------------------
// Attacks & the co-evolving archive (Phase 3)
// ---------------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum AttackKind {
    /// Hard: non-physical parameters or far-worse-than-baseline likelihood.
    NonPhysical,
    /// Hard: does not predict every observable in the suite.
    IncompleteCoverage,
    /// Hard: no falsifiable prediction stated.
    Unfalsifiable,
    /// Hard: does not declare a known-limit recovery (GR/Newtonian/LCDM).
    NoDeclaredLimit,
    /// Hard: one free parameter per observable (memorization, not a theory).
    Overfit,
    /// Hard: structurally incoherent (no pillars / empty claim / no predictions).
    Incoherent,
    /// Hard: black/gray-box or free-parameter fitting — a parameter without physical meaning or
    /// provenance, or "tuned to fit" language. OpenQG only considers whitebox theories.
    GrayBox,
    /// Soft & escalating: must beat the baseline by at least `margin` in delta-log-likelihood.
    BelowFrontier,
}

impl AttackKind {
    fn is_hard(&self) -> bool {
        !matches!(self, AttackKind::BelowFrontier)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Attack {
    pub id: String,
    pub kind: AttackKind,
    pub strength: f64,
    pub margin: f64,
    pub kills: usize,
}

/// Does this attack "land" on the candidate? Pure function of the structured artifact and
/// its deterministic physics outcome — the deterministic critic panel.
fn attack_lands(attack: &Attack, outcome: &RobustnessOutcome, violations: &[String]) -> bool {
    let has = |v: &str| violations.iter().any(|x| x == v);
    match attack.kind {
        AttackKind::NonPhysical => outcome.veto_reasons.iter().any(|r| {
            r == "non_physical_parameters" || r == "far_worse_than_baseline"
        }),
        AttackKind::IncompleteCoverage => outcome.coverage < 1.0,
        AttackKind::Unfalsifiable => has("no_falsifiable_prediction"),
        AttackKind::NoDeclaredLimit => has("no_declared_limit"),
        AttackKind::Overfit => has("parameter_per_observable_overfit"),
        AttackKind::Incoherent => {
            has("no_pillars") || has("pillar_without_claim") || has("no_predictions")
        }
        AttackKind::GrayBox => violations.iter().any(|x| {
            x == "black_box_language"
                || x.starts_with("free_parameter")
                || x.starts_with("parameter_without_provenance")
                || x.starts_with("parameter_without_meaning")
        }),
        AttackKind::BelowFrontier => outcome.delta_log_likelihood < attack.margin,
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttackArchive {
    pub attacks: Vec<Attack>,
    pub generation: usize,
    pub frontier_margin: f64,
}

impl AttackArchive {
    /// The seed adversary: all structural/physics attacks plus a frontier bar starting at 0
    /// (a candidate must at least tie the baseline).
    pub fn seed() -> Self {
        let hard = [
            ("a_nonphysical", AttackKind::NonPhysical),
            ("a_coverage", AttackKind::IncompleteCoverage),
            ("a_unfalsifiable", AttackKind::Unfalsifiable),
            ("a_no_limit", AttackKind::NoDeclaredLimit),
            ("a_overfit", AttackKind::Overfit),
            ("a_incoherent", AttackKind::Incoherent),
            ("a_graybox", AttackKind::GrayBox),
        ];
        let mut attacks: Vec<Attack> = hard
            .iter()
            .map(|(id, kind)| Attack {
                id: (*id).to_string(),
                kind: kind.clone(),
                strength: 1.0,
                margin: 0.0,
                kills: 0,
            })
            .collect();
        attacks.push(Attack {
            id: "a_frontier".to_string(),
            kind: AttackKind::BelowFrontier,
            strength: 1.0,
            margin: 0.0,
            kills: 0,
        });
        AttackArchive {
            attacks,
            generation: 0,
            frontier_margin: 0.0,
        }
    }

    /// Escalate the adversary for the next generation. The frontier bar rises while the
    /// `survive` anchors still clear the survival floor; otherwise it rolls back (honesty).
    pub fn escalate(&mut self, kills: &BTreeMap<String, usize>, anchors_ok: bool) {
        if anchors_ok {
            self.frontier_margin = (self.frontier_margin + ESCALATION_STEP).min(ESCALATION_CAP);
        } else {
            self.frontier_margin = (self.frontier_margin - ESCALATION_STEP).max(0.0);
        }
        for a in &mut self.attacks {
            if a.kind == AttackKind::BelowFrontier {
                a.margin = self.frontier_margin;
            }
            if let Some(k) = kills.get(&a.id) {
                a.kills += *k;
                a.strength = 1.0 + (a.kills as f64).ln_1p();
            }
        }
        self.generation += 1;
    }

    pub fn to_json(&self) -> Value {
        json!({
            "record_kind": "attack_archive",
            "generation": self.generation,
            "frontier_margin": round6(self.frontier_margin),
            "attacks": self.attacks.iter().map(|a| json!({
                "id": a.id,
                "kind": format!("{:?}", a.kind),
                "strength": round6(a.strength),
                "margin": round6(a.margin),
                "kills": a.kills,
            })).collect::<Vec<_>>(),
        })
    }
}

// ---------------------------------------------------------------------------------------
// Verdicts & pairwise Elo (Phase 2)
// ---------------------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Verdict {
    pub candidate_id: String,
    pub survived: bool,
    /// Moving-target survival in `[0,1)`: 0 when hard-killed/vetoed, else a plateau-free
    /// logistic of `delta_log_likelihood - parsimony - frontier_margin`.
    pub survival: f64,
    pub elo: f64,
    pub landed: Vec<String>,
}

impl Verdict {
    pub fn judge_block(&self) -> Value {
        json!({
            "record_kind": "judge_verdict",
            "candidate_id": self.candidate_id,
            "survived": self.survived,
            "survival": round6(self.survival),
            "elo": round6(self.elo),
            "landed_attacks": self.landed,
        })
    }
}

/// Judge one candidate against the current adversary. The hard attacks (physics + structure)
/// can kill outright; the escalating frontier bar modulates survival without a ceiling.
pub fn judge(
    candidate_id: &str,
    artifact: &TheoryArtifact,
    outcome: &RobustnessOutcome,
    archive: &AttackArchive,
) -> Verdict {
    let mut violations = artifact.structural_violations();
    violations.extend(artifact.whitebox_violations());
    let mut landed = Vec::new();
    let mut hard_killed = outcome.vetoed;
    for a in &archive.attacks {
        if attack_lands(a, outcome, &violations) {
            landed.push(a.id.clone());
            if a.kind.is_hard() {
                hard_killed = true;
            }
        }
    }
    let penalized = outcome.delta_log_likelihood
        - PARSIMONY_WEIGHT * outcome.metrics.parameter_count_penalty
        - archive.frontier_margin;
    let survival = if hard_killed {
        0.0
    } else {
        sigmoid(FRONTIER_STEEPNESS * penalized).clamp(0.0, 0.999)
    };
    Verdict {
        candidate_id: candidate_id.to_string(),
        survived: !hard_killed,
        survival,
        elo: 1500.0,
        landed,
    }
}

/// Pairwise, unbounded Elo over a population's verdicts (anti-saturation by construction;
/// pairwise comparison is less gameable than absolute scores). Deterministic round-robin.
pub fn assign_pairwise_elo(verdicts: &mut [Verdict]) {
    let n = verdicts.len();
    for i in 0..n {
        for j in 0..n {
            if i == j {
                continue;
            }
            let (si, sj) = (verdicts[i].survival, verdicts[j].survival);
            let expected = 1.0 / (1.0 + 10f64.powf((verdicts[j].elo - verdicts[i].elo) / 400.0));
            let actual = if si > sj {
                1.0
            } else if si < sj {
                0.0
            } else {
                0.5
            };
            verdicts[i].elo += 16.0 * (actual - expected);
        }
    }
}

// ---------------------------------------------------------------------------------------
// Hyper-focus allocation (Phase 3): pour the call budget at the most-attacked pillar
// ---------------------------------------------------------------------------------------

/// Softmax allocation of a fixed generative-call budget across pillars/stages by their
/// attack-success rate, with a per-pillar floor so nothing is fully starved.
pub fn focus_allocation(
    attack_success: &BTreeMap<String, f64>,
    budget: usize,
    floor: usize,
    temperature: f64,
) -> BTreeMap<String, usize> {
    let mut out = BTreeMap::new();
    if attack_success.is_empty() {
        return out;
    }
    let tau = temperature.max(1e-6);
    let max = attack_success.values().cloned().fold(f64::MIN, f64::max);
    let weights: BTreeMap<&String, f64> = attack_success
        .iter()
        .map(|(k, v)| (k, ((v - max) / tau).exp()))
        .collect();
    let total: f64 = weights.values().sum();
    let floored: usize = floor * attack_success.len();
    let discretionary = budget.saturating_sub(floored);
    let mut assigned = 0usize;
    let mut best_key = attack_success.keys().next().unwrap().clone();
    let mut best_w = f64::MIN;
    for (k, w) in &weights {
        let share = ((discretionary as f64) * (w / total)).floor() as usize;
        out.insert((*k).clone(), floor + share);
        assigned += floor + share;
        if *w > best_w {
            best_w = *w;
            best_key = (*k).clone();
        }
    }
    // Hand any rounding remainder to the most-attacked pillar.
    if budget > assigned {
        *out.entry(best_key).or_insert(0) += budget - assigned;
    }
    out
}

// ---------------------------------------------------------------------------------------
// MAP-Elites diversity archive (Phase 4): content descriptors, QD-score
// ---------------------------------------------------------------------------------------

/// Behavioral descriptor computed purely from the (numeric) physics outcome — NOT from any
/// stage/island name. (param-count bin, mechanism family, fit-quality bin).
pub fn descriptor(outcome: &RobustnessOutcome) -> (usize, usize, usize) {
    let param_bin = outcome.parameter_count.saturating_sub(3).min(4);
    let h0_active = outcome
        .genes
        .get("delta_h0_local")
        .and_then(Value::as_f64)
        .map(|v| v.abs() > 0.1)
        .unwrap_or(false);
    let s8_active = outcome
        .genes
        .get("s8_suppression")
        .and_then(Value::as_f64)
        .map(|v| v.abs() > 0.005)
        .unwrap_or(false);
    let mechanism_bin = h0_active as usize + 2 * s8_active as usize; // 0..3
    let fit_bin = if outcome.delta_log_likelihood > 8.0 {
        2
    } else if outcome.delta_log_likelihood > 0.0 {
        1
    } else {
        0
    };
    (param_bin, mechanism_bin, fit_bin)
}

#[derive(Debug, Default, Clone)]
pub struct MapElites {
    pub cells: BTreeMap<(usize, usize, usize), (String, f64)>,
}

impl MapElites {
    pub fn new() -> Self {
        MapElites {
            cells: BTreeMap::new(),
        }
    }
    /// Insert a candidate; the cell keeps the highest-fitness occupant (the elite).
    pub fn insert(&mut self, cell: (usize, usize, usize), candidate_id: &str, fitness: f64) {
        let entry = self
            .cells
            .entry(cell)
            .or_insert_with(|| (candidate_id.to_string(), f64::MIN));
        if fitness > entry.1 {
            *entry = (candidate_id.to_string(), fitness);
        }
    }
    pub fn coverage(&self) -> usize {
        self.cells.len()
    }
    /// QD-Score: sum of cell-best fitness. Monotone, unbounded, ungameable by clustering
    /// (identical behavior maps to a single cell).
    pub fn qd_score(&self) -> f64 {
        self.cells.values().map(|(_, f)| f.max(0.0)).sum()
    }
    pub fn to_json(&self) -> Value {
        json!({
            "record_kind": "map_elites_archive",
            "coverage": self.coverage(),
            "qd_score": round6(self.qd_score()),
            "cells": self.cells.iter().map(|(cell, (id, fit))| json!({
                "cell": [cell.0, cell.1, cell.2],
                "candidate_id": id,
                "fitness": round6(*fit),
            })).collect::<Vec<_>>(),
        })
    }
}

// ---------------------------------------------------------------------------------------
// Whole-pipeline assembly + holistic critique (Phase 3)
// ---------------------------------------------------------------------------------------

/// Detect a cross-pillar contradiction: the assembled theory predicts the same observable
/// twice with materially different values.
pub fn holistic_contradiction(artifact: &TheoryArtifact) -> bool {
    let mut seen: BTreeMap<&str, f64> = BTreeMap::new();
    for p in &artifact.predictions {
        if let Some(prev) = seen.get(p.observable_id.as_str()) {
            let scale = prev.abs().max(1e-9);
            if (prev - p.value).abs() / scale > 0.05 {
                return true;
            }
        } else {
            seen.insert(&p.observable_id, p.value);
        }
    }
    false
}

// ---------------------------------------------------------------------------------------
// Robustness quality gate (Phase 4)
// ---------------------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GateCheck {
    pub name: String,
    pub passed: bool,
    pub observed: Value,
}

/// The anti-saturation / honesty checks that certify a run "real". Pure function over the
/// champion frontier + anchor outcomes; the engine writes these to `quality-gate.json`.
pub fn robustness_gate(
    champion_scores: &[f64],
    decoys_all_killed: bool,
    baseline_survived: bool,
    attack_archive_grew: bool,
) -> Vec<GateCheck> {
    let n = champion_scores.len().max(1);
    let distinct = champion_scores
        .iter()
        .map(|s| (s * 1e6).round() as i64)
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let distinct_ratio = distinct as f64 / n as f64;
    // Largest cluster at any single score value (the old run had 61.8% at 0.995).
    let mut counts: BTreeMap<i64, usize> = BTreeMap::new();
    for s in champion_scores {
        *counts.entry((s * 1e6).round() as i64).or_default() += 1;
    }
    let max_cluster = counts.values().cloned().max().unwrap_or(0) as f64 / n as f64;

    vec![
        GateCheck {
            name: "distinct_champion_score_ratio".into(),
            passed: distinct_ratio >= 0.5,
            observed: json!(round6(distinct_ratio)),
        },
        GateCheck {
            name: "max_single_score_cluster".into(),
            passed: max_cluster <= 0.30,
            observed: json!(round6(max_cluster)),
        },
        GateCheck {
            name: "decoys_all_killed".into(),
            passed: decoys_all_killed,
            observed: json!(decoys_all_killed),
        },
        GateCheck {
            name: "baseline_survived".into(),
            passed: baseline_survived,
            observed: json!(baseline_survived),
        },
        GateCheck {
            name: "attack_archive_grew".into(),
            passed: attack_archive_grew,
            observed: json!(attack_archive_grew),
        },
    ]
}

// ---------------------------------------------------------------------------------------
// Anchor loading for the within-run honesty meta-loop and the gate
// ---------------------------------------------------------------------------------------

pub struct AnchorSet {
    pub survivors: Vec<(TheoryArtifact, RobustnessOutcome)>,
    pub decoys: Vec<(TheoryArtifact, RobustnessOutcome)>,
}

/// Load and evaluate the frozen anchor/decoy set from `ZYAL/anchors`. Survivors must keep
/// surviving as the adversary escalates (honesty loop); decoys must stay dead (calibration).
pub fn load_anchor_set(root: &Path, observables: &[ObservableRecord], baseline_ll: f64) -> AnchorSet {
    let mut survivors = Vec::new();
    let mut decoys = Vec::new();
    if let Ok(rd) = std::fs::read_dir(root.join("ZYAL/anchors")) {
        for entry in rd.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let Ok(artifact) = serde_json::from_str::<TheoryArtifact>(&text) else {
                continue;
            };
            let outcome = artifact.evaluate(observables, baseline_ll);
            match artifact.expected_anchor_outcome.as_str() {
                "survive" => survivors.push((artifact, outcome)),
                "die" => decoys.push((artifact, outcome)),
                _ => {}
            }
        }
    }
    AnchorSet { survivors, decoys }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zyal_robustness::{
        baseline_log_likelihood, load_tension_observables, score_predictions, Genes,
    };
    use openqg_core::ObservableRecord;
    use std::path::PathBuf;

    fn obs() -> Vec<ObservableRecord> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        load_tension_observables(&root).unwrap()
    }

    fn grade_genes(genes: &Genes, observables: &[ObservableRecord], baseline_ll: f64) -> RobustnessOutcome {
        let mut o = score_predictions(
            &genes.forward_map(),
            genes.parameter_count(),
            observables,
            baseline_ll,
            genes.within_physical_bounds(),
        );
        o.genes = serde_json::to_value(genes).unwrap();
        o
    }

    fn resolver() -> Genes {
        Genes {
            delta_h0_local: 5.6,
            s8_suppression: 0.05,
            ..Genes::baseline()
        }
    }

    #[test]
    fn graybox_free_parameter_theory_is_killed_even_if_it_fits() {
        let observables = obs();
        let baseline_ll = baseline_log_likelihood(&observables);
        let archive = AttackArchive::seed();
        let outcome = grade_genes(&resolver(), &observables, baseline_ll);
        assert!(outcome.delta_log_likelihood > 0.0, "the fit itself is good");

        // A theory that fits well but adds a free, ungrounded fudge parameter is gray-box.
        let mut graybox = resolver().to_artifact("graybox");
        graybox.pillars[0].parameters.push(crate::zyal_robustness::ParamMeaning {
            symbol: "k_fudge".into(),
            physical_meaning: String::new(),
            provenance: String::new(),
            kind: "free".into(),
        });
        let v = judge("graybox", &graybox, &outcome, &archive);
        assert!(
            !v.survived && v.survival == 0.0,
            "a gray-box / free-parameter theory must be killed regardless of fit"
        );

        // The grounded whitebox theory survives the same adversary on the same fit.
        let whitebox = resolver().to_artifact("whitebox");
        assert!(whitebox.whitebox_violations().is_empty(), "grounded genome must be whitebox");
        let v2 = judge("whitebox", &whitebox, &outcome, &archive);
        assert!(v2.survived, "a grounded whitebox theory must survive");
    }

    #[test]
    fn hard_attacks_kill_decoys_and_spare_good_theories() {
        let observables = obs();
        let baseline_ll = baseline_log_likelihood(&observables);
        let archive = AttackArchive::seed();

        let r = resolver();
        let ro = grade_genes(&r, &observables, baseline_ll);
        let v = judge("resolver", &r.to_artifact("resolver"), &ro, &archive);
        assert!(v.survived && v.survival > 0.5, "resolver should survive comfortably: {v:?}");

        let broken = Genes { h0: 100.0, ..Genes::baseline() };
        let bo = grade_genes(&broken, &observables, baseline_ll);
        let bv = judge("broken", &broken.to_artifact("broken"), &bo, &archive);
        assert!(!bv.survived && bv.survival == 0.0, "non-physical genome must be killed");
    }

    #[test]
    fn escalating_adversary_makes_survival_a_moving_target() {
        // The SAME candidate must score lower as the adversary escalates: anti-saturation.
        let observables = obs();
        let baseline_ll = baseline_log_likelihood(&observables);
        let r = resolver();
        let ro = grade_genes(&r, &observables, baseline_ll);

        let mut archive = AttackArchive::seed();
        let early = judge("r", &r.to_artifact("r"), &ro, &archive).survival;
        for _ in 0..6 {
            archive.escalate(&BTreeMap::new(), true);
        }
        let late = judge("r", &r.to_artifact("r"), &ro, &archive).survival;
        assert!(late < early, "escalation must lower survival of a fixed candidate ({late} !< {early})");
    }

    #[test]
    fn honesty_rollback_protects_good_anchors() {
        // Escalate far; the best anchor's survival must stay above the floor because escalation
        // rolls back whenever anchors_ok is reported false.
        let observables = obs();
        let baseline_ll = baseline_log_likelihood(&observables);
        let r = resolver();
        let ro = grade_genes(&r, &observables, baseline_ll);
        let mut archive = AttackArchive::seed();
        for _ in 0..50 {
            let anchor_survival = judge("r", &r.to_artifact("r"), &ro, &archive).survival;
            let anchors_ok = anchor_survival >= ANCHOR_SURVIVAL_FLOOR;
            archive.escalate(&BTreeMap::new(), anchors_ok);
        }
        let final_survival = judge("r", &r.to_artifact("r"), &ro, &archive).survival;
        assert!(
            final_survival >= ANCHOR_SURVIVAL_FLOOR - 0.05,
            "honesty rollback must keep the good anchor alive ({final_survival})"
        );
    }

    #[test]
    fn pairwise_elo_orders_by_survival_without_ceiling() {
        let mut verdicts = vec![
            Verdict { candidate_id: "a".into(), survived: true, survival: 0.9, elo: 1500.0, landed: vec![] },
            Verdict { candidate_id: "b".into(), survived: true, survival: 0.5, elo: 1500.0, landed: vec![] },
            Verdict { candidate_id: "c".into(), survived: false, survival: 0.0, elo: 1500.0, landed: vec![] },
        ];
        assign_pairwise_elo(&mut verdicts);
        assert!(verdicts[0].elo > verdicts[1].elo && verdicts[1].elo > verdicts[2].elo);
    }

    #[test]
    fn focus_concentrates_budget_on_the_most_attacked_pillar() {
        let mut success = BTreeMap::new();
        success.insert("foundations".to_string(), 0.1);
        success.insert("coefficients".to_string(), 0.8); // most attacked
        success.insert("observables".to_string(), 0.2);
        let alloc = focus_allocation(&success, 24, 1, 0.3);
        let total: usize = alloc.values().sum();
        assert_eq!(total, 24, "budget must be fully allocated");
        let max_key = alloc.iter().max_by_key(|(_, v)| **v).unwrap().0;
        assert_eq!(max_key, "coefficients", "the most-attacked pillar gets the bulk");
        assert!(alloc.values().all(|&v| v >= 1), "every pillar keeps a floor");
    }

    #[test]
    fn map_elites_coverage_grows_and_qd_is_name_independent() {
        let observables = obs();
        let baseline_ll = baseline_log_likelihood(&observables);
        let mut archive = MapElites::new();
        // A spread of distinct mechanisms/param-counts fills distinct cells.
        let variants = [
            Genes::baseline(),
            Genes { delta_h0_local: 5.6, ..Genes::baseline() },
            Genes { s8_suppression: 0.05, ..Genes::baseline() },
            resolver(),
        ];
        for (i, g) in variants.iter().enumerate() {
            let o = grade_genes(g, &observables, baseline_ll);
            archive.insert(descriptor(&o), &format!("c{i}"), o.delta_log_likelihood.max(0.0));
        }
        assert!(archive.coverage() >= 3, "distinct mechanisms must fill distinct cells");
        assert!(archive.qd_score() > 0.0);
        // Descriptor is computed from numeric outcome only (no names): re-inserting under a
        // different id into the same cell does not change coverage.
        let o = grade_genes(&resolver(), &observables, baseline_ll);
        let before = archive.coverage();
        archive.insert(descriptor(&o), "renamed-stage-xyz", o.delta_log_likelihood.max(0.0));
        assert_eq!(archive.coverage(), before, "behavior cells are name-independent");
    }

    #[test]
    fn gate_flags_saturation_and_passes_a_real_frontier() {
        // A saturated frontier (old behavior) must FAIL the gate.
        let saturated = vec![0.995; 20];
        let g = robustness_gate(&saturated, true, true, true);
        assert!(g.iter().any(|c| c.name == "max_single_score_cluster" && !c.passed));
        // A real spread frontier passes.
        let real: Vec<f64> = (0..20).map(|i| 0.4 + i as f64 * 0.02).collect();
        let g2 = robustness_gate(&real, true, true, true);
        assert!(g2.iter().all(|c| c.passed), "a real spread frontier must pass: {g2:?}");
    }
}
