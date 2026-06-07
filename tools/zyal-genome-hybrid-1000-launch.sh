#!/usr/bin/env bash
set -euo pipefail

HOME="${HOME:-/home/ubuntu}"
export PATH="/home/ubuntu/.local/bin:${PATH:-/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin}"

if [[ -f "${HOME}/.bashrc" ]]; then
  # Keep MCP launches aligned with the interactive TUI environment.
  # shellcheck disable=SC1090
  source "${HOME}/.bashrc" >/dev/null 2>&1 || true
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

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

cmd=(
  "$cargo_bin" run -p openqg-bench --
  zyal genome run
  --variant hybrid
  --runbook ZYAL/runs/run-hybrid-1000.zyal
  --generations 1000
  --run-id hybrid-1000
  --checkpoint-every 1
  --resume
  --live-selective
)

echo "Launching ZYAL hybrid-1000 from $repo_root"
echo "Monitor locally with: just zyal-genome-hybrid-1000-watch"
echo "Fallback monitor: bash tools/zyal-genome-live-log.sh hybrid-1000 5"
echo "Checkpoint: target/openqg/zyal-genome/hybrid/runs/hybrid-1000/checkpoint.json"
echo "Generation ledger: target/openqg/zyal-genome/hybrid/runs/hybrid-1000/generation-ledger.jsonl"
echo "Metrics ledger: target/openqg/zyal-genome/hybrid/runs/hybrid-1000/metrics-timeseries.jsonl"
echo "Plot index: target/openqg/zyal-genome/hybrid/runs/hybrid-1000/plot-index.json"
printf 'Command:'
printf ' %q' "${cmd[@]}"
printf '\n'

if [[ -n "${DRY_RUN:-}" ]]; then
  echo "DRY_RUN=1; not launching"
  exit 0
fi

exec "${cmd[@]}"
