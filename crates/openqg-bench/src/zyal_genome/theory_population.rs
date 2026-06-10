//! V4 M5b: the real theory-population evolution engine.
//!
//! V3's "evolution" scored *stage executions* with hash-jitter and re-promoted the gen-1 root
//! forever (`mutation_ops:["promote_lineage"]`, live calls only in g0001). This module replaces that
//! with a genuine search over real [`Theory`] objects: a seeded population is evolved with typed
//! `mutate`/`recombine` (recombination gated by `recombination_compatible`), every child is scored
//! veto-first by [`score_candidate`] (the real physics + the M3 rubric), and a per-generation
//! [`GenerationProgress`] proves new claim-fingerprints are actually produced. Promotion enforces a
//! **non-root lineage** after gen 1 (a champion must descend from a real parent and differ from it),
//! so the V3 gen-1-only collapse becomes structurally impossible and detectable
//! ([`population_progress_ok`]).
//!
//! Pure-parameter children carry an empty claim graph, so they earn DataFit + parsimony credit but
//! no derivation/unification credit — that is the honest V4 behaviour: rigor and unification are
//! earned only by *derivations*, which the LLM proposer attaches in a later step. The engine here is
//! deterministic (seeded `Rng`, deterministic forward model), so a run replays without the LLM.

use std::collections::BTreeSet;

use openqg_core::theory::{
    claim_fingerprint, mutate, recombination_compatible, recombine, ClaimGraph, MapEvidenceStore,
    Rng, ScorecardV4, Theory, UnificationClaim,
};
use openqg_core::ObservableRecord;

use super::physics_score::{final_score_unit, score_candidate};
use super::proposer::{score_proposal, Proposer};

const EVIDENCE_SCHEMA: &str = "genome-candidate.v1";

/// The role an island plays, which selects its reproduction operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IslandRole {
    /// Fresh exploration: mutate from a random survivor at a high rate.
    Explore,
    /// Hill-climb: low-rate mutation of the island's best survivor.
    Exploit,
    /// Diversity: proof-gated recombination across the two best survivors.
    Novelty,
}

impl IslandRole {
    fn name(self) -> &'static str {
        match self {
            IslandRole::Explore => "explore",
            IslandRole::Exploit => "exploit",
            IslandRole::Novelty => "novelty",
        }
    }
    fn mutation_rate(self) -> f64 {
        match self {
            IslandRole::Explore => 0.9,
            IslandRole::Exploit => 0.3,
            IslandRole::Novelty => 0.6,
        }
    }
    /// The default three-island roster.
    fn roster() -> [IslandRole; 3] {
        [
            IslandRole::Explore,
            IslandRole::Exploit,
            IslandRole::Novelty,
        ]
    }
}

/// One scored member of the population.
#[derive(Debug, Clone)]
pub(crate) struct Individual {
    pub id: String,
    pub generation: usize,
    pub island: &'static str,
    pub parent_ids: Vec<String>,
    pub theory: Theory,
    pub fingerprint: String,
    pub final_score: f64,
    pub disqualified: bool,
    /// The full veto-first scorecard for this individual (bare-theory or proposal-derived).
    pub scorecard: ScorecardV4,
}

/// Per-generation progress evidence — the metric that makes the gen-1-only collapse detectable.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GenerationProgress {
    pub generation: usize,
    /// Claim-fingerprints in this generation not seen in any prior generation.
    pub new_fingerprints: usize,
    /// Fingerprints already seen before.
    pub reused_fingerprints: usize,
    /// Distinct fingerprints among the promoted champions this generation.
    pub distinct_lineages: usize,
    /// True if every promoted champion is a re-promotion of an existing lineage (the V3 pathology).
    pub promote_lineage_only: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct EvolveConfig {
    pub population_size: usize,
    pub max_generations: usize,
    pub seed: u64,
}

impl Default for EvolveConfig {
    fn default() -> Self {
        Self {
            population_size: 9,
            max_generations: 8,
            seed: 0x05eed_5151,
        }
    }
}

/// V5 observability: one LLM call/repair attempt as it happened — success, parse failure, process
/// failure, or oracle kill. Streamed to `proposal-attempts.jsonl`; **nothing is ever swallowed**.
#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct ProposalAttemptRecord {
    pub record_kind: &'static str, // "proposal_attempt"
    pub generation: usize,
    pub source: String,              // "jekko" | "jailgun" | "fixture" | "engine"
    pub sample_index: usize,         // 0..K for best-of-K sampling
    pub attempt_index: usize,        // 0 = initial call, 1.. = repairs
    pub repair_kind: Option<String>, // None | "parse" | "oracle"
    pub outcome: String, // "ok" | "parse_error" | "llm_error" | "killed" | "engine_error"
    pub error: Option<String>,
    pub kill_reasons: Vec<String>,
    pub total: Option<f64>,
    pub raw_sha256: String,
    pub raw_len: usize,
    pub elapsed_seconds: f64,
    pub winner: bool,
}

impl ProposalAttemptRecord {
    /// A propose()-level failure recorded by the engine itself (no per-call instrumentation).
    pub(crate) fn engine_failure(generation: usize, error: &anyhow::Error) -> Self {
        Self {
            record_kind: "proposal_attempt",
            generation,
            source: "engine".into(),
            sample_index: 0,
            attempt_index: 0,
            repair_kind: None,
            outcome: "engine_error".into(),
            error: Some(format!("{error:#}")),
            kill_reasons: Vec::new(),
            total: None,
            raw_sha256: String::new(),
            raw_len: 0,
            elapsed_seconds: 0.0,
            winner: false,
        }
    }
}

/// One proposal (live LLM or fixture) as it entered the run — the audit + replay record. Storing the
/// full [`super::proposer::ProposalDoc`] (`doc`) makes the run replayable without re-calling the LLM;
/// `proposal_sha256` content-pins it.
#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct LiveProposalRecord {
    pub generation: usize,
    pub theory_id: String,
    pub proposal_sha256: String,
    pub disqualified: bool,
    pub total: f64,
    pub distinct_from_baseline: bool,
    pub doc: serde_json::Value,
}

/// The outcome of a full population run.
#[derive(Debug, Clone)]
pub(crate) struct EvolutionRun {
    pub progress: Vec<GenerationProgress>,
    pub champions: Vec<Individual>,
    pub best: Option<Individual>,
    /// Every proposal that entered the run, in order — the content-pinned audit + replay trail.
    pub live_proposals: Vec<LiveProposalRecord>,
}

/// Score a bare theory candidate (no attached derivations) through the real physics + rubric.
pub(crate) fn score_theory(
    theory: &Theory,
    observables: &[ObservableRecord],
    blocks: &[openqg_core::scoring::CovarianceBlock],
    baseline_ll: f64,
) -> ScorecardV4 {
    let empty_cg = ClaimGraph { claims: vec![] };
    let empty_store = MapEvidenceStore::default();
    let empty_uni = UnificationClaim { shared: vec![] };
    score_candidate(
        theory,
        observables,
        blocks,
        baseline_ll,
        &empty_cg,
        &[],
        &empty_uni,
        &empty_store,
        EVIDENCE_SCHEMA,
    )
}

/// Build an individual from an already-computed scorecard (used for proposal candidates that carry
/// a full ClaimGraph + obligations).
fn individual_from_scorecard(
    id: String,
    generation: usize,
    island: &'static str,
    parent_ids: Vec<String>,
    theory: Theory,
    scorecard: ScorecardV4,
) -> Individual {
    Individual {
        fingerprint: claim_fingerprint(&theory),
        final_score: final_score_unit(&scorecard),
        disqualified: scorecard.disqualified,
        scorecard,
        id,
        generation,
        island,
        parent_ids,
        theory,
    }
}

fn individual(
    id: String,
    generation: usize,
    island: &'static str,
    parent_ids: Vec<String>,
    theory: Theory,
    observables: &[ObservableRecord],
    blocks: &[openqg_core::scoring::CovarianceBlock],
    baseline_ll: f64,
) -> Individual {
    let sc = score_theory(&theory, observables, blocks, baseline_ll);
    individual_from_scorecard(id, generation, island, parent_ids, theory, sc)
}

/// Produce a proposal candidate from the proposer (if present): the LLM/fixture emits a derivation-
/// rich [`super::proposer::ProposalDoc`] which is adjudicated veto-first. Returns `None` if there is
/// no proposer or the proposal cannot be produced. This is how a champion earns derivation +
/// unification credit (the "LLM proposes, oracle disposes" path).
fn proposal_individual(
    proposer: Option<&dyn Proposer>,
    generation: usize,
    observables: &[ObservableRecord],
    blocks: &[openqg_core::scoring::CovarianceBlock],
    baseline_ll: f64,
    sink: &mut dyn super::ledger_sink::LedgerSink,
) -> Option<(Individual, LiveProposalRecord)> {
    let proposer = proposer?;
    let result = proposer.propose();
    // V5: drain the per-call attempt audit chain and stream it — every LLM call, repair, and
    // failure is on record regardless of the outcome.
    for mut rec in proposer.drain_attempts() {
        rec.generation = generation;
        sink.attempt(&rec);
    }
    let doc = match result {
        Ok(doc) => doc,
        // A budget off-generation is BY DESIGN — silent, not a failure.
        Err(e) if e.downcast_ref::<super::proposer::ProposeSkip>().is_some() => return None,
        // A real failure is NEVER swallowed: ledger it, then continue deterministically.
        Err(e) => {
            sink.attempt(&ProposalAttemptRecord::engine_failure(generation, &e));
            return None;
        }
    };
    let sc = score_proposal(&doc, observables, blocks, baseline_ll);
    // Audit/replay record: the full proposal + its content hash, before we drop the doc.
    let canonical = serde_json::to_string(&doc).unwrap_or_default();
    let record = LiveProposalRecord {
        generation,
        theory_id: doc.theory.id.clone(),
        proposal_sha256: openqg_core::sha256_digest(canonical.as_bytes()),
        disqualified: sc.disqualified,
        total: sc.total,
        distinct_from_baseline: sc.distinct_from_baseline,
        doc: serde_json::to_value(&doc).unwrap_or(serde_json::Value::Null),
    };
    let theory = doc.theory.clone();
    let parents = if generation == 1 {
        vec![]
    } else {
        vec![format!("proposer-g{}", generation - 1)]
    };
    let ind = individual_from_scorecard(
        format!("g{generation:04}-proposer"),
        generation,
        "proposer",
        parents,
        theory,
        sc,
    );
    Some((ind, record))
}

/// Seed generation 1 from the ΛCDM baseline plus low-rate mutations (so the initial pool is diverse
/// but anchored on a physically-sane theory).
fn seed_population(
    config: &EvolveConfig,
    observables: &[ObservableRecord],
    blocks: &[openqg_core::scoring::CovarianceBlock],
    baseline_ll: f64,
    rng: &mut Rng,
) -> Vec<Individual> {
    let mut pop = Vec::with_capacity(config.population_size);
    let base = Theory::baseline_lcdm();
    pop.push(individual(
        "g0001-seed".into(),
        1,
        "explore",
        vec![],
        base.clone(),
        observables,
        blocks,
        baseline_ll,
    ));
    for i in 1..config.population_size {
        let child = mutate(&base, rng, 0.5);
        pop.push(individual(
            format!("g0001-seed{i}"),
            1,
            "explore",
            vec![],
            child,
            observables,
            blocks,
            baseline_ll,
        ));
    }
    pop
}

/// Pick the best non-disqualified members as breeding survivors (falls back to all if none pass).
fn survivors(pop: &[Individual]) -> Vec<&Individual> {
    let mut alive: Vec<&Individual> = pop.iter().filter(|i| !i.disqualified).collect();
    if alive.is_empty() {
        alive = pop.iter().collect();
    }
    alive.sort_by(|a, b| {
        b.final_score
            .partial_cmp(&a.final_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    alive
}

/// Produce one child for `island` from the previous generation's survivors.
fn breed_child(
    island: IslandRole,
    survivors: &[&Individual],
    generation: usize,
    idx: usize,
    rng: &mut Rng,
    observables: &[ObservableRecord],
    blocks: &[openqg_core::scoring::CovarianceBlock],
    baseline_ll: f64,
) -> Individual {
    let id = format!("g{generation:04}-{}-{idx}", island.name());
    match island {
        IslandRole::Explore => {
            // mutate a random survivor at a high rate.
            let parent = &survivors[(rng.next_u64() as usize) % survivors.len()];
            let theory = mutate(&parent.theory, rng, island.mutation_rate());
            individual(
                id,
                generation,
                island.name(),
                vec![parent.id.clone()],
                theory,
                observables,
                blocks,
                baseline_ll,
            )
        }
        IslandRole::Exploit => {
            // low-rate mutation of the best survivor (hill-climb).
            let parent = survivors[0];
            let theory = mutate(&parent.theory, rng, island.mutation_rate());
            individual(
                id,
                generation,
                island.name(),
                vec![parent.id.clone()],
                theory,
                observables,
                blocks,
                baseline_ll,
            )
        }
        IslandRole::Novelty => {
            // proof-gated recombination of the two best survivors; fall back to mutation if the
            // recombination is not dimensionally/physically compatible.
            let a = survivors[0];
            let b = if survivors.len() > 1 {
                survivors[1]
            } else {
                survivors[0]
            };
            if a.id != b.id && recombination_compatible(&a.theory, &b.theory).compatible {
                let theory = recombine(&a.theory, &b.theory, rng);
                individual(
                    id,
                    generation,
                    island.name(),
                    vec![a.id.clone(), b.id.clone()],
                    theory,
                    observables,
                    blocks,
                    baseline_ll,
                )
            } else {
                let theory = mutate(&a.theory, rng, island.mutation_rate());
                individual(
                    id,
                    generation,
                    island.name(),
                    vec![a.id.clone()],
                    theory,
                    observables,
                    blocks,
                    baseline_ll,
                )
            }
        }
    }
}

/// Choose the promoted champion for a generation: the best non-disqualified individual that, after
/// generation 1, descends from a real parent and differs from every parent (the non-root-lineage
/// gate). Returns `None` if no qualifying champion exists (a stagnant generation).
fn promote_champion<'a>(
    pop: &'a [Individual],
    generation: usize,
    parent_fingerprints: &std::collections::BTreeMap<String, String>,
) -> Option<&'a Individual> {
    let mut ranked: Vec<&Individual> = pop.iter().filter(|i| !i.disqualified).collect();
    ranked.sort_by(|a, b| {
        b.final_score
            .partial_cmp(&a.final_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    ranked.into_iter().find(|c| {
        if generation == 1 {
            return true;
        }
        // non-root lineage: must have a parent, and differ in fingerprint from each parent.
        !c.parent_ids.is_empty()
            && c.parent_ids.iter().all(|p| {
                parent_fingerprints
                    .get(p)
                    .map(|fp| fp != &c.fingerprint)
                    .unwrap_or(true)
            })
    })
}

/// Run the full population evolution. Deterministic given `config` + `observables` (+ a
/// deterministic proposer). Pass `None` for pure parameter evolution; pass a [`Proposer`] to inject
/// a derivation-rich candidate each generation (the LLM-proposes path).
pub(crate) fn evolve_population(
    config: &EvolveConfig,
    observables: &[ObservableRecord],
    blocks: &[openqg_core::scoring::CovarianceBlock],
    proposer: Option<&dyn Proposer>,
    sink: &mut dyn super::ledger_sink::LedgerSink,
) -> EvolutionRun {
    let baseline_ll = super::physics_score::baseline_log_likelihood_cov(observables, blocks);
    let mut rng = Rng::new(config.seed);
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut progress = Vec::new();
    let mut champions = Vec::new();
    let mut live_proposals: Vec<LiveProposalRecord> = Vec::new();

    // Generation 1 — seed (+ optional proposal candidate).
    let mut pop = seed_population(config, observables, blocks, baseline_ll, &mut rng);
    if let Some((ind, rec)) =
        proposal_individual(proposer, 1, observables, blocks, baseline_ll, sink)
    {
        pop.push(ind);
        sink.proposal(&rec);
        live_proposals.push(rec);
    }
    let mut fp_by_id: std::collections::BTreeMap<String, String> = pop
        .iter()
        .map(|i| (i.id.clone(), i.fingerprint.clone()))
        .collect();
    record_progress(&pop, 1, &mut seen, &mut progress, &mut champions, &fp_by_id);
    if let Some(g) = progress.last() {
        sink.progress(g);
    }
    let mut best_so_far: Option<Individual> = champions.last().cloned();
    sink.champion_checkpoint(1, best_so_far.as_ref());

    // Generations 2..N — genuine reproduction.
    for generation in 2..=config.max_generations {
        let survs = survivors(&pop);
        let roster = IslandRole::roster();
        let mut next: Vec<Individual> = Vec::with_capacity(config.population_size);
        for idx in 0..config.population_size {
            let island = roster[idx % roster.len()];
            next.push(breed_child(
                island,
                &survs,
                generation,
                idx,
                &mut rng,
                observables,
                blocks,
                baseline_ll,
            ));
        }
        if let Some((ind, rec)) =
            proposal_individual(proposer, generation, observables, blocks, baseline_ll, sink)
        {
            next.push(ind);
            sink.proposal(&rec);
            live_proposals.push(rec);
        }
        fp_by_id = next
            .iter()
            .map(|i| (i.id.clone(), i.fingerprint.clone()))
            .collect();
        // include parents' fingerprints for the non-root gate
        for i in &pop {
            fp_by_id
                .entry(i.id.clone())
                .or_insert_with(|| i.fingerprint.clone());
        }
        record_progress(
            &next,
            generation,
            &mut seen,
            &mut progress,
            &mut champions,
            &fp_by_id,
        );
        if let Some(g) = progress.last() {
            sink.progress(g);
        }
        if let Some(c) = champions.last() {
            let better = best_so_far
                .as_ref()
                .map(|b| !c.disqualified && c.final_score > b.final_score)
                .unwrap_or(!c.disqualified);
            if better {
                best_so_far = Some(c.clone());
            }
        }
        sink.champion_checkpoint(generation, best_so_far.as_ref());
        pop = next;
    }

    let best = champions
        .iter()
        .filter(|c| !c.disqualified)
        .max_by(|a, b| {
            a.final_score
                .partial_cmp(&b.final_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .cloned();

    EvolutionRun {
        progress,
        champions,
        best,
        live_proposals,
    }
}

fn record_progress(
    pop: &[Individual],
    generation: usize,
    seen: &mut BTreeSet<String>,
    progress: &mut Vec<GenerationProgress>,
    champions: &mut Vec<Individual>,
    fp_by_id: &std::collections::BTreeMap<String, String>,
) {
    let mut new_fp = 0;
    let mut reused = 0;
    for i in pop {
        if seen.contains(&i.fingerprint) {
            reused += 1;
        } else {
            new_fp += 1;
        }
    }
    for i in pop {
        seen.insert(i.fingerprint.clone());
    }
    let champion = promote_champion(pop, generation, fp_by_id).cloned();
    let promote_lineage_only = champion.is_none();
    let distinct_lineages = champion.as_ref().map(|_| 1).unwrap_or(0);
    if let Some(c) = champion {
        champions.push(c);
    }
    progress.push(GenerationProgress {
        generation,
        new_fingerprints: new_fp,
        reused_fingerprints: reused,
        distinct_lineages,
        promote_lineage_only,
    });
}

/// The V4 progress gate (also a V3-regression detector): every non-seed generation must produce at
/// least one NEW claim-fingerprint and promote a genuine (non-root) champion. A V3-style run — where
/// later generations only re-promote the gen-1 lineage with zero new fingerprints — FAILS this.
pub(crate) fn population_progress_ok(progress: &[GenerationProgress]) -> bool {
    progress
        .iter()
        .filter(|g| g.generation > 1)
        .all(|g| g.new_fingerprints >= 1 && !g.promote_lineage_only)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs() -> Vec<ObservableRecord> {
        [
            "bao_dv_z038",
            "bao_dv_z051",
            "fsigma8_z038",
            "fsigma8_z051",
            "h0_riess",
        ]
        .iter()
        .enumerate()
        .map(|(i, id)| ObservableRecord {
            observable_id: (*id).into(),
            kind: "cosmology".into(),
            value: 1.0 + i as f64 * 0.1,
            uncertainty: 0.05,
            unit: "dimensionless".into(),
            source: None,
        })
        .collect()
    }

    #[test]
    fn every_generation_produces_new_fingerprints_and_a_non_root_champion() {
        let cfg = EvolveConfig {
            population_size: 9,
            max_generations: 6,
            seed: 42,
        };
        let run = evolve_population(
            &cfg,
            &obs(),
            &[],
            None,
            &mut super::super::ledger_sink::NullSink,
        );
        assert_eq!(run.progress.len(), 6);
        // The core anti-collapse guarantee:
        for g in run.progress.iter().filter(|g| g.generation > 1) {
            assert!(
                g.new_fingerprints >= 1,
                "gen {} produced no new fingerprints",
                g.generation
            );
            assert!(
                !g.promote_lineage_only,
                "gen {} only re-promoted a lineage",
                g.generation
            );
        }
        assert!(population_progress_ok(&run.progress));
        // Champions after gen 1 must be non-root (have parents).
        for c in run.champions.iter().filter(|c| c.generation > 1) {
            assert!(
                !c.parent_ids.is_empty(),
                "champion {} is root after gen 1",
                c.id
            );
        }
    }

    #[test]
    fn evolution_is_deterministic() {
        let cfg = EvolveConfig {
            population_size: 6,
            max_generations: 4,
            seed: 7,
        };
        let a = evolve_population(
            &cfg,
            &obs(),
            &[],
            None,
            &mut super::super::ledger_sink::NullSink,
        );
        let b = evolve_population(
            &cfg,
            &obs(),
            &[],
            None,
            &mut super::super::ledger_sink::NullSink,
        );
        assert_eq!(a.progress, b.progress);
        assert_eq!(
            a.best.map(|x| x.fingerprint),
            b.best.map(|x| x.fingerprint),
            "same seed must give the same champion"
        );
    }

    #[test]
    fn the_progress_gate_rejects_a_v3_style_stagnant_run() {
        // Synthetic V3 pattern: gen 1 has new work, later gens produce nothing new and only
        // re-promote the lineage.
        let v3 = vec![
            GenerationProgress {
                generation: 1,
                new_fingerprints: 7,
                reused_fingerprints: 0,
                distinct_lineages: 1,
                promote_lineage_only: false,
            },
            GenerationProgress {
                generation: 2,
                new_fingerprints: 0,
                reused_fingerprints: 9,
                distinct_lineages: 0,
                promote_lineage_only: true,
            },
            GenerationProgress {
                generation: 3,
                new_fingerprints: 0,
                reused_fingerprints: 9,
                distinct_lineages: 0,
                promote_lineage_only: true,
            },
        ];
        assert!(
            !population_progress_ok(&v3),
            "a gen-1-only collapse must FAIL the progress gate"
        );
    }

    #[test]
    fn a_proposer_run_yields_a_derivation_rich_champion() {
        use super::super::proposer::FixtureProposer;
        let cfg = EvolveConfig {
            population_size: 9,
            max_generations: 5,
            seed: 99,
        };
        // Without a proposer the champion is a bare parameter-fit: derivation_rigor == 0.
        let bare = evolve_population(
            &cfg,
            &obs(),
            &[],
            None,
            &mut super::super::ledger_sink::NullSink,
        )
        .best
        .unwrap();
        let bare_rigor = bare
            .scorecard
            .components
            .iter()
            .find(|c| c.name == "derivation_rigor")
            .map(|c| c.points)
            .unwrap_or(0.0);
        assert_eq!(bare_rigor, 0.0, "pure evolution earns no derivation rigor");

        // With the proposer, the champion carries verified derivations + unification.
        let rich = evolve_population(
            &cfg,
            &obs(),
            &[],
            Some(&FixtureProposer),
            &mut super::super::ledger_sink::NullSink,
        )
        .best
        .expect("a champion");
        let pts = |n: &str| {
            rich.scorecard
                .components
                .iter()
                .find(|c| c.name == n)
                .map(|c| c.points)
                .unwrap_or(0.0)
        };
        assert!(
            pts("derivation_rigor") > 0.0,
            "proposer champion must earn derivation rigor"
        );
        assert!(
            pts("unification") > 0.0,
            "proposer champion must earn unification"
        );
        assert!(
            rich.final_score > bare.final_score,
            "derivations should raise the score"
        );
    }
}
