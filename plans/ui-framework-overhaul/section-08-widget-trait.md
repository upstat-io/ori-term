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
    status: complete
  - id: "08.1b"
    title: "Custom Controllers (TextEdit, TerminalInput, MenuKey, DropdownKey)"
    status: complete
  - id: "08.2"
    title: "Migration Strategy"
    status: complete
  - id: "08.3"
    title: "Migrate Interactive Widgets"
    status: complete
  - id: "08.4"
    title: "Migrate Layout Widgets"
    status: complete
  - id: "08.5"
    title: "Migrate Passive Widgets"
    status: complete
  - id: "08.6"
    title: "Remove Legacy Event Methods"
    status: in-progress
  - id: "08.7"
    title: "Completion Checklist"
    status: in-progress
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

**File(s):** `oriterm/src/app/widget_pipeline/mod.rs` (new),
`oriterm/src/app/window_context.rs`, `oriterm/src/app/redraw/draw_helpers.rs`,
`oriterm/src/app/redraw/mod.rs`, `oriterm/src/app/redraw/multi_pane.rs`.

This subsection defines the per-frame pipeline that the application layer executes for
each widget. Without this, the `lifecycle()`, `anim_frame()`, `visual_states_mut()`,
and `paint()` methods are defined but never called in the correct order.

**RENDERING DISCIPLINE**: Steps 1-4a below are **state mutation** that happens BEFORE
`draw_frame()`. The GPU renderer's `draw_frame()` must remain pure computation (reads
state, builds instance buffers, no side effects). The pipeline below runs in the event
handler (e.g., `RedrawRequested` or a dedicated pre-render phase), NOT inside the GPU
render pass. `paint()` (step 4b) populates `DrawList` commands which `draw_frame()`
later consumes to build GPU instance buffers.

- [x] **Per-frame widget pipeline** (executed by the app layer BEFORE `draw_frame()`):
  Implemented in `oriterm/src/app/widget_pipeline/mod.rs` as `prepare_widget_frame()`.
  Wired into both single-pane (`redraw/mod.rs`) and multi-pane (`redraw/multi_pane.rs`)
  render paths. Drains lifecycle events from `InteractionManager`, delivers to
  controllers + `widget.lifecycle()`, runs `anim_frame()`, updates `VisualStateAnimator`.
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

- [x] **Widget tree traversal for the pipeline**: During the transition period,
  containers still call `child.paint()` directly. The framework calls
  `prepare_widget_frame()` on top-level widgets (tab bar) before `compose_scene()`.
  After migration, the framework will walk the full widget tree.

- [x] **`DispatchResult` (deferred from Section 03.2)**: Defined in
  `oriterm/src/app/widget_pipeline/mod.rs`. Includes `dispatch_step()` for the
  delivery loop and `apply_requests()` for side-effect application. Tested with
  7 unit tests covering merge, stop-on-handled, request accumulation, and
  lifecycle delivery.

  ```rust
  pub struct DispatchResult {
      pub handled: bool,
      pub actions: Vec<WidgetAction>,
      pub requests: ControllerRequests,
      pub source: Option<WidgetId>,
  }
  ```

- [x] **OverlayManager integration** (**HIGH RISK -- 348-line rewrite**):
  Overlay widgets now participate in the same pipeline. `try_controllers()` replaced
  with `deliver_via_pipeline()` which hit-tests the overlay's `LayoutNode` tree,
  runs `plan_propagation()` for Capture → Target → Bubble delivery, and dispatches
  to controllers at each matching phase. Legacy `handle_mouse()`/`handle_key()`
  fallback preserved for un-migrated widgets. `Overlay` struct stores
  `layout_node: Option<LayoutNode>` populated during `layout_overlays()`.
  Modal semantics, click-outside dismiss, capture routing, and hover tracking
  are all preserved. No caller changes — the three binary call sites are unaffected.
  `bridge_dispatch_to_response()` retained for test compatibility.

---

## 08.1b Custom Controllers (TextEdit, TerminalInput, MenuKey, DropdownKey)

**File(s):** New files in `oriterm_ui/src/controllers/` and `oriterm/src/widgets/`

Several widgets require custom controllers for widget-specific keyboard logic that
cannot be handled by the generic FocusController. These must be implemented BEFORE
the migration waves that depend on them.

**IMPORTANT**: These are all directory modules per test-organization.md (each `mod.rs`
gets a sibling `tests.rs`).

- [x] **`TextEditController`** — `oriterm_ui/src/controllers/text_edit/mod.rs` (~120 lines)
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

- [x] **`DropdownKeyController`** — `oriterm_ui/src/controllers/dropdown_key/mod.rs` (~60 lines)
  + `controllers/dropdown_key/tests.rs`
  - Handles: Up/Down arrow (change selection), Enter (confirm), Escape (close dropdown).
  - Phase: `EventPhase::Bubble`.
  - Requires mutable access to dropdown state (selected index, open/closed).
  - **NOTE**: Keyboard input must go through controllers, not `Widget::lifecycle()`.
    Lifecycle events are for state changes (HotChanged, FocusChanged, WidgetAdded/Removed),
    not for input routing. Keyboard input must flow through the controller pipeline.
  - Emits `WidgetAction::Selected(WidgetId, usize)` on Enter confirm.
  - **Dependency**: Used by `DropdownWidget` (Wave 1, 08.3).

- [x] **`MenuKeyController`** — `oriterm_ui/src/controllers/menu_key/mod.rs` (~60 lines)
  + `controllers/menu_key/tests.rs`
  - Handles: ArrowUp/Down (navigate items), Enter/Space (select), Escape (dismiss).
  - Phase: `EventPhase::Bubble`.
  - Emits `WidgetAction::Clicked(WidgetId)` on selection,
    `WidgetAction::DismissOverlay` on Escape.
  - **Dependency**: Used by `MenuWidget` (Wave 2, 08.4).

- [x] **`TerminalInputController`** — `oriterm/src/widgets/terminal_grid/input_controller.rs`
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

- [x] `./build-all.sh` green, `./clippy-all.sh` green, `./test-all.sh` green

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

- [x] **Wave 1 -- Interactive widgets** (have event handlers, hover state):
  ButtonWidget, ToggleWidget, CheckboxWidget, DropdownWidget, SliderWidget, TextInputWidget
  - Added HoverController + ClickController to all 6 widgets
  - Added VisualStateAnimator (common_states or focus_states) to all 6
  - Replaced draw() with paint() using animator colors
  - Removed manual `hovered: bool` tracking from all 6
  - Legacy handle_mouse/handle_hover/handle_key retained as compat shims
    until containers migrate in §08.4. DragController/FocusController wiring
    deferred to §08.6 when legacy methods are removed.

- [x] **Wave 2 -- Layout/container widgets** (route events to children):
  ContainerWidget, PanelWidget, ScrollWidget, StackWidget, FormLayout,
  FormSection, FormRow, SettingsPanel, DialogWidget, MenuWidget
  - Renamed draw() → paint() for all 10 widgets
  - Added sense() override (Sense::none() for containers, Sense::click()+focusable for Menu)
  - Legacy handle_* methods retained as compat shims during transition

- [x] **Wave 2b -- Chrome/interactive-container widgets:**
  WindowChromeWidget (sense+paint), WindowControlButton (full migration:
  controllers+animator like Wave 1), IdOverrideButton (sense+paint)

- [x] **Wave 2c -- TabBarWidget** (simple rename for transition period;
  full synthetic WidgetId migration deferred to §08.6)
  - Renamed draw() → paint(), added sense() → Sense::click()

- [x] **Wave 3 -- Passive widgets** (no event handling):
  LabelWidget, SeparatorWidget, SpacerWidget, RichLabel
  - Added sense() → Sense::none(), renamed draw() → paint()
  - Removed stub handle_mouse/handle_hover/handle_key and unused imports
  - Removed is_focusable() overrides (now derived from sense())

- [x] **Wave 4 -- Cross-crate widgets** (in `oriterm` crate, not `oriterm_ui`):
  TerminalGridWidget, TerminalPreviewWidget — renamed draw→paint, added sense(),
  removed is_focusable() overrides. Legacy handle_* methods retained.

---

## 08.3 Migrate Interactive Widgets

**File(s):** `oriterm_ui/src/widgets/button/mod.rs`, `toggle/mod.rs`, `checkbox/mod.rs`,
`dropdown/mod.rs`, `slider/mod.rs`, `text_input/mod.rs` + `text_input/widget_impl.rs`

For each interactive widget:

- [x] **ButtonWidget**:
  - Remove: `hovered: bool`, `hover_progress: AnimatedValue<f32>`,
    manual `HoverEvent` handling in `handle_hover()`, `current_bg()` helper method
    (replaced by `animator.get_bg_color(now)`)
  - Add: `controllers: Vec<Box<dyn EventController>>` with HoverController + ClickController
  - Add: `animator: VisualStateAnimator` with `common_states()` (Normal/Hovered/Pressed/Disabled)
  - `paint()`: use `animator.get_bg_color(now)` instead of manual interpolation.
    Remove `ctx.animations_running.set(true)` — replaced by
    `if self.animator.is_animating(now) { ctx.request_anim_frame(); }`.
  - `sense()`: `Sense::click()`
  - Note: `pressed: bool` and legacy `handle_mouse()`/`handle_hover()` retained as
    compat shims until containers migrate in §08.4. Tests rewritten.

- [x] **ToggleWidget**:
  - Remove: manual hover tracking (`hovered: bool` field)
  - Add: HoverController + ClickController
  - Add: `common_states()` animator for off-state hover bg transitions
  - Keep: `toggle_progress: AnimatedValue<f32>` for thumb slide (AnimProperty
    migration deferred — separate animation system concern)
  - `sense()`: `Sense::click()`
  - Note: `pressed: bool` and legacy methods retained as compat shims. Tests rewritten.

- [x] **CheckboxWidget**:
  - Remove: manual hover tracking (`hovered: bool` field)
  - Add: HoverController + ClickController
  - Add: `animator: VisualStateAnimator` with `common_states()` for unchecked hover transitions
  - `sense()`: `Sense::click()`
  - Note: `pressed: bool` and legacy methods retained as compat shims. Tests rewritten.

- [x] **DropdownWidget**:
  - Remove: `hovered: bool`, `current_bg()` helper
  - Add: HoverController + ClickController
  - Add: `animator: VisualStateAnimator` with `common_states()` for bg transitions
  - `paint()` uses animator bg, hybrid focus detection
  - `sense()`: `Sense::click()`
  - Note: `pressed: bool` and legacy methods retained as compat shims.
    DropdownKeyController wiring and FocusController deferred to §08.6. Tests rewritten.

- [x] **SliderWidget**:
  - Remove: `hovered: bool`, `is_hovered()`
  - Keep: `dragging: bool` for legacy drag tracking
  - Add: HoverController + ClickController
  - Add: `animator: VisualStateAnimator` with `common_states()` for thumb hover
  - `paint()` uses animator for thumb bg when not dragging
  - `sense()`: `Sense::click()`
  - Note: DragController + FocusController wiring deferred to §08.6. Tests rewritten.

- [x] **TextInputWidget**:
  - Remove: `hovered: bool`, `is_hovered()`
  - Add: HoverController + ClickController
  - Add: `animator: VisualStateAnimator` with `focus_states()` for border color
  - `paint()` uses animator border color, hybrid focus detection
  - `sense()`: `Sense::click_and_drag().union(Sense::focusable())`
  - Note: TextEditController + DragController wiring deferred to §08.6.
    Legacy handle_mouse/handle_key retained. Tests rewritten.

---

## 08.4 Migrate Layout Widgets

**File(s):** `oriterm_ui/src/widgets/container/mod.rs`, `panel/mod.rs`, `scroll/mod.rs`,
`stack/mod.rs`, `form_layout/mod.rs`, `form_section/mod.rs`, `form_row/mod.rs`,
`settings_panel/mod.rs`, `dialog/mod.rs`, `menu/widget_impl.rs`,
`tab_bar/widget/draw.rs` (Widget impl), `tab_bar/widget/mod.rs` (struct + state),
`window_chrome/mod.rs`, `window_chrome/controls.rs`

- [x] **ContainerWidget** (container/mod.rs: 462 lines — safe after event_dispatch.rs removal):
  - Remove: `ContainerInputState` (in `container/mod.rs`), entire
    `container/event_dispatch.rs` submodule (213 lines — manual per-container child dispatch)
  - Framework handles child event dispatch via propagation pipeline
  - Keep: layout logic unchanged (including `LayoutMode` enum from Section 07 — grid
    mode is a layout concern, not an event concern)
  - Keep: `layout_build.rs` submodule (extracted in Section 07)
  - Add: optional hover tracking via `.with_hover(true)` for setting rows
  - `sense()`: `Sense::none()` by default, `Sense::hover()` when hover enabled

- [x] **ScrollWidget** (PREREQUISITE: 08.0b split of `scroll/mod.rs` must be complete):
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

- [x] **StackWidget**:
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

- [x] **PanelWidget**: Rename `draw()` to `paint()`. Add `sense() -> Sense::none()`.
  No event handling changes needed (already uses `ctx.for_child()` for children).
- [x] **DialogWidget** (PREREQUISITE: 08.0b split of `dialog/mod.rs` must be complete):
  - Remove: manual event routing in `handle_mouse()`, `handle_hover()`, `handle_key()`
    (routes to header buttons and content widget — `dialog/mod.rs` lines 341-480).
  - Header drag (MoveOverlay action) -> DragController on the dialog header region.
  - Close/OK button clicks -> framework propagation to ButtonWidget children.
  - `sense()`: `Sense::none()` (container, but children handle interactions)
  - Keep: `accept_action()` override for dropdown selection routing.
  - Keep: `focusable_children()` override.

- [x] **FormSection**:
  - Remove: manual event routing in `handle_mouse()`, `handle_hover()`, `handle_key()`
    (routes events to child FormRow widgets).
  - Framework handles child dispatch via propagation pipeline.
  - `sense()`: `Sense::none()`

- [x] **FormRow**:
  - Remove: manual event routing in `handle_mouse()`, `handle_hover()`, `handle_key()`
    (routes events to its `control: Box<dyn Widget>` child).
  - Framework handles dispatch to the control child.
  - `sense()`: `Sense::none()`
  - Keep: `accept_action()` override for propagating actions to control child.

- [x] **FormLayout**:
  - Remove: manual event routing in `handle_mouse()`, `handle_hover()`, `handle_key()`
    (routes events to child FormSection widgets).
  - `sense()`: `Sense::none()`

- [x] **SettingsPanel** (PREREQUISITE: 08.0b split if migration adds lines):
  Rename `draw()` to `paint()`. Add `sense() -> Sense::none()` (delegates all
  interactions to its container child). Remove any stub `handle_mouse`/`handle_hover`/
  `handle_key` methods if present.
- [x] **WindowChromeWidget** (Wave 2b):
  - Add ClickController for buttons, DragController for titlebar.
  - Remove direct `DrawCtx` struct literal construction (line 263-276 of
    `window_chrome/mod.rs`) -- migrate to `ctx.for_child()`. Remove direct `EventCtx`
    struct literal construction (5 sites in `window_chrome/mod.rs`).
  - `sense()`: `Sense::none()` (children handle clicks).

- [x] **WindowControlButton** (Wave 2b):
  - Extract hover/press -> HoverController + ClickController.
  - Remove `hover_progress: AnimatedValue<f32>`, `hovered: bool`, `pressed: bool`,
    `current_bg()` helper (line 114 of `window_chrome/controls.rs`). Add
    `animator: VisualStateAnimator` with `common_states()`. Remove
    `ctx.animations_running.set(true)` (1 site in `controls.rs`).
  - `sense()`: `Sense::click()`.

- [x] **IdOverrideButton** (Wave 2b):
  - Extract click -> ClickController. `sense()`: `Sense::click()`.

- [x] **TabBarWidget** (Wave 2c -- most complex widget;
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

- [x] **MenuWidget**:
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

- [x] LabelWidget: `sense() -> Sense::none()`, rename `draw()` to `paint()`
- [x] SeparatorWidget: same
- [x] SpacerWidget: same
- [x] RichLabel: already has `sense() -> Sense::none()`, rename `draw()` to `paint()`,
  remove stub `handle_mouse`, `handle_hover`, `handle_key` that return `WidgetResponse::ignored()`
**Note:** StatusBadge does not implement `Widget` (it is a standalone drawing helper)
and does not need migration. WindowChrome widgets are migrated in Wave 2b (Section 08.4).

---

## 08.6 Remove Legacy Event Methods

**MANDATE: Every single UI control — buttons, toggles, sliders, dropdowns, text inputs,
window chrome buttons, tab bar tabs, close buttons, menu items, scroll thumbs, dialog
headers — goes through the unified controller + animator + propagation pipeline. Zero
special cases, zero manual `hovered: bool` fields, zero one-off `handle_mouse()`
implementations. One system, one path, no exceptions.**

**File(s):** `oriterm_ui/src/widgets/mod.rs` and across the full codebase.

### Framework Pipeline — Full Tree Traversal

The framework pipeline (`prepare_widget_frame()`) currently only runs on top-level
widgets (tab bar, overlay roots). For §08.6 to work, it MUST walk the full widget tree
so every child widget gets:
- Lifecycle events delivered to its controllers and `lifecycle()`
- `animator.update(&interaction_state, now)` called before `paint()`
- `anim_frame()` calls when requested

Without this, child widgets' animators are never updated and hover/pressed visual
states don't work (this is the current regression on window control buttons).

- [x] Extend `prepare_widget_frame()` (or add a new tree-walk function) to visit all
  widgets in the tree, not just top-level. Containers must expose their children for
  traversal — add `fn children(&self) -> &[Box<dyn Widget>]` or similar to the trait.
- [x] Remove `DrawCtx::animations_running: &Cell<bool>` field (~88 usages across
  ~31 files). The framework pipeline now owns animation scheduling via
  `FrameRequestFlags`. Widgets use `ctx.request_anim_frame()` exclusively.
  ~55 test `DrawCtx` constructions lose the field.

### Pipeline Prerequisites — Make Controller Dispatch Complete

The pipeline (`deliver_event_to_tree`) must handle ALL input for ALL widgets
before legacy methods can be removed. These items close gaps between the
generic controller pipeline and widget-specific behavior.

**Widget trait extensions:**

- [x] `Widget::on_action(&mut self, action, bounds) -> Option<WidgetAction>` — transforms
  generic controller actions (e.g., `Clicked`) into widget-specific semantic actions
  (e.g., `OpenDropdown`, `Toggled`). Called by `dispatch_to_widget_tree` after controller
  dispatch. Default: passthrough. Implemented on DropdownWidget, ToggleWidget,
  CheckboxWidget, MenuWidget.
- [x] `Widget::on_input(&mut self, event, bounds) -> bool` — fallback for input events not
  consumed by controllers. Called by `dispatch_to_widget_tree` when no controller
  handles the event. Used for widget-internal interaction (e.g., MenuWidget item hover
  tracking on MouseMove, scroll handling). Default: false.

**Layout tree completeness (hit testing must reach ALL widgets):**

- [x] `ScrollWidget::layout()` returns `LayoutBox::flex()` wrapping child layout with
  `clip=true`, not `LayoutBox::leaf()`. Without this, hit testing through
  `layout_hit_test_path` never finds widgets inside scroll containers.
- [x] Verify `SettingsPanel` layout tree includes Save/Cancel footer buttons in the
  hit test path. SettingsPanel returns `LayoutBox::flex(Column, [child_box])`;
  DialogWidget returns `flex(Column, [content_zone, footer_zone])` with buttons.
- [x] Verify all container widgets' `layout()` methods include children's layout boxes
  (not just leaf size reporting). Verified: FormLayout, FormSection, FormRow,
  DialogWidget, PanelWidget, ContainerWidget — all return `flex()` with children.
  **Exception:** StackWidget returns `leaf()` (no children in layout tree) — z-stack
  layout support not yet implemented. StackWidget is NOT used in production code.

**Coordinate space reconciliation:**

- [x] `deliver_event_to_tree` converts cursor positions to local space (subtracts
  `bounds.origin`) before hit testing, then offsets hit entry bounds back to screen
  space. Callers compute layout in LOCAL space (`Rect::new(0, 0, w, h)`).
- [x] `on_action` receives screen-space bounds (offset by `deliver_event_to_tree`).
  DropdownWidget uses these as the popup anchor rect.
- [x] Captured mouse events (MouseMove/MouseUp during press) use bounds from the hit
  path (`plan_captured_mouse` reads first entry bounds). `deliver_event_to_tree`
  builds a single-entry path with root bounds for the active widget case.

**Missing controllers on widgets:**

- [x] MenuWidget: add `ClickController` (was marked complete in §08.4 but never wired).
- [x] ButtonWidget: add `KeyActivationController` for Enter/Space → `Clicked`
  (needed to remove `handle_key` from Button).
- [x] ToggleWidget, CheckboxWidget: add `KeyActivationController` for Space → toggle
  (needed to remove `handle_key`). Toggle/Checkbox `on_action` already transforms
  `Clicked` → `toggle()`.
- [x] SliderWidget: add `ScrubController` for immediate drag-to-value and
  `SliderKeyController` for arrow/Home/End (needed to remove `handle_mouse` and
  `handle_key`). Widget impl extracted to `widget_impl.rs` (500-line limit).
- [x] TextInputWidget: keyboard input and click-to-cursor migrated to `on_input()`.
  Click-to-cursor uses cached character offsets from `layout()`. `TextEditController`
  wiring deferred (state ownership issue), but legacy methods removed via `on_input` fallback.

**Dialog context integration:**

- [x] Add `InteractionManager` and `FocusManager` to `DialogWindowContext`.
- [x] Add `prepare_widget_tree` to dialog rendering (delivers lifecycle events,
  updates visual state animators from InteractionManager state).
- [x] Chrome click dispatch: `WindowChromeWidget::dispatch_input()` with controller
  pipeline, `action_for_widget()` maps `Clicked(id)` → window actions.
- [x] Chrome hover: `InteractionManager::update_hot_path()` from cursor move,
  lifecycle events delivered via `prepare_widget_tree`.
- [x] Content click dispatch: `deliver_event_to_tree` with on-demand layout computation.
  Coordinate space: hit test in local space, bounds offset to screen space.
- [x] Content scroll dispatch: same pipeline as click, with `ScrollController` on
  `ScrollWidget` handling `InputEvent::Scroll`.
- [x] Content keyboard: `deliver_event_to_tree` with focus_path from
  `InteractionManager::focus_ancestor_path()`. FocusController on ButtonWidget
  handles Tab cycling, KeyActivationController handles Enter/Space.
- [x] Cursor leave: `InteractionManager::update_hot_path(&[])` clears hover
  via `clear_dialog_hover()`.
- [x] Overlay dispatch: already on controller pipeline via `deliver_via_pipeline`.

### Remove Legacy Compat Shims from Wave 1 Widgets

- [x] Remove `pressed: bool` from ButtonWidget, ToggleWidget, CheckboxWidget,
  DropdownWidget, SliderWidget, WindowControlButton (6 widgets). The framework's
  `InteractionManager::active_widget` replaces manual pressed tracking.
  Note: Button/Toggle/Checkbox/Dropdown already removed in §08.3. WindowControlButton
  removed in §08.6; pressed routing moved to parent (`pressed_control: Option<usize>`
  on WindowChromeWidget and TabBarWidget).
- [x] Remove `dragging: bool` from SliderWidget. Animator already returns pressed-state
  color. Move events always update value (container capture semantics ensure Move only
  arrives during drag).
- [x] Remove legacy `handle_mouse()` overrides from all 7 interactive widgets
  (Button, Toggle, Checkbox, Dropdown, Slider, TextInput, WindowControlButton).
  All 7 removed. TextInput mouse/key logic moved to `on_input()` with cached char offsets.
- [x] Remove legacy `handle_hover()` overrides from all interactive widgets that have them.
  Removed from Button, Toggle, Checkbox, Dropdown, Slider, WindowControlButton, Menu.
  Container widgets (Scroll, Panel, FormLayout, FormSection, FormRow, Dialog, etc.) still have
  delegation chains — these become no-ops since children use trait defaults.
- [x] Remove legacy `handle_key()` from ButtonWidget (keyboard activation moves to
  a controller — either extend ClickController for Enter/Space, or add
  `KeyActivationController`)
- [x] Remove legacy `handle_key()` from ToggleWidget, CheckboxWidget (Space toggle →
  controller), DropdownWidget (arrow nav → DropdownKeyController), SliderWidget
  (arrow/Home/End → controller). TextInputWidget deferred (all keys → TextEditController).
  Also removed dead helper methods: `select_next/select_prev` (Dropdown), `navigate` (Menu),
  `action` (WindowControlButton).
- [x] Verify: window control buttons have working hover via the pipeline (no regression)
  Dialog cursor move → `widget_at_point()` → `update_hot_path()` → `HotChanged` lifecycle
  → `prepare_widget_tree()` walks children via `for_each_child_mut()` → `animator.update()`
  → `paint()` reads `animator.get_bg_color()`. Verified end-to-end; legacy methods removed.

### Remove Legacy Methods from Widget Trait

- [x] Remove `handle_mouse()` from Widget trait
- [x] Remove `handle_hover()` from Widget trait
- [x] Remove `handle_key()` from Widget trait
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
- [x] Remove `ContainerInputState` from `container/mod.rs` (replaced by framework propagation).
  Also removed: `update_dirty()`, `event_dispatch.rs` (213 lines), `find_child_index()`,
  `hit_test_children()`. ContainerWidget legacy `handle_mouse/hover/key` impls removed.
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
- [x] Remove `container/event_dispatch.rs` file entirely (all 213 lines)
- [ ] Update `oriterm_ui/src/draw/scene_compose/mod.rs` line 29: change `root.draw(ctx)` to
  `root.paint(ctx)`. Also update the doc comment on line 17 which references `root.draw(ctx)`.
  Update `draw/scene_compose/tests.rs` which references `child.draw()` in comments.
- [x] Update `overlay/manager/event_routing.rs`: controller-first dispatch with legacy
  fallback at all 3 mouse sites + 1 key site. `bridge_dispatch_to_response()` converts
  `DispatchOutput` → `WidgetResponse`. `try_controllers()` helper provides the fast path.
  Hover (`handle_hover`) left as-is — lifecycle event, not input (migrates with §08.6+).

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
- [x] All 6 interactive widgets migrated to controllers + visual state animators
  (Button, Toggle, Checkbox, Dropdown, Slider, TextInput)
- [x] All 10 layout/container widgets migrated (draw→paint, sense added)
  (Container, Panel, Scroll, Stack, FormLayout, FormSection, FormRow,
   SettingsPanel, Dialog, Menu)
- [x] All 3 chrome widgets migrated (WindowChrome sense+paint, WindowControlButton
  full controllers+animator, IdOverrideButton sense+paint)
- [x] TabBarWidget migrated (draw→paint+sense; full synthetic WidgetId migration deferred)
- [x] All 4 passive widgets migrated (Label, Separator, Spacer, RichLabel; Sense::none, paint rename,
  stubs removed)
- [x] All 2 cross-crate widgets migrated (TerminalGridWidget, TerminalPreviewWidget;
  sense+paint, legacy handle_* retained)
- [x] All 3 test-only Widget implementations migrated (CountingWidget, TrackingWidget,
  CacheDetector; sense+paint, stubs removed)

### New Context Types (08.1)
- [x] `LifecycleCtx` and `AnimCtx` defined in `contexts.rs` with correct field types
  (`LifecycleCtx.interaction: &InteractionState`, `AnimCtx.frame_requests: Option<&FrameRequestFlags>`)

### Legacy Removal (08.6)
- [x] Framework pipeline walks full widget tree (not just top-level)
- [x] All `pressed: bool` / `dragging: bool` compat fields removed from Wave 1 widgets
- [x] All legacy `handle_mouse()`, `handle_hover()`, `handle_key()` overrides removed
  from every widget (not just from the trait — from every impl)
- [x] Legacy `handle_mouse()`, `handle_hover()`, `handle_key()` removed from trait
- [ ] `HoverEvent`, `ContainerInputState`, `WidgetResponse`, `CaptureRequest`,
  `EventResponse` removed
- [x] `DrawCtx::animations_running` field removed (framework owns scheduling)
- [ ] `EventCtx.is_focused`, `EventCtx.focused_widget`, `DrawCtx.focused_widget`
  fields removed (InteractionManager is the single source of truth)
- [x] `container/event_dispatch.rs` file deleted (framework propagation replaces it)
- [x] Window control button hover works via the unified pipeline (regression fixed)
- [ ] Every widget's visual state driven by InteractionManager + VisualStateAnimator
  — zero manual `hovered: bool` fields remain anywhere in the codebase

### Custom Controllers (08.1b)
- [x] `TextEditController` in `controllers/text_edit/` with `tests.rs`
- [x] `DropdownKeyController` in `controllers/dropdown_key/` with `tests.rs`
- [x] `MenuKeyController` in `controllers/menu_key/` with `tests.rs`
- [x] `TerminalInputController` in `oriterm/src/widgets/terminal_grid/input_controller.rs`
- [x] All custom controllers registered in `controllers/mod.rs` (except TerminalInputController)

### Framework & Integration (08.1a)
- [x] Framework orchestration pipeline implemented: lifecycle delivery,
  anim_frame dispatch, visual_states update/tick, then paint -- all BEFORE `draw_frame()`
  (`prepare_widget_tree` in `widget_pipeline/mod.rs`)
- [x] `DispatchResult` defined at app layer and delivery loop implemented
  (`widget_pipeline/mod.rs:31` + `dispatch_step` + `apply_dispatch_requests`)
- [x] `OverlayManager::process_mouse_event()` migrated to use `dispatch_to_controllers()`
  with full tree dispatch (legacy fallback removed from all 3 mouse + 1 key dispatch sites)
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

**Exit Criteria:** Every single UI control — settings dialog buttons, window chrome
buttons, tab bar, menu items, scroll thumbs, terminal grid — receives input and renders
visual state through the unified controller + propagation + animator pipeline. No widget
implements `handle_mouse()`, `handle_hover()`, or `handle_key()`. No widget has manual
`hovered: bool` or `pressed: bool` fields. `InteractionManager` is the single source of
truth for all interaction state. `VisualStateAnimator` drives all state-dependent
rendering. The old types (`WidgetResponse`, `EventResponse`, `HoverEvent`,
`ContainerInputState`) do not exist. Zero legacy event methods, zero special cases,
zero one-off implementations.
