#!/usr/bin/env bash
set -euo pipefail

HOME="${HOME:-/home/ubuntu}"

if [[ -f "${HOME}/.bashrc" ]]; then
  # shellcheck disable=SC1090
  source "${HOME}/.bashrc" >/dev/null 2>&1 || true
fi

export DISPLAY="${DISPLAY:-:99}"
export JAILGUN_SERVER_URL="${JAILGUN_SERVER_URL:-http://127.0.0.1:8797}"

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

run_id="${1:-hybrid-v2-100}"
generations="${2:-100}"
run_dir="target/openqg/zyal-genome/hybrid/runs/${run_id}"
lock_dir=".jekko/locks"
log_dir=".jekko/run-logs/zyal-genome-hybrid-v2"
pid_file="${lock_dir}/zyal-genome-${run_id}.pid"
log_file="${log_dir}/${run_id}-mcp-launch.log"
mkdir -p "$run_dir" "$lock_dir" "$log_dir"

pid_matches_run() {
  local pid="$1"
  local args
  args="$(ps -p "$pid" -o args= 2>/dev/null || true)"
  [[ "$args" == *"openqg-bench"* \
    && "$args" == *"zyal genome run"* \
    && "$args" == *"--run-id ${run_id}"* \
    && "$args" != *"pgrep -f"* \
    && "$args" != *"rtk pgrep"* ]]
}

active_pid() {
  local pid
  while IFS= read -r pid; do
    if [[ "$pid" != "$$" ]] && pid_matches_run "$pid"; then
      printf '%s\n' "$pid"
      return 0
    fi
  done < <(pgrep -f "openqg-bench zyal genome run.*--run-id ${run_id}" || true)
  return 1
}

pid=""
if [[ -s "$pid_file" ]]; then
  recorded_pid="$(tr -d '[:space:]' <"$pid_file")"
  if [[ "$recorded_pid" =~ ^[0-9]+$ ]] && pid_matches_run "$recorded_pid"; then
    pid="$recorded_pid"
  elif [[ -n "$recorded_pid" ]]; then
    echo "Ignoring stale pid file ${pid_file} (${recorded_pid})"
  fi
fi

if [[ -z "$pid" ]]; then
  pid="$(active_pid || true)"
fi

if [[ -n "$pid" ]]; then
  printf '%s\n' "$pid" >"$pid_file"
  echo "ZYAL ${run_id} is already running; adopted pid=$pid"
else
  echo "Starting ZYAL ${run_id} in background"
  nohup bash tools/zyal-genome-hybrid-v2-launch.sh "$run_id" "$generations" >"$log_file" 2>&1 &
  pid="$!"
  printf '%s\n' "$pid" >"$pid_file"
  echo "Started pid=$pid"
fi

echo "MCP log: $log_file"
echo "Monitor locally with: just zyal-genome-hybrid-v2-100-watch"
echo "Fallback monitor: bash tools/zyal-genome-live-log.sh ${run_id} 5"
echo "Checkpoint: ${run_dir}/checkpoint.json"

if [[ -s "${run_dir}/checkpoint.json" ]] && command -v jq >/dev/null 2>&1; then
  jq -r '{complete_generation,target_generation,complete_generation_id,updated_at}' "${run_dir}/checkpoint.json"
fi
