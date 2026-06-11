# V6 campaign — final report (2026-06-11)

**Config:** 6 chunks × 300 gens (pinned V6 binary), pop 12, multisector data + Planck 3×3 +
DESI covariance blocks, router proposer (slots every 20, best-of-4 on mechanism lanes
planck_mu0/dark_scattering/free/null_diagnostic, band rotation), reclothe slot every 25 gens.

## The machine (vs V5)
| | V5 campaign | V6 campaign |
|---|---|---|
| Scored proposals | 37 (~6/chunk) | **168 (28/chunk)** |
| Live calls | 276 (77% empty-output) | 754 (**0 empty-output**; 65 ok / 649 killed / 28 parse / 12 transport) |
| Lane yield (ok) | n/a | null_diagnostic 25 · planck_mu0 23 · free 11 · dark_scattering 6 |
| Model yield | 1 model | gpt-oss-120b 37 · gpt-oss-20b 19 · nemotron-3-super 9 (winner_model_id ledgered) |
| Replay | 0 mismatches | 0 mismatches |

## The champions — and the V6.1 verdict
| Chunk | Champion (V6 score) | V6.1 rescore |
|---|---|---|
| 1 | thy-m86aa-rc-rc (60.0) | 0.0 DQ (StructurallyUngenerated) |
| 2 | thy-x7220-rc-rc-rc (60.0) | 0.0 DQ |
| 3 | thy-meea4-rc-rc (60.0) | 0.0 DQ |
| 4 | **thy-m56e9-rc-rc-rc (77.0)** | 0.0 DQ — its +17 was a fit-set cmb_lA witness riding a +0.755 engine lA bias |
| 5 | thy-mbed0-rc-rc-rc-rc (60.0) | 0.0 DQ |
| 6 | **ndgp-proposed-fixed-rc (60.0)** | **25.0, SURVIVES** — the campaign's one honest candidate, re-cost |

Every champion is a re-clothed (`-rc`) lineage: the structure×fit operator dominated the
search, exactly as designed — and exactly as audited (its witness-refresh laundering is now
fixed: mechanism-attributable distinctness, fit-set caps, engine-refreshed tiers).

## The scientific outcome
The degeneracy-valley saga is resolved on the permanent record (regression test):
V5 diagonal +54.9 → V6 true-covariance +68.8 → **V6.1 calibrated+Occam: −20.5/−31.9 — closed.**
The valley was the sum of three artifacts: the engine's own +0.755 (8.4σ) cmb_lA fitting-formula
bias (~35 nats), uncosted background drift, and diagonal overconfidence. With all three fixed,
ΛCDM wins on the admitted data, and the one surviving candidate is a *properly-certified
mechanism* (nDGP) at an honest 25/100 — derivation-credited, novelty-capped, drift-costed.

The discovery engine's real product this cycle: each rubric generation's champions were killed
by the next generation's audit within hours (V4 87.5 → V4.1; V5 55-class → V6; V6 77 → V6.1),
and every exploit is now a permanent regression test. The next campaign hunts on a rubric where
every known way to cheat scores zero.
