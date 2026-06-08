#!/usr/bin/env bash
set -euo pipefail

HOME="${HOME:-/home/ubuntu}"
export PATH="/home/ubuntu/.local/bin:${PATH:-/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin}"

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

cargo_bin="${CARGO_BIN:-}"
if [[ -z "$cargo_bin" && -x "/home/ubuntu/.cargo/bin/cargo" ]]; then
  cargo_bin="/home/ubuntu/.cargo/bin/cargo"
elif [[ -z "$cargo_bin" && -x "/home/ubuntu/.local/bin/cargo" ]]; then
  cargo_bin="/home/ubuntu/.local/bin/cargo"
elif [[ -z "$cargo_bin" ]]; then
  cargo_bin="$(command -v cargo || true)"
fi
if [[ -z "$cargo_bin" ]]; then
  echo "missing cargo on PATH after sourcing ~/.bashrc" >&2
  exit 127
fi

quality_gate_passed() {
  local gate="$1"
  [[ -s "$gate" ]] || return 1
  if command -v jq >/dev/null 2>&1; then
    jq -e '.passed == true' "$gate" >/dev/null 2>&1
  else
    grep -Eq '"passed"[[:space:]]*:[[:space:]]*true' "$gate"
  fi
}

if [[ "$generations" -ge 100 ]]; then
  qual_gate="target/openqg/zyal-genome/hybrid/runs/hybrid-v2-50/quality-gate.json"
  if ! quality_gate_passed "$qual_gate"; then
    echo "hybrid-v2-50 quality gate has not passed; refusing ${run_id}" >&2
    exit 2
  fi
fi

preflight=(
  "$cargo_bin" run -p openqg-bench --
  zyal genome preflight
  --runbook ZYAL/runs/run-hybrid-1000-v2.zyal
  --run-id "$run_id"
  --require-hard-backend
  --live-smoke
  --jailgun-artifact-smoke
  --jailgun-artifact-smoke-extension json
  --live-selective
)

cmd=(
  "$cargo_bin" run -p openqg-bench --
  zyal genome run
  --variant hybrid
  --runbook ZYAL/runs/run-hybrid-1000-v2.zyal
  --generations "$generations"
  --run-id "$run_id"
  --checkpoint-every 1
  --resume
  --live-selective
)

echo "Launching ZYAL ${run_id} (${generations} generations) from $repo_root"
echo "Monitor locally with: just zyal-genome-hybrid-v2-100-watch"
echo "Fallback monitor: bash tools/zyal-genome-live-log.sh ${run_id} 5"
echo "Checkpoint: ${run_dir}/checkpoint.json"
printf 'Preflight:'
printf ' %q' "${preflight[@]}"
printf '\nCommand:'
printf ' %q' "${cmd[@]}"
printf '\n'

if [[ -n "${DRY_RUN:-}" ]]; then
  echo "DRY_RUN=1; not launching"
  exit 0
fi

"${preflight[@]}"
exec "${cmd[@]}"
