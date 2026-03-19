---
section: "07"
title: "WindowRoot Extraction"
status: not-started
reviewed: true
goal: "Extract WindowRoot as oriterm_ui's per-window composition unit — owning widget tree, interaction, focus, overlays, compositor, and pipeline execution — so every layer including windows is headless-testable."
inspired_by:
  - "GPUI Window (zed/crates/gpui/src/window.rs) — Window as entity in App's slot map, owns render root + layout + focus + dispatch tree"
  - "masonry AppRoot/WindowRoot (ARCHITECTURE.md) — AppRoot owns Vec<WindowRoot>, each with independent widget tree"
  - "druid Window<T> (druid/src/window.rs) — Window holds root widget + layout rect + focus + menus"
depends_on: ["01"]
sections:
  - id: "07.1"
    title: "WindowRoot Type Design"
    status: not-started
  - id: "07.2"
    title: "WindowRoot Implementation"
    status: not-started
  - id: "07.3"
    title: "WindowContext & DialogWindowContext Decomposition"
    status: not-started
  - id: "07.4"
    title: "Test Harness Unification"
    status: not-started
  - id: "07.5"
    title: "Completion Checklist"
    status: not-started
---

# Section 07: WindowRoot Extraction

**Status:** Not Started
**Goal:** `oriterm_ui::WindowRoot` becomes the per-window composition unit that owns all pure UI state. `WidgetTestHarness` wraps a `WindowRoot` (not raw fields). `WindowContext` and `DialogWindowContext` wrap a `WindowRoot` plus platform/GPU resources. Every layer — widgets, interaction, focus, overlays, windows — is headless-testable.

**Context:** Today the test harness (`WidgetTestHarness`) and app-layer window contexts (`WindowContext`, `DialogWindowContext`) independently compose overlapping sets of framework types — `InteractionManager`, `FocusManager` (harness + dialog only), `FrameRequestFlags`, `RenderScheduler` (harness only), etc. The window contexts additionally own `OverlayManager`, `LayerTree`, `LayerAnimator`, `DamageTracker`, and `InvalidationTracker` that the harness does not yet have. This duplication and inconsistency means:
- Bugs in framework wiring (e.g. "hover doesn't bubble to this button inside this dialog") can't be tested without launching the full app with a GPU and display server.
- Every new framework feature must be wired in 3 places (harness + 2 window contexts).
- There's no single type that represents "a window's UI state" — the concept is scattered.

GPUI, Masonry, and Druid all solve this by having the framework own a `Window`/`WindowRoot` type. We should too.

**Reference implementations:**
- **GPUI** `zed/crates/gpui/src/window.rs`: `Window` struct owns `RenderRoot`, layout tree, focus state, invalidation flags. Accessed via `WindowHandle<V>`. All windows in `SlotMap<WindowId, Window>`.
- **masonry** `ARCHITECTURE.md`: `AppRoot` owns `Vec<WindowRoot>`, each with independent widget tree and state. `TestHarness` wraps a single `WindowRoot`.
- **druid** `druid/src/window.rs`: `Window<T>` stores root widget, layout rect, focus, menus, timers.

**Depends on:** Section 01 (test harness must exist first — WindowRoot extracts from it).

---

## 07.1 WindowRoot Type Design

**File(s):** `oriterm_ui/src/window_root/mod.rs` (NEW)

`WindowRoot` consolidates the pure UI state that both the test harness and app-layer window contexts independently compose today. It owns no GPU, platform, or terminal-specific state.

**Design principle:** If a field can't be constructed in a `#[test]` without a GPU, window, or display server, it does NOT belong in `WindowRoot`.

- [ ] Define `WindowRoot` struct with fields extracted from `WidgetTestHarness` and `WindowContext`/`DialogWindowContext`:
  ```rust
  /// Per-window UI composition unit.
  ///
  /// Owns the widget tree and all framework state needed to process events,
  /// compute layout, manage focus, and track interaction — without requiring
  /// a GPU, platform window, or terminal. Both `WidgetTestHarness` (testing)
  /// and `WindowContext` (production) wrap this type.
  pub struct WindowRoot {
      // Widget tree
      widget: Box<dyn Widget>,
      layout: LayoutNode,
      viewport: Rect,

      // Framework state
      interaction: InteractionManager,
      focus: FocusManager,
      overlays: OverlayManager,

      // Compositor
      layer_tree: LayerTree,
      layer_animator: LayerAnimator,

      // Animation & scheduling
      frame_requests: FrameRequestFlags,
      scheduler: RenderScheduler,

      // Invalidation & damage
      invalidation: InvalidationTracker,
      damage: DamageTracker,

      // Redraw tracking
      dirty: bool,
      urgent_redraw: bool,

      // Action queue
      pending_actions: Vec<WidgetAction>,
  }
  ```

- [ ] Define the public API surface — accessors and mutators:
  ```rust
  impl WindowRoot {
      // Construction
      pub fn new(widget: impl Widget + 'static) -> Self;
      pub fn with_viewport(widget: impl Widget + 'static, viewport: Rect) -> Self;

      // Widget tree
      pub fn widget(&self) -> &dyn Widget;
      pub fn widget_mut(&mut self) -> &mut dyn Widget;
      pub fn replace_widget(&mut self, widget: Box<dyn Widget>);

      // Layout
      pub fn layout(&self) -> &LayoutNode;
      pub fn viewport(&self) -> Rect;
      pub fn set_viewport(&mut self, viewport: Rect);

      // Framework accessors
      pub fn interaction(&self) -> &InteractionManager;
      pub fn interaction_mut(&mut self) -> &mut InteractionManager;
      pub fn focus(&self) -> &FocusManager;
      pub fn focus_mut(&mut self) -> &mut FocusManager;
      pub fn overlays(&self) -> &OverlayManager;
      pub fn overlays_mut(&mut self) -> &mut OverlayManager;

      // Compositor
      pub fn layer_tree(&self) -> &LayerTree;
      pub fn layer_tree_mut(&mut self) -> &mut LayerTree;
      pub fn layer_animator(&self) -> &LayerAnimator;
      pub fn layer_animator_mut(&mut self) -> &mut LayerAnimator;

      // Animation
      pub fn frame_requests(&self) -> &FrameRequestFlags;
      pub fn frame_requests_mut(&mut self) -> &mut FrameRequestFlags;
      pub fn scheduler(&self) -> &RenderScheduler;
      pub fn scheduler_mut(&mut self) -> &mut RenderScheduler;

      // Invalidation & damage
      pub fn invalidation(&self) -> &InvalidationTracker;
      pub fn invalidation_mut(&mut self) -> &mut InvalidationTracker;

      // Redraw
      pub fn is_dirty(&self) -> bool;
      pub fn mark_dirty(&mut self);
      pub fn clear_dirty(&mut self);
      pub fn is_urgent_redraw(&self) -> bool;
      pub fn set_urgent_redraw(&mut self, urgent: bool);

      // Actions
      pub fn take_actions(&mut self) -> Vec<WidgetAction>;
      pub fn has_pending_actions(&self) -> bool;
  }
  ```

- [ ] Decide whether `WindowRoot` should expose pipeline methods directly (layout, dispatch, prepaint) or leave those as free functions that accept `&mut WindowRoot`. Recommendation: **methods on WindowRoot** — this is the composition unit, it should orchestrate its own pipeline:
  ```rust
  impl WindowRoot {
      /// Recomputes layout from the root widget.
      pub fn compute_layout(&mut self, measurer: &dyn TextMeasurer, theme: &UiTheme);

      /// Dispatches an event through overlays first, then the widget tree.
      pub fn dispatch_event(
          &mut self,
          event: &InputEvent,
          measurer: &dyn TextMeasurer,
          theme: &UiTheme,
          now: Instant,
      );

      /// Runs the pre-paint phase (lifecycle, animation ticks).
      pub fn prepare(&mut self, now: Instant);

      /// Registers all widgets and rebuilds focus order.
      pub fn rebuild(&mut self);
  }
  ```

- [ ] `TextMeasurer` and `UiTheme` parameters for `compute_layout` — the test harness passes `MockMeasurer` + `UiTheme::dark()`, production passes `CachedTextMeasurer<UiFontMeasurer>` (wrapping `TextShapeCache`) + the current theme. This keeps WindowRoot GPU-free and theme-agnostic. Note: the `TextMeasurer` trait already exists at `oriterm_ui/src/widgets/text_measurer.rs` with `measure()` and `shape()` methods — no new trait definition needed.

- [ ] **OverlayManager event routing design:** `OverlayManager` sits alongside the widget tree (not inside it). Today, the app layer calls `overlays.process_mouse_event()` before dispatching to the main widget tree, passing `&MouseEvent`, `&dyn TextMeasurer`, `&UiTheme`, `Option<WidgetId>` (focused widget), `&mut LayerTree`, `&mut LayerAnimator`, and `Instant`. Since WindowRoot owns all of these (`overlays`, `layer_tree`, `layer_animator`, `focus`), WindowRoot can orchestrate this priority routing internally:
  ```rust
  impl WindowRoot {
      /// Dispatches an event through overlays first, then the widget tree.
      ///
      /// Overlay events take priority — if an overlay handles the event,
      /// the main widget tree does not see it.
      pub fn dispatch_event(
          &mut self,
          event: &InputEvent,
          measurer: &dyn TextMeasurer,
          theme: &UiTheme,
          now: Instant,
      );
  }
  ```
  Note: `dispatch_event` needs `measurer` and `theme` because `OverlayManager::process_mouse_event()` requires them for overlay layout computation. The `focused_widget` parameter comes from `self.focus` internally. These external parameters are NOT stored on WindowRoot (they are per-frame resources), so they must be passed in.

  **Internal sequence of `dispatch_event` (must replicate the full pipeline):**
  1. For mouse events: hit test via `layout_hit_test_path`, update `InteractionManager::update_hot_path`.
  2. Drain and deliver lifecycle events from hot path changes (via `prepare_widget_tree`).
  3. Route through `OverlayManager::process_mouse_event()` (overlay priority).
  4. If overlay did not handle: dispatch to widget tree via `deliver_event_to_tree`.
  5. Apply controller requests via `apply_dispatch_requests`.
  6. Drain and deliver lifecycle events from request application.
  7. Collect emitted actions into `pending_actions`.
  8. Forward `PAINT`/`ANIM_FRAME` request flags to `RenderScheduler` and mark dirty as needed.

- [ ] **What WindowRoot does NOT own:** Terminal-specific widgets (`TabBarWidget`, `TerminalGridWidget`), GPU resources (`WindowRenderer`, `PaneRenderCache`), platform handles (`TermWindow`, `Arc<Window>`), drag states with `PaneId` dependencies (`FloatingDragState`, `DividerDragState`, `TabDragState`), and render caches (`Scene`, `FrameInput`, `chrome_scene`). These remain on `WindowContext`/`DialogWindowContext`. WindowRoot is the pure UI framework composition unit — it holds the machinery for processing events and managing interaction state, not the terminal-specific content.

- [ ] **Widget tree ownership differs by window type:**
  - **`DialogWindowContext`**: The dialog content widget (e.g., `SettingsPanel`, `DialogWidget`) plus `WindowChromeWidget` form the widget tree. DialogWindowContext wraps WindowRoot; the content widget is the root widget passed to `WindowRoot::new()`. The chrome widget may remain outside WindowRoot (it has its own layout pass offset by chrome height) or be composed as a parent container wrapping the content.
  - **`WindowContext`**: The tab bar and terminal grid are NOT generic widgets managed by WindowRoot's pipeline. They are terminal-specific, rendered by the GPU renderer, and their interaction is handled by app-level code (tab drag, selection, mouse reporting). WindowContext uses WindowRoot only for framework state (interaction, focus, overlays, compositor) — the widget tree in WindowRoot may be empty or contain only overlay-managed widgets. This is an important design decision to document.

- [ ] **Widget tree rebuild triggers:** When tabs change (add, remove, reorder, switch), the app layer must call `WindowRoot::rebuild()` to re-register widgets and rebuild the focus order. This is triggered from `tab_management.rs` (add/close/switch tab), `tab_drag` (reorder), and `pane_ops` (split/close pane). Today, `register_widget_tree` and `collect_focusable_ids` are called ad hoc after structural changes — WindowRoot consolidates these into a single `rebuild()` call. The caller (App) is responsible for calling `root.rebuild()` after any structural change.

---

**File size plan:** The struct definition (15 fields), accessors (~25 one-liner methods), construction (~2 methods), and pipeline methods (4 methods with nontrivial bodies ~30-50 lines each) total roughly 400-600 lines. To stay under the 500-line limit for `mod.rs`:
- `window_root/mod.rs` — struct definition, `new()`, `with_viewport()`, accessors, predicates, `rebuild()` (~250 lines)
- `window_root/pipeline.rs` — `compute_layout()`, `dispatch_event()`, `prepare()` logic (~200 lines)
- `window_root/tests.rs` — all unit tests (exempt from 500-line limit)

This mirrors the existing pattern where `pipeline.rs` is a separate module for orchestration logic.

---

## 07.2 WindowRoot Implementation

**File(s):** `oriterm_ui/src/window_root/mod.rs`, `oriterm_ui/src/window_root/pipeline.rs`, `oriterm_ui/src/window_root/tests.rs`

- [ ] Implement `WindowRoot::new()` — initializes all framework state, runs initial `rebuild()`. Note: `RenderScheduler` is currently only in `WidgetTestHarness`, not in `WindowContext` or `DialogWindowContext`. Adding it to WindowRoot makes it available in production for the first time — verify event loop integration (see `event_loop.rs` line 457, TODO for `RenderScheduler::next_wake_time()`):
  ```rust
  pub fn new(widget: impl Widget + 'static) -> Self {
      let mut root = Self {
          widget: Box::new(widget),
          layout: LayoutNode::new(Rect::default(), Rect::default()),
          viewport: Rect::new(0.0, 0.0, 800.0, 600.0),
          interaction: InteractionManager::new(),
          focus: FocusManager::new(),
          overlays: OverlayManager::new(Rect::new(0.0, 0.0, 800.0, 600.0)),
          layer_tree: LayerTree::new(Rect::new(0.0, 0.0, 800.0, 600.0)),
          layer_animator: LayerAnimator::new(),
          frame_requests: FrameRequestFlags::new(),
          scheduler: RenderScheduler::new(),
          invalidation: InvalidationTracker::new(),
          damage: DamageTracker::new(),
          dirty: true,
          urgent_redraw: false,
          pending_actions: Vec::new(),
      };
      root.rebuild();
      root
  }
  ```

- [ ] Implement `compute_layout()` — calls widget's `layout()`, runs `compute_layout`, rebuilds parent map, registers widgets, rebuilds focus order. Mirrors the current `WidgetTestHarness::rebuild_layout()` logic.

- [ ] Implement `dispatch_event()` — routes through `OverlayManager::process_mouse_event()` first (for overlay priority), then calls `deliver_event_to_tree` from `input::dispatch::tree` for the main widget tree, collects actions, updates interaction state via `apply_dispatch_requests`. The overlay-first routing mirrors what `dialog_context/event_handling/mod.rs` does today (lines 219-237). Note: `deliver_event_to_tree` (not `dispatch_step`) is the correct entry point — it does the full tree walk with hit testing and propagation. **Borrow splitting:** `dispatch_event` needs simultaneous `&mut` access to `self.overlays`, `self.layer_tree`, `self.layer_animator`, and `self.widget`. Use field destructuring (`let Self { overlays, layer_tree, .. } = self;`) to satisfy the borrow checker.

- [ ] Implement `prepare()` — calls `prepare_widget_tree` for lifecycle delivery and animation ticks. Mirrors the dialog_rendering pre-paint path.

- [ ] Implement `rebuild()` — registers widget tree with InteractionManager, rebuilds focus order. Called after structural changes.

- [ ] Add `window_root` module to `oriterm_ui/src/lib.rs`.

- [ ] Unit tests in `oriterm_ui/src/window_root/tests.rs`:
  - Construct a WindowRoot with a simple widget, verify layout computes.
  - Dispatch a mouse move, verify interaction state updates.
  - Dispatch a click on a button, verify action fires.
  - Replace widget, verify rebuild works.
  - Test dirty/urgent_redraw flag management.
  - Push a popup overlay, dispatch a click inside it, verify overlay handles the event (widget tree does not see it).
  - Push a popup overlay, dispatch a click outside it, verify overlay is dismissed and event reaches widget tree.
  - Verify `rebuild()` re-registers widgets and rebuilds focus order after structural changes.

---

## 07.3 WindowContext & DialogWindowContext Decomposition

**File(s):** `oriterm/src/app/window_context.rs`, `oriterm/src/app/dialog_context/mod.rs`

Slim down `WindowContext` and `DialogWindowContext` to wrap `WindowRoot` plus platform/GPU-specific state. The framework state they currently own individually moves into their `WindowRoot`.

- [ ] Add `WindowRoot` field to `WindowContext`, remove individual framework fields:
  ```rust
  // BEFORE (WindowContext) — fields that move into WindowRoot:
  interaction: InteractionManager,
  frame_requests: FrameRequestFlags,
  overlays: OverlayManager,
  layer_tree: LayerTree,
  layer_animator: LayerAnimator,
  invalidation: InvalidationTracker,
  damage: DamageTracker,
  dirty: bool,
  urgent_redraw: bool,
  // NOTE: WindowContext does NOT currently have FocusManager or RenderScheduler.
  // WindowRoot will add those (FocusManager for keyboard nav, RenderScheduler
  // for animation scheduling). WindowContext also has ui_stale: bool which may
  // stay outside WindowRoot (GPU-specific concern) or move in.

  // AFTER:
  root: WindowRoot,  // owns all of the above + FocusManager + RenderScheduler
  ```

  > **Behavior change:** Adding `FocusManager` and `RenderScheduler` to `WindowContext` via `WindowRoot` is not a pure refactor — it introduces keyboard Tab navigation and animation scheduling in terminal windows for the first time. Both need explicit integration points: focus order rebuild after tab changes, scheduler wake times fed into the event loop. These must be wired in this section, not deferred.

- [ ] Add `WindowRoot` field to `DialogWindowContext`, remove individual framework fields:
  ```rust
  // BEFORE (DialogWindowContext) — fields that move into WindowRoot:
  interaction: InteractionManager,
  focus: FocusManager,
  frame_requests: FrameRequestFlags,
  overlays: OverlayManager,
  layer_tree: LayerTree,
  layer_animator: LayerAnimator,
  invalidation: InvalidationTracker,
  damage: DamageTracker,
  dirty: bool,
  urgent_redraw: bool,
  // NOTE: DialogWindowContext also has lifecycle: SurfaceLifecycle and
  // cached_layout: Option<(Rect, Rc<LayoutNode>)> which may stay outside
  // WindowRoot (surface lifecycle is GPU-specific; cached_layout is
  // dialog-specific optimization).

  // AFTER:
  root: WindowRoot,  // owns all of the above
  ```

- [ ] Update all call sites that access framework state through `WindowContext` to go through `root`:
  - `self.interaction` → `self.root.interaction()`
  - `self.focus` → `self.root.focus()`
  - `self.overlays` → `self.root.overlays()`
  - `self.layer_tree` → `self.root.layer_tree()`
  - `self.dirty` → `self.root.is_dirty()`
  - etc.

  > **WARNING:** This is a large mechanical refactor with many call sites across
  > `oriterm/src/app/`. Use find-and-replace carefully. The compiler will catch
  > every missed site (field access on a type that no longer has that field).

- [ ] Update `WindowContext` construction sites (in `window_management.rs`, `init/mod.rs`) to create a `WindowRoot` and pass it in.

- [ ] Update `DialogWindowContext` construction site (in `dialog_management.rs`) similarly.

- [ ] Verify all pipeline calls (`prepare_widget_tree`, `deliver_event_to_tree`, etc.) now go through `WindowRoot` methods rather than being called directly with individual framework fields.

- [ ] Ensure `WindowContext` retains only platform/GPU/terminal-specific fields:
  - `window: TermWindow` — platform window
  - `renderer: Option<WindowRenderer>` — GPU renderer
  - `tab_bar: TabBarWidget`, `terminal_grid: TerminalGridWidget` — terminal widgets
  - `pane_cache: PaneRenderCache` — terminal pane rendering
  - `frame: Option<FrameInput>` — GPU frame input
  - `chrome_scene: Scene` — reusable Scene for chrome rendering
  - `last_rendered_pane: Option<PaneId>` — pane contamination detection
  - `cached_dividers: Option<Vec<DividerLayout>>` — layout cache
  - `tab_slide: TabSlideState` — tab slide animation state
  - Drag states (`floating_drag`, `divider_drag`, `tab_drag`), context menu, URL hover — terminal interaction (all depend on `PaneId`/`TabId` from `oriterm_mux`/`session`)
  - `hovering_divider`, `last_drag_area_press` — mouse interaction state
  - `search_bar_buf: String` — reusable search buffer
  - `ui_stale: bool` — GPU-specific chrome/overlay content staleness flag
  - `render_strategy`, `damage` — surface strategy (currently dead_code, retained-UI plan vocabulary)
  - `text_cache: TextShapeCache` — production text shaping cache (`CachedTextMeasurer` is constructed from it and passed to `root.compute_layout()`)

---

## 07.4 Test Harness Unification

**File(s):** `oriterm_ui/src/testing/harness.rs`

Refactor `WidgetTestHarness` to wrap a `WindowRoot` instead of owning the same fields independently.

- [ ] Replace raw fields with `WindowRoot`:
  ```rust
  // BEFORE (current harness fields):
  pub struct WidgetTestHarness {
      widget: Box<dyn Widget>,
      layout: LayoutNode,
      interaction: InteractionManager,
      focus: FocusManager,
      scheduler: RenderScheduler,
      clock: Instant,
      measurer: MockMeasurer,
      theme: UiTheme,
      viewport: Rect,
      pending_actions: Vec<WidgetAction>,
      frame_requests: FrameRequestFlags,
      mouse_pos: Point,
      // NOTE: harness currently does NOT have OverlayManager, LayerTree,
      // LayerAnimator, DamageTracker, or InvalidationTracker — those live
      // only in WindowContext/DialogWindowContext. WindowRoot adds them.
  }

  // AFTER:
  pub struct WidgetTestHarness {
      root: WindowRoot,
      clock: Instant,        // test-only: controlled time
      measurer: MockMeasurer, // test-only: mock text measurement
      theme: UiTheme,        // test-only: configurable theme
      mouse_pos: Point,      // test-only: simulated cursor position
  }
  ```

- [ ] Update all harness methods to delegate to `WindowRoot`:
  - `rebuild_layout()` → `self.root.compute_layout(&self.measurer, &self.theme)`
  - Tree-level event dispatch (in `harness_dispatch.rs`) → `self.root.dispatch_event(event, &self.measurer, &self.theme, self.clock)`
  - `interaction()` → `self.root.interaction()`
  - `focus()` → `self.root.focus()`
  - `take_actions()` → `self.root.take_actions()`
  - `find_widget_bounds()` → search `self.root.layout()`

- [ ] Update all test helpers (`harness_dispatch.rs`, `harness_input.rs`, `harness_inspect.rs`) for the new delegation pattern.

- [ ] Verify all existing harness tests still pass — this is a pure refactor, no behavioral change.

- [ ] Add new window-level test capabilities now possible:
  ```rust
  impl WidgetTestHarness {
      /// Returns the WindowRoot for direct framework state access.
      pub fn root(&self) -> &WindowRoot { &self.root }
      pub fn root_mut(&mut self) -> &mut WindowRoot { &mut self.root }
  }
  ```

- [ ] With WindowRoot wrapping, the harness now has `OverlayManager`, `LayerTree`, and `LayerAnimator` for the first time. This resolves the `TODO: OverlayTestHarness for end-to-end overlay flow testing` comment in `harness.rs` (line 40). Add overlay test helpers:
  ```rust
  impl WidgetTestHarness {
      /// Pushes a popup overlay at the given anchor.
      pub fn push_popup(&mut self, widget: impl Widget + 'static, anchor: Rect);
      /// Returns true if any overlays are active.
      pub fn has_overlays(&self) -> bool;
      /// Dismisses all overlays.
      pub fn dismiss_overlays(&mut self);
  }
  ```

---

## 07.5 Completion Checklist

- [ ] `oriterm_ui/src/window_root/mod.rs` stays under 500 lines (pipeline logic in `pipeline.rs` submodule)
- [ ] `WindowRoot` type exists in `oriterm_ui/src/window_root/mod.rs`
- [ ] `WindowRoot` owns: widget tree, layout, interaction, focus, overlays, compositor, scheduler, invalidation, dirty flags, action queue
- [ ] `WindowRoot` has NO GPU, platform, or terminal dependencies
- [ ] `WindowRoot::new()` constructs a fully functional instance in a `#[test]`
- [ ] `WindowRoot::compute_layout()` accepts `&dyn TextMeasurer` + `&UiTheme` (not concrete GPU types)
- [ ] `WindowRoot::dispatch_event()` processes events through the full pipeline
- [ ] `WidgetTestHarness` wraps `WindowRoot` (not raw fields)
- [ ] All existing harness tests pass without modification
- [ ] `WindowContext` wraps `WindowRoot` + platform/GPU state
- [ ] `DialogWindowContext` wraps `WindowRoot` + platform/GPU state
- [ ] No duplicate framework field declarations across WindowRoot, WindowContext, DialogWindowContext
- [ ] `WindowRoot::dispatch_event()` routes overlay events before widget tree events
- [ ] Overlay test helpers available on `WidgetTestHarness` (push_popup, has_overlays, dismiss_overlays)
- [ ] `RenderScheduler` is available in production via `WindowRoot` (previously test-only)
- [ ] Unit tests for WindowRoot in `oriterm_ui/src/window_root/tests.rs`
- [ ] `timeout 150 cargo test -p oriterm_ui` passes
- [ ] `timeout 150 cargo test -p oriterm` passes
- [ ] `./clippy-all.sh` clean
- [ ] `./build-all.sh` clean

**Exit Criteria:** A `WindowRoot` can be constructed in a `#[test]` with no GPU or platform dependencies. A test can build a widget hierarchy (e.g., Dialog → Panel → Button), dispatch events through `WindowRoot`, and assert that interaction state, focus, and actions behave correctly — all headlessly. `WidgetTestHarness` and both `WindowContext`/`DialogWindowContext` use `WindowRoot` as their composition core, eliminating all duplicated framework wiring.
