#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
# shellcheck source=ops/ci/lib.sh
source "$repo_root/ops/ci/lib.sh"
ci::cd_root

required_commands=(
  bash
  cargo
  git
  just
  jq
)

for cmd in "${required_commands[@]}"; do
  command -v "$cmd" >/dev/null 2>&1 || {
    printf 'missing required command: %s\n' "$cmd" >&2
    exit 1
  }
done

JANKURAI_SKIP_FETCH=1 version="$(ci::jankurai_run --version 2>&1 | head -1 || true)"
case "$version" in
  *"jankurai 1.5.1"*) ;;
  *)
    printf 'expected jankurai 1.5.1, got: %s\n' "${version:-unknown}" >&2
    exit 1
    ;;
esac

printf 'ci doctor ok\n'
