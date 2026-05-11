# Testing

OpenQG uses explicit lanes instead of ad hoc checks.

## Core Lanes

- `just fast`
- `just schema-sync`
- `just schema-check`
- `just data-verify`
- `just bench-smoke`
- `just zyal-validate`
- `just check`
- `just score`
- `just security`
- `just release-check`

## Recommended Order

1. `just fast`
2. `just schema-sync` after editing `contracts/specs/` or `contracts/registry.yml`
3. `just schema-check`
4. `just check`
5. `just score`
6. `just release-check` only after the earlier lanes are clean

## Evidence

- repo score JSON: `target/jankurai/repo-score.json`
- repo score markdown: `target/jankurai/repo-score.md`
- security evidence: `target/jankurai/security/evidence.json`
- security lane status: `target/jankurai/security/lane-status.txt`
- security lane log: `target/jankurai/security/lane.log`
- security command logs: `target/jankurai/security/*.log`
- benchmark scorecard: `target/openqg/bench-smoke/scorecard.json`
- data lock: `target/openqg/data/locks/data-lock.json`
- ZYAL preview: `target/openqg/zyal/preview.json`
- draft release bundle: `reports/releases/draft/release-manifest.json`
- draft release summary: `reports/releases/draft/release-manifest.md`
- human review receipts: `target/jankurai/review/*`

## Budgets

- stop after the first failing rerun of a lane
- do not rerun a failing lane until the source artifact is fixed
- do not expand a lane with ad hoc steps outside the declared command
- keep release review time bounded; if a human approval receipt is missing, stop and record that gap

## Stop Conditions

- `just schema-check` fails: regenerate `contracts/generated/schemas/` with `just schema-sync`
- `just data-verify` fails: fix the data registry or lock and rerun once
- `just bench-smoke` fails: fix the benchmark manifest or fixtures and rerun once
- `just zyal-validate` fails: fix the runbook and rerun once
- `just security` fails or reports a missing required tool: record the lane log and stop
- `just release-check` fails: fix the underlying lane, rerun once, then stop if it still fails

## Release Review Receipt

- a release candidate must pass `just release-check`
- the candidate must retain `CHANGELOG.md`, `docs/release.md`, and the draft release pack
- the approval record must identify the reviewer, approval timestamp, PR URL, and the exact `just release-check` run
- keep the human approval receipt under `target/jankurai/review/`
