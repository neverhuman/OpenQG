# 11. Concrete file-level engineering spec

Batch tab: 1.

This is the implementation checklist. It intentionally focuses on durable Rust policy plus ZYAL schemas/prompts, matching the Jankurai boundary that Rust owns policy/config/receipts/run contracts while TypeScript owns dashboards.

## Core theory files

- `crates/openqg-core/src/theory/proposal.rs`: add `ClaimGraph`, `Claim`, `ClaimKind`, `ClaimId`, `ParameterRef`, `EvidenceRef`, and `ModelComplexityLedger` references to `TheoryProposal`. Require proposals to declare sectors, assumptions, equations, derivations, predictions, and falsifiers.
- `crates/openqg-core/src/theory/certificate.rs`: replace prose certificate semantics with `DerivationCertificateV4`, `DerivationObligation`, `VerifierReceipt`, and certificate status. Require evidence hashes and typed obligation results.
- `crates/openqg-core/src/theory/vetoes.rs`: add hard vetoes: `MissingMaterializedEvidence`, `RouteTierMismatch`, `PromotedClaimWithoutCertificate`, `NarrativeUnificationOnly`, `HiddenSectorParameter`, `PromptLeakageSuspected`, `AnchorOverfit`, `NoConcreteFalsifier`.
- `crates/openqg-core/src/theory/evaluate.rs`: run vetoes and certificate checks before numeric scoring. Return vetoed scorecards without rank.
- `crates/openqg-core/src/theory/adversary.rs`: generate concrete falsifier tasks per claim, including limit, dimensional, stability, and data-split attacks.
- `crates/openqg-core/src/theory/anchors.rs`: split anchors into public calibration, private holdout, and rotating canary sets. Expose blinded IDs only to LLM stages.
- `crates/openqg-core/src/theory/robustness.rs`: add `HonestyRollback` records and perturbation-stability checks.
- `crates/openqg-core/src/theory/holdout.rs`: seal holdout manifests and ensure prompt exclusion.
- `crates/openqg-core/src/theory/unification.rs`: add `UnificationClaim`, `PhysicalSector`, `SharedObjectRef`, and `CrossSectorConstraint`.
- `crates/openqg-core/src/theory/unification_data.rs`: encode baseline sector requirements and known cross-sector constraints.
- `crates/openqg-core/src/theory/evolve.rs` and `mutation.rs`: distinguish pipeline, theory, evidence, adversary, and recombination mutations.
- Add `crates/openqg-core/src/theory/claim_graph.rs`, `dimensions.rs`, `limits.rs`, `verifier.rs`, and `complexity.rs`.

## Scoring files

- `crates/openqg-core/src/scoring/scorecard.rs`: implement veto-first `ScorecardV4` with uncertainty bands.
- `crates/openqg-core/src/scoring/evidence.rs`: score by evidence tier and claim relevance; require materialized refs.
- `crates/openqg-core/src/scoring/likelihood.rs`: enforce common splits, covariance, and no epsilon-only wins.
- `crates/openqg-core/src/scoring/covariance.rs`: expose uncertainty intervals to scorecards.
- Add `crates/openqg-core/src/scoring/rubric.rs`.

## Data and validation files

- `crates/openqg-data/src/registry.rs`: add `EvidenceTier`, `DatasetDescriptor`, `allowed_use`, `leakage_policy`, and `manifest_hash`.
- `crates/openqg-bench/src/data.rs` and `data_audit.rs`: add tier audit and leakage checks.
- `crates/openqg-core/src/validation/manifest/dataset.rs`: require tier, license, split, manifest hash, and sealed-holdout policy.
- `crates/openqg-core/src/validation/hash.rs`: provide helper hashing for evidence refs and verifier receipts.
- `crates/openqg-core/src/validation/runbook.rs`: validate runbook gates for evidence materialization and route canonicalization.

## Bench and ZYAL genome files

- `crates/openqg-bench/src/zyal_genome/route.rs`: replace string route tiers with a canonical enum. Normalize once; serialize canonical values only.
- `crates/openqg-bench/src/zyal_genome/quality_gate.rs`: fail closed on missing evidence contents, route mismatch, missing repaired gene payload, unresolved plateau, prose-only derivation, and uncalibrated judge.
- `crates/openqg-bench/src/zyal_genome/selection.rs`: add `PopulationProgressAudit`; block single-root replay promotion.
- `crates/openqg-bench/src/zyal_genome/novelty.rs`: compute novelty from canonical claim fingerprints, not prose.
- `crates/openqg-bench/src/zyal_genome/scoring.rs`: consume `ScorecardV4` and veto results.
- `crates/openqg-bench/src/zyal_genome/records.rs`: persist claim IDs, evidence hashes, route enum, and lineage graph.
- `crates/openqg-bench/src/zyal_genome/run_summary.rs`: emit population progress, judge calibration, certificate coverage, and data-tier usage.
- `crates/openqg-bench/src/zyal_judge.rs`: consume blinded `JudgePacketV4`; record judge calibration.
- `crates/openqg-bench/src/theory_league.rs`: add human-contender baseline league and decoy league.

## ZYAL files

- `ZYAL/schemas/zyal-gene-eval.schema.json`: add `gene_kind`, `claim_refs`, `evidence_refs`, `route_tier_canonical`, `certificate_obligations`, and `failure_penalty`.
- `ZYAL/stages/*/artifacts.schema.json`: update each stage with required fields described in review 10.
- `ZYAL/stages/*/score.yml`: penalize unresolved evidence and prose-only claims.
- `ZYAL/runs/run-jailgun-only.zyal`: add preflight materialization, route canonicalization, post-stage gates, and sealed holdout isolation.
- `docs/scoring.md`, `docs/theory-league.md`, `docs/data-policy.md`, `docs/ZYAL.md`, `docs/zyal-next-level-design.md`: update to v4 rubric, data tiers, claim graph, and certificate-first promotion.

## Tests

Add Rust tests in `crates/openqg-core/tests/theory_pipeline.rs`, `crates/openqg-core/tests/zyal.rs`, and `crates/openqg-bench/src/zyal_genome/tests/*`: route mismatch fails, missing evidence hash fails, single-root promotion fails, fake derivation vetoes, decoy overfit penalizes judge, and human baseline league emits uncertainty bands.
