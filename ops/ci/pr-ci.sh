#!/usr/bin/env bash
# Entrypoint invoked by the self-hosted ci.yml workflow (the "host-ci cascade"
# step). Delegates to the local-CI mirror so that GitHub CI and a developer
# running `bash scripts/ci-local.sh` execute the exact same lane sequence.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

exec bash scripts/ci-local.sh
