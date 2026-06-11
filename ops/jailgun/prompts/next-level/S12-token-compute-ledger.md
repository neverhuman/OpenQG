# OpenQG / ZYAL — next-level engineering review (spec S12 of 12)

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
parallel (S01–S12); yours is S12. The team is explicitly willing to make profound changes under the
hood: if the right answer requires a technology ZYAL does not have today, SPEC IT AS A HARD
REQUIREMENT with the engineering detail to build it.

The full curated source is **attached to this conversation as `source.tar.gz`**. FIRST extract it
in your code sandbox (`tar xzf source.tar.gz`) and read the ACTUAL file contents — do not review
from filenames or prior knowledge. If the archive is genuinely unavailable, say so explicitly in
your output and stop; DO NOT fabricate a review of files you have not read.

Platitudes are worthless; interfaces, data structures, pseudocode, named tools, and acceptance
tests are the currency.

## Your lens: LLM-ops / observability engineer — every token attributed, over all time

You instrument token flows the way a CFO instruments cash: an unattributed API call is a leak.
Today the engine's attempt records carry call counts, model IDs, raw-output hashes and lengths —
but NO token counts (`proposer_router.rs` only configures `max_completion_tokens`; the ledgers in
`ledger_sink.rs` stream events without usage data). The team's hard requirement: ZYAL tracks ALL
token use over time, so the economics of discovery are as observable as the physics. Spec it:

1. **The token ledger.** Per-call usage capture: the OpenAI-compatible `usage` object through both
   transport backends (the jnoccio gateway and the jailgun browser-bridge — note the browser path
   may not return usage; spec the fallback: local token counting with NAMED tokenizers per model
   family, flagged as estimated). Extended attempt-record schema: `prompt_tokens`,
   `completion_tokens`, `cached_tokens`, `reasoning_tokens`, latency_ms, provider, model_id,
   USD cost when priced, estimation flag. Storage in the existing streamed JSONL ledgers (extend
   `ledger_sink.rs` events — spec the new event types), plus a backfill strategy for historical
   campaigns: what is recoverable from stored raw-output lengths and call counts, and how is
   backfilled data marked?
2. **Aggregation views.** Rollups the campaign report must carry: tokens per proposal / lane /
   island / generation / repair round; tokens-to-kill vs tokens-to-survive; tokens per scorecard
   point earned; per-model and per-provider totals OVER TIME (the longitudinal view the team
   explicitly wants — spec the time-series storage and its query interface). Spec the exact new
   sections in the campaign report and the queries that produce them.
3. **Cost KPIs and stopping rules.** Cost-per-adjudicated-proposal, cost-per-point,
   marginal-token-productivity over a campaign (diminishing-returns detection feeding stopping
   rules — spec the statistic and the rule), and budget-aware routing: the existing win-rate
   routing extended with cost terms (does it already approximate cost-awareness? read the router
   and answer from the code).
4. **Standards decision.** OpenTelemetry GenAI semantic conventions vs self-hosted Langfuse /
   Arize Phoenix vs extending the native JSONL+rescore tooling. Recommend ONE with rationale; the
   determinism/replay contract must hold either way (an external observability store can NEVER be
   load-bearing for replay — the JSONL ledger remains the source of truth).
5. **Token-efficiency engineering.** Quantify expected savings with a measurement plan: prompt
   caching across the proposal fan-out, repair-round overhead (measure it — repair loops were a
   dominant V6 cost), strict-schema savings from the V7 sketch contract, retrieval token budgets
   (if S09-class retrieval lands — no dependency). For each: the experiment, the metric, the
   decision threshold.
6. **Acceptance.** A campaign report stating total tokens by model/provider/lane with ZERO
   unattributed calls; a regression test that fails any LLM call lacking usage capture (or an
   estimation flag); a replay test proving ledger extension broke nothing.

Read first, in order: `crates/openqg-bench/src/zyal_genome/proposer_router.rs`,
`crates/openqg-bench/src/zyal_genome/proposer.rs`,
`crates/openqg-bench/src/zyal_genome/ledger_sink.rs`, `docs/ZYAL.md` (§2, the two backends),
`paper/main.tex` (total-observability + cost-profile sections).

## Output contract (READ CAREFULLY — this determines whether your work is captured)

Produce your spec as a SINGLE Markdown file named exactly `spec-S12-token-compute-ledger.md`, and
return it as the ONE downloadable `.tar.gz` artifact you attach — the file at the archive root; no
project folder, no other files, no code archive, no patch. (If tarring fails, attaching the bare
`.md` is the accepted fallback.)

The spec must be a COMPREHENSIVE, PRIORITIZED engineering specification, 2,500–6,000 words:
1. Open with a RANKED BACKLOG — highest-leverage change first — each item stating: what, why it
   matters, rough effort (S/M/L), and how to verify it (an acceptance test a hostile reviewer
   would accept).
2. Then detailed design sections answering the charter — concrete interfaces, data structures,
   pseudocode, named tools/libraries with versions, and literature/standards citations. Cite the
   repo files and claims you respond to.
3. Where your spec touches a sibling topic (S01–S12), write "see SXX" and keep your own spec
   self-contained — do not depend on another spec's output.
4. Close with a mandatory section titled **"What we got wrong"** — the strongest claims in the
   attached materials you believe are mistaken, oversold, or self-deceiving, each with the concrete
   check that would settle it.

Depth over breadth where they conflict. Every deficiency you assert must come with the fix and its
verification.
