# ZYAL — what it is, and how it drives OpenQG

> Orientation for any agent advising this project. "ZYAL" is an **overloaded term**: it names two
> related but distinct things, in two different repositories. Confusing them is the single most
> common mistake, so this document separates them first, then details each, with simple examples and
> a glossary. Sources: `~/jekko` (the agent system) and this repo's `ZYAL/`, `crates/openqg-bench`,
> and `crates/openqg-core`.

---

## 0. The one paragraph to read first

**ZYAL means two things.** (1) In the **jekko** monorepo (`~/jekko`), ZYAL is *"Zero-Trust YAML
Agent Language"* — a **host-enforced operating contract** for long-running autonomous AI agents: a
strict YAML "runbook" that gives the *host runtime* (never the model) total control over a daemon's
lifecycle — budgets, stop conditions, evidence gates, permissions, memory. (2) In **OpenQG** (this
repo), ZYAL is a **theory-discovery engine** that *borrows* that runbook format and jekko's model
backends to evolve candidate physics theories under deterministic gates. The connective tissue is
two model-transport layers — **jnoccio** (a local multi-LLM gateway) and **jailgun** (a
browser-bridge LLM transport) — plus the principle both layers share: *the LLM may propose and
critique, but a deterministic host always judges.* In OpenQG the legacy ZYAL genome engine is now
being superseded by a cleaner symbolic engine (`theory evolve`) and a fair scoring adjudicator
(`theory league`); see §4.

```
        ZYAL (jekko)                              ZYAL (OpenQG)
  "Zero-Trust YAML Agent Language"        "theory-gene evolution engine"
  host-enforced agent operating contract  applies the contract to physics discovery
  ~/jekko/crates/zyal-*, docs/ZYAL_*      ZYAL/, crates/openqg-bench/src/zyal_*
        │                                          │
        └──────────── shared backends ─────────────┘
                jnoccio (local gateway, :4317)
                jailgun (browser bridge, :8797)
                rule: LLM proposes; the host judges
```

---

## 1. ZYAL the agent contract (in `~/jekko`)

### 1.1 Thesis
From `~/jekko/paper/ZYAL.md`: *"We present ZYAL (Zero-trust YAML Agent Language), a declarative,
host-enforced control language for governing long-running autonomous AI coding agents… the model is
treated as an untrusted executor within a trusted control plane. The model does the thinking; the
host does the governing; the human does the deciding."* The motivating failure modes (from
`~/jekko/docs/ZYAL_MISSION.md`): unbounded autonomy, no cross-session memory, no evidence discipline,
no safety boundary (`git push --force` one hallucination away), no cost control, and "vibe coding"
(deleting tests, weakening assertions, `@ts-ignore`, silent catches).

### 1.2 The runbook and its lifecycle
A ZYAL program is a YAML document wrapped in **typed sentinels**, armed by a *separate, deliberate*
human action:

```
<<<ZYAL v1:daemon id=my-script>>>
version: v1
intent: daemon
confirm: RUN_FOREVER
job: { name: ..., objective: ..., risk: medium }
# … iteration policy, stop conditions, budgets, evidence, permissions …
<<<END_ZYAL id=my-script>>>
ZYAL_ARM RUN_FOREVER id=my-script        # ← explicit human arming; preview ≠ arm
```

Lifecycle: **write → preview** (validate, no execution) **→ review** (a "Run Card" of budgets,
risks, capability leases) **→ arm** (the `ZYAL_ARM` sentinel) **→ start → monitor → pause/resume →
promote/block → checkpoint → stop**. The host owns the loop: the model never decides when to start,
stop, or promote.

### 1.3 What the contract can express (the capability envelope)
The runbook has 40+ declarative blocks. The ones worth knowing:

- **Bounded execution** — nested budgets (run / task / iteration / experiment-lane) on wall-clock,
  cost, tokens, diff-lines; on-exhaust actions `renew_with_approval | park | pause | abort`.
- **Evidence-gated promotion** — typed proof bundles (tests pass, scope bounded, rollback plan,
  risk delta) signed with SHA-256. *Model self-claims are structurally rejected; only
  host-observable facts count.*
- **Capability leases** — time-windowed, path-scoped tool permissions with an absolute
  `command_floor` (e.g. always block `git push --force`, `rm -rf /`) that no `allow` rule overrides.
- **Anti-vibe gates** — `block_test_deletion`, `block_assertion_weakening`, `block_silent_catch`,
  `block_ts_ignore`, `require_failing_test_first_for_bugfix` (fail-closed).
- **Incubator** — an 8-pass maturation pipeline for hard tasks (Scout → Idea×3 → Strengthen →
  Critic → Synthesize → Prototype → Review → Compress), each pass with a *context mode* (blind /
  inherit / strengthen / critic / pool / promotion) and *write scope* (scratch / isolated worktree).
- **Experiments / hypothesis tournaments** — competing strategies race in isolated git worktrees;
  a **blind critic on a different provider** scores them; the winner is promoted and **failed lanes
  become negative memory** so a dead end is never retried.
- **Fleet** — single-session multi-worker orchestration (hard cap 20) with telemetry.
- **Durable state** — everything persists in SQLite (the source of truth) plus a human-readable
  filesystem mirror at `.jekko/daemon/<runID>/` (`ledger.jsonl`, `STATE.md`, per-task folders).

> Built vs aspirational: the daemon loop, SQLite state, preview/arm, budgets, incubator and
> experiments are implemented; some zero-trust arming refinements (hash/nonce binding) are
> *previewed/parsed but not yet fully runtime-enforced* per `~/jekko/paper/ZYAL.md` §4.1. Treat the
> mission doc as narrative and `docs/ZYAL/SPEC.md` as the schema source of truth.

### 1.4 The implementing crates (in `~/jekko/crates`)
- **`zyal-core`** — lean shared types: `LaneId`, `RunId`, `ArtifactRef`, `SuperReasoningPacket`
  (the producer↔auditor contract), memory/credential-policy enums, and `FORBIDDEN_PATTERNS`
  (redaction).
- **`zyal-supervisor`** — durable orchestrator for "**SuperWorkflows**" (9–12 phase DAGs). Validates
  the manifest, computes phase readiness from the dependency DAG (`execution_layers()` →
  parallel-safe "waves"), and persists run/phase/task/memory/evidence/signoff state in SQLite.
  Phases carry an `exec: { kind, params }` dispatch hint; `ExecKind` includes `Agent`, **`Jailgun`**,
  `Critic`, `ResearchBatch`, `JnoccioRouter`, … (so "jailgun" is *also* the name of an executor kind
  here — see §3).
- **`zyal-key-pool`** — per-user credential pool: scans `~/.jekko/users/*/llm.env`, round-robins
  across slots per `(provider, model)` with a cursor persisted in `~/.jekko/users/.balancer.sqlite`.
- **`zyalc`** — the `.zyal` **compiler/CLI**: `compile`, `inspect`, `lint-super`, `verify-replay`,
  `audit-live-run`. Compiles a `.zyal` source into one of four profiles (validation runbook,
  declarative TOML, GitHub-Actions YAML, or a SuperWorkflow JSON).

---

## 2. The model backends — jnoccio and jailgun

These are the two ways a ZYAL run actually reaches an LLM. They are **not** models themselves; they
are transports/gateways.

- **jnoccio / jnoccio-fusion** — a **local, OpenAI-compatible HTTP+SSE gateway** that multiplexes
  requests across many credential-equipped LLM providers. In jekko it is booted as a background
  server on `http://127.0.0.1:4317` (`jekko-jnoccio-boot`), exposed in the provider catalog as the
  model id `jnoccio/jnoccio-fusion`. In OpenQG it is invoked indirectly as a subprocess:
  `rtk jekko run --headless --ephemeral --provider jnoccio --model jnoccio/jnoccio-fusion --cwd …`
  (`crates/openqg-bench/src/zyal_genome/mod.rs:61`). Think: "the default, fast, pooled LLM lane."
- **jailgun** — a **browser-bridge LLM transport** exposed as a local MCP (JSON-RPC 2.0) server. In
  OpenQG it lives at `http://127.0.0.1:8797/mcp` with a chrome bridge at
  `/home/ubuntu/jailgun/apps/chrome-bridge/bin/chrome-bridge.mjs`
  (`zyal_genome/mod.rs:47,76`); the engine calls `jailgun.run` / `run_status` / `run_summary`,
  resolving a token from `JAILGUN_INGEST_TOKEN` (or by scanning a running `jailgun serve` process).
  Think: "the heavier, browser-driven LLM lane used for hard stages." (In jekko's SuperWorkflow
  vocabulary, `jailgun` is *also* an `ExecKind` dispatch hint — same name, different layer.)

The invariant both share, and the most important rule for any advising agent: **an LLM may propose
and critique, but it never has the final say.** Every LLM output clears a deterministic oracle (a
veto cascade, a numeric forward model, a quality gate) before it can change a result.

---

## 3. ZYAL in OpenQG — the theory-discovery engine

This repo applies the ZYAL idea to physics: evolve candidate cosmological theories, let an LLM
propose/critique, and let deterministic gates judge. There are two pieces — a **workspace** (`ZYAL/`)
and a **pipeline** (`crates/openqg-bench/src/zyal_genome/`).

### 3.1 The `ZYAL/` workspace
A deterministic, replayable scaffold (`ZYAL/README.md`):
- **`stages/`** — 11 pipeline steps `00-atlas … 10-promotion`, each a folder with `stage.yml`
  (`schema_version`, `stage_id`, `family: standard|hard`, `inputs`, `outputs`, `required_evidence`,
  `handoff_contract`), `prompt.md`, `score.yml`, `memory.yml`. Example (`00-atlas/stage.yml`):
  *"Atlas intake — capture the initial genome context and freeze the first evidence bundle."*
- **`runs/`** — `.zyal` runbooks (the same sentinel-wrapped daemon format as §1.2), e.g.
  `run-hybrid-1000-v2.zyal`, `run-pure-jnoccio.zyal`, `run-jailgun-only.zyal`,
  `run-hybrid-10-live-smoke.zyal`. A runbook declares the whole evolutionary run: `evaluation`
  (run_id, generations, live-call cadence, routing, evolution params, quality_gates), `budgets`,
  `stop` (shell assertions), `permissions`.
- **`anchors/`** — frozen calibration set. **Anchors** must survive the judge (e.g.
  `good-lcdm-baseline.json`, `expected_anchor_outcome: "survive"` — the GR/ΛCDM reference the
  adversary must never kill); **decoys** must die (`decoy-gray-box-fudge.json`,
  `probe-overfit-echo.json`). If an anchor dies or a decoy survives, the engine's honesty has broken.
- **`schemas/`** — `zyal-gene-eval.schema.json` validates run artifacts.

A trimmed real runbook (`ZYAL/runs/run-hybrid-10-live-smoke.zyal`):
```
<<<ZYAL v1:daemon id=zyal-genome-hybrid-10-live-smoke>>>
version: v1
intent: daemon
confirm: RUN_FOREVER
imports: [ run-hybrid.zyal ]
job: { name: …, objective: …, risk: medium }
context: { repository_root: ., stage_root: ZYAL/stages, run_root: target/openqg/zyal-genome }
evaluation:
  run_id: hybrid-10-live-smoke
  max_generations_default: 10
  live: { enabled: true, timeout_seconds: 20, hard_stage_every: 10, promotion_judging: true }
  evolution: { population_size: 6, islands: 3, island_names: [foundations, observables, failure-repair] }
budgets: { max_generations: 10 }
<<<END_ZYAL id=…>>>
ZYAL_ARM RUN_FOREVER id=…
```

### 3.2 The genome pipeline and its vocabulary
The engine (`crates/openqg-bench/src/zyal_genome/`) runs an island-model evolutionary loop:

- **Genome** — a candidate theory (in the legacy engine, a named float vector of cosmological genes).
- **Generation** — one round: run stages over the population, score, select survivors, mutate.
- **Islands** — isolated sub-populations evolving in parallel (default 6: `foundations`,
  `coefficients`, `observables`, `failure-repair`, `interface-contracts`, `wildcards`), with
  migration. Preserves diversity.
- **Scoring blend** (`ZYAL/README.md`): `local 0.24 + interface 0.16 + macro 0.24 + innovation 0.18
  + novelty 0.13 − failure_penalty 0.05`.
- **Routing tracks** — how strictly stages are routed to the backends:
  - `pure-jnoccio` — everything via jnoccio; hard stages get "top-20%" model routing.
  - `hybrid` (the ambitious default) — light stages on jnoccio, hard synthesis/repair on jailgun.
  - `jailgun-only` — every stage through jailgun with stricter timeout/retry.
- **Judge + adversary** (`zyal_judge.rs`) — a co-evolving **adversary** escalates a *frontier* each
  generation (attack types: `GrayBox`, `Overfit`, `Unfalsifiable`, `BelowFrontier`, …) with an
  **honesty rollback**: if escalation starts killing the protected anchors, the attacks are unfair
  and roll back (`ANCHOR_SURVIVAL_FLOOR`).
- **Live critic** — optional LLM critique (`critique.rs`, behind `ZYAL_LIVE_CRITIC=1`) that emits
  strict JSON `{falsifiability, plausibility, fatal_flaw}` and can only *lower* a score that already
  passed the deterministic veto — never resurrect one. (The proposes-but-never-judges rule, made
  concrete.)
- **Quality gate** (`quality_gate.rs`) — pass/fail over the whole run: regression rate, live-timeout
  rate, decoy-survival, island coverage, novelty-champion rate, etc.
- **Artifacts** — per run: `run-events.jsonl`, `stage-ledger.jsonl`, `population-snapshot.json`,
  `novelty-archive.json`, `lineage-graph.md`, `quality-gate.json`, `run-summary.json`.

### 3.3 Running it
```bash
just zyal-validate                         # validate the ZYAL/ layout
cargo run -p openqg-bench -- zyal genome run --variant hybrid --max-generations 1
cargo run -p openqg-bench -- zyal genome run --variant pure-jnoccio --max-generations 1
cargo run -p openqg-bench -- zyal genome run --variant jailgun-only --max-generations 1
```
CLI subcommands (`crates/openqg-bench/src/cli/zyal.rs`): `zyal validate`, `zyal jekko-preview`, and
`zyal genome { preflight | run | emit | validate | quality-gate | selftest }`. Justfile recipes:
`zyal-validate`, `zyal-test`, `zyal-jekko-preview`.

### 3.4 The `.jekko/` runtime in this repo
Long-running daemons (e.g. the literature-radar that feeds constraints) are tracked under
`.jekko/daemon/<ULID>/` with an append-only `ledger.jsonl` and `STATE.md`. A ledger line is an
event like `{"event_type":"run.created","run_id":"…","payload_json":{"spec":{…}}}`.

---

## 4. The two physics engines — legacy genome vs. the new symbolic engine

This is the most important thing for an advising agent to understand about the *current* state.

| | **Legacy ZYAL genome** (`openqg-bench/src/zyal_genome`, `zyal_judge`, `zyal_robustness`) | **New symbolic engine** (`openqg-core/src/theory`, `cosmology`) |
|---|---|---|
| candidate | named **float vector** (`Genes{h0, omega_m, …, delta_h0_local, s8_suppression}`) | symbolic **`Theory`** (α-basis + provenanced parameters + structural terms) |
| forward map | **identity** — predicts `h0` by copying the gene (`zyal_robustness.rs`) | **real** FLRW integration (BAO, SNe, sound horizon, BBN, CMB priors, **growth fσ8/S8**) |
| whitebox check | keyword scan over prose | **structural** provenance veto (a `Free` param is a hard kill) |
| LLM | jnoccio/jailgun proposers + live critic | optional `--proposer-cmd` hook (proposes; never judges) |
| status | wired into production `.zyal` runs; **being retired** | the engine to build on |

The new engine added a rigorous **model-selection league** (`theory league`,
`docs/theory-league.md`) that profile-fits every model (the ΛCDM baseline re-fit too) under a
covariance-aware likelihood and ranks by ΔAIC / Δln-evidence. It corrected an earlier overstated
result: the champion's headline "+36.7 log-likelihood beats ΛCDM" was an artifact of comparing a
*fitted* candidate against a *fixed* baseline under a diagonal likelihood; re-fit fairly, evolving
dark energy is *not* preferred on geometry alone, and only becomes strongly favored once the local
distance-ladder H0 (the Hubble tension) is added. The co-evolving adversary was also found to be
telemetry-only and was changed to genuinely gate selection. New engine CLI:
```bash
cargo run -p openqg-bench -- theory evolve  --observables data/fixtures/cosmology/tier0-combined.jsonl
cargo run -p openqg-bench -- theory league  --observables data/fixtures/cosmology/tier0-combined.jsonl \
                                            --observables data/fixtures/cosmology/sh0es-h0.jsonl
```
Retirement path: `docs/legacy-retirement-plan.md` (Stage 0 enablers → adapter behind
`ZYAL_ENGINE=legacy|theory` → swap → flip default → delete the legacy modules). See also
`docs/zyal-engine-rebuild.md` (architecture), `docs/zyal-next-level-design.md` (critical review +
roadmap), and the memory note "credibility-sprint-fair-scoring".

---

## 5. Glossary (quick reference)

- **ZYAL** — (jekko) "Zero-Trust YAML Agent Language", a host-enforced agent operating contract;
  (OpenQG) the theory-gene evolution engine that uses that contract format.
- **jekko** — the agent runtime/monorepo (`~/jekko`) that hosts ZYAL, the daemon engine, and the
  model backends. Invoked here via `rtk jekko …`.
- **jnoccio / jnoccio-fusion** — local OpenAI-compatible gateway (`:4317`) multiplexing many LLM
  providers; the default pooled model lane (`jnoccio/jnoccio-fusion`).
- **jailgun** — browser-bridge LLM transport over MCP (`:8797`) for heavy/hard stages; also the name
  of a SuperWorkflow `ExecKind` in jekko.
- **daemon** — a long-running, host-governed autonomous agent loop defined by a `.zyal` runbook.
- **.zyal runbook** — the sentinel-wrapped YAML daemon spec (`<<<ZYAL v1:daemon …>>> … ZYAL_ARM …`).
- **SuperWorkflow** — jekko's 9–12-phase DAG manifest run by `zyal-supervisor`.
- **key pool** — round-robin credential balancer across `~/.jekko/users/*/llm.env`.
- **genome / generation / island** — a candidate theory / one evolutionary round / a parallel
  sub-population.
- **stage** — one step of the genome pipeline (`ZYAL/stages/NN-name`).
- **anchor / decoy** — frozen calibration theories that must *survive* / must *die* (honesty check).
- **frontier / honesty rollback** — the adversary's rising bar / the guard that rolls it back if it
  starts killing anchors.
- **incubator / experiment tournament** — jekko's hard-task maturation passes / competing-strategy
  races with a blind cross-provider critic and negative memory.
- **evidence gate** — host-checked proof bundle required before promotion (model self-claims rejected).
- **fleet** — single-session multi-worker orchestration (cap 20).
- **proposes-but-never-judges** — the core invariant: LLMs propose/critique; deterministic oracles
  decide.

---

## 6. For an agent advising this project — the short version

- If someone says "ZYAL", ask **which one**: the jekko agent contract, or the OpenQG theory engine.
- The **physics** work that matters now lives in `crates/openqg-core` (the symbolic `Theory`, the
  real forward model + growth, the `theory league` fair scorer), **not** in the legacy
  `zyal_genome` float-vector engine, which is on a retirement path.
- Any LLM-driven step (jnoccio/jailgun proposer, live critic) must clear a deterministic gate; never
  let an LLM score be the final word, and never report a raw Δlog-likelihood vs a fixed baseline —
  use the league's ΔAIC/Δln-evidence (`docs/theory-league.md`).
- Keep theories **whitebox**: derived/structural parameters only; a free fitting knob is a hard kill.
- Primary reading: this file → `docs/zyal-engine-rebuild.md` → `docs/theory-league.md` →
  `docs/zyal-next-level-design.md`; for the jekko contract, `~/jekko/docs/ZYAL_MISSION.md` and
  `~/jekko/docs/ZYAL/SPEC.md`.
