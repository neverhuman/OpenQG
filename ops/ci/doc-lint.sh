#!/usr/bin/env bash
# doc-lint: forbid bare superiority claims in docs/. A line may mention "+36.7" or "beats ΛCDM"
# ONLY if it is clearly framed as superseded/debunked (carries one of the allow-markers below) — so
# the historical record and the refutation docs are fine, but a new bare "beats ΛCDM by X" is not.
# v3.0.0 M0. Exit non-zero on any offending line.
set -uo pipefail
cd "$(dirname "$0")/../.." || exit 1

# Lines asserting a head-to-head win without a fair-comparison qualifier.
PATTERN='\+36\.7|beats (Λ|lambda)cdm|champion beats|better than (the )?(gr )?(Λ|lambda)cdm'
# A mention is allowed if the same line is explicitly framed as not-a-real-result.
ALLOW='supersed|historical|artifact|do not cite|debunk|indefensible|disfavor|refut|not a fair|opposite|inflated|overstate|guts the|only part that'
# The canonical critical-review / fair-scoring docs exist to refute the figure — exclude them.
EXCLUDE_DOCS='docs/theory-league.md|docs/zyal-next-level-design.md'

offenders=$(grep -rniE "$PATTERN" docs/ 2>/dev/null | grep -viE "$ALLOW" | grep -viE "$EXCLUDE_DOCS")
if [ -n "$offenders" ]; then
  echo "doc-lint FAIL: bare superiority claim(s) without a fair-comparison qualifier:" >&2
  echo "$offenders" >&2
  echo "Fix: quote the theory-league ΔAIC/Δln-evidence, or label the line superseded/artifact." >&2
  exit 1
fi
echo "doc-lint OK: no bare superiority claims in docs/"
