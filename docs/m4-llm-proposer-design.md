# Milestone 4 — LLM (jnoccio) as Theory Proposer + Derivation-Checker

> **Status:** design. Companion milestones: M1 forward model (`crates/openqg-core/src/cosmology`,
> `ForwardModel`), M2 deterministic veto cascade (`crates/openqg-core/src/theory/vetoes.rs`),
> M3 capstone assessment (`crates/openqg-core/src/theory/assessment.rs::assess`). This document
> designs how to wire the existing **jnoccio/jailgun** live-call machinery
> (`crates/openqg-bench/src/zyal_genome/`) into the rebuilt symbolic engine as a *proposer* and
> *derivation-checker* — **never** as the final judge.
>
> **Research basis:** `docs/research/automated-theory-discovery.md`, especially **§5
> "LLM-as-physicist"** and **§6 recommendations**. The non-negotiable principle from §5/§6 is cited
> throughout: *"LLM as proposer + critic, never judge. Every LLM output clears a non-LLM oracle.
> Adversarial proposer/skeptic resolved by the symbolic gates."* §5 also records the honesty
> result we must engineer around: *"a text-only supervisor cannot fully verify honesty ⇒ Never let
> an LLM be the final judge. Route every LLM claim through a non-LLM oracle (symbolic gate, numeric
> simulation/fit, or certificate)."*

---

## 0. The core invariant (read this first)

> **INVARIANT (M4-INV-1).** No LLM output is *trusted* until it clears a non-LLM check. The only
> things that can promote a candidate into the population or raise its fitness are: (a) the M2 veto
> cascade `run_veto_cascade` returning empty, (b) the M1 forward model producing a real likelihood,
> and (c) the M3 `assess()` `final_fitness > 0`. The LLM may *propose* and *critique*; it may never
> *decide*.

This is the operational form of `automated-theory-discovery.md` §6.6 ("LLM as proposer + critic,
never judge"). The LLM is a *search heuristic* that proposes structure into a space that is then
adjudicated entirely by deterministic Rust. Every LLM artifact is, by construction, untrusted text
until the oracle has spoken. The existing live critic already embodies this for the critique role
(`zyal_genome/hybrid_artifacts.rs` lines ~422–447, and the comment block at `mod.rs:563–568`: the
verdict "can only LOWER a candidate that already passed; it never resurrects a vetoed/gray-box
one"). M4 generalizes the same subordination to the *proposer* and *checker* roles.

---

## 1. Role split

The LLM (`jnoccio`, model `jnoccio/jnoccio-fusion`) takes three subordinate roles. All three feed
the same deterministic gate; none can bypass it.

### 1a. Proposer of `Theory` skeletons (LLM-SR program-skeleton style)

Per `automated-theory-discovery.md` §5 ("LLM as **structure proposer** seeded with domain priors
(LLM-SR) — shrinks the search, generalizes OOD *when paired with a numeric/symbolic fitter and an
external verifier*") and §1's LLM-SR entry ("LLM proposes equations as **program skeletons with
placeholder params**; decouples discrete structure (LLM) from continuous fitting"):

The LLM proposes the **discrete structure** of a candidate
`openqg_core::theory::Theory` — the α-basis term set and the *named* physical parameters — together
with a **derivation sketch** per parameter. It does **not** propose final fitted numbers as the
source of truth; the `value` it emits is a *seed* to be re-derived/refit and checked. Concretely it
emits the fields of `Theory`:

| `Theory` field | What the LLM proposes |
|---|---|
| `id: String` | a human label (sanitized, namespaced `m4-prop-<gen>-<n>`) |
| `terms: Vec<Term>` | the action building blocks: `name`, `mass_dimension`, `free_lorentz_indices` |
| `parameters: Vec<Parameter>` | `symbol`, `value` (seed), `physical_meaning`, and a **derivation sketch** that maps to `Provenance` |
| `alpha: AlphaBasis` | `alpha_m`, `alpha_b`, `alpha_k`, `alpha_t` |
| `stability: Stability` | `kinetic_coefficient`, `q_s`, `sound_speed_sq`, `has_nondegenerate_higher_derivatives` |
| `screening: Option<String>` | declared screening mechanism (chameleon/Vainshtein/symmetron/k-mouflage) |
| `background: CosmologyParams` | the background cosmology that drives the M1 forward model |

The decisive output is the **`Provenance` per parameter**. The LLM proposes one of
`Fundamental` / `Derived { mechanism }` / `Free` (see `theory/mod.rs:36–56`). The whole point of
the type is that "the engine trusts structure, not prose: only a `Derived` parameter with a
non-empty mechanism, or a `Fundamental` constant, is whitebox" (`mod.rs:34–35`).

### 1b. Derivation-checker (verify each claimed `Provenance::Derived`)

Per §2 ("Derivation / provenance verification, ordered by strength") and §6.3 (the AI-Descartes
"β derivation distance" idea), the LLM acts as a **checker**, not an asserter, of each claimed
mechanism. For every `Parameter` whose proposed provenance is `Derived { mechanism }`, the checker
must produce a *machine-verifiable derivation obligation* — a small, checkable chain from named
upstream quantities/axioms to the parameter value, in the dimensionless-group form §2.1 demands.

Crucially, the checker's output is itself untrusted (INVARIANT M4-INV-1): a claimed derivation is
only accepted if a **non-LLM** check confirms it (§3). If the obligation cannot be discharged by the
oracle, the parameter is **demoted to `Provenance::Free`**, which the whitebox veto then kills
(`vetoes.rs:73–76`, `VetoReason::FreeParameter`). This makes "I derived it" cost-free to *claim*
and lethal to *fake*.

### 1c. Adversarial critic (unchanged — keep current role)

The current `run_live_critique` role (`zyal_genome/critique.rs:87`) is preserved as-is: jnoccio
scores `falsifiability`/`plausibility` and names a `fatal_flaw`, and the verdict **only lowers**
the fitness of a candidate that already passed the deterministic gate
(`hybrid_artifacts.rs:438–447`). This is the §6.6 adversarial proposer/skeptic pairing, "resolved
by the symbolic gates, not by vote" (§5).

**Subordination summary.** Proposer feeds the gate (§3); checker's claims are re-verified by the
gate (§3); critic can only attenuate post-gate fitness. None can promote, resurrect, or finalize.

---

## 2. The proposal protocol

### 2a. Wire format

The proposer call reuses the existing prompt → stdin → subprocess → stdout → parse → receipt
pattern (`run_live_call_attempt` in `jailgun_live.rs:70`, exactly as `run_live_critique` uses it).
The LLM is instructed to return **exactly one strict JSON object and nothing else** — the same
discipline as `build_critique_prompt` ("Respond with EXACTLY one strict JSON object and nothing
else", `critique.rs:50–51`). Parsing reuses the brace-matched extractor pattern of
`parse_live_verdict` (`critique.rs:55–85`) so prose around the JSON is tolerated but the object is
authoritative.

### 2b. Exact JSON schema (maps 1:1 to the Rust types)

```json
{
  "id": "string  (sanitized → Theory.id; final id is namespaced server-side)",
  "terms": [
    { "name": "string", "mass_dimension": 4, "free_lorentz_indices": 0 }
  ],
  "parameters": [
    {
      "symbol": "string",
      "value": 0.0,
      "physical_meaning": "string (non-empty)",
      "provenance": {
        "kind": "fundamental | derived | free",
        "mechanism": "string  (REQUIRED and non-empty iff kind==derived)",
        "derivation_sketch": {
          "from": ["named upstream symbols/axioms, e.g. M_pl, beta_conformal"],
          "relation": "dimensionless relation, e.g. alpha_M0 = 2*beta^2 / (1 + ...)",
          "dimensionless_groups": ["pi_1 = ...", "..."],
          "limit_check": "GR limit recovered when beta -> 0"
        }
      }
    }
  ],
  "alpha":     { "alpha_m": 0.0, "alpha_b": 0.0, "alpha_k": 0.0, "alpha_t": 0.0 },
  "stability": { "kinetic_coefficient": 1.0, "q_s": 1.0, "sound_speed_sq": 1.0,
                 "has_nondegenerate_higher_derivatives": false },
  "screening": "chameleon | vainshtein | symmetron | k_mouflage | null",
  "background": { "...": "CosmologyParams fields (H0, Omega_m, n_eff, ...)" }
}
```

Field-by-field mapping to `crates/openqg-core/src/theory/mod.rs`:

- `terms[].{name,mass_dimension,free_lorentz_indices}` → `Term` (`mod.rs:70–75`).
- `parameters[].{symbol,value,physical_meaning}` → `Parameter` (`mod.rs:59–65`).
- `parameters[].provenance.kind` → `Provenance` variant (`mod.rs:36–45`):
  `"fundamental"` → `Provenance::Fundamental`; `"derived"` → `Provenance::Derived { mechanism }`
  (mechanism string is `provenance.mechanism`); `"free"` → `Provenance::Free`.
- `parameters[].provenance.derivation_sketch` → **not** a `Theory` field; it is the **derivation
  obligation** consumed by §3's checker. It is recorded in the receipt and discarded from the
  `Theory` itself (the `Theory` only carries the terse `mechanism` string).
- `alpha` → `AlphaBasis` (`mod.rs:78–88`).
- `stability` → `Stability` (`mod.rs:112–123`).
- `screening` → `Theory.screening: Option<String>`.
- `background` → `CosmologyParams` (deserialized via the existing serde model in
  `crates/openqg-core/src/cosmology`).

### 2c. Parse + validation pipeline

A new module `crates/openqg-bench/src/zyal_genome/proposer.rs` implements:

```text
fn parse_proposal(stdout: &str) -> Result<Theory, ProposalError>
```

1. Extract the first balanced `{...}` object (reuse the `parse_live_verdict` brace-matcher,
   `critique.rs:55–71`).
2. `serde_json::from_str` into a `ProposedTheory` DTO (a serde mirror of the schema above).
3. **Structural validation** (deterministic, pre-oracle):
   - every `Term.mass_dimension`, `free_lorentz_indices` present and integral;
   - every `Parameter.symbol` non-empty + unique; `physical_meaning` non-empty;
   - `provenance.kind ∈ {fundamental,derived,free}`; if `derived`, `mechanism` is non-empty
     (else it would only trip `VetoReason::UnprovenancedParameter` later — we reject earlier);
   - `alpha`/`stability`/`background` fields all present and finite (`f64::is_finite`).
4. Construct `Theory` with id namespaced as `m4-prop-<generation_id>-<n>` (LLM-supplied id is a
   label only; the engine owns identity so labels can't game the MAP-Elites archive — same
   rationale as the "numeric and name-independent" `Cell` descriptor in `evolve.rs:16–29`).

### 2d. Parse failure / hallucinated tool output ⇒ **lethal**

> Per `automated-theory-discovery.md` §5: tool/output hallucination (arXiv:2510.22977) and the
> theoretical inevitability of hallucination ⇒ the supervisor cannot trust LLM tokens.

Any of the following makes the proposal **lethal — it is dropped, never inserted, and logged as a
discarded proposal** (it does not get a "benefit of the doubt" insertion):

- no `{...}` object found, or `serde_json` parse error;
- a required field missing / non-finite / wrong type;
- a `derived` provenance with an empty `mechanism`;
- the JSON references tool output or "downloaded" results that don't exist in the receipt
  (hallucinated tool call — detected because the proposer subprocess produced no corresponding
  artifact; see §6).

Lethality here ≠ penalizing a real candidate: a malformed proposal simply *is not a candidate*.
The deterministic floor of the engine (the always-present `Theory::baseline_lcdm()` seed,
`evolve.rs:163`) guarantees the run still produces a credible champion even if the LLM emits pure
garbage. Transport failures (timeout, non-zero exit) are treated the same way the critic treats
them: they do not enter the population and do not penalize anything else (cf. `critique.rs:102–121`
returning `status: "unparsed"/"error"` with neutral scores; `hybrid_artifacts.rs:445–447`
restoring `after = before` when `status != "ok"`).

---

## 3. The verification gate (the oracle)

Every proposed (and parse-valid) `Theory` runs the **full deterministic stack before it can enter
the population**. This is `automated-theory-discovery.md` §6.2 ("Deterministic veto cascade as a
pre-fitness gate, cheapest-first … only survivors reach expensive stages") and §6.3 (ε/β dual
fitness via `assess`). Order, cheapest-first:

```text
proposed Theory
   │
   ├─(3.1) derivation check ── demote unverifiable Derived → Free
   │
   ├─(3.2) M2 veto cascade: run_veto_cascade(&theory)  ── non-empty ⇒ DROP
   │        (theory/vetoes.rs:48)
   │
   ├─(3.3) M1 forward model + M3 assess(&theory, obs, model, baseline_ll)
   │        (theory/assessment.rs:36) → CandidateAssessment
   │
   └─(3.4) insert into archive only via the existing evolve() insert()
            (theory/evolve.rs:50) — vetoed ⇒ final_fitness 0 ⇒ never a champion
```

### 3.1 Derivation check → demotion

For each `Parameter` with `Provenance::Derived { mechanism }`, the checker (§1b) discharges the
`derivation_sketch` obligation with a **non-LLM** check, strongest available first
(`automated-theory-discovery.md` §2 ordering):

1. **Dimensional homogeneity** of the stated `relation` over the `from` symbols (§2.1 Buckingham-π;
   the cheapest, deterministic check). Reuses the same mass-dimension bookkeeping the veto cascade
   already does on `Term` (`vetoes.rs:51–59`).
2. **Limit/correspondence check** (§3 "Correspondence principle"; §6.8): the stated `limit_check`
   must symbolically reduce the parameter to its GR value (`AlphaBasis::gr`, `mod.rs:92`) in the
   declared limit.
3. (future tiers, §2.5/§2.6) β-distance via a theorem prover / Positivstellensatz certificate —
   stubbed now, scored as "unverified-but-not-refuted".

> **Demotion rule.** If a `Derived` parameter's obligation **cannot be discharged** (dimensional
> mismatch, no limit recovery, or sketch absent), its provenance is rewritten to
> `Provenance::Free`. It then necessarily fails the whitebox veto at `vetoes.rs:73–76`
> (`VetoReason::FreeParameter`) and the whole theory is dropped at 3.2.

This is the mechanical realization of "derived, not fit": a *claim* of derivation that the oracle
cannot confirm is treated exactly as a free fitting knob — the gray-box trap the type system was
built to kill (`mod.rs:44`, `assessment.rs` `gray_box_is_not_credible` test).

### 3.2–3.4 Veto → assess → insert

`run_veto_cascade` (M2) is the hard gate; a non-empty `Vec<VetoReason>` drops the proposal. Survivors
go through `assess()` (M3), producing `CandidateAssessment` with `final_fitness`. Insertion uses the
*existing* `evolve.rs::insert` (`evolve.rs:50–59`) so a vetoed/gray-box proposal — `final_fitness =
0` — can never displace a real champion or be `is_credible()` (`assessment.rs:30–33`,
`evolve.rs:107`). No new "trust the LLM" path is introduced; the proposer just supplies more
candidates to the *same* gate.

---

## 4. Recitation guard

> `automated-theory-discovery.md` §5 ("recitation/memorization of textbook laws (defeat with
> structurally-novel benchmarks, LLM-SRBench)") and §6.7 ("Anti-recitation: version a
> modified-physics held-out benchmark; gate 'discovery' on OOD predictive performance + novelty,
> plus perplexity/known-law fingerprinting").

A proposer that simply regurgitates a textbook Lagrangian (ΛCDM, vanilla Horndeski) is not
discovering anything; LLM-SR (§1) documents this failure and built modified-physics benchmarks to
defeat it. Three deterministic guards, all post-gate (they only modulate a candidate that already
*passed* §3 — they never resurrect):

1. **Modified-physics held-out benchmark (primary).** Maintain a versioned set of
   *structurally-perturbed* reference theories under `data/fixtures/` (mirror of the existing
   `data/fixtures/tension/` convention) whose correct answer differs from any textbook law. A
   proposal is scored on its OOD fit there. Recitation of a memorized law scores poorly OOD; this is
   the §6.7 / LLM-SRBench discriminator. Implemented as an extra `ObservableRecord` set fed to the
   same `assess()` so it costs no new oracle.

2. **Novelty vs the archive.** Reuse the existing novelty machinery
   (`zyal_genome/novelty.rs`, `novelty_terms_from_text`, the `novelty-archive.json` artifact
   `hybrid_artifacts.rs:706–738`). Compute a structural-novelty distance of the proposed `Theory`
   (α-vector + term-set + parameter-mechanism fingerprint) against the archive; a proposal that is
   ε-close to an existing elite *or* to a known textbook fingerprint gets a **novelty penalty
   factor** applied to its post-gate fitness (same "only lowers" discipline as the critic,
   `hybrid_artifacts.rs:438–447`).

3. **Known-law / perplexity fingerprint.** Maintain a small table of canonical-law fingerprints
   (GR+Λ term set, standard single-field quintessence, DGP, etc.). An exact or near-exact match
   flags `recited = true` in the receipt and applies the novelty penalty. (Perplexity proper needs
   logprobs jnoccio may not expose; the fingerprint table is the deterministic fallback §6.7
   anticipates.)

None of these can *kill* a physically valid theory — they de-prioritize the *un-novel* among the
valid, exactly as §6.7 intends ("gate 'discovery' on OOD predictive performance + novelty").

---

## 5. Wiring

Reuse the existing jnoccio/jailgun live-call machinery — do **not** reinvent it.

### 5a. Subprocess + receipt (reuse)

- **Command.** Add a `JNOCCIO_PROPOSER_COMMAND` constant alongside the existing
  `JEKKO_LIVE_COMMAND` (`mod.rs:61–74`) and `JNOCCIO_CRITIQUE_COMMAND` (`mod.rs:569–580`), same
  invocation shape: `jekko run --headless --ephemeral --provider jnoccio --model
  jnoccio/jnoccio-fusion --cwd /home/ubuntu/openQG`.
- **Call.** Reuse `run_live_call_attempt(command, prompt, timeout, attempt, started_at)`
  (`jailgun_live.rs:70`) — prompt on stdin, stdout captured, process-group kill on timeout
  (`live_call.rs:300` `kill_live_process_tree`). For the routed/jailgun backend, the heavier
  `run_live_call` path (`live_call.rs:37`) already writes the full receipt bundle
  (`prompt.md`, `retrieval-packet.json`, `raw-output.txt`, `parsed-summary.json`, `receipt.json`)
  — proposals get the *same* audit trail as critiques.
- **Parse.** Reuse the brace-matched JSON extractor from `parse_live_verdict` (`critique.rs:55`),
  generalized in the new `proposer.rs::parse_proposal` (§2c).
- **Receipt.** Emit a `record_kind: "live_proposal"` ledger entry mirroring the `"live_critique"`
  record (`hybrid_artifacts.rs:462–477`), recording `status`, `parsed`/`dropped`, the discarded
  `derivation_sketch`s, demotions applied, and `recited` flag.

### 5b. Env-var controls (extend the existing pattern)

Mirror `critique.rs::live_critic_enabled` / `env_usize` (`critique.rs:3–15`) and the
`ZYAL_LIVE_TOPK` / `ZYAL_LIVE_EVERY` / `ZYAL_LIVE_TIMEOUT` reads in
`hybrid_artifacts.rs:82–84`:

| Env var | Meaning | Default |
|---|---|---|
| `ZYAL_LIVE_PROPOSER` | enable LLM proposer (`1`/`true`) | off |
| `ZYAL_LIVE_PROPOSE_N` | proposals requested per injection point | 3 |
| `ZYAL_LIVE_PROPOSE_EVERY` | inject every N generations | 1 |
| `ZYAL_LIVE_PROPOSE_TIMEOUT` | per-call timeout seconds | 90 |
| `ZYAL_LIVE_PROPOSE_SEED_N` | proposals at generation 0 (seeding) | 5 |

Add a `live_proposer_enabled()` reading `ZYAL_LIVE_PROPOSER`, structurally identical to
`live_critic_enabled()`.

### 5c. Where it plugs into `evolve()`

The reference loop is `crates/openqg-core/src/theory/evolve.rs::evolve` (`evolve.rs:62`). The
proposer plugs in at two points without changing the loop's determinism contract (same seed ⇒ same
archive, `evolve.rs:173` test) — LLM proposals are an *additional candidate source* that still pass
through the identical `assess` + `insert`:

1. **Seed injection (generation 0).** Before the main loop, request `ZYAL_LIVE_PROPOSE_SEED_N`
   proposals, run each through §3, and add survivors to `seeds` alongside `Theory::baseline_lcdm()`
   (the loop already iterates `for theory in seeds` at `evolve.rs:77`). The baseline seed remains
   mandatory so the run is robust to a fully-failing LLM (§2d).

2. **Per-generation injection.** Inside the `for _ in 0..generations` loop (`evolve.rs:82`), every
   `ZYAL_LIVE_PROPOSE_EVERY` generations, request `ZYAL_LIVE_PROPOSE_N` proposals *conditioned on
   the current archive elites* (pass the top elites + their veto diagnostics into the prompt as the
   "retrieval packet", reusing the `retrieval_packet` pattern of `live_call.rs:65–83`), gate them
   through §3, and `insert` survivors into `archive` exactly like mutated children
   (`evolve.rs:99–101`).

Because the live engine wraps `evolve` from `zyal_genome` (the per-generation critic already lives
in `hybrid_artifacts.rs`), M4 adds the proposer call adjacent to the existing critic block
(`hybrid_artifacts.rs:422`), keeping all live machinery in `zyal_genome/`. The pure-core `evolve`
stays LLM-free; the LLM hook is an injected candidate-source closure so `openqg-core` keeps no
dependency on the subprocess layer (preserving the deterministic core tests).

---

## 6. Honesty failure modes + mitigations

> All mitigations are concrete realizations of `automated-theory-discovery.md` §5 ("Fails / honesty
> risks") and §6.6.

| # | Failure mode (cited) | Why it's dangerous | Mitigation (deterministic) |
|---|---|---|---|
| H1 | **Sycophantic false proof** — LLM asserts a `Derived` mechanism that doesn't hold (BrokenMath; AI-Scientist eval arXiv:2502.14297, 2504.08066; §5) | A fake derivation would smuggle a free knob past the whitebox veto | §3.1 demotion: an undischargeable `derivation_sketch` ⇒ `Provenance::Free` ⇒ `VetoReason::FreeParameter` kill. The LLM cannot *self-certify*; only the dimensional/limit/certificate oracle can. (§6.6: "resolve disputes with the deterministic gates, not by vote.") |
| H2 | **Tool / output hallucination** (arXiv:2510.22977; §5) — JSON cites results/artifacts that were never produced | Trusting phantom evidence | §2d lethal handling: a proposal referencing tool output absent from the actual subprocess receipt is dropped. The receipt bundle (`live_call.rs:90–99,198–239`) is the *only* admissible evidence — the prompt already mandates "Use the retrieval packet as the only evidence context … Do not rely on raw provider logs, target artifacts, or unstated external evidence" (`live_call.rs:341–343`). |
| H3 | **Recitation / memorization** of textbook laws (LLM-SR; LLM-SRBench; §5/§6.7) | "Discovery" that is really retrieval | §4: modified-physics held-out OOD benchmark + archive novelty + known-law fingerprint, all post-gate penalties. |
| H4 | **Hallucination is inevitable over computable functions** (Xu et al. 2024; §5: "a text-only supervisor cannot fully verify honesty") | No amount of prompting makes the LLM trustworthy | INVARIANT M4-INV-1 + §3: the LLM is *structurally* never the judge. Every claim routes through `run_veto_cascade` / `assess`. The text channel is treated as adversarial by default. |
| H5 | **Label gaming** — LLM names a candidate to look like a known-good elite | Could try to win the MAP-Elites cell by name | Engine owns `Theory.id` (§2c); the `Cell` descriptor is numeric and name-independent by design (`evolve.rs:16–29`), and `insert` compares `final_fitness` only (`evolve.rs:52–53`). |
| H6 | **Critic capture** — proposer and critic are the same model and collude | Self-approval | The critic can only *lower* post-gate fitness (`hybrid_artifacts.rs:438–447`); it cannot approve. The *decision* is the deterministic gate, never the critic (§6.6 adversarial pair "resolved by the symbolic gates"). |

---

## 7. Incremental implementation plan

Build order follows `automated-theory-discovery.md` §6 build-order step (c) ("LLM proposer with
oracle verification + recitation guard"). Each step is independently testable with a **mocked LLM
response** (no live subprocess), so the whole feature is deterministic-testable in CI.

### Step 1 — Parser + DTO (no network). *First file.*
**Add** `crates/openqg-bench/src/zyal_genome/proposer.rs` with:
- `struct ProposedTheory` (serde DTO mirroring §2b) + `enum ProvenanceDto`.
- `fn parse_proposal(stdout: &str) -> Result<openqg_core::theory::Theory, ProposalError>`
  (reuse the `parse_live_verdict` brace-matcher, `critique.rs:55–71`).
- `enum ProposalError { NoJson, BadJson, MissingField, NonFinite, EmptyMechanism, … }`.

**Test (deterministic, mocked):** a `#[cfg(test)] mod tests` table:
- a hand-written valid JSON string parses to a `Theory` whose
  `run_veto_cascade` is **empty** (mirror the realistic survivor in
  `vetoes.rs:251–282` `a_realistic_alpha_t_zero_screened_modified_gravity_survives`);
- a JSON with `provenance.kind="free"` parses, then `run_veto_cascade` returns
  `VetoReason::FreeParameter` (mirror `vetoes.rs:144`);
- malformed strings (no brace / bad JSON / missing field / NaN α) each return the right
  `ProposalError` (§2d lethality);
- an LLM id is overwritten by the namespaced `m4-prop-*` id.

### Step 2 — Derivation checker + demotion.
**Add** `fn check_derivations(theory: &mut Theory, sketches: &[DerivationSketch])` in
`proposer.rs`: dimensional + limit checks (§3.1); on failure rewrite that `Parameter.provenance`
to `Provenance::Free`.
**Test:** a `Derived` parameter with a dimensionally-inconsistent sketch is demoted to `Free`, and
the theory is then vetoed by `run_veto_cascade` (closes H1 deterministically, no LLM).

### Step 3 — Gate function (wires §3 together).
**Add** `fn gate_proposal(stdout, observables, model, baseline_ll) -> Option<(Theory, CandidateAssessment)>`:
`parse_proposal` → `check_derivations` → `run_veto_cascade` (drop if non-empty) →
`assess` (`assessment.rs:36`). Returns `None` for any drop.
**Test:** feed a mocked valid stdout ⇒ `Some(_)` with `is_credible()`; feed a mocked gray-box
stdout ⇒ `None` (parallels `assessment.rs:97` `gray_box_is_not_credible`).

### Step 4 — Live call + receipt (network, behind env flag).
**Add** `JNOCCIO_PROPOSER_COMMAND` (mod.rs), `live_proposer_enabled()` + the `ZYAL_LIVE_PROPOSE_*`
readers (critique.rs pattern), `build_proposer_prompt(archive_elites)`, and
`run_live_proposal(...)` using `run_live_call_attempt` (`jailgun_live.rs:70`). Emit the
`record_kind: "live_proposal"` ledger.
**Test:** inject a mocked `LiveAttempt`-equivalent stdout through `gate_proposal` (the network call
is isolated so the test never spawns a subprocess) — assert the proposal is gated and the receipt
fields are populated. Transport-failure path returns `None` and penalizes nothing (mirror
`critique.rs:114–121`).

### Step 5 — Wire into the loop + recitation guard.
Hook seed + per-generation injection at the existing critic site
(`hybrid_artifacts.rs:422`), behind `ZYAL_LIVE_PROPOSER`. Add §4 novelty/held-out/fingerprint
penalties (reuse `novelty.rs`).
**Test:** with the proposer **disabled** (default), an end-to-end run is byte-identical to today
(no behavior change when off). With a **mocked proposer** returning one valid + one gray-box
proposal, the valid one can enter the archive and the gray-box one cannot — and the deterministic
baseline still wins if both LLM proposals are decoys (mirror
`evolve.rs:186` `structurally_broken_seeds_never_become_champions`).

### Determinism note
The mocked-LLM tests make M4 fully CI-testable without jnoccio. The live path is opt-in via
`ZYAL_LIVE_PROPOSER`; when off, `evolve` is unchanged and the existing determinism test
(`evolve.rs:173` `evolution_is_deterministic_in_the_seed`) still holds. When on, proposals are an
extra candidate source through the *same* gate, so the engine's correctness guarantees (vetoed ⇒
`final_fitness 0` ⇒ never champion) are preserved by construction.

---

## Appendix — cited files & symbols

- Principle source: `docs/research/automated-theory-discovery.md` §5 (LLM-as-physicist), §6.6
  (LLM as proposer + critic, never judge), §6.7 (anti-recitation), §2 (provenance verification
  ordering), §6.2 (veto cascade as pre-fitness gate), §6.8 (correspondence test).
- Types the LLM emits: `crates/openqg-core/src/theory/mod.rs` — `Theory`, `Parameter`,
  `Provenance` (`is_whitebox`), `Term`, `AlphaBasis`, `Stability`.
- Oracle: `crates/openqg-core/src/theory/vetoes.rs::run_veto_cascade`, `VetoReason`.
- Fitness: `crates/openqg-core/src/theory/assessment.rs::assess`, `CandidateAssessment::is_credible`.
- Loop: `crates/openqg-core/src/theory/evolve.rs::evolve`, `insert`, `behavior_cell`, `Cell`.
- Live machinery to reuse: `crates/openqg-bench/src/zyal_genome/` —
  `jailgun_live.rs::run_live_call_attempt`, `live_call.rs::run_live_call` (+ receipt bundle &
  `retrieval_packet`), `critique.rs` (`live_critic_enabled`, `env_usize`, `build_critique_prompt`,
  `parse_live_verdict`, `run_live_critique`, `LiveVerdict`), `mod.rs::JEKKO_LIVE_COMMAND`
  /`JNOCCIO_CRITIQUE_COMMAND`/`LiveAttempt`, `hybrid_artifacts.rs` (critic block,
  `ZYAL_LIVE_TOPK`/`ZYAL_LIVE_EVERY`/`ZYAL_LIVE_TIMEOUT`, `live_critique` ledger), `novelty.rs`.
