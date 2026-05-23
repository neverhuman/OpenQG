# ZYAL Research Loops

ZYAL runbooks are host-owned YAML documents that describe long-running loops.
Canonical runbooks live under `agent/zyal/` and use the `.zyal` extension.

OpenQG uses them for:

- literature radar
- data refresh
- hypothesis tournaments
- hero/judge prompt evolution
- theory incubation
- nightly benchmark regression
- release-candidate gating

## Validation Contract

- every runbook must use the canonical `<<<ZYAL v1:daemon id=...>>>` envelope
- every runbook must be stored as `agent/zyal/*.zyal`
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
- `agent/zyal/openqg-hero-judge-evolve.zyal` evolves OpenQG hero and judge prompts through verifier, literature, red-team, and meta-judge lanes
- `agent/zyal/openqg-hero-judge-live-smoke.zyal` runs the same loop with a smaller two-generation live population for bounded quality-trend proof
- `agent/zyal/openqg-data-refresh.zyal`, `agent/zyal/openqg-hypothesis-tournament.zyal`, `agent/zyal/openqg-theory-incubator.zyal`, and `agent/zyal/openqg-benchmark-nightly.zyal` are maintenance loops
- `agent/zyal/openqg-release-candidate.zyal` is the approval-gated release loop

## Artifact Policy

- generated and untracked: `.jekko/daemon/**`, `target/openqg/zyal/**`, `target/openqg/research/**`, raw provider receipts, daemon ledgers, and temporary smoke runbooks
- Hero/Judge generated and untracked: `target/openqg/hero-judge/<run_id>/prompt_lineage.json`, `frontier_scoreboard.json`, `promotion-decision.json`, `knowledge_compound.jsonl`, `quality_metrics.jsonl`, `quality_metrics.csv`, `quality_trend.json`, `lane_metrics.jsonl`, `lane_metrics.csv`, `hero_metrics.csv`, `judge_metrics.csv`, `reviewer_packet.json`, per-generation lane artifacts, `search/receipts.json`, and `complete.ok`
- reviewable and check-in eligible: `research/inbox/*.md` paper cards after review
- durable and check-in eligible after hardening: `research/knowledge/*.md` synthesis maps and claim ledgers
- never check in raw logs containing provider payloads, secrets, prompt-injection samples, or full fetched pages

## Commands

- `just zyal-validate` runs the repo-native validator and writes `target/openqg/zyal/preview.json`
- `just zyal-jekko-preview` runs `jekko daemon preview` across every file under `agent/zyal/` and writes `target/openqg/zyal/jekko-preview.jsonl`
- offline Hero/Judge proof: `JEKKO_DB=target/zyal-validation/openqg-hero-judge.db jankurai-runner --repo /Users/bentaylor/code/OpenQG --run-id openqg-hero-judge-smoke hero-judge-run --zyal /Users/bentaylor/code/OpenQG/agent/zyal/openqg-hero-judge-evolve.zyal --max-generations 1`
- live Hero/Judge proof: `JEKKO_DB=target/zyal-validation/openqg-hero-judge-live.db jankurai-runner --repo /Users/bentaylor/code/OpenQG --run-id openqg-hero-judge-live-smoke hero-judge-run --zyal /Users/bentaylor/code/OpenQG/agent/zyal/openqg-hero-judge-live-smoke.zyal --live`
- live Hero/Judge series: add `--runs 25` or higher to write aggregate `*-series/quality_metrics.csv`, `lane_metrics.csv`, `hero_metrics.csv`, `judge_metrics.csv`, `run_summaries.jsonl`, and `reviewer_index.json`
- live search uses `AGENT_SEARCH_LIVE=1`; missing live search providers follow the runbook `missing_provider` policy and always write receipts or fail closed
- live research runs should write receipts under `target/openqg/research/<loop>/latest/`

## Quality Metrics

Hero/Judge runs write plot-ready trend receipts:

- `quality_metrics.jsonl`: one JSON row per generation.
- `quality_metrics.csv`: the same rows for spreadsheet or chart tooling.
- `quality_trend.json`: first/latest/best generation summary and improvement flag.
- `lane_metrics.jsonl` and `lane_metrics.csv`: one row per lane artifact with `role_group`, `kind`, `score`, claim/question/rubric/evidence/storage metrics, and content hashes.
- `hero_metrics.csv` and `judge_metrics.csv`: role-separated views for showing whether theory invention and judgment quality improve independently.
- `reviewer_packet.json`: storage-safe summaries, hashes, scores, and reviewer questions; it excludes raw chain-of-thought.

Recommended first plots:

- `generation` vs `theory_quality_index` for universal physics idea quality.
- `generation` vs `question_quality_index` for falsification and research-question quality.
- `generation` vs `rubric_quality_index` and `judge_calibration_index` for judgement quality.
- `generation` vs `overall_quality_index` and `delta_overall_quality` for raw generation quality.
- `generation` vs `frontier_quality_index` and `delta_frontier_quality` for the retained best-known theory/rubric frontier.
- For series runs, plot `run_id` or trial number against final `frontier_quality_index`, plus separate hero and judge lane means from `hero_metrics.csv` and `judge_metrics.csv`.
