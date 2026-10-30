#!/usr/bin/env bash
set -euo pipefail
zsv_root="$(cd "$(dirname "$0")/.." && pwd)"
zsv_vectors="$zsv_root/zips/draft-ijaack-zsv"
python3 "$zsv_vectors/check_financial_vectors.py"
python3 "$zsv_vectors/check_wire_vectors.py"
python3 "$zsv_vectors/check_oracle_vector.py"
python3 "$zsv_root/scripts/check_package.py"
git -C "$zsv_root" diff --check
