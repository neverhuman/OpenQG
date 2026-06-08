#!/usr/bin/env bash
# Local mirror of CI: runs every lane wrapper in order, stopping on the first
# failure, with clear per-lane logging.
#
# Parity contract (see docs/ci-local.md): for each lane L below there is a
# matching ops/ci/L.sh wrapper and a <L>.yml GitHub workflow that runs the same
# wrapper, so `bash scripts/ci-local.sh` reproduces CI locally.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

# Order: cheap/fast feedback first, heavier audit cascade last.
lanes=(fast zyal benchmark-smoke security jankurai)

say() { printf '\n========== [ci-local] %s ==========\n' "$*" >&2; }

for lane in "${lanes[@]}"; do
  say "lane START: $lane (ops/ci/${lane}.sh)"
  if ! bash "ops/ci/${lane}.sh"; then
    printf '\n[ci-local] FAILED at lane: %s\n' "$lane" >&2
    exit 1
  fi
  say "lane PASS: $lane"
done

printf '\n[ci-local] all lanes passed: %s\n' "${lanes[*]}" >&2
