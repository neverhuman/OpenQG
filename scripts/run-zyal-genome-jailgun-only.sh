#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec cargo run -p openqg-bench -- zyal genome run --variant jailgun-only "$@"
