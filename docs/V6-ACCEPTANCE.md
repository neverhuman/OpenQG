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
