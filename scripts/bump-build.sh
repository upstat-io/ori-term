#!/usr/bin/env bash
# Derive the build number and write to BUILD_NUMBER.
#
# Format: MAJOR.MINOR.PATCH-alpha.YYYYMMDD
# The major.minor.patch is read from the current BUILD_NUMBER (or Cargo.toml).
# The date suffix is always today's UTC date.
#
# Usage:
#   ./scripts/bump-build.sh          # Write BUILD_NUMBER
#   ./scripts/bump-build.sh --check  # Dry-run: show what it would write

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
BUILD_FILE="$ROOT_DIR/BUILD_NUMBER"

GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
NC='\033[0m'

CHECK_MODE=false
if [[ "${1:-}" == "--check" ]]; then
    CHECK_MODE=true
fi

# Extract the base version (e.g., "0.2.0") from BUILD_NUMBER or Cargo.toml.
# BUILD_NUMBER format: 0.2.0-alpha.20260329
# Cargo.toml format:   0.2.0-alpha.20260329
extract_base_version() {
    local source=""
    if [[ -f "$BUILD_FILE" ]]; then
        source=$(tr -d '[:space:]' < "$BUILD_FILE")
    else
        source=$(sed -n '/\[workspace\.package\]/,/^\[/{ s/^version[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p }' "$ROOT_DIR/Cargo.toml")
    fi

    # Strip everything after the first hyphen: 0.2.0-alpha.20260329 -> 0.2.0
    echo "${source%%-*}"
}

# Extract the stage (alpha, beta, rc) from BUILD_NUMBER or Cargo.toml.
extract_stage() {
    local source=""
    if [[ -f "$BUILD_FILE" ]]; then
        source=$(tr -d '[:space:]' < "$BUILD_FILE")
    else
        source=$(sed -n '/\[workspace\.package\]/,/^\[/{ s/^version[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p }' "$ROOT_DIR/Cargo.toml")
    fi

    # Extract stage: 0.2.0-alpha.20260329 -> alpha
    if [[ "$source" == *-* ]]; then
        local suffix="${source#*-}"
        echo "${suffix%%.*}"
    else
        echo "alpha"
    fi
}

BASE=$(extract_base_version)
STAGE=$(extract_stage)
TODAY=$(date -u +"%Y%m%d")

# Build the version: MAJOR.MINOR.PATCH-STAGE.YYYYMMDD
NEXT="${BASE}-${STAGE}.${TODAY}"

CURRENT="(none)"
if [[ -f "$BUILD_FILE" ]]; then
    CURRENT=$(tr -d '[:space:]' < "$BUILD_FILE")
fi

if $CHECK_MODE; then
    echo -e "${YELLOW}Current${NC}: $CURRENT"
    echo -e "${GREEN}Derived${NC}: $NEXT"
else
    echo "$NEXT" > "$BUILD_FILE"
    echo -e "${GREEN}Build number${NC}: $CURRENT -> $NEXT"
fi
