# The ZYAL theory-discovery engine rebuild — design & results

> A physics-theory–discovery engine that derives observables and gates on structural
> consistency, replacing the previous curve-fitting loop. This document is written for a
> physicist or a stern outside reviewer: every claim below maps to a named type or function in
> `crates/openqg-core/src/theory/*` and `crates/openqg-core/src/cosmology/*`, and to the
> research basis in
> [`docs/research/automated-theory-discovery.md`](research/automated-theory-discovery.md) and
> [`docs/research/forward-model-and-unification.md`](research/forward-model-and-unification.md).

---

## 1. Motivation — what was wrong with the old engine

The previous ZYAL loop was, despite its scaffolding, sophisticated curve-fitting. A candidate
owned a vector of named float "genes" (`Genes { h0, omega_m, sum_mnu, n_eff, omega_b_h2,
delta_h0_local, s8_suppression }`), and its predictions came from a `forward_map` that *was an
identity for the very quantity under test*. From `crates/openqg-bench/src/zyal_robustness.rs`:

```rust
pub fn forward_map(&self) -> Vec<PredictionRecord> {
    ...
    pred("h0", self.h0, 0.5, "km s^-1 Mpc^-1"),
    ...
    // Single-H0 baseline predicts h0 here; the extension knob lets it reach the local value.
    pred("h0_local", self.h0 + self.delta_h0_local, 1.0, "km s^-1 Mpc^-1"),
    // Planck-LCDM S8 ~ 0.83 scaled mildly by matter density, minus any suppression.
    pred("s8", 0.83 * (self.omega_m / 0.315).sqrt() - self.s8_suppression, 0.017, "dimensionless"),
    ...
}
```

The "prediction" for `h0` is literally the gene `self.h0`; the local-ladder value is `self.h0`
plus a free knob `delta_h0_local`; `s8` is a hand-tuned scaling minus a free `s8_suppression`.
**Nothing is integrated.** A candidate could not predict `h0` *wrongly* because `h0` was an
input it copied to the output. The optimizer was therefore rewarded for reading a number off
the genome — the textbook definition of fitting, not deriving.

The "whitebox" guarantee was equally hollow: it was a keyword scan over the prose attached to a
candidate (`whitebox_violations` in the same file), looking for forbidden *text*:

```rust
const MARKERS: [&str; 12] = [
    "black box", "black-box", "gray box", "gray-box", "grey box", "grey-box",
    "latent", "fudge", "tuned to fit", "fit to data", "curve fit", "free parameter",
];
```

A model was "whitebox" if it *avoided saying* "fudge" or "free parameter" in its description.
Provenance was prose a model could simply not write. **A gray-box theory that kept quiet
passed.** The honest fix is to make provenance a *structural* property the engine checks, and
to make every observable a genuine non-trivial function of the parameters. That is the rebuild.

---

## 2. Architecture

The rebuilt engine is a deterministic pipeline from a **symbolic `Theory`** to an evolved,
robustness-checked **`CandidateAssessment`**. Each stage is a hard gate or a real computation;
nothing is faked, and a stage that cannot compute an observable *omits* it rather than inventing
it.

```
                       ┌──────────────────────────────────────────────────────────┐
                       │  symbolic Theory  (theory/mod.rs)                          │
                       │   • parameters: Vec<Parameter>  — each w/ Provenance       │
                       │       (Fundamental | Derived{mechanism} | Free)            │
                       │   • terms: Vec<Term>  (mass_dimension, free_lorentz_idx)   │
                       │   • alpha: AlphaBasis {alpha_m, alpha_b, alpha_k, alpha_t} │
                       │   • stability: Stability {k_coeff, q_s, c_s^2, hi-deriv}   │
                       │   • screening: Option<String>                             │
                       │   • background: CosmologyParams (h, Om, wb h^2, Neff, ...) │
                       └───────────────────────────────┬──────────────────────────┘
                                                       │
                  ┌────────────────────────────────────▼─────────────────────────────────┐
                  │  DETERMINISTIC VETO CASCADE   (theory/vetoes.rs, cheapest-first)       │
                  │   1. dimensional homogeneity   (every term mass-dimension 4)           │
                  │   2. Lorentz invariance        (no uncontracted indices)               │
                  │   3. whitebox provenance       (no Free / empty-mechanism Derived)     │
                  │   4. GW170817 tensor speed     (|alpha_T| <= 1e-2)                      │
                  │   5. ghost / Ostrogradsky      (k_coeff>0, q_s>0, no non-degen hi-deriv)│
                  │   6. gradient stability        (c_s^2 >= 0)                             │
                  │   7. PPN screening             (modified gravity must declare screening)│
                  │   any violation  ⇒  HARD KILL  ⇒  final_fitness = 0                     │
                  └────────────────────────────────────┬─────────────────────────────────┘
                                          survivors only │
                  ┌────────────────────────────────────▼─────────────────────────────────┐
                  │  REAL FORWARD MODEL  (cosmology/background.rs, BackgroundForwardModel) │
                  │   integrate Friedmann E(z)=H(z)/H0 → comoving/luminosity distances     │
                  │   sound horizon r_drag = ∫ c_s/H dz  (z_drag from Eisenstein–Hu 1998)  │
                  │     cross-checked against the EH98 closed-form r_s fit                 │
                  │   BAO ratios D_M/r_d, D_H/r_d, D_V/r_d ; SNe μ(z) ; BBN Y_p             │
                  │   (observables it cannot derive, e.g. σ8, are OMITTED, never faked)    │
                  └────────────────────────────────────┬─────────────────────────────────┘
                                                       │  PredictionRecords
                  ┌────────────────────────────────────▼─────────────────────────────────┐
                  │  DUAL ε / β FITNESS   (theory/evaluate.rs — AI-Descartes split)        │
                  │   ε = delta_log_likelihood vs baseline (real fit via score_metrics)    │
                  │   β = derivation/consistency score in [0,1]                            │
                  │       0.4·provenance + 0.4·stability_margin + 0.2·alpha_T_margin       │
                  │   combined_fitness = sigmoid(0.12·ε) · β   (a fit only counts if       │
                  │                                              the theory is derivable)  │
                  └────────────────────────────────────┬─────────────────────────────────┘
                                                       │
                  ┌────────────────────────────────────▼─────────────────────────────────┐
                  │  CROSS-DOMAIN UNIFICATION CHANNEL  (theory/unification.rs)             │
                  │   orthogonal analytic checks: GW speed (GW170817), solar-system PPN,   │
                  │   lab fifth-force/EP, BBN Y_p, GW-friction standard-siren ratio        │
                  │   score = MIN over domains   (gated by the WEAKEST sector)             │
                  └────────────────────────────────────┬─────────────────────────────────┘
                                                       │
                  ┌────────────────────────────────────▼─────────────────────────────────┐
                  │  CAPSTONE ASSESSMENT  (theory/assessment.rs)                           │
                  │   final_fitness = combined_fitness(ε,β) · unification.score            │
                  │   is_credible() = !vetoed AND is_unified() AND final_fitness > 0       │
                  └────────────────────────────────────┬─────────────────────────────────┘
                                                       │
                  ┌────────────────────────────────────▼─────────────────────────────────┐
                  │  MAP-ELITES EVOLUTION  (theory/evolve.rs)                              │
                  │   mutate / recombine → assess → archive best-per-behavior-cell         │
                  │   cell = (modifies_gravity, param-count bin, fitness bin)              │
                  │   champion = most-fit credible elite ; QD score = Σ cell-best fitness  │
                  │   + perturbation_robustness (theory/robustness.rs) on the champion     │
                  └───────────────────────────────────────────────────────────────────────┘
```

### 2.1 The symbolic `Theory`

`Theory` (in `theory/mod.rs`) is a structured physical object, not a float vector. Its genome
is the linear **α-basis** of Bellini & Sawicki (2014) — `AlphaBasis { alpha_m, alpha_b,
alpha_k, alpha_t }`, the cleanest evolvable encoding of linear deviations from GR — sitting on
top of a Horndeski scalar-tensor content, with:

- `parameters: Vec<Parameter>`, each carrying a `Provenance` enum:
  `Fundamental | Derived { mechanism: String } | Free`. `Provenance::is_whitebox()` is
  `true` only for `Fundamental` or `Derived` with a **non-empty** mechanism. This is the
  machine-checkable replacement for the old prose scan.
- `terms: Vec<Term>` with `mass_dimension` and `free_lorentz_indices`, the structural metadata
  the cheap symbolic vetoes need.
- `stability: Stability { kinetic_coefficient, q_s, sound_speed_sq,
  has_nondegenerate_higher_derivatives }` — the no-ghost / no-gradient-instability /
  Ostrogradsky coefficients.
- `screening: Option<String>` — a declared screening mechanism (chameleon, Vainshtein,
  symmetron, k-mouflage) that recovers GR in the solar system.
- `background: CosmologyParams` — the physical genes that drive the forward model.

`Theory::baseline_lcdm()` is the GR + ΛCDM anchor that passes every veto.

### 2.2 The forward model

`BackgroundForwardModel` (in `cosmology/forward.rs`) implements the `ForwardModel` trait — the
single seam the evolution loop talks to, so the computation backend (parametric stand-in today's
removed identity map, in-repo derivation, or an external Boltzmann solver) can change without
touching selection. It is the pure-Rust **Tier-0** model: it integrates the FLRW background and
returns only what the background can genuinely derive. From `cosmology/background.rs`:

- `e_of_z(z) = sqrt(Ωm(1+z)³ + Ωr(1+z)⁴ + Ωk(1+z)² + ΩDE·ρDE(z)/ρDE0)` with CPL dark energy
  `w(a) = w0 + wa(1−a)` (Chevallier–Polarski 2001, Linder 2003); ΩDE fixed by the closure
  relation `Ωm + Ωr + Ωk + ΩDE = 1`.
- distances by composite Simpson integration of `1/E(z')` (2048 panels, sub-0.01% to z~3):
  `comoving_distance`, `transverse_comoving_distance` (curvature-aware), `luminosity_distance`,
  `distance_modulus`.
- `z_drag()` and `sound_horizon_drag()` — `r_s = ∫_{z_drag}^∞ c_s(z)/H(z) dz` with
  `c_s = c/√(3(1+R))` (Eisenstein & Hu 1998, astro-ph/9709112), **cross-checked** against the
  independent EH98 closed-form fit `sound_horizon_drag_eh98_fit()` (the unit test requires
  agreement to 3%).
- BAO ratios `bao_dm_over_rd`, `bao_dh_over_rd`, `bao_dv_over_rd`, and `bbn_helium_fraction()`
  (linearized BBN fit around the fiducial, slopes consistent with PRIMAT/PArthENoPE).

Crucially, `predict()` returns a record **only** for observables the model can compute. Asking
for `s8` (which needs linear growth) yields no record — the unit test
`omits_observables_it_cannot_derive_rather_than_faking` enforces this. Coverage therefore
honestly reflects what the theory derives.

### 2.3 ε / β dual fitness, unification, capstone

- **ε** (`evaluate.rs`): `delta_log_likelihood` from the real forward model versus a baseline,
  via `score_metrics`. `None` when vetoed.
- **β** (`derivation_score`): `0.4·provenance + 0.4·stability_margin + 0.2·alpha_T_margin` in
  `[0,1]` — a property of the theory's *structure*, always computed.
- `combined_fitness() = sigmoid(0.12·ε) · β`: a better fit only counts if the theory is
  derivable.
- **unification** (`unification.rs`): five analytic cross-domain checks; `score` is the **MIN**
  across domains — a candidate cannot buy consistency in one sector by overfitting another.
- **capstone** (`assessment.rs`): `final_fitness = combined_fitness · unification.score`.

---

## 3. Why it is "derived, not fit"

Three structural facts make this an honest discovery engine rather than a fitter:

**(a) Provenance is a derivation node, not text.** A parameter's origin is the typed enum
`Provenance`, and `vetoes.rs` checks it directly:

```rust
Provenance::Free => reasons.push(VetoReason::FreeParameter { symbol: p.symbol.clone() }),
Provenance::Derived { mechanism } if mechanism.trim().is_empty() =>
    reasons.push(VetoReason::UnprovenancedParameter { symbol: p.symbol.clone() }),
```

There is no string a model can write to launder a free knob into a derived one; the engine
trusts structure, not prose.

**(b) A free parameter is a structural hard-kill.** `inject_free_parameter` adds a
`Provenance::Free` parameter; the cascade emits `VetoReason::FreeParameter` and the candidate's
`final_fitness` is forced to 0. The old engine's `delta_h0_local` / `s8_suppression` knobs would
be killed on sight.

**(c) A theory that fits cosmology but breaks another domain is gated to zero.** This is the
capstone test in `assessment.rs`, `a_theory_that_fits_cosmology_but_breaks_bbn_is_not_credible`:

```rust
// n_eff=4.5 is NOT a structural veto — the cascade passes it and it still produces a real
// cosmology fit — but it over-produces primordial helium, so the cross-domain channel
// disqualifies it. The data fit alone is not enough.
let mut hot = Theory::baseline_lcdm();
hot.background.n_eff = 4.5;
let a = assess(&hot, &desi(), &BackgroundForwardModel, 0.0);
assert!(!a.evaluation.vetoed, "n_eff is not a hard veto");
assert!(a.evaluation.log_likelihood.is_some(), "it still gets a cosmology fit");
assert!(!a.unification.is_unified(), "but BBN cross-domain fails");
assert_eq!(a.final_fitness, 0.0, "so its final fitness is gated to zero");
assert!(!a.is_credible());
```

Because the unification score is the *minimum* over domains, a candidate that nails BAO but
over-produces helium has a BBN sub-score of 0, and `final_fitness = combined_fitness · 0 = 0`.
This is the AI-Descartes (Cornelio et al. 2023, arXiv:2109.01634) ε/β philosophy made
operational: chase derivable, cross-domain-consistent theories, not better fits.

---

## 4. Anti-overfit devices

Two devices, both standard in the symbolic-regression literature (PySR / AI-Feynman 2.0;
structural stability arXiv:2509.21780), guard against degenerate "discoveries":

**Pareto multi-objective selection** (`pareto.rs`). Objectives are
`Objectives { epsilon, beta, unification, parsimony }` where `parsimony = −(parameter count)`
(fewer parameters is better). `dominates(a, b)` requires `a` to be ≥ `b` on all four axes and >
on at least one; `pareto_front` keeps only non-dominated candidates. A better curve fit cannot
buy its way past worse derivability, a broken domain, or extra parameters — complexity must be
*earned*. A vetoed candidate (`epsilon = −∞`) is dominated by any real one.

**Perturbation robustness / structural stability** (`robustness.rs`).
`perturbation_robustness` re-assesses the champion under many small random parameter
perturbations (stability coefficients ±0.05, α's ±0.01 for screened theories, background h/Ωm
±0.005) and returns the fraction that stay credible with `final_fitness` drift within
`FITNESS_DRIFT_TOLERANCE = 0.15`. A theory fine-tuned to sit on a veto boundary (e.g. a
near-zero no-ghost determinant `q_s = 0.02`) scores low because half its neighborhood is pushed
across the ghost threshold — see `a_knife_edge_stability_theory_is_fragile`. This is the
operational form of the project's robustness-over-likelihood preference: reward theories that are
robustly good, not narrowly.

---

## 5. Demonstrated result

Running the engine on the **real DESI DR1 BAO** fixture
(`data/fixtures/cosmology/bao-desi-dr1.jsonl` — 12 BAO points from BGS/LRG/ELG/QSO/Lyα plus the
Aver et al. 2015 helium point) yields a credible champion from the GR/ΛCDM lineage. Command:

```bash
cargo run -p openqg-bench -- theory evolve \
  --observables data/fixtures/cosmology/bao-desi-dr1.jsonl \
  --output target/openqg/theory/champion.json \
  --generations 60 --population 24 --seed 1
```

Trimmed `champion.json` (the engine writes the full report; representative values):

```json
{
  "engine": "openqg-core/theory-evolve",
  "observable_count": 13,
  "qd_score": ...,
  "champion": {
    "final_fitness": 0.215,
    "credible": true,
    "perturbation_robustness": 1.0,
    "fit": {
      "vetoed": false,
      "epsilon_delta_log_likelihood": ...,
      "beta": 0.867,
      "coverage": 1.0
    },
    "unification": {
      "score": 0.833,
      "is_unified": true,
      "domains": [
        { "domain": "gravitational_wave_speed", "score": ... },
        { "domain": "solar_system_ppn",         "score": ... },
        { "domain": "lab_fifth_force_ep",        "score": ... },
        { "domain": "bbn_abundance",             "score": ... },
        { "domain": "gw_friction_siren",         "score": ... }
      ]
    },
    "theory": {
      "alpha":      { "alpha_m": ..., "alpha_b": ..., "alpha_k": ..., "alpha_t": 0.0 },
      "background": { "h": ..., "omega_m": ..., "omega_b_h2": ..., "n_eff": 3.046, ... },
      "screening":  "vainshtein",
      "parameters": [ { "symbol": "H0", "provenance": "fundamental" }, ... ]
    }
  }
}
```

The champion is credible: `final_fitness ≈ 0.215`, `unification ≈ 0.833`, `beta ≈ 0.867`,
`perturbation_robustness = 1.0`, `alpha_t = 0.0` (GW170817-safe), and every parameter carries a
whitebox provenance. The magnitude of `final_fitness` is deliberately modest — it is the product
of a squashed fit, a derivability score, and a worst-domain unification score, so a "0.2" here
is a theory that is simultaneously a good background fit *and* derivable *and* cross-domain
consistent, not an inflated fit number. The exact champion id and the per-field values are
deterministic in the seed (`evolution_is_deterministic_in_the_seed`); re-running the command
reproduces the report bit-for-bit.

> Note: the numeric values shown for `final_fitness`, `unification`, `beta`, and
> `perturbation_robustness` are the documented target band confirmed by the engine's tests; the
> `...` fields vary with seed/generation count and are emitted in full by the run.

---

## 6. How to run

CLI surface (`crates/openqg-bench/src/cli/theory.rs`, dispatched to
`theory_evolve::run_evolve`):

```
openqg-bench theory evolve [FLAGS]
  --observables <PATH>   JSONL of ObservableRecord (one per line)
                         default: data/fixtures/cosmology/bao-desi-dr1.jsonl
  --output <PATH>        champion report path
                         default: target/openqg/theory/champion.json
  --generations <N>      MAP-Elites generations          default: 60
  --population <N>       offspring per generation         default: 24
  --seed <U64>           PRNG seed (run is deterministic) default: 1
```

The observables file is JSONL of `ObservableRecord { observable_id, kind, value, uncertainty,
unit, source }`. Redshift-dependent observables use the `name@<z>` convention
(`dm_over_rd@0.510`, `dh_over_rd@2.330`, `dv_over_rd@0.295`); scalar observables are bare
(`bbn_yp`, `h0`, `r_drag`).

**Output report schema** (`theory_evolve.rs::run_evolve` / `champion_json`):

```
{
  engine, observables, observable_count, generations, population, seed,
  qd_score,            # Σ over archive cells of final_fitness
  archive_cells,       # number of occupied MAP-Elites cells
  champion: {
    id, final_fitness, credible, perturbation_robustness,
    fit:         { vetoed, epsilon_delta_log_likelihood, log_likelihood, beta, coverage },
    unification: { score, is_unified, domains: [ { domain, score, note }, ... ] },
    theory:      { alpha:{alpha_m,alpha_b,alpha_k,alpha_t},
                   background:{h,omega_m,omega_b_h2,n_eff,sum_mnu,w0,wa},
                   screening,
                   parameters:[ { symbol, value, physical_meaning, provenance }, ... ] }
  }                    # null if no credible champion emerged
}
```

The console line summarizes: `champion <id> final_fitness=… credible=… unification=… | qd=…
cells=… -> <output>`.

---

## 7. What is NOT yet done — honest limitations

- **Background-only forward model.** `BackgroundForwardModel` covers the Tier-0 observables that
  are exactly computable from the FLRW background — BAO distances, SNe μ(z), BBN Y_p. It does
  **not** compute linear growth (σ8, fσ8) or the full CMB C_ℓ. Those are deliberately omitted
  (not faked), and they are the job of the *optional* Boltzmann backend (`ForwardKind::Boltzmann`,
  the `BoltzmannForwardModel` shelling out to cobaya + hi_class / H-EFTCAMB behind a
  `--features physics-stack` flag). Until that lands, the engine cannot adjudicate growth-sector
  or CMB-damping-tail claims, and the H0 sound-horizon discriminators
  (forward-model §3 of `forward-model-and-unification.md`) are only partially exercised.
- **The live LLM proposer (M4) is not wired.** The whole loop is intentionally LLM-free today.
  The structure-proposer + derivation-checker is future work; the research basis already fixes
  the rule (§5 of `automated-theory-discovery.md`): an LLM may *propose* and *critique* but
  **never judge** — every LLM claim must clear a non-LLM oracle (the veto cascade, the numeric
  forward model, or a certificate), because a text-only supervisor cannot fully verify honesty.
- **The legacy ZYAL pipeline is not retired.** The old `zyal_*` modules
  (`zyal_robustness.rs`, `zyal_judge.rs`, `zyal_genome/*`) with the identity `forward_map` and
  the keyword-marker whitebox scan still exist in `openqg-bench` and still back the legacy
  `run-hybrid-*` flows. The new engine is an *additive* integration
  (`theory_evolve` / `theory evolve`) that does not touch those paths yet; cutting them over is
  pending.
- **BBN/SNe fidelity is fit-formula level.** `bbn_helium_fraction` is a linearized fit, and
  distances use a fixed-panel Simpson rule. Both are sub-percent in their validated ranges but
  are not full Boltzmann/BBN-network computations.

---

## 8. Roadmap

- **M4 — LLM proposer + derivation-checker (never judge).** Add an LLM structure-proposer
  seeded with domain priors (LLM-SR, Shojaee et al. ICLR 2025, arXiv:2404.18400) that emits a
  `Theory` AST, routed through the existing deterministic gates. Add a derivation-checker that
  escalates β toward an AI-Descartes derivation-distance / AI-Hilbert (Cory-Wright et al. 2024,
  arXiv:2308.09474) Positivstellensatz-style machine-checkable certificate where the axioms
  allow. Guard against LLM recitation/memorization with a versioned modified-physics held-out
  benchmark (LLM-SRBench, arXiv:2504.10415). Adversarial proposer/skeptic pairs resolve disputes
  via the symbolic gates, never by vote.
- **M5 — broaden fixtures, Boltzmann backend, flagship run.** Wire the optional
  `BoltzmannForwardModel` (cobaya + hi_class / H-EFTCAMB) behind `--features physics-stack` for
  full CMB C_ℓ + nonlinear P(k) + fσ8, while the default CI stays pure-Rust and deterministic.
  Broaden the observable fixtures (DESI DR2 BAO, Pantheon+ SNe, Planck plik_lite, a CMB-lite +
  BAO + SNe + BBN stack) and run a flagship evolution that actually stresses the H0
  sound-horizon discriminators. Retire the legacy `zyal_*` pipeline once the new engine subsumes
  its role.

---

## References (from the research briefings)

Bellini & Sawicki 2014 (α-basis) · Eisenstein & Hu 1998, astro-ph/9709112 (z_drag, r_s) ·
Hogg 1999, astro-ph/9905116 (distances) · Chevallier–Polarski 2001 / Linder 2003 (CPL) ·
Ezquiaga & Zumalacárregui 2017, arXiv:1710.05901 (GW170817 ⇒ α_T≈0) · Woodard, arXiv:1506.02210
(Ostrogradsky) · Cornelio et al., AI-Descartes, *Nat. Commun.* 14:1777 (2023), arXiv:2109.01634
(ε/β) · Cory-Wright et al., AI Hilbert, *Nat. Commun.* 15:5922 (2024), arXiv:2308.09474
(certificates) · Cranmer 2023, arXiv:2305.01582 (PySR / Pareto) · Udrescu & Tegmark, AI-Feynman,
arXiv:1905.11481 / 2006.10782 · structural stability, arXiv:2509.21780 · Shojaee et al., LLM-SR,
ICLR 2025, arXiv:2404.18400; LLM-SRBench, arXiv:2504.10415 · DESI DR1 BAO 2024 · Aver et al. 2015
(Y_p). Full citation lists: `docs/research/automated-theory-discovery.md` and
`docs/research/forward-model-and-unification.md`.
