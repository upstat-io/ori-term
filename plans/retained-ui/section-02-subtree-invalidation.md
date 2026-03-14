---
section: "02"
title: "Subtree Invalidation"
status: not-started
goal: "Widget events produce typed, scoped dirty signals that propagate upward through the widget tree — the framework knows exactly which widget is dirty and whether the dirtiness is layout-level or paint-level."
inspired_by:
  - "Flutter RenderObject markNeedsPaint / markNeedsLayout separation"
  - "Chromium Views InvalidateLayout / SchedulePaint with dirty rects"
depends_on: ["01"]
sections:
  - id: "02.1"
    title: "DirtyKind Enum"
    status: not-started
  - id: "02.2"
    title: "InvalidationTracker"
    status: not-started
  - id: "02.3"
    title: "Widget Dirty Propagation"
    status: not-started
  - id: "02.4"
    title: "Render Path Integration"
    status: not-started
  - id: "02.5"
    title: "Completion Checklist"
    status: not-started
reviewed: true
---

# Section 02: Subtree Invalidation

**Status:** Not Started
**Goal:** Replace whole-window `dirty: bool` / `ui_stale: bool` with per-widget, typed invalidation tracking. A hover on one button only marks that button's subtree as paint-dirty. Unchanged subtrees are provably clean and can be skipped during draw.

**Context:** Today `WindowContext` has `dirty: bool` and `ui_stale: bool` (window_context.rs:76-87). `DialogWindowContext` has `dirty: bool` and `urgent_redraw: bool` (dialog_context/mod.rs:61-63). When any event occurs — mouse move, hover, scroll — the whole window is marked dirty, and `render_dialog()` rebuilds the entire scene: chrome, all content widgets, all overlays. The `ContainerWidget` already has `needs_layout: bool` and `needs_paint: bool` fields (container/mod.rs:50-52) with `update_dirty()` (container/mod.rs:159), but these are never consumed by the render path — `draw()` always traverses the full tree.

The existing `EventResponse` enum (input/event.rs:129-141) already distinguishes `RequestPaint`, `RequestLayout`, and `RequestFocus`. This information is available but discarded at the window level — `dialog_context/event_handling.rs` uses `wants_repaint()` which funnels into `request_urgent_redraw()` (sets `dirty = true`), and `keyboard_input/overlay_dispatch.rs` similarly sets `ctx.dirty = true` on any handled response.

**Reference implementations:**
- **Flutter** `rendering/object.dart`: `markNeedsPaint()` walks up to the nearest repaint boundary. `markNeedsLayout()` walks up to the nearest relayout boundary. Separate dirty lists for paint and layout.
- **Chromium** `ui/views/view.cc`: `InvalidateLayout()` marks the view and ancestors. `SchedulePaint()` marks a dirty rect. Paint skips clean subtrees.

**Depends on:** Section 01 (text caching makes skipping subtrees meaningful — without caching, even a "clean" subtree re-shapes text during draw).

---

## 02.1 DirtyKind Enum

**File(s):** new `oriterm_ui/src/invalidation/mod.rs`

**Module registration:** Add `pub mod invalidation;` to `oriterm_ui/src/lib.rs` (after `pub mod input;` at line 15). Create as a directory module from the start (`invalidation/mod.rs` + `invalidation/tests.rs`) since tests are required (see 02.5). The `DirtyKind` enum belongs in its own module, not in `input/event.rs`, because invalidation tracking is a framework concern separate from event types.

Formalize the dirty signal with enough granularity to drive selective rebuild.

- [ ] Define `DirtyKind`:
  ```rust
  /// What kind of invalidation a widget event produced.
  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub enum DirtyKind {
      /// No change — skip redraw entirely.
      Clean,
      /// Visual change only (hover color, focus ring, cursor blink).
      /// Repaint the widget but don't recompute layout.
      Paint,
      /// Structural change (text content, child add/remove, visibility).
      /// Recompute layout from this widget upward, then repaint.
      Layout,
  }
  ```

- [ ] `DirtyKind` composes via `merge()`: `Clean.merge(Paint) → Paint`, `Paint.merge(Layout) → Layout`. This is used when a container receives dirty signals from multiple children.

- [ ] Map existing `EventResponse` to `DirtyKind`:
  - `EventResponse::Handled` → `DirtyKind::Clean` (event consumed, no visual change)
  - `EventResponse::Ignored` → `DirtyKind::Clean`
  - `EventResponse::RequestPaint` → `DirtyKind::Paint`
  - `EventResponse::RequestLayout` → `DirtyKind::Layout`
  - `EventResponse::RequestFocus` → `DirtyKind::Paint` (focus ring is a paint change)

- [ ] **Invariant documentation:** Widgets must return `RequestPaint` or `RequestLayout` for any visual state change. `Handled` means consumed-with-no-visual-change. Audit all widget `handle_mouse` / `handle_hover` implementations to verify this invariant holds. Any widget returning `Handled` after changing visual state (e.g. hover color) is a bug that will cause stale rendering under the retained pipeline.

---

## 02.2 InvalidationTracker

**File(s):** `oriterm_ui/src/invalidation/mod.rs` (same file as 02.1 — both `DirtyKind` and `InvalidationTracker` live here)

A lightweight structure that tracks which widgets are dirty and at what level.

- [ ] Define `InvalidationTracker`:
  ```rust
  pub struct InvalidationTracker {
      /// Paint-dirty widgets (need redraw but not relayout).
      paint_dirty: HashSet<WidgetId>,
      /// Layout-dirty widgets (need relayout + redraw).
      layout_dirty: HashSet<WidgetId>,
      /// Whether the entire scene needs rebuild (e.g. theme change, resize).
      full_invalidation: bool,
  }

  impl InvalidationTracker {
      pub fn mark(&mut self, id: WidgetId, kind: DirtyKind) { ... }
      pub fn is_paint_dirty(&self, id: WidgetId) -> bool { ... }
      pub fn is_layout_dirty(&self, id: WidgetId) -> bool { ... }
      pub fn is_any_dirty(&self) -> bool { ... }
      pub fn needs_full_rebuild(&self) -> bool { ... }
      pub fn clear(&mut self) { ... }
      pub fn invalidate_all(&mut self) { ... }
  }
  ```

- [ ] Place on `WindowContext` and `DialogWindowContext` alongside the existing `dirty: bool`. Initially, both systems coexist — the old `dirty` flag is still set, and the tracker provides additional granularity. Section 03 will consume the tracker to skip subtrees.

- [ ] `invalidate_all()` is called on resize, theme change, font change, and scale factor change — these are genuinely global invalidations.

---

## 02.3 Widget Dirty Propagation

**File(s):** `oriterm_ui/src/widgets/container/event_dispatch.rs`, `oriterm_ui/src/overlay/manager/event_routing.rs`

When a widget returns a `WidgetResponse` with `RequestPaint` or `RequestLayout`, the container must propagate that upward, recording which widget ID is dirty.

- [ ] **Propagation mechanism -- WidgetId in response chain:** Currently `WidgetResponse` (widgets/mod.rs:68) does not carry the responding widget's `WidgetId`. The `InvalidationTracker` needs to know WHICH widget is dirty. **Complexity warning:** Option (a) below requires touching ~86 call sites across 18 widget source files (every place that calls `WidgetResponse::paint()` or `WidgetResponse::layout()`). Batch this as a mechanical change: search for these calls and add `.with_source(self.id())`. Test files also need updating. Two options:
  - **(a) Add `source: Option<WidgetId>` to `WidgetResponse`** (recommended): Each widget sets `source` to its own `self.id()` when returning `RequestPaint` or `RequestLayout`. Containers propagate the child's source upward. The tracker receives the leaf widget ID.
  - **(a-alt) Container-side source injection** (lower risk): Instead of requiring every widget to call `.with_source()`, have `ContainerWidget::dispatch_mouse()` and `OverlayManager::process_mouse_event()` inject the child's `widget.id()` into the response after receiving it. The container already knows which child it dispatched to. This avoids touching 86 call sites -- only container dispatch points (~5 locations) need updating. **Downside:** leaf widgets at the root (not inside a container) won't have source set. Since all UI is composed via containers, this is acceptable.
  - **(b) Thread `&mut InvalidationTracker` through `EventCtx`**: Event handlers mark the tracker directly. **Downside:** `EventCtx` is currently `&EventCtx<'_>` (shared reference) -- changing to `&mut` would require signature changes across all widget event handlers.
  - **Recommendation:** Option (a-alt) -- container-side injection. Lowest risk, fewest files touched, same end result. If individual widgets need to mark themselves dirty (rare), they can still use `.with_source(self.id())` as an escape hatch.

- [ ] **Sync point:** Adding `source: Option<WidgetId>` to `WidgetResponse` requires updating:
  - `WidgetResponse` struct definition (widgets/mod.rs:68) -- add `pub source: Option<WidgetId>` field
  - All `WidgetResponse` constructors (`handled()`, `paint()`, `layout()`, `focus()`, `ignored()`) -- set `source: None`
  - Add `with_source(id: WidgetId) -> Self` builder method
  - `ContainerWidget::dispatch_mouse()` (container/event_dispatch.rs) -- after child returns, set `response.source = Some(child.id())` if source is None
  - `ContainerWidget::update_dirty()` (container/mod.rs:159) -- extract source from response
  - `OverlayManager::process_mouse_event()` (overlay/manager/event_routing.rs) -- inject `overlay.widget.id()` as source
  - Dialog event handlers in `event_handling.rs` -- pass source to tracker
  - Chrome event handlers in `draw_helpers.rs` -- pass source to tracker
  - Note: with the container-side injection approach, individual widget files do NOT need modification

- [ ] `ContainerWidget::dispatch_mouse()` — after dispatching to a child, if the response is `RequestPaint` or `RequestLayout`, mark the child's `WidgetId` (from `response.source`) in the tracker. Currently this function exists in `event_dispatch.rs` — extend the propagation to record the ID.

- [ ] `ContainerWidget::update_dirty()` — already exists (container/mod.rs:159). Extend to accept an `InvalidationTracker` reference and record the widget ID, not just set boolean flags.

- [ ] `OverlayManager::process_mouse_event()` — same pattern. Mark the overlay's root widget ID, not the whole window.

- [ ] For containers with `clip_children: true`, paint invalidation of a child that is fully outside the clip rect can be ignored (culled at invalidation time, not just at draw time).

---

## 02.4 Render Path Integration

**File(s):** `oriterm/src/app/dialog_rendering.rs`, `oriterm/src/app/redraw/draw_helpers.rs`

Wire the invalidation tracker into the render decision.

- [ ] `render_dialog()` — check `tracker.is_any_dirty()` before doing work. If nothing is dirty, skip the entire render. Currently `render_dialog()` is called unconditionally when `ctx.dirty` is true.

- [ ] `draw_tab_bar()` / `draw_overlays()` in `draw_helpers.rs` — for chrome (tab bar, search bar, overlays), check whether chrome-related widgets are dirty before rebuilding `chrome_draw_list`. Currently `draw_list.clear()` happens unconditionally (draw_helpers.rs:48).

- [ ] **Important:** In this section, the tracker only gates whether to render at all. It does NOT yet skip individual subtrees during draw — that requires Section 03 (scene retention). This section's value is:
  1. Eliminating renders when nothing changed (e.g. mouse move outside any widget).
  2. Providing the data structure that Section 03 consumes.

- [ ] Transition plan for `dirty` / `ui_stale` flags:
  - Phase 1 (this section): `dirty = tracker.is_any_dirty()`. Both coexist.
  - Phase 2 (Section 03): `dirty` is removed. The tracker is the sole source of truth.

---

## 02.5 Completion Checklist

- [ ] `DirtyKind` enum and `InvalidationTracker` are defined and exported from `oriterm_ui`
- [ ] `ContainerWidget` propagates typed dirty signals to the tracker
- [ ] `OverlayManager` propagates typed dirty signals to the tracker
- [ ] `render_dialog()` skips render when tracker reports no dirty widgets
- [ ] `handle_redraw()` skips chrome rebuild when chrome widgets are clean
- [ ] Mouse moves that don't cross widget boundaries produce `DirtyKind::Clean`
- [ ] Hover enter/leave on a button produces `DirtyKind::Paint` for that button only
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./test-all.sh` green

**Tests:** Create as a directory module: `invalidation/mod.rs` + `invalidation/tests.rs`, following the sibling `tests.rs` pattern from `.claude/rules/test-organization.md`. The module will contain both `DirtyKind` and `InvalidationTracker`, so tests are guaranteed to be non-trivial.

- [ ] `DirtyKind::merge()` unit tests: `Clean.merge(Clean) → Clean`, `Clean.merge(Paint) → Paint`, `Paint.merge(Layout) → Layout`, `Layout.merge(Paint) → Layout`
- [ ] `InvalidationTracker`: mark + query correctness (mark paint, query paint-dirty = true, query layout-dirty = false)
- [ ] `InvalidationTracker::clear()` resets all state
- [ ] `InvalidationTracker::invalidate_all()` makes `needs_full_rebuild()` true
- [ ] Container propagation: child returns `RequestPaint` → parent's tracker gets child's `WidgetId` marked paint-dirty, no other IDs marked

**Exit Criteria:** Mouse movement over blank space in a dialog window does not trigger any draw calls. Hover enter/leave on a button marks exactly one `WidgetId` as paint-dirty — no other widgets are marked. Verified by adding a counter to `Widget::draw()` calls and observing it stays at 0 for non-dirty frames.
