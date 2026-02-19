#!/usr/bin/env bash
set -euo pipefail

echo "=== cargo test --workspace --features oriterm/gpu-tests ==="
cargo test --workspace --features oriterm/gpu-tests

echo ""
echo "All tests passed."
