# 06. Derivation checks & the derivation oracle

Batch tab: 1.

The derivation system is the main difference between a serious physics discovery engine and a rhetoric engine. V4 should treat derivations as typed, replayable artifacts. The current code layout already has the right anchor: `crates/openqg-core/src/theory/certificate.rs` with adjacent `proposal.rs`, `evaluate.rs`, and `vetoes.rs`. What is missing is a hard distinction between form-level plausibility and value-level derivation.

## Form versus value

A form-level derivation says: “Starting from action A, in limit L, result R follows.” This is often what an LLM can write. A value-level derivation binds named assumptions, equations, transformations, units, parameter domains, and a machine-checkable or numerically reproducible witness. V4 should give no promotion credit for form-level derivations except as TODOs.

## Required certificate structure

Add to `certificate.rs`:

```rust
pub struct DerivationCertificateV4 {
    pub claim_id: ClaimId,
    pub obligations: Vec<DerivationObligation>,
    pub evidence_refs: Vec<EvidenceRef>,
    pub status: CertificateStatus,
    pub verifier_receipts: Vec<VerifierReceipt>,
}
```

Obligation kinds should include:

- `DimensionalConsistency`: every term in an equation has compatible units.
- `KnownLimit`: e.g. Newtonian weak-field, GR limit, SR/local Lorentz, QFT local limit, LCDM limit.
- `SymbolicIdentity`: algebraic or variational derivation checked by a CAS.
- `NumericalWitness`: independent solver reproduces a stated curve or bound.
- `PositivstellensatzOrInequality`: proves positivity/stability/energy condition where expressible.
- `FormalSketch`: Lean/Isabelle/Coq skeleton for a restricted theorem.
- `NoGhostNoTachyonCheck`: perturbative stability for modified gravity sectors.
- `RenormalizationOrUVCheck`: where a theory claims UV completion.

## Plug-in points

`proposal.rs` should require every promoted `TheoryProposal` to contain a `ClaimGraph`; each derivation claim gets at least one obligation. `evaluate.rs` should compute certificate status before score. `vetoes.rs` should fail closed for `PromotedClaimWithoutCertificate`, `DimensionMismatch`, `KnownLimitFailure`, and `FakeDerivationProseOnly`. `assessment.rs` should display certificate coverage, not just judge commentary. `unification.rs` should require cross-sector derivation links: if a candidate says one principle explains both cosmology and particle content, the shared symbol or mechanism must appear in both claim subgraphs.

## Concrete checks to add

1. **Dimensional parser.** Add `crates/openqg-core/src/theory/dimensions.rs`. It can start small: parse symbolic terms with units for cosmology and gravity equations. Use it in `certificate.rs` and `sectors/{fr.rs,ndgp.rs}`.

2. **Limit checker.** Add `crates/openqg-core/src/theory/limits.rs`. A `LimitMap` should specify parameter substitutions and asymptotic operations. Require weak-field, low-energy, flat-space, and LCDM/GR limits where claimed.

3. **Symbolic derivation receipt.** Add `VerifierReceipt { tool, version, input_hash, output_hash, status }`. Do not store raw prompts. Store reproducible symbolic inputs under run artifacts and hash them through `validation/hash.rs`.

4. **AI-Hilbert-style value tasks.** For every claim, generate small theorem/proof tasks: “derive Friedmann equation under assumptions X,” “show Newtonian potential emerges,” “prove conservation law from symmetry.” Use LLMs to propose, but Rust should decide whether receipts exist.

5. **Counterexample search.** `adversary.rs` should generate parameter/limit counterexamples. Example: if an f(R)-like sector passes background expansion but fails stability, issue a veto rather than a small penalty.

## Why this matters for cheating

A candidate can currently write a “derivation certificate” that is just a good explanation. It can also overfit cosmology by adding a dark-energy knob and then claim unification because the same knob appears in multiple equations. Dimensional checks catch nonsense terms; limit checks catch theories that do not recover known physics; stability checks catch modified gravity pathologies; formal sketches catch circular definitions; numerical witnesses catch plot-level fraud.

## Engineering spec

Add files:

- `crates/openqg-core/src/theory/claim_graph.rs`
- `crates/openqg-core/src/theory/dimensions.rs`
- `crates/openqg-core/src/theory/limits.rs`
- `crates/openqg-core/src/theory/verifier.rs`
- `crates/openqg-bench/src/cli/theory.rs` subcommand `certificate-check`

Modify:

- `certificate.rs`: introduce `DerivationCertificateV4`.
- `proposal.rs`: require claim graph and obligation IDs.
- `vetoes.rs`: add derivation hard vetoes.
- `evaluate.rs`: run certificate checks before scoring.
- `ZYAL/stages/06-assemble-modules/prompt.md`: require each module claim to list obligations.
- `ZYAL/stages/10-promotion/artifacts.schema.json`: require certificate coverage table.

A candidate should be allowed to remain speculative with weak certificates. It should not be allowed to be promoted as expert-review-worthy without value-level derivation evidence.
