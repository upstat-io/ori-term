---
plan: "threaded-io"
title: "Threaded Terminal IO: Exhaustive Implementation Plan"
status: complete
supersedes: []
references:
  - "plans/bug-tracker/section-06-rendering-perf.md"
---

# Threaded Terminal IO: Exhaustive Implementation Plan

## Mission

Eliminate resize flashing, text repositioning, and cursor jumping during window resize by replacing the FairMutex shared-state model with a thread-separated architecture where a dedicated Terminal IO Thread owns all terminal state exclusively. The renderer reads lock-free snapshots produced by the IO thread, ensuring it never sees intermediate reflow states. This also resolves BUG-06.2 (garbled text after resize during key repeat).

## Architecture

### Current (FairMutex model)

```
PTY Reader Thread ──────┐
  read() → lease() →    │
  try_lock() → parse VTE │
  → unlock → wakeup      │    Arc<FairMutex<Term<T>>>
                          ├──►  (contended)
Main Thread ─────────────┘
  resize: lock → reflow → unlock
  render: lock → snapshot → unlock → GPU
```

**Problem**: During drag resize, the main thread synchronously reflows the grid under lock, then immediately renders. Every intermediate column width (83→82→81→...) gets a visible frame.

### Proposed (Terminal IO Thread model)

```
                    ┌─────────────┐
                    │   PTY FD    │
                    └──────┬──────┘
                           │ read() [blocking]
                    ┌──────▼──────┐
                    │ PTY Reader  │ (existing thread, simplified)
                    │ Thread      │ forwards raw bytes only
                    └──────┬──────┘
                           │ byte channel
    Commands         ┌─────▼─────────────┐      Snapshot
    (Resize,  ──────►│ Terminal IO Thread │────► SnapshotDoubleBuffer
     Scroll,         │ (NEW — owns Term)  │      (Mutex-guarded flip)
     Theme)          │ VTE parse + reflow │      + wakeup
    cmd channel      │ produce snapshot   │
                     └──────┬────────────┘
                            │ PTY resize (SIGWINCH)
                     ┌──────▼──────┐
                     │   PTY FD    │
                     └─────────────┘

    PTY Writer Thread (unchanged):
      receives Msg::Input from main thread → writes to PTY

    Main Thread (winit event loop):
    ├─ Sends commands via channel (non-blocking)
    ├─ Coalesces rapid resize events (only latest size sent)
    ├─ Reads latest snapshot from shared buffer (brief lock)
    ├─ Renders frame from snapshot (GPU)
    └─ Keyboard input → PTY writer thread (unchanged)
```

**Key invariant (after section 07)**: The main thread NEVER touches `Term`, `Grid`, or any mutable terminal state. It only reads completed snapshots and sends commands. During the transition period (sections 02-06), the main thread still locks the old `Arc<FairMutex<Term>>` for non-render operations.

### Thread ownership

| Thread | Owns | Communicates via |
|--------|------|-----------------|
| PTY Reader | PTY read fd, 1MB buffer | byte channel → IO thread |
| Terminal IO | `Term<T>`, VTE processors, `PtyControl` | cmd channel ← main; snapshot buffer → main; wakeup callback → main |
| PTY Writer | PTY write fd | msg channel ← main (unchanged) |
| Main (winit) | GPU, windows, session, widgets | cmd channel → IO; snapshot buffer ← IO |

### Where this lives in the crate hierarchy

All changes are internal to `oriterm_mux`. The `MuxBackend` trait boundary is preserved — `oriterm` sends the same `resize_pane_grid()`, `refresh_pane_snapshot()` calls. The difference is that these now route through channels instead of locking the terminal. Note: `MuxBackend` has three implementors — `EmbeddedMux`, `DaemonMux` (client/rpc_methods.rs), and `InProcessMux` (server). Trait signature changes (e.g. section 06.1 `scroll_to_*_prompt() -> bool` to `-> ()`) must update all three simultaneously.

```
oriterm_mux/src/pane/
├── mod.rs              ← Pane struct (refactored: no more Arc<FairMutex>)
├── io_thread/
│   ├── mod.rs          ← PaneIoThread, main loop
│   ├── commands.rs     ← PaneIoCommand enum
│   ├── event_proxy/
│   │   ├── mod.rs      ← IoThreadEventProxy (EventListener for IO thread's Term)
│   │   └── tests.rs
│   ├── snapshot/
│   │   ├── mod.rs      ← SnapshotDoubleBuffer, snapshot production
│   │   └── tests.rs
│   └── tests.rs        ← Thread lifecycle, command delivery
├── selection.rs        ← (unchanged, works from snapshot)
├── mark_cursor.rs      ← (unchanged, works from snapshot)
└── shutdown.rs         ← (updated for IO thread shutdown)
```

## Design Principles

1. **Thread isolation over shared state.** The terminal state has exactly one writer (the IO thread). The renderer has exactly one reader (main thread via snapshot). No mutex contention, no starvation, no priority inversion. Inspired by Ghostty's IO thread mailbox model (`src/termio/Termio.zig`) which eliminates the resize flashing that plagues FairMutex-based terminals.

2. **Incremental migration with continuous correctness.** Each section produces a working system. The FairMutex is not removed until all operations are migrated. Sections 01-04 establish the new path while the old path remains as fallback. Section 07 removes the old path after all operations are proven.

3. **Preserve performance invariants.** Zero idle CPU (ControlFlow::Wait), zero allocations in hot render path (buffer reuse via swap), stable RSS (scrollback bounded). The IO thread reuses `RenderableContent` buffers across frames. The main thread swaps (not copies) snapshot data. The existing `maybe_shrink()` discipline applies to the shared buffer.

## Section Dependency Graph

```
  01 (IO Thread Scaffold)
   │
   ├──► 02 (VTE Parsing Migration)
   │     │
   │     └──► 03 (Snapshot Production)
   │           │
   │           └──► 04 (Render Pipeline Migration)
   │                 │
   │                 ▼
   │                05 (Resize)
   │                 │
   │                 ▼
   │                06 (Remaining Ops)
   │                 │
   │                 ▼
   │   07 (Lifecycle & FairMutex Removal)
   │          │
   │          ▼
   └──► 08 (Verification)
```

- Sections 01→02→03→04 are strictly sequential (each builds on the previous).
- Sections 05 and 06 both require 04. They CAN be implemented in parallel but section 05 moves `PtyControl` to the IO thread, which section 06.6 (daemon dispatch resize) must account for. Implement 05 first, then 06.
- Section 07 requires ALL of 05 + 06 (all operations migrated before removing FairMutex).
- Section 08 requires 07 (full system verification).

**Cross-section interactions:**
- **Section 02 + 03**: VTE parsing produces terminal state changes; snapshot production reads them. The wakeup signal must be sent AFTER the snapshot is published (not during parsing like today).
- **Section 05 then 06 (ordered, not parallel)**: Section 05 moves `PtyControl` to the IO thread for resize. Section 06.6 (daemon dispatch) must account for this. Section 05 also changes the IO thread's grid dimensions, making the old `Term`'s grid stale. Section 06's scroll operations on the old `Term` would use stale dimensions for `display_offset` clamping. Implement 05 first, then 06 immediately after.
- **Section 06.1 + 06.3**: The `MuxBackend` trait signature change (`scroll_to_*_prompt() -> bool` to `-> ()`) must be applied to all three implementors simultaneously: `EmbeddedMux`, `DaemonMux` (client/rpc_methods.rs), and `InProcessMux` (if applicable). Do this in section 06.1 when scroll operations are migrated.

## Implementation Sequence

```
Phase 0 - Foundation
  └─ Section 01: Terminal IO Thread scaffold
     (PaneIoThread struct, command enum, channel, basic loop)

Phase 1 - Migration
  └─ Section 02: VTE Parsing on IO Thread (dual-Term)
     (reader keeps parsing AND adds byte forwarding; IO thread gets second Term + VTE parsing)
  └─ Section 03: Snapshot production on IO thread
     (IO thread builds RenderableContent, shared buffer, wakeup)
  Gate: IO thread produces valid snapshots from PTY output

Phase 2 - Consumption
  └─ Section 04: Render pipeline reads from IO thread
     (EmbeddedMux reads shared buffer, not FairMutex)
  Gate: Terminal renders correctly from IO-thread snapshots

Phase 3 - Payoff  [CRITICAL PATH]
  └─ Section 05: Resize flows through IO thread
     (async command, coalescing, no main-thread reflow)
  └─ Section 06: All remaining operations via commands (depends on 05)
     (scroll, search, theme, text extract, mark mode)
  Gate: All operations route through IO commands. Old parsing still active (dual-Term).

Phase 4 - Cleanup
  └─ Section 07: Pane lifecycle refactor, FairMutex removal
     (Pane no longer holds Arc<FairMutex>, clean shutdown)

Phase 5 - Verification
  └─ Section 08: Performance, cross-platform, regression
     (alloc regression, idle CPU, resize stress, BUG-06.2)
```

**Why this order:**
- Phase 0-1 are additive — the old FairMutex path still works alongside the new one. Two `Term` instances per pane during transition (dual parsing, doubled CPU cost — acceptable temporarily).
- Phase 2 switches the render path. The old parsing path and `Arc<FairMutex<Term>>` remain active for non-render operations (scroll, search, text extraction, mark mode).
- Phase 3 migrates remaining operations in order: resize first (05), then everything else (06). Section 05 moves `PtyControl` to the IO thread and changes grid dimensions on the IO thread's `Term`, making the old `Term`'s grid stale. Section 06 must follow (not run in parallel) because scroll/search on the old `Term` would use stale dimensions.
- Phase 4 removes `Arc<FairMutex<Term>>`, old `PtyEventLoop` parsing code, and FairMutex.
- Phase 5 validates everything end-to-end.

**Critical phasing constraint**: The old `PtyEventLoop` parsing CANNOT be disabled until ALL operations are migrated (phase 4). Operations that lock the old `Term` (scroll, search, extract_text, etc.) would get stale state if the old `Term` stops receiving bytes.

**Known failing tests (expected until plan completion):**
- None expected during phases 0-3 (old path preserved and active).
- During phase 4, tests that call `pane.terminal().lock()` directly will need updating.

## Metrics (Current State)

| Crate | Production LOC | Test LOC | Total |
|-------|---------------|----------|-------|
| `oriterm_mux` | ~9,100 | ~7,400 | ~16,500 |
| `oriterm_core` (sync module) | ~175 | ~563 | ~738 |
| `oriterm` (chrome module) | ~675 | ~486 | ~1,161 |
| **Total affected** | **~9,950** | **~8,449** | **~18,399** |

## Estimated Effort

| Section | Est. Lines (prod) | Est. Lines (test) | Complexity | Depends On |
|---------|-------------------|-------------------|------------|------------|
| 01 IO Thread Scaffold | ~300 | ~150 | Medium | — |
| 02 VTE Parsing Migration | ~250 | ~200 | Medium | 01 |
| 03 Snapshot Production | ~250 | ~150 | Medium | 02 |
| 04 Render Pipeline Migration | ~250 | ~150 | Medium | 03 |
| 05 Resize Pipeline Migration | ~150 | ~150 | High | 04 |
| 06 Remaining Operations | ~500 | ~250 | Medium-High | 05 |
| 07 Lifecycle & FairMutex Removal | ~400 | ~200 | High | 05, 06 |
| 08 Verification | ~200 | ~300 | Medium | 07 |
| **Total new** | **~2,300** | **~1,550** | | |

## Known Bugs (Pre-existing)

| Bug | Root Cause | Fix Location | Status |
|-----|-----------|-------------|--------|
| BUG-06.2: Garbled text after resize during key repeat | Race between queued key repeat events and PTY resize (SIGWINCH) — shell processes both simultaneously | Section 05 (resize via IO thread eliminates race) | Resolved |
| Resize flashing / text jumping during drag resize | Synchronous reflow + immediate render exposes every intermediate grid state | Section 05 (renderer only sees completed snapshots) | Resolved |

## Quick Reference

| ID | Title | File | Status |
|----|-------|------|--------|
| 01 | Terminal IO Thread Scaffold | `section-01-io-thread-scaffold.md` | Complete |
| 02 | VTE Parsing Migration | `section-02-vte-migration.md` | Complete |
| 03 | Snapshot Production & Transfer | `section-03-snapshot-production.md` | Complete |
| 04 | Render Pipeline Migration | `section-04-render-migration.md` | Complete |
| 05 | Resize Pipeline Migration | `section-05-resize-migration.md` | Complete |
| 06 | Remaining State Operations | `section-06-remaining-ops.md` | Complete |
| 07 | Pane Lifecycle & FairMutex Removal | `section-07-lifecycle-cleanup.md` | Complete |
| 08 | Verification | `section-08-verification.md` | Complete |
