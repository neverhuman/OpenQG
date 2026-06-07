# ZYAL Research Loops

ZYAL runbooks are host-owned YAML documents that describe long-running loops.
Canonical runbooks live under `agent/zyal/` and use the `.zyal` extension.

OpenQG uses them for:

- literature radar
- data refresh
- hypothesis tournaments
- theory incubation
- nightly benchmark regression
- release-candidate gating

## Validation Contract

- every runbook must use the canonical `<<<ZYAL v1:daemon id=...>>>` envelope
- canonical daemon runbooks must be stored as `agent/zyal/*.zyal`
- experimental genome runbooks must be stored as `ZYAL/runs/*.zyal`
- legacy `.zyal.yml` and `.zyal.yaml` files are rejected
- the closing sentinel id must match the opening id
- the trailing `ZYAL_ARM RUN_FOREVER id=...` line must match the same id
- `stop.all` is required for daemon loops
- `stop.any` is reserved for cases where the loop intentionally needs `all AND any`
- `ui.theme` is the supported UI field; `ui.mode` is rejected
- `checkpoint.when` should be `after_verified_change`, `manual`, or `on_error`
- shell checks should always include `assert.exit_code: 0`

## Canonical Runbooks

- `agent/zyal/openqg-literature-radar.zyal` is the highest-value search loop
- `agent/zyal/openqg-knowledge-hardening.zyal` turns reviewed cards into `research/knowledge/openqg-literature-map.md`
- `agent/zyal/openqg-data-refresh.zyal`, `agent/zyal/openqg-hypothesis-tournament.zyal`, `agent/zyal/openqg-theory-incubator.zyal`, and `agent/zyal/openqg-benchmark-nightly.zyal` are maintenance loops
- `agent/zyal/openqg-release-candidate.zyal` is the approval-gated release loop

## Artifact Policy

- generated and untracked: `.jekko/daemon/**`, `target/openqg/zyal/**`, `target/openqg/zyal-genome/**`, `target/openqg/research/**`, raw provider receipts, daemon ledgers, and temporary smoke runbooks
- reviewable and check-in eligible: `research/inbox/*.md` paper cards after review
- durable and check-in eligible after hardening: `research/knowledge/*.md` synthesis maps and claim ledgers
- never check in raw logs containing provider payloads, secrets, prompt-injection samples, or full fetched pages

## Commands

- `just zyal-validate` runs the repo-native validator and writes `target/openqg/zyal/preview.json`
- `just zyal-validate` also validates the experimental genome runbooks and writes `target/openqg/zyal-genome/preview.json`
- `just zyal-jekko-preview` runs the Jekko-compatible preview lane across every file under `agent/zyal/` and writes `target/openqg/zyal/jekko-preview.jsonl`
- `just zyal-genome-smoke` runs the three genome routing tracks and writes `target/openqg/zyal-genome/{variant}/latest/*`
- live research runs should write receipts under `target/openqg/research/<loop>/latest/`
