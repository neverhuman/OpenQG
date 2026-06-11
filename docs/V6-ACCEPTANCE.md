# V6 acceptance record (2026-06-10)

The reviews' go/no-go (openqg-v5-review-01): re-score every V5 champion under V6 —
each must be DISQUALIFIED or honestly re-cost. Reproduce with:
`zyal genome rescore --theory <champion.json> --observables data/fixtures/cosmology/tier1-multisector.jsonl [--covariance ...]`

| Candidate | V5 score | V6 verdict | Why |
|---|---|---|---|
| v5-chunk-1 `thy-m22ea` | 54.999 | **0.0 DQ** | ScreeningRecoveryUnquantified (bare "vainshtein" label on α-modified gravity) |
| v5-chunk-2 `thy-m2913` | 54.999 | **0.0 DQ** | same class |
| v5-chunk-3 `thy-m0b88` | 54.999 | **0.0 DQ** | same class |
| v5-chunk-4 `thy-ma639` | 54.999 | **0.0 DQ** | same class |
| v5-chunk-5 `thy-mb43f` | 54.999 | **0.0 DQ** | same class |
| v5-chunk-6 `thy-x9966` | 54.999 | **0.0 DQ** | same class |
| v5-mini `openqg-v4-planck-mg-suppression` | 58.0 | **11.0, survives** | honest physics, re-cost: no-witness novelty 0, background-drift dof, V6 gates |

Gates at acceptance: core 252 + bench 73 tests green; trust gate PASSED (decoy-FPR-0,
humans-survive); replay 0 mismatches (covariance mode); ops/replay.sh REPLAY OK.

Companion finding (permanent regression test `covariance_mode_is_recorded_...`): the full
published Planck distance-prior 3×3 + DESI blocks REOPEN the h/Ω_m degeneracy valley relative
to the diagonal treatment (+54.9 → +68.8 nats) — the diagonal likelihood OVER-stated the
compressed CMB's constraining power. The valley is genuinely open until richer data is admitted;
V6 accounts honestly (the shift pays parsimony dof; the likelihood mode is on every scorecard).

## V6.1 addendum (2026-06-11): the audit cascade continues

A 21-agent adversarial workflow audited V6 + the live campaign mid-flight; 14 confirmed
findings were fixed the same day (commits eb473a7 + c3ea9d0). The headline discoveries:

1. **The engine's own cmb_lA carried a +0.755 (8.4σ) fitting-formula bias** vs the published
   Planck distance priors — the campaign optimizer harvested ~35 nats of OUR model error by
   drifting h. Fixed by anchor calibration (P0.11) with a regression guard.
2. **The degeneracy-valley saga resolves** (permanent regression test): V5 diagonal +54.9 →
   V6 true-covariance +68.8 (diagonal had over-stated the CMB) → V6.1 calibrated+Occam:
   diag −20.5 / cov −31.9. **The valley was never open — it was model bias + uncosted drift.**
3. Novelty was re-circularized by the re-clothe witness refresh + fit-set witnesses: now
   mechanism-attributable distinctness (mechanism-off twin), fit-set cap 0.25, engine-refreshed
   tier 0.5, measurement-σ floor.
4. Grafted unification scaffolds (H0=67.4 over fitted h=0.701) earned 15/15: now a
   shadow-consistency kill; drifted descendants forfeit donor unification claims.
5. Claim/physics incoherence (rigor certified at μ0=−0.05 while the fit ran μ0=−0.1): now a kill.

| V6 campaign champion | V6 score | V6.1 verdict |
|---|---|---|
| chunks 1–5 (`-rc` lineage, 60.0/60.0/60.0/**77.0**/60.0) | — | **all 0.0 DISQUALIFIED** (StructurallyUngenerated: dials without generating terms) |

The pattern is the point: **every rubric generation's champions are killed by the next
generation's audit** — V4's 87.5 by V4.1, V5's 55-class by V6, V6's 77 by V6.1. The engine's
value is that it turns its own exploits into permanent regression tests within hours.
