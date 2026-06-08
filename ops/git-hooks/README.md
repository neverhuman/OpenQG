# Git hooks

Version-controlled hooks for OpenQG. Install with:

```sh
just install-hooks   # git config core.hooksPath ops/git-hooks
```

## `pre-commit` — jankurai regression gate

Runs a full advisory `jankurai audit` and **blocks the commit if it adds caps, findings, or hard
issues versus the accepted baseline** (`target/jankurai/accepted-baseline.json`). Pre-existing
accepted debt is tolerated; a change can only hold or improve the audit, which keeps the score
moving in one direction. (jankurai's own `--mode ratchet` additionally enforces the absolute
`minimum_score`, so it would block every commit until the repo reaches 85; this gate compares
deltas instead so the existing debt can be paid down incrementally.)

- Degrades to ADVISORY (warn + allow) if the audit exceeds `JANKURAI_HOOK_TIMEOUT` (default 90s).
- Bypass for emergencies: `JANKURAI_SKIP_HOOKS=1 git commit` or `git commit --no-verify`.

### Accepted-baseline ratchet (full-repo)

The diff gate stops *new* debt. To ratchet *down* existing debt, use the Justfile targets:

```sh
just audit-baseline   # snapshot the currently-accepted state -> target/jankurai/accepted-baseline.json
just audit-ratchet    # fail if the repo regressed below the accepted baseline
```

When you fix findings, re-run `just audit-baseline` to lower the accepted floor so it can never
creep back up. The repo currently sits below the `minimum_score = 85` target (see
`agent/audit-policy.toml`); the gate prevents regressions while that pre-existing debt is burned
down. Note: some `future-hostile-dead-language` findings are legitimate English words in semantic
strings (e.g. the `output-root-stale:` error messages, the `fallback_penalties` config field) and
should be resolved via jankurai policy allow-listing rather than by mangling correct code.
