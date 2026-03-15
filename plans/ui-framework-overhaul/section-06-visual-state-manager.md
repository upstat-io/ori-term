---
section: "06"
title: "Visual State Manager"
status: not-started
goal: "Widgets declare visual states with per-state property values; framework animates transitions"
inspired_by:
  - "WPF VisualStateManager (System.Windows.VisualStateManager)"
  - "QML States + Transitions (Qt Quick)"
depends_on: ["05"]
reviewed: false
sections:
  - id: "06.1"
    title: "State Groups & States"
    status: not-started
  - id: "06.2"
    title: "State Resolution"
    status: not-started
  - id: "06.3"
    title: "Animated Transitions"
    status: not-started
  - id: "06.4"
    title: "Completion Checklist"
    status: not-started
---

# Section 06: Visual State Manager

**Status:** Not Started
**Goal:** Widgets declare state groups (e.g., `CommonStates { Normal, Hovered, Pressed, Disabled }`)
with property values per state (e.g., bg_color for each state). The framework resolves the
active state and animates transitions between states. No manual `if is_hot { lerp(...) }` in
paint code.

**Context:** Currently every widget manually checks `is_hovered`, `is_pressed`, etc. and
computes interpolated colors via a `current_bg()` helper method. ButtonWidget has a
`hover_progress: AnimatedValue<f32>` field and an 8-line `current_bg()` helper. ToggleWidget
has similar logic with an additional thumb animation. DropdownWidget has its own
hovered/pressed color resolution. This pattern is duplicated across all interactive widgets
with slightly inconsistent easing curves and durations. WPF's VisualStateManager pattern
centralizes this: you declare what each state looks like, and the framework handles
transitions.

**Reference implementations:**
- **WPF** `VisualStateManager`: State groups with mutually exclusive states, `GoToState()`,
  animated transitions via Storyboards
- **QML** `states` + `transitions`: Property snapshots per state, animation declarations per
  transition

**Depends on:** Section 05 (Animation Engine — needs AnimBehavior, AnimProperty, Easing).

---

## 06.1 State Groups & States

**File(s):** `oriterm_ui/src/visual_state/mod.rs` (new module)

- [ ] Define `VisualStateGroup`:
  ```rust
  pub struct VisualStateGroup {
      /// Name of this group (for debugging/identification).
      pub name: &'static str,
      /// Available states in this group (mutually exclusive).
      pub states: Vec<VisualState>,
      /// Current active state index.
      active: usize,
  }
  ```
- [ ] Define `VisualState`:
  ```rust
  pub struct VisualState {
      pub name: &'static str,
      /// Property values when this state is active.
      pub properties: Vec<StateProperty>,
  }

  pub enum StateProperty {
      BgColor(Color),
      FgColor(Color),
      BorderColor(Color),
      BorderWidth(f32),
      CornerRadius(f32),
      Opacity(f32),
      // Extensible for new properties
  }
  ```
- [ ] Define common state group presets:
  ```rust
  pub fn common_states(
      normal_bg: Color, hover_bg: Color, pressed_bg: Color, disabled_bg: Color
  ) -> VisualStateGroup { ... }

  pub fn focus_states(
      unfocused_border: Color, focused_border: Color
  ) -> VisualStateGroup { ... }
  ```

---

## 06.2 State Resolution

**File(s):** `oriterm_ui/src/visual_state/resolver.rs`

Determine which state is active based on interaction state.

- [ ] Define `StateResolver`:
  ```rust
  pub struct StateResolver;

  impl StateResolver {
      /// Given interaction state, resolve the active state in a CommonStates group.
      pub fn resolve_common(interaction: &InteractionState) -> &'static str {
          if interaction.is_disabled() { return "Disabled"; }
          if interaction.is_active() { return "Pressed"; }
          if interaction.is_hot() { return "Hovered"; }
          "Normal"
      }

      /// Given interaction state, resolve the active state in a FocusStates group.
      pub fn resolve_focus(interaction: &InteractionState) -> &'static str {
          if interaction.is_focused() { "Focused" } else { "Unfocused" }
      }
  }
  ```
- [ ] State resolution happens automatically each frame before paint. The framework
  compares resolved state with the group's current active state. If different,
  triggers a transition.
- [ ] Multiple state groups are independent — a widget can be in `Hovered` (CommonStates)
  and `Focused` (FocusStates) simultaneously. Properties from different groups don't
  conflict because they target different properties.

---

## 06.3 Animated Transitions

**File(s):** `oriterm_ui/src/visual_state/transition.rs`

Animate property changes when transitioning between states.

- [ ] Define `StateTransition`:
  ```rust
  pub struct StateTransition {
      /// State transitioning from ("*" = any).
      pub from: &'static str,
      /// State transitioning to ("*" = any).
      pub to: &'static str,
      /// Animation behavior for this transition.
      pub behavior: AnimBehavior,
  }
  ```
- [ ] Define `VisualStateAnimator`:
  ```rust
  pub struct VisualStateAnimator {
      groups: Vec<VisualStateGroup>,
      transitions: Vec<StateTransition>,
      /// Currently interpolating color properties (one per animatable color property).
      color_animations: HashMap<&'static str, AnimProperty<Color>>,
      float_animations: HashMap<&'static str, AnimProperty<f32>>,
  }

  impl VisualStateAnimator {
      /// Resolve states and start transitions if state changed.
      pub fn update(&mut self, interaction: &InteractionState, now: Instant);

      /// Get the current interpolated value for a property.
      pub fn get_bg_color(&self, now: Instant) -> Color;
      pub fn get_fg_color(&self, now: Instant) -> Color;
      pub fn get_border_color(&self, now: Instant) -> Color;
      pub fn get_opacity(&self, now: Instant) -> f32;

      /// Are any transitions still animating?
      pub fn is_animating(&self, now: Instant) -> bool;
  }
  ```
- [ ] Default transitions: `AnimBehavior::ease_out(100)` for all state changes.
  Widgets can override with custom transitions per state pair.
- [ ] Paint integration: Widget's `paint()` calls `animator.get_bg_color(now)` instead
  of manually computing hover colors.
- [ ] When transitions are active, widget requests `anim_frame()`.
- [ ] **Animation frame integration**: `VisualStateAnimator` does not own the
  `request_anim_frame()` call itself — it reports `is_animating()` and the widget's
  `paint()` or `anim_frame()` method checks this and calls `ctx.request_anim_frame()`
  if needed. This keeps the animator decoupled from the framework context.
- [ ] **State update call site**: `animator.update()` must be called by the framework
  before each `paint()` call, passing the current `InteractionState`. The framework
  should do this in the render pipeline, not the widget. Sequence:
  1. Resolve interaction state for widget
  2. Call `animator.update(&interaction_state, now)`
  3. If state changed, animator starts transitions
  4. Call `widget.paint(ctx)` — widget reads `animator.get_bg_color(now)`
  5. If `animator.is_animating(now)`, framework requests anim frame for this widget
- [ ] **Multiple property types**: `VisualStateAnimator` must interpolate both `Color`
  and `f32` properties. Color interpolation requires `AnimProperty<Color>` which needs
  `Color: Lerp` (already implemented in `color/mod.rs`). The struct stores
  `color_animations: HashMap<&'static str, AnimProperty<Color>>` for color properties
  (BgColor, FgColor, BorderColor) and `float_animations: HashMap<&'static str,
  AnimProperty<f32>>` for scalar properties (Opacity, CornerRadius, BorderWidth).

---

## 06.4 Completion Checklist

- [ ] `VisualStateGroup` with mutually exclusive states
- [ ] `StateProperty` enum covers bg, fg, border, opacity, corner radius
- [ ] `StateResolver` maps interaction state to visual state names
- [ ] State resolution happens automatically before paint
- [ ] `StateTransition` declares animation per state-pair
- [ ] `VisualStateAnimator` interpolates properties during transitions
- [ ] Default transitions provided (100ms EaseOut)
- [ ] Multiple state groups compose (CommonStates + FocusStates)
- [ ] Widget paint code uses `animator.get_bg_color(now)` — no manual conditionals
- [ ] Active transitions request `anim_frame()`
- [ ] Unit tests: Normal → Hovered transition interpolates bg_color over 100ms
- [ ] Unit tests: rapid state changes (Normal → Hovered → Pressed within 50ms)
  correctly interrupt and restart transitions from current interpolated value
- [ ] Unit tests: disabled state bypasses all transitions (instant change)
- [ ] Test file: `oriterm_ui/src/visual_state/tests.rs`
- [ ] `./test-all.sh` green, `./clippy-all.sh` green, `./build-all.sh` green

**Exit Criteria:** A button widget with `common_states()` and default transitions shows
smooth 100ms bg_color interpolation when the pointer enters and leaves, with zero manual
hover logic in the widget's paint method.
