#!/usr/bin/env bash
# Render a human-readable summary of a ZYAL robustness genome run from its on-disk artifacts.
# Usage: tools/zyal-robustness-report.sh <run-dir>
# (no pipefail: the `| head` summaries intentionally close pipes early, which would otherwise
#  surface as a harmless SIGPIPE / non-zero exit on an otherwise-successful run)
set -eu
RUN_DIR="${1:?usage: zyal-robustness-report.sh <run-dir>}"
S="$RUN_DIR/run-summary.json"
G="$RUN_DIR/quality-gate.json"
M="$RUN_DIR/map-elites-archive.json"
A="$RUN_DIR/metrics-timeseries.jsonl"

echo "============================================================"
echo " ZYAL robustness run report: $(basename "$RUN_DIR")"
echo "============================================================"

if [[ -s "$G" ]] && command -v jq >/dev/null 2>&1; then
  echo "## Quality gate: $(jq -r '.passed' "$G")"
  jq -r '.checks[] | "   [\(if .passed then "PASS" else "FAIL" end)] \(.name) = \(.observed)"' "$G"
  echo "   adversary frontier_margin (final): $(jq -r '.attack_archive.frontier_margin // "n/a"' "$G")"
fi

if [[ -s "$S" ]] && command -v jq >/dev/null 2>&1; then
  echo "## Frontier"
  jq -r '
    (.generation_champions // []) as $c |
    "   generations: \($c | length)",
    "   distinct champion-survival values: \([$c[].final_score] | unique | length)",
    "   best champion survival: \(([$c[].final_score] | max))",
    "   island distribution: \(($c | group_by(.island) | map("\(.[0].island)=\(length)") | join(", ")))"
  ' "$S"
fi

if [[ -s "$M" ]] && command -v jq >/dev/null 2>&1; then
  echo "## Diversity (MAP-Elites)"
  jq -r '"   QD-score: \(.qd_score)   behavior cells covered: \(.coverage)"' "$M"
  echo "## Top whitebox theories (best delta_log_likelihood per behavior cell)"
  jq -r '.cells | sort_by(-.fitness) | .[:6][] | "   \(.candidate_id)  delta_log_likelihood=\((.fitness*1000|round)/1000)  cell(params,mechanism,fit)=\(.cell|@json)"' "$M"
fi

if [[ -s "$A" ]] && command -v jq >/dev/null 2>&1; then
  echo "## Diversity progression (QD-score over generations, sampled)"
  jq -rc 'select(.metric=="qd_score") | [.generation_id, (.value*100|round)/100, .meta.frontier_margin]' "$A" 2>/dev/null \
    | awk 'NR==1 || NR%200==0 {print "   " $0}'
fi

L="$RUN_DIR/live-critique-ledger.jsonl"
if [[ -s "$L" ]] && command -v jq >/dev/null 2>&1; then
  echo "## Live adversarial critic (jnoccio)"
  jq -s -r '
    (map(select(.status=="ok"))) as $ok |
    "   critiques: \(length)   ok: \($ok|length)   suppressed (after<before): \(map(select(.final_after < .final_before))|length)",
    (if ($ok|length) > 0 then
       "   mean falsifiability: \((($ok|map(.falsifiability)|add)/($ok|length)*100|round)/100)   mean plausibility: \((($ok|map(.plausibility)|add)/($ok|length)*100|round)/100)"
     else "   (no parsed verdicts yet)" end)
  ' "$L"
  echo "   most-cited fatal flaws:"
  jq -r 'select(.status=="ok" and (.fatal_flaw|length>0)) | .fatal_flaw' "$L" 2>/dev/null \
    | sort | uniq -c | sort -rn | head -5 | sed 's/^/     /'
fi
echo "============================================================"
echo "Full artifacts: $RUN_DIR/{run-summary.json,quality-gate.json,map-elites-archive.json,"
echo "  metrics-timeseries.jsonl,population-ledger.jsonl (per-candidate genes+physics+judge),"
echo "  live-critique-ledger.jsonl (jnoccio verdicts),"
echo "  island-leaderboard.json,novelty-archive.json,lineage-graph.jsonl}"
