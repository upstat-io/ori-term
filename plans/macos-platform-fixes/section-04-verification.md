---
section: "04"
title: "Verification"
status: not-started
goal: "All three platforms build, pass tests, and exhibit correct behavior for chrome, tab switch, and tear-off"
depends_on: ["01", "02", "03"]
sections:
  - id: "04.1"
    title: "Build and Test Matrix"
    status: not-started
  - id: "04.2"
    title: "Visual Regression"
    status: not-started
  - id: "04.3"
    title: "Completion Checklist"
    status: not-started
---

# Section 04: Verification

**Status:** Not Started
**Goal:** All changes from Sections 01-03 are verified across macOS, Windows, and Linux. No regressions in existing functionality. Visual behavior matches expectations on each platform.

**Context:** This plan touches platform-specific code paths across three modules (chrome rendering, snapshot refresh, tab tear-off). Each change must be verified to not break the other platforms.

**Depends on:** Sections 01, 02, 03 (all must be complete).

---

## 04.1 Build and Test Matrix

- [ ] `./build-all.sh` succeeds (all targets)
- [ ] `./clippy-all.sh` passes with zero warnings
- [ ] `./test-all.sh` passes with zero failures
- [ ] `cargo build` on macOS native target succeeds with zero warnings
- [ ] `cargo test` on macOS native target passes
- [ ] No `#[allow(dead_code)]` added — all code is properly `#[cfg]`-gated or genuinely used
- [ ] No `#[allow(unused_imports)]` added — imports are `#[cfg]`-gated to match usage
- [ ] `cargo build --target x86_64-pc-windows-gnu` succeeds (cross-compile from WSL)

---

## 04.2 Visual Regression

**macOS checks:**
- [ ] Native traffic lights (red/yellow/green circles) appear in correct position
- [ ] No custom rectangular minimize/maximize/close buttons visible
- [ ] Tab bar layout: tabs start after traffic light zone (~76px left inset)
- [ ] Tab switching is instant (no visible hang or freeze)
- [ ] New tab creation renders content within ~100ms
- [ ] Tab tear-off: dragging a tab down past threshold creates a new window
- [ ] Tab merge: releasing torn-off window over another window's tab bar merges the tab
- [ ] Single-tab window drag works (OS drag with merge detection)

**Windows checks (no regressions):**
- [ ] Custom minimize/maximize/close buttons render in the controls zone
- [ ] Tab switching remains instant
- [ ] Tab tear-off works (drag past threshold creates new window)
- [ ] Tab merge works (live merge during drag + post-drag merge)
- [ ] Seamless drag continuation after merge works

**Linux checks (no regressions):**
- [ ] Custom minimize/maximize/close buttons render in the controls zone
- [ ] Tab switching remains instant
- [ ] Tab tear-off compiles and runs (creates new window on drag past threshold)
- [ ] Wayland: tab tear-off creates a new window; merge detection may not work (global cursor pos unavailable) but must not crash

**Daemon mode checks (all platforms):**
- [ ] Tab switch in daemon mode is instant (no visible hang)
- [ ] Tab switch in embedded mode is instant (no regression)
- [ ] New pane spawn in daemon mode renders content within ~100ms
- [ ] `pending_refresh` state is properly cleaned up (no memory leak of pane IDs after panes are closed)
- [ ] Stale daemon scenario: kill old daemon, restart — client reconnects cleanly

---

## 04.3 Completion Checklist

- [ ] All build scripts pass on all platforms
- [ ] Visual behavior verified on macOS
- [ ] No regressions on Windows
- [ ] No regressions on Linux
- [ ] Log file shows no errors during normal operation on macOS
- [ ] Stale daemon scenario: killing old daemon and restarting works cleanly
- [ ] `MuxClient` tests in `oriterm_mux/src/backend/client/tests.rs` cover non-blocking snapshot refresh, `pending_refresh` lifecycle, and `clear_pane_snapshot_dirty` interaction. New tests added to the existing `tests.rs` sibling file (per test-organization.md), not inline.
- [ ] Tab drag tests in `oriterm/src/app/tab_drag/tests.rs` still pass (pure computation, unaffected by platform changes)
- [ ] No new test files needed for Section 01 (cfg-gating is compile-time, verified by `build-all.sh`)
- [ ] Any new tests for `drag_types.rs` follow the sibling `tests.rs` convention if the module is made a directory

**Exit Criteria:** `./build-all.sh`, `./clippy-all.sh`, and `./test-all.sh` all pass. Manual visual verification confirms correct chrome, instant tab switch, and working tear-off on macOS. No regressions on Windows or Linux.
