#!/usr/bin/env bash
# Lane: security — run the security lane (tools/security-lane.sh).
# Parity: mirror of `just security` and .github/workflows/security.yml.
CI_LANE=security
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

require_tool just cargo
log "running: just security"
just security
assert_artifact target/jankurai/security/lane-status.txt
log "security lane complete"
