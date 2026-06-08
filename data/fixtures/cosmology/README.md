# Cosmology data fixtures (Tier-0: background-derived observables)

Real, externally-sourced observables that require a genuine theory→observable derivation to
predict — unlike the parameter-style entries in `data/fixtures/tension/` (which a model can
"predict" by echoing a gene). These exercise the in-repo background forward model
(`openqg_core::cosmology::BackgroundForwardModel`).

`observable_id` uses the `name@<z>` convention for redshift-dependent observables, matching the
forward model's `compute` dispatch (`dm_over_rd@<z>`, `dh_over_rd@<z>`, `dv_over_rd@<z>`,
`mu@<z>`, plus scalar `bbn_yp`).

## `bao-desi-dr1.jsonl` — DESI 2024 DR1 BAO
Baryon Acoustic Oscillation distance ratios (D_M/r_d, D_H/r_d, D_V/r_d) at the DESI effective
redshifts. Values from DESI Collaboration 2024, "DESI 2024 VI: Cosmological Constraints from the
Measurements of Baryon Acoustic Oscillations" (arXiv:2404.03002). Diagonal (per-point)
uncertainties only; the full BAO covariance is a future addition for the optional Boltzmann
likelihood backend.

`bbn_yp` — primordial helium mass fraction Y_p from Aver et al. 2015 (JCAP 07 011,
arXiv:1503.08146), the standard BBN abundance anchor.

## `sh0es-h0.jsonl` — local distance-ladder H0 (Tier-2)
The Cepheid-calibrated local Hubble constant H0 = 73.04 ± 1.04 km/s/Mpc (Riess et al. 2022, ApJL
934 L7, arXiv:2112.04876). Added as the scalar `h0` observable; in the league it stresses ΛCDM and
makes the H0 tension quantitative.

## `growth-rsd.jsonl` — redshift-space-distortion fσ8 (Tier-1)
Linear growth-rate × amplitude fσ8(z) from RSD: 6dFGS (Beutler et al. 2012), the BOSS DR12
consensus z = 0.38/0.51/0.61 (Alam et al. 2017, MNRAS 470 2617), and eBOSS DR16 QSO z = 1.48 (Hou
et al. 2021). Uses the `fsigma8@<z>` id convention. Per-point (diagonal) uncertainties; the BOSS
DR12 intra-sample covariance is a labeled follow-up via the `--covariance` flag.

## `wl-s8.jsonl` — weak-lensing S8 (Tier-1)
The weak-lensing clustering amplitude S8 = σ8 √(Ω_m/0.3) = 0.776 ± 0.017 from the DES Year-3 3×2pt
analysis (Abbott et al. 2022, PRD 105 023520). Scalar `s8` observable.

## Tiering
**Tier-0** (BAO, SNe μ(z), sound horizon, BBN) is exactly computable from the FLRW background.
**Tier-1 linear growth** (fσ8, S8) is now also in-repo: the background+growth forward model
integrates the linear growth equation with the GW170817-safe `μ(a)` modified-gravity handle
(`openqg_core::cosmology::growth`). The full CMB C_ℓ and nonlinear P(k) still require the optional
Boltzmann backend — see `docs/boltzmann-backend.md` and `docs/theory-league.md`.
