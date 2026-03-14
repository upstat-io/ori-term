---
section: "07"
title: "Verification"
status: not-started
goal: "Comprehensive validation that the retained UI framework produces correct output, maintains performance invariants, and introduces no regressions."
depends_on: ["01", "02", "03", "04", "05", "06"]
sections:
  - id: "07.1"
    title: "Behavioral Equivalence"
    status: not-started
  - id: "07.2"
    title: "Performance Validation"
    status: not-started
  - id: "07.3"
    title: "Test Matrix"
    status: not-started
  - id: "07.4"
    title: "Completion Checklist"
    status: not-started
reviewed: true
---

# Section 07: Verification

**Status:** Not Started
**Goal:** Prove that the retained UI framework is correct (identical output to the immediate-mode path), performant (sub-millisecond UI interaction response), and complete (all widget types, all surface types, all interaction patterns).

**Depends on:** All previous sections.

---

## 07.1 Behavioral Equivalence

Verify that scene composition produces identical `DrawList` output to the full-rebuild path. Place equivalence tests in `oriterm_ui/src/draw/scene_compose/tests.rs` (sibling to `scene_compose/mod.rs`) following the project's test organization rules.

- [ ] **Equivalence test harness:** Create a test that renders the same widget tree twice -- once via full rebuild (all scene nodes invalidated), once via scene composition (all nodes valid from a prior frame). Compare `DrawList::commands()` output element-by-element.

- [ ] **Test cases:**
  - Simple label
  - Button with hover state
  - Container with 5 mixed children
  - ScrollWidget with clipped content
  - Nested containers (3 levels deep)
  - Full Settings panel form
  - Overlay popup on top of content

- [ ] **Clip/Layer stack correctness:** Verify that `PushClip`/`PopClip` and `PushLayer`/`PopLayer` are correctly balanced in composed output. A test that counts push/pop pairs and asserts balance.

- [ ] **Transform correctness:** Verify that `PushTranslate`/`PopTranslate` are correctly balanced and that the GPU converter produces identical pixel positions as the old bounds-shifting approach.

---

## 07.2 Performance Validation

- [ ] **Text cache hit rate:** Instrument `CachedTextMeasurer` to log cache hits vs misses. After warmup (first frame), hit rate should be 100% for static UI (Settings dialog with no changes).

- [ ] **Draw call reduction:** Count `Widget::draw()` invocations per frame:
  - Full rebuild: ~N calls (where N = widget count)
  - Hover on one button: 1 call (the button) + 0-2 calls (container chain for clip/layer)
  - Scroll without content change: 0 calls
  - Mouse move over blank space: 0 calls

- [ ] **Frame time budget:** 60fps target = 16.6ms per frame
  - Settings dialog hover: <1ms (no shaping, no layout, 1 widget draw)
  - Settings dialog scroll: <0.5ms (no shaping, no layout, no widget draw, transform update only)
  - Settings dialog open: <5ms (first frame, all widgets draw, all text shaped)
  - Tab bar hover: <0.5ms (1 tab widget draw)

- [ ] **Memory:** Scene cache and text cache do not grow unboundedly. After rendering the Settings dialog, the caches stabilize at a fixed size. Verified by monitoring cache `.len()` across 100 frames.

- [ ] **Cache eviction:** Text cache at capacity (1024 entries) evicts correctly when a new entry is inserted. Verify no panic, no stale entries returned after eviction.

- [ ] **Invalidation triggers:** Each of these events clears both text cache and scene cache:
  - Font reload (font family or size change in config)
  - Theme change (dark/light mode toggle, accent color change)
  - DPI/scale factor change (window moved to a different-DPI monitor)
  - Window resize (scene cache only — text cache unaffected by resize)
  Verify each trigger independently with a test that renders, triggers the invalidation, renders again, and confirms the second render produces correct output.

---

## 07.3 Test Matrix

Build a comprehensive test matrix covering every widget type through the retained pipeline.

- [ ] **Leaf widgets:**
  - LabelWidget — text cache hit after first frame
  - ButtonWidget — scene node invalidated on hover enter/leave, valid otherwise
  - CheckboxWidget — scene node invalidated on toggle
  - SliderWidget — scene node invalidated on value change
  - DropdownWidget — scene node invalidated on selection change
  - TextInputWidget — scene node invalidated on text change
  - SeparatorWidget — never invalidated (pure static)
  - SpacerWidget — never invalidated (pure static)

- [ ] **Container widgets:**
  - ContainerWidget — selective child rebuild
  - FormLayout — selective row rebuild
  - FormSection — collapse/expand invalidates section subtree only
  - ScrollWidget — scroll transform, child scene stable
  - StackWidget — z-order changes invalidate all children

- [ ] **Overlays:**
  - Dropdown popup — scene node created on open, destroyed on close
  - Context menu — transient, no caching
  - Tooltip — transient, no caching

- [ ] **Chrome:**
  - TabBarWidget — tab add/remove/rename invalidates affected tab only
  - WindowChromeWidget — close button hover invalidates button only
  - Search bar (drawn directly via `draw_search_bar()` in `redraw/search_bar.rs`) — not a widget, draw commands emitted directly. Invalidation is implicit (redrawn when search state changes).

---

## 07.4 Completion Checklist

- [ ] Behavioral equivalence verified for all widget types in test matrix
- [ ] Performance targets met for all measured scenarios
- [ ] Scene cache and text cache bounded-size verified
- [ ] Clip/layer/transform stack balance verified
- [ ] No regressions in existing tests (`./test-all.sh` green)
- [ ] No clippy warnings (`./clippy-all.sh` green)
- [ ] Cross-platform build (`./build-all.sh` green)
- [ ] Visual verification: Settings dialog looks identical before/after
- [ ] Visual verification: Tab bar looks identical before/after
- [ ] Visual verification: Overlay popups look identical before/after
- [ ] **Concurrent window interaction:** Open a terminal window AND a Settings dialog simultaneously. Verify:
  - Terminal output (PTY data) renders correctly while dialog is open
  - Dialog hover/scroll works correctly while terminal is receiving output
  - Closing the dialog does not corrupt the terminal window's render state
  - Text cache and scene cache are per-context (no cross-contamination between terminal chrome and dialog caches)
- [ ] **Lifecycle state machine:** Open and close a dialog rapidly (< 100ms between open and close). Verify no crash, no flash, no leaked GPU resources. Verify the `CreatedHidden → Primed → Visible → Closing → Destroyed` sequence completes cleanly even under rapid open/close.

**Exit Criteria:** All tests in the matrix pass. Frame time for Settings hover ≤1ms. Frame time for Settings scroll ≤0.5ms. `./test-all.sh`, `./clippy-all.sh`, `./build-all.sh` all green. Visual spot-check shows no rendering differences.
