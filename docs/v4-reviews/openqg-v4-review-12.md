# 12. v4 roadmap & milestones

Batch tab: 1.

The roadmap should sequence trust before ambition. OpenQG / ZYAL does not need to prove a unified theory in v4. It needs to prove that its pipeline can honestly identify, preserve, attack, and score candidate theory programs without fooling itself.

## Milestone 0 — Evidence closure

**Goal:** eliminate evidence-name laundering.

Implement materialized evidence manifests in `00-atlas`, `validate.rs`, and `quality_gate.rs`. Every cited evidence path must have sha256, byte length, jsonl count, schema ID, and claim/event binding. Update `run-data/*-run-summary.json` to include evidence audit results.

**Validation:** create fixtures where evidence paths are named but empty, missing, malformed, or inconsistent. Promotion must fail. The live artifacts already show this boundary; v4 should make it machine-enforced.

## Milestone 1 — Route and lineage hardening

**Goal:** fix `top20_pct` versus `top20_pct_only` and root-lineage replay.

Add canonical `RouteTier` enum in `route.rs`. Add `PopulationProgressAudit` in `selection.rs`. Fail promotion if route tiers mismatch or if non-root lineage count is zero in a supposed evolutionary run.

**Validation:** replay the current run pattern. A generation that only re-promotes `g0001` should be marked replay/hardening, not discovery. A hard-route alias mismatch should stop at `05-compatibility`.

## Milestone 2 — Claim graph and gene kind split

**Goal:** separate pipeline repair from physics search.

Add `ClaimGraph` to `proposal.rs` and `GeneKind::{Pipeline,Theory,Evidence,Adversary}` to ZYAL types and schema. Update `03-generate-genes`, `04-repair-genes`, `06-assemble-modules`, and `08-failure-slicing` to carry claim IDs and evidence refs.

**Validation:** a stage repair cannot be selected as a theory improvement unless it changes a theory claim fingerprint or unlocks a verified obligation.

## Milestone 3 — Certificate forge

**Goal:** prevent derivation theater.

Add `DerivationCertificateV4`, dimensional checks, limit checks, and verifier receipts. Wire into `evaluate.rs` and `vetoes.rs`. Start with cheap checks: dimensions, Newtonian/GR/LCDM limits, no-ghost/no-tachyon flags for modified gravity sectors.

**Validation:** seed fake derivation candidates that use plausible prose but fail units or limits. They must be vetoed. Seed LCDM/GR anchors should pass relevant baseline obligations.

## Milestone 4 — Judge calibration and blind anchors

**Goal:** make robustness-under-judge measurable.

Split anchors into public, private, and canary sets. Add blinded judge packets and judge scorecards. Add honesty rollback when later checks contradict earlier judge approvals.

**Validation:** decoys such as gray-box fudge, H0=100, omega runaway, and overfit echo must be rejected. Good anchors should not be rejected for style. Judge variance under paraphrase should be bounded.

## Milestone 5 — Data tier integration

**Goal:** connect candidates to discriminating evidence without overfitting.

Add `EvidenceTier` to `openqg-data`, dataset manifests, leakage policy, and tiered scoring. Integrate T1 invariant checks, T2 anchors, T3 public precision datasets, T4 sector-specific checks, and T5 sealed holdouts.

**Validation:** candidates cannot access T5 in prompts; public datasets must declare splits and covariance; scorecards must show “insufficient evidence” rather than invented precision.

## Milestone 6 — Human baseline league

**Goal:** fairly score top human unified-theory contenders.

Implement league entries for string/M-theory, loop quantum gravity, asymptotic safety, causal sets, and selected emergent/modified-gravity programs. Score by the same claim graph, certificate, evidence, and unification rubric. Do not force a single ranking when evidence is incomparable.

**Validation:** the league emits a matrix of strengths, gaps, vetoes, and uncertainty. A generated candidate must beat or complement baselines in a specific cell, not by rhetoric.

## Milestone 7 — v4 promotion dossier

**Goal:** produce an expert-review-worthy candidate dossier.

`10-promotion` emits exactly: claim graph, materialized evidence manifest, scorecard, veto table, certificate coverage, data-tier use, anchor/decoy results, judge calibration, lineage graph, MAP-Elites cell history, and rollback hooks.

**Trust threshold:** I would trust a v4 candidate enough for expert attention if it has no hard vetoes, passes materialized evidence checks, survives public/private decoys, has nontrivial certificates for its central claims, shows at least one discriminating prediction or failure mode, and is compared against human baselines under the same rubric. Anything less may still be useful research fuel, but it should not be promoted as a solid unified-theory candidate.
