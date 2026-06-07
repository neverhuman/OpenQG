#!/usr/bin/env bash
set -euo pipefail

HOME="${HOME:-/home/ubuntu}"

if [[ -f "${HOME}/.bashrc" ]]; then
  # Keep MCP launches aligned with the interactive TUI environment.
  # shellcheck disable=SC1090
  source "${HOME}/.bashrc" >/dev/null 2>&1 || true
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

run_dir="target/openqg/zyal-genome/hybrid/runs/hybrid-1000"
pid_file="${run_dir}/mcp-run.pid"
log_file="${run_dir}/mcp-launch.log"
mkdir -p "$run_dir"

active_pid() {
  local pid
  while IFS= read -r pid; do
    if [[ "$pid" != "$$" ]] && ps -p "$pid" >/dev/null 2>&1; then
      printf '%s\n' "$pid"
      return 0
    fi
  done < <(pgrep -f 'openqg-bench zyal genome run.*--run-id hybrid-1000' || true)
  return 1
}

pid=""
if [[ -s "$pid_file" ]]; then
  recorded_pid="$(tr -d '[:space:]' <"$pid_file")"
  if [[ "$recorded_pid" =~ ^[0-9]+$ ]] && ps -p "$recorded_pid" >/dev/null 2>&1; then
    pid="$recorded_pid"
  fi
fi

if [[ -z "$pid" ]]; then
  pid="$(active_pid || true)"
fi

if [[ -n "$pid" ]]; then
  printf '%s\n' "$pid" >"$pid_file"
  echo "ZYAL hybrid-1000 is already running; adopted pid=$pid"
else
  echo "Starting ZYAL hybrid-1000 in background"
  nohup bash tools/zyal-genome-hybrid-1000-launch.sh >"$log_file" 2>&1 &
  pid="$!"
  printf '%s\n' "$pid" >"$pid_file"
  echo "Started pid=$pid"
fi

echo "MCP log: $log_file"
echo "Monitor locally with: just zyal-genome-hybrid-1000-watch"
echo "Fallback monitor: bash tools/zyal-genome-live-log.sh hybrid-1000 5"
echo "Checkpoint: ${run_dir}/checkpoint.json"

if [[ -s "${run_dir}/checkpoint.json" ]] && command -v jq >/dev/null 2>&1; then
  jq -r '{complete_generation,target_generation,complete_generation_id,updated_at}' "${run_dir}/checkpoint.json"
fi
