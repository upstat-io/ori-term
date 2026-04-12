#!/usr/bin/env bash
# Spec Conformance — Manifest Fetch Script
#
# Usage:
#   bash manifest-fetch.sh            # Fetch all non-redistributable specs to cache
#   bash manifest-fetch.sh --verify   # Verify committed redistributable specs
#
# Fetched specs land in ~/.cache/ori_term/specs/ (not committed).
# Redistributable specs live in plans/spec-conformance/specs/ (committed).
#
# Cross-platform: POSIX shell for Linux/macOS. Windows: run via WSL or Git Bash.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
MANIFEST="$SCRIPT_DIR/manifest.toml"
CACHE_DIR="${HOME}/.cache/ori_term/specs"
VERIFY_ONLY=false
ERRORS=0

if [[ "${1:-}" == "--verify" ]]; then
    VERIFY_ONLY=true
fi

# Minimal TOML parser — extracts spec entries from manifest.toml.
# Reads [specs.<key>] blocks and extracts url, local_path, sha256,
# redistributable fields.
parse_manifest() {
    local current_key=""
    local url="" local_path="" sha256="" redistributable=""

    while IFS= read -r line; do
        # Strip comments and leading/trailing whitespace
        line="${line%%#*}"
        line="$(echo "$line" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
        [[ -z "$line" ]] && continue

        # New section header
        if [[ "$line" =~ ^\[specs\.([a-zA-Z0-9_]+)\]$ ]]; then
            # Emit previous entry if complete
            if [[ -n "$current_key" ]]; then
                echo "$current_key|$url|$local_path|$sha256|$redistributable"
            fi
            current_key="${BASH_REMATCH[1]}"
            url="" local_path="" sha256="" redistributable=""
            continue
        fi

        # Skip non-specs sections
        if [[ "$line" =~ ^\[.*\]$ ]]; then
            if [[ -n "$current_key" ]]; then
                echo "$current_key|$url|$local_path|$sha256|$redistributable"
            fi
            current_key=""
            continue
        fi

        [[ -z "$current_key" ]] && continue

        # Key-value pairs
        case "$line" in
            url\ =\ *)           url="$(echo "$line" | sed 's/^url = "//;s/"$//')" ;;
            local_path\ =\ *)    local_path="$(echo "$line" | sed 's/^local_path = "//;s/"$//')" ;;
            sha256\ =\ *)        sha256="$(echo "$line" | sed 's/^sha256 = "//;s/"$//')" ;;
            redistributable\ =\ *) redistributable="$(echo "$line" | sed 's/^redistributable = //')" ;;
        esac
    done < "$MANIFEST"

    # Emit last entry
    if [[ -n "$current_key" ]]; then
        echo "$current_key|$url|$local_path|$sha256|$redistributable"
    fi
}

verify_sha256() {
    local file="$1" expected="$2"
    if [[ -z "$expected" ]]; then
        echo "  SKIP (no sha256 in manifest)"
        return 0
    fi
    local actual
    actual="$(sha256sum "$file" | awk '{print $1}')"
    if [[ "$actual" == "$expected" ]]; then
        echo "  OK ($actual)"
        return 0
    else
        echo "  FAIL: expected $expected, got $actual"
        return 1
    fi
}

echo "=== Spec Corpus Manifest ==="
echo "Mode: $(if $VERIFY_ONLY; then echo 'verify'; else echo 'fetch'; fi)"
echo

while IFS='|' read -r key url local_path sha256 redistributable; do
    if $VERIFY_ONLY; then
        # --verify mode: only check committed redistributable specs
        if [[ "$redistributable" == "true" ]]; then
            committed_path="$SCRIPT_DIR/$(basename "$local_path")"
            echo "[$key] $committed_path"
            if [[ -f "$committed_path" ]]; then
                if ! verify_sha256 "$committed_path" "$sha256"; then
                    ERRORS=$((ERRORS + 1))
                fi
            else
                echo "  MISSING (not committed yet)"
            fi
        fi
    else
        # Fetch mode: download non-redistributable specs to cache
        if [[ "$redistributable" == "false" ]]; then
            cache_path="$CACHE_DIR/$(basename "$local_path")"
            echo "[$key] $url"
            if [[ -f "$cache_path" ]]; then
                echo "  cached: $cache_path"
                if [[ -n "$sha256" ]]; then
                    verify_sha256 "$cache_path" "$sha256" || ERRORS=$((ERRORS + 1))
                fi
            else
                mkdir -p "$CACHE_DIR"
                echo "  fetching → $cache_path"
                if curl -sSL --fail -o "$cache_path" "$url" 2>/dev/null; then
                    echo "  downloaded"
                    if [[ -n "$sha256" ]]; then
                        verify_sha256 "$cache_path" "$sha256" || ERRORS=$((ERRORS + 1))
                    fi
                else
                    echo "  FAIL: download failed from $url"
                    ERRORS=$((ERRORS + 1))
                fi
            fi
        fi
    fi
done < <(parse_manifest)

echo
if [[ $ERRORS -gt 0 ]]; then
    echo "FAILED: $ERRORS error(s)"
    exit 1
else
    echo "OK: all checks passed"
    exit 0
fi
