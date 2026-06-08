# Tension fixture (Phase 0)

A small, **informative** observable suite where the `sm-gr-lcdm-mnu` baseline is *not* a
perfect fit — so `delta_log_likelihood` has genuine headroom and the evolutionary frontier
can mean something. This deliberately contrasts with `data/fixtures/smoke/`, where
`predictions.jsonl` is byte-identical to `observables.jsonl` (baseline LL ≡ 0, no headroom).

This fixture is **not** a forward model. It is a frozen set of measured values plus the
baseline ΛCDM predictions, used to (a) compute a non-zero `baseline_log_likelihood` and
(b) act as the cheap, deterministic **physics honesty anchor / veto** in the
adversarial-robustness engine. A real solver (`openqg-theory predict`) remains a deferred
milestone; until it exists, this fixture is a *bound*, not a discovery currency.

## Observables (`observables.jsonl`)

| id | value | ± | unit | source | role |
| --- | ---: | ---: | --- | --- | --- |
| `h0` | 67.4 | 0.5 | km s⁻¹ Mpc⁻¹ | Planck 2018 TT,TE,EE+lowE+lensing | CMB anchor |
| `omega_m` | 0.315 | 0.007 | — | Planck 2018 | anchor |
| `sum_mnu` | 0.06 | 0.02 | eV | minimal normal ordering | anchor |
| `h0_local` | 73.04 | 1.04 | km s⁻¹ Mpc⁻¹ | SH0ES (Riess et al. 2022) | **Hubble tension** |
| `s8` | 0.776 | 0.017 | — | weak lensing (KiDS/DES) | **S8 tension** |
| `n_eff` | 2.99 | 0.17 | — | Planck 2018 | consistency |
| `omega_b_h2` | 0.02237 | 0.00015 | — | Planck 2018 / BBN | consistency |

`observable_count (7) > free_param_count (3)` so BIC/MDL parsimony always bites.

## Baseline predictions (`baseline-predictions.jsonl`)

ΛCDM has a single `H0`, so it predicts `h0_local = 67.4` (missing the local 73.04) and the
Planck-ΛCDM `S8 ≈ 0.83` (above the lensing 0.776). Those two residuals give the baseline a
log-likelihood ≈ −20 (not 0), which is the headroom a robust candidate must beat.

Values are literature central values for an interpretable, auditable baseline; they are not
re-fit here. Update with provenance if the anchor set is revised between runs.
