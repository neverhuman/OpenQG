# OpenQG Master Plan

## Phase 0: Bootstrap

- initialize workspace metadata
- define ownership, generated zones, and validation lanes

## Phase 1: Contracts and Data

- add schema specs under `contracts/specs/` and generate outputs under `contracts/generated/schemas/`
- register public datasets under `data/registry/`
- generate a data lock under `target/openqg/data/locks/`

## Phase 2: Benchmark Core

- keep the baseline theory manifest in `theories/sm-gr-lcdm-mnu/`
- define benchmark suites under `benchmarks/suites/`
- run smoke scoring against committed fixtures

## Phase 3: ZYAL Research Loops

- add host-owned runbooks under `agent/zyal/`
- validate runbooks before execution
- keep runtime and preview behavior separable

## Phase 4: Release and Audit

- compare benchmark output into `target/jankurai/repo-score.*`
- pack release evidence under `reports/releases/draft/`
- keep CI focused on reproducible local lanes
