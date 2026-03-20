---
section: "07"
title: "Verification & Benchmarks"
status: not-started
reviewed: true
goal: "Comprehensive verification that incremental rendering is correct, performant, and produces no visual regressions"
depends_on: ["06"]
sections:
  - id: "07.1"
    title: "Performance Benchmarks"
    status: not-started
  - id: "07.2"
    title: "Visual Regression Tests"
    status: not-started
  - id: "07.3"
    title: "Correctness Tests"
    status: not-started
  - id: "07.4"
    title: "Stress Tests"
    status: not-started
  - id: "07.5"
    title: "Completion Checklist"
    status: not-started
---

# Section 07: Verification & Benchmarks

**Status:** Not Started
**Goal:** Prove that incremental rendering is correct (no visual artifacts), performant (measurable frame time improvement), and robust (no edge case regressions).

**Depends on:** Section 06 (all incremental pipeline changes integrated).

---

## 07.1 Performance Benchmarks

Measure frame time for key scenarios before and after.

- [ ] **Scroll frame time:** Open Colors page, scroll continuously via mouse wheel:
  - Before: ~8-19ms per frame (full repaint)
  - After: <4ms per frame (viewport cull + retained scene)
  - With GPU scroll: <2ms per frame (texture blit)

- [ ] **Hover frame time:** Move cursor across setting rows:
  - Before: ~5-10ms (full repaint of 37+ widgets)
  - After: <2ms (repaint only hovered + previously hovered widget)

- [ ] **Idle frame time:** Dialog open, cursor stationary:
  - Before: 0ms (correct — no repaint)
  - After: 0ms (must remain zero — no regression)

- [ ] **Page switch frame time:** Click sidebar nav item:
  - Before: ~10-20ms (full layout + full repaint)
  - After: ~10-20ms (layout recomputation is unavoidable for page switch — but only ONE frame)

- [ ] **Animation frame time:** Toggle animation (thumb slide):
  - Before: ~8-15ms per animation frame
  - After: <3ms (only the toggle widget repaints)

---

## 07.2 Visual Regression Tests

Verify no visual artifacts from retained scene / viewport culling.

- [ ] **Scroll visual check:** Scroll Colors page slowly, verify no missing/stale cards, no gaps, no overlapping content

- [ ] **Hover visual check:** Hover each setting row, verify highlight appears/disappears correctly with no ghosting

- [ ] **Page switch visual check:** Switch between all 8 pages, verify correct content displayed, no stale content from previous page

- [ ] **Dropdown visual check:** Open a dropdown, verify popup renders correctly on top of content

- [ ] **Footer visual check:** Scroll content, verify footer remains fixed with no bleed-through

- [ ] **Resize visual check:** Resize dialog window, verify content reflows correctly with no artifacts

---

## 07.3 Correctness Tests

Automated tests verifying incremental pipeline equivalence.

- [ ] **Full-vs-incremental equivalence:** For each page, render via full repaint and via incremental pipeline. Compare Scene primitive counts and positions — they must match exactly.

- [ ] **Dirty tracking correctness:** Mark one widget dirty, render incrementally, verify only that widget's fragment was repainted (paint call count = 1).

- [ ] **Fragment cache correctness:** Render a frame, cache all fragments, render again with no changes, verify Scene is identical (bitwise comparison of primitive arrays).

- [ ] **Viewport culling correctness:** Render with culling enabled vs disabled, compare Scene primitive sets — visible primitives must match, culled primitives must be absent.

- [ ] **Damage rect correctness:** Hover a widget, compute damage rects, verify the rect covers exactly the widget's bounds (no overshoot, no undershoot).

- [ ] **Overlay during incremental render:** Open a dropdown overlay, verify overlay renders correctly on top of retained content scene. Close overlay, verify content scene is not corrupted (overlay clearing doesn't wipe retained content).

- [ ] **Scroll invalidates content fragments:** Scroll by 1px, verify all visible content fragments are invalidated (because absolute positions change). Verify chrome fragment is NOT invalidated (chrome doesn't scroll).

- [ ] **Chrome/content independence:** Change chrome state (e.g., hover close button), verify content fragments are untouched. Change content state (hover a setting row), verify chrome fragment is untouched.

- [ ] **Empty prepaint_bounds regression:** Verify prepaint receives populated bounds map (not empty HashMap). Check that `PrepaintCtx.bounds` is non-default for all widgets.

---

## 07.4 Stress Tests

Edge cases and stress scenarios.

- [ ] **Rapid scrolling:** Scroll at maximum speed (hold PageDown), verify no visual tearing, no missed frames, no crash

- [ ] **Rapid page switching:** Click sidebar items rapidly (1 click per 100ms), verify page content stays in sync with sidebar indicator

- [ ] **Resize during scroll:** Scroll while resizing the dialog, verify content reflows and scroll position is preserved

- [ ] **Overlay during scroll:** Open a dropdown while scrolling, verify dropdown popup renders correctly

- [ ] **100 scroll events in 1 second:** Simulate via test harness, verify no memory growth, no panic, no hang

- [ ] **DPI change during scroll:** Change scale factor while scrolled, verify scroll texture recreated, layout recomputed, all fragments invalidated, content renders correctly at new DPI

- [ ] **Page switch with retained scene:** Switch between all pages, verify fragment cache is cleared per page switch, no stale fragments from previous page leak through

---

## 07.5 Completion Checklist

- [ ] Frame time benchmarks show measurable improvement for scroll and hover
- [ ] No visual regressions in any dialog page
- [ ] Full-vs-incremental equivalence test passes
- [ ] Viewport culling correctness verified
- [ ] Fragment cache correctness verified
- [ ] Stress tests pass without crash or memory leak
- [ ] `./test-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./build-all.sh` green
- [ ] Overlay rendering works correctly with retained scene (no corruption)
- [ ] Chrome/content independence verified (dirty one doesn't repaint other)
- [ ] DPI change during scroll handled correctly
- [ ] Performance invariants from CLAUDE.md verified:
  - Zero idle CPU (no spurious repaints when idle)
  - Zero allocations in hot render path (retained scene reuses buffers)

**Exit Criteria:** All benchmarks show improvement. All visual regression checks pass. Full-vs-incremental equivalence test produces identical output. Stress tests pass. All 1580+ tests pass. Scroll frame time is <4ms (or <2ms with GPU scroll).
