#!/usr/bin/env bash
set -euo pipefail

TARGET="x86_64-pc-windows-gnu"

echo "=== cargo build --workspace (${TARGET}, debug) ==="
cargo build --workspace --target "${TARGET}"

echo ""
echo "=== cargo build --workspace (${TARGET}, release) ==="
cargo build --workspace --target "${TARGET}" --release

echo ""
echo "=== prose-lint (term_repo) ==="
if ! python3 scripts/prose-lint.py .; then
    echo "prose-lint failed — see violations above."
    exit 1
fi

echo ""
echo "Build succeeded (debug + release + prose-lint clean)."
