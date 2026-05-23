# ZYAL Research Loops

OpenQG now keeps one live ZYAL runbook:

- `agent/zyal/openqg-hero-judge-evolve.zyal`

Purpose:

- evolve harder questions, harder answers, and the judgment around them
- keep the loop versioned and reproducible
- write storage-safe metrics and receipts under `target/openqg/hero-judge/`

Validation:

- `just zyal-validate` validates the runbook envelope and schema
- `JEKKO_DB=target/zyal-validation/openqg-hero-judge-live.db jankurai-runner --repo /Users/bentaylor/code/OpenQG --run-id openqg-hero-judge-live-series hero-judge-run --zyal /Users/bentaylor/code/OpenQG/agent/zyal/openqg-hero-judge-evolve.zyal --live --runs 25`

25-run series outputs:

- `target/openqg/hero-judge/<run_id>/series_summary.csv`
- `target/openqg/hero-judge/<run_id>/quality_metrics.csv`
- `target/openqg/hero-judge/<run_id>/lane_metrics.csv`
- `target/openqg/hero-judge/<run_id>/hero_metrics.csv`
- `target/openqg/hero-judge/<run_id>/judge_metrics.csv`
- `target/openqg/hero-judge/<run_id>/run_summaries.jsonl`
- `target/openqg/hero-judge/<run_id>/reviewer_index.json`

Artifacts to keep out of git:

- raw provider payloads
- chain-of-thought content
- temporary receipts and per-run `complete.ok` markers
