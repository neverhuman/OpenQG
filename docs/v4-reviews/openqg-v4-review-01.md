# 01. Executive critical review — v4 go/no-go

Batch tab: 1. Review scope: OpenQG / ZYAL v4, especially `crates/openqg-core`, `crates/openqg-bench`, `crates/openqg-data`, `ZYAL/stages/*`, `ZYAL/runs/run-jailgun-only.zyal`, docs, and `run-data/` artifacts named in the invocation.

## Verdict

**No-go for claiming a v4 candidate theory is ready for expert physics review. Conditional go for v4 as an engineering hardening milestone.** The system has the right high-level loop — decompose a theory into stage/gene artifacts, use LLM reasoning to improve pieces, recombine via evolutionary search, and evaluate with adversary/judge/veto machinery — but the current live outputs show the loop is not yet evidence-closed. The latest run artifacts repeatedly distinguish “declared required evidence” from “inspected evidence contents,” which is good epistemic hygiene, but also shows the present promotion path can advance on packet metadata rather than hard proof.

## What is strong

First, the archive already has the right durable Rust boundaries. The theory-facing modules in `crates/openqg-core/src/theory/{proposal.rs,certificate.rs,vetoes.rs,evaluate.rs,adversary.rs,robustness.rs,anchors.rs,holdout.rs,league.rs,pareto.rs,unification.rs}` are the correct places to keep scientific policy outside prompts. The `validation` tree (`crates/openqg-core/src/validation/{runbook.rs,schema.rs,hash.rs,envelope.rs,manifest/*}`) is the right home for replay and receipt rules. The bench runner surfaces in `crates/openqg-bench/src/zyal_genome/{eval.rs,scoring.rs,quality_gate.rs,selection.rs,novelty.rs,records.rs,run_summary.rs,validate.rs}` are appropriately separated from theory semantics.

Second, the ZYAL stage decomposition is explicit and reviewable. `ZYAL/stages/00-atlas` through `10-promotion` make hidden workflow assumptions inspectable. The live `00-atlas` artifact for `run-data/review-artifact-11-openqg-live-batch-3-live-g0001-00-atlas-research_synthesis.json` correctly refuses to invent memory contents: it records `memory_refs_declared` but `memory_contents_available_in_packet=false`, and it refuses benchmark-derived claims when only `run-events.jsonl`, `stage-ledger.jsonl`, and stage evaluation events are named.

Third, the system already contains adversarial fixtures: `ZYAL/anchors/decoy-gray-box-fudge.json`, `decoy-h0-100.json`, `decoy-omega-runaway.json`, `probe-overfit-echo.json`, plus good anchors such as `good-lcdm-baseline.json` and `good-h0-tension-resolver.json`. That is exactly the right direction for judge calibration.

## Top five v4 risks

1. **Evidence-name laundering.** Multiple artifacts name evidence paths without proving contents. `00-atlas` explicitly says required evidence contents are unavailable. `05-compatibility` fails because a repaired gene payload is missing and route tier is ambiguous. A promoted candidate can therefore look grounded while remaining unparsed.

2. **Route-tier contract drift.** `run-data/review-artifact-09-openqg-v4-seed-12-live-g0001-05-compatibility-hard_stage_repair.json` and the hybrid artifacts repeatedly expose `expected=top20_pct_only`, `observed=top20_pct`. This is not cosmetic: it means the hard-route proof lane can be accepted by alias, prose, or inconsistent validator assumptions.

3. **Lineage stagnation.** The current live artifacts are overwhelmingly `g0001` stage repairs. The selection artifact for `09-selection-mutation` chooses a single `repair_anchor`, with `parents=[]`, `plateau_state=not_resolved_in_packet`, and `source_diversity=memory_ref_only`. That diagnoses a search loop that may be replaying a root lineage rather than evolving a population.

4. **Derivation theater.** The code has `certificate.rs`, `proposal.rs`, and `vetoes.rs`, but the artifacts discuss derivation confidence mostly as structured claims, not as value-level obligations checked by symbolic, dimensional, limit, or solver-backed machinery. A candidate can still survive by being self-consistent and rhetorically precise.

5. **Rubric incomparability.** The stated goal is to score string/M-theory, LQG, asymptotic safety, causal sets, and other human theories under the same bar. The present ZYAL stage outputs are stage-repair artifacts, not a league-grade rubric capable of separating mature human programs from LLM-generated fragments.

## v4 acceptance gate

Do not promote v4 until `crates/openqg-bench/src/zyal_genome/quality_gate.rs` fails closed on missing evidence contents, `selection.rs` proves at least N non-root lineages per island, `certificate.rs` stores typed derivation obligations, and `theory_league.rs` runs a baseline league containing real theories and decoys. The project is promising because the architecture is auditable; it is not yet trustworthy because its strongest artifacts are still boundary statements rather than verified physics claims.
