#!/usr/bin/env bash
# Live monitor for a theory-evolve run: progress + tail of the per-generation telemetry.
# Usage: tools/theory-evolve-monitor.sh [RUN_ID]
# Env: OUT_ROOT, INTERVAL (refresh secs), ONCE=1 (print once and exit).
set -euo pipefail
ROOT="$(git rev-parse --show-toplevel 2>/dev/null || true)"
[ -z "$ROOT" ] && ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

RUN_ID="${1:-prod-1000}"
OUT_ROOT="${OUT_ROOT:-target/openqg/theory-evolve}"
RUN_DIR="${OUT_ROOT}/runs/${RUN_ID}"
TS="${RUN_DIR}/metrics-timeseries.jsonl"
CFG="${RUN_DIR}/run-config.json"

status_line() {
  if [ ! -f "$CFG" ]; then echo "run not started: $RUN_DIR"; return; fi
  local target; target=$(jq -r '.generations' "$CFG" 2>/dev/null || echo "?")
  local last; last=$(tail -n 1 "$TS" 2>/dev/null || true)
  if [ -z "$last" ]; then echo "waiting for first generation (target $target)..."; return; fi
  echo "$last" | jq -r --arg t "$target" \
    '"gen \(.generation)/\($t)  qd=\(.qd_score)  cells=\(.archive_cells)  frontier=\(.frontier_margin)  anchor_health=\(.anchor_health)  honest=\(.calibration_honest)  champion=\(.champion_fitness) (\(.champion_id // "-"))"'
}

if [ "${ONCE:-0}" = "1" ]; then status_line; exit 0; fi

INTERVAL="${INTERVAL:-3}"
while true; do
  clear 2>/dev/null || true
  echo "== theory-evolve monitor: $RUN_ID =="
  status_line
  echo "--- last 6 generations ---"
  tail -n 6 "$TS" 2>/dev/null | jq -c \
    '{gen:.generation, qd:.qd_score, cells:.archive_cells, frontier:.frontier_margin, anchor:.anchor_health, honest:.calibration_honest, champ:.champion_fitness}' \
    2>/dev/null || echo "(no telemetry yet)"
  # Stop the loop once the run has produced its final summary.
  if [ -f "${RUN_DIR}/run-summary.json" ]; then echo "--- run complete ---"; break; fi
  sleep "$INTERVAL"
done
