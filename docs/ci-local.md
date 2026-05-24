# CI Local

OpenQG keeps the GitHub Actions lanes thin and mirrors them with local shell
wrappers in `ops/ci/`.

## Local entry points

- `bash scripts/ci-local.sh fast`
- `bash scripts/ci-local.sh benchmark-smoke`
- `bash scripts/ci-local.sh security`
- `bash scripts/ci-local.sh jankurai`
- `bash scripts/ci-local.sh doctor`
- `just ci-pr` creates a clean merge-result worktree against `origin/main`
  and runs the local bundle there.

## Pre-push hook

The local hook at `ops/git-hooks/pre-push` delegates to
`bash ops/ci/quality-gates.sh`, which runs the canonical quality gate bundle.

## Policy

- keep lane logic in `ops/ci/*.sh`
- keep GitHub workflows as thin wrappers over those scripts
- use `bash scripts/ci-doctor.sh` to confirm the local toolchain before a lane
