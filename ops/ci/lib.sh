#!/usr/bin/env bash
# Shared library for the OpenQG local-CI lane wrappers (ops/ci/*.sh).
#
# Source this at the top of every lane script:
#   source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
#
# It turns on strict mode, cd's to the repo root, and exposes logging plus
# tool/artifact assertion helpers. Dependency-light: bash + coreutils only.
set -euo pipefail

# Resolve the repo root. Prefer git; fall back to this library's own location
# (ops/ci/lib.sh -> repo root is two levels up) so it also works without git.
if command -v git >/dev/null 2>&1 \
  && _ci_root="$(git -C "$(dirname "${BASH_SOURCE[0]}")" rev-parse --show-toplevel 2>/dev/null)"; then
  CI_REPO_ROOT="$_ci_root"
else
  CI_REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
fi
unset _ci_root 2>/dev/null || true
cd "$CI_REPO_ROOT"

# log <msg...> : progress line on stderr, tagged with the current lane.
log() {
  printf '[ci:%s] %s\n' "${CI_LANE:-lib}" "$*" >&2
}

# die <msg...> : log an error and exit non-zero.
die() {
  printf '[ci:%s] ERROR: %s\n' "${CI_LANE:-lib}" "$*" >&2
  exit 1
}

# require_tool <name>... : ensure each named tool is on PATH, else die.
require_tool() {
  local tool
  for tool in "$@"; do
    command -v "$tool" >/dev/null 2>&1 || die "required tool not found on PATH: $tool"
  done
}

# assert_artifact <path>... : fail if any path is missing or empty.
assert_artifact() {
  local path
  for path in "$@"; do
    [[ -e "$path" ]] || die "expected artifact missing: $path"
    [[ -s "$path" ]] || die "expected artifact is empty: $path"
    log "artifact ok: $path"
  done
}
