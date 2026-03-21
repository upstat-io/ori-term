---
section: "05"
title: "Verification & Measurement"
status: not-started
reviewed: false
third_party_review:
  status: none
  updated: null
goal: "Measure the cumulative impact of Sections 01-04, verify no regressions, and decide whether advanced rendering work (retained scene, GPU scroll) is justified"
depends_on: ["01", "02", "03", "04"]
sections:
  - id: "05.1"
    title: "Performance Measurement"
    status: not-started
  - id: "05.2"
    title: "Test Matrix"
    status: not-started
  - id: "05.3"
    title: "Advanced Rendering Decision"
    status: not-started
  - id: "05.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "05.4"
    title: "Build & Verify"
    status: not-started
---

# Section 05: Verification & Measurement

**Status:** Not Started
**Goal:** Measure the cumulative impact of Sections 01-04 on dialog and main-window rendering costs. Verify no behavioral regressions across all render paths. Make a data-driven decision about whether advanced rendering (retained scene, GPU scroll) is worth the complexity.

**Production code path:** All render paths — `dialog_rendering.rs`, `redraw/mod.rs`, `redraw/multi_pane/mod.rs`. This section measures and verifies, it doesn't add new production code.

**Observable change:** Documentation of before/after metrics. A clear go/no-go decision on advanced rendering work.

**Context:** The overview's design principle #3 says "Escalate only when measurement justifies it." Sections 01-04 fix correctness bugs, add viewport culling, and reduce tree-walk costs. This section measures whether those changes are sufficient, or whether retained-scene patching and GPU-side scroll are still needed. If Sections 01-04 already make dialog scrolling and hover cheap enough, advanced work should not be undertaken.

**Depends on:** All prior sections (01-04).

---

## 05.1 Performance Measurement

**File(s):** No production code changes. Measurement via logging and profiling.

- [ ] **Dialog scroll cost:** Measure scene primitive count during dialog scroll (top → bottom → top). Record: primitives at top, primitives scrolled 50%, primitives scrolled to bottom.
- [ ] **Dialog hover cost:** Measure widget visit count during a single hover event on a button in a 50+ widget dialog page. Record: visits in prepare, visits in prepaint, total primitives painted.
- [ ] **Tab bar hover cost:** Same measurement for a tab bar hover in the main window.
- [ ] **Page switch cost:** Measure widget visit count and primitive count when switching pages in the settings dialog. After Section 02's `PageContainerWidget` fix, prepare/prepaint should visit only the active page's widgets, not all pages. Record: visits before fix (if baseline captured in Section 02), visits after fix
- [ ] **Idle frame cost:** Verify zero CPU cost when idle (no dirty widgets, no animations). The event loop should remain in `ControlFlow::Wait`.
- [ ] Document all measurements in this section file as baseline numbers.

### Measurement Method

Use `log::debug!` counters (already added in Sections 02-03) plus `Scene::len()`:

```
[dialog scroll] primitives: {N} (top), {N} (50%), {N} (bottom)
[dialog hover]  prepare visits: {N}, prepaint visits: {N}, primitives: {N}
[tab bar hover] prepare visits: {N}, prepaint visits: {N}, primitives: {N}
[page switch]   prepare visits: {N}, prepaint visits: {N}, primitives: {N}
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

- [ ] **Automated tests:**
  - `cargo test -p oriterm_ui` — all widget and harness tests pass
  - `cargo test -p oriterm` — all app tests pass
  - `cargo test -p oriterm --test architecture` — architectural tests pass

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

- [ ] If all targets are met: mark this plan as complete. Advanced rendering (retained scene, GPU scroll) is not needed.
- [ ] If dialog scroll is still expensive despite culling: document the remaining bottleneck and create a follow-up plan for retained-scene patching.
- [ ] If hover costs are still high despite selective walks: document why and determine if the `InvalidationTracker` approach needs rework.
- [ ] Record the decision and rationale in this section file.

---

## 05.R Third Party Review Findings

- None.

---

## 05.4 Build & Verify

- [ ] `./build-all.sh` passes
- [ ] `./clippy-all.sh` passes
- [ ] `./test-all.sh` passes
- [ ] All measurements documented in 05.1
- [ ] Test matrix in 05.2 fully verified (manual + automated)
- [ ] Go/no-go decision on advanced rendering documented in 05.3
- [ ] No `#[allow(dead_code)]` on any items introduced by Sections 01-04
- [ ] No `log::debug!` measurement instrumentation left in production code after this section — either remove it or gate behind a feature flag

**Exit Criteria:** All measurements are recorded. All test matrix items pass. A clear, documented decision exists about whether advanced rendering work is needed. If no advanced work is needed, this plan is complete. If advanced work is needed, a follow-up plan exists with specific scope based on the measurements.
