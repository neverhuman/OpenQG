# ZYAL Genome Engine

The `ZYAL/` workspace is the experimental genome loop scaffold for OpenQG.
It is separate from the canonical daemon runbooks under `agent/zyal/` and is
designed for deterministic, replayable evaluation of theory-gene evolution
tracks.

## What lives here

- `stages/`: reusable stage definitions for the genome pipeline
- `runs/`: runnable ZYAL runbook templates for the three routing tracks
- `schemas/`: JSON schema for offline evaluation artifacts
- `concepts/`: seed stage concepts for hybrid population evolution
- `docs/`: short architecture and metric notes
- `scripts/`: runner, evaluator, and validation entry points

## Routing Tracks

- `pure-jnoccio`: all work is routed through Jnoccio, with hard stages tagged
  for top-20% model routing.
- `hybrid`: light stages stay on Jnoccio; hard synthesis and repair stages use
  the Jailgun wrapper when available. The ambitious default is a deterministic
  population run with 128 generations, 6 islands, and 24 candidates per
  generation.
- `jailgun-only`: every stage is issued through the Jailgun wrapper with
  stricter timeout and retry metadata.

## Quick Start

Run the canonical layout validation first:

```bash
just zyal-validate
```

Run the genome smoke lane for all three tracks:

```bash
just zyal-genome-smoke
```

Run the hybrid population smoke lane:

```bash
just zyal-genome-hybrid-smoke
```

Run the ambitious hybrid default:

```bash
just zyal-genome-hybrid-128
```

Run a single variant directly:

```bash
cargo run -p openqg-bench -- zyal genome run --variant pure-jnoccio --max-generations 1
cargo run -p openqg-bench -- zyal genome run --variant hybrid --max-generations 1
cargo run -p openqg-bench -- zyal genome run --variant jailgun-only --max-generations 1
```

## Output Layout

Per variant, the runner writes to:

```text
target/openqg/zyal-genome/<variant>/latest/
target/openqg/zyal-genome/<variant>/runs/<run_id>/
target/openqg/zyal-genome/hybrid/runs/<run_id>/generations/g0001/
```

Core artifacts:

- `run-events.jsonl`: append-only event stream
- `stage-ledger.jsonl`: stage-by-stage summary ledger
- `run-summary.json`: deterministic aggregate summary
- `offline-eval.json`: scorecard-like offline evaluation
- `pareto-snapshot.json`: frontier view for the run
- `generations/g000*/population-snapshot.json`: hybrid candidate population
- `information-ledger.jsonl`: accepted local information cards
- `concept-gene-ledger.jsonl`: seed and source-derived concept genes
- `stage-concept-ledger.jsonl`: champion stage concept choices
- `novelty-archive.json`: best novelty-preserving candidates
- `lineage-graph.jsonl` and `lineage-graph.md`: candidate ancestry

The root comparison lane writes:

- `target/openqg/zyal-genome/comparison.json`
- `target/openqg/zyal-genome/comparison.md`
- `target/openqg/zyal-genome/hybrid-novelty-archive.json`
- `target/openqg/zyal-genome/hybrid-island-leaderboard.json`
- `target/openqg/zyal-genome/hybrid-lineage-graph.md`

## Metric Blend

Hybrid scoring uses:

- `local_score`: 0.24
- `interface_score`: 0.16
- `macro_score`: 0.24
- `innovation_score`: 0.18
- `novelty_score`: 0.13
- `failure_penalty`: 0.05

Useful run-level regressions to watch:

- `best_score_seen`
- `rolling_5_median`
- `best_nonregressive_delta`
- `unique_contributions`
- `decoy_failures`
