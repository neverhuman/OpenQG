#!/usr/bin/env bash
# Generate zyal-jekko-preview.jsonl from .zyal runbooks.
#
# The jekko package path is configured via JEKKO_PKG_DIR (defaults to the
# original author's Mac path). If the package directory does not exist — which
# is expected on CI runners and Linux dev boxes — the script writes a
# skip-marker entry and exits 0 so CI is not blocked.
set -euo pipefail

OUT="target/openqg/zyal/jekko-preview.jsonl"
JEKKO_PKG="${JEKKO_PKG_DIR:-/Users/bentaylor/code/opencode/packages/jekko}"

if [ ! -d "$JEKKO_PKG" ]; then
    printf 'WARN: jekko package not found at %s (set JEKKO_PKG_DIR to override); preview skipped.\n' \
        "$JEKKO_PKG" >&2
    printf '{"skipped":true,"reason":"jekko package not found","hint":"set JEKKO_PKG_DIR"}\n' \
        >> "$OUT"
    exit 0
fi

for file in agent/zyal/*.zyal; do
    rtk bun --cwd "$JEKKO_PKG" src/index.ts daemon preview -f "$(pwd)/$file" >> "$OUT"
done
