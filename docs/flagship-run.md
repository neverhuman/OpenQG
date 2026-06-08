# Flagship run — an evolved, cross-domain-consistent theory

A reproducible flagship run of the rebuilt engine on real combined Tier-0 cosmology data, seeded
with the GR/ΛCDM baseline plus LLM-style proposals. This is the artifact a stern outside physicist
is invited to attack: every claim below is a gate the champion *passed*.

## Reproduce
```
openqg-bench theory evolve \
  --observables data/fixtures/cosmology/tier0-combined.jsonl \
  --proposals   data/fixtures/proposals/example.jsonl \
  --generations 400 --population 48 --seed 1
```
Deterministic in the seed; ~27 s (release). Data: 15 observables = DESI DR1 BAO + Planck-2018 CMB
distance priors (R, ℓ_A) + the Aver-2015 BBN helium anchor.

## Champion `thy-m4bb4`
A **screened modified-gravity** theory (descended from the proposal lineage, then refined by
mutation/recombination against the data):

| sector | value |
|---|---|
| H0 | 69.57 km/s/Mpc (h = 0.6957) |
| Ω_m | 0.2982 |
| w0, wa (CPL) | −1.058, 0 |
| α_M, α_B, α_K, **α_T** | −0.0313, 0, 0.0728, **0** |
| screening | chameleon |
| ω_b h², N_eff, Σmν | 0.02237, 3.046, 0.06 eV |

## Why it is credible (the gates it cleared)
- **Deterministic veto cascade** — passes all 7: dimensionally homogeneous, Lorentz-scalar,
  whitebox provenance, **GW170817** (α_T = 0), ghost-free, gradient-stable, PPN-screened.
- **Real forward model** — fits all 15 observables (coverage = **1.0**) with log-likelihood
  −7.84; ε is a genuine fit to BAO + CMB-prior + BBN data, not a parameter echo.
- **Derivability β = 0.867** — every parameter is fundamental or mechanism-derived (one proposed
  hand-wavy "derived-from-quantum-gravity" parameter was **demoted and killed**).
- **Cross-domain unification = 0.833, unified = true** — clears GW speed, solar-system PPN (via
  chameleon screening), lab fifth-force/EP, BBN abundances, and the GW-friction siren ratio. A
  theory that fit the data but broke any one of these would be gated to zero.
- **Honesty self-check** — the frozen anchor/decoy calibration set passed (`honest = true`): the
  GR baseline and a screened-MG anchor survived; the ghost, gray-box, and BBN-violating decoys
  died.
- **Quality-diversity** — archive of 8 behavioral cells, QD score 0.967.

## Honest limitations
- **Background-only forward model.** The Tier-0 observables (BAO distances, CMB *distance priors*
  R/ℓ_A, BBN Y_p) are computed exactly from the FLRW background. The full CMB C_ℓ power spectrum
  and growth (σ8/S8) need the optional Boltzmann backend (`docs/boltzmann-backend.md`) — until then
  the α-basis deviations are constrained only through their effect on distances + the priors, not
  the full perturbation spectrum, so this champion's modified-gravity sector is *under-constrained*
  relative to a full DES/KiDS + Planck-C_ℓ analysis.
- **Perturbation robustness 0.48.** The champion sits closer to the stability margins than the GR
  baseline (robustness 1.0) — it is credible but not maximally robust; a full run should prefer
  higher-robustness cells.
- This is a demonstration of the *engine*, not a claim of new physics. The value is that the
  pipeline produces only theories that are derived, structurally consistent, and cross-domain
  consistent — and transparently reports where it falls short.

See `docs/zyal-engine-rebuild.md` for the architecture and `docs/research/*` for the method basis.
