#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
# shellcheck source=ops/ci/lib.sh
source "$repo_root/ops/ci/lib.sh"
ci::cd_root
ci::log "running fast lane"
just fast

