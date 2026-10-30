#!/usr/bin/env bash
set -euo pipefail
zsv_root="$(cd "$(dirname "$0")/.." && pwd)"
manifest="$zsv_root/zebra/dev-harness/Cargo.toml"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$zsv_root/target/zebra-dev}"
cargo nextest run --locked --manifest-path "$manifest"
cargo fmt --manifest-path "$manifest" --all -- --check
cargo clippy --locked --manifest-path "$manifest" --all-targets -- -D warnings
scenario_output="$(mktemp)"
trap 'rm -f "$scenario_output"' EXIT
cargo run --locked --manifest-path "$manifest" --bin scenarios > "$scenario_output"
cmp "$zsv_root/zebra/dev-harness/scenario-output.json" "$scenario_output"
