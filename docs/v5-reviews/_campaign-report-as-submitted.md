# OpenQG / ZYAL V5 — Campaign Report: results, issues, and insights

This is the engineering team's own honest account of the V5 build + live campaign (2026-06-10/11),
written before requesting external review. Verify every claim against the attached code and
`run-data/` ledgers.

## The numbers (V4.1 → V5)

| Metric | V4.1 | V5 |
|---|---|---|
| Live-slot yield | 4% (2/50 browser-ChatGPT calls parsed) | **~88% (37/42 jekko slots produced a scored proposal)** |
| Parse failures | ~90% of calls | **<1% (2/276 calls)** |
| Proposals aimed at the data tensions | 0 | **49/49 — every live proposal was a suppressed-growth theory** |
| Best genuine data credit | 0 (every candidate tied or trailed ΛCDM) | **data_fit 20/20, Δln Z ≈ +55 vs Planck-ΛCDM** |
| Generations completed | 2000 (1 run) | **1800 (6×300 chunks, all completed despite a hostile host)** |
| Replay integrity | 0 mismatches | **0 mismatches, every run** |

## What V5 changed (all in the attached code)

1. **Truth-binding** (`crates/openqg-core/src/theory/binding.rs`): a verified modified-gravity
   certificate is BOUND into the background the forward model integrates (nDGP β→Ω_rc closed-form
   inversion; f(R) tracking inversion; Planck-μ0 direct). Three new kill classes:
   `UnimplementedModification`, `UnexplainedModification`, `ConflictingModification`.
   Claims now have computable consequences — the V4.1 champion's "certified geff=7/6 but the model
   computes plain ΛCDM" arbitrage is dead.
2. **Machine-computed novelty** (`binding.rs::audit_novel_predictions` + `scorecard.rs`): a
   novel-prediction witness's declared numbers are checked against the model-computed values for the
   bound background, within the witness's own `min_detectable`. Distinctness and honesty are
   computed, never declared; a dishonest declaration demotes novelty to 0 and is flagged on record.
3. **The suppressed-growth vocabulary** (`certificate.rs`): `planck_mu0_geff` (negative μ0 allowed,
   rigor 0.3 — a cited parametrization, not a mechanism) and `ndgp_beta_from_omega_rc` (rigor 1.0).
   Honest motivation: every pre-existing registry relation ENHANCES growth (geff ≥ 1) while the
   admitted fσ8/S8 data sit LOW — the old vocabulary could not express a tension-resolving theory.
4. **Jekko at scale** (`crates/openqg-bench/src/zyal_genome/proposer_jekko.rs`): high-volume
   proposals on the free jnoccio API with parse-repair + oracle-repair loops (the oracle's
   kill_reasons are the teacher), best-of-K parallel sampling, and an injected transport for
   no-network tests.
5. **Total observability** (`theory_population.rs::ProposalAttemptRecord`, `ledger_sink.rs`): every
   LLM call/repair/failure is a ledger record (outcome, error, kill reasons, raw sha, stderr
   excerpt); ledgers and champion checkpoints STREAM during the run (crash-proof); the old silent
   `.ok()?` error swallow is gone.
6. **Compounding memory + computed data brief** (`proposer_memory.rs`): every proposer prompt
   carries (a) a deterministic DATA BRIEF computed from the actual observables (real pulls:
   "h0 +5.4σ ← LARGEST TENSION; growth pulls negative — SUPPRESS"), and (b) a MEMORY section
   assembled from ALL prior run ledgers (top scorers, kill-reason histogram, DO/DON'T lines).

## The two scientific results

### 1. The machine blindly rediscovered cosmology's degeneracy escape-hatch
Six independent seeds (chunks 1–6) all converged to the SAME champion (54.999 ± 0.0001): an evolved
ΛCDM at **h ≈ 0.718, Ω_m ≈ 0.282 with Ω_m·h² held nearly constant** — sliding along the CMB
degeneracy valley to drop the H0 pull from 5.4σ to ~1.2σ and S8 to ~0.6σ simultaneously, with
data_fit maxed (Δln Z ≈ +54.9 nats vs the Planck-ΛCDM baseline). No modified gravity at all
(μ0 = 0). This is precisely the move human cosmologists attempt on the H0 tension, found here by
blind mutation against the admitted data.

**Honest caveat (stated by us before any reviewer says it):** the admitted CMB is the compressed
(R, ℓ_A) pair, which cannot close that valley; the full Planck likelihood almost certainly would.
This champion is a *diagnosis of our evidence set*, not a discovery — and it tells us exactly which
data to admit next (the full CMB distance-prior covariance) to test it.

### 2. The 55-point ceiling is precisely understood
Every chunk's champion decomposes identically: data_fit 20/20 + novelty 10/20 (distinct but no
witness) + robustness 13 + parsimony 12 + derivation 0 + unification 0 ≈ 55. Meanwhile every jekko
proposal decomposes as: novelty 20/20 + unification 15/15 + parsimony 12 + small rigor + small data
≈ 33.7. **The evolved children have the fit but no claim structure; the proposals have the
structure but un-optimized fits.** The two halves of an ~80-point candidate exist in every chunk,
separately. The obvious next capability is a "re-clothe" step: graft a proposal's verified
claim-graph (claims/obligations/witnesses, recomputed against the descendant's bound background)
onto its best evolved descendant.

## Operational issues encountered (and how they were handled)

- **jnoccio provider degradation:** intermittent `http 503: all 131 models unavailable: 102
  missing_key` and many 1-byte empty stdout responses. Mitigations shipped: stderr capture in
  attempt records, `empty_output` classified separately from parse errors with a free retry,
  quality-band routing made configurable (`--jekko-quality-band none` routes across all models).
  Residual: 213/276 calls were empty-output blips, absorbed by retries + best-of-K.
- **Hostile host:** load average 60+; two long campaigns were externally killed (gen 679, gen 159)
  with no panic. Mitigation: streaming ledgers + atomic checkpoints meant zero loss; final design is
  the chunked driver (`ops/v5-chunked-campaign.sh`) — 6×300-gen chunks, each a complete auditable
  run, with the MEMORY section compounding learning across chunks.
- **Over-strict preflight:** the jekko smoke aborted a campaign on ONE transient empty reply; it now
  retries 3× before failing.
- **LLM schema drift:** the model invented `mg_family: "planck_mu0"`; fixed by a lossless
  normalization in the shared repair pass (the μ0 parametrization IS the `none` family).

## What we'd do next (our ordering — challenge it)
1. The **re-clothe step** (structure × fit ⇒ ~80-point candidates).
2. Admit the **full CMB distance-prior covariance** and watch whether the degeneracy champion
   survives — either outcome is real science.
3. **Registry growth toward mechanism-backed suppressed growth** (the Planck-μ0 parametrization is
   honest phenomenology at rigor 0.3; a real mechanism with geff < 1 would change the game).
