---
section: "04"
title: "Verification"
status: complete
goal: "All three platforms build, pass tests, and exhibit correct behavior for chrome, tab switch, and tear-off"
depends_on: ["01", "02", "03"]
sections:
  - id: "04.1"
    title: "Build and Test Matrix"
    status: complete
  - id: "04.2"
    title: "Visual Regression"
    status: complete
  - id: "04.3"
    title: "Completion Checklist"
    status: complete
---

# Section 04: Verification

**Status:** Complete
**Goal:** All changes from Sections 01-03 are verified across macOS, Windows, and Linux. No regressions in existing functionality. Visual behavior matches expectations on each platform.

**Context:** This plan touches platform-specific code paths across three modules (chrome rendering, snapshot refresh, tab tear-off). Each change must be verified to not break the other platforms.

**Depends on:** Sections 01, 02, 03 (all must be complete).

---

## 04.1 Build and Test Matrix

- [x] `./build-all.sh` succeeds (all targets)
- [x] `./clippy-all.sh` passes with zero warnings
- [x] `./test-all.sh` passes with zero failures
- [x] `cargo build` on macOS native target succeeds with zero warnings
- [x] `cargo test` on macOS native target passes
- [x] No `#[allow(dead_code)]` added — all code is properly `#[cfg]`-gated or genuinely used
- [x] No `#[allow(unused_imports)]` added — imports are `#[cfg]`-gated to match usage
- [x] `cargo build --target x86_64-pc-windows-gnu` succeeds (cross-compile from WSL)

---

## 04.2 Visual Regression

**macOS checks:**
- [x] Native traffic lights (red/yellow/green circles) appear in correct position
- [x] No custom rectangular minimize/maximize/close buttons visible
- [x] Tab bar layout: tabs start after traffic light zone (~76px left inset)
- [x] Tab switching is instant (no visible hang or freeze)
- [x] New tab creation renders content within ~100ms
- [x] Tab tear-off: dragging a tab down past threshold creates a new window
- [x] Tab merge: releasing torn-off window over another window's tab bar merges the tab
- [x] Single-tab window drag works (OS drag with merge detection)

**Windows checks (no regressions):**
- [x] Custom minimize/maximize/close buttons render in the controls zone
- [x] Tab switching remains instant
- [x] Tab tear-off works (drag past threshold creates new window)
- [x] Tab merge works (live merge during drag + post-drag merge)
- [x] Seamless drag continuation after merge works

**Linux checks (no regressions):**
- [x] Custom minimize/maximize/close buttons render in the controls zone
- [x] Tab switching remains instant
- [x] Tab tear-off compiles and runs (creates new window on drag past threshold)
- [x] Wayland: tab tear-off creates a new window; merge detection may not work (global cursor pos unavailable) but must not crash

**Daemon mode checks (all platforms):**
- [x] Tab switch in daemon mode is instant (no visible hang)
- [x] Tab switch in embedded mode is instant (no regression)
- [x] New pane spawn in daemon mode renders content within ~100ms
- [x] `pending_refresh` state is properly cleaned up (no memory leak of pane IDs after panes are closed)
- [x] Stale daemon scenario: kill old daemon, restart — client reconnects cleanly

---

## 04.3 Completion Checklist

- [x] All build scripts pass on all platforms
- [x] Visual behavior verified on macOS
- [x] No regressions on Windows
- [x] No regressions on Linux
- [x] Log file shows no errors during normal operation on macOS
- [x] Stale daemon scenario: killing old daemon and restarting works cleanly
- [x] `MuxClient` tests in `oriterm_mux/src/backend/client/tests.rs` cover non-blocking snapshot refresh, `pending_refresh` lifecycle, and `clear_pane_snapshot_dirty` interaction. New tests added to the existing `tests.rs` sibling file (per test-organization.md), not inline.
- [x] Tab drag tests in `oriterm/src/app/tab_drag/tests.rs` still pass (pure computation, unaffected by platform changes)
- [x] No new test files needed for Section 01 (cfg-gating is compile-time, verified by `build-all.sh`)
- [x] Any new tests for `drag_types.rs` follow the sibling `tests.rs` convention if the module is made a directory

**Exit Criteria:** `./build-all.sh`, `./clippy-all.sh`, and `./test-all.sh` all pass. Manual visual verification confirms correct chrome, instant tab switch, and working tear-off on macOS. No regressions on Windows or Linux.
