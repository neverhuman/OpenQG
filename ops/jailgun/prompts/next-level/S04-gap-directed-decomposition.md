# OpenQG / ZYAL — next-level engineering review (spec S04 of 12)

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
parallel (S01–S12); yours is S04. The team is explicitly willing to make profound changes under the
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

## Your lens: AI/ML systems critic — theory decomposition, gap attribution, focused reasoning

The team believes its highest-leverage unexploited idea is DECOMPOSITION: break a theory into its
components (relations, parameters, sector bindings, claims), measure exactly where each component
helps or hurts against evidence, identify the BIGGEST gaps, and then point reasoning agents at JUST
those components instead of regenerating whole theories. V7 already took the first step (the
per-relation input-key contract in `proposer_sketch.rs`; a claim graph exists in `claim_graph.rs`).
Turn the idea into machinery:

1. **Component-level evidence attribution.** Spec the ablation engine: knock out / neutralize one
   component of a champion (a relation, a binding, a term), re-run the forward model and scorecard,
   and attribute score and per-observable pull deltas to components. Address interaction effects
   honestly — pairwise ablations or Shapley-style attribution over component subsets, with the
   compute cost stated. Output: a per-champion COMPONENT LEDGER (data structure spec) recording
   each component's contribution to fit, rigor, novelty, and which observables it moves.
2. **The gap dashboard.** From the component ledger, rank components by evidence shortfall (this
   component is why fσ8@0.51 still pulls 2σ), rigor shortfall (weakest certificate in the chain),
   and novelty shortfall. Define the gap metric precisely; this ranking is what the whole engine's
   attention should follow.
3. **Focused re-proposal protocol.** Freeze the rest of the theory; task an agent with ONE weak
   component, giving it only that component's local context: its inputs and input-key contract,
   the observables it drives, the constraint surface from the ablation runs, the failed attempts
   that came before. Spec the prompt contract (extending the V7 per-relation input-key approach),
   the merge-back path (how a partial proposal recombines with the frozen remainder and what must
   be re-verified — the existing recombination machinery is your starting point), and the
   convergence criterion.
4. **Credit assignment across the campaign.** Track which agents/lanes/mutation operators improve
   which component classes; spec bandit-style routing of proposal effort toward high-yield gaps
   (and cite the relevant ML literature — successive halving / UCB over lanes, or better).
5. **Anti-gaming.** Component freezing creates a laundering channel: improvements smuggled through
   "frozen" parts, or a weak component made to look strong by shifting its job onto a sibling.
   Define the kill rules and the invariants the merge-back verifier must check (the V6.1 coherence
   kill is prior art).
6. **Decomposition as audit instrument.** The same machinery, pointed at the ORACLE: which
   scorecard dimensions and which oracle approximations does each champion lean on hardest
   (sensitivity of total score to oracle perturbations)? This generalizes how the 8.4σ ℓ_A bias was
   found — spec it as a standing audit job.
7. **Acceptance tests.** On the V7 champion class: the ledger reproduces hand-derived attribution
   on a known case; a deliberately weakened component is correctly ranked top gap; a focused
   re-proposal on synthetic ground truth recovers the planted fix.

Read first, in order: `crates/openqg-core/src/theory/claim_graph.rs`,
`crates/openqg-bench/src/zyal_genome/proposer_sketch.rs`,
`crates/openqg-core/src/theory/mutation.rs`, `crates/openqg-core/src/theory/scorecard.rs`,
`crates/openqg-bench/src/zyal_genome/mod.rs`, `docs/zyal-next-level-design.md`.

## Output contract (READ CAREFULLY — this determines whether your work is captured)

Produce your spec as a SINGLE Markdown file named exactly `spec-S04-gap-directed-decomposition.md`,
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
