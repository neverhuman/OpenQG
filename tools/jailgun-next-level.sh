#!/usr/bin/env bash
# jailgun-next-level.sh — the 12-spec "next-level" engineering-review event (S01–S12).
#
# Each spec is one single-tab `jailhard --download-only` run: it tars EXACTLY the files in
# ops/jailgun/next-level-manifest.txt (via --include-manifest), uploads them to one ChatGPT
# account/tab with the spec's prompt, and downloads the returned `.tar.gz` (which the prompt
# instructs to contain a single spec-SXX-<slug>.md). We extract that .md into tips/jailgun/next-level/.
#
# Differences from jailgun-physics-review.sh (the prior L1–L6 event):
#   * results preview is STAGED out of target/ (jailhard hard-blocks target/ paths) — see stage_results
#   * harvest QUALITY GATE: size floor + "archive unavailable" degradation signature → retry,
#     because 3/12 prior harvests were degraded stubs that passed the old non-empty check
#   * MAX_ATTEMPTS=3 (12 distinct irreplaceable specs), single pass (no waves)
#   * --only Sxx,Syy straggler re-runs; --force-account A|B reroutes around a sick account
#   * preflight mirrors the jailhard denylist + enforces the 1.5 MB payload budget
#   * final INDEX.md generation with per-spec status
#
# Modes:
#   --smoke N            run only the first N specs, all on account A, sequentially
#   --only S03,S07       re-run just those specs (home account unless --force-account)
#   --force-account A|B  with --only: route to that account instead of the home one
#   --build              rebuild the jailhard binary first
#   --out DIR            output dir for the specs (default: tips/jailgun/next-level)
# Default (no mode flag): full event — S01-S06 on account A, S07-S12 on account B, concurrent.

set -uo pipefail

REPO="/home/ubuntu/openQG"
JAILGUN="/home/ubuntu/jailgun"
JAILHARD="$JAILGUN/target/debug/jailhard"
BRIDGE_MJS="$JAILGUN/apps/chrome-bridge/bin/chrome-bridge.mjs"
CONFIG_BASE="$JAILGUN/config/jailgun.example.toml"
MANIFEST="ops/jailgun/next-level-manifest.txt"
ARTBASE="/tmp/openqg-jailgun-next-level"
PROMPT_DIR="ops/jailgun/prompts/next-level"
RESULTS_STAGE="ops/jailgun/next-level-payload/results"
ACCT_A="acct-19ae9aed"   # jepson@veox.ai
ACCT_B="acct-4690d657"   # bentaylorche@gmail.com
PER_RUN_TIMEOUT="${JAILGUN_NL_TIMEOUT:-1500}"   # seconds per spec attempt
MAX_ATTEMPTS=3
MIN_BYTES="${JAILGUN_NL_MIN_BYTES:-8192}"
BUDGET_BYTES=1500000
DEGRADED_RE='source archive (unavailable|not available)|archive (was |is )?not (present|available)|not present in the sandbox|could not (be )?extract'

# Spec order = risk order: theme-bearing specs first. Index 0-5 → account A, 6-11 → account B.
SLUGS=(
  S01-derivation-sandbox
  S02-dual-path-hybrid-search
  S03-term-algebra
  S04-gap-directed-decomposition
  S05-data-acquisition
  S06-kpi-anticheating
  S07-statistical-evidence
  S08-boltzmann-emulator
  S09-knowledge-hardening
  S10-field-state-tensions
  S11-paper-profundity
  S12-token-compute-ledger
)

# Staged results: "<staged-name>:<source-path>" (sources live under target/, which jailhard refuses).
STAGE_PAIRS=(
  "v6-replay-run-summary.json:target/openqg/zyal-genome/runs/v6-acceptance-replay/run-summary.json"
  "v6-replay-champion.json:target/openqg/zyal-genome/runs/v6-acceptance-replay/champion.json"
  "v6-replay-quality-gate.json:target/openqg/zyal-genome/runs/v6-acceptance-replay/quality-gate.json"
  "prod-1000-run-summary.json:target/openqg/theory-evolve/runs/prod-1000/run-summary.json"
  "prod-1000-champion.json:target/openqg/theory-evolve/runs/prod-1000/champion.json"
  "prod-1000-quality-gate.json:target/openqg/theory-evolve/runs/prod-1000/quality-gate.json"
  "v7-smoke-proposal-ledger.jsonl:target/openqg/zyal-genome/runs/v7-live-smoke/proposal-ledger.jsonl"
  "v7-smoke-progress-ledger.jsonl:target/openqg/zyal-genome/runs/v7-live-smoke/progress-ledger.jsonl"
  "v7-tg-trust-gate.json:target/openqg/zyal-genome/runs/v7-tg/trust-gate.json"
)

OUT="tips/jailgun/next-level"
SMOKE=0
ONLY=""
FORCE_ACCT=""
DO_BUILD=0
while [ $# -gt 0 ]; do
  case "$1" in
    --smoke) SMOKE="$2"; shift 2;;
    --only) ONLY="$2"; shift 2;;
    --force-account) FORCE_ACCT="$2"; shift 2;;
    --out) OUT="$2"; shift 2;;
    --build) DO_BUILD=1; shift;;
    *) echo "unknown arg: $1" >&2; exit 2;;
  esac
done
if [ -n "$FORCE_ACCT" ] && [ -z "$ONLY" ]; then
  echo "--force-account requires --only" >&2; exit 2
fi

cd "$REPO" || { echo "cannot cd $REPO" >&2; exit 1; }
mkdir -p "$OUT" "$ARTBASE"

if [ "$DO_BUILD" = 1 ] || [ ! -x "$JAILHARD" ]; then
  echo "[build] cargo build -p jailgun-cli --bin jailhard"
  ( cd "$JAILGUN" && cargo build -p jailgun-cli --bin jailhard ) || { echo "build failed" >&2; exit 1; }
fi

# --- Step 0: stage results preview out of target/ (jailhard refuses target/ paths) ---
stage_results() {
  mkdir -p "$RESULTS_STAGE"
  local pair staged src
  for pair in "${STAGE_PAIRS[@]}"; do
    staged="${pair%%:*}"; src="${pair#*:}"
    if [ ! -f "$src" ]; then
      echo "[stage] MISSING source: $src — regenerate the run or trim the manifest" >&2
      return 1
    fi
    cp -f "$src" "$RESULTS_STAGE/$staged" || return 1
  done
  echo "[stage] ${#STAGE_PAIRS[@]} results staged into $RESULTS_STAGE"
}
stage_results || { echo "[stage] staging failed — aborting" >&2; exit 1; }

# --- Preflight ---
miss=0; total_bytes=0
while IFS= read -r line; do
  case "$line" in ''|\#*) continue;; esac
  # Mirror the jailhard denylist so we fail here, not mid-run.
  if echo "$line" | grep -qE '(^|/)(target|artifacts|logs|tmp|build|out|downloads|node_modules|dist|vendor)(/|$)'; then
    echo "[preflight] DENYLISTED manifest path: $line" >&2; miss=$((miss+1)); continue
  fi
  if basename "$line" | grep -qiE 'token|secret'; then
    echo "[preflight] SECRET-LIKE filename: $line" >&2; miss=$((miss+1)); continue
  fi
  if [ ! -f "$line" ]; then
    echo "[preflight] MISSING manifest file: $line" >&2; miss=$((miss+1)); continue
  fi
  total_bytes=$((total_bytes + $(wc -c <"$line")))
done < "$MANIFEST"
[ "$miss" = 0 ] || { echo "[preflight] $miss manifest problems — aborting" >&2; exit 1; }
if [ "$total_bytes" -gt "$BUDGET_BYTES" ]; then
  echo "[preflight] payload $total_bytes bytes exceeds budget $BUDGET_BYTES — trim the manifest" >&2; exit 1
fi
for slug in "${SLUGS[@]}"; do
  [ -f "$PROMPT_DIR/$slug.md" ] || { echo "[preflight] MISSING prompt: $PROMPT_DIR/$slug.md" >&2; exit 1; }
done
echo "[preflight] manifest ok ($(grep -cvE '^\s*(#|$)' "$MANIFEST") files, $total_bytes bytes); 12 prompts ok"
for port in 9224 9225; do
  if ! curl -s --max-time 3 "http://127.0.0.1:$port/json/version" >/dev/null 2>&1; then
    echo "[preflight] note: no live CDP on :$port — bridge will managed-relaunch Chrome from ~/.jailgun profiles; if the account is auth-required the run fails (check ~/.jailgun/browser-profiles.json)"
  fi
done

# run_spec <slug> <account>  → writes $OUT/spec-<slug>.md or warns. Quality-gated harvest.
run_spec() {
  local slug="$1" account="$2"
  local prompt="$PROMPT_DIR/$slug.md"
  local target="$OUT/spec-${slug}.md"
  local attempt
  for attempt in $(seq 1 "$MAX_ATTEMPTS"); do
    local rundir="$ARTBASE/${slug}-$$-${RANDOM}"
    mkdir -p "$rundir/extract"
    local cfg="$rundir/jailgun.toml"
    sed 's#^artifacts_dir = .*#artifacts_dir = "'"$rundir"'"#' "$CONFIG_BASE" > "$cfg"

    echo "[$slug] attempt $attempt on $account → $rundir"
    timeout "$PER_RUN_TIMEOUT" "$JAILHARD" \
      --include-manifest "$MANIFEST" \
      --download-only \
      --task-file "$prompt" \
      --account "$account" \
      --tabs 1 \
      --config "$cfg" \
      --bridge-cmd node "$BRIDGE_MJS" \
      --bridge-env JAILGUN_ARTIFACT_STALL_REPAIR_SECONDS=45 \
      --bridge-env JAILGUN_ARTIFACT_REPAIR_ATTEMPT_LIMIT=2 \
      >"$rundir/jailhard.log" 2>&1
    local rc=$?

    # Harvest: find the downloaded artifact (.tar.gz preferred; a bare .md is also accepted).
    local dl md=""
    dl=$(find "$rundir/downloads" -type f \( -name '*.tar.gz' -o -name '*.tgz' -o -name '*.md' \) 2>/dev/null | head -1)
    case "$dl" in
      *.tar.gz|*.tgz) tar -xzf "$dl" -C "$rundir/extract" 2>/dev/null
                      md=$(find "$rundir/extract" -type f -name '*.md' | head -1);;
      *.md) md="$dl";;
    esac

    if [ -n "$md" ] && [ -s "$md" ]; then
      local bytes; bytes=$(wc -c <"$md")
      if [ "$bytes" -lt "$MIN_BYTES" ]; then
        echo "[$slug] REJECTED attempt $attempt (only $bytes bytes < $MIN_BYTES) — kept at $rundir" >&2
        continue
      fi
      if head -15 "$md" | grep -qiE "$DEGRADED_RE"; then
        echo "[$slug] REJECTED attempt $attempt (degraded: archive-unavailable signature) — kept at $rundir" >&2
        continue
      fi
      cp "$md" "$target"
      echo "[$slug] OK → $target ($bytes bytes, rc=$rc)"
      return 0
    fi
    echo "[$slug] MISS (rc=$rc, dl='${dl:-none}') — see $rundir/jailhard.log" >&2
  done
  echo "[$slug] FAILED after $MAX_ATTEMPTS attempts" >&2
  return 1
}

# index_for <Sxx> → array index, or -1
index_for() {
  local want="$1" i
  for i in "${!SLUGS[@]}"; do
    case "${SLUGS[$i]}" in "$want"|"$want"-*) echo "$i"; return;; esac
  done
  echo -1
}

write_index() {
  local idx="$OUT/INDEX.md" ok=0
  {
    echo "# Next-level spec harvest — INDEX"
    echo
    echo "| spec | status | bytes | words | first heading |"
    echo "|------|--------|-------|-------|---------------|"
  } > "$idx"
  local slug f st bytes words head1
  for slug in "${SLUGS[@]}"; do
    f="$OUT/spec-${slug}.md"
    if [ ! -s "$f" ]; then
      echo "| $slug | MISSING | — | — | — |" >> "$idx"; continue
    fi
    bytes=$(wc -c <"$f"); words=$(wc -w <"$f")
    head1=$(grep -m1 '^#' "$f" 2>/dev/null | head -c 80 | tr '|' '-')
    [ -n "$head1" ] || head1=$(head -1 "$f" | head -c 80 | tr '|' '-')
    if grep -qiE "$DEGRADED_RE" "$f"; then st="FLAGGED"; else st="OK"; ok=$((ok+1)); fi
    echo "| $slug | $st | $bytes | $words | $head1 |" >> "$idx"
  done
  {
    echo
    echo "Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ) — payload $total_bytes bytes — $ok/12 OK"
  } >> "$idx"
  echo "[index] $idx written ($ok/12 OK)"
}

# --- Smoke mode: first N specs, all on account A, sequential ---
if [ "$SMOKE" -gt 0 ] 2>/dev/null; then
  echo "=== SMOKE: $SMOKE spec run(s) on $ACCT_A ==="
  for i in $(seq 0 $((SMOKE-1))); do
    run_spec "${SLUGS[$i]}" "$ACCT_A"
  done
  got=0
  for i in $(seq 0 $((SMOKE-1))); do
    [ -s "$OUT/spec-${SLUGS[$i]}.md" ] && got=$((got+1))
  done
  echo "=== SMOKE done: $got/$SMOKE specs in $OUT ==="
  exit 0
fi

# --- Only mode: re-run named specs (straggler tool) ---
if [ -n "$ONLY" ]; then
  echo "=== ONLY: $ONLY ==="
  fail=0
  IFS=',' read -ra wants <<< "$ONLY"
  for want in "${wants[@]}"; do
    i=$(index_for "$want")
    if [ "$i" = "-1" ]; then echo "[only] unknown spec: $want" >&2; fail=1; continue; fi
    acct="$ACCT_A"; [ "$i" -ge 6 ] && acct="$ACCT_B"
    case "$FORCE_ACCT" in A) acct="$ACCT_A";; B) acct="$ACCT_B";; esac
    run_spec "${SLUGS[$i]}" "$acct" || fail=1
  done
  write_index
  exit "$fail"
fi

# --- Full mode: S01-S06 on A, S07-S12 on B, accounts concurrent, sequential within account ---
echo "=== FULL: 12 specs (S01-S06 → $ACCT_A | S07-S12 → $ACCT_B) ==="
( for i in 0 1 2 3 4 5;  do run_spec "${SLUGS[$i]}" "$ACCT_A"; done ) &
pidA=$!
( for i in 6 7 8 9 10 11; do run_spec "${SLUGS[$i]}" "$ACCT_B"; done ) &
pidB=$!
wait "$pidA" "$pidB"

write_index
got=$(ls "$OUT"/spec-S*.md 2>/dev/null | wc -l)
echo "=== FULL done: $got/12 specs in $OUT ==="
ls -la "$OUT"/spec-S*.md 2>/dev/null
[ "$got" -ge 12 ]
