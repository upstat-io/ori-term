portable-pty (vendored by ori_term)
===================================

Rust abstraction over native PTY APIs (Unix `openpty`, Windows `ConPTY`). Consumed by `oriterm_mux` for child-process I/O on the terminal IO thread.

## Vendored patches (ori_term)

This crate is a vendored fork of upstream `portable-pty`, patched for oriterm-specific transport requirements. Patches are marked in source with a `// VENDORED PATCH (oriterm): ...` breadcrumb naming the owning roadmap section or bug.

### ConPTY pipe buffer: 128 KiB → 8 MiB

- **File**: `src/win/conpty.rs::ConPtySystem::openpty`
- **Change**: `OverlappedPipe::new(PipeAccess::Duplex, 8 * 1024 * 1024)` (was `128 * 1024`).
- **Reason**: notcurses-class graphics-flood producers (`notcurses-demo xray`, `intro`) issue initial-frame transmits of ~3 MB and compressed delta-frames of ~256 KB. Pre-patch the 128 KiB pipe blocked the producer on conhost's read-rate ceiling, and notcurses' deadline-based frame dropper registered every blocked write as a dropped frame, producing operator-visible scene lag. The 8 MiB buffer absorbs the initial transmit + back-pressures only when conhost actually falls behind, removing the first-tier transport ceiling. conhost's own internal buffer is the next-tier limit; that remains upstream-owned.
- **Discovered**: BUG-06-086 cycle 3 (measured: 11.4 FPS → 28.9 FPS on `notcurses-demo xray`).
- **Upstream**: not yet proposed — buffer size is hardcoded in upstream and changing it has cross-cutting implications for low-throughput callers that rely on smaller buffers for back-pressure responsiveness.

## Rebase discipline

When pulling a new upstream release of `portable-pty`:

1. Re-apply each `VENDORED PATCH (oriterm)` block from this README to the new tree.
2. Verify the buffer-size change at `src/win/conpty.rs::openpty` still has the same surface (no upstream refactor that moved or renamed it).
3. Run `./test-all.sh` and the operator-side `notcurses-demo xray` regression to confirm the cure measurements still hold.
