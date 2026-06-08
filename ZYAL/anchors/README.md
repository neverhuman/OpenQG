# Calibration anchors & decoys (Phase 0)

A frozen, hand-curated set used to keep the (eventually adversarial) judge **honest**. These
are the ground truth against which judge calibration and adversary drift are measured.

| file | kind | expected outcome | tests |
| --- | --- | --- | --- |
| `good-lcdm-baseline.json` | anchor | `survive` (delta == 0) | the reference must never be killed |
| `good-h0-tension-resolver.json` | anchor | `survive` (delta > 0) | a robust theory beats the baseline |
| `decoy-h0-100.json` | decoy | `die` | physics veto kills nonsense |
| `decoy-omega-runaway.json` | decoy | `die` | physics veto kills nonsense |
| `probe-overfit-echo.json` | probe | `demote_by_parsimony` | BIC ranks a 7-param echo below a 5-param resolver |

Roles across the phased rollout:
- **Phase 0 (now):** the physics honesty-anchor / veto scores these with the real
  `score_metrics` over `data/fixtures/tension/`. Decoys must score below baseline; good
  anchors must not. Verified by `tension_fixture_files_have_headroom_and_calibrate_anchors`.
- **Phase 2+:** the same set calibrates the LLM judge — decoys the judge MUST kill, the
  baseline it MUST keep — and feeds the Spearman-vs-physics honesty gate.
- **Phase 3:** if the escalating attack archive starts killing the `survive` anchors, the
  adversary has drifted into unfair attacks → prune/recalibrate.

These artifacts also double as the **canonical example** of the Phase-1 structured-theory
schema (`pillars` → claim / mechanism / assumptions / declared_limits / falsifiers, plus an
assembled `predictions` array and a `parameter_count`).

Curation/sign-off is an open item for the lead (see the plan's "Open items").
