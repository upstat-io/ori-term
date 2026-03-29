#!/usr/bin/env bash
# Synchronize version across all workspace Cargo.toml files.
#
# Single source of truth: BUILD_NUMBER file at the repo root.
# Format: 0.2.0-alpha.20260329 (already valid SemVer for Cargo)
#
# Usage:
#   ./scripts/sync-version.sh         # Update all version files
#   ./scripts/sync-version.sh --check # Check if versions are in sync (for CI)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
BUILD_FILE="$ROOT_DIR/BUILD_NUMBER"

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

CHECK_MODE=false
if [[ "${1:-}" == "--check" ]]; then
    CHECK_MODE=true
fi

get_build_number() {
    if [[ ! -f "$BUILD_FILE" ]]; then
        echo -e "${RED}ERROR${NC}: BUILD_NUMBER file not found at $BUILD_FILE" >&2
        echo "Run ./scripts/bump-build.sh to create it." >&2
        exit 1
    fi
    tr -d '[:space:]' < "$BUILD_FILE"
}

# Update the workspace version in root Cargo.toml
update_workspace_version() {
    local version="$1"
    local file="$ROOT_DIR/Cargo.toml"
    local current
    current=$(sed -n '/\[workspace\.package\]/,/^\[/{ s/^version[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p }' "$file")

    if [[ "$current" != "$version" ]]; then
        if $CHECK_MODE; then
            echo -e "${RED}MISMATCH${NC}: $file has version '$current', expected '$version'"
            return 1
        else
            sed -i "s/^version = \"[^\"]*\"/version = \"$version\"/" "$file"
            echo -e "${GREEN}UPDATED${NC}: $file -> $version"
        fi
    else
        echo -e "${GREEN}OK${NC}: $file ($version)"
    fi
}

main() {
    local version
    version=$(get_build_number)
    echo "BUILD_NUMBER: $version"
    echo ""

    local failed=false

    echo "=== Workspace Cargo.toml ==="
    update_workspace_version "$version" || failed=true

    echo ""

    if $failed; then
        echo -e "${RED}Version sync check failed!${NC}"
        echo "Run './scripts/sync-version.sh' to fix."
        exit 1
    fi

    if $CHECK_MODE; then
        echo -e "${GREEN}All versions in sync!${NC}"
    else
        echo -e "${GREEN}Version sync complete!${NC}"
    fi
}

main
