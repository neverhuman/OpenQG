# ZYAL — critical review of the 1000-gen run, and the next-level design

> Written 2026-06-08 as a stern internal review of `e78a493` (the 1000-generation production run)
> plus a concrete leveling-up plan: more discriminating data, the top-5 leading scoreable theories
> done justice, and a genuinely multi-agent ZYAL. Every criticism maps to a file:line; nothing here
> is hand-waved. Companion to `docs/zyal-engine-rebuild.md` and `docs/production-run-1000.md`.

---

## Part 1 — Critical review (what is real, what is oversold, what is load-bearing)

The rebuild did one big thing right: it made the **gates** honest. The veto cascade is structural
(`theory/vetoes.rs`), the forward model integrates a real FLRW background and *omits* what it cannot
derive (`cosmology/background.rs`), and provenance is a typed enum rather than a prose scan
(`theory/mod.rs:47`). Those are genuine, defensible improvements over the old identity-`forward_map`
engine. Keep all of it.

But the flagship narrative ("a co-evolving adversary sustained robustness-under-judge for 1000
generations and surfaced a champion that beats ΛCDM by 36.7 log-likelihood units") overstates what
the code does. Six issues, in descending order of how much they matter.

### 1.1 The "co-evolving adversary" is telemetry — it does not affect a single output (load-bearing)

`Adversary` (`theory/adversary.rs`) is one float, `frontier_margin`, that increments `+0.02`/gen and
rolls back when the anchor's pressured fitness drops below `0.10`. That by itself would be a weak
adversary. But the deeper problem is in `evolve_run` (`theory/evolve.rs`):

- archiving uses `assessment.final_fitness` (`insert`, evolve.rs:57);
- the returned champion uses `final_fitness` (`best_champion`, evolve.rs:133, 252);
- `frontier_margin` / `pressured_fitness` appear **only** inside the `GenerationReport` struct
  (evolve.rs:238–245).

The adversary never feeds back into mutation, archiving, or selection. **The 1000-gen run produces
bit-identical archive and champion with the adversary deleted.** "Honesty held for 1000 generations"
reduces to "a logged scalar equilibrated at `anchor_fitness − floor ≈ 0.28` and we printed it each
generation." It is not a 1000-round game; it is a thermostat reaching setpoint, recorded as a
sideband. This is the claim most likely to be torched by an outside reviewer, and it is the easiest
to fix (Part 4).

### 1.2 "Symbolic discovery" is still 7-float parameter evolution (load-bearing)

`mutate()` (`theory/mutation.rs:54`) jitters exactly `{α_m, α_b, α_k, h, Ω_m, w0}` plus a screening
flag. `Term`s — the actual symbolic structure (mass dimension, free Lorentz indices) that the whole
`Theory` type exists to carry — are **never mutated**: `proposal.rs:183` and `mutation.rs` copy
`base.terms` verbatim. The MAP-Elites behavior descriptor is `(modifies_gravity ∈ {0,1},
param_count.min(5), fitness_bin ∈ {0,1,2})` (evolve.rs:24) — ≤ 36 cells, 17 reached.

So despite the "symbolic Theory, not a float vector" framing, the *evolving genome* is the same ~7
numbers the old engine had. That is why the champion plateaus at gen ~50 and the remaining 950
generations are a no-op (QD 8.29 / 17 cells, flat). The rebuild fixed the **judge** but kept the old
**generator**. There is no structural search happening.

### 1.3 "Derived, not fit" holds for the *form*, not the *value* (load-bearing)

This is the subtle one and it goes to the project's core "whitebox only / no parameter-fitting"
hard gate. The provenance veto (`vetoes.rs`) kills *Free-typed* parameters, and the M4
derivation-checker (`proposal.rs:103`) demotes "derived" claims whose `mechanism` string is empty or
whose `derived_from` symbols are unknown. Good. **But:**

- The "Fundamental" params `H0`, `Ω_m` and the background `w0, wa, α_m, α_b, α_k` are all freely
  tuned by `mutate()` to fit the data. A `Fundamental` label does not constrain a value — the
  champion's `H0 = 68.96`, `w0 = −1.032` were *fit*, not derived. The whitebox gate forbids *adding*
  a knob; it does nothing about *fitting the knobs that are already there*.
- The derivation-checker never checks that the *number* follows from the mechanism. A proposal
  `α_M0 = 0.05, "derived from Ω_m via conformal coupling β"` passes with `0.05` arbitrary
  (`proposal.rs:118–128` checks only string-non-empty + symbol-exists). The value is still free,
  just relabeled.

Net: the champion's `ε = +36.7` is **w0waCDM continuous-parameter inference** — exactly the
curve-fitting the rebuild claimed to abolish, with the knobs relabeled "fundamental." That is a
legitimate thing to do (it *is* the DESI evolving-DE story), but it must be *described* as fitting an
allowed model class, not as "derived." To make "derived, not fit" true at the level of *value*, the
engine needs a derivation oracle that checks `value == f(mechanism inputs)` — the AI-Hilbert /
Positivstellensatz certificate the roadmap names but never built (Part 4.3).

### 1.4 ε = +36.7 is not a fair model-selection number

Three reasons it cannot be quoted as "beats ΛCDM by 36.7":

1. **Diagonal likelihood, no covariance.** `score_metrics` (`scoring/likelihood.rs:27–46`) sums
   independent Gaussians with `σ = max(obs_σ, pred_σ)`. DESI BAO `D_M`/`D_H` at the same tracer are
   correlated; the Planck distance priors `(R, ℓ_A, ω_b)` come with a 3×3 covariance. Treating them
   diagonal mis-estimates χ² and inflates Δχ².
2. **The baseline is fixed, not re-fit.** `Theory::baseline_lcdm()` is pinned at Planck `H0=67.4,
   Ω_m=0.315, w=−1` (mod.rs:169) and never re-optimized to *this* dataset, while the champion's
   params *are* fit to it. "+36.7 vs baseline" compares a fitted model to an unfitted reference. A
   fair number re-fits ΛCDM to the same data first. (Δχ² ≈ 73 is implausibly large for DESI DR1 +
   CMB-prior + BBN; the real evolving-DE preference is Δχ² ~ a few.)
3. **No complexity penalty in the selected objective.** `combined_fitness = sigmoid(0.12·ε)·β`
   (evaluate.rs:40) rewards raw ε. AIC/BIC/MDL are computed (`likelihood.rs:52–54`) but never used in
   selection; the Pareto `parsimony` axis exists (`pareto.rs`) but the *reported champion* is
   `max final_fitness`, not a Pareto/evidence choice. Two extra parameters (`w0, wa`) buy ε for free.

The honest replacement is profile-likelihood or Bayesian evidence: re-fit both models, compare
best-fit χ² with ΔAIC/ΔBIC or Δln(Z). Then "champion beats ΛCDM" becomes defensible.

### 1.5 "Unification" is analytic self-consistency, not cross-domain *data*

The five "cross-domain" checks (`theory/unification.rs`) are closed-form functions of the *same*
genome: `α_T` (already a veto), a binary `screening.is_some()`, `Y_p(n_eff)` (which *duplicates* the
BBN observable already in the data fit), and a siren ratio `(1+z)^(−α_M/2)`. None ingests an
*independent dataset* — no real LVK siren-H0 posterior, no MICROSCOPE η, no Eöt-Wash, no GW170817 Δt,
no D/H. A candidate earns "unified" for free by setting `α_T=0` and writing `screening: "vainshtein"`.
The `score = MIN over domains` design is right; the domains are just placeholders for real likelihoods.

### 1.6 Coverage 1.0 is over a deliberately narrow set

All 15 observables are FLRW-background distances + `Y_p` + two CMB distance priors. "Coverage 1.0"
means "predicts all 15 background quantities," not "covers cosmology." Growth (`fσ8, S8`), full CMB
`C_ℓ` (TT/TE/EE), lensing, Pantheon+ SNe, the real GW siren H0, D/H — all absent. The champion's
`α_K = −0.134` (and `α_B`) — the entire *point* of modified gravity — is unconstrained by the data
being fit, because kineticity/braiding barely touch background distances. The doc admits this, but it
guts the "beats ΛCDM" headline: the only part that beats ΛCDM is the `w0` background fit, i.e. plain
w0waCDM. **The discriminating data for everything interesting is exactly the data not yet wired.**

### 1.7 Minor but worth noting

- The live LLM proposer (`--proposer-cmd`) was **not** used in prod-1000; it was seeded from 4 static
  fixtures (`run-config.json: seeds=4`). The "LLM proposes, never judges" architecture exists but was
  not exercised in the flagship.
- Two engines still coexist: the new `theory_evolve` (openqg-core, LLM-free) and the legacy
  `zyal_genome` (openqg-bench, jailgun/jnoccio-routed). The legacy retirement plan
  (`docs/legacy-retirement-plan.md`) is not executed.

**Bottom line.** The gates are good and honest. The generator is weak, the adversary is inert, the
data is too thin to test the physics the champion claims, and the marquee number is a fitted-vs-fixed
comparison under a diagonal likelihood. The next level is not "more generations" — it is **discriminating
data + a real generator + a real adversary + fair model selection.**

---

## Part 2 — Leveling up data coverage (the discriminating tiers)

Each tier is a new `kind` of `ObservableRecord` *plus* the forward-model sector needed to predict it.
Priority is by discriminating power per unit of engineering, given the genome already carries the
α-basis. The registry scaffolding for most sources already exists (`data/registry/*.yml`:
desi-dr1, planck-legacy, sparc, gwosc, hepdata, pdg-api).

| Tier | Observables | New physics needed | Why it matters | Lift |
|---|---|---|---|---|
| **T1 Growth** | `fσ8(z)` (DESI DR1 full-shape, eBOSS, 6dF/BOSS), `S8` (DES-Y3, KiDS-1000) | linear growth ODE `D''(a)` with `G_eff(a,k;α_i)` (Bellini–Sawicki) | **The** constraint on `α_B, α_M, α_K`. Breaks the degeneracy §1.6 admits. Tests every MG theory and the S8 tension. | **M** |
| **T2 SNe + ladder** | Pantheon+ `μ(z)` (1701 SNe + full cov), SH0ES `H0` prior | none (background-only) — but use the covariance | Adjudicates the **H0 tension** (the headline physics). Cheap, high value. | **S** |
| **T3 Real cross-domain likelihoods** | GW170817 Δt → `c_GW`; LVK dark/bright-siren `H0`; Cassini `γ−1=(2.1±2.3)e−5`; MICROSCOPE `η<1e−15`; BBN `D/H` | turn the §1.5 analytic stubs into likelihoods; compute the screening's *actual* solar-system recovery (Vainshtein radius) | Makes "unification" mean *consistency with independent data*, not self-consistency. | **M** |
| **T4 Full CMB** | Planck plik_lite TT/TE/EE (or CMB-lite), lensing `φφ` | Boltzmann backend (`BoltzmannForwardModel` via CLASS/hi_class behind `--features physics-stack`, already designed) | Damping tail, ISW (constrains `α_B`), EDE's recombination-era signature. The big one for QG-adjacent claims. | **L** |
| **T5 Galactic / DM-vs-MG** | SPARC rotation curves (`sparc.yml`), cluster lensing, Bullet | a galactic-dynamics predictor (deep-MOND `a_0`, or NFW for CDM) | Discriminates dark matter vs modified inertia/gravity (MOND/RelMOND/Verlinde). | **L** |

Recommended order: **T2 → T1 → T3 → T4 → T5.** T2 is a day's work and immediately makes the H0
story real; T1 is the single highest-value physics addition; T3 makes unification honest; T4/T5 are
the Boltzmann/galactic lifts. Every tier also needs the **covariance-aware likelihood** of §1.4
(generalize `score_metrics` to a block-Gaussian `−½ rᵀ C⁻¹ r`).

---

## Part 3 — The top-5 leading scoreable theories, done justice

"Scoreable" is itself a filter that does the field justice: a theory earns a score only if it makes a
*derived, falsifiable* prediction on data we hold. Pure-QG programs (string landscape, LQG, CDT,
asymptotic safety) are **not** directly scoreable here — and that is the correct, honest verdict —
*except* through a low-energy effective handle (a specific running-Λ, a bounce signature, a fixed
`α_i(a)`), which is how we admit them. The five below are the leading contenders the engine can
discriminate **once Tiers 1–4 land**. Each needs (a) a proper `Theory` encoding with *derived*
parameters, (b) the forward-model sector that discriminates it, (c) a fair model-selection score.

1. **ΛCDM (GR + Λ + CDM)** — the incumbent and the reference. *Justice:* use the actual joint
   Planck+DESI best-fit with full covariance, re-fit to each dataset, as the null every challenger
   must beat on ΔAIC/Δln(Z) — not the fixed strawman of §1.4.2.

2. **w0waCDM / evolving dark energy (CPL)** — the DESI DR2 headline; what the champion already
   rediscovered. *Justice:* already expressible (`w0, wa`); score it with the 2-parameter penalty and
   the BAO+SNe+CMB covariance so the "preference" is a real Δln(Z), not +36.7.

3. **Early Dark Energy (EDE)** — the leading H0-tension resolver (Poulin et al.; axion-like
   `V(φ)=V0(1−cosφ)ⁿ` active near `z~3500`). *Justice (and the sharpest test of the engine's honesty):*
   today the engine would **kill** an `f_ede` knob as a Free parameter — correctly, *if it is a knob.*
   To do EDE justice it must be encoded as a **scalar-field background sector** where `f_ede` is
   *computed* from the field dynamics `(V0, n, z_c, θ_i)`, then it modifies `r_drag` and the CMB. This
   requires (i) a scalar-field background integrator and (ii) Tier-4 CMB. It is the cleanest
   demonstration that the engine rewards *derived* new physics and rejects the *fitted* version of the
   same idea.

4. **Screened scalar-tensor MG — f(R) Hu–Sawicki / nDGP / covariant Galileon** — the leading modified-
   gravity class, the α-basis's home turf. *Justice:* stop treating `α_i` as constants to fit; encode
   the model's actual `α_i(a)` time-dependence *derived* from its Lagrangian (e.g. f(R): `α_M=α_B`
   relation + chameleon; nDGP: `α_B(a)` from the crossover scale `r_c`), with the screening's real PPN
   recovery (Tier-3), scored against growth `fσ8` (Tier-1). This is where "derived not fit" must bite:
   one fundamental parameter (`f_R0` or `r_c`) generating the whole `α_i(a)`.

5. **Interacting / coupled dark energy (coupled quintessence)** — leading joint H0+S8 alternative;
   energy exchange `Q` between DE and DM from a conformal/disformal coupling `β`. *Justice:* the
   prod-1000 run already **demoted-and-killed** a hand-wavy `xi_dm` — exactly right. Doing it justice
   means encoding coupled quintessence with `β` *fundamental* and `Q`, the modified `Ω_m` dilution, and
   the growth suppression all *derived from* `β` — then scoring on background + growth (Tiers 1–2).

*Honorable mention, gated by Tier-5:* MOND / RelMOND (Skordis–Złośnik) and Verlinde emergent gravity —
discriminated by SPARC + CMB; admit them once the galactic sector exists.

The deliverable is a **theory league table**: each of the five encoded as an anchor `Theory`, re-fit
to the full multi-tier data with covariance, ranked by Δln(Z)/ΔAIC vs ΛCDM, each with its
cross-domain consistency vector. That artifact — five real theories scored head-to-head, fairly — is
far more compelling than one evolved near-ΛCDM champion.

---

## Part 4 — A genuinely multi-agent ZYAL (agents, memory, real adversary, more jailgun)

The current agent stack: `jekko` (event-sourced run daemon, `.jekko/`), `jailgun` (MCP→Claude+browser
over `http://127.0.0.1:8797/mcp`), `jnoccio` (subprocess LLM via `rtk jekko run --provider jnoccio`),
and the `--proposer-cmd` hook (wired, unused in prod). The invariant is right and must be kept: **an
LLM may propose and critique; it may never judge — every claim clears a deterministic oracle.** The
upgrade keeps that invariant and uses agents to do the *structural* search `mutate()` cannot.

### 4.1 A proposer *fan-out* — the "more jailgun calls" with a purpose

Replace the single static seed with N parallel jailgun proposer agents per generation, each pinned to
a **distinct physical lens** so the fan-out is diverse, not redundant:

- `proposer:mg` (Horndeski/α-basis), `proposer:ede` (scalar-field early-DE),
  `proposer:ide` (coupled dark sector), `proposer:dm` (galactic/DM-vs-MG),
  `proposer:qg-effective` (running-Λ / bounce / fixed-`α_i` low-energy handles).

Each emits a `Theory` AST JSON (the existing `TheoryProposal` schema, extended with **term-grammar
edits** so agents can propose *new action terms*, not just numbers — this is what fixes §1.2). All
outputs flow through `proposal_into_theory` → veto cascade → real forward model. The agent never sees
the score until the oracle has spoken.

### 4.2 A real adversary agent (replaces the inert thermostat of §1.1)

Each generation an `adversary:redteam` jailgun agent reads the current champion and must produce, as
structured output: (a) a **decoy** that *should* die but is engineered to look like the champion
(keeps the honesty calibration sharp and growing, not frozen), and (b) the **held-out observable or
regime** the champion has not been tested on (drives the next data pull). The honesty metric becomes
real: *did any agent-authored decoy survive the oracle? did the champion's claimed coverage hold
out-of-sample?* — adjudicated by the deterministic gates, never by the agent. This is the
`alternating_holdout` machinery (`theory/holdout.rs`) given an active opponent. Wire the result back
into selection (pressured fitness *actually* gates champion eligibility) so the adversary stops being
telemetry.

### 4.3 A derivation oracle + skeptic pair (makes §1.3 true)

To upgrade β from "label" to "certificate": a `proposer` agent that claims `derived` must also emit a
**derivation sketch** (the algebra from mechanism inputs to the parameter value). A deterministic
**derivation oracle** checks `value == f(inputs)` numerically/symbolically (start: closed-form
relations like f(R)'s `α_M=α_B`, coupled-DE's `Q(β)`; grow toward an AI-Hilbert Positivstellensatz
certificate where the axioms allow). A `skeptic` agent is spawned to *refute* each derivation
(adversarial-verify pattern: N skeptics, default-to-refuted). Disputes resolve at the oracle, never by
vote. Only oracle-certified derivations raise β; everything else is demoted to Free and killed — so a
parameter's *value* is now gated, not just its type.

### 4.4 Smarter jekko + ZYAL memory (persistent reasoning, not per-run amnesia)

The `jekko` literature-radar daemon already runs (`.jekko/daemon/…`, iteration 47, primary-source QG
radar on a 15-min loop). Make it the engine's long-term memory with three stores:

- **Derivation lemmas** proven once by the oracle (4.3), reused across runs — the engine stops
  re-deriving `α_M=α_B` every generation.
- **Refuted regions** — theory neighborhoods the adversary/oracle killed, so proposers don't re-walk
  dead ground (this is the cure for §1.2's premature convergence — diversity pressure with memory).
- **Live constraints** — when literature-radar finds a new bound or data release (a new DESI DR2
  point, a tighter MICROSCOPE η), it updates the anchor/decoy set and the data tiers automatically.

This is the "memory + enhanced reasoning" upgrade: ZYAL accumulates a derivation knowledge base and a
falsification map rather than starting blank each run.

### 4.5 The loop, end-to-end

```
jekko (memory: lemmas, refuted-regions, live-constraints)
   │  seeds proposer priors + current frontier
   ▼
proposer fan-out (jailgun ×N, diverse lenses, term-grammar edits)        [LLM proposes]
   ▼
derivation oracle + skeptic refutation  ─────────────────────────────────[deterministic + LLM critique]
   ▼   (β certified, not labeled)
veto cascade → real forward model (Tiers 1–4) → cov-aware likelihood     [deterministic JUDGE]
   ▼
cross-domain real-data likelihoods (MIN gate)                            [deterministic JUDGE]
   ▼
Pareto / Bayesian-evidence selection (ΔAIC, Δln Z)                       [fair model selection]
   ▼
adversary redteam: new decoy + held-out target  ────────────────────────[active opponent]
   ▼
memory update (lemmas, refuted regions) → next generation
```

LLMs only ever occupy *propose / critique / derive-sketch* roles; every box that decides survival is
deterministic. That is the existing invariant, scaled up.

---

## Part 5 — Prioritized recommendation

Highest value per unit work, in order:

1. **Honesty fix (S, hours).** Make the adversary feed selection, OR relabel it accurately in the
   docs. Today's "1000-gen adversarial" claim is the biggest reviewer risk (§1.1).
2. **Fair scoring (S–M).** Covariance-aware likelihood + re-fit-the-baseline + report ΔAIC/Δln(Z).
   Turns +36.7 into a defensible number (§1.4).
3. **Tier-2 SNe+ladder, then Tier-1 growth (S, then M).** The H0 story made real, then the constraint
   that actually tests modified gravity (§2).
4. **The 5-theory league table (M).** Encode the five as anchors, score head-to-head fairly (§3).
   This is the compelling artifact.
5. **Real generator: term-grammar mutation + proposer fan-out (M–L).** Fix premature convergence and
   make "discovery" mean structural search (§1.2, §4.1).
6. **Derivation oracle + memory (L).** Make "derived not fit" true at the value level; give ZYAL
   persistent reasoning (§1.3, §4.3–4.4).
7. **Tier-4 Boltzmann + Tier-5 galactic (L).** The remaining physics depth (§2).

Items 1–4 are a focused sprint that would replace the current overstated headline with a genuinely
strong, defensible one. Items 5–7 are the ambitious multi-agent build.
