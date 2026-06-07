#!/usr/bin/env bash
set -euo pipefail

HOME="${HOME:-/home/ubuntu}"

if [[ -f "${HOME}/.bashrc" ]]; then
  # Match the TUI environment so local tool shims and PATH updates are present.
  # Suppress startup noise; the monitor output below stays stable and readable.
  # shellcheck disable=SC1090
  source "${HOME}/.bashrc" >/dev/null 2>&1 || true
fi

RUN_ID="${1:-${RUN_ID:-hybrid-1000}}"
INTERVAL="${2:-${INTERVAL:-5}}"
LINES="${LINES:-8}"
RUN_DIR="${RUN_DIR:-target/openqg/zyal-genome/hybrid/runs/${RUN_ID}}"
ROOT_DIR="${ROOT_DIR:-target/openqg/zyal-genome}"

if [[ -t 1 && -z "${NO_COLOR:-}" ]]; then
  BOLD="$(tput bold 2>/dev/null || true)"
  RESET="$(tput sgr0 2>/dev/null || true)"
  DIM="$(tput dim 2>/dev/null || true)"
  RED="$(tput setaf 1 2>/dev/null || true)"
  GREEN="$(tput setaf 2 2>/dev/null || true)"
  YELLOW="$(tput setaf 3 2>/dev/null || true)"
  BLUE="$(tput setaf 4 2>/dev/null || true)"
  MAGENTA="$(tput setaf 5 2>/dev/null || true)"
  CYAN="$(tput setaf 6 2>/dev/null || true)"
else
  BOLD=""; RESET=""; DIM=""; RED=""; GREEN=""; YELLOW=""; BLUE=""; MAGENTA=""; CYAN=""
fi

has_jq() {
  command -v jq >/dev/null 2>&1
}

json_value() {
  local file="$1"
  local filter="$2"
  local fallback="$3"
  if [[ -s "$file" ]] && has_jq; then
    jq -r "${filter} // \"${fallback}\"" "$file" 2>/dev/null || printf '%s' "$fallback"
  else
    printf '%s' "$fallback"
  fi
}

line_count() {
  local file="$1"
  if [[ -f "$file" ]]; then
    wc -l <"$file" | tr -d ' '
  else
    printf '0'
  fi
}

status_count() {
  local file="$1"
  local status="$2"
  if [[ -f "$file" ]]; then
    if has_jq; then
      jq -r --arg status "$status" 'select(.status == $status) | 1' "$file" 2>/dev/null | wc -l | tr -d ' '
    else
      grep -c "\"status\":\"${status}\"" "$file" || true
    fi
  else
    printf '0'
  fi
}

jq_filter() {
  local file="$1"
  local filter="$2"
  local fallback="$3"
  if [[ -s "$file" ]] && has_jq; then
    jq -r "$filter // \"${fallback}\"" "$file" 2>/dev/null || printf '%s' "$fallback"
  else
    printf '%s' "$fallback"
  fi
}

jq_json_compact() {
  local file="$1"
  local filter="$2"
  local fallback="$3"
  if [[ -s "$file" ]] && has_jq; then
    jq -c "$filter // ${fallback}" "$file" 2>/dev/null || printf '%s' "$fallback"
  else
    printf '%s' "$fallback"
  fi
}

jq_count_filter() {
  local file="$1"
  local filter="$2"
  if [[ -f "$file" ]] && has_jq; then
    jq -r "$filter | 1" "$file" 2>/dev/null | wc -l | tr -d ' '
  else
    printf '0'
  fi
}

rate_or_zero() {
  local numerator="$1"
  local denominator="$2"
  awk -v n="$numerator" -v d="$denominator" 'BEGIN { if (d > 0) printf "%.4f", n/d; else printf "0.0000" }'
}

artifact_status() {
  local label="$1"
  local path="$2"
  if [[ -s "$path" ]]; then
    printf '  %bOK%b   %-24s %s\n' "$GREEN" "$RESET" "$label" "$path"
  elif [[ -e "$path" ]]; then
    printf '  %bWAIT%b %-24s %s\n' "$YELLOW" "$RESET" "$label" "$path"
  else
    printf '  %bMISS%b %-24s %s\n' "$DIM" "$RESET" "$label" "$path"
  fi
}

progress_bar() {
  local done_count="$1"
  local target_count="$2"
  local width=36
  local filled=0
  local pct="0.0"
  if [[ "$target_count" =~ ^[0-9]+$ && "$target_count" -gt 0 && "$done_count" =~ ^[0-9]+$ ]]; then
    filled="$(awk -v d="$done_count" -v t="$target_count" -v w="$width" 'BEGIN { v=int(w*d/t); if (v>w) v=w; print v }')"
    pct="$(awk -v d="$done_count" -v t="$target_count" 'BEGIN { printf "%.1f", 100*d/t }')"
  fi
  local empty=$((width - filled))
  printf '%b[%s%s]%b %s%%' \
    "$GREEN" \
    "$(printf '%*s' "$filled" '' | tr ' ' '#')" \
    "$(printf '%*s' "$empty" '' | tr ' ' '-')" \
    "$RESET" \
    "$pct"
}

color_for_line() {
  local line="$1"
  if [[ "$line" == *'"status":"ok"'* || "$line" == *'"event_type":"generation_end"'* ]]; then
    printf '%s' "$GREEN"
  elif [[ "$line" == *'"timeout"'* || "$line" == *'"degraded_router"'* ]]; then
    printf '%s' "$YELLOW"
  elif [[ "$line" == *'"failed"'* || "$line" == *'"error"'* ]]; then
    printf '%s' "$RED"
  else
    printf '%s' "$CYAN"
  fi
}

summarize_live_call() {
  local line="$1"
  if has_jq; then
    jq -r '
      (.summary // "" | tostring | gsub("[\r\n\t]+"; " ") | .[0:140]) as $summary
      | "\(.generation_id // "-")  \(.stage_id // "-")  \(.purpose // "-")  exec=\(.execution_backend // .route_backend // "-")  jailgun=\(.jailgun_run_id // "-")  status=\(.status // "-")  elapsed=\(.elapsed_seconds // "-")s  \($summary)"
    ' <<<"$line" 2>/dev/null || printf '%s' "$line"
  else
    printf '%s' "$line"
  fi
}

summarize_metric() {
  local line="$1"
  if has_jq; then
    jq -r '"\(.generation_id // "-")  \(.series // "-")  \(.metric_name // "-")=\(.metric_value // "-")  stages=\(.stage_passes // "-")/\(.stage_count // "-")  best=\(.best_stage_score // "-")"' <<<"$line" 2>/dev/null || printf '%s' "$line"
  else
    printf '%s' "$line"
  fi
}

summarize_event() {
  local line="$1"
  if has_jq; then
    jq -r '"\(.generation_id // "-")  \(.event_type // "-")  stage=\(.stage_id // "-")  score=\(.final_score // "-")  router=\(.router_state // "-")"' <<<"$line" 2>/dev/null || printf '%s' "$line"
  else
    printf '%s' "$line"
  fi
}

print_tail_section() {
  local title="$1"
  local file="$2"
  local formatter="$3"
  printf '\n%b%s%b\n' "$BOLD$BLUE" "$title" "$RESET"
  if [[ ! -f "$file" ]]; then
    printf '  %bwaiting for %s%b\n' "$DIM" "$file" "$RESET"
    return
  fi
  tail -n "$LINES" "$file" | while IFS= read -r line; do
    local color
    local formatted
    color="$(color_for_line "$line")"
    formatted="$("$formatter" "$line")"
    printf '  %b%s%b\n' "$color" "$formatted" "$RESET"
  done
}

print_text_tail_section() {
  local title="$1"
  local file="$2"
  printf '\n%b%s%b\n' "$BOLD$BLUE" "$title" "$RESET"
  if [[ ! -f "$file" ]]; then
    printf '  %bwaiting for %s%b\n' "$DIM" "$file" "$RESET"
    return
  fi
  tail -n "$LINES" "$file" | while IFS= read -r line; do
    printf '  %b%s%b\n' "$CYAN" "$line" "$RESET"
  done
}

print_failed_quality_checks() {
  local file="$1"
  if [[ ! -s "$file" ]] || ! has_jq; then
    return
  fi
  local failed
  failed="$(jq -r '.checks[]? | select((.passed // false) != true) | "  FAIL \(.name): observed=\(.observed|tostring) threshold=\(.threshold|tostring)"' "$file" 2>/dev/null || true)"
  if [[ -n "$failed" ]]; then
    printf '\n%bFailed quality checks%b\n' "$BOLD$RED" "$RESET"
    printf '%s\n' "$failed"
  fi
}

while true; do
  if [[ -z "${NO_CLEAR:-}" && -z "${ONCE:-}" ]]; then
    clear || true
  fi

  checkpoint="${RUN_DIR}/checkpoint.json"
  generation_ledger="${RUN_DIR}/generation-ledger.jsonl"
  stage_ledger="${RUN_DIR}/stage-ledger.jsonl"
  metrics_ledger="${RUN_DIR}/metrics-timeseries.jsonl"
  live_ledger="${RUN_DIR}/live-call-ledger.jsonl"
  run_events="${RUN_DIR}/run-events.jsonl"
  mcp_log="${MCP_LOG:-.jekko/run-logs/zyal-genome-hybrid-v2/${RUN_ID}-mcp-launch.log}"
  plot_index="${RUN_DIR}/plot-index.json"
  quality_gate="${RUN_DIR}/quality-gate.json"
  summary="${RUN_DIR}/run-summary.json"

  complete_generation="$(json_value "$checkpoint" '.complete_generation' '0')"
  target_generation="$(json_value "$checkpoint" '.target_generation' '1000')"
  complete_generation_id="$(json_value "$checkpoint" '.complete_generation_id' '-')"
  updated_at="$(json_value "$checkpoint" '.updated_at' '-')"
  live_calls="$(line_count "$live_ledger")"
  live_timeouts="$(status_count "$live_ledger" timeout)"
  live_failures="$(status_count "$live_ledger" failed)"
  live_timeout_rate="$(rate_or_zero "$live_timeouts" "$live_calls")"
  quality_status="$(jq_filter "$quality_gate" '.status' 'pending')"
  degraded_routes="$(jq_filter "$quality_gate" '.metrics.degraded_route_count' "$(jq_filter "$summary" '.degraded_route_count' "$(jq_count_filter "$stage_ledger" 'select(.router_state == "degraded_router")')")")"
  regression_rate="$(jq_filter "$quality_gate" '.metrics.regression_rate' "$(jq_filter "$summary" '.regression_rate' '-')")"
  perfect_rate="$(jq_filter "$quality_gate" '.metrics.champion_perfect_rate' '-')"
  island_counts="$(jq_json_compact "$quality_gate" '.metrics.island_champion_counts' '{}')"
  mode_counts="$(jq_json_compact "$quality_gate" '.metrics.mode_champion_counts' '{}')"

  printf '%bZYAL Genome Live Monitor%b  %brun=%s%b  %s\n' "$BOLD$MAGENTA" "$RESET" "$BOLD" "$RUN_ID" "$RESET" "$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
  printf '%bRun dir:%b %s\n' "$DIM" "$RESET" "$RUN_DIR"
  printf '%bProgress:%b %s/%s (%s)  ' "$DIM" "$RESET" "$complete_generation" "$target_generation" "$complete_generation_id"
  progress_bar "$complete_generation" "$target_generation"
  printf '  %bupdated=%s%b\n' "$DIM" "$updated_at" "$RESET"

  printf '\n%bLedger counts%b  generations=%s  metrics=%s  events=%s  live_calls=%s  ok=%b%s%b  timeout=%b%s%b  failed=%b%s%b\n' \
    "$BOLD$BLUE" "$RESET" \
    "$(line_count "$generation_ledger")" \
    "$(line_count "$metrics_ledger")" \
    "$(line_count "$run_events")" \
    "$live_calls" \
    "$GREEN" "$(status_count "$live_ledger" ok)" "$RESET" \
    "$YELLOW" "$live_timeouts" "$RESET" \
    "$RED" "$live_failures" "$RESET"

  printf '%bQuality%b  gate=%b%s%b  timeout_rate=%s  degraded_routes=%s  regression=%s  perfect_champions=%s\n' \
    "$BOLD$BLUE" "$RESET" \
    "$BOLD" "$quality_status" "$RESET" \
    "$live_timeout_rate" \
    "$degraded_routes" \
    "$regression_rate" \
    "$perfect_rate"
  printf '%bChampion distribution%b  islands=%s  modes=%s\n' "$DIM" "$RESET" "$island_counts" "$mode_counts"

  if [[ "$RUN_ID" == "hybrid-v2-100" ]]; then
    upstream_gate="${ROOT_DIR}/hybrid/runs/hybrid-v2-50/quality-gate.json"
    upstream_passed="$(jq_filter "$upstream_gate" '.passed' 'false')"
    if [[ "$upstream_passed" != "true" ]]; then
      printf '\n%bBlocked before launch%b  hybrid-v2-50 quality gate has not passed (%s)\n' "$BOLD$RED" "$RESET" "$upstream_gate"
    fi
  fi

  print_failed_quality_checks "$quality_gate"

  printf '\n%bOutput artifacts%b\n' "$BOLD$BLUE" "$RESET"
  artifact_status checkpoint "$checkpoint"
  artifact_status generation-ledger "$generation_ledger"
  artifact_status metrics-timeseries "$metrics_ledger"
  artifact_status live-call-ledger "$live_ledger"
  artifact_status run-events "$run_events"
  artifact_status mcp-launch-log "$mcp_log"
  artifact_status plot-index "$plot_index"
  artifact_status quality-gate "$quality_gate"
  artifact_status root-plot-index "${ROOT_DIR}/plot-index.json"

  print_text_tail_section "MCP launch log" "$mcp_log"
  print_tail_section "Recent live calls" "$live_ledger" summarize_live_call
  print_tail_section "Recent generation rollups" "$generation_ledger" summarize_metric
  print_tail_section "Recent run events" "$run_events" summarize_event

  if [[ -n "${ONCE:-}" ]]; then
    break
  fi
  sleep "$INTERVAL"
done
