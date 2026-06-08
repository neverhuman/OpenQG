//! Minimal deterministic MAP-Elites evolution loop over symbolic [`Theory`] candidates — the
//! clean reference engine that drives `mutate`/`recombine` → [`assess`] through a
//! quality-diversity archive. It demonstrates the whole rebuilt foundation end-to-end and is the
//! logic the live bench engine will adopt in place of float-vector genes.
//!
//! MAP-Elites keeps the *best* candidate in each behavioral cell (a numeric, name-independent
//! descriptor), so the population stays diverse and elites are never lost — the GR/ΛCDM baseline
//! holds its cell unless something genuinely beats it there, and structurally-broken candidates
//! (vetoed ⇒ `final_fitness = 0`) can never become champions.

use super::{
    anchor_set, assess, mutate, recombine, run_veto_cascade, Adversary, AnchorKind,
    CandidateAssessment, Rng, Theory,
};
use crate::cosmology::{CosmologyParams, ForwardModel};
use crate::types::ObservableRecord;
use serde::Serialize;
use std::collections::BTreeMap;

/// Behavioral descriptor for the MAP-Elites archive: (modifies-gravity, parameter-count bin,
/// fitness bin). Numeric and name-independent so the archive can't be gamed by labels.
pub type Cell = (u8, u8, u8);

fn behavior_cell(theory: &Theory, assessment: &CandidateAssessment) -> Cell {
    let grav = u8::from(theory.modifies_gravity());
    let params = theory.parameters.len().min(5) as u8;
    let fit_bin = match assessment.final_fitness {
        f if f > 0.7 => 2,
        f if f > 0.4 => 1,
        _ => 0,
    };
    (grav, params, fit_bin)
}

/// The best candidate found in one behavioral cell.
#[derive(Debug, Clone)]
pub struct Champion {
    pub theory: Theory,
    pub assessment: CandidateAssessment,
}

/// The outcome of an evolution run.
#[derive(Debug, Clone)]
pub struct EvolutionResult {
    /// Best candidate per behavioral cell (the quality-diversity archive).
    pub archive: BTreeMap<Cell, Champion>,
    /// Quality-diversity score: sum of cell-best `final_fitness` (monotone, unbounded).
    pub qd_score: f64,
    /// The single most-fit credible champion, if any.
    pub champion: Option<Champion>,
    pub generations: usize,
}

fn insert(archive: &mut BTreeMap<Cell, Champion>, theory: Theory, assessment: CandidateAssessment) {
    let cell = behavior_cell(&theory, &assessment);
    let better = match archive.get(&cell) {
        Some(occupant) => assessment.final_fitness > occupant.assessment.final_fitness,
        None => true,
    };
    if better {
        archive.insert(cell, Champion { theory, assessment });
    }
}

/// Run the evolution loop. Deterministic in `seed`: the same inputs reproduce the same archive.
pub fn evolve<M>(
    seeds: &[Theory],
    observables: &[ObservableRecord],
    model: &M,
    baseline_log_likelihood: f64,
    generations: usize,
    population: usize,
    seed: u64,
) -> EvolutionResult
where
    M: ForwardModel<Theory = CosmologyParams>,
{
    let mut rng = Rng::new(seed);
    let mut archive: BTreeMap<Cell, Champion> = BTreeMap::new();

    for theory in seeds {
        let a = assess(theory, observables, model, baseline_log_likelihood);
        insert(&mut archive, theory.clone(), a);
    }

    for _ in 0..generations {
        // Snapshot the current elites to breed from (avoid mutating while iterating).
        let elites: Vec<Theory> = archive.values().map(|c| c.theory.clone()).collect();
        if elites.is_empty() {
            break;
        }
        for _ in 0..population {
            let i = (rng.next_u64() as usize) % elites.len();
            let child = if elites.len() > 1 && rng.chance(0.3) {
                let mut j = (rng.next_u64() as usize) % elites.len();
                if j == i {
                    j = (j + 1) % elites.len();
                }
                recombine(&elites[i], &elites[j], &mut rng)
            } else {
                mutate(&elites[i], &mut rng, 0.6)
            };
            let a = assess(&child, observables, model, baseline_log_likelihood);
            insert(&mut archive, child, a);
        }
    }

    let qd_score = archive.values().map(|c| c.assessment.final_fitness).sum();
    let champion = archive
        .values()
        .filter(|c| c.assessment.is_credible())
        .max_by(|a, b| {
            a.assessment
                .final_fitness
                .partial_cmp(&b.assessment.final_fitness)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .cloned();

    EvolutionResult {
        archive,
        qd_score,
        champion,
        generations,
    }
}

/// The most-fit credible candidate that also **clears the adversary's frontier** (pressured
/// fitness > 0, i.e. `final_fitness > margin`). `None` if nothing in the archive beats the bar —
/// the honest signal that the rising frontier has out-run the whole population. Because the margin
/// is a constant offset it does not change the *ranking*; its job is to *cull* sub-frontier
/// candidates, so this is the most-fit credible elite restricted to survivors of the threshold.
fn pressured_champion(archive: &BTreeMap<Cell, Champion>, margin: f64) -> Option<Champion> {
    archive
        .values()
        .filter(|c| c.assessment.is_credible() && c.assessment.final_fitness > margin)
        .max_by(|a, b| {
            a.assessment
                .final_fitness
                .partial_cmp(&b.assessment.final_fitness)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .cloned()
}

/// Per-generation telemetry emitted by [`evolve_run`] (one JSONL line per generation when logged).
#[derive(Debug, Clone, Serialize)]
pub struct GenerationReport {
    pub generation: usize,
    pub qd_score: f64,
    pub archive_cells: usize,
    /// The co-evolving adversary's current frontier margin (the rising bar).
    pub frontier_margin: f64,
    /// Mean pressured fitness of the `Survive` anchors (the honesty-loop signal).
    pub anchor_health: f64,
    /// Whether the frozen anchor/decoy set still calibrated this generation (good survive, decoys
    /// die). If this ever goes false the engine's honesty has broken.
    pub calibration_honest: bool,
    pub champion_id: Option<String>,
    pub champion_fitness: Option<f64>,
    /// The champion's fitness after the adversary's frontier margin (does it still beat the bar?).
    pub champion_pressured_fitness: Option<f64>,
    pub champion_credible: bool,
}

/// Adversarial, observed evolution run: the same MAP-Elites loop as [`evolve`], but each generation
/// a co-evolving [`Adversary`] escalates a frontier the champion must keep beating — with **honesty
/// rollback** driven by the `Survive` anchors so the pressure never unfairly kills the reference —
/// the frozen anchor/decoy set is re-checked for calibration, and `observer` is invoked with a
/// [`GenerationReport`]. The archive itself is unchanged (cell-bests by `final_fitness`); the
/// adversary gates champion-eligibility and is the telemetry's pulse, sustaining "robustness under
/// judge" over a long run rather than letting it saturate.
pub fn evolve_run<M, F>(
    seeds: &[Theory],
    observables: &[ObservableRecord],
    model: &M,
    baseline_log_likelihood: f64,
    generations: usize,
    population: usize,
    seed: u64,
    mut observer: F,
) -> EvolutionResult
where
    M: ForwardModel<Theory = CosmologyParams>,
    F: FnMut(&GenerationReport),
{
    let mut rng = Rng::new(seed);
    let mut archive: BTreeMap<Cell, Champion> = BTreeMap::new();
    let mut adversary = Adversary::new();

    for theory in seeds {
        let a = assess(theory, observables, model, baseline_log_likelihood);
        insert(&mut archive, theory.clone(), a);
    }

    for gen in 0..generations {
        // The adversary's frontier acts as a *survival threshold*: only candidates whose
        // final_fitness clears the current margin are allowed to breed. The rising bar therefore
        // progressively prunes the weakest cells from the gene pool (a real selection effect, not
        // the old constant offset that left the ranking — and thus the output — untouched). If the
        // bar momentarily out-runs every cell, fall back to the full archive so the run does not
        // collapse; the honesty rollback lowers the margin next generation.
        let margin = adversary.frontier_margin;
        let mut elites: Vec<Theory> = archive
            .values()
            .filter(|c| c.assessment.final_fitness > margin)
            .map(|c| c.theory.clone())
            .collect();
        if elites.is_empty() {
            elites = archive.values().map(|c| c.theory.clone()).collect();
        }
        if elites.is_empty() {
            break;
        }
        for _ in 0..population {
            let i = (rng.next_u64() as usize) % elites.len();
            let child = if elites.len() > 1 && rng.chance(0.3) {
                let mut j = (rng.next_u64() as usize) % elites.len();
                if j == i {
                    j = (j + 1) % elites.len();
                }
                recombine(&elites[i], &elites[j], &mut rng)
            } else {
                mutate(&elites[i], &mut rng, 0.6)
            };
            let a = assess(&child, observables, model, baseline_log_likelihood);
            insert(&mut archive, child, a);
        }

        // Re-assess the frozen anchor/decoy set once: drives both the honesty rollback (Survive
        // anchors' pressured health) and the calibration self-check (all anchors' outcomes).
        let mut survive_assessments = Vec::new();
        let mut calibration_honest = true;
        for anchor in anchor_set() {
            let vetoed = !run_veto_cascade(&anchor.theory).is_empty();
            let asmt = assess(&anchor.theory, observables, model, baseline_log_likelihood);
            let ok = match anchor.kind {
                AnchorKind::Survive => !vetoed && asmt.is_credible(),
                AnchorKind::Die => vetoed || !asmt.is_credible(),
            };
            if !ok {
                calibration_honest = false;
            }
            if anchor.kind == AnchorKind::Survive {
                survive_assessments.push(asmt);
            }
        }
        adversary.update(&survive_assessments);

        // The reported champion must clear the (post-update) frontier — the adversary now gates
        // champion-eligibility, not just the telemetry.
        let champion = pressured_champion(&archive, adversary.frontier_margin);
        let anchor_health = adversary.anchor_health(&survive_assessments);
        let report = GenerationReport {
            generation: gen + 1,
            qd_score: archive.values().map(|c| c.assessment.final_fitness).sum(),
            archive_cells: archive.len(),
            frontier_margin: adversary.frontier_margin,
            anchor_health,
            calibration_honest,
            champion_id: champion.as_ref().map(|c| c.theory.id.clone()),
            champion_fitness: champion.as_ref().map(|c| c.assessment.final_fitness),
            champion_pressured_fitness: champion
                .as_ref()
                .map(|c| adversary.pressured_fitness(&c.assessment)),
            champion_credible: champion.is_some(),
        };
        observer(&report);
    }

    let qd_score = archive.values().map(|c| c.assessment.final_fitness).sum();
    // The final champion must clear the final frontier (None ⇒ the bar out-ran the population).
    let champion = pressured_champion(&archive, adversary.frontier_margin);
    EvolutionResult {
        archive,
        qd_score,
        champion,
        generations,
    }
}

#[cfg(test)]
mod tests {
    use super::super::mutation::{flip_to_quintic_decoy, inject_free_parameter};
    use super::super::Theory;
    use super::*;
    use crate::cosmology::BackgroundForwardModel;
    use crate::types::ObservableRecord;

    fn desi() -> Vec<ObservableRecord> {
        vec![
            ObservableRecord {
                observable_id: "dm_over_rd@0.510".into(),
                kind: "bao".into(),
                value: 13.62,
                uncertainty: 0.25,
                unit: "dimensionless".into(),
                source: None,
            },
            ObservableRecord {
                observable_id: "dh_over_rd@0.510".into(),
                kind: "bao".into(),
                value: 20.98,
                uncertainty: 0.61,
                unit: "dimensionless".into(),
                source: None,
            },
            ObservableRecord {
                observable_id: "bbn_yp".into(),
                kind: "bbn".into(),
                value: 0.2453,
                uncertainty: 0.0034,
                unit: "dimensionless".into(),
                source: None,
            },
        ]
    }

    #[test]
    fn evolution_yields_a_credible_champion() {
        let seeds = vec![Theory::baseline_lcdm()];
        let r = evolve(&seeds, &desi(), &BackgroundForwardModel, 0.0, 40, 16, 1234);
        let champ = r.champion.expect("a credible champion should emerge");
        assert!(champ.assessment.is_credible());
        assert!(champ.assessment.final_fitness > 0.0);
        assert!(r.qd_score > 0.0);
        assert!(r.archive.len() >= 1);
    }

    #[test]
    fn evolution_is_deterministic_in_the_seed() {
        let seeds = vec![Theory::baseline_lcdm()];
        let a = evolve(&seeds, &desi(), &BackgroundForwardModel, 0.0, 20, 12, 99);
        let b = evolve(&seeds, &desi(), &BackgroundForwardModel, 0.0, 20, 12, 99);
        assert_eq!(a.archive.len(), b.archive.len());
        assert_eq!(
            a.champion.map(|c| c.theory.id),
            b.champion.map(|c| c.theory.id)
        );
        assert!((a.qd_score - b.qd_score).abs() < 1e-12);
    }

    #[test]
    fn structurally_broken_seeds_never_become_champions() {
        // Seed the population with two hard decoys and the baseline; only the baseline lineage
        // can win — a vetoed theory has final_fitness 0 and is never credible.
        let base = Theory::baseline_lcdm();
        let seeds = vec![
            base.clone(),
            flip_to_quintic_decoy(&base),
            inject_free_parameter(&base, "f_ede", 0.07),
        ];
        let r = evolve(&seeds, &desi(), &BackgroundForwardModel, 0.0, 30, 16, 7);
        let champ = r.champion.expect("baseline lineage should win");
        assert!(champ.assessment.is_credible());
        assert!(!champ.theory.id.contains("decoy"));
        assert!(!champ.theory.id.contains("graybox"));
    }

    #[test]
    fn adversarial_run_observes_each_generation_and_stays_honest() {
        let seeds = vec![Theory::baseline_lcdm()];
        let mut reports = Vec::new();
        let r = evolve_run(
            &seeds,
            &desi(),
            &BackgroundForwardModel,
            0.0,
            40,
            16,
            1234,
            |gr| reports.push(gr.clone()),
        );
        // The observer fired once per generation.
        assert_eq!(reports.len(), 40);
        // The honesty loop held throughout: the anchor/decoy set never miscalibrated, and the
        // Survive anchors were never driven to zero pressured fitness (rollback protected them).
        assert!(
            reports.iter().all(|g| g.calibration_honest),
            "calibration broke"
        );
        assert!(
            reports.iter().all(|g| g.anchor_health > 0.0),
            "anchors killed"
        );
        // The adversary actually escalated the frontier at some point (it is not inert).
        assert!(
            reports.iter().any(|g| g.frontier_margin > 0.0),
            "frontier never escalated"
        );
        // A credible champion emerged.
        let champ = r.champion.expect("credible champion");
        assert!(champ.assessment.is_credible());
        assert_eq!(reports.last().unwrap().generation, 40);
    }

    #[test]
    fn the_frontier_culls_candidates_below_it_and_can_out_run_the_population() {
        // The adversary's margin is now a real survival threshold (not the old constant offset
        // that left the ranking untouched). Below the champion's fitness it survives; above it the
        // frontier out-runs the whole population and there is no eligible champion.
        let base = Theory::baseline_lcdm();
        let a = assess(&base, &desi(), &BackgroundForwardModel, 0.0);
        let f = a.final_fitness;
        assert!(f > 0.0);
        let mut archive = BTreeMap::new();
        insert(&mut archive, base.clone(), a);
        assert!(
            pressured_champion(&archive, f - 0.01).is_some(),
            "champion below the frontier should survive"
        );
        assert!(
            pressured_champion(&archive, f + 0.01).is_none(),
            "no candidate clears a frontier above the best fitness"
        );
    }

    #[test]
    fn reported_champion_actually_clears_the_final_frontier() {
        // End-to-end: the champion evolve_run returns must beat the final adversarial bar.
        let seeds = vec![Theory::baseline_lcdm()];
        let mut last = None;
        let r = evolve_run(
            &seeds,
            &desi(),
            &BackgroundForwardModel,
            0.0,
            60,
            16,
            1234,
            |gr| last = Some(gr.clone()),
        );
        let champ = r.champion.expect("a champion that clears the frontier");
        let final_margin = last.unwrap().frontier_margin;
        assert!(
            champ.assessment.final_fitness > final_margin,
            "champion fitness {} must clear final frontier {}",
            champ.assessment.final_fitness,
            final_margin
        );
    }

    #[test]
    fn evolve_run_is_deterministic_in_the_seed() {
        let seeds = vec![Theory::baseline_lcdm()];
        let a = evolve_run(
            &seeds,
            &desi(),
            &BackgroundForwardModel,
            0.0,
            20,
            12,
            99,
            |_| {},
        );
        let b = evolve_run(
            &seeds,
            &desi(),
            &BackgroundForwardModel,
            0.0,
            20,
            12,
            99,
            |_| {},
        );
        assert_eq!(
            a.champion.map(|c| c.theory.id),
            b.champion.map(|c| c.theory.id)
        );
        assert!((a.qd_score - b.qd_score).abs() < 1e-12);
    }

    #[test]
    fn more_generations_do_not_lose_quality() {
        // QD score is monotone: a longer run never has a lower archive quality than a short one
        // from the same seed prefix (elitism preserves cell bests).
        let seeds = vec![Theory::baseline_lcdm()];
        let short = evolve(&seeds, &desi(), &BackgroundForwardModel, 0.0, 5, 12, 55);
        let long = evolve(&seeds, &desi(), &BackgroundForwardModel, 0.0, 50, 12, 55);
        assert!(long.qd_score >= short.qd_score - 1e-9);
        assert!(long.archive.len() >= short.archive.len());
    }
}
