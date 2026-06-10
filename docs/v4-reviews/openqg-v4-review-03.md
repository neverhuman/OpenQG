# 03. Theory decomposition & the gene/stage model

Batch tab: 1.

The 11-stage ZYAL decomposition is a good workflow skeleton but not yet a sufficient decomposition of a unified theory of physics. It decomposes the *engineering process* more cleanly than it decomposes the *scientific object*. That is the main v4 design gap.

## Current stage model

The archive defines stages `00-atlas` through `10-promotion` under `ZYAL/stages/*`, with each stage carrying `README.md`, `stage.yml`, `prompt.md`, `score.yml`, `memory.yml`, and `artifacts.schema.json`. The stages are:

`00-atlas`, `01-decompose-known`, `02-decompose-failed`, `03-generate-genes`, `04-repair-genes`, `05-compatibility`, `06-assemble-modules`, `07-macro-test`, `08-failure-slicing`, `09-selection-mutation`, and `10-promotion`.

This is a strong agent-workflow abstraction: intake, decomposition, gene generation, repair, compatibility, assembly, macro testing, failure slicing, selection, and promotion. The live artifacts confirm the intended contracts. For example, `00-atlas` records run context, stage registry, seed memory, evidence boundaries, and lineage; `02-decompose-failed` requires root-cause labels, route-tier grounding, and mutation lineage; `04-repair-genes` requires failure-class-first repairs and mutation ops; `08-failure-slicing` requires per-slice evidence refs and penalties; `09-selection-mutation` selects survivors and emits mutation ops.

## What is missing scientifically

A unified theory is not naturally decomposed into “stage genes.” It needs at least four orthogonal decompositions:

1. **Claim graph decomposition.** Claims should be first-class objects: assumptions, equations, limits, derivations, predictions, empirical fits, and falsifiers. Add `ClaimId`, `ClaimKind`, `depends_on`, `evidence_refs`, `status`, and `failure_history` to `crates/openqg-core/src/theory/proposal.rs`.

2. **Physical sector decomposition.** A candidate must specify how it touches cosmology, gravity, QFT, particle masses/couplings, black holes, thermodynamics, Lorentz symmetry, and quantum measurement. Current sector modules include `crates/openqg-core/src/theory/sectors/{fr.rs,ndgp.rs}` and `crates/openqg-core/src/cosmology/*`; v4 needs a generic `SectorClaim` interface rather than only f(R)/nDGP-like examples.

3. **Mathematical object decomposition.** The proposal should identify its action, fields, symmetries, gauge group/geometry, quantization rule, free constants, and limit maps. Without this, self-consistency can masquerade as unification.

4. **Verification task decomposition.** Each claim should spawn obligations in `certificate.rs`: dimensional check, known-limit recovery, symbolic identity, numerical witness, dataset comparison, literature-equivalence, and adversarial counterexample.

## Failure revealed by current artifacts

The stage model currently handles *failed stage traces* better than *failed theory claims*. `02-decompose-failed` artifacts focus on failure slices lacking root-cause labels and lineage. That is necessary, but a physics candidate can still have a clean stage pipeline and bad physics. Conversely, a theory fragment might fail a stage interface but remain scientifically valuable.

The repair loop should therefore separate:

- **Pipeline genes:** prompt/config/schema/routing/selection mechanisms.
- **Theory genes:** physical assumptions, equations, sector modules, derivation lemmas.
- **Evidence genes:** datasets, cuts, likelihood choices, holdouts, covariance assumptions.
- **Adversary genes:** falsifiers, anchors, decoys, perturbation tests.

Put this distinction in `ZYAL/schemas/zyal-gene-eval.schema.json` and `crates/openqg-core/src/types/zyal.rs`. Today a “gene” can mean a stage repair, a candidate idea, or a failure assumption. That ambiguity is reward-hackable.

## Stage-specific upgrades

`01-decompose-known` should produce a normalized baseline decomposition for GR+SM fragments and for the top human contenders. `02-decompose-failed` should slice both stage failures and theory-claim failures. `03-generate-genes` should emit typed theory genes, not just plausible ideas. `04-repair-genes` should require a before/after claim graph diff. `05-compatibility` should check interface compatibility between physical sectors, not only stage handoff fields. `06-assemble-modules` should validate unit consistency and shared constants across modules. `07-macro-test` should run known-limit and anchor tests. `08-failure-slicing` should assign penalties to failed claims. `09-selection-mutation` should select across claim-level Pareto fronts. `10-promotion` should never promote a theory blob; it should promote a candidate plus a certificate dossier.

## Engineering spec

Add `crates/openqg-core/src/theory/claim_graph.rs`. Export it from `theory/mod.rs`. Add `ClaimGraph` to `TheoryProposal` in `proposal.rs`. Add `GeneKind::{Pipeline,Theory,Evidence,Adversary}` to `crates/openqg-core/src/types/zyal.rs` and the ZYAL JSON schema. Update all `ZYAL/stages/*/artifacts.schema.json` to require `gene_kind`, `claim_refs`, and `evidence_refs` where relevant. The stage model is worth keeping, but v4 should make theory decomposition independent of agent workflow decomposition.
