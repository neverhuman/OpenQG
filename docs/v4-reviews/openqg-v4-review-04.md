# 04. Evolution & recombination — MAP-Elites, islands, novelty, mutation, stage swapping

Batch tab: 1.

The evolution layer is the part most at risk of looking sophisticated while doing little search. The latest run pattern — generation 1 producing live work while later generations re-promote or repair the same lineage — is consistent with an evolutionary loop that lacks enforced novelty, population pressure, and lineage-valid recombination.

## Evidence from live artifacts

The strongest diagnostic is `run-data/review-artifact-14-openqg-live-batch-3-live-g0001-09-selection-mutation-hard_stage_repair.json` and related hybrid selection artifacts. The `09-selection-mutation` artifact selects a single survivor with `lineage_id=g0001-09-selection-mutation`, `parents=[]`, `survivor_class=repair_anchor`, `plateau_state=not_resolved_in_packet`, and `source_diversity=memory_ref_only`. The mutation ops are conservative: `source_diversity_injection`, `island_gain_rebalance`, and `historical_return_dampening`. Those are valid repairs, but they do not prove recombination of independent theory genes.

This matters because the project philosophy depends on recombining improved components. If each generation mostly repairs stage contracts and preserves a root anchor, the system is not discovering theories; it is hardening its own workflow.

## Likely code hotspots

The relevant surfaces are `crates/openqg-core/src/theory/{evolve.rs,mutation.rs,pareto.rs,league.rs}` and `crates/openqg-bench/src/zyal_genome/{selection.rs,novelty.rs,candidates.rs,records.rs,resume.rs,run_variant.rs,run_summary.rs,stages.rs}`. The defect may not be a single bug. It is probably a missing invariant: the selection module can accept a survivor without proving population diversity, parentage, novelty return, and cell movement.

## MAP-Elites failure modes

1. **Descriptor gaming.** If cells are keyed by prose descriptors, candidates can occupy cells by changing words. Use claim-graph descriptors: sector coverage, verified limit count, parameter ledger, derivation-obligation depth, anchor survival, and holdout status.

2. **Empty-cell inflation.** MAP-Elites rewards filling cells, but an LLM can produce many weak variants that satisfy cell labels. Require cell occupancy to include at least one materialized evidence hash and one typed certificate result.

3. **Root-lineage replay.** A resume/retry mechanism can treat a root survivor as safe and repeatedly promote it. Add a promotion-time veto if a generation has no non-root parentage or no accepted mutation with a changed claim fingerprint.

4. **Island isolation theater.** Islands are useful only if they have different operators, data exposure, and objective pressures. If all islands share the same prompts and judge, they become copies.

5. **Stage swapping without semantic recombination.** Swapping a stage prompt variant is not equivalent to recombining theory components. Track stage lineage separately from theory lineage.

## Fix the g0001 plateau

Add a `PopulationProgressAudit` in `crates/openqg-bench/src/zyal_genome/selection.rs`:

- `generation_id`
- `candidate_count`
- `unique_parent_count`
- `non_root_lineage_count`
- `accepted_mutation_count`
- `claim_fingerprint_delta_count`
- `map_elites_cells_entered`
- `island_gain_by_island`
- `promotion_lineage_depth`

`quality_gate.rs` should reject promotion when `non_root_lineage_count < min_non_root_lineages`, when `claim_fingerprint_delta_count == 0`, or when `plateau_state` is unresolved after the first generation. `run_summary.rs` should emit the audit in `run-data/*-run-summary.json`.

## Recombination spec

In `crates/openqg-core/src/theory/mutation.rs`, distinguish mutation operators:

- `StageContractMutation`: prompt/schema/routing changes.
- `TheoryClaimMutation`: changes an assumption/equation/limit map.
- `EvidenceMutation`: changes dataset/cut/likelihood.
- `AdversaryMutation`: adds a falsifier, decoy, or anchor.
- `Recombination`: merges two claim subgraphs with conflict checks.

In `evolve.rs`, recombination should require a compatibility proof from `crates/openqg-core/src/theory/unification.rs`: shared symbols, dimensions, constants, sectors, and limit maps must not conflict. `05-compatibility` should call the same Rust logic, not merely prompt-level checks.

## Diversity mechanisms

Use three island types:

- **Conservative islands:** repair known contenders and LCDM/GR/SM baselines.
- **Speculative islands:** explore new claim graphs with heavy novelty penalties if unverifiable.
- **Adversary islands:** evolve tests, not theories.

Only delayed external validation should award compute. A candidate’s proposer island should not judge itself. `zyal_judge.rs` and `zyal_robustness.rs` should run cross-island judging with blinded IDs.

The key v4 requirement is simple: a generation must prove it created new, evidence-bound, claim-level variation. Otherwise, call it replay hardening, not evolution.
