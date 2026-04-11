---
paths:
  - "oriterm_mux/src/**"
  - "oriterm_mux/tests/**"
---

# oriterm_mux — Pane Server

The canonical home for the pane server: pane lifecycle (create, resize, close), Terminal IO thread, snapshot double-buffer, PTY I/O, PaneRegistry (flat pane storage), MuxBackend trait (embedded + daemon), daemon server, wire protocol (PDU codec), and the ID types `PaneId`, `DomainId`, `ClientId`. Depends on `oriterm_core` and `oriterm_ipc` only — see `.claude/rules/crate-boundaries.md` for the allowed dependency direction.

`oriterm_mux` is a **flat pane-only server**. It does not know about tabs, windows, sessions, layouts, or any presentation concern — those live in `oriterm`'s session model. The mux's job is to own a Pane and pump bytes between the PTY and the consumer.

## Terminal IO Thread Ownership Model

Every pane has a dedicated Terminal IO thread that **owns `Term` exclusively**. The main thread never touches `Term` directly — it reads lock-free snapshots via `SnapshotDoubleBuffer`.

**Who owns what:**
- **IO thread** (`oriterm_mux/src/pane/io_thread/`): owns `Term`, VTE parser state, reflow, snapshot production, command processing. All terminal-state mutations happen here.
- **Main thread**: reads snapshots via `SnapshotDoubleBuffer::swap_front()`, dispatches input commands (keyboard, mouse, paste, resize) via a command channel, never holds a reference into `Term`.

**Command channel contract**: commands are sent from the main thread to the IO thread via a bounded channel. The IO thread drains commands between VTE parse batches. Never block the main thread waiting for a command to complete — the main thread must be able to render while commands are in flight.

## Snapshot Double-Buffer Discipline

`SnapshotDoubleBuffer` (`oriterm_mux/src/pane/io_thread/snapshot/`) is the lock-free transfer mechanism between the IO thread and the main thread. **The discipline is load-bearing** — violations introduce allocation regressions in the hot render path or race conditions in snapshot visibility.

**IO thread side**:
- Write into the back buffer via `renderable_content_into()` (reuses existing capacity)
- Call `flip_swap()` to exchange the back buffer with the front buffer via `std::mem::swap()` — zero allocation
- Never hand out a reference to the back buffer; the main thread only ever sees the front buffer

**Main thread side**:
- Read via `swap_front()` which returns the front buffer by value (pointer swap)
- Use `swap_renderable_content()` to exchange the returned content with the main thread's scratch buffer
- All `Vec` buffers on `RenderableContent` are reused via `.clear()` + capacity retention
- `HashSet` scratch buffers live on `RenderableContent` and are also cleared + reused

**No `Vec::new()` or `Box::new()` per cell or per frame is permitted.** If you need scratch storage, add a field to `RenderableContent` and clear it each frame.

## Pane Lifecycle

1. **Create**: main thread calls `InProcessMux::create_pane()` → spawns IO thread → opens PTY → first snapshot produced → pane is ready
2. **Resize**: main thread sends `Resize { cols, rows }` command → IO thread re-queries PTY size → reflows `Grid` → produces new snapshot
3. **Close**: main thread calls `InProcessMux::close_pane()` → sends `Shutdown` command → IO thread drains any remaining PTY output → closes PTY → thread exits → `PaneId` recycled
4. **Process death**: IO thread detects PTY EOF → transitions to `PaneStatus::Exited(code)` → main thread observes via snapshot read → user decides whether to reuse or close

Every state transition MUST be idempotent. Calling `close_pane()` on an already-closed pane is a no-op, not a panic.

## Exports

`oriterm_mux` exports: `PaneId`, `DomainId`, `ClientId`. It does NOT export `TabId`, `WindowId`, or `SessionId` — those are owned by `oriterm/src/session/`.

## Testing

- **Pane unit tests**: `cargo test -p oriterm_mux` — covers the flat pane server in isolation
- **Threaded IO verification tests**: cover the IO thread ↔ main thread snapshot discipline end-to-end
- **Resize stress**: simulate rapid resize-during-output to catch snapshot visibility races
- **PTY mock tests**: use the in-memory PTY mock for deterministic PTY I/O testing

## Cross-Platform Discipline

Every `#[cfg(target_os = ...)]` branch in `oriterm_mux` (PTY open / read / write / resize) MUST have counterparts for all three supported targets: Linux (`openpty` / `read` / `write` / `TIOCSWINSZ`), macOS (same POSIX API), and Windows (ConPTY / `ReadFile` / `WriteFile` / `ResizePseudoConsole`). Windows cross-compile from WSL must succeed: `cargo build --target x86_64-pc-windows-gnu`.

## Forbidden

- No UI framework types (widgets, layout, interaction) — those live in `oriterm_ui`
- No GPU types (wgpu, shaders, rendering) — those live in `oriterm`
- No session model (tabs, windows, layouts) — that is `oriterm`'s concern
- No window or platform types (winit) — those live in `oriterm`
- No direct `Term` access from the main thread — always go through snapshots
- No allocation in the hot snapshot path — see §Snapshot Double-Buffer Discipline
- No `println!` debugging — use `log` macros
- No `unwrap()` outside of test code
