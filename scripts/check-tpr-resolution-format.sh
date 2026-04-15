#!/usr/bin/env bash
# Verify that resolved TPR findings in plan files follow the canonical
# "title + Resolved line only" format per .claude/skills/tpr-review/SKILL.md
# §"Resolution format".
#
# When a `- [ ]` TPR finding is fixed and marked `- [x]`, its original
# Evidence / Impact / Required plan update / Basis / Confidence / Citations
# lines must be dropped. The Resolved line is the canonical body of the
# closed finding. Leaving the initial body lines produces drift — the
# finding's diagnostic text is preserved twice (once on the original filing,
# once implied by the Resolved note) and subsequent TPR rounds routinely
# flag the stale lines as format drift.
#
# This script scans `plans/**/section-*.md` for `## NN.R Third Party Review
# Findings` blocks and reports any resolved entry whose continuation lines
# include fields other than `Resolved:`.
#
# Usage:
#   ./scripts/check-tpr-resolution-format.sh            # check all plan sections
#   ./scripts/check-tpr-resolution-format.sh --fix      # auto-strip the drift lines
#   ./scripts/check-tpr-resolution-format.sh PATH ...   # check explicit files
#   ./scripts/check-tpr-resolution-format.sh --help     # show help
#
# Exit codes:
#   0  no drift found
#   1  drift found (or --fix applied changes — rerun to confirm clean)
#   2  usage error

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# The banned continuation fields — permitted only on `- [ ]` open findings.
# Once a finding is marked `- [x]`, these lines must be dropped; the
# Resolved note is the canonical body.
BANNED_FIELDS_RE='^[[:space:]]+(Evidence|Impact|Required plan update|Basis|Confidence|Citations|Agreement):'

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

usage() {
    sed -n 's/^# \{0,1\}//p' "$0" | sed -n '/^Verify/,/^Exit codes:/p' | head -n -1
}

FIX_MODE=false
EXPLICIT_PATHS=()

while [[ $# -gt 0 ]]; do
    case "$1" in
        --help|-h)
            usage
            exit 0
            ;;
        --fix)
            FIX_MODE=true
            shift
            ;;
        --)
            shift
            EXPLICIT_PATHS+=("$@")
            break
            ;;
        -*)
            echo "Error: unknown flag '$1'" >&2
            usage >&2
            exit 2
            ;;
        *)
            EXPLICIT_PATHS+=("$1")
            shift
            ;;
    esac
done

# Collect plan files to scan.
files=()
if [[ ${#EXPLICIT_PATHS[@]} -gt 0 ]]; then
    files=("${EXPLICIT_PATHS[@]}")
else
    # Default: every section-*.md under plans/ (active + completed).
    while IFS= read -r f; do
        files+=("$f")
    done < <(find "$ROOT_DIR/plans" -type f -name 'section-*.md' 2>/dev/null | sort)
fi

if [[ ${#files[@]} -eq 0 ]]; then
    echo "No plan section files to check." >&2
    exit 0
fi

# Scan one file. Writes drift reports to stdout, returns 0 if clean.
# When $FIX_MODE=true, rewrites the file in-place with drift lines removed.
check_file() {
    local file="$1"
    local awk_out
    awk_out="$(
        awk -v file="$file" -v banned_re="$BANNED_FIELDS_RE" '
        BEGIN {
            insec = 0
            in_resolved = 0
            current_id = ""
            drift = 0
        }
        # Enter / leave a §NN.R Third Party Review Findings block.
        /^## [0-9]+\.R[[:space:]]+Third Party Review Findings/ { insec = 1; next }
        /^## / && insec {
            insec = 0
            in_resolved = 0
        }
        /^---$/ && insec {
            # Horizontal rule does not end the block by itself, but does
            # end a resolved entry if one is open.
            in_resolved = 0
        }
        # Start of a new resolved entry (`- [x] `).
        insec && /^- \[x\] / {
            # Extract the tag id (`[TPR-...]`) from the header for
            # diagnostic output.
            match($0, /`\[[^]]+\]`/)
            current_id = substr($0, RSTART, RLENGTH)
            gsub(/`/, "", current_id)
            in_resolved = 1
            next
        }
        # Start of a new open entry (`- [ ] `) closes any resolved scope.
        insec && /^- \[ \] / {
            in_resolved = 0
            current_id = ""
            next
        }
        # Blank line ends a resolved entry.
        insec && in_resolved && /^$/ {
            in_resolved = 0
            current_id = ""
            next
        }
        # New top-level list item closes scope.
        insec && in_resolved && /^- / {
            in_resolved = 0
            current_id = ""
            next
        }
        # Inside a resolved entry: continuation lines must start with
        # `Resolved:` (after whitespace). Anything else matching the
        # banned-fields regex is drift.
        insec && in_resolved && $0 ~ banned_re {
            drift = 1
            printf("%s:%d:%s: drift line under resolved entry — %s\n",
                   file, NR, current_id, $0)
        }
        END { exit drift }
        ' "$file" 2>&1 || true
    )"

    if [[ -n "$awk_out" ]]; then
        echo "$awk_out"
        return 1
    fi
    return 0
}

# Fix one file by stripping drift lines under `- [x]` TPR entries.
fix_file() {
    local file="$1"
    python3 - "$file" <<'PY'
import re
import sys

path = sys.argv[1]
with open(path, "r", encoding="utf-8") as f:
    lines = f.readlines()

banned = re.compile(
    r"^\s+(Evidence|Impact|Required plan update|Basis|Confidence|Citations|Agreement):"
)

out = []
in_sec = False
in_resolved = False
for line in lines:
    if re.match(r"^## \d+\.R\s+Third Party Review Findings", line):
        in_sec = True
        in_resolved = False
        out.append(line)
        continue
    if in_sec and line.startswith("## "):
        in_sec = False
        in_resolved = False
        out.append(line)
        continue
    if in_sec and line.startswith("- [x] "):
        in_resolved = True
        out.append(line)
        continue
    if in_sec and line.startswith("- [ ] "):
        in_resolved = False
        out.append(line)
        continue
    if in_sec and in_resolved and line.strip() == "":
        in_resolved = False
        out.append(line)
        continue
    if in_sec and in_resolved and line.startswith("- "):
        in_resolved = False
        out.append(line)
        continue
    if in_sec and in_resolved and banned.match(line):
        # Drop the line.
        continue
    out.append(line)

with open(path, "w", encoding="utf-8") as f:
    f.writelines(out)
PY
}

drift_total=0
fixed_files=()

for file in "${files[@]}"; do
    if [[ ! -f "$file" ]]; then
        echo "Warning: $file not found, skipping" >&2
        continue
    fi
    if [[ "$FIX_MODE" == true ]]; then
        if ! check_file "$file" >/dev/null 2>&1; then
            fix_file "$file"
            fixed_files+=("$file")
            # Re-verify.
            if check_file "$file" >/dev/null 2>&1; then
                echo -e "${GREEN}fixed${NC}: $file"
            else
                echo -e "${RED}partial${NC}: $file (some drift remains after auto-fix)"
                drift_total=1
            fi
        fi
    else
        if ! check_file "$file"; then
            drift_total=1
        fi
    fi
done

if [[ "$FIX_MODE" == true ]]; then
    if [[ ${#fixed_files[@]} -eq 0 ]]; then
        echo -e "${GREEN}All TPR resolution blocks follow the canonical format.${NC}"
        exit 0
    fi
    echo ""
    echo -e "${YELLOW}Rewrote ${#fixed_files[@]} file(s). Re-run without --fix to confirm clean.${NC}"
    exit 1
fi

if [[ "$drift_total" -eq 0 ]]; then
    echo -e "${GREEN}All TPR resolution blocks follow the canonical format.${NC}"
    exit 0
fi

echo ""
echo -e "${RED}TPR resolution format drift found.${NC}"
echo "Fix manually by removing the flagged lines, or run with --fix to strip automatically."
echo "Canonical format: each '- [x]' TPR entry has one continuation line starting with 'Resolved:'."
exit 1
