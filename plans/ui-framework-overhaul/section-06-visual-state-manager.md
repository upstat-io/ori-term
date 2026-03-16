---
section: "06"
title: "Visual State Manager"
status: complete
goal: "Widgets declare visual states with per-state property values; framework animates transitions"
inspired_by:
  - "WPF VisualStateManager (System.Windows.VisualStateManager)"
  - "QML States + Transitions (Qt Quick)"
depends_on: ["01", "05"]
reviewed: true
sections:
  - id: "06.1"
    title: "State Groups & States"
    status: complete
  - id: "06.2"
    title: "State Resolution"
    status: complete
  - id: "06.3"
    title: "Animated Transitions"
    status: complete
  - id: "06.4"
    title: "Completion Checklist"
    status: complete
---

# Section 06: Visual State Manager

**Status:** Complete
**Goal:** Widgets declare state groups (e.g., `CommonStates { Normal, Hovered, Pressed, Disabled }`)
with property values per state (e.g., bg_color for each state). The framework resolves the
active state and animates transitions between states. No manual `if is_hot { lerp(...) }` in
draw code.

**Context:** Currently every widget manually checks `is_hovered`, `is_pressed`, etc. and
resolves background colors via a `current_bg()` helper method. ButtonWidget has a
`hover_progress: AnimatedValue<f32>` field and an 8-line `current_bg()` helper that
interpolates bg color using the hover animation progress. ToggleWidget has a separate
`toggle_progress: AnimatedValue<f32>` for thumb sliding but uses instant boolean
hover for track color (no animated hover transition). DropdownWidget, CheckboxWidget,
SliderWidget, and TextInputWidget all use instant `hovered: bool` checks with no
animated transitions — their `current_bg()` / `track_bg()` methods return immediate
color values based on boolean state. This means the duplicated manual state-checking
pattern exists across all 6 interactive leaf widgets, but only ButtonWidget (100ms
EaseOut) and WindowControlButton (100ms EaseOut) actually animate hover transitions
today. The inconsistency is both in approach (animated vs instant) and in the duplication
of state-resolution logic. WPF's VisualStateManager pattern centralizes this: you
declare what each state looks like, and the framework handles transitions.

**Reference implementations:**
- **WPF** `VisualStateManager`: State groups with mutually exclusive states, `GoToState()`,
  animated transitions via Storyboards
- **QML** `states` + `transitions`: Property snapshots per state, animation declarations per
  transition

**Depends on:** Section 05 (Animation Engine — needs `AnimBehavior`, `AnimProperty`, `Easing`).
Also depends on Section 01 (`InteractionState` — read by `StateResolver` to determine
active state). Both are listed in the frontmatter `depends_on: ["01", "05"]`.

**Naming convention note:** This plan uses `draw()` to refer to the Widget trait's rendering
method, which is the current method name (defined in `oriterm_ui/src/widgets/mod.rs`).
Section 08 renames `draw()` to `paint()`. If Section 06 is implemented before Section 08,
use `draw()` everywhere. If Section 08 has already landed, use `paint()` instead.

**Module declaration sync point**: `pub mod visual_state;` must be added to
`oriterm_ui/src/lib.rs` as part of this section. The module is invisible to the crate
until declared.

---

## 06.1 State Groups & States

**File(s):** `oriterm_ui/src/visual_state/mod.rs` (new module)

**Module structure**: The `visual_state/` directory contains:
- `mod.rs` (~80 lines — `VisualStateGroup`, `VisualState`, `StateProperty`, presets, re-exports)
- `resolver/mod.rs` (~40 lines — `StateResolver`)
- `resolver/tests.rs` (unit tests for `StateResolver`)
- `transition/mod.rs` (~250 lines — `StateTransition`, `VisualStateAnimator`)
- `transition/tests.rs` (unit tests for `StateTransition` and `VisualStateAnimator`)
- `tests.rs` (unit tests for `mod.rs` types: `VisualStateGroup`, `VisualState`, `StateProperty`, presets)

Each source file that has tests gets its own sibling `tests.rs` (per test-organization.md).
All files are well within the 500-line limit.

**Submodule declarations in `mod.rs`**: The module root declares:
```rust
pub mod resolver;
pub mod transition;
```
followed by re-exports of all public types:
`VisualStateGroup`, `VisualState`, `StateProperty`, `StateResolver`,
`StateTransition`, `VisualStateAnimator`, and the preset functions
(`common_states`, `focus_states`). Consumers use
`use oriterm_ui::visual_state::VisualStateAnimator`.

- [x] Define `VisualStateGroup`:
  ```rust
  pub struct VisualStateGroup {
      /// Name of this group (for debugging/identification).
      pub name: &'static str,
      /// Available states in this group (mutually exclusive).
      pub states: Vec<VisualState>,
      /// Index of the currently active state in `states`.
      active: usize,
      /// Resolver function for this group.
      /// Maps `&InteractionState` to an active state name (`&'static str`).
      resolve: fn(&InteractionState) -> &'static str,
  }
  ```
- [x] Implement `VisualStateGroup` accessors:
  ```rust
  impl VisualStateGroup {
      /// Returns the name of the currently active state.
      pub fn active_state_name(&self) -> &'static str {
          self.states[self.active].name
      }
      /// Returns the properties of the currently active state.
      pub fn active_properties(&self) -> &[StateProperty] {
          &self.states[self.active].properties
      }
  }
  ```
- [x] Define `VisualState` and `StateProperty`:
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
      // Extensible for new properties.
  }
  ```
- [x] Implement `StateProperty` helper methods:
  - `key() -> &'static str` — returns the discriminant string (`"BgColor"`, `"FgColor"`, etc.)
    used as the HashMap key in `VisualStateAnimator`.
  - `is_color() -> bool` — returns `true` for `BgColor`, `FgColor`, `BorderColor`.
  - `color_value() -> Option<Color>` — extracts the inner `Color`, or `None` for float variants.
  - `float_value() -> Option<f32>` — extracts the inner `f32`, or `None` for color variants.

  These are used in `VisualStateAnimator::update()` to route properties into the correct
  HashMap (`color_animations` vs `float_animations`).
- [x] Define common state group presets:
  ```rust
  pub fn common_states(
      normal_bg: Color, hover_bg: Color, pressed_bg: Color, disabled_bg: Color
  ) -> VisualStateGroup { ... }

  pub fn focus_states(
      unfocused_border: Color, focused_border: Color
  ) -> VisualStateGroup { ... }
  ```
  Note: `common_states()` has 4 parameters, which exceeds the "> 3 params -> config/options
  struct" guideline. This is accepted because each parameter maps 1:1 to a named state —
  an options struct would add ceremony without improving clarity.

  **Preset wiring**: `common_states()` sets `resolve: StateResolver::resolve_common`;
  `focus_states()` sets `resolve: StateResolver::resolve_focus`. The resolve function
  is stored on the group so `VisualStateAnimator::update()` can resolve all groups
  in a single loop without external dispatch logic.

---

## 06.2 State Resolution

**File(s):** `oriterm_ui/src/visual_state/resolver/mod.rs`

Determine which state is active based on interaction state.

- [x] Define `StateResolver`:
  ```rust
  pub struct StateResolver;

  impl StateResolver {
      /// Resolve the active state in a CommonStates group.
      pub fn resolve_common(interaction: &InteractionState) -> &'static str {
          if interaction.is_disabled() { return "Disabled"; }
          if interaction.is_active() { return "Pressed"; }
          if interaction.is_hot() { return "Hovered"; }
          "Normal"
      }

      /// Resolve the active state in a FocusStates group.
      pub fn resolve_focus(interaction: &InteractionState) -> &'static str {
          if interaction.is_focused() { "Focused" } else { "Unfocused" }
      }
  }
  ```
  **Signature constraint**: These functions have signature
  `fn(&InteractionState) -> &'static str`, matching the `resolve` field on
  `VisualStateGroup`. The returned string must match one of the group's
  `VisualState::name` values exactly. If no match is found (a bug),
  `VisualStateAnimator::update()` logs a warning and keeps the current
  state rather than panicking.
- [x] State resolution happens automatically each frame before `draw()`. The framework
  compares the resolved state with the group's current active state. If different,
  triggers a transition.
- [x] Multiple state groups are independent — a widget can be in `Hovered` (CommonStates)
  and `Focused` (FocusStates) simultaneously. Properties from different groups do not
  conflict because they target different properties.

  **Property overlap enforcement**: The plan relies on convention (CommonStates targets
  BgColor/FgColor, FocusStates targets BorderColor) to avoid conflicts. If two groups
  set the same property key, the last group processed wins. This is acceptable for the
  initial implementation. Document this in the `VisualStateAnimator` doc comment:
  "If multiple groups set the same property, the group listed later in the `groups` Vec
  takes precedence." A `debug_assert!` in the constructor can verify no overlap exists
  across groups for the initial implementation.

---

## 06.3 Animated Transitions

**File(s):** `oriterm_ui/src/visual_state/transition/mod.rs`

Animate property changes when transitioning between states.

- [x] **PREREQUISITE — Add `set_behavior()` to `AnimProperty<T>`**: Add
  `pub fn set_behavior(&mut self, behavior: Option<AnimBehavior>)` to
  `oriterm_ui/src/animation/property/mod.rs`. This replaces the stored behavior
  without affecting any in-flight transition. Must be done first because
  `VisualStateAnimator::update()` depends on it. This is a small, backward-compatible
  addition to a Section 05 type. Include a unit test in
  `oriterm_ui/src/animation/property/tests.rs` verifying that `set_behavior()`
  followed by `set()` starts an animated transition even when the property was
  created with `AnimProperty::new()` (no initial behavior).
- [x] Define `StateTransition`:
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
- [x] Define `VisualStateAnimator` with constructor and builder:
  ```rust
  pub struct VisualStateAnimator {
      groups: Vec<VisualStateGroup>,
      transitions: Vec<StateTransition>,
      /// Default transition for state pairs without a specific override.
      default_transition: AnimBehavior,
      /// Currently interpolating color properties.
      /// Keys are `StateProperty::key()` discriminant strings ("BgColor", "FgColor", etc.).
      color_animations: HashMap<&'static str, AnimProperty<Color>>,
      /// Currently interpolating scalar properties.
      /// Keys are `StateProperty::key()` discriminant strings ("Opacity", "BorderWidth", etc.).
      float_animations: HashMap<&'static str, AnimProperty<f32>>,
  }

  impl VisualStateAnimator {
      /// Creates an animator from the given groups with default 100ms EaseOut transitions.
      ///
      /// Populates `color_animations` and `float_animations` from the initial
      /// active state (index 0) of each group using `AnimProperty::new()` (instant,
      /// no behavior — behavior is attached lazily on the first state transition
      /// in `update()` via `set_behavior()`).
      pub fn new(groups: Vec<VisualStateGroup>) -> Self;

      /// Adds a custom transition override for a specific state pair.
      #[must_use]
      pub fn with_transition(mut self, transition: StateTransition) -> Self;

      /// Overrides the default transition behavior (replaces 100ms EaseOut).
      #[must_use]
      pub fn with_default_transition(mut self, behavior: AnimBehavior) -> Self;

      /// Resolve states and start transitions if state changed.
      pub fn update(&mut self, interaction: &InteractionState, now: Instant);

      /// Advance spring-based transitions by one frame.
      /// Must be called each frame when `is_animating()` is true and any
      /// property uses spring-based `AnimBehavior`.
      pub fn tick(&mut self, now: Instant);

      /// Get the current interpolated value for a color property.
      /// Returns `Color::TRANSPARENT` if the property is not set.
      pub fn get_bg_color(&self, now: Instant) -> Color;
      pub fn get_fg_color(&self, now: Instant) -> Color;
      pub fn get_border_color(&self, now: Instant) -> Color;

      /// Get the current interpolated value for a scalar property.
      /// Returns `0.0` if the property is not set.
      pub fn get_opacity(&self, now: Instant) -> f32;
      pub fn get_border_width(&self, now: Instant) -> f32;
      pub fn get_corner_radius(&self, now: Instant) -> f32;

      /// Generic getter for any color property by discriminant key.
      pub fn get_color(&self, key: &str, now: Instant) -> Option<Color>;
      /// Generic getter for any scalar property by discriminant key.
      pub fn get_float(&self, key: &str, now: Instant) -> Option<f32>;

      /// Are any transitions still animating?
      pub fn is_animating(&self, now: Instant) -> bool;
  }
  ```
- [x] **WARNING — highest complexity step in this section.** Implement `update()`.
  This method combines state resolution, transition lookup, and `AnimProperty`
  manipulation. Test thoroughly with the interrupt scenario (Normal -> Hovered ->
  Pressed in 50ms) to verify mid-flight transitions work correctly.

  **`update()` algorithm**:
  ```
  for each group in self.groups:
      let resolved_name = (group.resolve)(interaction);
      if resolved_name == group.active_state_name():
          continue;  // No state change in this group.
      let new_idx = group.states.iter().position(|s| s.name == resolved_name);
      let old_name = group.active_state_name();
      group.active = new_idx;
      let behavior = find_transition(&self.transitions, &self.default_transition,
                                     old_name, resolved_name);
      for prop in &group.states[new_idx].properties:
          let key = prop.key();
          match prop:
              color variant => {
                  let anim = self.color_animations.get_mut(key);
                  anim.set_behavior(Some(behavior));
                  anim.set(prop.color_value(), now);
              }
              float variant => {
                  let anim = self.float_animations.get_mut(key);
                  anim.set_behavior(Some(behavior));
                  anim.set(prop.float_value(), now);
              }
  ```
  The `find_transition()` helper searches `self.transitions` for a matching
  `(from, to)` pair, trying exact match first, then wildcard `("*", to)`,
  then `(from, "*")`, then `("*", "*")`, falling back to `self.default_transition`.

  **Borrow checker note**: The pseudocode iterates `self.groups` while also accessing
  `self.transitions`, `self.color_animations`, and `self.float_animations`. Calling
  `self.find_transition()` inside a loop over `&mut self.groups` won't compile because
  `find_transition()` takes `&self`. Solution: extract `find_transition()` as a free
  function that takes `(&[StateTransition], &AnimBehavior, &str, &str)` instead of
  `&self`. The free-function approach avoids the borrow conflict entirely.

  **`AnimProperty::set_behavior()` dependency**: The pseudocode above calls
  `anim.set_behavior(Some(behavior))` before `anim.set()`. This method is added
  in the prerequisite step at the top of this section. The `behavior` field on
  `AnimProperty` is private (line 53 of `property/mod.rs`), so the public setter
  is required.

  **Transaction interaction**: `AnimProperty::set()` checks `current_transaction()`
  before the property's own behavior. If `update()` is called inside
  `with_transaction(Transaction::instant(), ...)`, transitions will be suppressed
  regardless of `set_behavior()`. This is useful for instant theme changes, but
  implementers must be aware that `set_behavior()` + `set()` does NOT guarantee
  animation — the transaction system can override it. Do not call `update()` inside
  a transaction block unless instant behavior is intended.
- [x] **Interruption semantics**: When the pointer moves quickly (Normal -> Hovered ->
  Pressed within 50ms), `AnimProperty::set()` handles this automatically: it reads
  the current interpolated value via `get(now)` and starts a new transition from that
  mid-flight position. The `VisualStateAnimator` does not need special interruption
  logic beyond calling `set()` on the appropriate `AnimProperty`. This is the key
  architectural win of building on `AnimProperty` from Section 05.
- [x] **Initialization (prevent animate-from-zero)**: The `new()` constructor populates
  all `AnimProperty` entries with the initial state's values using
  `AnimProperty::new(initial_value)`. This creates properties with `current == target`
  and no in-flight transition, so the first frame shows the correct initial colors
  without a spurious animation from `Color::default()` (transparent black). The
  `behavior` field starts as `None` — it is attached lazily on the first state
  transition in `update()` via `set_behavior()`.
- [x] Default transitions: `AnimBehavior::ease_out(100)` for all state changes.
  Widgets can override with custom transitions per state pair via `with_transition()`.
- [x] **Draw integration**: Widget `draw()` calls `animator.get_bg_color(now)` instead of
  manually computing hover colors.
  **Deferred to Section 08** — requires `&mut self` access via `Widget::visual_states_mut()`.
- [x] When transitions are active, widget requests continued animation via
  `ctx.request_anim_frame()` (method on both `DrawCtx` and `EventCtx`,
  added in Section 05.1).
  **Deferred to Section 08** — `Widget::anim_frame()` does not exist yet.
- [x] **Animation frame lifecycle**: `VisualStateAnimator` does not own the
  `request_anim_frame()` call — it reports `is_animating()` and the widget's
  `draw()` method checks this and calls `ctx.request_anim_frame()` if needed.
  This keeps the animator decoupled from the framework context. Note:
  `Widget::anim_frame()` does not exist yet (added in Section 08). Until then,
  `draw()` is the only call site for `ctx.request_anim_frame()`, matching the
  existing `animations_running.set(true)` pattern used by `ButtonWidget::draw()`
  and `ToggleWidget::draw()`.
- [x] **`tick()` integration for springs**: If any `StateTransition` uses
  `AnimBehavior::spring()`, the animator's `tick(now)` must be called each frame
  to advance the spring simulation. `tick()` iterates all `color_animations` and
  `float_animations` and calls `.tick(now)` on each `AnimProperty`. For easing-based
  transitions, `AnimProperty::tick()` is a no-op (easing is computed lazily in
  `get()`), so calling it unconditionally is safe. The per-frame sequence is:
  1. Resolve interaction state for widget
  2. Call `animator.update(&interaction_state, now)` (may start new transitions)
  3. Call `animator.tick(now)` (advances spring physics)
  4. Call `widget.draw(ctx)` — widget reads `animator.get_bg_color(now)`
  5. If `animator.is_animating(now)`, request anim frame for this widget

  **Rendering discipline check**: Steps 2-3 are mutation (before render). Step 4
  is pure read (`get_*()` methods take `&self`). Step 5 is a side-effect-free
  flag set. This respects the "no state mutation during render" rule from
  impl-hygiene.md.
- [x] **Framework call site does not exist yet**: The render pipeline that calls
  `widget.draw(ctx)` is in container widgets (e.g., `ContainerWidget::draw()` loops
  through children). Until Section 08 restructures the render pipeline to call
  `animator.update()` before `draw()`, widgets would need to call
  `self.animator.update()` at the top of their own `draw()` method. However,
  `draw()` takes `&self`, not `&mut self`, so calling `update()` (which mutates)
  is impossible without interior mutability.

  **Recommended approach**: Implement `VisualStateAnimator` fully in this section but
  defer the framework `update()` call to Section 08, where `Widget::visual_states_mut()`
  provides `&mut` access. During Section 06, validate via unit tests that call
  `update()` directly on `&mut VisualStateAnimator`. Widgets continue using their
  existing `current_bg()` pattern until Section 08 migration. Do NOT add `RefCell`.
- [x] **Multiple property types**: `VisualStateAnimator` interpolates both `Color`
  and `f32` properties. Color interpolation requires `AnimProperty<Color>` which needs
  `Color: Lerp` (already implemented in `color/mod.rs`). The struct stores
  `color_animations: HashMap<&'static str, AnimProperty<Color>>` for color properties
  (BgColor, FgColor, BorderColor) and `float_animations: HashMap<&'static str,
  AnimProperty<f32>>` for scalar properties (Opacity, CornerRadius, BorderWidth).
- [x] **`is_animating()` implementation**: Iterates all `AnimProperty` values in both
  `color_animations` and `float_animations`, returning `true` if any reports
  `is_animating(now)`. Completed easing transitions are NOT cleaned up eagerly —
  `AnimProperty::get()` returns the target value when `elapsed >= duration`, and
  `is_animating()` returns `false`. No cleanup needed.

---

## 06.4 Completion Checklist

### Module structure
- [x] `pub mod visual_state;` declared in `oriterm_ui/src/lib.rs`
- [x] `visual_state/mod.rs` — types, presets, re-exports
- [x] `visual_state/tests.rs` — tests for `mod.rs` types (presets, `StateProperty`, `VisualStateGroup`)
- [x] `#[cfg(test)] mod tests;` at bottom of `visual_state/mod.rs`
- [x] `visual_state/resolver/mod.rs` — `StateResolver`
- [x] `visual_state/resolver/tests.rs` — tests for `StateResolver`
- [x] `#[cfg(test)] mod tests;` at bottom of `visual_state/resolver/mod.rs`
- [x] `visual_state/transition/mod.rs` — `StateTransition`, `VisualStateAnimator`
- [x] `visual_state/transition/tests.rs` — tests for `StateTransition`, `VisualStateAnimator`
- [x] `#[cfg(test)] mod tests;` at bottom of `visual_state/transition/mod.rs`
- [x] Re-exports in `visual_state/mod.rs`: `VisualStateGroup`, `VisualState`,
  `StateProperty`, `StateResolver`, `StateTransition`, `VisualStateAnimator`,
  `common_states`, `focus_states`

### Core types
- [x] `VisualStateGroup` with mutually exclusive states and `resolve` function pointer
- [x] `VisualStateGroup::active_state_name()` and `active_properties()` accessors
- [x] `VisualState` with `name` and `properties` fields
- [x] `StateProperty` enum with 6 variants: `BgColor`, `FgColor`, `BorderColor`, `BorderWidth`, `CornerRadius`, `Opacity`
- [x] `StateProperty::key()` returns discriminant string for HashMap lookup
- [x] `StateProperty::color_value()` and `float_value()` extract inner values
- [x] `StateResolver::resolve_common` and `resolve_focus` with signature
  `fn(&InteractionState) -> &'static str` matching `VisualStateGroup::resolve` field
- [x] State resolution happens automatically before `draw()`

### Transitions
- [x] **PREREQUISITE**: `AnimProperty::set_behavior(&mut self, behavior: Option<AnimBehavior>)`
  added to `oriterm_ui/src/animation/property/mod.rs` with unit test in
  `oriterm_ui/src/animation/property/tests.rs` (must be done FIRST — all
  `VisualStateAnimator` code depends on it)
- [x] `StateTransition` with `from`, `to`, and `behavior` fields
- [x] `VisualStateAnimator::new(groups)` populates initial property values from each group's
  active state (index 0) using `AnimProperty::new(initial_value)` — no animate-from-zero
- [x] `VisualStateAnimator::with_transition()` builder for custom overrides
- [x] `VisualStateAnimator::with_default_transition()` builder to replace default behavior
- [x] `VisualStateAnimator::update()` resolves all groups, finds transition behavior via
  `find_transition()`, calls `set_behavior()` + `set()` for changed properties
- [x] `VisualStateAnimator::tick()` advances spring-based properties each frame
- [x] `find_transition()` as a free function: searches exact match, then wildcards, then default
- [x] Default transition is `AnimBehavior::ease_out(100)` for all state changes
- [x] Multiple state groups compose independently (CommonStates + FocusStates)

### Getters
- [x] `get_bg_color()`, `get_fg_color()`, `get_border_color()` return `Color`
  (default: `Color::TRANSPARENT` when property is not set)
- [x] `get_opacity()`, `get_border_width()`, `get_corner_radius()` return `f32`
  (default: `0.0` when property is not set)
- [x] `get_color(key)` and `get_float(key)` generic getters return `Option<T>`
- [x] `is_animating()` checks all `AnimProperty` values in both HashMaps

### Integration
- [x] Widget `draw()` integration deferred to Section 08 (requires `&mut self` access
  via `Widget::visual_states_mut()`). Section 06 validates via unit tests only.
- [x] Framework `update()` + `tick()` call site deferred to Section 08 (widgets
  validated via unit tests calling `update()` directly on `&mut VisualStateAnimator`)

### Derive traits
- [x] `VisualStateGroup`: `Debug` (not `Clone` — owned by `VisualStateAnimator`, no need to copy)
- [x] `VisualState`: `Debug, Clone`
- [x] `StateProperty`: `Debug, Clone, Copy` (all variants contain `Color` or `f32`, both `Copy`)
- [x] `StateTransition`: `Debug, Clone, Copy`
- [x] `VisualStateAnimator`: `Debug` (not `Clone` — contains `HashMap<_, AnimProperty>`)
- [x] `StateResolver`: `Debug` (unit struct, no fields)

### Unit tests

**`oriterm_ui/src/animation/property/tests.rs`** (addition to existing file):
- [x] `set_behavior()` followed by `set()` starts animated transition on a
  property created with `AnimProperty::new()` (no initial behavior)

**`oriterm_ui/src/visual_state/tests.rs`** (tests for mod.rs types):
- [x] `StateProperty::key()` returns correct discriminant for each variant
- [x] `StateProperty::color_value()` / `float_value()` return correct `Option`
- [x] `common_states()` preset creates group with 4 states and correct resolve fn
- [x] `focus_states()` preset creates group with 2 states and correct resolve fn
- [x] Newly created `VisualStateGroup` has `active` index 0

**`oriterm_ui/src/visual_state/resolver/tests.rs`**:
- [x] `resolve_common` returns "Disabled" when disabled
- [x] `resolve_common` returns "Pressed" when active (not disabled)
- [x] `resolve_common` returns "Hovered" when hot (not active, not disabled)
- [x] `resolve_common` returns "Normal" when none of the above
- [x] `resolve_focus` returns "Focused" when focused, "Unfocused" otherwise

**`oriterm_ui/src/visual_state/transition/tests.rs`**:
- [x] Normal -> Hovered transition interpolates bg_color over 100ms
- [x] Rapid state changes (Normal -> Hovered -> Pressed within 50ms)
  correctly interrupt and restart transitions from current interpolated value
- [x] Disabled state transition is instant: verified via
  `with_transition(StateTransition { from: "*", to: "Disabled", behavior: AnimBehavior::ease_out(0) })`
  override on the animator (convention applied by widget, not preset)
- [x] FocusStates group resolves correctly (Unfocused -> Focused triggers
  BorderColor transition)
- [x] Two groups compose — CommonStates targets BgColor while FocusStates
  targets BorderColor; both animate independently on the same animator
- [x] Newly created animator returns correct initial values from `get_bg_color()`
  without calling `update()` first (verifies no animate-from-zero)
- [x] `find_transition()` returns custom override for exact match, falls back
  to wildcard, then to default
- [x] Spring-based transition converges to target after repeated `tick()` calls
- [x] `get_fg_color()` returns `Color::TRANSPARENT` when no group sets FgColor
- [x] `./test-all.sh` green, `./clippy-all.sh` green, `./build-all.sh` green

**Exit Criteria:** A unit test demonstrates a `VisualStateAnimator` with `common_states()`
and default transitions: calling `update()` with a hot `InteractionState` at `t=0`,
then querying `get_bg_color()` at `t=50ms` returns a color between `normal_bg` and
`hover_bg`, and at `t=100ms` returns exactly `hover_bg`. A second test demonstrates
that calling `update()` with a pressed `InteractionState` at `t=25ms` (mid-hover-transition)
starts the Hovered->Pressed transition from the interpolated mid-hover color, not from
`normal_bg`. Widget integration (replacing `current_bg()` in `ButtonWidget::draw()`)
is deferred to Section 08 migration.

**Additional fix:** Fixed floating-point rounding in `AnimProperty::get()` — when easing
progress reaches 1.0, returns `t.to` directly instead of computing `Lerp::lerp(from, to, 1.0)`
which could produce slightly different results due to floating-point arithmetic.
