# Production run — 1000 generations, fully logged + adversarial

> **⚠️ SUPERSEDED (v3.0.0).** The headline number below — "ε = +36.7, a champion that beats ΛCDM" —
> was an **artifact** of a fitted candidate scored against a *fixed* Planck baseline under a
> *diagonal* likelihood with no complexity penalty. Re-fitting ΛCDM fairly (covariance-aware,
> profile-fit, with a parameter penalty) *inverts* the result: on geometry alone ΛCDM wins and
> evolving dark energy is disfavored. The defensible number is the `theory league` ΔAIC / Δln-evidence
> — see `docs/theory-league.md`. This document is retained as a historical record of the engine run;
> **do not cite the +36.7 figure.** (Two further v3.0.0 corrections: the adversary was telemetry-only
> until it was wired into selection; and a `set_param` bug meant the growth-sector σ8/μ0 were not
> actually being fit — both fixed.)

A reproducible **1000-generation** production run of the new symbolic engine, with the co-evolving
adversary wired in and every generation logged + monitored. This is the artifact a stern outside
physicist is invited to attack.

## Reproduce / monitor
```
tools/theory-evolve-run.sh 1000 prod-1000 1          # combined Tier-0 + proposals, pop 48, seed 1
tools/theory-evolve-monitor.sh prod-1000             # live: gen/target, qd, frontier, anchor, honesty, champion
```
Deterministic in the seed. Run directory `target/openqg/theory-evolve/runs/prod-1000/`:
`run-config.json`, `metrics-timeseries.jsonl` (1000 lines, one per generation, appended live),
`map-elites-archive.json`, `quality-gate.json`, `champion.json`, `run-summary.json`.
Data: 15 observables — DESI DR1 BAO + Planck-2018 CMB distance priors (R, ℓ_A) + the Aver-2015 BBN
helium anchor.

## The headline: honesty held for all 1000 generations
The co-evolving adversary escalated a frontier the champion had to keep beating, with **honesty
rollback** driven by the GR baseline:

| metric | value |
|---|---|
| generations with a **dishonest** calibration | **0 / 1000** |
| frontier margin (escalated → self-limited) | 0.02 → **0.28**, oscillating at the floor |
| min anchor health (baseline's pressured fitness) | 0.08 (rollback kept it > 0 — never killed) |
| quality gate | **passed** |
| QD score / archive cells | 8.29 / 17 |
| one proposed "derived-from-quantum-gravity" parameter | **demoted and killed** |

The frontier rose to 0.28 by ~gen 50 and oscillated there — the rollback prevented it from ever
driving the GR baseline below the survival floor, so the reference was protected while sustained
pressure was applied. *The engine maintained "robustness under judge" for 1000 generations without
once breaking its own honesty calibration.*

## Champion `thy-xcde1` (the "beats ΛCDM" claim here is SUPERSEDED — see the banner)
A Vainshtein-screened, GW170817-safe, ghost-free modified-gravity / mild-phantom-DE theory:

| sector | value |
|---|---|
| H0 | 68.96 km/s/Mpc (h = 0.6896) |
| Ω_m | 0.3026 |
| w0, wa (CPL) | −1.032, 0 |
| α_M, α_B, α_K, **α_T** | 0, 0, −0.1336, **0** |
| screening | Vainshtein |
| ω_b h², N_eff, Σmν | 0.02237, 3.046, 0.06 eV |

**ε = +36.7 — SUPERSEDED artifact, do not cite.** This figure compared a *fitted* candidate to a
*fixed* Planck ΛCDM baseline under a *diagonal* likelihood with no complexity penalty. The fair
`theory league` (re-fit baseline, covariance-aware, parameter-penalized) shows the opposite on
geometry data: ΛCDM wins and the raw χ² gain of evolving DE is only ~a few, disfavored once its two
extra parameters are counted (`docs/theory-league.md`).

Why it is credible — the gates it cleared:
- **7-stage veto cascade**: dimensionally homogeneous, Lorentz-scalar, whitebox provenance,
  **GW170817** (α_T = 0), ghost-free, gradient-stable, **PPN-screened** (Vainshtein).
- **Real forward model**: coverage **1.0** (predicts all 15 observables). (The ε figure is the
  superseded artifact above; use the `theory league` ΔAIC instead.)
- **Derivability β = 0.867**: every parameter fundamental or mechanism-derived.
- **Cross-domain unification = 0.833 (unified)**: clears GW speed, solar-system PPN (screened),
  lab fifth-force/EP, BBN abundances, the GW-friction siren ratio. A theory that fit the data but
  broke any one of these would be gated to zero.
- **Perturbation robustness 0.60**: stays credible under most small parameter perturbations.
- **Honesty self-check passed**: the frozen anchor/decoy set calibrated (baseline + screened-MG
  survive; ghost / gray-box / BBN-violating decoys die).

## Progression
| gen | QD | cells | frontier | anchor health | champion fitness |
|---|---|---|---|---|---|
| 1 | 2.40 | 8 | 0.02 | 0.34 | 0.679 |
| 50 | 8.19 | 17 | 0.28 | 0.08 | 0.713 |
| 500 | 8.29 | 17 | 0.28 | 0.08 | 0.7135 |
| 1000 | 8.29 | 17 | 0.28 | 0.08 | 0.7135 |

The engine found its optimum (champion fitness ~0.713, 17 behavioral cells) by ~gen 50; the
remaining generations sustained the adversarial frontier and confirmed the archive/honesty held.

## Honest limitations
- **Background-only forward model.** The 15 observables are exactly computable from the FLRW
  background (BAO distances, CMB *distance priors* R/ℓ_A, BBN Y_p). The full CMB C_ℓ spectrum and
  growth (σ8/S8) need the optional Boltzmann backend (`docs/boltzmann-backend.md`), so the
  champion's modified-gravity (α_K) sector is *under-constrained* relative to a full DES/KiDS +
  Planck-C_ℓ analysis. (The superseded ε artifact was driven mainly by the background expansion /
  distance sector; see the banner and `docs/theory-league.md` for the fair number.)
- The champion's α_K is constrained only weakly (kineticity barely affects background distances);
  a full perturbation analysis would pin it down or rule it out.
- This is a demonstration that the *engine* evolves only derived, structurally- and
  cross-domain-consistent theories, logs every generation, and never breaks its own honesty
  calibration — not a claim of new physics.

See `docs/zyal-engine-rebuild.md` (architecture) and `docs/flagship-run.md` (the shorter 400-gen
flagship).
