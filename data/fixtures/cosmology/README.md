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

## Tiering
These are **Tier-0**: exactly computable from the FLRW background alone (distances + sound
horizon + BBN), so the default pure-Rust engine can score them deterministically. Tier-1 (CMB
C_ℓ) and Tier-2 (DES/KiDS 3×2pt for S8) require the optional Boltzmann backend and full
covariances — see `docs/research/forward-model-and-unification.md`.
