---
section: "04"
title: "Prepaint Phase (3-Pass Rendering)"
status: not-started
reviewed: false
goal: "Split the current 2-pass rendering (layout + paint) into 3 passes (layout -> prepaint -> paint) to separate visual state resolution and interaction state queries from actual painting, enabling layout caching independent of paint."
inspired_by:
  - "GPUI Element trait (src/element.rs) — request_layout(), prepaint(), paint() three-pass model"
depends_on: []
sections:
  - id: "04.1"
    title: "Widget Trait Prepaint Method"
    status: not-started
  - id: "04.2"
    title: "Prepaint Pipeline Integration"
    status: not-started
  - id: "04.3"
    title: "Layout Caching"
    status: not-started
  - id: "04.4"
    title: "Completion Checklist"
    status: not-started
---

# Section 04: Prepaint Phase (3-Pass Rendering)

**Status:** Not Started
**Goal:** Add a `prepaint()` phase between layout and paint. Currently, interaction state queries (is_hot, is_active, is_focused) and visual state interpolation (VisualStateAnimator) happen during `paint()`. Separating these into prepaint enables: layout results to be cached across frames when only visual state changes (hover color), and visual state resolution to be separated from draw command emission.

**Context:** GPUI's Element trait has three phases with associated type state that flows between them. Our simpler retained-mode architecture doesn't need the full associated-type machinery, but the phase separation is valuable. When a button changes hover color, we currently relayout + repaint. With prepaint, we'd skip layout (unchanged) and only run prepaint (resolve visual state) + paint (emit draw commands with resolved state).

**Depends on:** Section 03 is recommended but not required. Prepaint works without Scene -- widgets still paint to DrawList. Scene's DamageTracker benefits from prepaint's `DirtyKind::Prepaint` for finer-grained damage tracking, but the dependency is bidirectional enhancement, not blocking.

---

## 04.1 Widget Trait Prepaint Method

**File(s):** `oriterm_ui/src/widgets/mod.rs` (currently 261 lines; +5 lines stays under 500), `oriterm_ui/src/widgets/contexts.rs` (currently 271 lines; +20 lines stays under 500)

- [ ] Add `prepaint()` method to Widget trait with default no-op body:
  ```rust
  /// Resolves visual states and caches interaction state queries.
  /// Called after layout, before paint.
  fn prepaint(&mut self, _ctx: &mut PrepaintCtx<'_>) {}
  ```
  **Impact:** All 35 existing widget types get the default no-op `prepaint()`.
  No existing code changes required for compilation. Widgets are migrated
  incrementally: move interaction state queries from `paint()` to `prepaint()`
  one widget at a time, validating each migration with the test harness.

- [ ] Define `PrepaintCtx`:
  ```rust
  pub struct PrepaintCtx<'a> {
      pub widget_id: WidgetId,
      pub bounds: Rect,
      pub interaction: Option<&'a InteractionManager>,
      pub theme: &'a UiTheme,
      pub now: Instant,
      /// Shared frame request flags for scheduling.
      pub frame_requests: Option<&'a FrameRequestFlags>,
  }
  ```

---

## 04.2 Prepaint Pipeline Integration

**File(s):** `oriterm_ui/src/pipeline.rs` (after 01.2a; currently ~180 lines, +20 lines stays under 500)

The current frame pipeline is:
```
event dispatch -> apply requests -> lifecycle delivery -> layout -> paint
```

Prepaint inserts between layout and paint:
```
event dispatch -> apply requests -> lifecycle delivery -> layout -> prepaint -> paint
```

- [ ] Add `prepaint_widget_tree()` function that walks the tree depth-first and calls `widget.prepaint(&mut PrepaintCtx { ... })` for each widget

- [ ] Move VisualStateAnimator interpolation from paint into prepaint:
  Currently, widgets call `animator.interpolate(property, now)` inside `paint()`.
  After this change, `prepaint()` resolves all animated properties and stores the
  results in the widget's own fields (e.g., `resolved_bg: Color`, `resolved_opacity: f32`).
  `paint()` then reads these pre-resolved values without calling the animator.

  **Migration scope:** ~15 widgets use VisualStateAnimator. Each migration is ~20 lines
  of changes (add `resolved_*` fields, implement `prepaint()`, update `paint()`).
  Total: ~300 lines across ~15 files. Widgets are migrated one at a time.

- [ ] Move interaction state queries (is_hot, is_active, is_focused) from paint into prepaint:
  Currently, `DrawCtx` queries `InteractionManager` during paint. After this change,
  `PrepaintCtx` queries interaction state and stores results on the widget. `DrawCtx`
  no longer needs `interaction: Option<&InteractionManager>`.

  **Migration:** Keep `DrawCtx.interaction` during migration. Remove it only after
  ALL widgets that use `ctx.is_hot()` etc. are converted to read from `prepaint()`
  results instead. This is a gradual migration, not a big-bang switch.

---

## 04.3 Layout Caching

- [ ] Extend `DirtyKind` enum in `oriterm_ui/src/invalidation/mod.rs` -- add `Prepaint` variant between `Paint` and `Layout`:
  ```rust
  pub enum DirtyKind {
      Clean,
      Paint,     // Visual change only (cursor blink) -- skip layout + prepaint
      Prepaint,  // Interaction state change (hover) -- skip layout, run prepaint + paint
      Layout,    // Structural change -- run all three phases
  }
  ```
- [ ] Skip layout when dirty kind is `Prepaint` or `Paint` (hover color change)
- [ ] Skip prepaint when dirty kind is `Paint` (cursor blink -- only paint changes)
- [ ] Track dirty levels with ordering: `Layout > Prepaint > Paint > Clean`

---

## 04.4 Completion Checklist

- [ ] `prepaint()` added to Widget trait with default no-op body
- [ ] `PrepaintCtx` defined with widget_id, bounds, interaction, theme, now, frame_requests
- [ ] `prepaint_widget_tree()` function walks tree and calls `widget.prepaint()`
- [ ] `DirtyKind::Prepaint` variant added between `Paint` and `Layout`
- [ ] `prepaint()` called for all widgets after layout, before paint
- [ ] Hover color change skips layout pass (`DirtyKind::Prepaint`)
- [ ] Cursor blink skips layout + prepaint passes (`DirtyKind::Paint`)
- [ ] At least one widget (ButtonWidget) migrated to use prepaint for visual state resolution
- [ ] `DrawCtx.interaction` retained during migration period (removed only after all widgets converted)
- [ ] No regressions in `./test-all.sh`
- [ ] `./clippy-all.sh` clean

**Exit Criteria:** A hover state change (mouse enters button) triggers only prepaint + paint, not layout. Measured via a test that counts phase invocations. At least ButtonWidget demonstrates the full prepaint pattern (resolved fields populated in prepaint, read in paint).
