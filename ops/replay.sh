#!/usr/bin/env bash
# OpenQG v3.0.0 — independent reproducible-replay package (M6).
#
# A fresh checkout + `./ops/replay.sh` re-derives the headline 5-theory league from the committed
# fixtures and FAILS on drift beyond a tolerance. The league fit is deterministic (fixed-seed
# Nelder-Mead, pure-Rust forward model), so the numbers are reproducible; the tolerance absorbs
# only cross-platform f64 rounding. This is the reviewers' top reproducibility ask: a referee runs
# one command and re-derives the flagship numbers.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
REPO="$(cd "${SCRIPT_DIR}/.." >/dev/null 2>&1 && pwd)"
cd "${REPO}"

EXPECTED="ops/replay/expected-league-v1.json"
GOT="target/openqg/replay/league-v1.json"
TOL="${OPENQG_REPLAY_TOL:-0.05}"   # absolute tolerance on chi2 / AIC / dAIC

echo "== OpenQG reproducible replay =="
echo "repo:   ${REPO}"
echo "commit: $(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
echo "[build] cargo build -p openqg-bench"
cargo build -q -p openqg-bench

echo "[audit] data integrity (sha256 vs data/manifest.json)"
cargo run -q -p openqg-bench -- data audit

echo "[run] theory league (tier0 + growth + S8) -> ${GOT}"
mkdir -p "$(dirname "${GOT}")"
cargo run -q -p openqg-bench -- theory league \
  --observables data/fixtures/cosmology/tier0-combined.jsonl \
  --observables data/fixtures/cosmology/growth-rsd.jsonl \
  --observables data/fixtures/cosmology/wl-s8.jsonl \
  --output "${GOT}" >/dev/null

echo "[compare] ${GOT} vs sealed ${EXPECTED} (tol=${TOL})"
python3 - "$EXPECTED" "$GOT" "$TOL" <<'PY'
import json, sys
exp, got, tol = json.load(open(sys.argv[1])), json.load(open(sys.argv[2])), float(sys.argv[3])
def by_id(d): return {r["model_id"]: r for r in d["ranking"]}
e, g = by_id(exp), by_id(got)
if set(e) != set(g):
    print(f"FAIL: model set drift: expected {sorted(e)} got {sorted(g)}"); sys.exit(1)
bad = 0
for m in sorted(e):
    for field in ("chi2", "aic", "delta_aic"):
        de, dg = e[m][field], g[m][field]
        if abs(de - dg) > tol:
            print(f"FAIL: {m}.{field} drift: expected {de:.4f} got {dg:.4f} (>{tol})"); bad += 1
        else:
            print(f"  ok  {m:14s} {field:10s} {dg:+.4f}")
if bad:
    print(f"REPLAY FAILED: {bad} metric(s) drifted beyond {tol}"); sys.exit(1)
print("REPLAY OK: 5-theory league reproduced within tolerance")
PY
