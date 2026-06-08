# Legacy engine retirement — migration plan

How to retire the float-genome physics engine (`zyal_robustness.rs` + `zyal_judge.rs`) and point
the live ZYAL robustness pipeline at the new symbolic-theory engine
(`openqg_core::theory` + `cosmology`). The new engine already runs in parallel via
`openqg-bench theory evolve`; this plan migrates the *live* pipeline without a big-bang rewrite.

## Integration surface
The entire legacy integration lives in one function: `zyal genome run --variant hybrid` →
`run_variant()` (`zyal_genome/run_variant.rs:744`) → **`emit_hybrid_evolution_artifacts()`**
(`zyal_genome/hybrid_artifacts.rs`). Both engines already share the grading kernel
`openqg_core::score_metrics` — the seam that makes the swap tractable.

## Replacement mapping (legacy → new)
| Legacy (bench) | New (core) |
|---|---|
| `zyal_robustness::Genes` (7 floats) | `theory::Theory` (parameters+provenance, α-basis, stability, screening, `background: CosmologyParams`) |
| `Genes::baseline()` | `Theory::baseline_lcdm()` / `CosmologyParams::planck_lcdm()` |
| `Genes::forward_map()` (identity) | `BackgroundForwardModel::predict()` (real Friedmann/distances/sound-horizon/BBN/CMB-priors) |
| `derive_genes`/`mutate_genes` | `theory::mutate`/`recombine` + `Rng` (splitmix64) |
| `score_predictions`→`RobustnessOutcome` | `theory::evaluate`→`Evaluation` (+`assess`→`CandidateAssessment`) |
| `whitebox_violations()` (text markers) | `run_veto_cascade` → `FreeParameter`/`UnprovenancedParameter` (structural) |
| judge survival score | `CandidateAssessment.final_fitness` (β-gated, unification-gated) |
| `MapElites`/`descriptor`/`qd_score` | `theory::evolve::{Cell, behavior_cell, EvolutionResult.qd_score}` |
| `perturbation`/stability | `theory::perturbation_robustness` (already used in `theory_evolve.rs`) |

## The crux (highest-risk design decision): the observable fixture
The legacy `forward_map` **fabricated** `h0_local` and `s8` from the `delta_h0_local` /
`s8_suppression` knobs — exactly the gray-box fudge the new whitebox engine is built to forbid.
The `BackgroundForwardModel` maps `h0_local → h0` (can't reach SH0ES 73 without a real
mechanism) and *omits* `s8` (growth not yet modeled). So migration is **not** a like-for-like
port of the tension-resolution mechanism — it is a deliberate move from *fabricated* tension
observables to *derived* ones. **Decision: migrate the live fixture to background-derivable
observables** (DESI BAO + BBN + CMB distance priors, already used in the `theory evolve` tests)
and let the α-basis / CPL (`w0,wa`) / `n_eff` knobs carry the modified-gravity / early-universe
signal. Settle this first; everything else is mechanical adapter-and-swap.

## Capabilities to port (load-bearing gaps in the new engine)
- **(A) Fixture/coverage** — above. Resolve before any wiring.
- **(B) Co-evolving adversary** — escalation + honesty rollback (`ANCHOR_SURVIVAL_FLOOR`). New
  `evolve()` is a static MAP-Elites loop. Port to a `theory::adversary` module operating on
  `CandidateAssessment` (subtract a `frontier_margin` from `combined_fitness` before binning).
- **(C) Anchor/decoy calibration set** (`ZYAL/anchors/*.json`) — re-author in the new schema (or
  an adapter parsing the legacy `TheoryArtifact` schema → `Theory`). Decoys must still die for the
  same reasons (`flip_to_quintic_decoy`, `inject_free_parameter` give two generators).
- **(D) Live jnoccio critic** — rebuild the `critique.rs` prompt from `Theory` (not `Genes`);
  keep the subordination contract (live can only *lower* a post-veto score, transport failure =
  no-op).
- **(E) Ledgers/artifacts** — `hybrid_artifacts.rs` writes ~10 JSONL ledgers + `quality-gate.json`
  + `map-elites-archive.json`, consumed by `run_summary.rs`/`quality_gate.rs`/`markdown.rs`/schema.
  Swap *behind* the ledger-writing code, preserving the `scores.physics`/`scores.judge`/`scores.live`
  JSON block keys.
- **(F) Island model / lineage / parent inheritance** — retained around the new per-candidate
  `assess`; parent inheritance reads a serialized `Theory`.
- **(G) `Theory` Serde** — blocker for C/E/F: add `Serialize`/`Deserialize` to `Theory` & friends
  (Serde repr for `Provenance`). First code change.

## Staged sequence
- **Stage 0 (enablers, no behavior change):** add Serde to `Theory`; add `theory::adversary`
  (escalation/rollback); add an anchor loader + re-authored anchor set; **land the fixture
  decision (A)** with a calibration test.
- **Stage 1 (adapter, off by default):** `zyal_genome/theory_adapter.rs` exposing the shapes
  `hybrid_artifacts.rs` consumes, backed by core; guard behind `ZYAL_ENGINE=theory` (default
  `legacy`).
- **Stage 2 (swap behind flag):** run `tools/zyal-robustness-run.sh` with `ZYAL_ENGINE=theory`;
  diff ledgers / `quality-gate.json` / `map-elites-archive.json` vs a legacy run for schema
  validity + gate pass + name-independence. Highest-risk verification.
- **Stage 3 (flip default):** `ZYAL_ENGINE=theory` default; keep legacy reachable one cycle; run
  the jankurai ratchet across the flip.
- **Stage 4 (delete):** remove `zyal_robustness.rs`, `zyal_judge.rs`, the flag branch, legacy
  anchors; drop the `pub mod` lines from `lib.rs`.

## Highest-risk steps (ranked)
1. The observable-fixture/coverage gap (A) — resolve in Stage 0.
2. Anchor re-authoring (C) — a mis-ported decoy silently weakens the quality gate.
3. Adversary escalation parity (B) — wrong anti-saturation curve flips the gate's cluster checks.
4. Live-critic prompt rebuild (D) — wrong prompt shape silently degrades to no-op verdicts.

## Guard tests for the migration
`Theory` Serde round-trip; adapter reproducibility + name-independence; `scores.*` JSON blocks
still validate; anchor calibration (survivors survive the escalated adversary, decoys → 0);
honesty-rollback parity; anti-saturation gate parity; coverage==1.0 on the chosen live fixture;
end-to-end `emit_hybrid_evolution_artifacts` smoke with `ZYAL_ENGINE=theory` → gate passes.
