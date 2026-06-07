# ZYAL Metrics

The scaffold uses a fixed blend so runs stay comparable while the engine is
still experimental.

## Core Scores

- `local_score`: stage-level execution quality
- `interface_score`: contract and handoff quality
- `macro_score`: whole-run coherence
- `innovation_score`: novelty or useful mutation signal
- `failure_penalty`: router, retry, or validation penalty
- `final_score`: weighted blend of the above

## Comparison Fields

- `best_score_seen`: highest final score observed in the run
- `rolling_5_median`: median of the last five generation scores
- `best_nonregressive_delta`: strongest positive step without a regression
- `unique_contributions`: count of distinct stage contributions above threshold
- `decoy_failures`: count of stages that failed or fell back to a degraded path

## Routing Metadata

- `route_backend`: `jnoccio` or `jailgun`
- `route_tier`: `standard`, `top20_pct`, or `manual`
- `judge.family`: model family or wrapper family used for the decision
- `judge.provenance`: `scripted-route-policy`, `backend-wrapper`, or fallback
