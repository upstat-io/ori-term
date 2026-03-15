---
section: "08"
title: "Widget Trait Overhaul"
status: not-started
goal: "New Widget trait shape integrating Sense, controllers, visual states, and lifecycle; all existing widgets migrated"
inspired_by:
  - "Druid Widget trait (druid/src/widget.rs)"
  - "GTK4 Widget + EventController composition"
depends_on: ["01", "02", "03", "04", "05", "06", "07"]
reviewed: false
sections:
  - id: "08.1"
    title: "New Widget Trait"
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
`accept_action`, `focusable_children`) to the new shape (Sense declaration, controller
composition, visual state groups, lifecycle method). All 25 existing Widget implementations
are migrated (23 in `oriterm_ui` + 2 in `oriterm`). No regressions in behavior.

**Context:** This is the convergence point where all prior sections come together. The new
trait must support the framework-managed interaction state (Section 01), Sense filtering
(Section 02), two-phase event propagation (Section 03), composable controllers (Section 04),
animation frames (Section 05), visual state management (Section 06), and new layout/theme
capabilities (Section 07). Every existing widget must be migrated without breaking the
settings dialog, tab bar, or any other UI element.

**Depends on:** All prior sections (01-07).

---

## 08.1 New Widget Trait

**File(s):** `oriterm_ui/src/widgets/mod.rs`

**File size warning**: `widgets/mod.rs` is currently 361 lines. Adding `LifecycleCtx`,
`AnimCtx`, new trait methods, and convenience methods on `DrawCtx`/`EventCtx` will add
~80-100 lines (~450 total). If it reaches 480+ lines during implementation, extract
context types (`DrawCtx`, `EventCtx`, `LayoutCtx`, `LifecycleCtx`, `AnimCtx`) into a
`widgets/contexts.rs` submodule.

- [ ] Define the new trait shape:
  ```rust
  pub trait Widget {
      /// Unique identifier for this widget instance.
      fn id(&self) -> WidgetId;

      /// What interactions this widget cares about.
      fn sense(&self) -> Sense { Sense::none() }

      /// Hit test behavior (how this widget participates in hit testing).
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
      fn is_focusable(&self) -> bool { self.sense().contains(Sense::FOCUS) }

      /// Children for focus traversal and hit testing.
      fn focusable_children(&self) -> Vec<WidgetId> { vec![] }

      /// Accept an action from a child (for overlay/popup propagation).
      fn accept_action(&mut self, action: &WidgetAction) -> bool { false }
  }
  ```
- [ ] Note: `handle_mouse()`, `handle_hover()`, `handle_key()` are REMOVED.
  All input handling moves to controllers.
- [ ] `draw()` renamed to `paint()` for clarity (Druid convention).
- [ ] `is_focusable()` replaced by `sense().contains(Sense::FOCUS)` (default impl).
- [ ] `accept_action()` and `focusable_children()` retained with same semantics.
- [ ] `WidgetResponse` and `CaptureRequest` types removed; controllers emit actions and
  request capture via `ControllerCtx` instead.
- [ ] **Migration ordering**: The trait change cannot be atomic (changing the trait
  signature breaks all 23 implementations simultaneously). Strategy:
  1. Add new methods with default impls to the existing `Widget` trait (backward compatible)
  2. Migrate widgets one wave at a time (implement new methods, stop using old ones)
  3. After all widgets are migrated, remove old methods
  This means the trait temporarily has both old and new methods. The old methods
  get `#[deprecated]` markers in step 1 to ensure nothing new uses them.
- [ ] **`LifecycleCtx` and `AnimCtx` definition**: The new `lifecycle()` and
  `anim_frame()` methods reference `LifecycleCtx` and `AnimCtx` context types that
  are not defined elsewhere in the plan. Define them in `oriterm_ui/src/widgets/mod.rs`.
  Use the same request mechanism as `ControllerCtx` (Section 04) for consistency:
  ```rust
  pub struct LifecycleCtx<'a> {
      pub widget_id: WidgetId,
      pub interaction: &'a InteractionManager,
      pub requests: ControllerRequests,  // same bitflag type as ControllerCtx
  }

  pub struct AnimCtx {
      pub widget_id: WidgetId,
      pub now: Instant,
      pub requests: ControllerRequests,
  }
  ```
  After the lifecycle/anim_frame call returns, the framework reads `requests` and
  applies side effects (same pattern as controller dispatch).

---

## 08.2 Migration Strategy

Migrate widgets in four waves: interactive (most complex), layout/chrome (including TabBar),
passive (simplest), and cross-crate.

- [ ] **Wave 1 — Interactive widgets** (have event handlers, hover state):
  ButtonWidget, ToggleWidget, CheckboxWidget, DropdownWidget, SliderWidget, TextInputWidget
  - Extract hover logic → HoverController
  - Extract click logic → ClickController
  - Extract drag logic → DragController
  - Extract focus logic → FocusController
  - Replace manual `is_hovered` → `ctx.is_hot()`
  - Replace manual color interpolation → VisualStateAnimator

- [ ] **Wave 2 — Layout/container widgets** (route events to children):
  ContainerWidget, PanelWidget, ScrollWidget, StackWidget, FormLayout,
  FormSection, FormRow, SettingsPanel, DialogWidget, MenuWidget
  - Remove manual event routing (framework handles propagation)
  - Keep layout logic unchanged
  - Update child iteration for new propagation model

- [ ] **Wave 2b — Chrome/interactive-container widgets:**
  WindowChromeWidget, WindowControlButton, IdOverrideButton
  - WindowChromeWidget: add ClickController for buttons, DragController for titlebar
  - WindowControlButton: extract hover/press → HoverController + ClickController
  - IdOverrideButton: extract click → ClickController
- [ ] **Wave 2c — TabBarWidget** (special case — most complex widget):
  TabBarWidget has per-tab hover tracking (`hover_progress: Vec<AnimatedValue<f32>>`),
  per-tab close button opacity, width multiplier animations, drag-to-reorder,
  tab tear-off, and context menu. This widget has its own submodule tree
  (`tab_bar/widget/`, `tab_bar/hit.rs`, `tab_bar/layout.rs`, `tab_bar/slide/`).
  Migration approach:
  - Per-tab hover: each tab gets a HoverController (but tabs are not separate widgets —
    they are painted regions within TabBarWidget). **Decision needed**: either
    (a) convert each tab into a child widget, or (b) keep TabBarWidget as a monolithic
    widget that internally uses the interaction state system.
  - Recommended: option (b) — TabBarWidget is too tightly integrated to decompose.
    It should use `InteractionManager` for per-region hover via synthetic widget IDs
    (one per tab) but keep its monolithic layout/paint.
  **Risk note**: Synthetic WidgetIds create lifecycle management burden. Each tab
  creates a synthetic WidgetId. When tabs are added/removed, InteractionManager
  must register/deregister these IDs. Tab reorder must update hot path positions.
  This is the highest-risk migration in the plan. The old hover mechanism
  (`Vec<AnimatedValue<f32>>` indexed by tab position) can coexist with the new
  framework temporarily if TabBarWidget migration blocks the rest of the plan.
  - Tab drag: DragController with tear-off threshold
  - Context menu: handled at app layer (no controller needed)

- [ ] **Wave 3 — Passive widgets** (no event handling):
  LabelWidget, SeparatorWidget, SpacerWidget
  - Add `sense() -> Sense::none()`
  - Rename `draw()` to `paint()`
  - No other changes needed
  - **Note:** StatusBadge does NOT implement `Widget` — it is a standalone drawing helper
    and does not need migration. RichLabel is a new widget (Section 07), not an existing one.

- [ ] **Wave 4 — Cross-crate widgets** (in `oriterm` crate, not `oriterm_ui`):
  TerminalGridWidget (`oriterm/src/widgets/terminal_grid/mod.rs`, 141 lines),
  TerminalPreviewWidget (`oriterm/src/widgets/terminal_preview/mod.rs`, 106 lines)
  - These live in the binary crate, not the library crate
  - Both implement `Widget` and use `handle_mouse()`, `handle_hover()`, `handle_key()`
  - Both import from `oriterm_ui::input::{HoverEvent, KeyEvent, MouseEvent}`
  - Must migrate to new trait shape (add `sense()`, `paint()`, remove old methods)
  - TerminalGridWidget: `Sense::click_and_drag().union(Sense::FOCUS)` (receives all input)
  - TerminalPreviewWidget: `Sense::none()` (display only, 106 lines)
  - **IMPORTANT**: These must be migrated AFTER Section 08.6 removes the old trait methods,
    or simultaneously. Cannot remove old methods from the trait while these still use them.

---

## 08.3 Migrate Interactive Widgets

**File(s):** `oriterm_ui/src/widgets/button/mod.rs`, `toggle/mod.rs`, `checkbox/mod.rs`,
`dropdown/mod.rs`, `slider/mod.rs`, `text_input/mod.rs` + `text_input/widget_impl.rs`

For each interactive widget:

- [ ] **ButtonWidget**:
  - Remove: `hovered: bool`, `pressed: bool`, `hover_progress: AnimatedValue<f32>`,
    manual `HoverEvent` handling in `handle_hover()`
  - Add: `controllers: Vec<Box<dyn EventController>>` with HoverController + ClickController
  - Add: `animator: VisualStateAnimator` with `common_states()` (Normal/Hovered/Pressed/Disabled)
  - `paint()`: use `animator.get_bg_color(now)` instead of manual interpolation
  - `sense()`: `Sense::click()`

- [ ] **ToggleWidget**:
  - Remove: manual hover tracking
  - Add: HoverController + ClickController
  - Add: `common_states()` animator
  - Keep: thumb slide animation (explicit `AnimProperty<f32>` for thumb position)
  - `sense()`: `Sense::click()`

- [ ] **CheckboxWidget**:
  - Remove: manual hover tracking
  - Add: HoverController + ClickController
  - `sense()`: `Sense::click()`

- [ ] **DropdownWidget**:
  - Remove: manual hover, click handling
  - Add: HoverController + ClickController + FocusController
  - Keep: keyboard arrow navigation (in FocusController or custom controller)
  - `sense()`: `Sense::click().union(Sense::FOCUS)`

- [ ] **SliderWidget**:
  - Remove: manual drag tracking
  - Add: HoverController + DragController + FocusController
  - `sense()`: `Sense::drag().union(Sense::FOCUS)`

- [ ] **TextInputWidget**:
  - Remove: manual click, drag, key handling
  - Add: ClickController + DragController + FocusController
  - `sense()`: `Sense::click_and_drag().union(Sense::FOCUS)`

---

## 08.4 Migrate Layout Widgets

**File(s):** `oriterm_ui/src/widgets/container/mod.rs`, `panel/mod.rs`, `scroll/mod.rs`,
`stack/mod.rs`, `form_layout/mod.rs`, `form_section/mod.rs`, `form_row/mod.rs`,
`settings_panel/mod.rs`, `dialog/mod.rs`, `menu/widget_impl.rs`,
`tab_bar/widget/mod.rs`, `window_chrome/mod.rs`, `window_chrome/controls.rs`

- [ ] **ContainerWidget**:
  - Remove: `ContainerInputState` (in `container/mod.rs`), entire
    `container/event_dispatch.rs` submodule (manual per-container child dispatch)
  - Framework handles child event dispatch via propagation pipeline
  - Keep: layout logic unchanged
  - Add: optional hover tracking via `.with_hover(true)` for setting rows
  - `sense()`: `Sense::none()` by default, `Sense::hover()` when hover enabled

- [ ] **ScrollWidget**:
  - Remove: manual scroll handling, scrollbar drag
  - Add: ScrollController for wheel events
  - Add: DragController for scrollbar thumb drag
  - Keep: clip/translate rendering logic
  - `sense()`: `Sense::drag()` (for scrollbar)

- [ ] **PanelWidget**: minimal changes (no event handling)
- [ ] **FormLayout, FormSection, FormRow**: minimal changes (layout only)
- [ ] **SettingsPanel**: minimal changes (delegates to children)

---

## 08.5 Migrate Passive Widgets

**File(s):** `oriterm_ui/src/widgets/label/mod.rs`, `separator/mod.rs`, `spacer/mod.rs`

- [ ] LabelWidget: `sense() -> Sense::none()`, rename `draw()` to `paint()`
- [ ] SeparatorWidget: same
- [ ] SpacerWidget: same
- [ ] **StatusBadge**: No migration needed — it does not implement `Widget` (it's a
  standalone drawing helper, not part of the widget tree)
- [ ] **WindowChrome**: Migrated in Wave 2b (Section 08.2), not here

---

## 08.6 Remove Legacy Event Methods

**File(s):** `oriterm_ui/src/widgets/mod.rs`

- [ ] Remove `handle_mouse()` from Widget trait
- [ ] Remove `handle_hover()` from Widget trait
- [ ] Remove `handle_key()` from Widget trait
- [ ] Remove `HoverEvent` enum (replaced by `LifecycleEvent::HotChanged`)
- [ ] Remove `ContainerInputState` from `container/mod.rs` (replaced by framework propagation)
- [ ] Remove `WidgetResponse` and `CaptureRequest` from `widgets/mod.rs`
  (actions now emitted via controllers, capture via `ControllerCtx`)
- [ ] Remove or update `From<EventResponse> for DirtyKind` in `invalidation/mod.rs`
  (7 usages in production, 7 in tests). The invalidation system needs a new conversion
  from `ControllerRequests` bitflags to `DirtyKind` to replace the old `EventResponse`
  path. This is a **required** co-change when removing `EventResponse`.
- [ ] Update all callers in `oriterm` crate:
  - `dialog_context/content_actions.rs` — currently calls `handle_key`, `handle_hover`
  - `dialog_rendering.rs` — renders dialog content via `draw()`
  - `dialog_context/event_handling/` — dispatches mouse/hover events
  - `dialog_context/event_handling/mouse.rs` — mouse event dispatch
  - `app/settings_overlay/` — builds and manages settings panel
  - `app/settings_overlay/action_handler/mod.rs` — routes widget actions to config
  - `app/mouse_input.rs` — routes mouse events to widgets via `InputState`
- [ ] Update all callers that use `EventResponse` / `HoverEvent` / `MouseEvent` types
  from `oriterm_ui::input`. Search for `use oriterm_ui::input::` across the `oriterm`
  crate to find all import sites. Known callers (8 files):
  - `widgets/terminal_preview/mod.rs` — imports `HoverEvent, KeyEvent, MouseEvent`
  - `widgets/terminal_grid/mod.rs` — imports `HoverEvent, KeyEvent, MouseEvent`
  - `widgets/terminal_grid/tests.rs` — imports `HoverEvent, KeyEvent, Modifiers`
  - `app/tab_bar_input.rs` — imports `MouseButton, MouseEvent, MouseEventKind`
  - `app/keyboard_input/mod.rs` — imports `Key`
  - `app/dialog_context/content_actions.rs` — imports event types
  - `app/dialog_context/event_handling/mouse.rs` — imports `MouseButton, MouseEvent, MouseEventKind, ScrollDelta`
  - `app/dialog_context/event_handling/mod.rs` — imports `EventResponse, HoverEvent, MouseEvent, MouseEventKind`
  - `app/cursor_hover.rs` — cursor hover tracking
  - `app/chrome/` — chrome event handling
  - `app/event_loop.rs` — main event loop dispatching
- [ ] Remove `container/event_dispatch.rs` file entirely (all 201 lines)
- [ ] Update `overlay/manager/event_routing.rs` (333 lines): calls `handle_mouse()` (2 sites),
  `handle_hover()` (2 sites) directly on overlay widgets. Must be updated to use the new
  event pipeline/controller dispatch. This is a **required** co-change when removing the
  old trait methods — overlay widgets will not receive events otherwise.

---

## 08.7 Completion Checklist

- [ ] New Widget trait shape with `sense()`, `controllers()`, `visual_states()`,
  `paint()`, `lifecycle()`, `anim_frame()`
- [ ] All 6 interactive widgets migrated to controllers + visual state animators
  (Button, Toggle, Checkbox, Dropdown, Slider, TextInput)
- [ ] All 10 layout/container widgets migrated (no manual event routing)
  (Container, Panel, Scroll, Stack, FormLayout, FormSection, FormRow,
   SettingsPanel, Dialog, Menu)
- [ ] All 3 chrome widgets migrated (WindowChrome, WindowControlButton, IdOverrideButton)
- [ ] TabBarWidget migrated (highest-risk widget -- synthetic WidgetIds for per-tab hover)
- [ ] All 3 passive widgets migrated (Label, Separator, Spacer; Sense::none, paint rename)
- [ ] All 2 cross-crate widgets migrated (TerminalGridWidget, TerminalPreviewWidget
  in `oriterm/src/widgets/`)
- [ ] Legacy `handle_mouse()`, `handle_hover()`, `handle_key()` removed from trait
- [ ] `HoverEvent`, `ContainerInputState` removed
- [ ] `draw()` → `paint()` rename: call sites across `oriterm_ui/src/widgets/` (23 impls)
  plus 2 in `oriterm/src/widgets/` (TerminalGridWidget, TerminalPreviewWidget). All
  container widgets that call `child.draw()` must be updated to `child.paint()`. All test
  files that call `.draw()` must be updated. The `dialog/rendering.rs` file calls `.draw()`
  on the settings panel.
- [ ] `DrawCtx::animations_running: &Cell<bool>` field removed. 71 usages across
  29 files (production and test code):
  - **Production widgets** that set it: `ButtonWidget::draw()`, `ToggleWidget::draw()`,
    `WindowControlButton::draw()`, `TabBarWidget::draw()` (2 sites)
  - **Container widgets** that propagate it: `ContainerWidget`, `StackWidget`,
    `ScrollWidget`, `PanelWidget`, `FormLayout`, `FormSection`, `FormRow`,
    `SettingsPanel`, `WindowChromeWidget`, `DialogWidget`
  - **Test files**: ~40 sites construct `DrawCtx` with `animations_running: &anim_flag`
  All production usages migrate to `ctx.request_anim_frame()` (called in `anim_frame()`
  or `paint()` when `animator.is_animating(now)`). All test `DrawCtx` constructions
  lose the field. Container propagation is no longer needed (framework owns scheduling).
- [ ] Settings dialog works with new trait (all controls functional)
- [ ] Tab bar works with new trait
- [ ] Overlay/popup system works with new propagation
- [ ] No regressions in any existing UI functionality
- [ ] `./test-all.sh` green, `./clippy-all.sh` green, `./build-all.sh` green

**Exit Criteria:** The settings dialog opens, all dropdowns/toggles/checkboxes work with
hover animations, keyboard navigation (tab) works, and the dialog closes with Save/Cancel.
All via the new Widget trait, controllers, and visual state animators. Zero legacy event
methods remain in the codebase.
