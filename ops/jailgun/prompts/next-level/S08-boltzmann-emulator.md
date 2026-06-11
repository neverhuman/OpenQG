# OpenQG / ZYAL — next-level engineering review (spec S08 of 12)

You are one of twelve independent, world-class reviewers. Attached (as a source tarball you must
read in full) is a curated snapshot of OpenQG: an evidence-gated, "whitebox", derived-not-fit
autonomous theory-discovery engine for cosmology and gravity, plus its ZYAL orchestration layer.
LLM proposers generate symbolic modified-gravity theories; a deterministic Rust oracle verifies
derivation certificates against a registry of closed-form relations, truth-binds verified claims
into a real FLRW forward model (background + growth), scores them against real data (DESI DR1 BAO
with covariance blocks, Planck distance priors, RSD fσ8, SH0ES H0, KiDS S8, BBN) under a veto-first
100-point rubric, and evolves populations under a co-evolving adversary. The LLM proposes and
critiques; it NEVER judges — a deterministic host always has the final say.

The honest history is the project's proudest asset: every era's champion was audited to
destruction. V4 scored 87.5 → killed (rediscovery/tie-credit exploits). V5 scored 55.0 → killed
(free background drift, bare screening). V6 scored 77.0 → killed (its novel-prediction witness was
riding an 8.4σ bias in the oracle's OWN CMB fitting formula — the engine found that bias itself).
The V6 survivor fell 60→25 (a costless certified β with no generating brane term). The V7 standing
champion class scores 43.0 — suppressed-growth / dark-scattering phenomenology — an HONEST NEGATIVE
that does not yet clear the evidence bar. Every exploit became a permanent regression test. Every
degree of freedom is priced; data fit is one-sided (a tie with ΛCDM scores zero); novelty must be
mechanism-attributable.

This review event is the design gate for the FINAL phase (V8). Twelve specs are commissioned in
parallel (S01–S12); yours is S08. The team is explicitly willing to make profound changes under the
hood: if the right answer requires a technology ZYAL does not have today — a computer algebra
system, a Boltzmann backend, a proof assistant, a symbolic-regression engine, a literature corpus,
token telemetry — SPEC IT AS A HARD REQUIREMENT with the engineering detail to build it. Do not
soften your demands to fit the current architecture.

The full curated source is **attached to this conversation as `source.tar.gz`**. FIRST extract it
in your code sandbox (`tar xzf source.tar.gz`) and read the ACTUAL file contents — do not review
from filenames or prior knowledge. If the archive is genuinely unavailable, say so explicitly in
your output and stop; DO NOT fabricate a review of files you have not read.

Calibration: read `docs/zyal-next-level-design.md` (the team's own stern internal critique) and the
limitations section of `paper/main.tex` first — your job is to go BEYOND what the team already
knows. Platitudes are worthless; interfaces, data structures, pseudocode, named algorithms, and
acceptance tests are the currency. The goal is a result that ends up PROFOUND because it is
defensible.

## Your lens: computational-cosmology engineer — forward-model fidelity without losing the loop

The forward model is fitting-formula grade (`cosmology/{forward,background,growth}.rs`): real FLRW
background + growth integration, but no Boltzmann hierarchy; CMB enters via distance priors and a
fitting formula that has already produced one 8.4σ instrument bias. The paper names "calibration
residual envelope + Boltzmann backend" as the top next step (`docs/boltzmann-backend.md` records
the team's current thinking). The evolution loop needs ~10²–10⁴ forward evaluations per generation,
so fidelity cannot simply replace speed. Spec the production design:

1. **The options matrix.** CLASS / CAMB subprocess; hi_class / EFTCAMB / MGCAMB for modified
   gravity; trained emulators (CosmoPower and successors, Capse.jl-style, or self-trained on our
   own theory space); a native-Rust reimplementation of the minimal needed subset. For each:
   accuracy on our observables, per-evaluation latency, MG coverage (μ/Σ phenomenology? α-basis?
   arbitrary term-algebra output — see S03, interface only), maintenance burden, determinism, and
   license. Recommend a configuration with numbers, not vibes.
2. **The calibration residual envelope.** Spec it precisely: per-observable systematic error
   floors derived from fitting-formula-vs-Boltzmann differences SAMPLED ACROSS THE SEARCHED THEORY
   SPACE (not just at ΛCDM — the envelope must cover where champions actually live), folded into
   the likelihood as additional (co)variance; the sampling design, refresh policy, and the test
   that the ℓ_A-class bias would have been caught by it.
3. **Architecture.** Production design for an external solver in a deterministic, replayable
   engine: process pool with warm workers, request/response schema, caching keyed by (theory
   fingerprint, parameter vector, solver version), bit-level determinism strategy (pinned solver
   versions and compiler flags; documented tolerance on replay), failure semantics (a solver crash
   must not silently zero a likelihood), and the reproducibility contract extension (ledger records
   solver version + settings hash per evaluation).
4. **Tiered fidelity.** Cheap fitting formulas for evolution-loop triage; full Boltzmann for
   adjudication, league fits, and champion promotion. Spec the promotion pipeline, the
   tier-consistency check (candidates whose triage and adjudication scores diverge beyond the
   envelope are flagged as instrument-risk, not just rescored), and the cache economics.
5. **What it unlocks.** With a Boltzmann backend: full CMB TT/TE/EE likelihoods (or plik-lite
   compressed), CMB lensing, P(k)/full-shape — for each, the accuracy requirement, the runtime
   cost, and the verdict impact on the suppressed-growth champion class (growth-sector data is
   where it lives or dies). Coordinate conceptually with data acquisition (S05) without depending
   on it.
6. **Acceptance tests.** Reproduce published Planck ΛCDM χ² through the new path; match published
   nDGP and f(R) fσ8(z) curves to stated tolerance; demonstrate the envelope catches a planted
   fitting-formula bias of the ℓ_A class; end-to-end determinism replay test.

Read first, in order: `docs/boltzmann-backend.md`, `crates/openqg-core/src/cosmology/forward.rs`,
`crates/openqg-core/src/cosmology/background.rs`, `crates/openqg-core/src/cosmology/growth.rs`,
`crates/openqg-core/src/cosmology/observables.rs`, `paper/main.tex` (degeneracy-valley +
limitations sections), `docs/zyal-next-level-design.md`.

## Output contract (READ CAREFULLY — this determines whether your work is captured)

Produce your spec as a SINGLE Markdown file named exactly `spec-S08-boltzmann-emulator.md`, and
return it as the ONE downloadable `.tar.gz` artifact you attach — the file at the archive root; no
project folder, no other files, no code archive, no patch. (If tarring fails, attaching the bare
`.md` is the accepted fallback.)

The spec must be a COMPREHENSIVE, PRIORITIZED engineering specification, 2,500–6,000 words:
1. Open with a RANKED BACKLOG — highest-leverage change first — each item stating: what, why it
   matters, rough effort (S/M/L), and how to verify it (an acceptance test a hostile reviewer
   would accept).
2. Then detailed design sections answering the charter — concrete interfaces, data structures,
   pseudocode, named tools/libraries with versions, and literature citations (arXiv IDs where
   possible). Cite the repo files and claims you respond to.
3. Where your spec touches a sibling topic (S01–S12), write "see SXX" and keep your own spec
   self-contained — do not depend on another spec's output.
4. Close with a mandatory section titled **"What we got wrong"** — the strongest claims in the
   attached materials you believe are mistaken, oversold, or self-deceiving, each with the concrete
   check that would settle it.

Depth over breadth where they conflict. Every deficiency you assert must come with the fix and its
verification.
