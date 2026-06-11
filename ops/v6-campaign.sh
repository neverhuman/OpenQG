#!/usr/bin/env bash
# V6 chunked campaign driver — router-native proposer + covariance-aware likelihood.
#
# Chunked for a hostile (heavily loaded) box: each chunk is a complete, auditable run (white
# paper, streamed ledgers, replayable); the proposer MEMORY compounds across chunks via the run
# ledgers, so this is one long campaign in kill-resistant pieces. The router proposer fires a
# best-of-K slot (mechanism lanes: planck_mu0 / dark_scattering / free / null_diagnostic;
# quality-band rotation) every ROUTER_EVERY generations against the local jnoccio-fusion gateway
# (free tokens, ~131 models, server-side strict-schema validation).
set -u
cd /home/ubuntu/openQG

CHUNKS="${CHUNKS:-6}"
GENS="${GENS:-300}"
SEED_BASE="${SEED_BASE:-20260640}"
ROUTER_EVERY="${ROUTER_EVERY:-20}"
SAMPLES="${SAMPLES:-4}"
COV_ARGS=(--covariance data/fixtures/cosmology/covariance/planck18-distance-priors.json
          --covariance data/fixtures/cosmology/covariance/desi-dr1-bao.json)
LOG_DIR=target/openqg
mkdir -p "$LOG_DIR"

for i in $(seq 1 "$CHUNKS"); do
  seed=$((SEED_BASE + i))
  run_id="${RUN_PREFIX:-v6-chunk}-$i"
  log="$LOG_DIR/$run_id.log"
  echo "[driver] chunk $i/$CHUNKS — $GENS gens, seed $seed, run-id $run_id ($(date -u +%H:%M:%S))"
  nice -n 10 env RUST_BACKTRACE=1 ./target/openqg/pinned/openqg-bench-v6-campaign zyal genome whitepaper \
    --observables data/fixtures/cosmology/tier1-multisector.jsonl \
    "${COV_ARGS[@]}" \
    --max-generations "$GENS" --population-size 12 --seed "$seed" \
    --with-router-proposer --router-every "$ROUTER_EVERY" --router-samples "$SAMPLES" \
    --router-repairs 2 \
    --run-id "$run_id" > "$log" 2>&1
  ec=$?
  d="target/openqg/zyal-genome/runs/$run_id"
  gens=$(wc -l < "$d/progress-ledger.jsonl" 2>/dev/null || echo 0)
  props=$(wc -l < "$d/proposal-ledger.jsonl" 2>/dev/null || echo 0)
  echo "[driver] chunk $i exit=$ec gens=$gens proposals=$props"
  if [ -f "$d/white-paper.json" ]; then
    jq -c '{champion:.champion.id, total:.champion.scorecard.total, distinct:.champion.distinct_from_baseline}' "$d/white-paper.json" 2>/dev/null | sed 's/^/[driver]   /'
  else
    echo "[driver]   (chunk killed mid-flight — ledgers + checkpoint preserved, continuing)"
  fi
  sleep 5
done

echo "[driver] ALL CHUNKS DONE ($(date -u +%H:%M:%S)) — cross-run summary:"
for i in $(seq 1 "$CHUNKS"); do
  f="target/openqg/zyal-genome/runs/${RUN_PREFIX:-v6-chunk}-$i/white-paper.json"
  [ -f "$f" ] && jq -c --arg c "$i" '{chunk:$c, champion:.champion.id, total:.champion.scorecard.total}' "$f" 2>/dev/null
done
echo "[driver] attempt funnel:"
cat target/openqg/zyal-genome/runs/${RUN_PREFIX:-v6-chunk}-*/proposal-attempts.jsonl 2>/dev/null | jq -r '.outcome' | sort | uniq -c
echo "[driver] per-lane yield:"
cat target/openqg/zyal-genome/runs/${RUN_PREFIX:-v6-chunk}-*/proposal-attempts.jsonl 2>/dev/null | jq -r 'select(.outcome=="ok") | .mechanism_lane' | sort | uniq -c
echo "[driver] per-model yield (top 10):"
cat target/openqg/zyal-genome/runs/${RUN_PREFIX:-v6-chunk}-*/proposal-attempts.jsonl 2>/dev/null | jq -r 'select(.outcome=="ok") | .model' | sort | uniq -c | sort -rn | head -10
