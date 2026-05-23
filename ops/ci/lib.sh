#!/usr/bin/env bash
set -euo pipefail

ci::repo_root() {
  git rev-parse --show-toplevel
}

ci::cd_root() {
  cd "$(ci::repo_root)"
}

ci::log() {
  printf '[ci] %s\n' "$*"
}

ci::require_command() {
  command -v "$1" >/dev/null 2>&1
}

