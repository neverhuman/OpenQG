# OpenQG / ZYAL — next-level engineering review (spec S03 of 12)

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
parallel (S01–S12); yours is S03. The team is explicitly willing to make profound changes under the
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

## Your lens: theoretical physicist (EFT of dark energy) — replace the allowlist with an algebra

Today the searchable theory space is an ALLOWLIST: 9 closed-form certificate relations
(`certificate.rs`) plus a V7 term registry that demands bound MG families have generating terms
(`proposer_sketch.rs`). That stops invented terms, but it also caps discovery at recombinations of
relations the team hand-picked. The paper itself names "term algebra replacing registry allowlist"
as the way out. Spec it:

1. **The generative grammar.** A Lagrangian-level term algebra from which the searchable space is
   GENERATED rather than enumerated: Horndeski G2–G5 functions and/or the EFT-of-DE α-basis
   (α_M, α_B, α_K, α_T) as generators; state your position on beyond-Horndeski/DHOST and on
   non-scalar-tensor escapes (vector/massive gravity) — included, excluded, why. Define
   well-formedness rules (mass dimension, symmetry, locality, parity) enforced by construction.
2. **The compilation chain, mechanically.** term set → background E(z) → linear perturbations →
   μ(a,k), Σ(a,k), c_s²(a), q_s(a) → observables. Specify exactly what is computed symbolically
   (and with which CAS) vs numerically, where the quasi-static approximation enters and how its
   validity is CHECKED per theory rather than assumed, and how ghost/gradient stability becomes a
   COMPUTED property of the term set instead of an asserted flag. Cite the literature you are
   compiling from: Bellini & Sawicki 2014, the hi_class and EFTCAMB papers, Zumalacárregui et al.,
   Lagos et al. — with arXiv IDs.
3. **Closure and correctness.** The regression suite that proves the compiler: for nDGP, f(R),
   quintessence, coupled DE, k-essence, reproduce the KNOWN published μ(a,k)/Σ(a,k) limits and
   growth results to stated tolerance. Every certificate relation in today's registry must fall
   out as a THEOREM of the algebra — show the mapping for all 9.
4. **What dies and what is unlocked.** The exploit history is full of "certified relation with no
   generating mechanism" (the V6 survivor's costless β). Show structurally why the algebra makes
   that class unrepresentable, and quantify the unlocked space: roughly how many physically
   distinct mechanism families become searchable that the allowlist excludes today?
5. **Screening, computed.** Vainshtein and chameleon screening as computable properties of the
   term set in the quasi-static limit (not declared strings, which V5 exploited): the algorithm,
   its inputs, the PPN/Cassini check it feeds, and its failure modes.
6. **Interfaces.** The algebra is the natural candidate for the "midpoint representation" the
   dual-path search needs (see S02) and the natural compilation target for a Boltzmann backend
   (see S08). Define the public interface both would consume — without depending on their specs.
7. **Migration + cost.** Stepwise plan from registry to algebra (registry relations as algebra
   theorems first, generative proposals second), proposer-prompt changes, what it does to the
   parse/repair economics, and the acceptance gate for switching the scorecard's rigor dimension
   onto algebra-derived theories.

Read first, in order: `crates/openqg-bench/src/zyal_genome/proposer_sketch.rs` (term registry),
`crates/openqg-core/src/theory/vetoes.rs`, `crates/openqg-core/src/theory/binding.rs`,
`crates/openqg-core/src/cosmology/background.rs`, `crates/openqg-core/src/cosmology/growth.rs`,
`paper/main.tex`, `docs/zyal-next-level-design.md`.

## Output contract (READ CAREFULLY — this determines whether your work is captured)

Produce your spec as a SINGLE Markdown file named exactly `spec-S03-term-algebra.md`, and return it
as the ONE downloadable `.tar.gz` artifact you attach — the file at the archive root; no project
folder, no other files, no code archive, no patch. (If tarring fails, attaching the bare `.md` is
the accepted fallback.)

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
