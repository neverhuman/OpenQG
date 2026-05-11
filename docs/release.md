# Release Process

OpenQG releases are evidence-gated, reproducible from declared commands, and immutable after publication.

## Inputs

- benchmark version
- `target/openqg/data/locks/data-lock.json`
- `target/openqg/bench-smoke/scorecard.json`
- `target/openqg/contracts/*.witness.json`
- `target/openqg/zyal/preview.json`
- `target/openqg/zyal/jekko-preview.jsonl`
- `target/jankurai/repo-score.json`
- `target/jankurai/repo-score.md`
- `target/jankurai/security/evidence.json`
- `reports/releases/draft/release-manifest.json`
- `reports/releases/draft/release-manifest.md`
- human approval receipt under `target/jankurai/review/`

## Required Commands

- `just check`
- `just score`
- `just release-pack`
- `just release-check`

`just release-check` is the release gate. It runs `just check`, then `just score`, then `just release-pack`.

## Launch Gate

Before publication, the release must have:

- a backup or restore-point receipt for the release data surface
- a rollback path that can be executed without rewriting published artifacts
- monitoring or alerting evidence for the release window
- rate-limit or abuse-control evidence when the release exposes a public interface
- integrity evidence from generated headers, hashes, and immutable draft artifacts
- provenance evidence that ties the draft bundle back to the exact source commit

## Release Evidence

- `just check` must produce the validation evidence for schema, data, benchmark, ZYAL, and security lanes
- `just score` must produce `target/jankurai/repo-score.json` and `target/jankurai/repo-score.md`
- `just security` must produce `target/jankurai/security/evidence.json`, `target/jankurai/security/lane-status.txt`, and `target/jankurai/security/lane.log`
- `just release-pack` must produce `reports/releases/draft/release-manifest.json` and `reports/releases/draft/release-manifest.md`
- `reports/releases/draft/release-manifest.json` must carry non-empty `version`, `benchmark_version`, `scorecard_hash`, `data_lock_hash`, `scorecard_path`, and `candidate_hashes`
- `reports/releases/draft/release-manifest.json` must record `approved: false` until human review is complete

## Verification Artifacts

- `target/openqg/data/locks/data-lock.json`
- `target/openqg/bench-smoke/scorecard.json`
- `target/openqg/contracts/*.witness.json`
- `target/openqg/zyal/preview.json`
- `target/openqg/zyal/jekko-preview.jsonl`
- `target/jankurai/repo-score.json`
- `target/jankurai/repo-score.md`
- `target/jankurai/security/evidence.json`
- `target/jankurai/security/lane-status.txt`
- `target/jankurai/security/lane.log`
- `reports/releases/draft/release-manifest.json`
- `reports/releases/draft/release-manifest.md`
- `target/jankurai/review/*`

Suggested mechanical checks:

```bash
jq -e '.approved == false and (.candidate_hashes | length > 0)' reports/releases/draft/release-manifest.json
test -s target/openqg/bench-smoke/scorecard.json
test -s target/jankurai/security/evidence.json
test -s target/jankurai/security/lane.log
```

## Approval Record

- human approval is recorded in the release PR review, not as hand-edited evidence under `reports/releases/draft/`
- the approval record must identify the reviewer, the approval timestamp, the PR URL, and the exact `just release-check` run that was reviewed
- keep the approval receipt under `target/jankurai/review/`
- publication may start only after the PR shows an approved review and the reviewed draft bundle is unchanged

## Acceptance Criteria

- `just release-check` succeeds on the candidate commit
- every artifact listed above exists and matches the source hashes in `reports/releases/draft/release-manifest.json`
- `reports/releases/draft/release-manifest.json` remains `approved: false` until human approval is recorded
- the release PR has an explicit approved review
- published releases are immutable

## Failure Interpretation

- `just check` fails: fix the underlying lane and rerun once
- `just score` fails: the repo score artifacts are missing or inconsistent
- `just security` fails: security evidence is incomplete and the release is blocked
- `just release-pack` fails: the draft manifest cannot be assembled from the current artifacts
- `reports/releases/draft/release-manifest.json` shows a hash mismatch or missing field: treat the bundle as stale and rerun the lane after fixing the source artifact
- `approved: false` after review is expected for the draft pack; do not publish until approval is recorded in the PR review

## Rollback

- keep the prior published release intact
- do not rewrite or delete a published release pack
- publish a new release version instead of mutating an old one

## Budgets And Stops

- stop on the first failing command
- stop if any required artifact is missing, empty, or hash-inconsistent
- stop if any candidate manifest hash is missing from `reports/releases/draft/release-manifest.json`
- stop if the release PR does not have an explicit approved review
- stop if the draft bundle or published bundle would need to be mutated in place
- do not spend operator time on unbounded retries; fix the source artifact, rerun the lane once, then escalate

## Notes

- automation may assemble release evidence
- automation may not bypass review or publish without approval
- keep draft release output under `reports/releases/draft/` until the approval gate is satisfied
