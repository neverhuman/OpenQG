# OpenQG

OpenQG is an evidence-gated benchmark and control plane for interpretable
unified-physics theories.

It keeps candidate quantum-gravity work close to typed manifests, reproducible
smoke data, explicit scores, and auditable release receipts. The first release
is intentionally conservative: baseline physics stays anchored to
`SM + GR + LambdaCDM + massive neutrinos`, public data is tracked through
manifests instead of raw dumps, and candidate theories must expose named
physical parameters.

## Status

| Signal | Score | State |
| --- | ---: | --- |
| Jankurai audit | `90` | pass |
| Repo benchmark score | `99` | pass |
| Release | `v0.0.1` | candidate |
| Smoke benchmark | `benchmark-v0.1.0` | pass |
| Draft release pack | generated | `approved: false` |

Evidence artifacts:

- `target/jankurai/current-audit.json`
- `target/jankurai/current-audit.md`
- `target/jankurai/repo-score.json`
- `target/jankurai/repo-score.md`
- `target/openqg/bench-smoke/scorecard.json`
- `reports/releases/draft/release-manifest.json`

## Quick Start

```bash
just setup
just fast
just check
just security
just release-check
jankurai audit . \
  --json target/jankurai/current-audit.json \
  --md target/jankurai/current-audit.md \
  --no-score-history
```

## Surface Map

- `crates/openqg-core`: typed manifests, validation helpers, and scoring primitives
- `crates/openqg-data`: registry loading, data locks, and fixture verification
- `crates/openqg-bench`: CLI orchestration for schema, data, benchmark, score,
  release, and ZYAL lanes
- `contracts/specs`: editable YAML contract specs
- `contracts/generated/schemas`: generated JSON schemas from `just schema-sync`
- `contracts/registry.yml`: source-of-truth registry for contract generation
- `data/registry`: public dataset source manifests
- `benchmarks/suites`: benchmark suite manifests
- `theories`: baseline and candidate theory manifests
- `agent/zyal`: runnable research and maintenance loops
- `reports/releases/draft`: generated draft release pack from `just release-pack`

## Release

`v0.0.1` is the first release candidate for the OpenQG control plane. It ships
the Rust benchmark workspace, contract registry, smoke benchmark lane, Jankurai
audit evidence, ZYAL runbooks, release-pack generation, and the initial
documentation surface.

Publication follows the evidence gate in [docs/release.md](docs/release.md).
The draft release manifest remains `approved: false` until review records the
approval outside the generated release pack.

## Key Links

- [Moonshot](docs/MOONSHOT.md)
- [Architecture](docs/architecture.md)
- [Boundaries](docs/boundaries.md)
- [Testing](docs/testing.md)
- [Scoring](docs/scoring.md)
- [Benchmark methodology](docs/benchmark-methodology.md)
- [Theory admission](docs/theory-admission.md)
- [Release process](docs/release.md)

## Policy

- Do not commit raw upstream data, secrets, or derived lockfiles.
- Do not broaden generated zones without updating `agent/generated-zones.toml`.
- Keep candidate theories readable: every parameter needs unit, meaning, and provenance.
