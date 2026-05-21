#!/usr/bin/env bash
set -euo pipefail

TARGET="x86_64-pc-windows-gnu"

echo "=== cargo build --workspace (${TARGET}, debug) ==="
cargo build --workspace --target "${TARGET}"

echo ""
echo "=== cargo build --workspace (${TARGET}, release) ==="
cargo build --workspace --target "${TARGET}" --release

echo ""
# Sideloaded ConPTY staging — copy `tools/conpty/{OpenConsole.exe,conpty.dll}`
# (wrapper-root sources that survive `cargo clean`) into the target build
# dirs so `crates/portable-pty/src/win/psuedocon.rs`'s sideload lookup finds
# them on next launch. Silent no-op when source files are absent — kernel32
# fallback works for unsideloaded checkouts. Per BUG-07-054 + BUG-06-073.
echo "=== sideloaded ConPTY staging ==="
WRAPPER_ROOT="$(cd .. && pwd)"
CONPTY_SRC="${WRAPPER_ROOT}/tools/conpty"
staged=0
skipped=0
for binary in OpenConsole.exe conpty.dll; do
    src="${CONPTY_SRC}/${binary}"
    if [ ! -f "$src" ]; then
        printf '  SKIP: %s missing from tools/conpty/ (run tools/conpty/stage-from-source.sh once to populate)\n' "$binary"
        skipped=$((skipped + 1))
        continue
    fi
    for profile in debug release; do
        dest_dir="target/${TARGET}/${profile}"
        [ -d "$dest_dir" ] || continue
        dest="${dest_dir}/${binary}"
        if [ -f "$dest" ] && cmp -s "$src" "$dest"; then
            continue
        fi
        cp -f "$src" "$dest"
        staged=$((staged + 1))
        printf '  STAGED: %s\n' "$dest"
    done
done
if [ "$staged" -eq 0 ] && [ "$skipped" -gt 0 ]; then
    printf '  (kernel32 built-in ConPTY will be used at runtime — see tools/conpty/README.md)\n'
elif [ "$staged" -gt 0 ]; then
    printf '  (sideloaded ConPTY active for both debug and release Windows binaries)\n'
fi

echo ""
echo "=== prose-lint (term_repo) ==="
if ! python3 scripts/prose-lint.py .; then
    echo "prose-lint failed — see violations above."
    exit 1
fi

echo ""
echo "Build succeeded (debug + release + prose-lint clean)."
