---
section: "08"
title: "Widget Trait Overhaul"
status: in-progress
goal: "New Widget trait shape integrating Sense, controllers, visual states, and lifecycle; all existing widgets migrated"
inspired_by:
  - "Druid Widget trait (druid/src/widget.rs)"
  - "GTK4 Widget + EventController composition"
depends_on: ["01", "02", "03", "04", "05", "06", "07"]
reviewed: true
sections:
  - id: "08.0"
    title: "Prerequisites (Context Extraction & File Size)"
    status: complete
  - id: "08.1"
    title: "New Widget Trait"
    status: complete
  - id: "08.1a"
    title: "Framework Orchestration (Per-Frame Widget Pipeline)"
    status: not-started
  - id: "08.1b"
    title: "Custom Controllers (TextEdit, TerminalInput, MenuKey, DropdownKey)"
    status: not-started
  - id: "08.2"
    title: "Migration Strategy"
    status: not-started
  - id: "08.3"
    title: "Migrate Interactive Widgets"
    status: not-started
  - id: "08.4"
    title: "Migrate Layout Widgets"
    status: not-started
  - id: "08.5"
    title: "Migrate Passive Widgets"
    status: not-started
  - id: "08.6"
    title: "Remove Legacy Event Methods"
    status: not-started
  - id: "08.7"
    title: "Completion Checklist"
    status: not-started
---

# Section 08: Widget Trait Overhaul

**Status:** Not Started

**Goal:** The `Widget` trait evolves from the current shape (separate `handle_mouse`,
`handle_hover`, `handle_key` methods plus `id`, `is_focusable`, `layout`, `draw`,
`accept_action`, `focusable_children`, `sense`, `hit_test_behavior`) to the new shape
(controller composition, visual state groups, lifecycle method, paint rename). All 26
existing Widget implementations are migrated (24 in `oriterm_ui` + 2 in `oriterm`).
No regressions in behavior.

**Context:** This is the convergence point where all prior sections come together. The new
trait must support the framework-managed interaction state (Section 01), Sense filtering
(Section 02), two-phase event propagation (Section 03), composable controllers (Section 04),
animation frames (Section 05), visual state management (Section 06), and new layout/theme
capabilities (Section 07). Every existing widget must be migrated without breaking the
settings dialog, tab bar, or any other UI element.

**Depends on:** All prior sections (01-07).

---

## 08.0 Prerequisites (Context Extraction & File Size)

**This subsection must complete before ANY other 08.x work begins.** It creates
headroom in files that are near the 500-line limit and establishes the `contexts.rs`
module that 08.1 depends on.

### 08.0a Extract `contexts.rs` from `widgets/mod.rs`

**File(s):** `oriterm_ui/src/widgets/mod.rs` (492 lines), `oriterm_ui/src/widgets/contexts.rs` (new)

`widgets/mod.rs` is currently 492 lines (verified 2026-03-16) — already at the 500-line
hard limit. Adding `LifecycleCtx`, `AnimCtx`, and new trait methods will push it over.
**Extract context types first**: move `DrawCtx`, `EventCtx`, `LayoutCtx` (plus their
`impl` blocks) into a `widgets/contexts.rs` submodule, then add new types there.

**Extraction blast radius for `contexts.rs`**: Moving `DrawCtx`, `EventCtx`, `LayoutCtx`
out of `widgets/mod.rs` into `widgets/contexts.rs` changes the import path. Every file
that imports these types via `super::DrawCtx` (widget impl files) or
`crate::widgets::DrawCtx` (other modules) must update. Known import sites:
- **`oriterm_ui` internal** (~30 files): All widget `mod.rs` files import from `super::`,
  which still works if `contexts.rs` is declared in `widgets/mod.rs` and re-exported.
  Use `pub mod contexts;` + `pub use contexts::{DrawCtx, EventCtx, LayoutCtx};` in
  `widgets/mod.rs` to keep existing `super::DrawCtx` imports working.
- **`oriterm` binary crate** (~10 files): Imports `oriterm_ui::widgets::DrawCtx`. The
  re-export ensures no change needed.
- **Strategy**: Use re-exports to make this a zero-blast-radius extraction. The only new
  file is `widgets/contexts.rs`. No existing imports break.

- [x] Create `oriterm_ui/src/widgets/contexts.rs`
- [x] Move `DrawCtx`, `EventCtx`, `LayoutCtx` structs and their `impl` blocks from
  `widgets/mod.rs` into `contexts.rs`. This removes ~290 lines from `mod.rs` (lines
  ~206-492 contain the three structs + impls), leaving ~200 lines for the trait
  definition + module declarations + re-exports.
- [x] Add `pub mod contexts;` to `widgets/mod.rs` and re-export:
  `pub use contexts::{DrawCtx, EventCtx, LayoutCtx};`
- [x] Verify `./build-all.sh` green (all existing imports work via re-exports)

**File size projections after extraction:**
- `widgets/mod.rs`: ~200 lines (trait + mod declarations + re-exports)
- `widgets/contexts.rs`: ~340 lines (3 structs + 3 impl blocks + new LifecycleCtx/AnimCtx)

### 08.0b File size audit for migration targets

**WARNING**: Several widget files are already near the 500-line limit. Migration adds
`controllers` field, `animator` field, `sense()` override, `paint()` impl, and removes
`handle_mouse/hover/key` — net line change varies, but the intermediate state (both old
and new methods present) temporarily INCREASES file size. Files at risk:

| File | Current Lines | Risk |
|------|--------------|------|
| `scroll/mod.rs` | 494 | **CRITICAL** — must split before migration |
| `tab_bar/widget/mod.rs` | 486 | **CRITICAL** — must split before migration |
| `tab_bar/widget/draw.rs` | 478 | **HIGH** — near limit |
| `settings_panel/mod.rs` | 484 | **CRITICAL** — must split before migration |
| `window_chrome/mod.rs` | 463 | **HIGH** — near limit |
| `container/mod.rs` | 462 | **HIGH** — already has `layout_build.rs` extraction |
| `dialog/mod.rs` | 490 | **CRITICAL** — must split before migration |
| `form_section/mod.rs` | 434 | **MODERATE** — may need split |

**Pre-migration splits required (do these BEFORE any Wave 1-4 work on the file):**
- [x] `scroll/mod.rs` (494→305 lines): Extracted `draw()` into `scroll/rendering.rs`
  (50 lines). Extracted `handle_mouse()`/`handle_hover()`/`handle_key()` into
  `scroll/event_handling.rs` (201 lines). Widget trait impl delegates via thin methods.
- [x] `dialog/mod.rs` (490→327 lines): Extracted `handle_mouse()`/`handle_hover()`/
  `handle_key()` + helper methods (`map_button_click`, `update_button_hover`,
  `clear_button_hover`) into `dialog/event_handling.rs` (207 lines).
- [x] `settings_panel/mod.rs` (484→403 lines): Extracted `handle_mouse()`/`handle_hover()`/
  `handle_key()` into `settings_panel/event_handling.rs` (125 lines).
- [x] `tab_bar/widget/mod.rs` (487 lines): Event handlers already stubs (no logic to
  extract). Created `tab_bar/widget/event_handling.rs` (64 lines) with stub delegation.
  Widget trait impl in `draw.rs` delegates to stubs. File stays under 500 with ~13 lines
  headroom (migration adds ~5 lines for struct fields + sense override).
- [x] Verify `./build-all.sh`, `./clippy-all.sh`, `./test-all.sh` green after all splits

---

## 08.1 New Widget Trait

**File(s):** `oriterm_ui/src/widgets/mod.rs`, `oriterm_ui/src/widgets/contexts.rs`

**PREREQUISITE**: Section 08.0 (contexts.rs extraction) must be complete before this
subsection starts. `widgets/mod.rs` must be under 300 lines with contexts extracted.

- [x] Define the new trait shape (additions/changes only — `sense()` and
  `hit_test_behavior()` already exist from Section 02 with their current defaults):
  ```rust
  pub trait Widget {
      /// Unique identifier for this widget instance.
      fn id(&self) -> WidgetId;

      /// What interactions this widget cares about. [EXISTING — Section 02]
      /// Default changes from Sense::all() to Sense::none() after migration.
      fn sense(&self) -> Sense { Sense::all() /* temporary backward compat */ }

      /// Hit test behavior. [EXISTING — Section 02]
      fn hit_test_behavior(&self) -> HitTestBehavior { HitTestBehavior::DeferToChild }

      /// Layout descriptor (unchanged from current).
      fn layout(&self, ctx: &LayoutCtx) -> LayoutBox;

      /// Paint the widget. Use ctx.is_hot(), ctx.is_active(), ctx.is_focused()
      /// for interaction-dependent rendering. Use visual state animator for
      /// property interpolation.
      fn paint(&self, ctx: &mut DrawCtx);

      /// Handle lifecycle events (hot/active/focus changes, widget added/removed).
      fn lifecycle(&mut self, event: &LifecycleEvent, ctx: &mut LifecycleCtx) {}

      /// Handle animation frame (called when anim_frame was requested).
      fn anim_frame(&mut self, event: &AnimFrameEvent, ctx: &mut AnimCtx) {}

      /// Event controllers attached to this widget.
      fn controllers(&self) -> &[Box<dyn EventController>] { &[] }

      /// Mutable access to controllers (for event dispatch).
      /// Default returns empty slice. Widgets with controllers override this.
      fn controllers_mut(&mut self) -> &mut [Box<dyn EventController>] { &mut [] }

      /// Visual state groups (for automatic state resolution and animation).
      fn visual_states(&self) -> Option<&VisualStateAnimator> { None }
      fn visual_states_mut(&mut self) -> Option<&mut VisualStateAnimator> { None }

      /// Whether this widget is focusable (derived from sense).
      fn is_focusable(&self) -> bool { self.sense().has_focus() }

      /// Children for focus traversal and hit testing.
      /// NOTE: Current default returns vec![self.id()] when is_focusable().
      /// Changing to vec![] is a semantic break — leaf widgets that relied on
      /// the auto-include must now explicitly override. Prefer keeping the
      /// current default unless containers are refactored to not depend on it.
      fn focusable_children(&self) -> Vec<WidgetId> {
          if self.is_focusable() { vec![self.id()] } else { Vec::new() }
      }

      /// Accept an action from a child (for overlay/popup propagation).
      fn accept_action(&mut self, action: &WidgetAction) -> bool { false }
  }
  ```
- [ ] Note: `handle_mouse()`, `handle_hover()`, `handle_key()` are REMOVED.
  All input handling moves to controllers. (Deferred to 08.6 — methods have
  default impls returning `ignored()` for now.)
- [x] `draw()` renamed to `paint()` for clarity (Druid convention).
  `draw()` deprecated with default empty impl; `paint()` added with default
  forwarding to `draw()`. All 14 production call sites + test call sites
  updated from `.draw()` to `.paint()`.
- [x] `is_focusable()` replaced by `sense().has_focus()` (default impl).
  Note: `Sense::focusable()` is the constructor for focus-only sense.
- [x] `accept_action()` and `focusable_children()` retained with same semantics.
  `sense()` and `hit_test_behavior()` already exist (Section 02) — no new addition
  needed, but `sense()` default changes from `Sense::all()` to `Sense::none()` after
  all widgets provide explicit overrides.
  **CRITICAL ORDERING**: The `sense()` default change (`Sense::all()` -> `Sense::none()`)
  is a **silent breaking change** — any widget that doesn't explicitly override `sense()`
  becomes invisible to hit testing. This must happen ONLY after all 26 widgets provide
  explicit `sense()` overrides. Verify with a grep for `fn sense` in all Widget impl
  blocks before changing the default. Add a `debug_assert!` in the hit test function
  that warns if a widget with `Sense::none()` previously had `Sense::all()` (detecting
  forgotten overrides). Alternatively, keep `Sense::all()` as the default permanently
  and rely on explicit overrides for correct behavior.
- [ ] `WidgetResponse` and `CaptureRequest` types removed; controllers emit actions and
  request capture via `ControllerCtx` instead. (Deferred to 08.6.)
- [x] **`draw()` / `paint()` coexistence during migration**: During step 1 (additive),
  both `draw()` and `paint()` must exist on the trait. Strategy:
  1. Add `paint()` with a default that calls `self.draw(ctx)` (backward compat).
  2. Mark `draw()` as `#[deprecated(note = "use paint()")]`.
  3. Migrate widgets one at a time: implement `paint()`, remove `draw()` body
     (leave the trait method until all widgets are done).
  4. After all widgets implement `paint()`, remove `draw()` from the trait.
  **Container widgets** that call `child.draw()` must call `child.paint()` instead.
  During the transition, containers can call `child.paint()` immediately (the default
  impl forwards to `draw()`). This means container migration (changing `child.draw()`
  to `child.paint()`) can happen in any wave, even before leaf widgets are migrated.
  The `compose_scene()` function in `draw/scene_compose/mod.rs` (line 29) also calls
  `root.draw(ctx)` and must be updated to `root.paint(ctx)`.
- [x] **Migration ordering**: The trait change cannot be atomic (changing the trait
  signature breaks all 26 implementations simultaneously). Strategy:
  1. Add new methods with default impls to the existing `Widget` trait (backward compatible)
  2. Migrate widgets one wave at a time (implement new methods, stop using old ones)
  3. After all widgets are migrated, remove old methods
  This means the trait temporarily has both old and new methods. The old methods
  get `#[deprecated]` markers in step 1 to ensure nothing new uses them.
  **Done:** `draw()` deprecated, `handle_mouse/hover/key` given default impls.
  `paint/lifecycle/anim_frame/controllers/visual_states` added with defaults.
- [x] **`LifecycleCtx` and `AnimCtx` definition**: The new `lifecycle()` and
  `anim_frame()` methods reference `LifecycleCtx` and `AnimCtx` context types that
  are not defined elsewhere in the plan. Define them in the extracted
  `oriterm_ui/src/widgets/contexts.rs` submodule (prerequisite 08.0a).
  Use the same request mechanism as `ControllerCtx` (Section 04) for consistency:
  ```rust
  pub struct LifecycleCtx<'a> {
      pub widget_id: WidgetId,
      /// Per-widget interaction state (same type as ControllerCtx::interaction).
      /// Use `&InteractionState`, NOT `&InteractionManager` — widgets should only
      /// see their own state, matching the ControllerCtx pattern.
      pub interaction: &'a InteractionState,
      pub requests: ControllerRequests,  // same bitflag type as ControllerCtx
  }

  pub struct AnimCtx<'a> {
      pub widget_id: WidgetId,
      pub now: Instant,
      pub requests: ControllerRequests,
      /// Shared frame request flags so widgets can call `request_anim_frame()`
      /// to continue animation, or `request_paint()` for a repaint without
      /// another anim frame. Without this, widgets have no way to signal that
      /// animation should continue after `anim_frame()` returns.
      pub frame_requests: Option<&'a FrameRequestFlags>,
  }

  impl AnimCtx<'_> {
      /// Request another animation frame on the next vsync.
      /// Call this when `animator.is_animating(now)` is true.
      pub fn request_anim_frame(&self) {
          if let Some(flags) = self.frame_requests {
              flags.request_anim_frame();
          }
      }

      /// Request a repaint without an animation frame.
      pub fn request_paint(&self) {
          if let Some(flags) = self.frame_requests {
              flags.request_paint();
          }
      }
  }
  ```
  After the lifecycle/anim_frame call returns, the framework reads `requests` and
  applies side effects (same pattern as controller dispatch).

---

## 08.1a Framework Orchestration (Per-Frame Widget Pipeline)

**File(s):** App layer in `oriterm` crate (specific file TBD — likely
`oriterm/src/app/event_dispatch.rs` or a new `oriterm/src/app/widget_pipeline.rs`).

This subsection defines the per-frame pipeline that the application layer executes for
each widget. Without this, the `lifecycle()`, `anim_frame()`, `visual_states_mut()`,
and `paint()` methods are defined but never called in the correct order.

**RENDERING DISCIPLINE**: Steps 1-4a below are **state mutation** that happens BEFORE
`draw_frame()`. The GPU renderer's `draw_frame()` must remain pure computation (reads
state, builds instance buffers, no side effects). The pipeline below runs in the event
handler (e.g., `RedrawRequested` or a dedicated pre-render phase), NOT inside the GPU
render pass. `paint()` (step 4b) populates `DrawList` commands which `draw_frame()`
later consumes to build GPU instance buffers.

- [ ] **Per-frame widget pipeline** (executed by the app layer BEFORE `draw_frame()`):
  ```
  1. Drain lifecycle events from InteractionManager (HotChanged, FocusChanged, etc.)
  2. For each lifecycle event targeting widget W:
     a. If W has controllers: dispatch_lifecycle_to_controllers(W.controllers_mut(), event, args)
     b. Call W.lifecycle(event, &mut lifecycle_ctx)
  3. For each widget W that requested an anim frame (from RenderScheduler):
     a. Call W.anim_frame(&anim_frame_event, &mut anim_ctx)
     b. Read anim_ctx.requests and apply side effects
  4. Before painting widget W:
     a. If W.visual_states_mut() returns Some(animator):
        - Call animator.update(&interaction_state, now)   [mutation — pre-render]
        - Call animator.tick(now)                          [mutation — pre-render]
        - If animator.is_animating(now): request_anim_frame for W
     b. Call W.paint(&mut draw_ctx)   [populates DrawList — no state mutation]
  ```

- [ ] **Widget tree traversal for the pipeline**: The pipeline must visit widgets in
  tree order (parent before child for lifecycle delivery, child before parent for
  paint in some Z-order cases). The current container-driven traversal
  (`ContainerWidget::draw()` loops through children) is replaced by the framework
  doing a tree walk. During the transition period, containers still call
  `child.paint()` directly, and the framework inserts steps 1-4a before each
  `child.paint()` call. After migration, the framework owns the full traversal.

- [ ] **`DispatchResult` (deferred from Section 03.2)**: Now needed. Define at the
  app/caller layer. The delivery loop calls `dispatch_to_controllers()` for each
  `DeliveryAction` from `plan_propagation()`, accumulates results into `DispatchResult`,
  and applies side effects (`SET_ACTIVE` -> `InteractionManager::set_active()`, etc.).

  **Note**: Section 03.2 sketched a transitional `DispatchResult` with an
  `effect: EventResponse` field. This section defines the final form that uses
  `ControllerRequests` (from Section 04) instead. The transitional version is
  never implemented -- only this final form is built.

  ```rust
  pub struct DispatchResult {
      pub handled: bool,
      pub actions: Vec<WidgetAction>,
      pub requests: ControllerRequests,
      pub source: Option<WidgetId>,
  }
  ```

- [ ] **OverlayManager integration** (**HIGH RISK -- 348-line rewrite**):
  Overlay widgets must participate in the same pipeline.
  `OverlayManager::process_mouse_event()` (348 lines in
  `overlay/manager/event_routing.rs`) currently calls `handle_mouse()` /
  `handle_hover()` / `handle_key()` directly. Replace with:
  1. Hit test the overlay's layout tree (overlay has its own `LayoutNode` root).
  2. `plan_propagation()` for the overlay hit path.
  3. Delivery loop using `dispatch_to_controllers()` per `DeliveryAction`.
  This is the same pipeline used for the main widget tree. The overlay just has
  a separate layout root.

  **WARNING — Overlay modal semantics**: Modal overlays currently consume ALL events
  (clicks outside the overlay dismiss it). This behavior must be preserved. The new
  pipeline must check whether the hit point is inside the overlay bounds BEFORE
  running `plan_propagation()`. If outside and modal, emit `DismissOverlay` action
  without propagating. This is NOT handled by the generic pipeline — it is
  overlay-specific logic that must wrap the pipeline call.

  **WARNING — Three caller sites** in `oriterm` binary:
  - `app/mouse_input.rs` (1 site)
  - `app/dialog_context/event_handling/mouse.rs` (1 site)
  - `app/dialog_context/event_handling/mod.rs` (1 site)
  All three must be updated simultaneously (old methods are removed in 08.6).

---

## 08.1b Custom Controllers (TextEdit, TerminalInput, MenuKey, DropdownKey)

**File(s):** New files in `oriterm_ui/src/controllers/` and `oriterm/src/widgets/`

Several widgets require custom controllers for widget-specific keyboard logic that
cannot be handled by the generic FocusController. These must be implemented BEFORE
the migration waves that depend on them.

**IMPORTANT**: These are all directory modules per test-organization.md (each `mod.rs`
gets a sibling `tests.rs`).

- [ ] **`TextEditController`** — `oriterm_ui/src/controllers/text_edit/mod.rs` (~120 lines)
  + `controllers/text_edit/tests.rs`
  - Handles: cursor movement (Left/Right/Home/End), text selection (Shift+arrow),
    clipboard (Ctrl+C/V/X), character input, Backspace/Delete.
  - Phase: `EventPhase::Bubble` (default).
  - Consumes all `KeyDown`/`KeyUp` events when the widget is focused (returns `true`).
  - Emits `WidgetAction::TextChanged(WidgetId)` on text modification.
  - **Dependency**: Used by `TextInputWidget` (Wave 1, 08.3).
  - Add `mod text_edit;` + `pub use text_edit::TextEditController;` to
    `controllers/mod.rs`.
  - File size projection: ~120 lines. Well under 500-line limit.

- [ ] **`DropdownKeyController`** — `oriterm_ui/src/controllers/dropdown_key/mod.rs` (~60 lines)
  + `controllers/dropdown_key/tests.rs`
  - Handles: Up/Down arrow (change selection), Enter (confirm), Escape (close dropdown).
  - Phase: `EventPhase::Bubble`.
  - Requires mutable access to dropdown state (selected index, open/closed).
  - **NOTE**: Keyboard input must go through controllers, not `Widget::lifecycle()`.
    Lifecycle events are for state changes (HotChanged, FocusChanged, WidgetAdded/Removed),
    not for input routing. Keyboard input must flow through the controller pipeline.
  - Emits `WidgetAction::Selected(WidgetId, usize)` on Enter confirm.
  - **Dependency**: Used by `DropdownWidget` (Wave 1, 08.3).

- [ ] **`MenuKeyController`** — `oriterm_ui/src/controllers/menu_key/mod.rs` (~60 lines)
  + `controllers/menu_key/tests.rs`
  - Handles: ArrowUp/Down (navigate items), Enter/Space (select), Escape (dismiss).
  - Phase: `EventPhase::Bubble`.
  - Emits `WidgetAction::Clicked(WidgetId)` on selection,
    `WidgetAction::DismissOverlay` on Escape.
  - **Dependency**: Used by `MenuWidget` (Wave 2, 08.4).

- [ ] **`TerminalInputController`** — `oriterm/src/widgets/terminal_grid/input_controller.rs`
  (~30 lines) (NOTE: this lives in the `oriterm` binary crate, NOT `oriterm_ui`,
  because it is tightly coupled to terminal grid behavior)
  - "Claim all" controller: returns `true` (consumed) for ALL `MouseDown`, `MouseUp`,
    `MouseMove`, `KeyDown`, `KeyUp` events.
  - The actual terminal input dispatch (sending to PTY, updating grid) stays at the
    app layer — this controller only prevents events from bubbling past the terminal grid.
  - Phase: `EventPhase::Target`.
  - File is NOT a directory module (no tests needed — it is a trivial catch-all with
    one `match` that returns `true`).
  - **Dependency**: Used by `TerminalGridWidget` (Wave 4, 08.2).

- [ ] `./build-all.sh` green, `./clippy-all.sh` green, `./test-all.sh` green

---

## 08.2 Migration Strategy

Migrate widgets in four waves: interactive (most complex), layout/chrome (including TabBar),
passive (simplest), and cross-crate.

**Dependency ordering for this section:**
```
08.0 (prerequisites) → 08.1 (trait additive changes) → 08.1a (orchestration)
                                                      → 08.1b (custom controllers)
                                                      → 08.2 waves begin
    Wave 1 (08.3) — requires 08.1b (TextEditController, DropdownKeyController)
    Wave 2 (08.4) — requires 08.1b (MenuKeyController), 08.0b (file size splits)
    Wave 3 (08.5) — requires 08.1 only (simplest migration)
    Wave 4 (08.2 cross-crate) — requires 08.1, 08.1b (TerminalInputController)
    All waves complete → 08.6 (remove legacy methods)
```
Waves 1-4 can run in parallel with each other (they touch different files), but ALL
require 08.0 + 08.1 + 08.1b to be complete first. 08.6 requires ALL waves complete.

- [ ] **Wave 1 -- Interactive widgets** (have event handlers, hover state):
  ButtonWidget, ToggleWidget, CheckboxWidget, DropdownWidget, SliderWidget, TextInputWidget
  - Extract hover logic -> HoverController
  - Extract click logic -> ClickController
  - Extract drag logic -> DragController
  - Extract focus logic -> FocusController
  - Replace manual `is_hovered` -> `ctx.is_hot()`
  - Replace manual color interpolation -> VisualStateAnimator

- [ ] **Wave 2 -- Layout/container widgets** (route events to children):
  ContainerWidget, PanelWidget, ScrollWidget, StackWidget, FormLayout,
  FormSection, FormRow, SettingsPanel, DialogWidget, MenuWidget
  - Remove manual event routing (framework handles propagation)
  - Keep layout logic unchanged
  - Update child iteration for new propagation model

- [ ] **Wave 2b -- Chrome/interactive-container widgets:**
  WindowChromeWidget, WindowControlButton, IdOverrideButton
  (detailed migration in Section 08.4)

- [ ] **Wave 2c -- TabBarWidget** (special case -- most complex widget;
  detailed migration in Section 08.4)

- [ ] **Wave 3 -- Passive widgets** (no event handling):
  LabelWidget, SeparatorWidget, SpacerWidget, RichLabel
  - Add `sense() -> Sense::none()` (RichLabel already has this)
  - Rename `draw()` to `paint()`
  - Remove stub `handle_mouse`, `handle_hover`, `handle_key` implementations
  - No other changes needed
  - **Note:** StatusBadge does NOT implement `Widget` — it is a standalone drawing helper
    and does not need migration.

- [ ] **Wave 4 -- Cross-crate widgets** (in `oriterm` crate, not `oriterm_ui`):
  TerminalGridWidget (`oriterm/src/widgets/terminal_grid/mod.rs`, 141 lines),
  TerminalPreviewWidget (`oriterm/src/widgets/terminal_preview/mod.rs`, 106 lines)
  - These live in the binary crate, not the library crate
  - Both implement `Widget` and use `handle_mouse()`, `handle_hover()`, `handle_key()`
  - Both import from `oriterm_ui::input::{HoverEvent, KeyEvent, MouseEvent}`
  - Must migrate to new trait shape (add `sense()`, `paint()`, remove old methods)
  - TerminalGridWidget: `Sense::click_and_drag().union(Sense::focusable())` (receives all input)
  - TerminalPreviewWidget: `Sense::none()` (display only, 106 lines)
  - **TerminalGridWidget input claim**: Currently `handle_mouse()` returns
    `WidgetResponse::handled()` and `handle_key()` returns `WidgetResponse::handled()`
    to claim all input (routing happens at the app layer, not in the widget). After
    migration, this "claim all" behavior is preserved by `TerminalInputController`
    (defined in Section 08.1b) — a trivial controller that returns `true` for all events.
    The app layer handles actual terminal input dispatch (sending to PTY, updating
    grid); the controller just prevents events from bubbling past the terminal grid.
  - **IMPORTANT**: These must be migrated BEFORE Section 08.6 removes the old trait methods.
    Cannot remove old methods from the trait while these still use them. Migration order:
    Wave 4 (implement `paint()`, `sense()`, stop using old methods) -> then 08.6 (remove
    old methods from trait). Wave 4 can run in parallel with Waves 1-3 since it only
    requires the additive step (08.1) to have landed.

---

## 08.3 Migrate Interactive Widgets

**File(s):** `oriterm_ui/src/widgets/button/mod.rs`, `toggle/mod.rs`, `checkbox/mod.rs`,
`dropdown/mod.rs`, `slider/mod.rs`, `text_input/mod.rs` + `text_input/widget_impl.rs`

For each interactive widget:

- [ ] **ButtonWidget**:
  - Remove: `hovered: bool`, `pressed: bool`, `hover_progress: AnimatedValue<f32>`,
    manual `HoverEvent` handling in `handle_hover()`, `current_bg()` helper method
    (replaced by `animator.get_bg_color(now)`)
  - Add: `controllers: Vec<Box<dyn EventController>>` with HoverController + ClickController
  - Add: `animator: VisualStateAnimator` with `common_states()` (Normal/Hovered/Pressed/Disabled)
  - `paint()`: use `animator.get_bg_color(now)` instead of manual interpolation.
    Remove `ctx.animations_running.set(true)` — replaced by
    `if self.animator.is_animating(now) { ctx.request_anim_frame(); }`.
  - `sense()`: `Sense::click()`

- [ ] **ToggleWidget**:
  - Remove: manual hover tracking (`hovered: bool` field)
  - Add: HoverController + ClickController
  - Add: `common_states()` animator for bg color transitions
  - Migrate: `toggle_progress: AnimatedValue<f32>` -> `AnimProperty<f32>` with
    `AnimBehavior::ease_out(100)`. This is the thumb slide animation, NOT a
    VisualStateAnimator property (it animates on value change, not on interaction state).
    Keep it as an explicit `AnimProperty<f32>` field on the widget struct.
  - `sense()`: `Sense::click()`

- [ ] **CheckboxWidget**:
  - Remove: manual hover tracking (`hovered: bool` field)
  - Add: HoverController + ClickController
  - Add: `animator: VisualStateAnimator` with `common_states()` (Normal/Hovered/Pressed/Disabled)
    — currently has no animated hover transition (instant boolean `hovered` check in
    `current_bg()`). The animator adds consistent 100ms EaseOut hover transitions.
  - `sense()`: `Sense::click()`

- [ ] **DropdownWidget**:
  - Remove: manual hover, click handling (`hovered: bool`, `pressed: bool` fields)
  - Add: HoverController + ClickController + FocusController
  - Keep: keyboard arrow navigation via `DropdownKeyController` (defined in Section 08.1b).
    `DropdownWidget::handle_key()` currently handles Up/Down arrow (change selection),
    Enter (confirm), Escape (close). This logic moves to `DropdownKeyController`, which
    is tightly coupled to dropdown state (open/closed, selected index) and emits
    appropriate `WidgetAction` variants.
  - Add: `animator: VisualStateAnimator` with `common_states()` for hover/pressed transitions
    — currently uses instant boolean checks.
  - **Focus ring**: Current `draw()` (line 251) uses `ctx.focused_widget == Some(self.id)`
    directly. In `paint()`, migrate to `ctx.is_interaction_focused()` or use
    `focus_states()` animator for animated focus ring transitions.
  - `sense()`: `Sense::click().union(Sense::focusable())`

- [ ] **SliderWidget**:
  - Remove: manual drag tracking (`dragging: bool`, `hovered: bool` fields)
  - Add: HoverController + DragController + FocusController
  - Add: `animator: VisualStateAnimator` with `common_states()` (Normal/Hovered/Pressed/Disabled)
    — currently has no animated hover transition (instant boolean `hovered` check in
    `track_bg()`/`handle_bg()`). The animator adds consistent transitions.
  - `sense()`: `Sense::drag().union(Sense::focusable())`

- [ ] **TextInputWidget**:
  - Remove: manual click, drag, key handling from `handle_mouse()`, `handle_key()`
  - Add: ClickController + DragController + FocusController
  - **Keyboard handling**: `TextInputWidget::handle_key()` (in `text_input/widget_impl.rs`)
    handles cursor movement (Left/Right/Home/End), text selection (Shift+arrow),
    clipboard (Ctrl+C/V/X), character input, and Backspace/Delete. This logic moves to
    `TextEditController` (defined in Section 08.1b). FocusController handles Tab only.
    TextEditController handles all other `KeyDown`/`KeyUp`.
  - Add: `animator: VisualStateAnimator` with `focus_states()` (Unfocused/Focused) for
    border color animation on focus change.
  - `sense()`: `Sense::click_and_drag().union(Sense::focusable())`

---

## 08.4 Migrate Layout Widgets

**File(s):** `oriterm_ui/src/widgets/container/mod.rs`, `panel/mod.rs`, `scroll/mod.rs`,
`stack/mod.rs`, `form_layout/mod.rs`, `form_section/mod.rs`, `form_row/mod.rs`,
`settings_panel/mod.rs`, `dialog/mod.rs`, `menu/widget_impl.rs`,
`tab_bar/widget/draw.rs` (Widget impl), `tab_bar/widget/mod.rs` (struct + state),
`window_chrome/mod.rs`, `window_chrome/controls.rs`

- [ ] **ContainerWidget** (container/mod.rs: 462 lines — safe after event_dispatch.rs removal):
  - Remove: `ContainerInputState` (in `container/mod.rs`), entire
    `container/event_dispatch.rs` submodule (213 lines — manual per-container child dispatch)
  - Framework handles child event dispatch via propagation pipeline
  - Keep: layout logic unchanged (including `LayoutMode` enum from Section 07 — grid
    mode is a layout concern, not an event concern)
  - Keep: `layout_build.rs` submodule (extracted in Section 07)
  - Add: optional hover tracking via `.with_hover(true)` for setting rows
  - `sense()`: `Sense::none()` by default, `Sense::hover()` when hover enabled

- [ ] **ScrollWidget** (PREREQUISITE: 08.0b split of `scroll/mod.rs` must be complete):
  - Remove: manual scroll handling (`handle_mouse(Scroll)` with delta conversion),
    scrollbar thumb drag, `child_captured: bool` field (child capture is handled by
    the framework's `InteractionManager::active_widget` mechanism), `hovered_scrollbar`
    field, manual `EventCtx` struct literal construction for child dispatch (3 sites
    at lines 316, 358, 387 of `scroll/mod.rs`).
  - Remove: direct `DrawCtx` struct literal construction (line 286-298) — migrate to
    `ctx.for_child()` with child-specific bounds.
  - Add: ScrollController for wheel events
  - Add: DragController for scrollbar thumb drag
  - **Child event routing**: Currently `handle_mouse()` manually routes events to
    `self.child` with adjusted coordinates (subtracting scroll offset). The framework
    propagation pipeline handles this automatically since the child's layout node is
    positioned at the scrolled offset.
  - Keep: clip/translate rendering logic, `draw_scrollbar()` helper.
  - `sense()`: `Sense::drag()` (for scrollbar)

- [ ] **StackWidget**:
  - Remove: `hovered_child: Option<usize>` field, manual back-to-front event routing
    in `handle_mouse()`, `handle_hover()` (tracks hovered child via `HoverEvent::Enter`/
    `Leave`), and `handle_key()` (routes to frontmost child with focus discrimination).
  - Remove: direct `DrawCtx` struct literal construction in `draw()` — currently builds
    `DrawCtx { ... interaction: None, widget_id: None, frame_requests: None }` per child
    instead of using `ctx.for_child()`. Migrate to `ctx.for_child(child.id(), ctx.bounds)`.
  - Framework handles back-to-front event dispatch via hit testing (frontmost child's
    layout is on top, so it wins hit tests naturally).
  - `sense()`: `Sense::none()` (delegates to children)
  - Keep: `focusable_children()` override (flat_maps through children)

- [ ] **PanelWidget**: Rename `draw()` to `paint()`. Add `sense() -> Sense::none()`.
  No event handling changes needed (already uses `ctx.for_child()` for children).
- [ ] **DialogWidget** (PREREQUISITE: 08.0b split of `dialog/mod.rs` must be complete):
  - Remove: manual event routing in `handle_mouse()`, `handle_hover()`, `handle_key()`
    (routes to header buttons and content widget — `dialog/mod.rs` lines 341-480).
  - Header drag (MoveOverlay action) -> DragController on the dialog header region.
  - Close/OK button clicks -> framework propagation to ButtonWidget children.
  - `sense()`: `Sense::none()` (container, but children handle interactions)
  - Keep: `accept_action()` override for dropdown selection routing.
  - Keep: `focusable_children()` override.

- [ ] **FormSection**:
  - Remove: manual event routing in `handle_mouse()`, `handle_hover()`, `handle_key()`
    (routes events to child FormRow widgets).
  - Framework handles child dispatch via propagation pipeline.
  - `sense()`: `Sense::none()`

- [ ] **FormRow**:
  - Remove: manual event routing in `handle_mouse()`, `handle_hover()`, `handle_key()`
    (routes events to its `control: Box<dyn Widget>` child).
  - Framework handles dispatch to the control child.
  - `sense()`: `Sense::none()`
  - Keep: `accept_action()` override for propagating actions to control child.

- [ ] **FormLayout**:
  - Remove: manual event routing in `handle_mouse()`, `handle_hover()`, `handle_key()`
    (routes events to child FormSection widgets).
  - `sense()`: `Sense::none()`

- [ ] **SettingsPanel** (PREREQUISITE: 08.0b split if migration adds lines):
  Rename `draw()` to `paint()`. Add `sense() -> Sense::none()` (delegates all
  interactions to its container child). Remove any stub `handle_mouse`/`handle_hover`/
  `handle_key` methods if present.
- [ ] **WindowChromeWidget** (Wave 2b):
  - Add ClickController for buttons, DragController for titlebar.
  - Remove direct `DrawCtx` struct literal construction (line 263-276 of
    `window_chrome/mod.rs`) -- migrate to `ctx.for_child()`. Remove direct `EventCtx`
    struct literal construction (5 sites in `window_chrome/mod.rs`).
  - `sense()`: `Sense::none()` (children handle clicks).

- [ ] **WindowControlButton** (Wave 2b):
  - Extract hover/press -> HoverController + ClickController.
  - Remove `hover_progress: AnimatedValue<f32>`, `hovered: bool`, `pressed: bool`,
    `current_bg()` helper (line 114 of `window_chrome/controls.rs`). Add
    `animator: VisualStateAnimator` with `common_states()`. Remove
    `ctx.animations_running.set(true)` (1 site in `controls.rs`).
  - `sense()`: `Sense::click()`.

- [ ] **IdOverrideButton** (Wave 2b):
  - Extract click -> ClickController. `sense()`: `Sense::click()`.

- [ ] **TabBarWidget** (Wave 2c -- most complex widget;
  PREREQUISITE: 08.0b split of `tab_bar/widget/mod.rs` if needed):
  TabBarWidget has per-tab hover tracking (`hover_progress: Vec<AnimatedValue<f32>>`),
  per-tab close button opacity, width multiplier animations, drag-to-reorder,
  tab tear-off, and context menu. This widget has its own submodule tree
  (`tab_bar/widget/`, `tab_bar/hit.rs`, `tab_bar/layout.rs`, `tab_bar/slide/`).
  Migration approach:
  - Per-tab hover: each tab gets a HoverController (but tabs are not separate widgets --
    they are painted regions within TabBarWidget). **Decision needed**: either
    (a) convert each tab into a child widget, or (b) keep TabBarWidget as a monolithic
    widget that internally uses the interaction state system.
  - Recommended: option (b) -- TabBarWidget is too tightly integrated to decompose.
    It should use `InteractionManager` for per-region hover via synthetic widget IDs
    (one per tab) but keep its monolithic layout/paint.
  - **Risk note**: Synthetic WidgetIds create lifecycle management burden. Each tab
    creates a synthetic WidgetId. When tabs are added/removed, InteractionManager
    must register/deregister these IDs. Tab reorder must update hot path positions.
    This is the highest-risk migration in the plan. The old hover mechanism
    (`Vec<AnimatedValue<f32>>` indexed by tab position) can coexist with the new
    framework temporarily if TabBarWidget migration blocks the rest of the plan.
  - Tab drag: DragController with tear-off threshold.
  - Context menu: handled at app layer (no controller needed).

- [ ] **MenuWidget**:
  - Remove: manual hover tracking (`hovered: Option<usize>` field updated in
    `handle_mouse(MouseMove)` and cleared in `handle_hover(Leave)`).
  - Remove: manual scroll handling (`handle_mouse(Scroll)` with `scroll_by()`) and
    `handle_key()` (ArrowUp/Down navigation, Enter/Space selection, Escape dismiss).
  - Add: HoverController (for enter/leave), ClickController (for item selection),
    ScrollController (for scroll events), FocusController (for keyboard).
  - **Keyboard handling**: Menu keyboard navigation (ArrowUp/Down + Enter/Escape)
    moves to `MenuKeyController` (defined in Section 08.1b). Cannot be handled by
    generic FocusController alone — it is widget-specific navigation logic.
  - `sense()`: `Sense::click().union(Sense::focusable())`

---

## 08.5 Migrate Passive Widgets

**File(s):** `oriterm_ui/src/widgets/label/mod.rs`, `separator/mod.rs`, `spacer/mod.rs`,
`rich_label/mod.rs`

- [ ] LabelWidget: `sense() -> Sense::none()`, rename `draw()` to `paint()`
- [ ] SeparatorWidget: same
- [ ] SpacerWidget: same
- [ ] RichLabel: already has `sense() -> Sense::none()`, rename `draw()` to `paint()`,
  remove stub `handle_mouse`, `handle_hover`, `handle_key` that return `WidgetResponse::ignored()`
**Note:** StatusBadge does not implement `Widget` (it is a standalone drawing helper)
and does not need migration. WindowChrome widgets are migrated in Wave 2b (Section 08.4).

---

## 08.6 Remove Legacy Event Methods

**File(s):** `oriterm_ui/src/widgets/mod.rs`

- [ ] Remove `handle_mouse()` from Widget trait
- [ ] Remove `handle_hover()` from Widget trait
- [ ] Remove `handle_key()` from Widget trait
- [ ] Remove `HoverEvent` enum (replaced by `LifecycleEvent::HotChanged`)
- [ ] Remove `is_focused: bool` and `focused_widget: Option<WidgetId>` from `EventCtx`.
  These are superseded by `InteractionManager` lookups via `ctx.is_interaction_focused()`.
  **Blast radius**: ~67 `EventCtx` struct literal constructions across ~20 test files,
  plus ~52 production sites (listed in Section 01.4). All `EventCtx::for_child()` calls
  that set `is_focused` from `focused_widget` comparison must be updated. The
  `for_child()` method (line 365 of `widgets/mod.rs`) currently computes
  `is_focused: child_id.is_some_and(|id| self.focused_widget == Some(id))` — this logic
  is removed since `InteractionManager` tracks focus per-widget.
- [ ] Remove `focused_widget: Option<WidgetId>` from `DrawCtx`. Superseded by
  `InteractionManager` lookup via `ctx.is_interaction_focused()`. Currently propagated
  through `DrawCtx::for_child()`. Blast radius: ~18 production sites + ~55 test sites.
- [ ] Remove `ContainerInputState` from `container/mod.rs` (replaced by framework propagation)
- [ ] Remove `WidgetResponse` and `CaptureRequest` from `widgets/mod.rs`
  (actions now emitted via controllers, capture via `ControllerCtx`)
- [ ] Remove or update `From<EventResponse> for DirtyKind` in `invalidation/mod.rs`
  (2 production usages: `container/mod.rs`, `widgets/mod.rs`; 5 test usages in
  `invalidation/tests.rs`). The invalidation system needs a new conversion from
  `ControllerRequests` bitflags to `DirtyKind` to replace the old `EventResponse`
  path. This is a **required** co-change when removing `EventResponse`.
- [ ] Replace `WidgetResponse::mark_tracker()` usage in the invalidation pipeline.
  `WidgetResponse::mark_tracker(&self, tracker)` extracts `source` and `DirtyKind`
  from the response and calls `tracker.mark()`. After removing `WidgetResponse`, the
  framework must derive invalidation from `DispatchOutput`: `PAINT` request ->
  `DirtyKind::Paint`, any structural change -> `DirtyKind::Layout`. Add
  `impl From<ControllerRequests> for DirtyKind` to handle this mapping.
- [ ] Remove `EventResponse` internal callers within `oriterm_ui` crate. Known sites:
  - `input/event.rs` — `EventResponse` enum definition and `is_handled()` method.
  - `invalidation/mod.rs` — `From<EventResponse> for DirtyKind` (covered above).
  - `widgets/mod.rs` — `WidgetResponse.response: EventResponse` field.
  - `container/event_dispatch.rs` — uses `EventResponse::Handled` / `Ignored` for
    child dispatch decisions (removed with the file).
  - `overlay/manager/event_routing.rs` — constructs `EventCtx`, reads `WidgetResponse`.
  Note: `MouseEvent`, `KeyEvent`, `Key`, `Modifiers`, `ScrollDelta`, `MouseButton`,
  `MouseEventKind` types in `input/event.rs` are NOT removed — they are still used by
  `InputEvent` (Section 03) and controller implementations. Only `EventResponse` and
  `HoverEvent` are removed.
- [ ] Update all callers in `oriterm` crate:
  - `dialog_context/content_actions.rs` — calls `handle_key` (1 site), `handle_hover` (2 sites)
  - `dialog_rendering.rs` — renders dialog content via `compose_scene()` (calls `.draw()`
    internally). Also constructs `DrawCtx` with `animations_running` field (7 sites).
  - `dialog_context/event_handling/mod.rs` — calls `handle_hover`, `handle_mouse`, imports
    `EventResponse, HoverEvent, MouseEvent, MouseEventKind`
  - `dialog_context/event_handling/mouse.rs` — calls `handle_mouse` (3 sites)
  - `app/settings_overlay/` — builds and manages settings panel
  - `app/settings_overlay/action_handler/mod.rs` — routes widget actions to config
  - `app/mouse_input.rs` — calls `process_mouse_event` on overlays (1 site)
- [ ] Update all callers that use `EventResponse` / `HoverEvent` / `MouseEvent` types
  from `oriterm_ui::input`. Search for `use oriterm_ui::input::` across the `oriterm`
  crate to find all import sites. Known callers (8 files with `use` imports):
  - `widgets/terminal_preview/mod.rs` — imports `HoverEvent, KeyEvent, MouseEvent`
  - `widgets/terminal_grid/mod.rs` — imports `HoverEvent, KeyEvent, MouseEvent`
  - `widgets/terminal_grid/tests.rs` — imports `HoverEvent, KeyEvent, Modifiers`
  - `app/tab_bar_input.rs` — imports `MouseButton, MouseEvent, MouseEventKind`
  - `app/keyboard_input/mod.rs` — imports `Key`
  - `app/dialog_context/content_actions.rs` — imports `EventResponse, HoverEvent, Key, KeyEvent, Modifiers`
  - `app/dialog_context/event_handling/mouse.rs` — imports `MouseButton, MouseEvent, MouseEventKind, ScrollDelta`
  - `app/dialog_context/event_handling/mod.rs` — imports `EventResponse, HoverEvent, MouseEvent, MouseEventKind`
  Additionally, `app/chrome/mod.rs` uses `oriterm_ui::input::EventResponse` via
  fully-qualified paths (2 sites) without a `use` import.
- [ ] Remove `container/event_dispatch.rs` file entirely (all 213 lines)
- [ ] Update `oriterm_ui/src/draw/scene_compose/mod.rs` line 29: change `root.draw(ctx)` to
  `root.paint(ctx)`. Also update the doc comment on line 17 which references `root.draw(ctx)`.
  Update `draw/scene_compose/tests.rs` which references `child.draw()` in comments.
- [ ] Update `overlay/manager/event_routing.rs` (348 lines): calls `handle_mouse()` (2 sites),
  `handle_hover()` (2 sites), and `handle_key()` (1 site) directly on overlay widgets.
  Must be updated to use the new event pipeline/controller dispatch. This is a **required**
  co-change when removing the old trait methods — overlay widgets will not receive events
  otherwise.

---

## 08.7 Completion Checklist

### Prerequisites (08.0)
- [x] `contexts.rs` extracted from `widgets/mod.rs` with re-exports (zero blast radius)
- [x] `widgets/mod.rs` under 300 lines after extraction (268 lines)
- [x] `scroll/mod.rs` split into `scroll/rendering.rs` + `scroll/event_handling.rs`
- [x] `dialog/mod.rs` split into `dialog/event_handling.rs`
- [x] `settings_panel/mod.rs` split (event_handling.rs extracted)
- [x] `tab_bar/widget/mod.rs` split (event_handling.rs with stub delegation)
- [x] All splits pass `./build-all.sh`, `./clippy-all.sh`, `./test-all.sh`

### New Widget Trait (08.1)
- [x] New Widget trait shape with `sense()`, `controllers()`, `visual_states()`,
  `paint()`, `lifecycle()`, `anim_frame()`

### Widget Migration (08.3, 08.4, 08.5, Wave 4)
- [ ] All 6 interactive widgets migrated to controllers + visual state animators
  (Button, Toggle, Checkbox, Dropdown, Slider, TextInput)
- [ ] All 10 layout/container widgets migrated (no manual event routing)
  (Container, Panel, Scroll, Stack, FormLayout, FormSection, FormRow,
   SettingsPanel, Dialog, Menu)
- [ ] All 3 chrome widgets migrated (WindowChrome, WindowControlButton, IdOverrideButton)
- [ ] TabBarWidget migrated (highest-risk widget -- synthetic WidgetIds for per-tab hover)
- [ ] All 4 passive widgets migrated (Label, Separator, Spacer, RichLabel; Sense::none, paint rename)
- [ ] All 2 cross-crate widgets migrated (TerminalGridWidget, TerminalPreviewWidget
  in `oriterm/src/widgets/`)
- [ ] All 3 test-only Widget implementations migrated (CountingWidget, TrackingWidget,
  CacheDetector)

### New Context Types (08.1)
- [x] `LifecycleCtx` and `AnimCtx` defined in `contexts.rs` with correct field types
  (`LifecycleCtx.interaction: &InteractionState`, `AnimCtx.frame_requests: Option<&FrameRequestFlags>`)

### Legacy Removal (08.6)
- [ ] Legacy `handle_mouse()`, `handle_hover()`, `handle_key()` removed from trait
- [ ] `HoverEvent`, `ContainerInputState` removed
- [ ] `draw()` to `paint()` rename complete: all 26 widget impls, all container
  `child.draw()` call sites, `compose_scene()` in `draw/scene_compose/mod.rs`,
  `dialog/rendering.rs` button draw calls, and all test files updated
- [ ] `DrawCtx::animations_running: &Cell<bool>` field removed (~88 usages across
  ~31 files). Production widgets migrate to `ctx.request_anim_frame()`. Container
  propagation removed (framework owns scheduling). ~55 test `DrawCtx` constructions
  lose the field.
- [ ] `EventCtx.is_focused` and `EventCtx.focused_widget` fields removed (~67 test
  sites + ~52 production sites). Strategy: remove only after all widgets use
  `ctx.is_interaction_focused()` instead.
- [ ] `DrawCtx.focused_widget` field removed (~18 production + ~55 test sites).
- [ ] Test-only Widget implementations migrated (3 total: CountingWidget,
  TrackingWidget, CacheDetector -- add `sense() -> Sense::none()`, rename
  `draw()` to `paint()`, remove stub event handlers)

### Custom Controllers (08.1b)
- [ ] `TextEditController` in `controllers/text_edit/` with `tests.rs`
- [ ] `DropdownKeyController` in `controllers/dropdown_key/` with `tests.rs`
- [ ] `MenuKeyController` in `controllers/menu_key/` with `tests.rs`
- [ ] `TerminalInputController` in `oriterm/src/widgets/terminal_grid/input_controller.rs`
- [ ] All custom controllers registered in `controllers/mod.rs` (except TerminalInputController)

### Framework & Integration (08.1a)
- [ ] Framework orchestration pipeline implemented: lifecycle delivery,
  anim_frame dispatch, visual_states update/tick, then paint -- all BEFORE `draw_frame()`
- [ ] `DispatchResult` defined at app layer and delivery loop implemented
- [ ] `OverlayManager::process_mouse_event()` migrated to use `plan_propagation()` +
  `dispatch_to_controllers()` (same pipeline as main widget tree)
- [ ] `sense()` default changed from `Sense::all()` to `Sense::none()` ONLY after all
  26 widgets provide explicit overrides (verified by grep)
- [ ] `From<ControllerRequests> for DirtyKind` conversion added to replace
  `From<EventResponse> for DirtyKind`

### Verification
- [ ] Settings dialog works with new trait (all controls functional)
- [ ] Tab bar works with new trait
- [ ] Overlay/popup system works with new propagation
- [ ] No regressions in any existing UI functionality
- [ ] `./test-all.sh` green, `./clippy-all.sh` green, `./build-all.sh` green

**Exit Criteria:** The settings dialog opens, all dropdowns/toggles/checkboxes work with
hover animations, keyboard navigation (tab) works, and the dialog closes with Save/Cancel.
All via the new Widget trait, controllers, and visual state animators. Zero legacy event
methods remain in the codebase.
