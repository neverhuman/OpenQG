# Forward model + cross-domain unification — design briefing

> Research basis for the ZYAL real forward model (Milestone 1), symbolic genome (M2), and the
> cross-domain unification score channel (M3). Companion:
> [automated-theory-discovery.md](automated-theory-discovery.md).

## 1. Real cosmological forward-model + likelihood tooling

**Boltzmann solvers (parameters → observables):**
- **CLASS** (Blas, Lesgourgues, Tram 2011). C + Python wrapper `classy`. Inputs: `omega_b`,
  `omega_cdm`, `H0`/`h`, `A_s`/`ln10^10A_s`, `n_s`, `tau_reio`, + extensions. Outputs: CMB C_ℓ
  (TT/TE/EE/BB/φφ), linear+nonlinear P(k,z), transfer functions, H(z)/distances, σ8. ~seconds–10s
  per eval. **Easiest to extend** (modular `source/`); hi_class is built on it.
- **CAMB** (Lewis, Challinor, Lasenby 2000). Fortran + `camb` Python pkg. Same observable menu;
  includes a `camb.bbn` module mapping (ω_b, N_eff) → Y_p via PArthENoPE/PRIMAT tables. Agrees
  with CLASS at sub-percent — the field's cross-check pair.

**Samplers / orchestration:**
- **Cobaya** (Torrado & Lewis 2021, arXiv:2005.05290). Wraps CAMB/CLASS + likelihoods; MCMC with
  speed-blocking, PolyChord, minimizers. Posterior eval overhead ~0.5 ms (negligible; the Boltzmann
  solve dominates). The natural thing for a Rust loop to shell out to (YAML in → samples out).
- **MontePython** (Brinckmann & Lesgourgues 2018) — CLASS-centric MCMC. **CosmoSIS** — heavier
  DES-ecosystem pipeline.

**Likelihoods (spectra → χ²):**
- **Planck** — `clik`/`plc` 2018 (PR3) / NPIPE (PR4); plus lightweight **`plik_lite`**
  (foreground-marginalized, fast). Dominant constraint, heaviest to install.
- **DES-Y3 / KiDS-1000** 3×2pt weak lensing — needs nonlinear P(k,z) (HMcode/Halofit) +
  IA/photo-z/shear nuisance. Heaviest to wire; **this is where S8 lives**.
- **BAO** — BOSS DR12, eBOSS DR16, **DESI DR1 (2024)/DR2 (2025)**. Cheap: compare D_M/r_d, D_H/r_d,
  D_V/r_d at a few redshifts. Near-instant given H(z) + r_drag.
- **Pantheon+ SNe** (Brout et al. 2022, arXiv:2202.04077) — 1701 light curves, z=0.001–2.26, with
  a 1701×1701 stat+sys covariance. Cheap; only needs background distances. SH0ES Cepheid block
  optional (adds absolute H0).
- **BBN** — analytic (ω_b, N_eff) → (Y_p, D/H). Codes: PArthENoPE, PRIMAT, **PRyMordial**
  (arXiv, PMC11266446), CAMB tables. Essentially free as a prior.

**Minimal realistic stack for genuine χ²:**
```
classy|camb → cobaya → { plik_lite + Commander/SimAll (CMB),
                         DESI DR2 BAO, Pantheon+ SNe, BBN(ω_b,N_eff) }
```
This CMB-lite + BAO + SNe + BBN combo is the cheapest defensible χ². Add DES-Y3/KiDS only when you
need S8 (≈3× cost). For modified gravity, swap the theory code to **hi_class** or **EFTCAMB**.

## 2. Parametrized MG / dark-energy frameworks (the "theory genome")

- **Horndeski (1974)** — most general scalar-tensor with 2nd-order EOMs (no Ostrogradsky ghost):
  four functions G_2(φ,X), G_3, G_4, G_5. Maximal whitebox scalar-tensor space; "beyond
  Horndeski"/DHOST extend it.
- **EFT of Dark Energy** (Gubitosi, Piazza, Vernizzi 2013) — unitary-gauge background functions
  {Ω/M_*², Λ, c} + operator coefficients {M_2⁴, M̄_1³, …}.
- **α-basis** (Bellini & Sawicki 2014) — **the cleanest evolvable genome.** Linear deviations from
  GR collapse to four functions of time + Planck-mass run:
  - **α_M** = d ln M_*²/d ln a — Planck-mass run; modifies lensing and GW friction
    (d_L^GW/d_L^EM ≠ 1).
  - **α_B** — braiding; scalar/metric kinetic mixing → DE clustering, ISW, growth.
  - **α_K** — kineticity; quintessence/k-essence; weakly constrained (degenerate).
  - **α_T** — tensor speed excess, c_GW² = c²(1+α_T). **GW170817 ⇒ α_T ≈ 0** (biggest pruning).
  - plus M_*²(a)/Ω and background H(a) (equivalently w(a)).
  Stability needs no ghost/gradient instability (c_s² > 0, Q_s > 0) — a fast analytic gate.
- **μ–Σ phenomenology** — μ(a,k) modifies the Poisson equation (clustering/growth, RSD/fσ8);
  Σ(a,k) modifies the lensing/Weyl potential (WL/ISW). GR ⇒ μ=Σ=1. Model-agnostic, observable-facing
  — use as a *projection* of the α-genome onto data, not the genome itself. (Pogosian et al. 2021
  arXiv:2009.01189; DESI FS MG arXiv:2411.12026.)

**Evaluable codes:** **hi_class** (Zumalacárregui et al. 2017, arXiv:1605.06102; hiclass-code.net) —
CLASS fork natively in the α-basis; **genome ≈ input**, best fit for an evolutionary loop.
**EFTCAMB / H-EFTCAMB** (2026, arXiv:2603.01662) — accepts an arbitrary covariant Horndeski
Lagrangian; the most direct "symbolic Lagrangian → cosmology" pipe.

## 3. H0/S8 tensions (2024–2026) — the killer observable per resolution

H0 discrepancy (Planck ~67.4 vs SH0ES ~73) now >5–6σ and robust (CosmoVerse, Di Valentino et al.
2025, arXiv:2504.01669). **S8 has softened** — DES-Y3 + KiDS-Legacy + CMB-lensing now largely
consistent with Planck (multiprobe S8=0.819±0.016, arXiv:2510.06114); treat as a milder ~2σ,
survey-dependent effect. **DESI DR2 (2025)** reports up to 4.2σ for evolving w0waCDM (with DES-Y5
SNe), partly driven by a z≈0.51 BAO outlier (tempered by arXiv:2511.10631).

| Resolution | Mechanism | What kills it |
|---|---|---|
| **Early Dark Energy** | scalar near z~3500 shrinks r_s → raises H0 | raises ω_cdm → **worsens S8**; Lyman-α inconsistency; ACT DR6+DESI+Planck give f_EDE < 0.07–0.09 (95%). Survives as ≤2–3σ hint. |
| **Modified recombination** | shift r_s | CMB damping tail + EE peak phases; generic shifts spoil polarization. |
| **Varying m_e** | shifts recombination z, degenerate with H0 (+curvature) | **BBN**: m_e at BBN within ~1% of today (Schöneberg et al. 2022 arXiv:2206.13209). |
| **Interacting DE–DM** | energy exchange Q | high-ℓ CMB disfavors; coupling that helps H0 shifts growth → RSD/WL tension. |
| **Decaying DM** | DM → dark radiation | CMB polarization (pre-recomb) + CMB lensing (post-recomb) bound it (arXiv:2203.04818). |
| **Late-time modified gravity** | change G_eff/expansion at low z | **GW170817** kills α_T and G4(X)/G5; surviving MG barely moves r_s ⇒ **cannot fix H0**. |

**Key fitness principle:** H0 is fundamentally a **sound-horizon (early-time) problem.** Late-time-only
fixes are structurally incapable of resolving it without breaking BAO/SNe distances. A genome
claiming to fix H0 *must* alter pre-recombination physics, and then **BBN (Y_p, D/H) + CMB damping
tail + EE phases** are the discriminators. Encode this into the fitness; track r_drag and EE-phase /
damping-tail residuals as dedicated non-degeneracy sub-scores.

## 4. Cross-domain unification consistency constraints (the "unified-theory bar")

**Quick analytic gates (hard filters, ~µs):**
1. **GW speed (GW170817 + GRB170817A):** \|c_GW/c − 1\| ≲ 5×10⁻¹⁶ (Ezquiaga & Zumalacárregui 2017,
   arXiv:1710.05901). ⇒ **α_T = 0**: G_4 = G_4(φ) only, **G_5 = 0**. One-line check on the
   Lagrangian's X-dependence.
2. **PPN (solar system):** Cassini ⇒ \|γ−1\| ≤ 2.3×10⁻⁵; LLR ⇒ \|β−1\| ≲ 10⁻⁴. A bare conformal
   coupling violates this by orders of magnitude ⇒ genome must include a **screening mechanism**
   (chameleon / Vainshtein / symmetron / k-mouflage). Check: does the model recover GR at
   solar-system density?
3. **Lab fifth-force / EP:** Eöt-Wash (\|α\|≲1 at λ~50µm), MICROSCOPE EP η≲10⁻¹⁵. Same screening
   requirement; quick Yukawa (α,λ) range check.
4. **BBN abundances:** (ω_b, N_eff, varying constants) → Y_p≈0.247, D/H≈2.5×10⁻⁵. Closed-form via
   PRyMordial/CAMB tables; flags early-time energy-injection genomes.
5. **GW friction / standard-siren:** α_M ≠ 0 ⇒ d_L^GW/d_L^EM = exp(½∫α_M d ln a) ≠ 1; cheap analytic
   ratio against LVK sirens.

**Heavier (need a solver):** ghost/gradient stability (Q_s>0, c_s²>0 across the trajectory) —
gate via EFTCAMB/hi_class; SM precision / particle bounds if the dark sector couples to SM;
full r_s / damping-tail self-consistency (Boltzmann run).

Items 1–5 are **pre-filters** rejecting ≳90% of random genomes before any expensive Boltzmann call.
Keep this channel **orthogonal to χ²_cosmo** — a unified theory must pass cosmology *and* this
channel, and cannot trade one off against the other.

## 5. Reproducibility / determinism wrapping Python/C from Rust

- **Pin everything**: CLASS git SHA + precision files; `camb`/`classy`/`cobaya` versions; Planck
  `clik` data version (PR3 vs PR4); NumPy/SciPy; gfortran/gcc version. Record in a manifest hashed
  into each result.
- **Containerize** (Docker/Apptainer). Avoid `-ffast-math` (breaks IEEE determinism).
- **Threads:** OpenMP reductions are not bit-reproducible across thread counts ⇒ pin
  `OMP_NUM_THREADS=1` (+ `MKL`/`OPENBLAS_NUM_THREADS`) for reproducible χ², or accept ~1e-10 jitter
  with tolerance-based equality. Pin one BLAS backend (OpenBLAS vs MKL differ in roundoff).
- **Content-hash cache:** hash (canonicalized parameter dict + code manifest) → cache spectra/χ².
  Canonicalize the *symbolic* genome before hashing for cache hits across equivalent forms — the
  single biggest throughput win.
- **Boundary:** subprocess + hard timeout (fault isolation: CLASS can segfault/hang on pathological
  MG params). **Treat crash/timeout/instability as ∞ χ² (a lethal genome), not an error.** PyO3 only
  for hot loops once stable.
- **Emulators:** defer until the genome space converges, then **OLÉ** online-learning emulation
  (arXiv:2503.13183) — tracks the evolving theory distribution; pre-trained CosmoPower won't
  generalize to arbitrary evolved MG genomes.

## 6. Recommendations for the forward-model architecture

1. **hi_class (α-basis) as the primary external theory code; H-EFTCAMB as the covariant-Lagrangian
   path.** {α_M, α_B, α_K, α_T} + w(a) + M_*² is the most direct evolvable + Boltzmann-evaluable
   genome (hi_class input ≈ our genome).
2. **GR-deviation gates as pre-filters before any Boltzmann call** (α_T≠0, missing screening,
   ghost/gradient instability, BBN-violating injection). Protects against CLASS crashes.
3. **Tier the likelihood stack by cost:** Tier-0 (free) BAO + Pantheon+ + BBN; Tier-1 (seconds)
   CMB `plik_lite`/Commander/SimAll; Tier-2 (opt-in, expensive) DES/KiDS 3×2pt. Score cheaply first.
4. **Encode the H0 sound-horizon constraint structurally** into fitness (penalize late-time-only
   "fixes" that break BAO/SNe; track r_drag, EE-phase, damping-tail residuals).
5. **Drive the external stack through cobaya via subprocess + timeout + content-hash cache;**
   crash/timeout/instability = ∞ χ². Canonicalize the symbolic genome before hashing.
6. **Pin + containerize the whole physics stack;** embed the manifest hash in every score record.
7. **Cross-domain "unification" score channel orthogonal to χ²_cosmo** (GW speed, PPN γ/β,
   fifth-force, EP η, BBN, GW-friction siren ratio). Pass cosmology *and* this channel.
8. **Defer emulators** until the genome space converges, then OLÉ-style online emulation.

## In-repo (M1) vs external (optional) split

The **pure-Rust `InHouseForwardModel`** integrates the background Friedmann H(a) + linear-growth
(α-basis perturbation) ODEs derived from the symbolic theory — covering the **Tier-0** observables
(BAO distances, SNe μ(z), BBN Y_p) exactly and deterministically, with the derivation chain itself
as provenance. The **optional `BoltzmannForwardModel`** (cargo `--features physics-stack`) shells
out to cobaya + hi_class / H-EFTCAMB for full CMB C_ℓ + nonlinear P(k) + fσ8. Default CI stays
pure-Rust and deterministic; the external backend is opt-in.

## Key citations

CLASS class-code.net; CAMB camb.readthedocs.io · Cobaya, Torrado & Lewis 2021 arXiv:2005.05290 ·
Bellini & Sawicki 2014; Gubitosi, Piazza, Vernizzi 2013 · hi_class arXiv:1605.06102,
hiclass-code.net; H-EFTCAMB arXiv:2603.01662 · μ–Σ Pogosian et al. 2021 arXiv:2009.01189; DESI FS MG
arXiv:2411.12026 · CosmoVerse, Di Valentino et al. 2025 arXiv:2504.01669 · DESI DR2 + Bayesian
re-eval arXiv:2511.10631 · Pantheon+ Brout et al. 2022 arXiv:2202.04077 · EDE constraints
arXiv:2404.16805, 2505.08051 · varying m_e + BBN arXiv:2206.13209 · decaying DM arXiv:2203.04818 ·
GW170817 Ezquiaga & Zumalacárregui 2017 arXiv:1710.05901 · PPN/screening arXiv:2111.13270 ·
PRyMordial PMC11266446 · emulators CosmoPower arXiv:2106.03846; OLÉ arXiv:2503.13183 · multiprobe
S8 arXiv:2510.06114.
