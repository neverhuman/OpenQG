# OpenQG

OpenQG is a benchmark and control plane for interpretable unified-physics theories.

The repo is intentionally conservative:

- baseline physics stays anchored to `SM + GR + LambdaCDM + massive neutrinos`
- theory candidates must expose named physical parameters
- public-source data is tracked through manifests, not raw bulk dumps
- smoke validation uses tiny committed fixtures only

## Quick Start

```bash
just setup
just fast
just check
just data-verify
just bench-smoke
just zyal-validate
just score
```

## Layout

- `crates/openqg-core`: domain types, validators, scoring primitives
- `crates/openqg-data`: registry loading and lock generation
- `crates/openqg-bench`: CLI for schema, data, benchmark, score, release, and ZYAL checks
- `contracts/specs`: editable YAML contract specs for datasets, theories, observables, predictions, scores, suites, and releases
- `contracts/generated/schemas`: generated JSON schema artifacts from `just schema-sync`
- `contracts/registry.yml`: the source-of-truth registry for contract generation
- `data/registry`: source manifests for public datasets
- `benchmarks/suites`: benchmark suite manifests
- `theories`: baseline and candidate theory manifests plus adapters
- `ops/zyal`: host-owned runbooks for long-running research loops

## Policy

- Do not commit raw upstream data, secrets, or derived lockfiles.
- Do not broaden generated zones without updating `agent/generated-zones.toml`.
- Keep candidate theories readable: every parameter needs unit, meaning, and provenance.
