---
section: "05"
title: "Verification & Measurement"
status: in-progress
reviewed: true
third_party_review:
  status: resolved
  updated: 2026-03-21
goal: "Measure the cumulative impact of Sections 01-04, verify no regressions, and decide whether advanced rendering work (retained scene, GPU scroll) is justified"
depends_on: ["01", "02", "03", "04"]
sections:
  - id: "05.1"
    title: "Performance Measurement"
    status: not-started
  - id: "05.2"
    title: "Test Matrix"
    status: in-progress
  - id: "05.3"
    title: "Advanced Rendering Decision"
    status: not-started
  - id: "05.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "05.4"
    title: "Build & Verify"
    status: in-progress
---

# Section 05: Verification & Measurement

**Status:** In Progress
**Goal:** Measure the cumulative impact of Sections 01-04 on dialog and main-window rendering costs. Verify no behavioral regressions across all render paths. Make a data-driven decision about whether advanced rendering (retained scene, GPU scroll) is worth the complexity.

**Production code path:** All render paths — `dialog_rendering.rs`, `redraw/mod.rs`, `redraw/multi_pane/mod.rs`. This section measures and verifies, it doesn't add new production code.

**Observable change:** Documentation of before/after metrics. A clear go/no-go decision on advanced rendering work.

**Context:** The overview's design principle #3 says "Escalate only when measurement justifies it." Sections 01-04 fix correctness bugs, add viewport culling, and reduce tree-walk costs. This section measures whether those changes are sufficient, or whether retained-scene patching and GPU-side scroll are still needed. If Sections 01-04 already make dialog scrolling and hover cheap enough, advanced work should not be undertaken.

**Depends on:** All prior sections (01-04).

---

## 05.1 Performance Measurement

**File(s):** Temporary `log::debug!` additions to `oriterm/src/app/dialog_rendering.rs`, `oriterm/src/app/redraw/mod.rs`, `oriterm/src/app/redraw/multi_pane/mod.rs`, and `oriterm_ui/src/pipeline/tree_walk.rs`. These are measurement-only additions that will be removed in 05.4.

> **PRECONDITION:** These measurements are only meaningful after Section 04 wires the `InvalidationTracker` into all app-layer render paths. If any path still passes `None`, selective walks are not active and the measurements will show full tree walks regardless of dirty state.

### Step 1 — Add measurement instrumentation

> **NOTE:** No measurement counters currently exist in the production render paths (Sections 02 and 03 deferred all instrumentation). This step must add them before any measurement can happen.

- [ ] **Widget visit counter in `tree_walk.rs`.** Add a thread-local `Cell<u32>` counter incremented once per `prepare_widget_frame()` call and once per `prepaint_widget_tree` recursive entry. Log the count via `log::debug!` at the end of each top-level `prepare_widget_tree` / `prepaint_widget_tree` call (zero-cost when RUST_LOG is not set to debug). Do not add parameters to function signatures
- [ ] **Scene primitive counter in render paths.** Add `log::debug!("scene primitives: {}", scene.len())` after `paint()` calls in `compose_dialog_widgets()`, `handle_redraw()`, and `handle_redraw_multi_pane()`
- [ ] **Dirty state counter.** Add `log::debug!("dirty_map.len={}, dirty_ancestors.len={}", ...)` before `clear()` calls in `render_dispatch.rs` and `event_loop_helpers/mod.rs`

### Step 2 — Collect measurements

- [ ] **Dialog scroll cost:** Measure scene primitive count during dialog scroll (top, 50%, bottom). Record numbers in this file
- [ ] **Dialog hover cost:** Measure widget visit count during a single hover event on a button in a 50+ widget dialog page. Record: visits in prepare, visits in prepaint, total primitives painted
- [ ] **Tab bar hover cost:** Same measurement for a tab bar hover in the main window
- [ ] **Page switch cost:** Measure widget visit count and primitive count when switching pages in the settings dialog
- [ ] **Idle frame cost:** Verify zero CPU cost when idle (no dirty widgets, no animations). The event loop should remain in `ControlFlow::Wait`
- [ ] **Dirty state leak check:** Verify that `InvalidationTracker::dirty_map` is empty between frames when no interaction is occurring. If dirty state leaks, selective walks degrade to full walks
- [ ] Document all measurements in this section file as baseline numbers

### Measurement Results

_(To be filled in during implementation)_

```
[dialog scroll] primitives: ??? (top), ??? (50%), ??? (bottom)
[dialog hover]  prepare visits: ???, prepaint visits: ???, primitives: ???
[tab bar hover] prepare visits: ???, prepaint visits: ???, primitives: ???
[page switch]   prepare visits: ???, prepaint visits: ???, primitives: ???
[idle]          dirty_map.len: ???, dirty_ancestors.len: ???
```

---

## 05.2 Test Matrix

Verify all render paths still work correctly after Sections 01-04.

- [ ] **Dialog rendering:**
  - Settings dialog opens, renders correctly
  - Page switching works (all pages render)
  - Scroll works (content scrolls smoothly)
  - Hover works (buttons highlight, controls respond)
  - Focus works (tab navigation, focus rings)
  - Overlays work (dropdown lists render on top)
  - Confirmation dialogs render correctly

- [ ] **Single-pane rendering:**
  - Tab bar renders with correct tab titles
  - Tab hover shows close button
  - Tab click switches tabs
  - Tab bar animation (close button fade) works
  - Overlay popups render correctly

- [ ] **Multi-pane rendering:**
  - Tab bar renders in multi-pane mode
  - Pane dividers render correctly
  - Tab hover/click work in multi-pane
  - Overlay popups render in multi-pane

- [x] **Automated tests:**
  - `cargo test -p oriterm_ui` — 1,612 tests pass, 0 failures
  - `cargo test -p oriterm` — 10 tests pass, 0 failures
  - `cargo test -p oriterm --test architecture` — 10 tests pass, 0 failures

---

## 05.3 Advanced Rendering Decision

Based on measurements from 05.1, decide whether advanced rendering work is justified.

**Decision criteria:**

| Metric | Target | Action if met | Action if not met |
|--------|--------|---------------|-------------------|
| Dialog hover widget visits | < 15 per hover event | No further work needed | Investigate why selective walks aren't effective |
| Dialog scroll primitives | Proportional to visible content | No GPU scroll needed | Consider retained scene for scroll optimization |
| Tab bar hover visits | < 5 per hover event | No further work needed | Investigate tab bar tree structure |
| Page switch prepare visits | Active page only (not all pages) | PageContainer fix working | `for_each_child_mut` still visiting hidden pages |
| Idle CPU | Zero (cursor blink only) | Confirmed | Investigate spurious invalidation |
| Dirty state between frames | Empty dirty_map when idle | No state leaks | Investigate missing `clear()` calls |

- [ ] If all targets are met: mark this plan as complete. Advanced rendering (retained scene, GPU scroll) is not needed.
- [ ] If dialog scroll is still expensive despite culling: document the remaining bottleneck and create a follow-up plan for retained-scene patching.
- [ ] If hover costs are still high despite selective walks: document why and determine if the `InvalidationTracker` approach needs rework.
- [ ] Record the decision and rationale in this section file.

---

## 05.R Third Party Review Findings

- [x] `[TPR-05-001][low]` `plans/incremental-rendering/index.md:103` — incremental-rendering status metadata is internally inconsistent after the latest plan edits.
  **Resolved 2026-03-21**: Accepted. Synchronized all stale status text:
  `index.md` Section 05 "Not Started" → "In Progress",
  `00-overview.md` Quick Reference Section 04 "Not Started" → "In Progress",
  `00-overview.md` Quick Reference Section 05 "Not Started" → "In Progress",
  `section-05-verification.md` body "**Status:** Not Started" → "**Status:** In Progress".
  All now match the YAML frontmatter.

---

## 05.4 Build & Verify

- [ ] **Remove all measurement instrumentation added in 05.1 Step 1.** All `log::debug!` counters, `AtomicU32`/thread-local counters, and scene len logging added for measurement purposes must be removed before this section is marked complete. Grep for any `log::debug!` calls containing "visit", "primitive", "dirty_map" in the render paths to verify removal is complete. The project has `dead_code = "deny"` so unused counter infrastructure will fail to compile
- [x] `./build-all.sh` passes
- [x] `./clippy-all.sh` passes
- [x] `./test-all.sh` passes
- [ ] All measurements documented in 05.1
- [ ] Test matrix in 05.2 fully verified (manual + automated)
- [ ] Go/no-go decision on advanced rendering documented in 05.3
- [x] No `#[allow(dead_code)]` on any items introduced by Sections 01-04

**Exit Criteria:** All measurements are recorded and documented in 05.1. All test matrix items pass. All measurement instrumentation is removed from production code. A clear, documented decision exists about whether advanced rendering work is needed. If no advanced work is needed, this plan is marked complete. If advanced work is needed, a follow-up plan exists with specific scope derived from the measurements.
