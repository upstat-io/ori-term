---
plan: "macos-platform-fixes"
title: "macOS Platform Fixes: Chrome, Tear-Off, and Snapshot Latency"
status: complete
references:
  - "plans/roadmap/"
---

# macOS Platform Fixes: Chrome, Tear-Off, and Snapshot Latency

## Mission

Fix three platform regressions that make oriterm non-functional on macOS: Windows-style chrome rendering instead of native traffic lights, broken tab tear-off (gated `#[cfg(target_os = "windows")]` only), and tab-switch hangs caused by synchronous RPC on the UI thread. After this plan, oriterm on macOS should look native, support tab tear-off/merge, and switch tabs without stalling.

## Architecture

```
                    ┌─────────────────────────────┐
                    │      Tab Bar Widget          │
                    │  (oriterm_ui/widgets/tab_bar) │
                    ├──────────┬──────────┬────────┤
  Section 01 ────►  │ controls │  tabs    │ layout │
  Platform gate     │ draw_*() │  draw_*()│        │
  on draw_window_   └────┬─────┴─────┬────┴────────┘
  controls() +           │           │
  downstream call   ────►│ chrome/   │ tab_bar_input.rs
  sites (01.2b)          ▼           ▼
                    ┌─────────────────────────────┐
                    │       App Event Loop         │
                    │    (oriterm/src/app/)         │
                    ├──────────────────────────────┤
  Section 02 ────►  │ redraw → refresh_pane_snap() │  Sync RPC blocks UI
  Non-blocking      │ mux_pump → poll_events()     │
  snapshot          └──────────┬───────────────────┘
                               │
                               ▼
                    ┌─────────────────────────────┐
                    │      Tab Drag Module         │
                    │  (oriterm/src/app/tab_drag/)  │
                    ├──────────────────────────────┤
  Section 03 ────►  │ tear_off.rs  (Windows only)  │  Need macOS impl
  macOS tear-off    │ merge.rs     (Windows only)  │
                    └──────────────────────────────┘
```

## Design Principles

1. **Platform parity**: Every feature that works on Windows must work on macOS and Linux. `#[cfg]` gates must have counterparts for all targets — no "not supported on this platform" log lines.

2. **No blocking on the UI thread**: The render path (`handle_redraw`) must never make synchronous RPC calls. Rendering reads whatever snapshot is available; if none exists, it uses a stale frame or shows a placeholder. The mux pump fills snapshots asynchronously.

3. **Native platform chrome**: macOS uses OS-provided traffic lights via `fullsize_content_view(true)`. Custom window control buttons are Windows/Linux only. The tab bar reserves space via `left_inset` on macOS but must not draw over native controls.

## Section Dependency Graph

```
  01 (Chrome Gate)     02 (Non-Blocking Snapshot)
         │                       │
         │  (independent)        │  (independent)
         ▼                       ▼
         03 (macOS Tear-Off)
         │  depends on 01 for correct chrome metrics
         ▼
         04 (Verification)
         │  depends on all
```

- Sections 01 and 02 are independent and can be worked in any order.
- Section 03 depends on 01 (correct chrome metrics on macOS needed for merge rect computation).
- Section 04 depends on all three.

## Implementation Sequence

```
Phase 1 - Independent fixes (parallel)
  +-- 01: Gate draw_window_controls() to Windows/Linux only
  +-- 02: Replace synchronous RPC fallback with non-blocking path
  Gate: macOS shows native traffic lights; tab switch does not hang

Phase 2 - Platform feature
  +-- 03: Implement macOS tab tear-off using Cocoa APIs
  Gate: Tab tear-off works on macOS with merge detection

Phase 3 - Verification
  +-- 04: Cross-platform test matrix, build-all, clippy-all, test-all
  Gate: All three platforms build and pass tests
```

**Why this order:**
- Phase 1 fixes are independent and unblock visual testing on macOS.
- Phase 2 requires correct chrome metrics from Phase 1.
- Phase 3 validates all changes across platforms.

## New Types and Fields Introduced

| Type/Field | Location | Section | Purpose |
|-----------|----------|---------|---------|
| `pending_refresh: HashSet<PaneId>` | `MuxClient` in `oriterm_mux/src/backend/client/mod.rs` | 02 | Tracks panes awaiting async snapshot delivery; prevents `clear_pane_snapshot_dirty` from clearing dirty prematurely |
| `OsDragResult` (relocated) | `oriterm_ui/src/drag_types.rs` (new file, ~15 lines) | 03 | Shared enum for OS drag session results, moved from Windows-only `platform_windows` |
| `cursor_screen_pos()` | `platform_macos.rs`, `platform_linux.rs` | 03 | Cross-platform cursor position in screen coordinates |
| `visible_frame_bounds()` | `platform_macos.rs`, `platform_linux.rs` | 03 | Cross-platform window frame bounds |
| `set_transitions_enabled()` | `platform_macos.rs`, `platform_linux.rs` | 03 | No-op stubs (only Windows has DWM transitions) |
| `show_window()` | `platform_macos.rs`, `platform_linux.rs` | 03 | Cross-platform show hidden window |

## Known Bugs (Pre-existing)

| Bug | Root Cause | Fix Location | Status |
|-----|-----------|-------------|--------|
| Windows chrome drawn on macOS | `draw_window_controls()` unconditional | Section 01 | Not Started |
| Tab switch hangs ~5s on first render | `refresh_pane_snapshot()` sync RPC fallback | Section 02 | Not Started |
| Tab tear-off no-ops on macOS | `tear_off.rs`/`merge.rs` gated Windows-only | Section 03 | Not Started |
| Stale daemon causes protocol mismatch | Old daemon from previous build | Workaround: kill daemon | Documented |

## Quick Reference

| ID | Title | File | Status |
|----|-------|------|--------|
| 01 | Window Chrome Platform Gate | `section-01-chrome-platform-gate.md` | Not Started |
| 02 | Non-Blocking Snapshot Refresh | `section-02-nonblocking-snapshot.md` | Not Started |
| 03 | macOS Tab Tear-Off | `section-03-macos-tear-off.md` | Not Started |
| 04 | Verification | `section-04-verification.md` | Not Started |
