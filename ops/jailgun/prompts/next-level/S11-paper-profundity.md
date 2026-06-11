# OpenQG / ZYAL — next-level engineering review (spec S11 of 12)

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
parallel (S01–S12); yours is S11. The team is explicitly willing to make profound changes under the
hood — and to RUN the additional computations your verdict requires.

The full curated source is **attached to this conversation as `source.tar.gz`**. FIRST extract it
in your code sandbox (`tar xzf source.tar.gz`) and read the ACTUAL file contents — do not review
from filenames or prior knowledge. If the archive is genuinely unavailable, say so explicitly in
your output and stop; DO NOT fabricate a review of files you have not read.

Platitudes are worthless. The goal is a conclusion that ends up PROFOUND because it is defensible —
your job is to define exactly what stands between the current paper and that.

## Your lens: journal editor / hostile referee — the path from respectable to profound

You are a journal editor and career referee who has rejected better papers than this one. You know
the difference between a respectable engineering report and a result people cite for a decade.
Read `paper/main.tex` IN FULL before anything else.

1. **Contribution class and venue.** What is the actual contribution — a cosmology result, an
   autonomous-science methodology, or both? Which venue does the strongest version belong to
   (PRD? JCAP? Nature Astronomy? NeurIPS/ICML for the methodology?), and describe THAT venue's
   strongest possible version of this paper.
2. **Rank the products by profundity.** The conclusion currently sells four: (i) the
   truth-binding/evidence architecture, (ii) the exploit→regression-test audit cadence, (iii) the
   evidence-valley-was-instrument-bias resolution, (iv) the honest negative champion. Rank them by
   what an OUTSIDE reader would cite, and name what is missing that would lift each — e.g.
   time-to-invalidate proposed as a field-standard metric for autonomous science; the audit
   cascade generalized into a transferable result about evaluator bias; a head-to-head against
   named human MG-scan pipelines.
3. **Convert the null into strength.** Spec the additional computation that turns "V7 scored 43.0
   and does not clear the bar" into "**we exclude this mechanism class at stated strength over the
   declared search volume**": the class definition, the exhaustiveness argument the search must
   support, the statistical prerequisites (goodness-of-fit, rank stability — see S07, no
   dependency), and the mandatory caveats. Write the exclusion sentence as it should appear in the
   abstract.
4. **Position against the AI-for-science wave, with citations.** FunSearch, AlphaEvolve,
   AI-Descartes / AI-Hilbert, The AI Scientist (Sakana), Coscientist (Boiko et al.), the Robot
   Scientist lineage (Adam/Eve). What claim can this project make that NONE of them can? Write
   that claim as a single defensible sentence — then defend it against the obvious attacks
   (n=23 observables; fitting-formula oracle; no confirmed discovery).
5. **The rewrite spec.** Concrete abstract + conclusion rewrite: claims to add, drop, or
   strengthen, each tied to an artifact or computation that must EXIST FIRST, with an explicit
   dependency list into the other eleven specs of this series (reference as "requires SXX-class
   work" — do not depend on their content).
6. **The headline experiment.** Define the single 12-month experiment that would make the result
   undeniable: "the engine pre-registered prediction X for release Y; outcome Z" — choose X and Y
   concretely (DESI DR3? Euclid DR1?), spec the pre-registration mechanics (hash-sealed,
   third-party timestamped), the decision tree for each outcome, and what must be built starting
   now for the registration to be credible.

Read first, in order: `paper/main.tex` (ALL of it), `paper/refs.bib`, `docs/V6-ACCEPTANCE.md`,
`MISSION.md`, `docs/MOONSHOT.md`, `docs/zyal-next-level-design.md`,
`paper/data/story.json` + `paper/data/predictions.txt`.

## Output contract (READ CAREFULLY — this determines whether your work is captured)

Produce your spec as a SINGLE Markdown file named exactly `spec-S11-paper-profundity.md`, and
return it as the ONE downloadable `.tar.gz` artifact you attach — the file at the archive root; no
project folder, no other files, no code archive, no patch. (If tarring fails, attaching the bare
`.md` is the accepted fallback.)

The spec must be a COMPREHENSIVE, PRIORITIZED specification, 2,500–6,000 words:
1. Open with a RANKED BACKLOG — highest-leverage change first — each item stating: what, why it
   matters, rough effort (S/M/L), and how to verify it.
2. Then detailed sections answering the charter — the rewritten abstract/conclusion passages
   verbatim where chartered, venue analysis, the exclusion-statement spec, the headline-experiment
   protocol. Cite the repo files and claims you respond to (with literature arXiv IDs).
3. Where your spec touches a sibling topic (S01–S12), write "see SXX" and keep your own spec
   self-contained — do not depend on another spec's output.
4. Close with a mandatory section titled **"What we got wrong"** — the strongest claims in the
   attached materials you believe are mistaken, oversold, or self-deceiving, each with the concrete
   check that would settle it.

Depth over breadth where they conflict. Every deficiency you assert must come with the fix and its
verification.
