//! Minimal deterministic MAP-Elites evolution loop over symbolic [`Theory`] candidates — the
//! clean reference engine that drives `mutate`/`recombine` → [`assess`] through a
//! quality-diversity archive. It demonstrates the whole rebuilt foundation end-to-end and is the
//! logic the live bench engine will adopt in place of float-vector genes.
//!
//! MAP-Elites keeps the *best* candidate in each behavioral cell (a numeric, name-independent
//! descriptor), so the population stays diverse and elites are never lost — the GR/ΛCDM baseline
//! holds its cell unless something genuinely beats it there, and structurally-broken candidates
//! (vetoed ⇒ `final_fitness = 0`) can never become champions.

use super::{assess, mutate, recombine, CandidateAssessment, Rng, Theory};
use crate::cosmology::{CosmologyParams, ForwardModel};
use crate::types::ObservableRecord;
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
