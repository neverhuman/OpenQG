#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
# shellcheck source=ops/ci/lib.sh
source "$repo_root/ops/ci/lib.sh"
ci::cd_root

merge_worktree="$(mktemp -d)"

cleanup() {
  cd "$repo_root"
  git worktree remove "$merge_worktree" >/dev/null 2>&1 || rm -rf "$merge_worktree"
}
trap cleanup EXIT

ci::log "fetching origin/main for merge-result preflight"
git fetch origin main
ci::log "creating clean merge-result worktree"
git worktree add --detach "$merge_worktree" HEAD
cd "$merge_worktree"
ci::log "merging origin/main into the worktree"
git merge --no-edit origin/main
ci::log "running local CI bundle on the merged worktree"
bash scripts/ci-local.sh all
just release-check
