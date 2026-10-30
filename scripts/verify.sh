#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo test --locked --all-targets
cargo test --locked --doc
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
scenario_output="$(mktemp)"
trap 'rm -f "$scenario_output"' EXIT
cargo run --locked --example scenario > "$scenario_output"
cmp examples/scenario-output.json "$scenario_output"
cargo run --locked --example reserve_scenario > "$scenario_output"
cmp examples/reserve-scenario-output.json "$scenario_output"
cargo run --locked --example combined_stress -- --check
PYTHONDONTWRITEBYTECODE=1 python3 analysis/check_combined.py
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s analysis -p 'test_*.py'
PYTHONDONTWRITEBYTECODE=1 python3 analysis/run_study.py --check
PYTHONDONTWRITEBYTECODE=1 python3 -m analysis.reserve_study --check
PYTHONDONTWRITEBYTECODE=1 python3 -m analysis.lifecycle_study --check
python3 scripts/check_package.py
git diff --check
