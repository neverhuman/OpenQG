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

## Your lens: scientific software, verification & reproducibility

Emphasize TRUST IN THE CODE as a referee of computational results. The engine is pure-Rust and
deterministic, omits observables it cannot derive (never fakes them), and reports 172 passing tests.
Review the actual modules (`cosmology/background.rs`, `cosmology/growth.rs`, `scoring/covariance.rs`,
`theory/league.rs`, `theory/vetoes.rs`). What verification is missing: cross-checks against
CLASS / CAMB / hi_class on the same parameters; analytic limits (matter-dominated growth, de Sitter,
flat-ΛCDM distances); convergence/error budgets for the Simpson distance integrals and the RK4 growth
solver; unit-level validation of every veto threshold and of the Cholesky covariance solve;
fitting-formula error bars (r_drag, z_*, Y_p)? How should the optional Boltzmann backend be integrated
WITHOUT breaking determinism and the pure-Rust default? Address provenance and reproducibility a
referee would demand: data lockfiles and source manifests, seed determinism across platforms, CI that
re-derives the headline league numbers, an independent-replay package, and guarding against silent
regressions. Identify the highest-leverage software/verification work that would most raise the
believability of the results.

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
