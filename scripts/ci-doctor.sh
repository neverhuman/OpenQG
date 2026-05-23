#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

required_commands=(
  bash
  cargo
  git
  just
  jankurai
  jq
)

for cmd in "${required_commands[@]}"; do
  command -v "$cmd" >/dev/null 2>&1 || {
    printf 'missing required command: %s\n' "$cmd" >&2
    exit 1
  }
done

printf 'ci doctor ok\n'

