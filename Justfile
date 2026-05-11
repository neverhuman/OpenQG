default: fast

setup:
	mkdir -p target/openqg target/jankurai reports/releases/draft
	cargo fetch

fast:
	cargo test --workspace --locked

schema-check:
	cargo run -p openqg-bench -- schema check

schema-sync:
	cargo run -p openqg-bench -- schema sync

data-lock:
	cargo run -p openqg-bench -- data lock --output target/openqg/data/locks/data-lock.json

data-verify: data-lock
	cargo run -p openqg-bench -- data verify

data-smoke:
	cargo run -p openqg-bench -- data smoke

bench-smoke:
	cargo run -p openqg-bench -- bench run --suite benchmarks/suites/smoke.yml --theory theories/sm-gr-lcdm-mnu/manifest.yml --output target/openqg/bench-smoke/scorecard.json

bench-full:
	cargo run -p openqg-bench -- bench run --suite benchmarks/suites/full.yml --theory theories/sm-gr-lcdm-mnu/manifest.yml --output target/openqg/bench-full/scorecard.json

zyal-validate:
	cargo run -p openqg-bench -- zyal validate

score:
	cargo run -p openqg-bench -- score compare --scorecard target/openqg/bench-smoke/scorecard.json --json target/jankurai/repo-score.json --md target/jankurai/repo-score.md

security:
	bash tools/security-lane.sh

check:
	just schema-check
	just data-verify
	just bench-smoke
	just zyal-validate
	just security

release-check:
	just check
	just score
	just release-pack

release-pack:
	cargo run -p openqg-bench -- release pack --scorecard target/openqg/bench-smoke/scorecard.json --output reports/releases/draft
