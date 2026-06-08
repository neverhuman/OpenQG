#!/usr/bin/env bash
# Lane: benchmark-smoke — run the smoke benchmark suite and assert a scorecard.
# Parity: mirror of `just bench-smoke` and .github/workflows/benchmark-smoke.yml.
CI_LANE=benchmark-smoke
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

require_tool just cargo
log "running: just bench-smoke"
just bench-smoke
assert_artifact target/openqg/bench-smoke/scorecard.json
log "benchmark-smoke lane complete"
