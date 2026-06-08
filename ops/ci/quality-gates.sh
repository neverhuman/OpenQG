#!/usr/bin/env bash
# Aggregated quality gates for the pre-push hook (ops/git-hooks/pre-push).
# Runs the cheap-but-meaningful lanes plus the jankurai audit ratchet so a
# regression can never be pushed. The audit ratchet is guarded: if no accepted
# baseline exists yet it warns instead of hard-failing.
CI_LANE=quality-gates
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

log "pre-push quality gates: fast -> zyal -> audit-ratchet"

log "lane: fast"
bash ops/ci/fast.sh

log "lane: zyal"
bash ops/ci/zyal.sh

if [[ -s target/jankurai/accepted-baseline.json ]]; then
  log "running: just audit-ratchet"
  just audit-ratchet
else
  log "WARN: no accepted baseline (target/jankurai/accepted-baseline.json); skipping audit-ratchet."
  log "WARN: run 'just audit-baseline' to establish the accepted floor."
fi

log "quality gates passed"
