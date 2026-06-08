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

run_root="target/openqg/zyal-genome/hybrid/runs"
genome_root="target/openqg/zyal-genome"
pipeline_dir=".jekko/run-logs/zyal-genome-hybrid-v2"
lock_dir=".jekko/locks"
archive_dir="${run_root}/archive"
log_file="${pipeline_dir}/pipeline.log"
poll_seconds="${PIPELINE_POLL_SECONDS:-60}"
mkdir -p "$pipeline_dir" "$lock_dir" "$archive_dir"

exec > >(tee -a "$log_file") 2>&1

timestamp() {
  date -u '+%Y-%m-%dT%H:%M:%SZ'
}

log() {
  printf '[%s] %s\n' "$(timestamp)" "$*"
}

cargo_bin="${CARGO_BIN:-}"
if [[ -z "$cargo_bin" && -x "/home/ubuntu/.cargo/bin/cargo" ]]; then
  cargo_bin="/home/ubuntu/.cargo/bin/cargo"
elif [[ -z "$cargo_bin" && -x "/home/ubuntu/.local/bin/cargo" ]]; then
  cargo_bin="/home/ubuntu/.local/bin/cargo"
elif [[ -z "$cargo_bin" ]]; then
  cargo_bin="$(command -v cargo || true)"
fi
if [[ -z "$cargo_bin" ]]; then
  log "missing cargo on PATH after sourcing ~/.bashrc"
  exit 127
fi

cargo_genome=(
  "$cargo_bin" run -p openqg-bench --
  zyal genome
)

quote_command() {
  local quoted
  printf -v quoted '%q ' "$@"
  printf '%s' "${quoted% }"
}

active_run_pid() {
  local run_id="$1"
  local pid
  local args
  while IFS= read -r pid; do
    args="$(ps -p "$pid" -o args= 2>/dev/null || true)"
    if [[ "$pid" != "$$" \
      && "$args" == *"openqg-bench"* \
      && "$args" == *"zyal genome run"* \
      && "$args" == *"--run-id ${run_id}"* \
      && "$args" != *"pgrep -f"* \
      && "$args" != *"rtk pgrep"* ]]; then
      printf '%s\n' "$pid"
      return 0
    fi
  done < <(pgrep -f "openqg-bench zyal genome run.*--run-id ${run_id}" || true)
  return 1
}

archive_run_dir() {
  local run_id="$1"
  local run_dir="${run_root}/${run_id}"
  if [[ ! -e "$run_dir" ]]; then
    return
  fi
  if [[ -n "${DRY_RUN:-}" ]]; then
    log "would archive stale ${run_id} -> ${archive_dir}/${run_id}-<timestamp>"
    return
  fi
  local pid
  pid="$(active_run_pid "$run_id" || true)"
  if [[ -n "$pid" ]]; then
    local args
    args="$(ps -p "$pid" -o args= 2>/dev/null || true)"
    log "stale active run dir refuses launch: run_id=${run_id} pid=${pid} args=${args}"
    return 1
  fi
  local stamp
  stamp="$(date -u '+%Y%m%dT%H%M%SZ')"
  local dest="${archive_dir}/${run_id}-${stamp}"
  local suffix=1
  while [[ -e "$dest" ]]; do
    dest="${archive_dir}/${run_id}-${stamp}-${suffix}"
    suffix=$((suffix + 1))
  done
  log "archiving stale ${run_id} -> ${dest}"
  mv "$run_dir" "$dest"
}

run_step() {
  local label="$1"
  shift
  log "START ${label}: $(quote_command "$@")"
  if [[ -n "${DRY_RUN:-}" ]]; then
    log "DRY_RUN ${label}: skipped"
    return
  fi
  local start
  local status
  local elapsed
  start="$(date +%s)"
  set +e
  "$@"
  status="$?"
  set -e
  elapsed="$(($(date +%s) - start))"
  log "END ${label}: status=${status} elapsed=${elapsed}s"
  return "$status"
}

require_no_deleted_jailgun_executable() {
  if [[ -n "${DRY_RUN:-}" ]]; then
    log "DRY_RUN active Jailgun executable guard: skipped"
    return 0
  fi
  local pid
  local args
  local exe
  local found=0
  local offenders=()
  while IFS= read -r pid; do
    [[ -n "$pid" && -r "/proc/${pid}/cmdline" ]] || continue
    args="$(tr '\0' ' ' <"/proc/${pid}/cmdline" 2>/dev/null || true)"
    [[ "$args" == *"jailgun"* && "$args" == *"serve"* ]] || continue
    exe="$(readlink "/proc/${pid}/exe" 2>/dev/null || true)"
    [[ -n "$exe" ]] || continue
    found=1
    if [[ "$exe" == *" (deleted)" ]]; then
      offenders+=("pid=${pid} exe=${exe} args=${args}")
      continue
    fi
    log "active Jailgun executable ok: pid=${pid} exe=${exe}"
  done < <(pgrep -f 'jailgun.*serve|serve.*jailgun' || true)
  if [[ "${#offenders[@]}" -gt 0 ]]; then
    log "active Jailgun executable guard failed; rebuild and restart Jailgun before spending provider capacity"
    printf '%s\n' "${offenders[@]}" | while IFS= read -r line; do log "deleted Jailgun executable: ${line}"; done
    return 1
  fi
  if [[ "$found" -eq 0 ]]; then
    log "active Jailgun executable guard: no running jailgun serve process found"
  fi
  return 0
}

require_no_deleted_openqg_target_fds() {
  if [[ -n "${DRY_RUN:-}" ]]; then
    log "DRY_RUN deleted OpenQG target FD guard: skipped"
    return 0
  fi
  local proc
  local pid
  local args
  local fd
  local target
  local offenders=()
  for proc in /proc/[0-9]*; do
    [[ -d "$proc" ]] || continue
    pid="${proc##*/}"
    [[ "$pid" != "$$" && -r "${proc}/cmdline" ]] || continue
    args="$(tr '\0' ' ' <"${proc}/cmdline" 2>/dev/null || true)"
    [[ "$args" == *"openQG"* || "$args" == *"openqg"* || "$args" == *"openqg-bench"* || "$args" == *"$repo_root"* ]] || continue
    for fd in "${proc}"/fd/*; do
      target="$(readlink "$fd" 2>/dev/null || true)"
      if [[ "$target" == *"target/openqg/zyal-genome"* && "$target" == *" (deleted)" ]]; then
        offenders+=("pid=${pid} fd=${fd##*/} target=${target} args=${args}")
        break
      fi
    done
  done
  if [[ "${#offenders[@]}" -gt 0 ]]; then
    log "deleted OpenQG target FD guard failed; refusing launch"
    printf '%s\n' "${offenders[@]}" | while IFS= read -r line; do log "deleted OpenQG fd: ${line}"; done
    return 1
  fi
  log "deleted OpenQG target FD guard passed"
}

require_no_competing_live_workloads() {
  if [[ -n "${DRY_RUN:-}" ]]; then
    log "DRY_RUN competing live workload guard: skipped"
    return 0
  fi
  local pid
  local args
  local offenders=()
  while IFS= read -r pid; do
    [[ -n "$pid" && "$pid" != "$$" ]] || continue
    args="$(ps -p "$pid" -o args= 2>/dev/null || true)"
    [[ -n "$args" ]] || continue
    [[ "$args" != *"pgrep -f"* && "$args" != *"rtk pgrep"* ]] || continue
    if [[ "$args" == *"/home/ubuntu/lastcommit/scripts/run-zyal-refinement-campaign.sh"* \
      || "$args" == *"jekko run"* \
      || ( "$args" == *"openqg-bench"* && "$args" == *"zyal genome run"* ) ]]; then
      offenders+=("pid=${pid} args=${args}")
    fi
  done < <(pgrep -f 'run-zyal-refinement-campaign\.sh|jekko run|openqg-bench.*zyal genome run' || true)
  if [[ "${#offenders[@]}" -gt 0 ]]; then
    log "competing live workload guard failed; refusing launch without killing outside processes"
    printf '%s\n' "${offenders[@]}" | while IFS= read -r line; do log "competing workload: ${line}"; done
    return 1
  fi
  log "competing live workload guard passed"
}

quality_gate_passed() {
  local gate="$1"
  [[ -s "$gate" ]] || return 1
  if command -v jq >/dev/null 2>&1; then
    jq -e '.passed == true' "$gate" >/dev/null 2>&1
  else
    grep -Eq '"passed"[[:space:]]*:[[:space:]]*true' "$gate"
  fi
}

require_quality_gate() {
  local run_id="$1"
  local gate="${run_root}/${run_id}/quality-gate.json"
  if [[ -n "${DRY_RUN:-}" ]]; then
    log "DRY_RUN required gate ${run_id}: would require ${gate}"
    return 0
  fi
  if quality_gate_passed "$gate"; then
    log "required gate passed: ${run_id} (${gate})"
    return 0
  fi
  log "required gate missing or failed: ${run_id} (${gate})"
  return 1
}

log_jailgun_summary() {
  local run_id="$1"
  local preflight="${run_root}/${run_id}/preflight.json"
  if [[ -z "${DRY_RUN:-}" && -s "$preflight" ]] && command -v jq >/dev/null 2>&1; then
    jq -r '
      .backend_health.jailgun as $j
      | "jailgun summary '"${run_id}"': available=\(.backend_health.jailgun_available // $j.available // "-") token_source=\($j.token_source // "-") account_source=\($j.account_source // "-") ready_accounts=\(($j.ready_account_ids // []) | length) account_count=\(($j.account_ids // []) | length)"
    ' "$preflight" 2>/dev/null | while IFS= read -r line; do log "$line"; done
  fi
}

log_live_counts() {
  local run_id="$1"
  local ledger="${run_root}/${run_id}/live-call-ledger.jsonl"
  if [[ -n "${DRY_RUN:-}" ]]; then
    log "DRY_RUN live ledger ${run_id}: skipped"
    return
  fi
  if [[ ! -f "$ledger" ]]; then
    log "live ledger ${run_id}: missing (${ledger})"
    return
  fi
  if command -v jq >/dev/null 2>&1; then
    jq -s -r '
      "live ledger '"${run_id}"': total=\(length) ok=\([.[] | select(.status == "ok")] | length) timeout=\([.[] | select(.status == "timeout")] | length) failed=\([.[] | select((.status // "failed") != "ok")] | length)"
    ' "$ledger" 2>/dev/null | while IFS= read -r line; do log "$line"; done
  else
    log "live ledger ${run_id}: total=$(wc -l <"$ledger" | tr -d ' ')"
  fi
}

log_failed_quality_checks() {
  local run_id="$1"
  local gate="${run_root}/${run_id}/quality-gate.json"
  if [[ -n "${DRY_RUN:-}" ]]; then
    log "DRY_RUN quality checks ${run_id}: skipped"
    return
  fi
  if [[ ! -s "$gate" ]] || ! command -v jq >/dev/null 2>&1; then
    return
  fi
  local failed
  failed="$(jq -r '.checks[]? | select((.passed // false) != true) | "\(.name): observed=\(.observed|tostring) threshold=\(.threshold|tostring)"' "$gate" 2>/dev/null || true)"
  if [[ -z "$failed" ]]; then
    log "quality checks ${run_id}: all passed"
  else
    while IFS= read -r line; do
      log "quality check failed ${run_id}: ${line}"
    done <<<"$failed"
  fi
}

run_monitor_snapshot() {
  local run_id="$1"
  local label="$2"
  local snapshot="${pipeline_dir}/monitor-${run_id}.log"
  if [[ -n "${DRY_RUN:-}" ]]; then
    log "DRY_RUN monitor snapshot ${run_id}/${label}: skipped"
    return
  fi
  {
    printf '\n[%s] snapshot=%s run_id=%s\n' "$(timestamp)" "$label" "$run_id"
    NO_COLOR=1 NO_CLEAR=1 ONCE=1 LINES=6 bash tools/zyal-genome-live-log.sh "$run_id" 1
  } >>"$snapshot" 2>&1 || true
  log "monitor snapshot ${run_id}/${label}: ${snapshot}"
}

run_stage() {
  local run_id="$1"
  local generations="$2"

  run_step "${run_id} preflight" "${cargo_genome[@]}" preflight \
    --runbook ZYAL/runs/run-hybrid-1000-v2.zyal \
    --run-id "$run_id" \
    --require-hard-backend \
    --live-smoke \
    --jailgun-artifact-smoke \
    --jailgun-artifact-smoke-extension json \
    --live-selective || return "$?"
  log_jailgun_summary "$run_id"
  run_step "${run_id} run" "${cargo_genome[@]}" run \
    --variant hybrid \
    --runbook ZYAL/runs/run-hybrid-1000-v2.zyal \
    --generations "$generations" \
    --run-id "$run_id" \
    --checkpoint-every 1 \
    --resume \
    --live-selective || {
      log_live_counts "$run_id"
      run_monitor_snapshot "$run_id" "run-failed"
      return 1
    }
  log_live_counts "$run_id"
  run_step "${run_id} validate" "${cargo_genome[@]}" validate \
    --root "target/openqg/zyal-genome/hybrid/runs/${run_id}" || return "$?"
  run_step "${run_id} plot-index" "${cargo_genome[@]}" plot-index \
    --run-dir "target/openqg/zyal-genome/hybrid/runs/${run_id}" || return "$?"
  run_step "${run_id} quality-gate" "${cargo_genome[@]}" quality-gate \
    --run-dir "target/openqg/zyal-genome/hybrid/runs/${run_id}" || {
      log_failed_quality_checks "$run_id"
      run_monitor_snapshot "$run_id" "quality-gate-failed"
      return 1
    }
  log_failed_quality_checks "$run_id"
  run_monitor_snapshot "$run_id" "stage-complete"
}

checkpoint_complete() {
  local run_id="$1"
  local target="$2"
  local checkpoint="${run_root}/${run_id}/checkpoint.json"
  [[ -s "$checkpoint" ]] || return 1
  if command -v jq >/dev/null 2>&1; then
    local complete
    complete="$(jq -r '.complete_generation // 0' "$checkpoint" 2>/dev/null || printf '0')"
    [[ "$complete" =~ ^[0-9]+$ && "$complete" -ge "$target" ]]
  else
    grep -Eq '"complete_generation"[[:space:]]*:[[:space:]]*'"$target" "$checkpoint"
  fi
}

log_checkpoint_progress() {
  local run_id="$1"
  local checkpoint="${run_root}/${run_id}/checkpoint.json"
  if [[ -s "$checkpoint" ]] && command -v jq >/dev/null 2>&1; then
    jq -r '"checkpoint '"${run_id}"': complete=\(.complete_generation // 0)/\(.target_generation // "?") id=\(.complete_generation_id // "-") updated=\(.updated_at // "-")"' "$checkpoint" 2>/dev/null |
      while IFS= read -r line; do log "$line"; done
  else
    log "checkpoint ${run_id}: waiting for ${checkpoint}"
  fi
}

wait_for_completion() {
  local run_id="$1"
  local target="$2"
  if [[ -n "${DRY_RUN:-}" ]]; then
    log "DRY_RUN wait ${run_id}: skipped"
    return 0
  fi
  while true; do
    log_checkpoint_progress "$run_id"
    log_live_counts "$run_id"
    run_monitor_snapshot "$run_id" "poll"
    if checkpoint_complete "$run_id" "$target"; then
      log "${run_id} reached generation ${target}"
      return 0
    fi
    if [[ -z "$(active_run_pid "$run_id" || true)" ]]; then
      log "${run_id} is not complete and no active run process was found"
      return 1
    fi
    sleep "$poll_seconds"
  done
}

run_final_checks() {
  local run_id="$1"
  local run_dir="${run_root}/${run_id}"
  run_step "${run_id} final validate" "${cargo_genome[@]}" validate \
    --root "$run_dir" || return "$?"
  run_step "${run_id} final run plot-index" "${cargo_genome[@]}" plot-index \
    --run-dir "$run_dir" || return "$?"
  run_step "${run_id} final root plot-index" "${cargo_genome[@]}" plot-index \
    --root "$genome_root" || return "$?"
  run_step "${run_id} final quality-gate" "${cargo_genome[@]}" quality-gate \
    --run-dir "$run_dir" || {
      log_failed_quality_checks "$run_id"
      run_monitor_snapshot "$run_id" "final-quality-gate-failed"
      return 1
    }
  log_failed_quality_checks "$run_id"
  run_monitor_snapshot "$run_id" "final-complete"
}

log "starting hybrid v2 pipeline from ${repo_root}"
log "pipeline log: ${log_file}"
log "tail with: tail -F ${log_file}"
log "poll interval: ${poll_seconds}s"

require_no_deleted_jailgun_executable
require_no_deleted_openqg_target_fds
require_no_competing_live_workloads

archive_run_dir "hybrid-v2-10"
archive_run_dir "hybrid-v2-20"
archive_run_dir "hybrid-v2-50"
archive_run_dir "hybrid-v2-100"

run_stage "hybrid-v2-10" 10
require_quality_gate "hybrid-v2-10"

run_stage "hybrid-v2-20" 20
require_quality_gate "hybrid-v2-20"

run_stage "hybrid-v2-50" 50
require_quality_gate "hybrid-v2-50"

log "qualification gates passed; running hybrid-v2-100"
run_stage "hybrid-v2-100" 100
run_final_checks "hybrid-v2-100"
log "hybrid v2 pipeline complete"
