#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
font_cache="$(mktemp -d)"
trap 'rm -rf -- "$font_cache"' EXIT
XDG_CACHE_HOME="$font_cache" dot -Tpng -Gdpi=160 docs/architecture.dot -o docs/architecture.png
pandoc docs/whitepaper.md --resource-path=docs --standalone --number-sections \
  --pdf-engine=pdflatex -o docs/whitepaper.pdf
