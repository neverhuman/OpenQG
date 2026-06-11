# OpenQG / ZYAL — next-level engineering review (spec S07 of 12)

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
parallel (S01–S12); yours is S07. The team is explicitly willing to make profound changes under the
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

## Your lens: statistics critic — make every evidence number referee-proof

The evidence path today: covariance-aware Gaussian likelihood (`scoring/covariance.rs`,
`scoring/likelihood.rs`), Nelder–Mead profile fits, and Schwarz/BIC-proxy evidence
ln Z ≈ −½·BIC vs ΛCDM (`theory/league.rs`); an implemented-but-unused `laplace_log_evidence`
sits in `scoring/evidence.rs`. The headline claims of this project are statistical claims. Your
charter — make them survive a hostile statistics referee:

1. **Where the current path fails a referee.** Audit concretely: when does the Schwarz proxy
   diverge badly from real evidence at n=23 observables and k≤4 parameters (prior sensitivity,
   non-Gaussian posteriors, boundary-pinned parameters — the `boundary_hit` flag exists for a
   reason)? Quantify on the engine's ACTUAL champion class, not in generalities.
2. **The upgrade spec.** Nested sampling as the evidence backbone: compare dynesty / UltraNest /
   PolyChord (and a pure-Rust option if credible) for this problem size; spec the integration
   (process boundary, determinism/seeding, runtime budget per league adjudication), prior
   specification discipline (WHO sets priors for an auto-generated theory, and how prior volume is
   priced so evidence cannot be gamed by prior choice — this is an anti-cheating surface), and
   evidence error bars as first-class outputs.
3. **Profile likelihood alongside Bayesian.** Both have constituencies; spec running both, when
   they disagree on our actual data, and which one gates promotion. Treat the boundary_hit problem
   properly (profile likelihoods at prior boundaries are a known failure mode of cosmology
   pipelines — cite the literature).
4. **Goodness-of-fit gates.** A candidate can beat ΛCDM on ΔlnZ while both fit terribly. Spec the
   GoF battery: χ²/dof with the covariance structure respected, posterior-predictive checks,
   per-sector pulls — and the gate thresholds at which "beats ΛCDM" is not even reportable.
5. **Selection effects / look-elsewhere.** Hundreds of proposals have been scored against the SAME
   23 observables; the champion's ΔlnZ is the maximum of a search. Spec the correction: trials
   accounting, stability of rank under bootstrap/jackknife of the data vector, and the sealed
   replication protocol (champion's evidence recomputed on data it never touched — coordinate
   with the holdout design in `theory/holdout.rs`, which you should audit for soundness:
   alternating holdout at n=23 — is the generalization-gap statistic meaningful at all?).
6. **The n=23 reality check.** Be brutally honest about what claims are POSSIBLE at this sample
   size, what minimum data scale the desired claims require (feed the requirements to S05 — no
   dependency, just state them), and which currently published numbers in `paper/main.tex` are
   statistically overdressed for n=23.
7. **The exclusion statement machinery.** The team wants to convert the honest null into "we
   exclude this mechanism class at stated strength over the searched volume". Spec exactly the
   statistical machinery that makes such a sentence defensible: the class definition, the search
   volume measure, the strength statistic, and its caveats. Also: finish-or-delete verdict on
   `laplace_log_evidence`, with the spec for whichever you choose.

Read first, in order: `crates/openqg-core/src/theory/league.rs`,
`crates/openqg-core/src/scoring/covariance.rs`, `crates/openqg-core/src/scoring/likelihood.rs`,
`crates/openqg-core/src/scoring/evidence.rs`, `crates/openqg-core/src/theory/holdout.rs`,
`crates/openqg-core/src/theory/scorecard.rs` (DataFitOutcome), `paper/main.tex`.

## Output contract (READ CAREFULLY — this determines whether your work is captured)

Produce your spec as a SINGLE Markdown file named exactly `spec-S07-statistical-evidence.md`, and
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
