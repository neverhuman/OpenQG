# 07. Adversary / judge / robustness-under-judge

Batch tab: 1.

The adversary/judge layer is directionally right but presently too close to the proposer’s language game. The goal should not be to make a harsher LLM judge. The goal should be to make judges route claims into hard checks, use anchors/decoys to calibrate themselves, and record rollback-worthy failures.

## Current surfaces

Core adversarial modules live in `crates/openqg-core/src/theory/{adversary.rs,anchors.rs,robustness.rs,vetoes.rs,holdout.rs}`. Bench-side orchestration appears in `crates/openqg-bench/src/{zyal_judge.rs,zyal_robustness.rs}` and `crates/openqg-bench/src/zyal_genome/{quality_gate.rs,quality_metrics.rs,jailgun_classify.rs,jailgun_health.rs,live_call.rs}`. Anchor fixtures live under `ZYAL/anchors/`, including good references (`good-lcdm-baseline.json`, `good-h0-tension-resolver.json`) and decoys (`decoy-gray-box-fudge.json`, `decoy-h0-100.json`, `decoy-omega-runaway.json`, `probe-overfit-echo.json`).

This is a strong start: real systems need known-good and known-bad calibration. But the decoys must become blinded, rotating, and scored against judge behavior.

## Judge failure modes

1. **Rubric echo.** If the candidate sees stage prompts and anchor labels, it can emit the expected fields. The judge then rewards conformance rather than substance.

2. **Decoy familiarity.** Public decoys become training examples. A candidate can learn “gray-box fudge bad” and avoid those words while preserving the hack.

3. **Self-consistency preference.** LLM judges are easily impressed by internal coherence. A unified theory must be externally constrained, not merely self-consistent.

4. **Vague adversarial criticism.** Judges can sound skeptical without producing falsifiers. That wastes compute and can unfairly bury speculative but useful fragments.

5. **Collusive evolution.** Proposer, repairer, and judge co-adapt to a private style. The system begins scoring its own dialect.

## Hardening design

### Blinded Judge Packets

`zyal_judge.rs` should receive a blinded packet: claim graph, evidence hashes, certificate statuses, and anonymized sector labels. It should not see proposer prompt, stage variant history, or candidate rhetoric unless specifically evaluating communication quality. Add `JudgePacketV4` to `crates/openqg-core/src/types/zyal.rs`.

### Anchor split

Move anchors into three classes:

- `public_calibration`: visible in docs and prompts.
- `private_holdout`: loaded only by Rust from a non-prompt path.
- `rotating_canary`: regenerated or sampled per run.

`crates/openqg-core/src/theory/anchors.rs` should expose only hashed IDs to LLM agents. `ZYAL/anchors/*` can remain public calibration, but private anchors must not sit in prompt-visible stage memory.

### Judge scorecard

Judges need their own league. For every judging batch, score:

- false acceptance of decoys,
- false rejection of good anchors,
- ability to produce concrete falsifiers,
- ability to cite materialized evidence refs,
- stability under paraphrase,
- independence from proposer wording.

Store this in `crates/openqg-bench/src/zyal_genome/quality_metrics.rs` and `run-data/v4-live-feedback-index.jsonl`.

### Honesty rollback

When a promoted candidate later fails a private anchor or certificate check, rollback should penalize the exact earlier claims and judges. Add `HonestyRollback` records to `crates/openqg-core/src/theory/robustness.rs`: `{claim_id, prior_decision, later_failure, responsible_judge_ids, penalty}`. Feed this into `selection.rs` so judges with bad calibration lose influence.

## Robustness-under-judge gates

A candidate should pass four gates before promotion:

1. **Anchor gate:** survives good/bad anchor calibration and does not score decoys above baselines.
2. **Falsifier gate:** adversary emits at least one concrete, runnable falsifier per major claim.
3. **Perturbation gate:** small wording changes do not change judge verdict materially.
4. **Blind holdout gate:** candidate did not see the holdout or its labels.

`vetoes.rs` should include `JudgeUncalibrated`, `AnchorOverfit`, `NoConcreteFalsifier`, and `PromptLeakageSuspected`.

## Stage-level changes

`07-macro-test` should run anchors and decoys as first-class macro tests. `08-failure-slicing` should require `failure_penalty` and `evidence_refs` per slice, as already diagnosed by live artifacts. `10-promotion` should include a judge calibration appendix: how judges performed on decoys during the same run.

The adversary should not be a clever critic. It should be a machine that transforms beautiful claims into ugly obligations. That is the robustness-under-judge philosophy v4 should enforce.
