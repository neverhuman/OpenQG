#!/usr/bin/env bash
# Lane: zyal — ZYAL validation / robustness tests.
# Parity: mirror of `just zyal-test` (added by the zyal-test worker).
CI_LANE=zyal
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

require_tool just cargo
log "running: just zyal-test"
just zyal-test
log "zyal lane complete"
