# Architecture

OpenQG is now organized around one live hero/judge loop plus the support code
that keeps it honest:

- `agent/zyal/openqg-hero-judge-evolve.zyal` is the canonical runbook
- `reports/hero-judge-progress/2026-05-23.md` records the 25-run series
- `crates/openqg-core`, `crates/openqg-data`, and `crates/openqg-bench` keep
  validation, data loading, and scoring reproducible
- `contracts/registry.yml` and `contracts/specs/` own the schema surface
- `benchmarks/suites/smoke.yml` and `theories/sm-gr-lcdm-mnu/manifest.yml`
  provide the minimal benchmark baseline

The intended flow is:

1. edit the live runbook, manifests, or code
2. run the narrow proof lane
3. regenerate generated artifacts through the declared command
4. record evidence under `reports/hero-judge-progress/` and `reports/releases/draft/`
