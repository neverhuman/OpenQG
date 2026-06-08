#!/usr/bin/env bash
# Launch a theory-evolve production run (the new symbolic engine) with full logging.
# Usage: tools/theory-evolve-run.sh [GENERATIONS] [RUN_ID] [SEED]
# Env: POP, OBS, PROPOSALS, PROPOSER_CMD, OUT_ROOT, CHECKPOINT_EVERY, CARGO_TARGET_DIR.
set -euo pipefail
ROOT="$(git rev-parse --show-toplevel 2>/dev/null || true)"
[ -z "$ROOT" ] && ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

GENS="${1:-1000}"
RUN_ID="${2:-prod-${GENS}}"
SEED="${3:-1}"
POP="${POP:-48}"
OBS="${OBS:-data/fixtures/cosmology/tier0-combined.jsonl}"
PROPOSALS="${PROPOSALS:-data/fixtures/proposals/example.jsonl}"
OUT_ROOT="${OUT_ROOT:-target/openqg/theory-evolve}"
CHECKPOINT_EVERY="${CHECKPOINT_EVERY:-1}"
LOG_DIR=".jekko/run-logs/theory-evolve"
mkdir -p "$LOG_DIR"
LOG="${LOG_DIR}/${RUN_ID}.log"
RUN_DIR="${OUT_ROOT}/runs/${RUN_ID}"

echo "theory-evolve run: gens=$GENS run_id=$RUN_ID seed=$SEED pop=$POP"
echo "  observables: $OBS"
echo "  run dir:     $RUN_DIR"
echo "  log:         $LOG"
echo "  monitor:     tools/theory-evolve-monitor.sh $RUN_ID"

args=(theory evolve
  --observables "$OBS"
  --output-root "$OUT_ROOT"
  --run-id "$RUN_ID"
  --checkpoint-every "$CHECKPOINT_EVERY"
  --generations "$GENS"
  --population "$POP"
  --seed "$SEED")
[ -n "${PROPOSALS:-}" ] && [ -f "$PROPOSALS" ] && args+=(--proposals "$PROPOSALS")
[ -n "${PROPOSER_CMD:-}" ] && args+=(--proposer-cmd "$PROPOSER_CMD")

CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target/fast}" \
  cargo run -q --release -p openqg-bench -- "${args[@]}" 2>&1 | tee "$LOG"
