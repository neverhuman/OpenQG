#!/usr/bin/env bash
# Lane: fast — fmt-check + core/bench unit & doc tests.
# Parity: mirror of `just fast` and .github/workflows/fast.yml.
CI_LANE=fast
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

require_tool just cargo
log "running: just fast"
just fast
log "fast lane complete"
