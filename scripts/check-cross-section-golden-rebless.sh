#!/usr/bin/env bash
# §13.N close-out gate: any commit that touches the §05 GPU pipeline
# (shaders / pipeline / prepare / window_renderer) MUST NOT in the same
# commit re-bless §13 kitty golden PNGs. Re-blessing must be a SEPARATE
# commit so the operator visual-diff review can attribute pixel changes
# to §05 (font/rasterizer) versus §13 (placement/z-order/quadrant).
#
# Failing this gate forces the two-commit discipline:
#   1. Land the §05 code change.
#   2. Re-bless the goldens in a follow-up commit with operator
#      visual-confirmation that pixel shifts trace to §05, not §13.
#
# Usage:
#   scripts/check-cross-section-golden-rebless.sh           # check HEAD vs HEAD~1
#   scripts/check-cross-section-golden-rebless.sh <ref>     # check <ref> vs <ref>~1
set -euo pipefail

ref="${1:-HEAD}"
prev="${ref}~1"

if ! git rev-parse --verify "${prev}" >/dev/null 2>&1; then
    echo "SKIP: ${prev} not reachable — first commit on branch?"
    exit 0
fi

changed=$(git diff --name-only "${prev}" "${ref}" --)

gpu_re='^oriterm/src/gpu/(shaders|pipeline|prepare|window_renderer)/'
golden_re='^oriterm/tests/references/kitty_.*\.png$'

gpu_files=$(printf '%s\n' "${changed}" | grep -E "${gpu_re}" || true)
golden_files=$(printf '%s\n' "${changed}" | grep -E "${golden_re}" || true)

if [ -n "${gpu_files}" ] && [ -n "${golden_files}" ]; then
    echo "FAIL: commit ${ref} touches BOTH §05 GPU pipeline AND §13 kitty goldens."
    echo
    echo "§05 GPU pipeline files in this commit:"
    printf '  %s\n' ${gpu_files}
    echo
    echo "§13 kitty golden PNGs in this commit:"
    printf '  %s\n' ${golden_files}
    echo
    echo "Rule: §13.N close-out gate forbids re-blessing kitty goldens in the"
    echo "same commit as §05 code changes. Split into two commits so operator"
    echo "visual-diff can attribute pixel changes to the correct section."
    exit 1
fi

exit 0
