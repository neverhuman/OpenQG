#!/usr/bin/env bash
# Lane: jankurai — the full validate cascade + audit ratchet.
# Parity: relocated verbatim from the inline cascade in
# .github/workflows/jankurai.yml (the `validate` job's run: steps), so the
# workflow can collapse to a single `bash ops/ci/jankurai.sh` call.
CI_LANE=jankurai
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

require_tool just cargo jankurai

log "cargo fetch --locked"
cargo fetch --locked

log "openqg-core nextest"
CARGO_TARGET_DIR=target/fast CARGO_INCREMENTAL=0 CARGO_NET_OFFLINE=true cargo nextest run -p openqg-core --locked --frozen
log "openqg-core doctests"
CARGO_TARGET_DIR=target/fast CARGO_INCREMENTAL=0 CARGO_NET_OFFLINE=true cargo test -p openqg-core --locked --frozen --doc
log "openqg-bench nextest"
CARGO_TARGET_DIR=target/fast CARGO_INCREMENTAL=0 CARGO_NET_OFFLINE=true cargo nextest run -p openqg-bench --locked --frozen
log "openqg-bench doctests"
CARGO_TARGET_DIR=target/fast CARGO_INCREMENTAL=0 CARGO_NET_OFFLINE=true cargo test -p openqg-bench --locked --frozen --doc

log "schema check"
cargo run -p openqg-bench -- schema check
log "zyal validate"
cargo run -p openqg-bench -- zyal validate
log "zyal jekko preview"
just zyal-jekko-preview

log "jankurai security run"
jankurai security run . --strict --profile ci --out target/jankurai/security/evidence.json

log "bench smoke scorecard"
cargo run -p openqg-bench -- bench run --suite benchmarks/suites/smoke.yml --theory theories/sm-gr-lcdm-mnu/manifest.yml --output target/openqg/bench-smoke/scorecard.json
log "score compare"
cargo run -p openqg-bench -- score compare --scorecard target/openqg/bench-smoke/scorecard.json --json target/jankurai/repo-score.json --md target/jankurai/repo-score.md

log "audit advisory (accepted baseline)"
jankurai audit . --mode advisory --full --json target/jankurai/accepted-baseline.json --md target/jankurai/accepted-baseline.md
log "audit ratchet"
jankurai audit . --mode ratchet --full --baseline target/jankurai/accepted-baseline.json --json target/jankurai/repo-score.json --md target/jankurai/repo-score.md

log "release pack"
cargo run -p openqg-bench -- release pack --scorecard target/openqg/bench-smoke/scorecard.json --output reports/releases/draft

# Assert the key evidence artifacts the cascade is expected to produce.
assert_artifact \
  target/openqg/bench-smoke/scorecard.json \
  target/jankurai/security/evidence.json \
  target/jankurai/accepted-baseline.json \
  target/jankurai/repo-score.json

log "jankurai lane complete"
