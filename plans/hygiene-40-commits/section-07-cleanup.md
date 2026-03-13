---
section: "07"
title: "Cleanup — Final Verification"
status: complete
goal: "Verify all hygiene fixes pass the full build/test/lint suite"
depends_on: ["01", "02", "03", "04", "05", "06"]
sections:
  - id: "07.1"
    title: "Full Verification Suite"
    status: complete
  - id: "07.2"
    title: "Completion Checklist"
    status: complete
---

# Section 07: Cleanup — Final Verification

**Status:** Complete
**Goal:** All hygiene fixes verified by the complete build/test/lint pipeline. No regressions.

**Context:** After all 6 boundary sections are complete, this section runs the full verification suite to catch any cross-section regressions, unresolved compilation errors, or clippy warnings introduced by the combined changes.

**Depends on:** Sections 01-06 (all complete).

---

## 07.1 Full Verification Suite

Run each verification step in order. All must pass.

- [x] `./fmt-all.sh` — All code formatted consistently. Fixed pre-existing formatting diffs in `selection/mod.rs`, `clipboard_ops/mod.rs`, `constructors.rs`, `context_menu/tests.rs`, `mouse_input.rs`, `multi_pane.rs`, `merge.rs`, `merge_linux.rs`, `merge_macos.rs`, `move_ops.rs`, `frame_input/mod.rs`, `render.rs`.
- [x] `./build-all.sh` — Cross-compiles for all targets (debug + release) without errors.
- [x] `./clippy-all.sh` — Zero clippy warnings (Windows cross-compile + host).
- [x] `./test-all.sh` — All tests pass. No new `#[ignore]` annotations.

---

## 07.2 Completion Checklist

- [x] `./fmt-all.sh` passes
- [x] `./build-all.sh` passes
- [x] `./clippy-all.sh` passes (zero warnings)
- [x] `./test-all.sh` passes (no regressions)
- [x] No new `#[allow(...)]` without `reason = "..."` — all new `#[allow(dead_code)]` have reason strings
- [x] No new `#[expect(...)]` without justification
- [x] All files touched by this plan are under 500 lines (excluding `tests.rs`) — VTE `lib.rs` at 895 lines is pre-existing (production code was always ~895 lines; we only extracted 812 lines of inline tests)
- [x] `[PLANNED]` items verified as present in `plans/roadmap/section-23-performance.md` — Section 23 covers buffer discipline, allocation optimization, and snapshot extraction patterns

**Exit Criteria Met:** `./fmt-all.sh && ./build-all.sh && ./clippy-all.sh && ./test-all.sh` all green. Zero new warnings, zero new test failures.
