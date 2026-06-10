#!/usr/bin/env bash
# V5 chunked campaign driver — survives a hostile (heavily loaded) box.
#
# Runs N short campaign chunks (each its own run-id + seed). Each chunk is a complete, auditable
# run (white paper, ledgers, replayable). The proposer MEMORY section reads ALL prior run ledgers,
# so learning compounds across chunks: this is one long campaign, delivered in kill-resistant
# pieces. A chunk that dies mid-flight loses nothing (streaming sinks + checkpoints) and the
# driver simply moves on.
set -u
cd /home/ubuntu/openQG

CHUNKS="${CHUNKS:-6}"
GENS="${GENS:-300}"
SEED_BASE="${SEED_BASE:-20260620}"
LOG_DIR=target/openqg
mkdir -p "$LOG_DIR"

for i in $(seq 1 "$CHUNKS"); do
  seed=$((SEED_BASE + i))
  run_id="v5-chunk-$i"
  log="$LOG_DIR/$run_id.log"
  echo "[driver] chunk $i/$CHUNKS — $GENS gens, seed $seed, run-id $run_id ($(date -u +%H:%M:%S))"
  nice -n 10 env RUST_BACKTRACE=1 cargo run -q -p openqg-bench -- zyal genome whitepaper \
    --observables data/fixtures/cosmology/tier1-multisector.jsonl \
    --max-generations "$GENS" --population-size 12 --seed "$seed" \
    --with-jekko-proposer --jekko-every 40 --jekko-samples 3 --jekko-repairs 2 \
    --jekko-timeout-seconds 300 --jekko-quality-band none \
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
  f="target/openqg/zyal-genome/runs/v5-chunk-$i/white-paper.json"
  [ -f "$f" ] && jq -c --arg c "$i" '{chunk:$c, champion:.champion.id, total:.champion.scorecard.total}' "$f" 2>/dev/null
done
