# Local ↔ CI parity (`ci-local-parity`)

This repo guarantees a **single source of truth per CI lane**. For every lane `L`:

```
just <L-recipe>   ≡   bash ops/ci/L.sh   ≡   .github/workflows/L.yml step   ≡   a line in scripts/ci-local.sh
```

The GitHub workflows are **thin**: each one only checks out, sets up the Rust
toolchain (and cache, where relevant), then runs a single
`bash ops/ci/<lane>.sh`. All real logic lives in `ops/ci/*.sh`, so what runs in
CI is exactly what you can run locally.

## Lanes

| Lane | `ops/ci/*.sh` | Underlying `just` work | Workflow | Asserted artifacts |
|------|---------------|------------------------|----------|--------------------|
| fast | `ops/ci/fast.sh` | `just fast` (fmt-check, core/bench unit + doc tests) | `fast.yml` | — |
| zyal | `ops/ci/zyal.sh` | `just zyal-test` | (run via `ci.yml` / pre-push) | — |
| benchmark-smoke | `ops/ci/benchmark-smoke.sh` | `just bench-smoke` | `benchmark-smoke.yml` | `target/openqg/bench-smoke/scorecard.json` |
| security | `ops/ci/security.sh` | `just security` (`tools/security-lane.sh`) | `security.yml` | `target/jankurai/security/lane-status.txt` |
| jankurai | `ops/ci/jankurai.sh` | full validate cascade + score compare + `audit-baseline` + `audit-ratchet` + release pack (relocated from `jankurai.yml`) | `jankurai.yml` | `target/openqg/bench-smoke/scorecard.json`, `target/jankurai/security/evidence.json`, `target/jankurai/accepted-baseline.json`, `target/jankurai/repo-score.json` |

Every lane wrapper sources `ops/ci/lib.sh`, which:

- enables `set -euo pipefail`,
- `cd`s to the repo root (git `--show-toplevel`, falling back to the script's
  own location so it works without git),
- and provides `log`, `die`, `require_tool <name>...`, and
  `assert_artifact <path>...` (fails if a declared artifact is missing/empty).

## Running everything locally

```sh
bash scripts/ci-local.sh     # runs all lanes in order, stops on first failure:
                             #   fast -> zyal -> benchmark-smoke -> security -> jankurai
```

The self-hosted runner (`.github/workflows/ci.yml`) calls
`ops/ci/pr-ci.sh`, which simply `exec`s `bash scripts/ci-local.sh` — so the
self-hosted CI and a local developer run the identical sequence.

## Doctor (preflight tool check)

```sh
bash scripts/ci-doctor.sh    # prints OK/MISSING for each required external tool
```

Required tools: `just`, `cargo`, `cargo-nextest`, `jankurai`, `jq`, `git`.
Optional: `shellcheck` (only used to lint these scripts). The doctor exits
non-zero if any **required** tool is missing.

## Pre-push hook

`ops/git-hooks/pre-push` runs `bash ops/ci/quality-gates.sh`, which aggregates
the cheap-but-meaningful gates before a push:

1. `bash ops/ci/fast.sh`
2. `bash ops/ci/zyal.sh`
3. `just audit-ratchet` — **guarded**: if no accepted baseline exists yet
   (`target/jankurai/accepted-baseline.json`), it warns and skips instead of
   hard-failing.

Install hooks with `just install-hooks` (sets `core.hooksPath=ops/git-hooks`).
Bypass a push gate with `git push --no-verify` (use sparingly).

## Future work

Determinism hardening — e.g. exporting `OMP_NUM_THREADS=1` (and other
thread/seed pinning) across lanes so byte-for-byte reproducible artifacts can be
asserted — is **not yet wired in** and is tracked as future work.
