#!/usr/bin/env bash
# Launch the updated ZYAL robustness genome run and render a tracked summary.
#
# This is the rebuilt engine: real survival fitness (whitebox theories graded by a co-evolving
# adversarial critic), MAP-Elites diversity, and an auto-written quality gate. The default
# (deterministic) critic is cheap-first and needs no live stack; the live-LLM critic is the
# later "scale" tier.
#
# Usage: tools/zyal-robustness-run.sh [generations] [run-id] [seed]
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

GENS="${1:-1000}"
RUN_ID="${2:-robustness-${GENS}}"
SEED="${3:-7}"
OUT_ROOT="target/openqg/zyal-genome/robustness"
RUN_DIR="${OUT_ROOT}/runs/${RUN_ID}"
LOG_DIR=".jekko/run-logs/zyal-robustness"
mkdir -p "$LOG_DIR"
LOG="${LOG_DIR}/${RUN_ID}.log"

echo "launch ${RUN_ID}: ${GENS} generations, seed=${SEED} -> ${RUN_DIR}" | tee "$LOG"
rm -rf "$RUN_DIR"

cargo run -q -p openqg-bench -- zyal genome run \
  --variant hybrid --generations "$GENS" --run-id "$RUN_ID" \
  --output-root "$OUT_ROOT" --seed "$SEED" 2>&1 | tee -a "$LOG"

echo "" | tee -a "$LOG"
"$(dirname "$0")/zyal-robustness-report.sh" "$RUN_DIR" | tee -a "$LOG"
echo "log: ${LOG}"
