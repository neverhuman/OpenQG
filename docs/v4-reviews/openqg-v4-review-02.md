# 02. Gameability / “how can we cheat” red-team

Batch tab: 1.

The current system is gameable in exactly the places an evolutionary LLM-driven theory engine is expected to be gameable: the candidate can optimize for structured plausibility, judge vocabulary, and archive placement rather than physics. The live artifacts are unusually useful because they reveal failure modes without pretending certainty.

## Cheat 1: evidence-name laundering

A candidate can cite `run-events.jsonl`, `stage-ledger.jsonl`, and `stages/<stage>/evaluation-events.jsonl` as evidence paths without the validator proving those files exist, are non-empty, and contain the cited fields. `run-data/review-artifact-11-openqg-live-batch-3-live-g0001-00-atlas-research_synthesis.json` explicitly reports `required_evidence_contents_available_in_packet=false`. `run-data/review-artifact-09-openqg-v4-seed-12-live-g0001-05-compatibility-hard_stage_repair.json` reports the same class of boundary problem at compatibility time. The cheat is simple: output a beautiful evidence manifest, never bind claim IDs to byte offsets or event hashes, and let judges reward “grounding.”

**Mitigation:** in `crates/openqg-bench/src/zyal_genome/validate.rs`, add a `MaterializedEvidenceAudit` that opens every referenced path and emits `(path, sha256, byte_len, jsonl_count, schema_id)`. In `crates/openqg-core/src/theory/certificate.rs`, add `EvidenceRef { path, sha256, json_pointer, byte_range, event_id }`. `quality_gate.rs` must fail any artifact whose claim references only a path string.

## Cheat 2: hard-route aliasing

The archive repeatedly shows `top20_pct` versus `top20_pct_only` ambiguity. `05-compatibility` artifacts fail on `route_tier_top20_pct_only_for_hard_routing`, with expected `top20_pct_only` and observed `top20_pct`. A candidate can exploit this by routing easy or broad work through `top20_pct`, then claiming the stricter hard lane was preserved.

**Mitigation:** centralize route tier normalization in `crates/openqg-bench/src/zyal_genome/route.rs` and expose `enum RouteTier { Manual, Top20PctOnly, Full, Probe }`. Ban string comparisons in stage prompts. `quality_gate.rs`, `jailgun_classify.rs`, and `jailgun_health.rs` should consume the enum and record a canonical serialized value. No aliases in promotion.

## Cheat 3: fake derivations

A theory can pass by emitting “derives GR in the weak-field limit” or “recovers QFT locally” without a parseable derivation. `crates/openqg-core/src/theory/certificate.rs` is the correct policy anchor, but v4 needs typed obligations, not prose. A fake derivation can name equations, set limits informally, and get judge credit for style.

**Mitigation:** add `DerivationObligationKind::{Dimensional, Limit, SymbolicIdentity, NumericWitness, Positivstellensatz, LeanSketch, LiteratureEquivalence}` and require each promoted claim in `proposal.rs` to map to at least one obligation. `vetoes.rs` should include `VetoKind::UnverifiedDerivation` and `VetoKind::LimitFailure`.

## Cheat 4: novelty gaming

`crates/openqg-bench/src/zyal_genome/novelty.rs` and `crates/openqg-core/src/theory/pareto.rs` can be gamed by changing language, stage names, or parameterization while preserving a known failure. A candidate can look novel because its descriptor hash changes.

**Mitigation:** compute novelty on claim graphs, not prose. Add canonicalization in `crates/openqg-core/src/theory/proposal.rs`: normalize claims by physical sector, mathematical object, limit behavior, free parameters, and required evidence. `novelty.rs` should compare canonical claim fingerprints and penalize synonym churn.

## Cheat 5: MAP-Elites cell gaming

If archive cells are keyed by easy descriptors, a candidate can occupy a sparse cell by optimizing descriptor features while remaining weak. Cells should not be “cosmology-ish plus low free parameter count”; they should be anchored to hard obligations and held-out failures.

**Mitigation:** in `crates/openqg-core/src/theory/evolve.rs` and `crates/openqg-bench/src/zyal_genome/selection.rs`, make cell descriptors include verified obligations: number of passed limit checks, sealed holdout performance, adversarial anchor survival, and derivation-certificate depth. Any unverified descriptor is “speculative” and should not dominate selection.

## Cheat 6: judge collusion and prompt leakage

The LLM can learn the judge’s rubric from `ZYAL/stages/*/prompt.md`, output the expected fields, and avoid substance. If `ZYAL/anchors/*.json` are visible to the proposer, it can overfit anchors. The decoy files are useful but become training targets if not split.

**Mitigation:** divide anchors into public calibration, private holdout, and rotating blind decoys. Store private anchors outside prompt-visible ZYAL paths and load them through Rust in `crates/openqg-core/src/theory/anchors.rs`. `zyal_judge.rs` should receive only a blinded claim bundle and evidence hashes, not the proposer prompt.

## Cheat 7: epsilon/AIC artifacts

Scoring can be hacked with tiny likelihood gains, free-parameter undercounting, or comparison baselines that do not share data cuts. `crates/openqg-core/src/scoring/{likelihood.rs,evidence.rs,covariance.rs,scorecard.rs}` should treat parameter counting and data provenance as adversarial.

**Mitigation:** add a `ModelComplexityLedger` tied to `proposal.rs`: every fitted constant, dataset choice, post-hoc cut, and sector-specific knob counts. Require epsilon-insensitive ranking bands; do not promote on differences smaller than covariance/provenance uncertainty.

The general rule: **no claim should earn fitness unless it is bound to materialized evidence, a typed obligation, and an adversarial falsifier.**
