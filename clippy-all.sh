#!/usr/bin/env bash
set -uo pipefail

TARGET="x86_64-pc-windows-gnu"
FAILED=()
FIRST_ERR=""

run_clippy() {
    local label="$1"; shift
    echo "=== ${label} ==="
    if cargo clippy "$@" -- -D warnings 2>&1 | tee /tmp/clippy-output.txt; then
        echo "✓ ${label} PASSED"
    else
        FAILED+=("${label}")
        local first
        first=$(grep -m1 '^error\b' /tmp/clippy-output.txt | head -1)
        [ -z "$FIRST_ERR" ] && [ -n "$first" ] && FIRST_ERR="$first"
        echo "✗ ${label} FAILED"
    fi
    echo ""
}

run_clippy "cargo clippy --workspace (${TARGET})" --workspace --target "${TARGET}"
run_clippy "cargo clippy --workspace (host)"        --workspace

if [ ${#FAILED[@]} -eq 0 ]; then
    echo "All clippy checks passed."
else
    echo ""
    echo "===== CLIPPY FAILURE SUMMARY ====="
    echo "Failed checks: ${FAILED[*]}"
    [ -n "$FIRST_ERR" ] && echo "First error: ${FIRST_ERR}"
    exit 1
fi
