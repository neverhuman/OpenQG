# Hybrid V2 Progress 2026-06-07

This note records the durable state of the live ZYAL hybrid V2 run that followed
the Jailgun generic-artifact fixes on 2026-06-07. Generated receipts remain in
their generator-owned locations and are not copied into git.

## Jailgun Artifact Fixes

- `957ba22` in `/home/ubuntu/jailgun`: generic ChatGPT artifact downloads now
  accept exact requested targets instead of requiring archive-shaped
  `.tar.gz` outputs.
- `af29996` in `/home/ubuntu/jailgun`: tab prompt prefixes no longer pollute
  the model-visible request.
- `43f8407` in `/home/ubuntu/jailgun`: malformed sandbox artifact responses can
  be repaired when the caller opts in.
- `3f241e1` in `/home/ubuntu/openQG`: OpenQG passes absolute Jailgun prompt
  paths, so the Jailgun server working directory does not affect prompts.
- `1133ec0` in `/home/ubuntu/openQG`: ZYAL Jailgun runs opt into one artifact
  repair attempt while leaving generic conversation recovery disabled.

## Pipeline Result

Command:

```sh
rtk bash tools/zyal-genome-hybrid-v2-pipeline.sh
```

The pipeline ran in order and stopped at the first failed gate:

| Stage | Result | Live calls | Jailgun live calls |
| --- | --- | ---: | ---: |
| `hybrid-v2-10` | passed | 7/7 ok | clean |
| `hybrid-v2-20` | passed | 7/7 ok | clean |
| `hybrid-v2-50` | failed quality gate | 19/19 ok | 15/15 ok |

`hybrid-v2-100` was not launched.

The `hybrid-v2-50` checkpoint reached `complete_generation = 50`. The quality
gate failed only on distribution metrics:

| Check | Observed | Threshold |
| --- | ---: | ---: |
| `regression_rate` | `0.326531` | `0.25` |
| `novelty_champion_rate` | `0.04` | `0.05` |

Transport and proof counters were clean at the failing gate:

- `failed_live_calls = 0`
- `failed_jailgun_live_calls = 0`
- `hard_stage_jailgun_proof_missing = 0`
- `degraded_route_count = 0`

## Local Evidence Paths

- Pipeline log: `.jekko/run-logs/zyal-genome-hybrid-v2/pipeline.log`
- Monitor snapshot:
  `.jekko/run-logs/zyal-genome-hybrid-v2/monitor-hybrid-v2-50.log`
- Run receipts:
  `target/openqg/zyal-genome/hybrid/runs/hybrid-v2-50/`
- Quality gate:
  `target/openqg/zyal-genome/hybrid/runs/hybrid-v2-50/quality-gate.json`
- Live ledger:
  `target/openqg/zyal-genome/hybrid/runs/hybrid-v2-50/live-call-ledger.jsonl`
