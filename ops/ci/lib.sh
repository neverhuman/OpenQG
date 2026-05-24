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

ci::jankurai_source_root() {
  printf '%s\n' "${JANKURAI_SOURCE_ROOT:-$(ci::repo_root)/target/vendor/jankurai-src}"
}

ci::ensure_jankurai_source_root() {
  local source_root
  source_root="$(ci::jankurai_source_root)"

  if [ -d "$source_root/.git" ] && [ -f "$source_root/Cargo.toml" ]; then
    printf '%s\n' "$source_root"
    return 0
  fi

  rm -rf "$source_root"
  mkdir -p "$(dirname "$source_root")"
  git clone --depth 1 --branch "${JANKURAI_SOURCE_TAG:-v1.5.1}" \
    https://github.com/neverhuman/jankurai.git "$source_root"
  printf '%s\n' "$source_root"
}

ci::jankurai_run() {
  local source_root target_dir
  source_root="$(ci::ensure_jankurai_source_root)"
  target_dir="${JANKURAI_TARGET_DIR:-$(ci::repo_root)/target/jankurai-source-target}"
  if [ "${JANKURAI_SKIP_FETCH:-0}" != "1" ]; then
    CARGO_TARGET_DIR="$target_dir" CARGO_NET_OFFLINE=false cargo fetch \
      --locked --manifest-path "$source_root/crates/jankurai/Cargo.toml"
  fi
  CARGO_TARGET_DIR="$target_dir" CARGO_NET_OFFLINE=false cargo run --quiet \
    --manifest-path "$source_root/crates/jankurai/Cargo.toml" -- "$@"
}
