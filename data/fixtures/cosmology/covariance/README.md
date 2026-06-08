# Cosmology covariance fixtures (v3.0.0 M2)

Real, published covariance blocks for the correlated cosmology observables, loaded by the
`CovarianceRegistry` in `openqg_core::scoring::covariance`. Every block carries a literature
citation, the ordered observable id set the matrix is over, a content hash, and a
positive-definite + condition-number health check (a non-PD block is rejected, fail-closed).

These are the *off-diagonal* correlations the committed Tier-0 fixtures (diagonal only) omit; the
matrices below are the published covariances, not the diagonal we already ship.

## `planck18-distance-priors.json` — Planck 2018 compressed CMB distance priors
The 3x3 covariance of the compressed CMB distance priors `(R, l_A, omega_b h^2)` for the base-LCDM
chain, from **Chen, Huang & Wang 2019**, "Distance priors from Planck final release", JCAP 02
(2019) 028 (arXiv:1808.05724), Table I (Planck 2018 TT,TE,EE+lowE) and the Appendix A
inverse-covariance file `Distance_invcov.txt`.

The matrix is obtained by **inverting the exact published inverse-covariance** (the machine-readable
matrix the authors ship), which is the most faithful encoding. Its implied 1-sigma errors
(0.00462, 0.0889, 0.000149) and correlations (rho_R,lA = +0.463, rho_R,wb = -0.660,
rho_lA,wb = -0.327) reproduce Table I's quoted values (0.0046, 0.090, 0.00015; +0.46, -0.66, -0.33).

Mean values (page 8 / `distance.ini`): R = 1.750235, l_A = 301.4707, omega_b h^2 = 0.02235976.

Observable ids: `cmb_R`, `cmb_lA`, `cmb_omega_b_h2` (the background forward model reports
`cmb_omega_b_h2` as the baryon density `omega_b h^2`).

## `desi-dr1-bao.json` — DESI DR1 BAO intra-tracer correlations
Per-tracer 2x2 covariance blocks for `D_M/r_d` and `D_H/r_d`, from **DESI Collaboration 2024**,
"DESI 2024 VI: Cosmological Constraints from the Measurements of Baryon Acoustic Oscillations",
JCAP 02 (2025) 021 (arXiv:2404.03002), **Table 1**. Each block is
`C = [[sigma_DM^2, r*sigma_DM*sigma_DH], [r*sigma_DM*sigma_DH, sigma_DH^2]]` with the published
correlation coefficient `r`:

| tracer    | z_eff | D_M/r_d        | D_H/r_d        | r       |
|-----------|-------|----------------|----------------|---------|
| LRG1      | 0.510 | 13.62 +/- 0.25 | 20.98 +/- 0.61 | -0.445  |
| LRG2      | 0.706 | 16.85 +/- 0.32 | 20.08 +/- 0.60 | -0.420  |
| LRG3+ELG1 | 0.930 | 21.71 +/- 0.28 | 17.88 +/- 0.35 | -0.389  |
| ELG2      | 1.317 | 27.79 +/- 0.69 | 13.82 +/- 0.42 | -0.444  |
| Lya QSO   | 2.330 | 39.71 +/- 0.94 | 8.52  +/- 0.17 | -0.477  |

The BGS (z=0.295, D_V/r_d = 7.93 +/- 0.15) and QSO (z=1.491, D_V/r_d = 26.07 +/- 0.67) tracers are
isotropic `D_V/r_d`-only measurements with no intra-tracer covariance, so they carry no block.
Inter-tracer correlations are negligible (the paper's redshift bins are disjoint), so each tracer is
a self-contained block.
