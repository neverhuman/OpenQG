# 05. Scoring & fitness, and a strict rubric for top human contenders

Batch tab: 1.

The scoring system must be redesigned around defensible scientific evidence, not local stage quality. The near-term goal is to surface a candidate worthy of expert review and to score mature human contenders under the same bar. That requires a rubric that can make “not enough evidence” a stable outcome without rewarding vagueness.

## Current scoring surfaces

The core scoring modules are `crates/openqg-core/src/scoring/{likelihood.rs,evidence.rs,covariance.rs,repo.rs,scorecard.rs}` and theory scoring/promotion surfaces in `crates/openqg-core/src/theory/{evaluate.rs,assessment.rs,vetoes.rs,robustness.rs,holdout.rs,league.rs}`. The bench side includes `crates/openqg-bench/src/{score.rs,theory_league.rs,zyal_judge.rs,zyal_robustness.rs}` plus `crates/openqg-bench/src/zyal_genome/{scoring.rs,quality_metrics.rs,quality_gate.rs,eval.rs}`. Documentation anchors include `docs/scoring.md` and `docs/theory-league.md`.

The current live artifacts are mostly stage repair quality checks: route tier, lineage, evidence refs, failure penalties, mutation ops, and compatibility. These are necessary preconditions, not physics fitness.

## Rubric proposal: 100-point score with veto-first semantics

Promotion should be veto-first, then score. A candidate with a hard veto receives no numeric rank except diagnostic subscores.

### Hard vetoes

- Unmaterialized evidence for any promoted claim.
- Hidden/post-hoc parameter use not declared in a complexity ledger.
- Known-limit failure without explicit scope demotion.
- Fake derivation: prose where a typed derivation obligation is required.
- Data leakage: training or prompt access to private anchors/holdouts.
- Judge conflict: proposer, repairer, and judge share unblinded context.

Implement these in `crates/openqg-core/src/theory/vetoes.rs` as typed `VetoKind`s.

### Numeric components

1. **Known-limit recovery (20).** GR weak-field, Newtonian limit, special relativity/local Lorentz invariance, QFT/locality where applicable, thermodynamic consistency. Checked through `certificate.rs` obligations.

2. **Empirical contact (15).** Fit/survival on datasets with transparent covariance and holdouts. Scored in `likelihood.rs`, `covariance.rs`, and `evidence.rs`, not by judges.

3. **Unification compression (15).** Fewer independent assumptions across sectors without hiding parameters. Requires `unification.rs` and a `ModelComplexityLedger`.

4. **Derivation quality (15).** Formal/symbolic/numeric/dimensional obligations passed. Partial credit only for typed, reproducible artifacts.

5. **Robustness-under-judge (10).** Survives blinded adversary, decoys, anchors, and perturbation tests in `adversary.rs` and `robustness.rs`.

6. **Predictive specificity (10).** Emits testable predictions or discriminating signatures, not just reinterpretations.

7. **Novelty over baselines (5).** Novel claim graph after canonicalization; no credit for vocabulary novelty.

8. **Simplicity/complexity penalty (5).** AIC/BIC/MDL-like but with explicit parameter and data-cut accounting.

9. **Reproducibility and receipts (5).** Sealed nondeterminism, hashes, manifests, and replayability through `validation/*`.

## Scoring human contenders

The top-5 human programs should be league entries, not special cases:

- **String/M-theory:** high mathematical richness and unification ambition; penalize empirical specificity and landscape/falsifiability unless a claim is scoped to a verified fragment.
- **Loop quantum gravity:** strong background-independent quantum geometry program; penalize incomplete matter unification and limited empirical discrimination.
- **Asymptotic safety:** strong UV-completion motivation and calculational program; penalize truncation dependence and unsettled Standard Model/gravity integration.
- **Causal sets:** strong discrete spacetime ontology and Lorentz-invariance aspirations; penalize derivation of continuum physics and particle phenomenology gaps.
- **Emergent/entropic/other quantum-gravity programs:** score only through explicit claim graphs and checked limits, not philosophical appeal.

Do not rank these globally unless the same evidence tiers are populated. `crates/openqg-bench/src/theory_league.rs` should emit a matrix of subscores and “insufficient evidence” cells.

## Engineering changes

Add `crates/openqg-core/src/scoring/rubric.rs` with `ScoreComponent`, `ScorecardV4`, `VetoedScore`, and `RubricVersion`. Add `crates/openqg-core/src/theory/complexity.rs` for `ModelComplexityLedger`. Extend `crates/openqg-bench/src/cli/score.rs` to support `--league human-baselines --rubric v4 --emit scorecard.json`. Update `docs/scoring.md` with veto-first semantics and uncertainty bands. Update `docs/theory-league.md` with the human-contender template.

## ε/AIC warning

A tiny likelihood improvement must not dominate the rubric. `evidence.rs` should return uncertainty intervals, and `scorecard.rs` should mark differences inside uncertainty as tied. Penalize every post-hoc dataset cut, fitted nuisance constant, and prompt-selected comparison baseline. A candidate theory worthy of expert attention should win by surviving hard checks, not by exploiting an epsilon in an under-specified likelihood.
