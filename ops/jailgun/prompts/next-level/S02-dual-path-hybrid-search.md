# OpenQG / ZYAL — next-level engineering review (spec S02 of 12)

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
parallel (S01–S12); yours is S02. The team is explicitly willing to make profound changes under the
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

## Your lens: AI/ML search architect — the dual path, and where the two paths meet

Today the search is TOP-DOWN ONLY: an LLM proposes a complete symbolic theory and the deterministic
oracle judges it. The team's conviction, which you must turn into engineering: physics discovery is
a TWO-WAY street. Smart theories can evolve DOWNWARD toward data hoping to connect — but data can
also co-evolve UPWARD via genetic programming / symbolic regression over KNOWN physical variables
and coefficients, with the two paths meeting in the middle. At one extreme the answer is found with
0% intelligence (brute-force GP); at the other with 100% intelligence and no derivation. Spec the
hybrid that beats both:

1. **The bottom-up track.** A symbolic-regression pipeline over residuals vs ΛCDM in the engine's
   actual observable space (23 observables: BAO ratios, fσ8(z), H0, S8, CMB distance priors, BBN).
   Primitive set restricted to KNOWN physical variables and dimensionless coefficients: a, z, H(a),
   Ω_m(a), Ω_DE(a), w(a), k/k_*, … Choose and justify the engine (PySR, Operon, a pure-Rust GP, or
   FunSearch/AlphaEvolve-style LLM-guided program search), the complexity measure, and the
   accuracy–complexity Pareto archive. State the compute budget honestly.
2. **The midpoint representation.** Define the shared algebra into which BOTH paths emit — a term
   grammar / μ(a,k), Σ(a,k) functional space / claim graph — such that a GP-discovered
   phenomenological form and an LLM-derived theory are directly comparable, diffable, and
   MATCHABLE ("this GP form is the quasi-static limit of that Horndeski term"). This is the
   load-bearing design decision; give it data structures (see S03 for the term algebra — design
   the interface, not their internals).
3. **The meet-in-the-middle protocol.** Lifecycle spec: a GP form that fits enters the population
   as a PHENOMENOLOGICAL CONTENDER with zero derivation rigor, quarantined from the whitebox
   scoreboard; LLM agents are then tasked to DERIVE it (attach obligations/certificates per the
   existing machinery) — "explanation bounties". Conversely, theory-side proposals emit predicted
   functional forms that re-seed the GP primitive set. Define the promotion rules, the scoring of
   underived-but-fitting forms WITHOUT violating the whitebox gates, and the credit assignment
   when a derivation lands.
4. **The dual-path analysis, quantitatively.** What can 0%-intelligence brute force actually find
   here? Estimate the search-space and identifiability limits: with background+growth data only,
   which MG signatures are even distinguishable (μ–Σ degeneracies, the documented h–Ωm valley)?
   What does 100%-intelligence-no-derivation uniquely contribute (priors over physically sensible
   forms)? Where is the crossover, and what does the COMBINATION unlock that neither has alone?
   Engage the literature by name: FunSearch (Romera-Paredes et al. 2024), AlphaEvolve, AI-Descartes
   / AI-Hilbert (axiom-constrained symbolic regression — closest prior art to this whole project),
   AI Feynman, Cranmer's PySR work, quality-diversity (MAP-Elites).
5. **Population design.** Is the current 6-island model the right scaffold? Spec a
   quality-diversity archive whose behavior descriptors are physical (which observable pulls a
   candidate addresses, which mechanism family, screening behavior) so the engine maintains a
   diverse frontier instead of converging on one basin.
6. **Anti-cheating for the GP track.** GP is the ultimate overfitter and it will rediscover the
   data's noise. Extend the sealed-holdout / kill-dataset discipline and complexity pricing to the
   bottom-up track; define the GP-specific gaming modes and their kill rules.
7. **Pipeline + acceptance tests.** End-to-end: residual extraction → GP search → midpoint
   emission → explanation bounty → certified promotion. Acceptance: the pipeline, run on synthetic
   data generated from a KNOWN MG theory, recovers that theory's functional form bottom-up AND the
   derivation top-down — the closed loop demonstrated.

Read first, in order: `docs/ZYAL.md`, `crates/openqg-bench/src/zyal_genome/theory_population.rs`,
`crates/openqg-bench/src/zyal_genome/proposer_sketch.rs`,
`crates/openqg-core/src/theory/scorecard.rs`, `crates/openqg-core/src/theory/holdout.rs`,
`crates/openqg-core/src/theory/contenders.rs`, `docs/zyal-next-level-design.md`.

## Output contract (READ CAREFULLY — this determines whether your work is captured)

Produce your spec as a SINGLE Markdown file named exactly `spec-S02-dual-path-hybrid-search.md`,
and return it as the ONE downloadable `.tar.gz` artifact you attach — the file at the archive root;
no project folder, no other files, no code archive, no patch. (If tarring fails, attaching the bare
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
