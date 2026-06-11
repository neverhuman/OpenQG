# OpenQG / ZYAL — next-level engineering review (spec S05 of 12)

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
parallel (S01–S12); yours is S05. The team is explicitly willing to make profound changes under the
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

## Your lens: data critic / survey scientist — exactly which data, from exactly where, and why

The engine currently scores against 23 observables (read `data/fixtures/cosmology/*.jsonl` and the
covariance blocks — that IS the entire evidence base). That is enough to build honest machinery and
nowhere near enough for a profound claim. Your charter — every row sourced, every claim concrete:

1. **The acquisition table.** For EACH candidate dataset, give: exact source (URL and/or DOI),
   data format, covariance availability (full / compressed / none), license/usage terms,
   approximate size, ingestion effort (S/M/L), and — decisively — WHAT VERDICT IT CHANGES for the
   current suppressed-growth / dark-scattering champion class. Cover at minimum: Pantheon+ SNe
   with full stat+sys covariance (and Union3, DES-SN5YR as cross-checks); DESI DR2 BAO; full-shape
   RSD (eBOSS / DESI full-shape); KiDS-Legacy, DES Y6, HSC Y3 lensing; ACT DR6 lensing and
   compressed CMB likelihoods; Planck PR4 / plik-lite or equivalent compressed vectors; updated
   BBN (PryMordial-grade inputs); TDCOSMO time delays; GW standard sirens; and anything you judge
   we have missed. Flag anything near your knowledge cutoff with a confidence note.
2. **Priority and the kill question.** Rank the top 3 by verdict-sharpening per unit effort for
   the CURRENT champion class, and separately state which datasets are REQUIRED before an
   exclusion statement ("this mechanism class is ruled out at stated strength") is defensible.
   A suppressed-growth mechanism lives or dies on growth data — be specific about which fσ8 /
   full-shape / lensing combination is decisive and why.
3. **Ingestion spec.** Concrete extension of the existing discipline: `data/registry/*.yml`
   manifest schema entries, fixture JSONL rows with canonicalized observable names (the
   canonicalizer in `cosmology/observables.rs` folds e.g. fsigma8_z051 → fsigma8@0.51 — follow
   it), covariance block format per `data/fixtures/cosmology/covariance/`, provenance hashing per
   the data-lock discipline, and per-dataset validation tests.
4. **The contamination problem, head-on.** Every dataset above is public, and the proposer
   ensemble has memorized the headline values (an LLM knows H0 = 73.04 from SH0ES). What does
   honest generalization MEAN here? Spec the sealed kill-dataset / holdout registry: pre-registered
   and hash-sealed before any proposal sees it, opened only at adjudication; rules for
   future-release pre-positioning (register the observable names and the entry NOW for DESI DR3 /
   Euclid DR1 / Rubin, score when published — the only data an LLM cannot have memorized);
   and the firewall between mechanism knowledge (allowed) and value knowledge (priced or banned).
5. **Compressed vs full likelihoods.** When distance priors / compressed vectors are scientifically
   adequate vs misleading for MG (the 8.4σ ℓ_A episode is the cautionary tale — the compressed
   pipeline itself was the instrument error); criteria for when a sector must graduate to a full
   likelihood, and what that requires of the forward model (see S08, no dependency).
6. **What NOT to ingest.** Datasets that would add noise, double-count (shared calibration or
   overlapping volumes — e.g. SH0ES H0 vs Pantheon+ absolute calibration), or import systematics
   the team cannot model; the correlation/double-counting audit for the CURRENT 23 observables.

Read first, in order: `data/fixtures/cosmology/` (all files incl. covariance/),
`data/registry/` (all), `crates/openqg-core/src/scoring/covariance.rs`,
`crates/openqg-core/src/cosmology/observables.rs`, `paper/main.tex` (data table + limitations),
`docs/zyal-next-level-design.md`.

## Output contract (READ CAREFULLY — this determines whether your work is captured)

Produce your spec as a SINGLE Markdown file named exactly `spec-S05-data-acquisition.md`, and
return it as the ONE downloadable `.tar.gz` artifact you attach — the file at the archive root; no
project folder, no other files, no code archive, no patch. (If tarring fails, attaching the bare
`.md` is the accepted fallback.)

The spec must be a COMPREHENSIVE, PRIORITIZED engineering specification, 2,500–6,000 words:
1. Open with a RANKED BACKLOG — highest-leverage change first — each item stating: what, why it
   matters, rough effort (S/M/L), and how to verify it (an acceptance test a hostile reviewer
   would accept).
2. Then detailed design sections answering the charter — concrete interfaces, data structures,
   pseudocode, named datasets with URLs/DOIs, and literature citations (arXiv IDs where possible).
   Cite the repo files and claims you respond to.
3. Where your spec touches a sibling topic (S01–S12), write "see SXX" and keep your own spec
   self-contained — do not depend on another spec's output.
4. Close with a mandatory section titled **"What we got wrong"** — the strongest claims in the
   attached materials you believe are mistaken, oversold, or self-deceiving, each with the concrete
   check that would settle it.

Depth over breadth where they conflict. Every deficiency you assert must come with the fix and its
verification.
