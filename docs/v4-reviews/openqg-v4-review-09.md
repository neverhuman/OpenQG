# 09. Unification / cross-domain consistency

Batch tab: 1.

The current system risks treating unification as cross-stage self-consistency. A candidate can look unified because its modules agree with each other, use shared vocabulary, and pass internal compatibility. Real unification requires cross-domain constraint: the same mathematical mechanism must explain or constrain multiple physical sectors without hidden independent knobs.

## Current unification surfaces

The core files are `crates/openqg-core/src/theory/{unification.rs,unification_data.rs,proposal.rs,certificate.rs,evaluate.rs,vetoes.rs}` plus sector modules `crates/openqg-core/src/theory/sectors/{fr.rs,ndgp.rs}` and cosmology modules `crates/openqg-core/src/cosmology/{background.rs,forward.rs,growth.rs,subprocess.rs}`. Stage-level assembly happens in `ZYAL/stages/05-compatibility`, `06-assemble-modules`, and `07-macro-test`.

The file layout is promising, but v4 should make “unification” a typed claim with obligations rather than a score sentiment.

## How self-consistency can masquerade as unification

1. **Shared words, independent parameters.** A theory can call the same scalar field responsible for inflation, dark energy, and particle masses while fitting each sector with separate functions or constants.

2. **Compatibility without derivation.** `05-compatibility` currently focuses on handoff contracts and route tier. That does not prove that two physical sectors share a coherent action, symmetry, or limit.

3. **Cosmology-only success.** A candidate can fit background expansion and growth while saying nothing about quantum sectors. That may be useful modified gravity, but not unified physics.

4. **Retrofitted limits.** Candidates can assert GR/QFT recovery after the fact without parameter-domain proof.

5. **Compression fraud.** A low parameter count can hide complexity in functional forms, priors, data cuts, or prompt-selected sectors.

## Required unification claim type

Add `UnificationClaim` to `crates/openqg-core/src/theory/unification.rs`:

```rust
pub struct UnificationClaim {
    pub claim_id: ClaimId,
    pub sectors: Vec<PhysicalSector>,
    pub shared_objects: Vec<SharedObjectRef>,
    pub independent_parameters: Vec<ParameterRef>,
    pub cross_sector_constraints: Vec<ConstraintRef>,
    pub required_limits: Vec<LimitObligationId>,
    pub failure_modes: Vec<UnificationFailureMode>,
}
```

A shared object can be an action term, symmetry, field, gauge structure, geometry, conservation law, or quantization rule. It should not be a prose theme.

## Cross-domain tests

1. **Shared-parameter audit.** If the same parameter appears in cosmology and particle physics, require the same value and uncertainty unless a transformation is derived. Put this in `unification_data.rs`.

2. **Limit map consistency.** Every sector must declare how it recovers known physics. `limits.rs` should check that sector limit maps do not contradict each other.

3. **Dimensional bridge.** A cross-sector equation must pass dimensional checks in both sectors.

4. **No-hidden-knob test.** `ModelComplexityLedger` should count sector-specific functions and data cuts. A unification bonus is allowed only when adding a sector does not add an unconstrained free knob.

5. **Adversarial sector omission.** `adversary.rs` should ask: “Which sector would falsify this unification claim fastest?” If the theory cannot answer, demote it from unified to speculative.

## Stage changes

`05-compatibility` should split into interface compatibility and physical compatibility. Interface compatibility checks handoff fields, route tier, and lineage. Physical compatibility checks shared objects, parameters, dimensions, and limits.

`06-assemble-modules` should not assemble modules unless every cross-reference resolves to a typed `SharedObjectRef` or `ConstraintRef`.

`07-macro-test` should run cross-domain stress tests: change a parameter in one sector and verify predicted effects in another. If no cross-effect exists, the “unification” is likely only narrative.

`10-promotion` should display a unification matrix: rows are sectors, columns are shared objects, limits, empirical contacts, and certificate status. Empty cells are acceptable if scoped honestly. They are not acceptable in a top-level unification claim.

## Engineering spec

Modify:

- `crates/openqg-core/src/theory/unification.rs`: add typed `UnificationClaim`, `PhysicalSector`, `SharedObjectRef`, `CrossSectorConstraint`.
- `crates/openqg-core/src/theory/unification_data.rs`: store baseline sector requirements and known cross-sector constraints.
- `crates/openqg-core/src/theory/vetoes.rs`: add `NarrativeUnificationOnly`, `HiddenSectorParameter`, and `ContradictoryLimitMap`.
- `crates/openqg-bench/src/theory_league.rs`: score human theories with the same unification matrix.
- `docs/scoring.md`: make unification bonus impossible without cross-sector constraints.

Unification should mean fewer independent explanations under more constraints. Anything else is self-consistency with a grand title.
