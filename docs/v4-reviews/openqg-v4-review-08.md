# 08. Datasets & evidence — tiers T1–T5 and integration

Batch tab: 1.

The current system has useful evidence boundaries but not enough discriminating evidence. `docs/data-policy.md` and `crates/openqg-data/src/registry.rs` should become central to v4. A unified-theory candidate does not need broad data-fitting to be interesting, but it must touch reality through known limits, precision constraints, and discriminating datasets.

## Evidence problem in current runs

The live `00-atlas` artifacts correctly avoid benchmark-derived claims when only evidence manifests are present. `05-compatibility` and `08-failure-slicing` artifacts show the same pattern: required evidence is named, but actual contents are not always attached. This protects honesty, but it also means many current scores are evidence-boundary scores, not physics scores.

## Data tiers

Use `docs/data-policy.md` to enforce a tiered evidence policy:

### T1 — invariant theory checks

No empirical dataset required. Dimensional consistency, known mathematical identities, conservation laws, and limit maps. Implement through `crates/openqg-core/src/theory/{dimensions.rs,limits.rs,certificate.rs}`. These are cheap and should run for every claim.

### T2 — canonical baselines and anchors

LCDM, GR weak-field, Standard Model/QFT fragments, known modified-gravity failures, and ZYAL anchors. Store anchor metadata in `crates/openqg-core/src/theory/anchors.rs` and baseline league entries in `crates/openqg-bench/src/theory_league.rs`.

### T3 — public precision datasets

These should be wired through `crates/openqg-data/src/registry.rs` with manifests validated by `crates/openqg-core/src/validation/manifest/dataset.rs`:

- CMB constraints: Planck likelihood summaries, BAO distances.
- DESI BAO / expansion history where licenses permit.
- Pantheon+/supernova distance moduli.
- BBN light-element constraints.
- Hubble/local distance ladder summaries.
- Growth structure: fσ8, weak lensing summaries.
- Gravitational wave propagation constraints, especially GW170817 speed bounds.
- Solar-system and binary-pulsar gravity constraints.
- Lorentz-invariance violation bounds.
- Equivalence principle tests.
- Neutrino mass/cosmology constraints.

### T4 — sector-specific discriminators

For theories claiming particle unification: PDG masses/couplings, neutrino oscillation parameters, running couplings, anomaly cancellation checks. For quantum gravity: black-hole thermodynamics, entropy-area relation, semiclassical Hawking limits, causal structure constraints. These should not be reduced to one likelihood; they should populate claim obligations.

### T5 — sealed holdouts and expert packs

Private or delayed-release datasets, blinded decoy anchors, and human-expert challenge packs. These are not for broad fitting. They are for robustness-under-judge and overfit detection.

## Integration spec

In `crates/openqg-data/src/registry.rs`, define:

```rust
pub enum EvidenceTier { T1Invariant, T2Anchor, T3PublicPrecision, T4SectorSpecific, T5SealedHoldout }
pub struct DatasetDescriptor { id, tier, domain, license, manifest_hash, allowed_use, leakage_policy }
```

In `crates/openqg-bench/src/data.rs` and `data_audit.rs`, add `openqg data audit --tier T3 --candidate <id>` to verify no candidate saw sealed or forbidden data. `cli/data.rs` should print tier, license, citation, split status, and leakage risk.

In `crates/openqg-core/src/scoring/evidence.rs`, score datasets by tier and claim relevance, not by total count. A candidate should not get extra credit for touching many weak datasets. It gets credit for surviving the right discriminators.

## Discriminating examples

A dark-energy modified-gravity candidate must survive CMB+BAO+SN background *and* growth constraints, plus GW speed and stability. A theory with a new scalar field must declare screening, fifth-force constraints, and parameter domains. A quantum-gravity candidate that claims black-hole unification must address entropy/temperature limits and semiclassical recovery. A particle-unification claim must engage anomaly cancellation and running couplings.

## Data leakage controls

`ZYAL/stages/00-atlas/prompt.md` should state which tiers are visible. `07-macro-test` should never reveal T5 labels. `holdout.rs` should log sealed nondeterminism receipts: random seed, split ID hash, data manifest hash, and candidate prompt hash. `quality_gate.rs` should veto if a candidate includes suspicious exact private anchor language.

The right v4 posture is not “fit more data.” It is “bind every empirical claim to the most discriminating allowed evidence tier, and make leakage impossible to ignore.”
