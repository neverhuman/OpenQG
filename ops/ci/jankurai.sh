#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
# shellcheck source=ops/ci/lib.sh
source "$repo_root/ops/ci/lib.sh"
ci::cd_root

mkdir -p target/openqg target/jankurai reports/releases/draft

cargo fetch --locked
cargo install --locked cargo-nextest
cargo install --git https://github.com/neverhuman/jankurai --tag v1.5.1 --locked jankurai
cargo install --locked just

export CARGO_TARGET_DIR=target/fast
export CARGO_INCREMENTAL=0
export CARGO_NET_OFFLINE=true

cargo nextest run -p openqg-core --locked --frozen
cargo test -p openqg-core --locked --frozen --doc
cargo nextest run -p openqg-bench --locked --frozen
cargo test -p openqg-bench --locked --frozen --doc
just schema-check
just data-verify
just zyal-validate
just zyal-jekko-preview
bash tools/security-lane.sh
cargo run -p openqg-bench -- bench run --suite benchmarks/suites/smoke.yml --theory theories/sm-gr-lcdm-mnu/manifest.yml --output target/openqg/bench-smoke/scorecard.json
cargo run -p openqg-bench -- score compare --scorecard target/openqg/bench-smoke/scorecard.json --json target/jankurai/repo-score.json --md target/jankurai/repo-score.md
jankurai audit . --mode advisory --json target/jankurai/accepted-baseline.json --md target/jankurai/accepted-baseline.md
jankurai audit . --mode ratchet --baseline target/jankurai/accepted-baseline.json --json target/jankurai/repo-score.json --md target/jankurai/repo-score.md
cargo run -p openqg-bench -- release pack --scorecard target/openqg/bench-smoke/scorecard.json --output reports/releases/draft

