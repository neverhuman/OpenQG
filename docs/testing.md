# Testing

OpenQG uses explicit lanes instead of ad hoc checks.

## Core Lanes

- `just fmt-check`
- `just core-test`
- `just bench-test`
- `just fast`
- `just schema-sync`
- `just schema-check`
- `just data-verify`
- `just bench-smoke`
- `just zyal-validate`
- `just zyal-jekko-preview`
- `just check`
- `just score`
- `just security`
- `just release-check`

## Recommended Order

1. `just fmt-check`
2. `just core-test`
3. `just bench-test`
4. `just fast`
5. `just schema-sync` after editing `contracts/specs/` or `contracts/registry.yml`
6. `just schema-check`
7. `just zyal-jekko-preview`
8. `just check`
9. `just score`
10. `just release-check` only after the earlier lanes are clean

## Evidence

- repo score JSON: `target/jankurai/repo-score.json`
- repo score markdown: `target/jankurai/repo-score.md`
- security evidence: `target/jankurai/security/evidence.json`
- security lane status: `target/jankurai/security/lane-status.txt`
- security lane log: `target/jankurai/security/lane.log`
- security command logs: `target/jankurai/security/*.log`
- benchmark scorecard: `target/openqg/bench-smoke/scorecard.json`
- data lock: `target/openqg/data/locks/data-lock.json`
- contract witnesses: `target/openqg/contracts/*.witness.json`
- ZYAL preview: `target/openqg/zyal/preview.json`
- Jekko ZYAL preview: `target/openqg/zyal/jekko-preview.jsonl`
- literature radar receipts: `target/openqg/research/literature-radar/latest/*`
- knowledge hardening receipts: `target/openqg/research/knowledge-hardening/latest/*`
- knowledge map: `research/knowledge/openqg-literature-map.md`
- draft release bundle: `reports/releases/draft/release-manifest.json`
- draft release summary: `reports/releases/draft/release-manifest.md`
- human review receipts: `target/jankurai/review/*`

## Repair Receipts

- validation errors should carry `purpose`, `reason`, `common_fixes`, `docs_url`, and `repair_hint`
- keep the failing command and matching artifact path in the receipt
- rerun the declared lane only after the source fix lands
- if a lane needs a human decision, record the exact stop condition instead of inventing new evidence

## Budgets

- stop after the first failing rerun of a lane
- each lane gets one repair rerun after the source artifact is fixed
- do not expand a lane with ad hoc steps outside the declared command
- declare a quota and a kill switch for paid or unbounded work before starting it
- use `.halt-*` files or equivalent stop-condition evidence for long-lived ZYAL loops

## Stop Conditions

- `just schema-check` fails: regenerate `contracts/generated/schemas/` with `just schema-sync`
- `just data-verify` fails: fix the data registry or lock and rerun once
- `just bench-smoke` fails: fix the benchmark manifest or fixtures and rerun once
- `just zyal-validate` fails: fix the runbook and rerun once
- `just zyal-jekko-preview` fails: fix the runbook envelope or Jekko compatibility issue and rerun once
- `just security` fails or reports a missing required tool: record the lane log and stop
- `just release-check` fails: fix the underlying lane, rerun once, then stop if it still fails

## Release Review Receipt

- a release candidate must pass `just release-check`
- the candidate must retain `CHANGELOG.md`, `docs/release.md`, and the draft release pack
- the approval record must identify the reviewer, approval timestamp, PR URL, and the exact `just release-check` run
- keep the human approval receipt under `target/jankurai/review/`
