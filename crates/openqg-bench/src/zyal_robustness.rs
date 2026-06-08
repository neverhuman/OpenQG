//! Phase 1 of the adversarial-robustness rebuild: an *executable phenotype* for ZYAL
//! genome candidates plus the deterministic hard-veto honesty anchors.
//!
//! The previous engine scored candidates from SHA256 string hashes with a hard 0.995
//! saturation cap, decoupled from any real signal. Here a candidate owns a small vector of
//! named physical-parameter *genes*; a deterministic, documented `forward_map` turns those
//! genes into predictions over the tension observable fixture; and the real
//! `openqg_core::score_metrics` grades them. Selection then runs on a genuine, reproducible,
//! unbounded `delta_log_likelihood` (mapped through a monotone, plateau-free sigmoid for the
//! `[0,1]` `final_score` consumers) instead of hash jitter.
//!
//! This module is intentionally LLM-free. The live critic/judge panel (Phase 2) layers on
//! top of — and stays subordinate to — these cheap deterministic checks: the physics veto
//! and the structural anchors can kill a candidate regardless of any judge verdict.

use anyhow::Result;
use openqg_core::{score_metrics, ObservableRecord, PredictionRecord, ScoreMetrics};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;

/// Default location of the Phase 0 tension fixture relative to the repository root.
pub const TENSION_OBSERVABLES: &str = "data/fixtures/tension/observables.jsonl";

/// Hard physics-veto margin: a candidate whose log-likelihood is worse than the baseline by
/// more than this is "clearly broken" (the decoys) and is killed regardless of any judge.
const VETO_LL_MARGIN: f64 = 40.0;
/// Logistic steepness mapping `delta_log_likelihood - parsimony` to a plateau-free `[0,1]`.
const FITNESS_STEEPNESS: f64 = 0.12;
/// Parsimony weight applied to `parameter_count_penalty` (k.log10) in the fitness blend.
const PARSIMONY_WEIGHT: f64 = 2.0;

/// The genome of a candidate theory: a small vector of named physical parameters. Baseline
/// values reproduce `sm-gr-lcdm-mnu`; non-baseline values are "extension knobs" that BIC
/// will penalize unless they buy enough likelihood.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Genes {
    pub h0: f64,
    pub omega_m: f64,
    pub sum_mnu: f64,
    pub n_eff: f64,
    pub omega_b_h2: f64,
    /// How much the local distance-ladder H0 exceeds the CMB H0 (0 == single-H0 baseline).
    pub delta_h0_local: f64,
    /// Late-time growth suppression that lowers S8 below the Planck-LCDM value (0 == none).
    pub s8_suppression: f64,
}

impl Genes {
    /// The LambdaCDM baseline genome (single H0, no extension knobs active).
    pub fn baseline() -> Self {
        Genes {
            h0: 67.4,
            omega_m: 0.315,
            sum_mnu: 0.06,
            n_eff: 3.046,
            omega_b_h2: 0.02237,
            delta_h0_local: 0.0,
            s8_suppression: 0.0,
        }
    }

    /// Number of *free* parameters: the 3-parameter LambdaCDM core plus any extension knob
    /// that meaningfully deviates from its baseline default. Drives the BIC/MDL penalty.
    pub fn parameter_count(&self) -> usize {
        let mut k = 3; // h0, omega_m, sum_mnu (always counted)
        if (self.n_eff - 3.046).abs() > 0.02 {
            k += 1;
        }
        if (self.omega_b_h2 - 0.02237).abs() > 0.0002 {
            k += 1;
        }
        if self.delta_h0_local.abs() > 0.1 {
            k += 1;
        }
        if self.s8_suppression.abs() > 0.005 {
            k += 1;
        }
        k
    }

    /// Deterministic, documented forward map from genes to predicted observables. This is a
    /// transparent parametric stand-in, NOT a solver — a real `openqg-theory predict` binary
    /// can later replace this behind the same signature without touching the evolution loop.
    pub fn forward_map(&self) -> Vec<PredictionRecord> {
        let pred = |id: &str, value: f64, uncertainty: f64, unit: &str| PredictionRecord {
            observable_id: id.into(),
            value,
            uncertainty,
            unit: unit.into(),
            theory_id: None,
        };
        vec![
            pred("h0", self.h0, 0.5, "km s^-1 Mpc^-1"),
            pred("omega_m", self.omega_m, 0.007, "dimensionless"),
            pred("sum_mnu", self.sum_mnu, 0.02, "eV"),
            // Single-H0 baseline predicts h0 here; the extension knob lets it reach the local value.
            pred(
                "h0_local",
                self.h0 + self.delta_h0_local,
                1.0,
                "km s^-1 Mpc^-1",
            ),
            // Planck-LCDM S8 ~ 0.83 scaled mildly by matter density, minus any suppression.
            pred(
                "s8",
                0.83 * (self.omega_m / 0.315).sqrt() - self.s8_suppression,
                0.017,
                "dimensionless",
            ),
            pred("n_eff", self.n_eff, 0.17, "dimensionless"),
            pred("omega_b_h2", self.omega_b_h2, 0.00015, "dimensionless"),
        ]
    }

    /// Project the genome into a structured theory artifact so the critic/judge panel has
    /// real content (claims, assumptions, declared limits, falsifiers) to attack. Pillars are
    /// emitted for whichever extension knobs are active.
    pub fn to_artifact(&self, id: &str) -> TheoryArtifact {
        let mut pillars = vec![Pillar {
            name: "concordance-background".into(),
            claim: "An FLRW background with the stated densities reproduces the CMB-anchored observables.".into(),
            mechanism: "LambdaCDM expansion history".into(),
            assumptions: vec!["spatial flatness".into(), "GR on cosmological scales".into()],
            declared_limits: vec![DeclaredLimit {
                limit_name: "newtonian".into(),
                reduces_to: "Newtonian gravity in the weak-field limit".into(),
                check: "phi << c^2".into(),
            }],
            falsifiers: vec!["concordance observables inconsistent at high significance".into()],
            parameters: vec![
                ParamMeaning {
                    symbol: "H0".into(),
                    physical_meaning: "present-day Hubble expansion rate".into(),
                    provenance: "Planck 2018 cosmological parameters".into(),
                    kind: "fundamental_constant".into(),
                },
                ParamMeaning {
                    symbol: "Omega_m".into(),
                    physical_meaning: "total non-relativistic matter density fraction".into(),
                    provenance: "Planck 2018 cosmological parameters".into(),
                    kind: "fundamental_constant".into(),
                },
                ParamMeaning {
                    symbol: "sum_mnu".into(),
                    physical_meaning: "summed neutrino mass scale".into(),
                    provenance: "neutrino oscillation data + cosmological bounds".into(),
                    kind: "fundamental_constant".into(),
                },
            ],
        }];
        if self.delta_h0_local.abs() > 0.1 {
            pillars.push(Pillar {
                name: "h0-reconciliation".into(),
                claim: "A short pre-recombination process lifts the local distance-ladder H0 above the CMB value.".into(),
                mechanism: "brief early dark energy injection near matter-radiation equality".into(),
                assumptions: vec!["injection ends before recombination".into()],
                declared_limits: vec![DeclaredLimit {
                    limit_name: "lcdm".into(),
                    reduces_to: "standard LambdaCDM as the injected fraction -> 0".into(),
                    check: "f_ede -> 0".into(),
                }],
                falsifiers: vec!["local H0 measured consistent with the CMB value".into()],
                parameters: vec![ParamMeaning {
                    symbol: "f_ede".into(),
                    physical_meaning: "peak fractional energy density of the early-dark-energy field".into(),
                    provenance: "early dark energy mechanism (Poulin et al. 2019)".into(),
                    kind: "derived".into(),
                }],
            });
        }
        if self.s8_suppression.abs() > 0.005 {
            pillars.push(Pillar {
                name: "growth-suppression".into(),
                claim: "Late-time, scale-dependent growth suppression lowers S8 toward the lensing value.".into(),
                mechanism: "scale-dependent growth from a dark-sector interaction".into(),
                assumptions: vec!["suppression is late-time".into()],
                declared_limits: vec![DeclaredLimit {
                    limit_name: "lcdm".into(),
                    reduces_to: "unsuppressed growth as the coupling -> 0".into(),
                    check: "xi_dm -> 0".into(),
                }],
                falsifiers: vec!["S8 measured consistent with the Planck-LCDM value".into()],
                parameters: vec![ParamMeaning {
                    symbol: "xi_dm".into(),
                    physical_meaning: "dark-sector interaction strength setting late-time growth suppression".into(),
                    provenance: "interacting dark matter/energy models".into(),
                    kind: "derived".into(),
                }],
            });
        }
        TheoryArtifact {
            id: id.to_string(),
            kind: "candidate".into(),
            expected_anchor_outcome: String::new(),
            summary: "Genome-derived candidate theory.".into(),
            pillars,
            predictions: self.forward_map(),
            parameter_count: self.parameter_count(),
        }
    }

    /// Physical sanity bounds. Anything outside is non-physical and gets vetoed.
    pub fn within_physical_bounds(&self) -> bool {
        (50.0..=90.0).contains(&self.h0)
            && (0.05..=0.95).contains(&self.omega_m)
            && (0.0..=2.0).contains(&self.sum_mnu)
            && (1.0..=6.0).contains(&self.n_eff)
            && (0.005..=0.05).contains(&self.omega_b_h2)
            && (-1.0..=20.0).contains(&self.delta_h0_local)
            && (-0.2..=0.5).contains(&self.s8_suppression)
    }
}

/// Deterministic 64-bit mix (splitmix64) — numeric only, so candidate identity (NOT any
/// stage/island *name*) drives gene sampling. This is what makes "rename a stage and every
/// champion id is byte-identical" hold.
fn mix(mut x: u64) -> u64 {
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
    x ^ (x >> 31)
}

fn unit(generation_index: usize, candidate_index: usize, seed: u64, salt: u64) -> f64 {
    let key = mix(seed)
        ^ mix(generation_index as u64).rotate_left(17)
        ^ mix(candidate_index as u64).rotate_left(33)
        ^ mix(salt).rotate_left(7);
    (mix(key) >> 11) as f64 / (1u64 << 53) as f64
}

fn lerp(u: f64, lo: f64, hi: f64) -> f64 {
    lo + u * (hi - lo)
}

/// Sample a candidate's genes deterministically from its NUMERIC identity. Different
/// candidates explore different points of parameter space; the same identity always yields
/// the same genome (reproducibility). Inheritance/mutation from parents is a Phase 3 refinement.
pub fn derive_genes(generation_index: usize, candidate_index: usize, seed: u64) -> Genes {
    Genes {
        h0: lerp(unit(generation_index, candidate_index, seed, 1), 66.0, 69.0),
        omega_m: lerp(unit(generation_index, candidate_index, seed, 2), 0.30, 0.33),
        sum_mnu: lerp(unit(generation_index, candidate_index, seed, 3), 0.0, 0.15),
        n_eff: lerp(unit(generation_index, candidate_index, seed, 4), 2.7, 3.3),
        omega_b_h2: lerp(
            unit(generation_index, candidate_index, seed, 5),
            0.0220,
            0.0228,
        ),
        delta_h0_local: lerp(unit(generation_index, candidate_index, seed, 6), 0.0, 7.0),
        s8_suppression: lerp(unit(generation_index, candidate_index, seed, 7), 0.0, 0.08),
    }
}

/// Parse stored genes (from a candidate record's `physics.genes`) back into a `Genes`.
pub fn genes_from_json(value: &Value) -> Option<Genes> {
    serde_json::from_value(value.clone()).ok()
}

fn centered(generation_index: usize, candidate_index: usize, seed: u64, salt: u64) -> f64 {
    2.0 * unit(generation_index, candidate_index, seed, salt) - 1.0
}

fn clampr(value: f64, lo: f64, hi: f64) -> f64 {
    value.max(lo).min(hi)
}

/// Uniform crossover: each parameter is inherited from one parent or the other by a
/// deterministic coin.
fn recombine(
    a: &Genes,
    b: &Genes,
    generation_index: usize,
    candidate_index: usize,
    seed: u64,
) -> Genes {
    let pick = |salt: u64, x: f64, y: f64| {
        if unit(generation_index, candidate_index, seed, salt) < 0.5 {
            x
        } else {
            y
        }
    };
    Genes {
        h0: pick(21, a.h0, b.h0),
        omega_m: pick(22, a.omega_m, b.omega_m),
        sum_mnu: pick(23, a.sum_mnu, b.sum_mnu),
        n_eff: pick(24, a.n_eff, b.n_eff),
        omega_b_h2: pick(25, a.omega_b_h2, b.omega_b_h2),
        delta_h0_local: pick(26, a.delta_h0_local, b.delta_h0_local),
        s8_suppression: pick(27, a.s8_suppression, b.s8_suppression),
    }
}

/// Inheritance + mutation: produce a child genome from its (already-selected) parents. With no
/// parents (generation one) a fresh genome is sampled; otherwise the child inherits — single
/// parent or uniform crossover of two — and is perturbed by an amount keyed to the mutation
/// operator. This is what lets selection pressure climb the fitness landscape against the
/// escalating adversary instead of re-sampling blindly each generation.
pub fn mutate_genes(
    parents: &[Genes],
    mutation_op: &str,
    generation_index: usize,
    candidate_index: usize,
    seed: u64,
) -> Genes {
    let mut g = match parents.len() {
        0 => return derive_genes(generation_index, candidate_index, seed),
        1 => parents[0].clone(),
        _ => recombine(
            &parents[0],
            &parents[1],
            generation_index,
            candidate_index,
            seed,
        ),
    };
    // Step size per operator: explore widely on novelty jumps, refine on contract tightening.
    let scale = match mutation_op {
        "novelty_jump" => 1.0,
        "domain_bridge" | "source_card_graft" => 0.5,
        "cross_stage_recombine" => 0.3,
        "contract_tighten" => 0.12,
        _ => 0.25,
    };
    let step = |salt: u64| centered(generation_index, candidate_index, seed, salt);
    g.h0 = clampr(g.h0 + scale * 1.5 * step(31), 50.0, 90.0);
    g.omega_m = clampr(g.omega_m + scale * 0.015 * step(32), 0.05, 0.95);
    g.sum_mnu = clampr(g.sum_mnu + scale * 0.075 * step(33), 0.0, 2.0);
    g.n_eff = clampr(g.n_eff + scale * 0.3 * step(34), 1.0, 6.0);
    g.omega_b_h2 = clampr(g.omega_b_h2 + scale * 0.0004 * step(35), 0.005, 0.05);
    g.delta_h0_local = clampr(g.delta_h0_local + scale * 3.5 * step(36), -1.0, 20.0);
    g.s8_suppression = clampr(g.s8_suppression + scale * 0.04 * step(37), -0.2, 0.5);
    // failure_mode_invert flips the dominant extension knob to probe the opposite regime.
    if mutation_op == "failure_mode_invert" {
        g.delta_h0_local = if g.delta_h0_local.abs() > 0.1 {
            0.0
        } else {
            5.6
        };
    }
    g
}

/// Outcome of evaluating a candidate against the cheap, deterministic honesty layer.
#[derive(Debug, Clone)]
pub struct RobustnessOutcome {
    /// Monotone, plateau-free `[0,1]` score for the existing `final_score` consumers.
    pub final_score: f64,
    /// The real, unbounded discovery currency (vs the frozen baseline).
    pub delta_log_likelihood: f64,
    pub log_likelihood: f64,
    pub bic: f64,
    pub coverage: f64,
    pub parameter_count: usize,
    pub vetoed: bool,
    pub veto_reasons: Vec<String>,
    pub genes: Value,
    pub predictions: Value,
    pub metrics: ScoreMetrics,
}

impl RobustnessOutcome {
    /// JSON block embedded into a candidate record under `physics` for full auditability.
    pub fn physics_block(&self) -> Value {
        json!({
            "delta_log_likelihood": round6(self.delta_log_likelihood),
            "log_likelihood": round6(self.log_likelihood),
            "bic": round6(self.bic),
            "coverage": round6(self.coverage),
            "parameter_count": self.parameter_count,
            "vetoed": self.vetoed,
            "veto_reasons": self.veto_reasons,
            "genes": self.genes,
            "predictions": self.predictions,
        })
    }
}

fn round6(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}

fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

/// Score a concrete prediction set + parameter count against the observables, applying the
/// physics veto. This is the single grading path shared by genome candidates, anchors, and
/// (later) LLM-proposed artifacts.
pub fn score_predictions(
    predictions: &[PredictionRecord],
    parameter_count: usize,
    observables: &[ObservableRecord],
    baseline_ll: f64,
    bounds_ok: bool,
) -> RobustnessOutcome {
    let (metrics, findings) = score_metrics(observables, predictions, parameter_count, baseline_ll);
    let mut veto_reasons = Vec::new();
    if !bounds_ok {
        veto_reasons.push("non_physical_parameters".to_string());
    }
    if metrics.coverage < 1.0 {
        veto_reasons.push(format!("incomplete_coverage:{:.3}", metrics.coverage));
    }
    if metrics.invalid_prediction_count > 0 {
        veto_reasons.push(format!(
            "invalid_predictions:{}",
            metrics.invalid_prediction_count
        ));
    }
    if metrics.log_likelihood < baseline_ll - VETO_LL_MARGIN {
        veto_reasons.push("far_worse_than_baseline".to_string());
    }
    if !findings.is_empty() {
        veto_reasons.push(format!("findings:{}", findings.len()));
    }
    let vetoed = !veto_reasons.is_empty();

    // Unbounded discovery currency, parsimony-penalized; mapped to a plateau-free [0,1].
    let penalized =
        metrics.delta_log_likelihood - PARSIMONY_WEIGHT * metrics.parameter_count_penalty;
    let final_score = if vetoed {
        0.0
    } else {
        sigmoid(FITNESS_STEEPNESS * penalized).clamp(0.0, 0.999)
    };

    RobustnessOutcome {
        final_score,
        delta_log_likelihood: metrics.delta_log_likelihood,
        log_likelihood: metrics.log_likelihood,
        bic: metrics.bic,
        coverage: metrics.coverage,
        parameter_count,
        vetoed,
        veto_reasons,
        genes: Value::Null,
        predictions: serde_json::to_value(predictions).unwrap_or(Value::Null),
        metrics,
    }
}

/// Evaluate a genome candidate end to end (derive genes → forward map → veto + fitness).
pub fn evaluate_candidate(
    generation_index: usize,
    candidate_index: usize,
    seed: u64,
    observables: &[ObservableRecord],
    baseline_ll: f64,
) -> RobustnessOutcome {
    let genes = derive_genes(generation_index, candidate_index, seed);
    let predictions = genes.forward_map();
    let mut outcome = score_predictions(
        &predictions,
        genes.parameter_count(),
        observables,
        baseline_ll,
        genes.within_physical_bounds(),
    );
    outcome.genes = serde_json::to_value(&genes).unwrap_or(Value::Null);
    outcome
}

/// Frozen baseline log-likelihood, computed once from the LambdaCDM baseline genome over the
/// tension fixture. This replaces the hardcoded `0.0` / `HYBRID_BASELINE_SCORE` constants.
pub fn baseline_log_likelihood(observables: &[ObservableRecord]) -> f64 {
    let (metrics, _) = score_metrics(observables, &Genes::baseline().forward_map(), 3, 0.0);
    metrics.log_likelihood
}

/// Load the tension observable fixture relative to a repository root.
pub fn load_tension_observables(root: &Path) -> Result<Vec<ObservableRecord>> {
    crate::util::read_jsonl(&root.join(TENSION_OBSERVABLES))
}

// ----- Structured theory artifact (anchors, decoys, and Phase 2 LLM proposals) -----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclaredLimit {
    pub limit_name: String,
    pub reduces_to: String,
    #[serde(default)]
    pub check: String,
}

/// A named parameter with its physical meaning and provenance. Whitebox theories require every
/// parameter to be a grounded constant or a derived quantity — never a free fudge factor tuned
/// to fit data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamMeaning {
    pub symbol: String,
    #[serde(default)]
    pub physical_meaning: String,
    #[serde(default)]
    pub provenance: String,
    #[serde(default)]
    pub kind: String, // "fundamental_constant" | "derived" | "free"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pillar {
    pub name: String,
    pub claim: String,
    #[serde(default)]
    pub mechanism: String,
    #[serde(default)]
    pub assumptions: Vec<String>,
    #[serde(default)]
    pub declared_limits: Vec<DeclaredLimit>,
    #[serde(default)]
    pub falsifiers: Vec<String>,
    #[serde(default)]
    pub parameters: Vec<ParamMeaning>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TheoryArtifact {
    pub id: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub expected_anchor_outcome: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub pillars: Vec<Pillar>,
    #[serde(default)]
    pub predictions: Vec<PredictionRecord>,
    #[serde(default = "default_parameter_count")]
    pub parameter_count: usize,
}

fn default_parameter_count() -> usize {
    3
}

impl TheoryArtifact {
    /// Cheap, deterministic STRUCTURAL anchors: a robust theory must be falsifiable and
    /// internally well-formed. Returns the list of violations (empty == structurally sound).
    /// These run independently of physics and of any LLM judge.
    pub fn structural_violations(&self) -> Vec<String> {
        let mut v = Vec::new();
        if self.pillars.is_empty() {
            v.push("no_pillars".to_string());
        }
        if self.pillars.iter().any(|p| p.claim.trim().is_empty()) {
            v.push("pillar_without_claim".to_string());
        }
        let has_falsifier =
            !self.pillars.is_empty() && self.pillars.iter().any(|p| !p.falsifiers.is_empty());
        if !has_falsifier {
            v.push("no_falsifiable_prediction".to_string());
        }
        if self.predictions.is_empty() {
            v.push("no_predictions".to_string());
        }
        // Parsimony sanity: a "theory" with a free parameter per observable is memorization.
        if self.parameter_count >= self.predictions.len().max(1) && self.predictions.len() > 1 {
            v.push("parameter_per_observable_overfit".to_string());
        }
        // Must declare at least one known-limit recovery (e.g. GR/Newtonian/LCDM).
        let declares_limit = self.pillars.iter().any(|p| !p.declared_limits.is_empty());
        if !declares_limit {
            v.push("no_declared_limit".to_string());
        }
        v
    }

    /// Whitebox enforcement: OpenQG only considers REAL theories. Every parameter must carry a
    /// physical meaning + provenance and be a grounded constant or derived quantity — never a
    /// free fudge factor, and never black/gray-box or "tuned to fit". Returns the violations
    /// (empty == whitebox). A candidate with any violation is killed by the GrayBox critic.
    pub fn whitebox_violations(&self) -> Vec<String> {
        const MARKERS: [&str; 12] = [
            "black box",
            "black-box",
            "gray box",
            "gray-box",
            "grey box",
            "grey-box",
            "latent",
            "fudge",
            "tuned to fit",
            "fit to data",
            "curve fit",
            "free parameter",
        ];
        let flagged = |text: &str| {
            let lower = text.to_lowercase();
            MARKERS.iter().any(|marker| lower.contains(marker))
        };
        let mut v = Vec::new();
        for pillar in &self.pillars {
            if flagged(&pillar.claim) || flagged(&pillar.mechanism) {
                v.push("black_box_language".to_string());
            }
            for param in &pillar.parameters {
                if param.kind == "free" {
                    v.push(format!("free_parameter:{}", param.symbol));
                }
                if param.physical_meaning.trim().is_empty() {
                    v.push(format!("parameter_without_meaning:{}", param.symbol));
                }
                if param.provenance.trim().is_empty() {
                    v.push(format!("parameter_without_provenance:{}", param.symbol));
                }
            }
        }
        v
    }

    /// Evaluate the artifact's physics against the fixture (used to calibrate anchors and to
    /// anchor the LLM judge in later phases).
    pub fn evaluate(
        &self,
        observables: &[ObservableRecord],
        baseline_ll: f64,
    ) -> RobustnessOutcome {
        score_predictions(
            &self.predictions,
            self.parameter_count,
            observables,
            baseline_ll,
            true,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn observables() -> Vec<ObservableRecord> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        load_tension_observables(&root).expect("tension fixture loads")
    }

    #[test]
    fn baseline_has_headroom_but_is_not_vetoed() {
        let obs = observables();
        let baseline_ll = baseline_log_likelihood(&obs);
        assert!(
            baseline_ll < 0.0,
            "baseline must not perfectly fit the tension data"
        );
        // Scoring the baseline against itself: delta == 0, survives the veto.
        let outcome =
            score_predictions(&Genes::baseline().forward_map(), 3, &obs, baseline_ll, true);
        assert!(
            !outcome.vetoed,
            "baseline must survive the physics veto: {:?}",
            outcome.veto_reasons
        );
        assert!(outcome.delta_log_likelihood.abs() < 1e-9);
    }

    #[test]
    fn tension_resolving_genes_beat_baseline_and_broken_genes_are_vetoed() {
        let obs = observables();
        let baseline_ll = baseline_log_likelihood(&obs);

        // A genome that turns on the H0 and S8 knobs to resolve the tensions.
        let resolver = Genes {
            delta_h0_local: 5.6,
            s8_suppression: 0.05,
            ..Genes::baseline()
        };
        let r = score_predictions(
            &resolver.forward_map(),
            resolver.parameter_count(),
            &obs,
            baseline_ll,
            resolver.within_physical_bounds(),
        );
        assert!(!r.vetoed, "resolver must survive: {:?}", r.veto_reasons);
        assert!(r.delta_log_likelihood > 0.0, "resolver must beat baseline");

        // A non-physical genome (H0 = 100) must be vetoed.
        let broken = Genes {
            h0: 100.0,
            ..Genes::baseline()
        };
        let b = score_predictions(
            &broken.forward_map(),
            broken.parameter_count(),
            &obs,
            baseline_ll,
            broken.within_physical_bounds(),
        );
        assert!(b.vetoed, "broken genome must be vetoed");
        assert_eq!(b.final_score, 0.0);
    }

    #[test]
    fn fitness_is_monotone_and_does_not_saturate() {
        let obs = observables();
        let baseline_ll = baseline_log_likelihood(&obs);
        // Sweep the H0 extension knob; fitness must increase toward the tension-resolving value
        // and never clamp to a single plateau value.
        let mut scores = Vec::new();
        for i in 0..=10 {
            let g = Genes {
                delta_h0_local: i as f64 * 0.56,
                s8_suppression: 0.05,
                ..Genes::baseline()
            };
            let o = score_predictions(
                &g.forward_map(),
                g.parameter_count(),
                &obs,
                baseline_ll,
                true,
            );
            scores.push(o.final_score);
        }
        let distinct = scores
            .iter()
            .map(|s| (s * 1e6) as i64)
            .collect::<std::collections::BTreeSet<_>>();
        assert!(
            distinct.len() >= scores.len() - 1,
            "fitness must be spread, not saturated: {scores:?}"
        );
        assert!(
            scores.last().unwrap() > &scores[0],
            "moving toward the local H0 must help"
        );
    }

    #[test]
    fn genes_are_name_independent_and_reproducible() {
        // Same numeric identity -> identical genes (no stage/island name in the inputs).
        let a = derive_genes(42, 7, 99);
        let b = derive_genes(42, 7, 99);
        assert_eq!(a, b);
        let c = derive_genes(42, 8, 99);
        assert_ne!(a, c);
    }

    #[test]
    fn mutation_inherits_from_parents_and_stays_in_bounds() {
        // No parents -> fresh sample equals derive_genes.
        assert_eq!(
            mutate_genes(&[], "novelty_jump", 5, 3, 99),
            derive_genes(5, 3, 99)
        );
        // Reproducible.
        let parent = resolver_like();
        let c1 = mutate_genes(std::slice::from_ref(&parent), "contract_tighten", 5, 3, 99);
        let c2 = mutate_genes(std::slice::from_ref(&parent), "contract_tighten", 5, 3, 99);
        assert_eq!(c1, c2);
        // A tightening child stays close to its parent; a novelty jump moves further.
        let near = mutate_genes(std::slice::from_ref(&parent), "contract_tighten", 5, 3, 99);
        let far = mutate_genes(std::slice::from_ref(&parent), "novelty_jump", 5, 3, 99);
        assert!((near.h0 - parent.h0).abs() <= (far.h0 - parent.h0).abs() + 1e-9);
        // Child genome is always physical.
        assert!(
            far.within_physical_bounds(),
            "mutated child must stay in bounds"
        );
    }

    fn resolver_like() -> Genes {
        Genes {
            delta_h0_local: 5.6,
            s8_suppression: 0.05,
            ..Genes::baseline()
        }
    }

    #[test]
    fn anchor_artifacts_calibrate_and_structural_checks_fire() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let obs = observables();
        let baseline_ll = baseline_log_likelihood(&obs);
        let mut checked = 0;
        for entry in std::fs::read_dir(root.join("ZYAL/anchors")).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let artifact: TheoryArtifact =
                serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            let outcome = artifact.evaluate(&obs, baseline_ll);
            match artifact.expected_anchor_outcome.as_str() {
                "die" => assert!(
                    outcome.vetoed
                        || outcome.delta_log_likelihood < 0.0
                        || !artifact.whitebox_violations().is_empty(),
                    "decoy {} must be killed by physics or the whitebox gate",
                    artifact.id
                ),
                "survive" => assert!(
                    !outcome.vetoed && outcome.delta_log_likelihood >= 0.0,
                    "good anchor {} must survive and not be worse than baseline ({:?})",
                    artifact.id,
                    outcome.veto_reasons
                ),
                "demote_by_parsimony" => assert!(
                    !artifact.structural_violations().is_empty(),
                    "overfit probe {} must trip a structural anchor",
                    artifact.id
                ),
                _ => {}
            }
            checked += 1;
        }
        assert!(
            checked >= 4,
            "expected >=4 anchor artifacts, found {checked}"
        );
    }
}
