#!/usr/bin/env bash
# CI doctor: report presence (OK/MISSING) of every external tool the
# ops/ci/* lanes depend on. Exits non-zero if a *required* tool is missing.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

# Tools the lanes invoke (just/cargo recipes, jankurai audit, jq in the
# pre-commit gate, git for repo-root resolution).
required_tools=(just cargo cargo-nextest jankurai jq git)
# Nice-to-have; only used for linting these scripts.
optional_tools=(shellcheck)

missing=0

check_tool() {
  local tool="$1" kind="$2" path
  if path="$(command -v "$tool" 2>/dev/null)"; then
    printf 'OK       %-16s %s\n' "$tool" "$path"
  elif [[ "$kind" == required ]]; then
    printf 'MISSING  %-16s (required)\n' "$tool"
    missing=$((missing + 1))
  else
    printf 'missing  %-16s (optional)\n' "$tool"
  fi
}

printf 'ci-doctor: external tool check for ops/ci/* lanes\n\n'
for t in "${required_tools[@]}"; do check_tool "$t" required; done
for t in "${optional_tools[@]}"; do check_tool "$t" optional; done
printf '\n'

if (( missing > 0 )); then
  printf 'ci-doctor: %d required tool(s) MISSING\n' "$missing" >&2
  exit 1
fi
printf 'ci-doctor: all required tools present\n'
