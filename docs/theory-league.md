# The theory league — fair model selection, and the honest number

> The credibility-sprint deliverable (`docs/zyal-next-level-design.md`, Part 5 items 1–4). Where
> `theory evolve` is a *search heuristic*, `theory league` is the **rigorous adjudicator**: it
> profile-fits every model class to the same data — the baseline ΛCDM re-fit, not held at a fixed
> Planck point — under a covariance-aware likelihood, and ranks them by ΔAIC / Δln-evidence with a
> complexity penalty. It exists to replace the indefensible "+36.7 log-likelihood, beats ΛCDM"
> comparison (`docs/production-run-1000.md`) with a number a referee accepts.

## Why the old number was wrong

The flagship champion's `ε = +36.7` was a *fitted candidate* (h, Ω_m, w0 tuned to the data) scored
against a *fixed* Planck ΛCDM baseline, summed over a **diagonal** likelihood. Three errors
(`zyal-next-level-design.md` §1.4): the baseline was never re-fit to the same data; correlated
observables (DESI BAO within a tracer, Planck `R/ℓ_A/ω_b`) were treated independent; and there was
no penalty for the extra CPL parameters. Fix all three and the picture inverts.

## What the league computes

1. Each model is a `ModelClass` (`openqg-core/src/theory/league.rs`): a fixed structural form plus a
   list of *free* parameters. ΛCDM frees `{H0, Ω_m}` (and `σ8` when growth data is present); wCDM
   adds `w0`; w0waCDM adds `w0, wa`; screened-MG adds the modified-growth amplitude `μ0`.
2. Every model — **including the baseline** — is profile-fit to the maximum likelihood by
   deterministic Nelder–Mead against the covariance-aware likelihood (`scoring/covariance.rs`).
3. Models are compared at their best fit by **AIC** (`2k − 2 ln L`), **BIC** (`k ln n − 2 ln L`),
   and the Schwarz/Laplace log-evidence `ln Z ≈ −½ BIC`. ΔAIC / Δln Z vs ΛCDM is the headline.

The fit is deterministic and the forward model is pure, so a league reproduces bit-for-bit.

## The honest results (DESI DR1 BAO + Planck distance priors + BBN, plus the named tiers)

| data | favored | ΔAIC (vs ΛCDM) | Δln Z | reading |
|---|---|---|---|---|
| **Geometry only** (BAO+CMB-prior+BBN, n=15) | **ΛCDM** | w0waCDM **+0.24** | −0.83 | Re-fitting ΛCDM shrinks CPL's gain from "+36.7 log-L" to **raw Δχ²≈3.8**; once its 2 extra parameters are penalized, evolving DE is *disfavored*. **The +36.7 was an artifact.** |
| **+ growth** (fσ8 RSD + DES-Y3 S8, n=21) | w0waCDM (marginal) | −1.2 | −0.44 | Adding structure growth gives at most a weak, evidence-inconclusive hint for evolving DE. Modified-growth `μ0` is degenerate with σ8 here and **not** independently favored (ΔAIC +2). |
| **+ local H0** (SH0ES ladder, full stack n=22) | **w0waCDM** | **−16.2** | **+7.0** | The local distance ladder stresses ΛCDM hard (χ² 27→44 — the H0 tension, quantified); a dark-energy extension is then strongly favored. |

The engine now responds correctly to *which* data it is given: no false evolving-DE signal on
geometry alone, a real and large preference once the Cepheid-calibrated H0 is included, and an
honest "can't tell yet" on modified gravity until the growth data carries scale information.

Reproduce:
```bash
# geometry only — debunks +36.7
cargo run -p openqg-bench -- theory league \
  --observables data/fixtures/cosmology/tier0-combined.jsonl

# full stack — the H0 tension drives the preference
cargo run -p openqg-bench -- theory league \
  --observables data/fixtures/cosmology/tier0-combined.jsonl \
  --observables data/fixtures/cosmology/growth-rsd.jsonl \
  --observables data/fixtures/cosmology/wl-s8.jsonl \
  --observables data/fixtures/cosmology/sh0es-h0.jsonl
```
Each run writes a JSON artifact (`--output`, default `target/openqg/theory/league.json`) with every
model's best-fit parameters, χ², AIC/BIC and the Δ's.

## The leading theories, and the honesty of "scoreable"

"Scoreable" is itself a filter that does the field justice: a theory earns a score only if it makes
a *derived, falsifiable* prediction on data we hold. The five leading contenders
(`zyal-next-level-design.md` §3):

| theory | status | what it needs |
|---|---|---|
| **ΛCDM** | ✅ scored (the re-fit null) | — |
| **w0waCDM** (CPL evolving DE) | ✅ scored | — |
| **Screened scalar-tensor MG** (Horndeski/α-basis, GW170817-safe) | ✅ scored at leading order (`μ0`) | full α-basis `G_eff(a,k)` needs the Boltzmann backend to break the σ8 degeneracy |
| **Early Dark Energy** | ⏳ not faked | a scalar-field background sector (so `f_ede` is *computed*, not a fitted knob the whitebox veto would kill) + Tier-4 CMB |
| **Coupled / interacting DE** | ⏳ not faked | a dark-sector coupling sector (modified Ω_m dilution + growth, `β` derived) |

EDE and coupled-DE are deliberately **not** included as fitted knobs: doing so would be exactly the
gray-box parameter-fitting the engine is built to reject. They join the league when their derived
sectors land — that is the next build, not a number we are willing to fake today.

## Honest limitations

- **Growth is leading-order.** Linear `fσ8`/`S8` are integrated exactly from the background (DE
  enters through `E(a)`); modified gravity enters through the late-time `μ(a)=1+μ0 ρ_DE(a)/ρ_DE0`
  parametrization (Planck-2018 MG), which is GW170817-safe but cannot capture the *scale*
  dependence that breaks the `μ0`–σ8 degeneracy. The full CMB `C_ℓ` + nonlinear `P(k)` remain the
  Boltzmann backend's job (`docs/boltzmann-backend.md`).
- **Covariances are wired but the committed runs are diagonal.** `scoring/covariance.rs` scores a
  full block covariance (`−½ rᵀ C⁻¹ r`, marginalizing missing members); the data-policy keeps real
  covariance matrices out of git, so the committed fixtures are diagonal and the off-diagonal DESI
  BAO / Planck-prior correlations are a labeled follow-up via `--covariance`.
- **BIC/Schwarz ≈ evidence.** Δln Z is the Laplace/Schwarz approximation `−½ ΔBIC`, not a
  nested-sampling evidence; it is reported as an approximation, and AIC is given alongside.
