# ZYAL Architecture

The ZYAL genome engine is a standalone experimental workspace that reuses the
OpenQG runbook envelope but does not replace the canonical `agent/zyal/`
research loops.

## Design Goals

- keep stage definitions reusable and replayable
- keep run variants explicit instead of hidden behind flags
- emit typed JSON and JSONL evidence for offline scoring
- make a single run and a thousand-generation run use the same artifact shape

## Surface Map

- `ZYAL/stages/`: stage registry entries
- `ZYAL/concepts/`: seed concept families used by the hybrid island model
- `ZYAL/runs/`: variant runbooks and the shared template
- `cargo run -p openqg-bench -- zyal genome ...`: primary runner entry point
- `scripts/run-zyal-genome*.sh`: thin compatibility wrappers around the Rust runner
- `scripts/emit-offline-eval.sh`: thin compatibility wrapper around the Rust runner
- `scripts/validate-eval-schema.sh`: thin compatibility wrapper around the Rust runner

## Evidence Flow

1. A runbook selects a routing track and output root.
2. The runner loads the stage registry and emits stage-local evidence.
3. Hybrid runs also emit generation-local population snapshots under
   `generations/g000*/`, plus information, concept, novelty, and lineage ledgers.
4. Per-stage events accumulate into `run-events.jsonl` and `stage-ledger.jsonl`.
5. The offline evaluator computes deterministic rollups and comparison outputs.
6. The schema validator checks the JSON artifacts and invariant rules.

## Variant Behavior

- `pure-jnoccio` uses a Jnoccio-first route policy with top-20% routing for the
  hard stage families.
- `hybrid` sends hard synthesis and repair stages through the Jailgun wrapper
  and records fallback penalties when the wrapper is unavailable. It also runs
  the six-island population model for concept evolution and novelty pressure.
- `jailgun-only` keeps all stage work on the Jailgun wrapper and captures the
  strongest provenance fields in the emitted metadata.
