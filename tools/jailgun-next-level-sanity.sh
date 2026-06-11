#!/usr/bin/env bash
# jailgun-next-level-sanity.sh — post-harvest sanity pass over tips/jailgun/next-level/spec-S*.md
# Checks: count, size floor, markdown shape, degradation signature, on-topic keywords,
# truncation heuristic, duplicate artifacts. Exit 0 only if all 12 pass.

set -uo pipefail
OUT="${1:-tips/jailgun/next-level}"
MIN_BYTES=8192
DEGRADED_RE='source archive (unavailable|not available)|archive (was |is )?not (present|available)|not present in the sandbox|could not (be )?extract'

# slug|regex of topic keywords (case-insensitive; at least one must match)
KEYWORDS=(
  'S01-derivation-sandbox|derivation|certificate|CAS|sympy|lean|trace|rigor'
  'S02-dual-path-hybrid-search|symbolic regression|genetic|PySR|funsearch|bottom-up|pareto'
  'S03-term-algebra|horndeski|lagrangian|alpha|EFT|grammar|term'
  'S04-gap-directed-decomposition|ablation|attribution|component|gap|shapley|freeze'
  'S05-data-acquisition|pantheon|DESI|covariance|dataset|DOI|holdout'
  'S06-kpi-anticheating|rubric|scorecard|KPI|gaming|exploit|metric'
  'S07-statistical-evidence|nested sampling|evidence|BIC|likelihood|prior|posterior'
  'S08-boltzmann-emulator|boltzmann|CLASS|CAMB|emulator|cosmopower|envelope'
  'S09-knowledge-hardening|corpus|retrieval|RAG|arxiv|literature|knowledge'
  'S10-field-state-tensions|H0|tension|SH0ES|S8|DESI|w0wa'
  'S11-paper-profundity|venue|abstract|conclusion|referee|exclusion|claim'
  'S12-token-compute-ledger|token|usage|ledger|cost|telemetry|observability'
)

fail=0
declare -A sums
for entry in "${KEYWORDS[@]}"; do
  slug="${entry%%|*}"; re="${entry#*|}"
  f="$OUT/spec-${slug}.md"
  problems=()
  if [ ! -s "$f" ]; then
    echo "FAIL $slug: MISSING"; fail=1; continue
  fi
  bytes=$(wc -c <"$f")
  [ "$bytes" -ge "$MIN_BYTES" ] || problems+=("too small: $bytes B")
  head -1 "$f" | grep -q '^#' || problems+=("line 1 not a markdown heading")
  grep -qiE "$DEGRADED_RE" "$f" && problems+=("degradation signature in body")
  grep -qiE "$re" "$f" || problems+=("off-topic: no keyword of /$re/")
  lastchar=$(tail -c 200 "$f" | tr -d '[:space:]' | tail -c 1)
  case "$lastchar" in
    .|!|\?|\)|\`|\||\*|\]|\"|\') :;;
    *) problems+=("possible truncation: ends with '$lastchar'");;
  esac
  sum=$(md5sum "$f" | awk '{print $1}')
  if [ -n "${sums[$sum]:-}" ]; then problems+=("duplicate of ${sums[$sum]}"); else sums[$sum]="$slug"; fi
  if [ ${#problems[@]} -eq 0 ]; then
    echo "OK   $slug ($bytes B, $(wc -w <"$f") words)"
  else
    echo "FAIL $slug: ${problems[*]}"; fail=1
  fi
done
exit "$fail"
