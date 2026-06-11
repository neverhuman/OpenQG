# OpenQG / ZYAL — next-level engineering review (spec S09 of 12)

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
parallel (S01–S12); yours is S09. The team is explicitly willing to make profound changes under the
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

## Your lens: AI/ML knowledge-systems critic — agents that study physics and get smarter

Today the proposer ensemble has NO knowledge infrastructure: no literature corpus, no retrieval, no
cross-campaign memory beyond per-generation prompt state (`proposer_memory` exists only as
plumbing). Registry relations cite their sources as code comments. The team wants agents that STUDY
online physics — and a system that measurably gets smarter campaign over campaign. Spec it:

1. **The corpus.** Acquisition pipeline for a physics knowledge base: arXiv bulk access
   (astro-ph.CO, gr-qc, hep-th subsets), INSPIRE-HEP metadata/citations, Living Reviews, review
   articles. Spec: source APIs and licensing reality, equation-aware chunking (LaTeX-native — name
   the parsing/embedding approaches that handle math, not just prose), embedding/index choice,
   corpus size and refresh cadence, and the build cost. Target the lanes that exist: what does the
   dark-scattering lane need in its corpus that the Planck-μ0 lane does not?
2. **Retrieval wired into the pipeline.** Per-lane retrieval policies feeding `proposer_prompt.rs`
   construction: what gets retrieved at proposal time vs repair time vs critique time, token
   budgets, and MANDATORY provenance — every mechanism claim in a proposal carries retrieval IDs
   the ledger records. Spec the schema changes.
3. **Literature-anchored certificates.** A new obligation kind: the cited relation is checked
   against the actual source passage (retrieve → extract the equation → match against the
   certificate relation). And the kill rule this enables: a proposal citing a paper's FITTED
   POSTERIOR VALUE as a derivation input dies (values are data, not derivation — this is the
   laundering channel between literature and certificates).
4. **Cross-campaign learning.** The knowledge ledger: distilled, structured lessons from every
   kill and every survival ("dark-scattering with constant A_drag dies on BAO+growth combination
   at z>1; the surviving variant needed w0 within [-1.05,-0.9]") persisted, indexed, and retrieved
   into future campaign prompts. Spec the schema, the distillation pipeline (who writes the
   lesson — the deterministic host from ledger facts, not the LLM from vibes), and the retrieval
   trigger. Contrast with finetuning (LoRA on repair transcripts; reinforcement from kill
   outcomes): recommend buy/build/skip with rationale.
5. **The contamination firewall.** The corpus CONTAINS the answers to the fit data (every Planck
   and DESI paper states the measured values). Define the firewall between mechanism knowledge
   (allowed, encouraged) and value knowledge (priced or banned): retrieval-content filters
   (numbers stripped? posterior tables redacted?), what the anti-cheating gates must additionally
   check once retrieval exists, and how this interacts with the memorization problem the sealed
   holdouts already face (the proposers know the values WITHOUT retrieval — be honest about what
   the firewall can and cannot achieve).
6. **The pilot, pre-registered.** A 2-week pilot with a control arm (same lanes, no retrieval):
   primary metrics (kill-rate reduction, rigor-score lift, novel-mechanism rate per 100 proposals,
   tokens per surviving proposal), success/failure thresholds DECLARED IN ADVANCE, and the
   decision rule for ship/iterate/kill.

Read first, in order: `crates/openqg-bench/src/zyal_genome/proposer_prompt.rs`,
`crates/openqg-bench/src/zyal_genome/proposer.rs`,
`crates/openqg-bench/src/zyal_genome/proposer_router.rs`,
`crates/openqg-core/src/theory/obligation.rs`, `docs/ZYAL.md`, `docs/zyal-next-level-design.md`.

## Output contract (READ CAREFULLY — this determines whether your work is captured)

Produce your spec as a SINGLE Markdown file named exactly `spec-S09-knowledge-hardening.md`, and
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
