# 10. ZYAL stage-by-stage change list and runbook changes

Batch tab: 1.

This is the concrete ZYAL workflow change list for `ZYAL/stages/00-atlas` through `10-promotion` plus `ZYAL/runs/run-jailgun-only.zyal`.

## 00-atlas

Current live artifacts correctly record run context, stage registry, memory refs, and evidence boundaries. Strengthen `ZYAL/stages/00-atlas/prompt.md` and `artifacts.schema.json` to require `materialized_evidence_manifest`: path, sha256, byte length, jsonl count, and schema ID. Do not allow downstream stages to treat `required_evidence` as proof. Add a stage score penalty in `score.yml` for unresolved memory refs used as contents.

## 01-decompose-known

Add a baseline decomposition requirement. The stage must produce claim graphs for GR/LCDM/SM fragments and the top human contenders. Each baseline claim should include sector, known limits, empirical contacts, and open failures. This prevents generated theories from being scored against a weak or undefined comparator.

## 02-decompose-failed

The live artifacts identify root-cause labels, route-tier leakage, and lineage loss. Update `artifacts.schema.json` to require `root_cause_label`, `smallest_failing_assumption`, `evidence_refs`, `lineage`, and `repair_target`. Reject broad labels such as “macro failure.” Require a claim/stage distinction: `failure_kind = pipeline | theory | evidence | adversary`.

## 03-generate-genes

Require `gene_kind` and `claim_graph_delta`. For theory genes, require physical sector, mathematical object, parameter ledger entry, and at least one falsifier. For pipeline genes, require stage target and schema diff. Ban pure prose genes from entering selection.

## 04-repair-genes

The live artifacts require failure class and mutation op. Keep that, but make mutation ops typed: `insert_constraint`, `tighten_schema`, `add_obligation`, `split_claim`, `demote_claim`, `add_falsifier`, `repair_lineage`. Each op must include before/after and evidence refs. Allow exactly one primary mutation op unless a composite repair is declared.

## 05-compatibility

Fix the `top20_pct` versus `top20_pct_only` contract. `stage.yml` should consume a canonical route enum from Rust, not string aliases. Split outputs into `interface_compatibility` and `physical_compatibility`. Interface checks: fields, lineage, route, protected paths. Physical checks: shared symbols, dimensions, parameters, sector limits.

## 06-assemble-modules

Require module assembly to produce a unified `ClaimGraph`. Every module edge must be typed: depends-on, shares-parameter, shares-symmetry, derives-limit, contradicts, or unresolved. Add `unresolved_edges` as a hard promotion blocker unless explicitly scoped speculative.

## 07-macro-test

Add macro tests for T1/T2/T3 tiers: dimensional, known-limit, anchor/decoy, and public precision summaries. Macro tests should produce machine-readable `test_receipts`, not only judge prose. Run private holdouts only through Rust and emit blinded verdicts.

## 08-failure-slicing

The live artifact already points to missing `evidence_refs` and `failure_penalty`. Make both required. Each slice should include `claim_id`, `failed_obligation_id`, `severity`, `penalty`, `repair_hint`, and `falsifier`. This stage should be the bridge from failed tests to repair genes.

## 09-selection-mutation

Add population progress gates: unique parents, non-root lineages, accepted mutations, claim fingerprint deltas, MAP-Elites cell movement, and island gain. Do not select a lone `repair_anchor` for promotion after generation 1 unless the run is explicitly a replay/hardening run.

## 10-promotion

Promotion should emit a dossier: claim graph, evidence manifest, scorecard, vetoes, certificate coverage, judge calibration, anchor results, data tier use, lineage graph, and rollback hooks. Missing evidence contents, unresolved plateau, route-tier mismatch, or prose-only derivations are blockers.

## New stages

Add `11-baseline-league`: score the candidate against GR/LCDM/SM fragments and human contenders. Add `12-certificate-forge`: convert claims into derivation obligations and run cheap checks. Add `13-blind-holdout`: sealed robustness tests outside prompt-visible context.

## Runbook changes

In `ZYAL/runs/run-jailgun-only.zyal`, add preflight steps:

1. materialize evidence manifest;
2. canonicalize route tiers;
3. load public anchors only;
4. initialize population progress audit;
5. enforce private holdout exclusion from prompts.

Add post-stage gates after `05`, `07`, `09`, and `10`. If `05` fails route tier, stop. If `07` lacks receipts, stop. If `09` shows no non-root lineage, stop or mark replay-only. If `10` lacks certificate coverage, do not promote.
