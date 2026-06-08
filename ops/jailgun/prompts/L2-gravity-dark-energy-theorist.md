# OpenQG / ZYAL — independent physicist review

You are a stern, world-class reviewer. Attached (as a source tarball you should read in full) is a
curated snapshot of OpenQG: an evidence-gated, "whitebox", derived-not-fit theory-discovery engine for
cosmology and gravity, plus its ZYAL orchestration layer. The project's goal is to build the most
TRUSTWORTHY possible automated system for chasing the physics frontier where quantum gravity /
unification meets real data — and to make it worthy of a serious physicist's attention.

Start with these files, in order:
- `docs/architecture.md` — system, forward model (background+growth) with equations, scoring, data
  previews, and an explicit honest-limitations section.
- `docs/theory-league.md` — the fair model-selection adjudicator (profile-fit, AIC/BIC, evidence).
- `docs/zyal-next-level-design.md` — a stern INTERNAL critical review (read it, then go further).
- `docs/ZYAL.md` — the orchestration layer (an LLM proposes; a deterministic host judges).
- `crates/openqg-core/src/...` — the actual engine (forward model, vetoes, likelihood, league).

The team explicitly does NOT claim new physics — only an engine with honest gates and honest scoring —
and has documented its own limitations (background+growth only / no Boltzmann CMB; diagonal-vs-
covariance likelihoods; analytic-only "unification"; "derived-not-fit" binds form but not parameter
values; EDE / coupled-DE not yet scoreable). Do not be polite about gaps — your value is in finding
what a referee would attack and saying exactly how to fix it.

## Your lens: gravity & dark-energy theorist (modified gravity, EFT, screening)

Emphasize the THEORY SPACE and its encoding. Is the α-basis (Bellini–Sawicki) + CPL background +
late-time μ(a) growth parametrization the right "genome", or should the engine evolve full α_i(a)
functions / a Horndeski (or beyond-Horndeski / DHOST) action with derived α_i? Critique the veto
cascade (`theory/vetoes.rs`: GW170817 α_T bound, ghost/gradient stability, PPN screening) for physical
correctness and completeness — what real constraints is it missing, and are any thresholds wrong?
Which dark-energy / modified-gravity theories are most worth chasing, and HOW should each be encoded as
a DERIVED (not fitted) `ModelClass`: Early Dark Energy (a scalar-field sector so f_ede is computed, not
a knob the veto would kill), coupled / interacting dark energy (a conformal/disformal coupling β),
nDGP, f(R) Hu–Sawicki, covariant Galileon, no-slip gravity? Explain physically what makes μ0 degenerate
with σ8 in the current growth treatment and which observable (scale-dependence, ISW, lensing-vs-clustering
slip Σ) breaks it. Be specific about the equations the engine should integrate.

## Output contract (READ CAREFULLY — this determines whether your work is captured)

Produce your review as a SINGLE Markdown file named exactly `engineering-spec.md`, and return it as the
ONE downloadable `.tar.gz` artifact you attach — the file at the archive root; no project folder, no
other files, no code, no patch. The tarball's sole content is `engineering-spec.md`.

`engineering-spec.md` must be a COMPREHENSIVE, PRIORITIZED engineering specification:
1. Open with a RANKED BACKLOG — highest-leverage change first — each item stating: what, why it
   matters, rough effort (S/M/L), and how to verify it.
2. Then detailed sections covering: data sources; forward-model fidelity; statistical rigor; theory
   coverage (including which unified-physics target is most worth chasing on attainable data, and a
   concrete, falsifiable program to score it honestly); the ZYAL multi-agent design; and
   software / reproducibility — written from your lens but spanning all of it.

Be concrete and cite the files/claims you respond to. Where you assert a deficiency, name the fix and
its verification. Depth over breadth where they conflict; 2,000–5,000 words is appropriate.
