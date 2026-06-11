# OpenQG / ZYAL — next-level engineering review (spec S10 of 12)

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
parallel (S01–S12); yours is S10. The team is explicitly willing to make profound changes under the
hood — but YOUR lens is calibration against the outside world, not internal machinery.

The full curated source is **attached to this conversation as `source.tar.gz`**. FIRST extract it
in your code sandbox (`tar xzf source.tar.gz`) and read the ACTUAL file contents — do not review
from filenames or prior knowledge. If the archive is genuinely unavailable, say so explicitly in
your output and stop; DO NOT fabricate a review of files you have not read. (Your lens reads less
code than the others by design — but `paper/main.tex` and the data fixtures you MUST read.)

Platitudes are worthless; numbers, arXiv IDs, named groups and pipelines are the currency. The goal
is a result that ends up PROFOUND because it is defensible.

## Your lens: senior cosmologist — the state of the field, and where this engine honestly stands

You are writing the annual-review chapter on cosmic tensions as of mid-2026: every measurement,
every resolution attempt, who is winning. The team needs this to calibrate its claims (deliverable:
"the best humans have done"). Every claim with citations; flag anything near your knowledge cutoff
with an explicit confidence note, and say what to verify against the live literature.

1. **H0, with numbers.** SH0ES Cepheids; CCHP/JWST TRGB and JAGB; TDCOSMO time delays; megamasers;
   GW sirens; the Planck/ACT/SPT + BAO + BBN inverse ladder. What is the live tension level, what
   changed 2024–2026, and which resolution classes remain viable (early vs late per the "H0
   Olympics" framework of Schöneberg et al.; EDE status after ACT DR6; the CosmoVerse synthesis)?
2. **S8 and growth.** KiDS-Legacy vs DES Y3/Y6 vs Planck: is the S8 tension alive in 2026? DESI
   full-shape fσ8 results; published hints of suppressed growth (e.g. the σ8-tension literature,
   Nguyen/Huterer/Wen-style growth-index fits). Then the key deliverable: score OUR V7
   suppressed-growth / dark-scattering champion class's EXTERNAL plausibility against published
   constraints — does the mechanism class survive data the engine has not yet ingested? Cite the
   dark-scattering literature lineage (Simpson 2010 onward) and its current constraint status.
3. **Evolving dark energy.** The DESI DR1/DR2 w0waCDM story by dataset combination; the
   Pantheon+/Union3/DES-SN5YR pulls and their mutual inconsistencies; the strongest published
   counter-arguments (SN systematics, profile-likelihood critiques); what settles it and when
   (DESI DR3, Euclid, Rubin/LSST).
4. **MG searches — the competition.** Current best constraints on μ–Σ / α-basis phenomenology
   (Planck MG analyses, DES extended-model papers, DESI MG papers); the post-GW170817 viable
   theory space; and NAME the groups and pipelines that run the best systematic MG scans today.
   Then the honest comparison: what does our engine do that they cannot or do not (autonomous
   search breadth? adversarial self-audit? honesty economics?), and where is it simply outclassed
   (likelihood fidelity, data coverage, perturbation theory)?
5. **Positioning verdict, sector by sector.** For each of (1)–(4): what is this engine positioned
   to CONTRIBUTE that the best human teams do not — concretely, this calibrates which claims the
   paper may make (see S11) — and what would embarrass the team if claimed?
6. **The 12–24 month horizon, with dates.** DESI DR3, Euclid DR1, Rubin/LSST first cosmology,
   ACT/SPT successors, LISA-era siren forecasts: which releases should the engine pre-position
   for, and what does pre-positioning mean in engineering terms (sealed prediction-registry
   entries with observable definitions BEFORE release dates — the only un-memorizable test an
   LLM-driven engine can pass)?

Read first, in order: `paper/main.tex` (introduction, results, limitations, conclusion),
`data/fixtures/cosmology/` (all — this is the engine's entire evidence base),
`docs/zyal-next-level-design.md`, `ops/jailgun/next-level-payload/results/` (champion JSONs —
what the engine actually found), `docs/V6-CAMPAIGN-REPORT.md`.

## Output contract (READ CAREFULLY — this determines whether your work is captured)

Produce your spec as a SINGLE Markdown file named exactly `spec-S10-field-state-tensions.md`, and
return it as the ONE downloadable `.tar.gz` artifact you attach — the file at the archive root; no
project folder, no other files, no code archive, no patch. (If tarring fails, attaching the bare
`.md` is the accepted fallback.)

The spec must be a COMPREHENSIVE, PRIORITIZED report, 2,500–6,000 words:
1. Open with a RANKED BACKLOG of actions the field-state implies for this project — each item
   stating: what, why it matters, rough effort (S/M/L), and how to verify it.
2. Then the detailed sections answering the charter — numbers, arXiv IDs, named groups/pipelines,
   explicit confidence flags near your knowledge cutoff. Cite the repo files and claims you
   respond to.
3. Where your report touches a sibling topic (S01–S12), write "see SXX" and keep your own report
   self-contained — do not depend on another spec's output.
4. Close with a mandatory section titled **"What we got wrong"** — the strongest claims in the
   attached materials you believe are mistaken, oversold, or self-deceiving, each with the concrete
   check that would settle it.

Depth over breadth where they conflict. Every deficiency you assert must come with the fix and its
verification.
