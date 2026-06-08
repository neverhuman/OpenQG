#!/usr/bin/env bash
# jailgun-physics-review.sh — fan curated OpenQG payload out to physicist-review browser tabs and
# collect their engineering-spec.md outputs.
#
# Each lens is one single-tab `jailhard --download-only` run: it tars EXACTLY the files in
# ops/jailgun/review-manifest.txt (via the new --include-manifest flag), uploads them to one ChatGPT
# account/tab with a lens prompt, and downloads the returned `.tar.gz` (which the prompt instructs to
# contain a single engineering-spec.md). We extract that .md into tips/jailgun/.
#
# Modes:
#   --smoke N     run only the first N lenses, all on account A, sequentially (the live shakeout)
#   --waves K     full mode: K waves of all 6 lenses (3 on acct A, 3 on acct B, accounts concurrent)
#   --build       rebuild the jailhard binary first
#   --out DIR     output dir for the specs (default: tips/jailgun)
# Default (no mode flag): --waves 2  → 12 specs.
#
# Robust by design: 1 tab per run (one failure loses only that lens), best-effort harvest, up to
# 2 retries per lens, per-run isolated /tmp artifacts dir, and a final N/expected report.

set -uo pipefail

REPO="/home/ubuntu/openQG"
JAILGUN="/home/ubuntu/jailgun"
JAILHARD="$JAILGUN/target/debug/jailhard"
BRIDGE_MJS="$JAILGUN/apps/chrome-bridge/bin/chrome-bridge.mjs"
CONFIG_BASE="$JAILGUN/config/jailgun.example.toml"
MANIFEST="ops/jailgun/review-manifest.txt"
ARTBASE="/tmp/openqg-jailgun-review"
PROMPT_DIR="ops/jailgun/prompts"
ACCT_A="acct-19ae9aed"   # jepson@veox.ai
ACCT_B="acct-4690d657"   # bentaylorche@gmail.com
PER_RUN_TIMEOUT="${JAILGUN_REVIEW_TIMEOUT:-1500}"   # seconds per lens attempt
MAX_ATTEMPTS=2

# Lens order: index 0-2 → account A, 3-5 → account B.
LENS_SLUGS=(
  L1-observational-cosmologist
  L2-gravity-dark-energy-theorist
  L3-quantum-gravity-unification
  L4-statistics-inference
  L5-software-reproducibility
  L6-adversarial-skeptic
)

OUT="tips/jailgun"
WAVES=2
SMOKE=0
DO_BUILD=0
while [ $# -gt 0 ]; do
  case "$1" in
    --smoke) SMOKE="$2"; shift 2;;
    --waves) WAVES="$2"; shift 2;;
    --out) OUT="$2"; shift 2;;
    --build) DO_BUILD=1; shift;;
    *) echo "unknown arg: $1" >&2; exit 2;;
  esac
done

cd "$REPO" || { echo "cannot cd $REPO" >&2; exit 1; }
mkdir -p "$OUT" "$ARTBASE"

if [ "$DO_BUILD" = 1 ] || [ ! -x "$JAILHARD" ]; then
  echo "[build] cargo build -p jailgun-cli --bin jailhard"
  ( cd "$JAILGUN" && cargo build -p jailgun-cli --bin jailhard ) || { echo "build failed" >&2; exit 1; }
fi

# Pre-flight: every manifest path must exist (jailhard bails on a missing path).
miss=0
while IFS= read -r line; do
  case "$line" in ''|\#*) continue;; esac
  [ -f "$line" ] || { echo "[preflight] MISSING manifest file: $line" >&2; miss=$((miss+1)); }
done < "$MANIFEST"
[ "$miss" = 0 ] || { echo "[preflight] $miss manifest files missing — aborting" >&2; exit 1; }
echo "[preflight] manifest ok ($(grep -cvE '^\s*(#|$)' "$MANIFEST") files)"

# run_lens <prompt_file> <slug> <account> <wave_label>  → writes $OUT/spec-<slug>-<wave>.md or warns.
run_lens() {
  local prompt="$1" slug="$2" account="$3" wave="$4"
  local target="$OUT/spec-${slug}-${wave}.md"
  local attempt
  for attempt in $(seq 1 "$MAX_ATTEMPTS"); do
    local rundir="$ARTBASE/${slug}-${wave}-$$-${RANDOM}"
    mkdir -p "$rundir/extract"
    local cfg="$rundir/jailgun.toml"
    sed 's#^artifacts_dir = .*#artifacts_dir = "'"$rundir"'"#' "$CONFIG_BASE" > "$cfg"

    echo "[$slug/$wave] attempt $attempt on $account → $rundir"
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
      cp "$md" "$target"
      echo "[$slug/$wave] OK → $target ($(wc -c <"$target") bytes, rc=$rc)"
      return 0
    fi
    echo "[$slug/$wave] MISS (rc=$rc, dl='${dl:-none}') — see $rundir/jailhard.log" >&2
  done
  echo "[$slug/$wave] FAILED after $MAX_ATTEMPTS attempts" >&2
  return 1
}

# --- Smoke mode: first N lenses, all on account A, sequential ---
if [ "$SMOKE" -gt 0 ] 2>/dev/null; then
  echo "=== SMOKE: $SMOKE lens run(s) on $ACCT_A ==="
  for i in $(seq 0 $((SMOKE-1))); do
    run_lens "$PROMPT_DIR/${LENS_SLUGS[$i]}.md" "${LENS_SLUGS[$i]}" "$ACCT_A" "smoke"
  done
  got=$(ls "$OUT"/spec-*-smoke.md 2>/dev/null | wc -l)
  echo "=== SMOKE done: $got/$SMOKE specs in $OUT ==="
  exit 0
fi

# --- Full mode: WAVES waves of 6 lenses; A runs L1-3, B runs L4-6, concurrently ---
echo "=== FULL: $WAVES wave(s) × 6 lenses = $((WAVES*6)) specs ==="
for w in $(seq 1 "$WAVES"); do
  echo "--- wave $w ---"
  ( for i in 0 1 2; do run_lens "$PROMPT_DIR/${LENS_SLUGS[$i]}.md" "${LENS_SLUGS[$i]}" "$ACCT_A" "w$w"; done ) &
  pidA=$!
  ( for i in 3 4 5; do run_lens "$PROMPT_DIR/${LENS_SLUGS[$i]}.md" "${LENS_SLUGS[$i]}" "$ACCT_B" "w$w"; done ) &
  pidB=$!
  wait "$pidA" "$pidB"
done

got=$(ls "$OUT"/spec-*-w*.md 2>/dev/null | wc -l)
echo "=== FULL done: $got/$((WAVES*6)) specs in $OUT ==="
ls -la "$OUT"/spec-*.md 2>/dev/null
[ "$got" -ge $((WAVES*6)) ]
