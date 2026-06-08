# OpenQG / ZYAL — independent physicist review

You are a stern, world-class reviewer. Attached (as a source tarball you should read in full) is a
curated snapshot of OpenQG: an evidence-gated, "whitebox", derived-not-fit theory-discovery engine for
cosmology and gravity, plus its ZYAL orchestration layer. The project's goal is to build the most
TRUSTWORTHY possible automated system for chasing the physics frontier where quantum gravity /
unification meets real data — and to make it worthy of a serious physicist's attention.

The full curated source is **attached to this conversation as `source.tar.gz`**. FIRST extract it in your code sandbox (`tar xzf source.tar.gz`) and read the ACTUAL file contents below — do not review from filenames or prior knowledge; if the archive is genuinely unavailable, say so explicitly.

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

## Your lens: adversarial skeptic / red-team ("why should I trust ANY of this?")

Your job is to find the STRONGEST reasons a hostile, busy expert would dismiss this project outright —
then tell the team exactly what would change your mind. Attack the whole claim, not the periphery:
- Is "derived, not fit" real or marketing, given the continuous parameters (h, Ω_m, w0, μ0, σ8) are
  still fit to data? Does the provenance gate actually prevent overfitting, or just relabel it?
- Is the deterministic-veto / "the LLM never judges" architecture genuinely robust, or can a
  plausible-but-wrong theory slip through the gates? Construct the theory that would.
- Is the evolutionary search doing anything beyond rediscovering near-ΛCDM? Is the "co-evolving
  adversary" meaningful or theatre?
- Where is the project at risk of OVERCLAIMING (the corrected +36.7 episode is a warning sign — are
  there others lurking)? Is the honest-limitations list complete, or are there undisclosed gaps?
- Would any of this survive peer review or a sharp seminar question, and if not, what is the single
  thing that sinks it?

Rank the failure modes by how damaging they are to credibility, and for each give the concrete,
checkable change that would neutralize it. Be ruthless, but specific and fair — a useful red-team
leaves a to-do list, not just a verdict.

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
