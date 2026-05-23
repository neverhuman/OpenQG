#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

case "${1:-all}" in
  fast)
    bash ops/ci/fast.sh
    ;;
  benchmark-smoke|bench-smoke)
    bash ops/ci/benchmark-smoke.sh
    ;;
  security)
    bash ops/ci/security.sh
    ;;
  jankurai)
    bash ops/ci/jankurai.sh
    ;;
  doctor)
    bash scripts/ci-doctor.sh
    ;;
  all)
    bash scripts/ci-doctor.sh
    bash ops/ci/fast.sh
    bash ops/ci/benchmark-smoke.sh
    bash ops/ci/security.sh
    ;;
  *)
    printf 'usage: %s [all|fast|benchmark-smoke|security|jankurai|doctor]\n' "$0" >&2
    exit 1
    ;;
esac

