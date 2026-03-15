---
section: "05"
title: "Animation Engine"
status: not-started
goal: "Unified animation system with AnimFrame timing, property behaviors, transactions, and springs"
inspired_by:
  - "Druid request_anim_frame() / AnimFrame(delta) (druid/src/contexts.rs)"
  - "QML Behavior on property (Qt Quick)"
  - "SwiftUI Transaction-based animation (SwiftUI/Animations)"
  - "egui request_repaint() / request_repaint_after() (egui/src/context.rs)"
depends_on: []
reviewed: false
sections:
  - id: "05.1"
    title: "AnimFrame Integration"
    status: not-started
  - id: "05.2"
    title: "Property Behaviors"
    status: not-started
  - id: "05.3"
    title: "Transactions"
    status: not-started
  - id: "05.4"
    title: "Spring Physics"
    status: not-started
  - id: "05.5"
    title: "Render Scheduling"
    status: not-started
  - id: "05.6"
    title: "Completion Checklist"
    status: not-started
---

# Section 05: Animation Engine

**Status:** Not Started
**Goal:** Widgets request animation frames via `ctx.request_anim_frame()` and receive
`AnimFrame(delta)` timing pulses. Properties can have `Behavior` declarations that
auto-animate changes. State mutations carry `Transaction` metadata specifying animation
curves. The event loop sleeps when no animations are active.

**Context:** The current animation system (`AnimatedValue<T>`) works but is ad-hoc: each
widget creates and manages its own `AnimatedValue` fields, manually calls `.set(value, now)`
to start animations, and checks `.is_animating()` to keep the render loop alive by setting
`ctx.animations_running` (a `&Cell<bool>` in `DrawCtx`). There's also `AnimationGroup`,
`AnimationSequence`, and `AnimationDelegate` infrastructure in the existing `animation/`
module for more complex animations, but there's no way to say "animate all transitions on
this property" — you have to explicitly manage animation start/end in every widget's event
handler. Druid, QML, and SwiftUI each solved a piece of this problem.

**Reference implementations:**
- **Druid** `druid/src/contexts.rs`: `request_anim_frame()` → `Event::AnimFrame(nanos)`
- **QML** `Behavior on`: declare "this property animates when it changes"
- **SwiftUI** `withAnimation(.spring())`: animation metadata on state changes
- **egui** `ctx.request_repaint_after(Duration)`: deferred repaint scheduling

**Depends on:** Nothing (independent of event system).

---

## 05.1 AnimFrame Integration

**File(s):** `oriterm_ui/src/animation/anim_frame.rs` (new file)

**Existing animation module files** (for reference): `mod.rs` (356 lines — Lerp, Easing,
Animation, AnimatedValue), `builder.rs` (AnimationBuilder), `delegate.rs` (AnimationDelegate),
`group.rs` (AnimationGroup), `sequence.rs` (AnimationSequence).

Replace the current `animations_running: &Cell<bool>` flag (set per-frame in `DrawCtx`) with
explicit animation frame requests.

- [ ] Add `request_anim_frame()` to contexts (`EventCtx`, `ControllerCtx`, `DrawCtx`).
  This replaces the current `ctx.animations_running.set(true)` pattern in `DrawCtx`.
  All contexts are defined in `oriterm_ui/src/widgets/mod.rs`:
  ```rust
  /// Request an animation frame on the next vsync. The widget will receive
  /// an AnimFrame event with the time delta since the last frame.
  pub fn request_anim_frame(&mut self) {
      self.anim_frame_requested = true;
  }
  ```
- [ ] Define `AnimFrameEvent`:
  ```rust
  pub struct AnimFrameEvent {
      /// Nanoseconds since the last AnimFrame delivered to this widget.
      /// 0 on the first frame after transitioning from idle to animating.
      pub delta_nanos: u64,
      /// Absolute timestamp for this frame.
      pub now: Instant,
  }
  ```
- [ ] Add `Widget::anim_frame(&mut self, event: &AnimFrameEvent, ctx: &mut AnimCtx)`:
  Widget method called when an animation frame is delivered (see Section 08).
- [ ] Framework tracks which widgets have requested anim frames. On next frame:
  1. Deliver `AnimFrame` to all requesting widgets
  2. Widget processes animation, calls `request_paint()` if visual change
  3. Widget calls `request_anim_frame()` again if animation is still running
  4. If no widgets request another frame, event loop returns to sleep

---

## 05.2 Property Behaviors

**File(s):** `oriterm_ui/src/animation/behavior.rs`

QML-inspired: declare "this property animates when changed."

- [ ] Define `AnimBehavior`:
  ```rust
  /// Declares how a property transitions when its target value changes.
  pub struct AnimBehavior {
      pub curve: AnimCurve,
  }

  impl AnimBehavior {
      pub fn ease_out(ms: u64) -> Self {
          Self { curve: AnimCurve::Easing {
              easing: Easing::EaseOut,
              duration: Duration::from_millis(ms),
          }}
      }
      pub fn spring() -> Self {
          Self { curve: AnimCurve::Spring(Spring::default()) }
      }
  }
  ```
- [ ] Define `AnimProperty<T: Lerp>` (replaces `AnimatedValue<T>`):
  ```rust
  pub struct AnimProperty<T: Lerp> {
      target: T,
      current: T,
      behavior: Option<AnimBehavior>,
      animation: Option<ActiveTransition<T>>,
  }

  impl<T: Lerp> AnimProperty<T> {
      pub fn new(value: T) -> Self { /* no behavior, instant changes */ }
      pub fn with_behavior(value: T, behavior: AnimBehavior) -> Self { /* auto-animate */ }

      /// Set the target value. If a behavior is set, starts an animation.
      /// If no behavior, changes instantly.
      pub fn set(&mut self, value: T) { ... }

      /// Set without animation (even if behavior exists).
      pub fn set_immediate(&mut self, value: T) { ... }

      /// Get the current interpolated value.
      pub fn get(&self, now: Instant) -> T { ... }

      /// Is an animation currently running?
      pub fn is_animating(&self, now: Instant) -> bool { ... }
  }
  ```
- [ ] `AnimProperty` replaces `AnimatedValue` — same concept but with optional `Behavior`.
  Existing `AnimatedValue` already provides: `new(value, duration, easing)`, `set(value, now)`,
  `set_immediate(value)`, `get(now)`, `is_animating(now)`, `target()`. `AnimProperty` adds
  the `Behavior` as optional (None = instant, Some = animated). Existing `AnimatedValue`
  usage migrates to `AnimProperty::with_behavior()`.
- [ ] Implement `Lerp` for additional types needed by new widgets:
  - `Color` (already implemented in `color/mod.rs`)
  - `f32` (already implemented in `animation/mod.rs`)
  - `Point<U>` (already implemented in `animation/mod.rs`)
  - `Size<U>` (already implemented in `animation/mod.rs`)
  - `Rect<U>` (already implemented in `animation/mod.rs`)
  - `Transform2D` (already implemented in `animation/mod.rs`)
  - `Insets` (**not yet implemented** — needs a new `Lerp` impl)

---

## 05.3 Transactions

**File(s):** `oriterm_ui/src/animation/transaction.rs`

SwiftUI-inspired: animation metadata that travels with state changes.

- [ ] Define `Transaction`:
  ```rust
  /// Animation metadata attached to a state change.
  /// When a Transaction is active, all AnimProperty::set() calls
  /// within it use the Transaction's animation curve instead of
  /// the property's default behavior.
  pub struct Transaction {
      pub animation: Option<AnimBehavior>,
  }

  impl Transaction {
      pub fn instant() -> Self { Self { animation: None } }
      pub fn animated(behavior: AnimBehavior) -> Self {
          Self { animation: Some(behavior) }
      }
  }
  ```
- [ ] Thread-local `Transaction` stack (or context parameter):
  ```rust
  pub fn with_transaction<F: FnOnce()>(tx: Transaction, f: F) {
      CURRENT_TRANSACTION.with(|cell| {
          let prev = cell.replace(Some(tx));
          f();
          cell.set(prev);
      });
  }
  ```
- [ ] When `AnimProperty::set()` is called inside a `with_transaction()` block,
  it uses the transaction's animation curve instead of its own behavior.
  If the transaction is `instant()`, no animation regardless of property behavior.
- [ ] This enables: "animate all state changes in this block with spring()" or
  "make all changes instant" without touching individual properties.

---

## 05.4 Spring Physics

**File(s):** `oriterm_ui/src/animation/spring.rs`

First-class spring model for natural-feeling motion.

- [ ] Define `Spring` parameters:
  ```rust
  pub struct Spring {
      /// How quickly the spring responds (lower = faster). Default: 0.55.
      pub response: f32,
      /// Damping ratio. 1.0 = critically damped (no overshoot). Default: 0.825.
      pub damping: f32,
      /// Velocity at which animation is considered complete. Default: 0.001.
      pub epsilon: f32,
  }

  impl Default for Spring {
      fn default() -> Self {
          Self { response: 0.55, damping: 0.825, epsilon: 0.001 }
      }
  }
  ```
- [ ] Implement spring simulation:
  ```rust
  impl Spring {
      /// Given current value, target, velocity, and delta_time,
      /// return (new_value, new_velocity, is_done).
      pub fn step(&self, current: f32, target: f32, velocity: f32, dt: f32)
          -> (f32, f32, bool) { ... }
  }
  ```
- [ ] Handle Spring separately from `Easing` -- do NOT add an `Easing::Spring` variant.
  Springs are velocity-based and stateful (need per-frame `Spring::step()`), while
  `Easing::apply(t) -> f32` is stateless and time-fraction-based. These are
  fundamentally incompatible APIs.

  Instead, introduce an `AnimCurve` enum that wraps both approaches:
  ```rust
  pub enum AnimCurve {
      /// Duration-based easing (stateless, fraction-based).
      Easing { easing: Easing, duration: Duration },
      /// Velocity-based spring (stateful, per-frame step).
      Spring(Spring),
  }
  ```
  Update `AnimBehavior` to hold `AnimCurve` instead of separate `duration` + `easing`
  fields. `AnimProperty::get()` dispatches on `AnimCurve`: for `Easing`, compute
  elapsed fraction and call `easing.apply(t)`; for `Spring`, call `Spring::step()`
  with the stored velocity.

  **Sync point**: `Easing` is in `oriterm_ui/src/animation/mod.rs` (line ~84).
  The existing `Easing` enum and its `apply()` method remain unchanged. `AnimCurve`
  is a new type in `animation/mod.rs` or `animation/behavior.rs`.
- [ ] Spring animations request `anim_frame()` each frame until `is_done`
- [ ] Unit tests: spring converges to target, critically damped (no overshoot with
  damping=1.0), underdamped (overshoot with damping=0.5)

---

## 05.5 Render Scheduling

**File(s):** `oriterm_ui/src/animation/scheduler.rs`

Centralized tracking of which widgets need animation frames and repaints.

- [ ] Define `RenderScheduler`:
  ```rust
  pub struct RenderScheduler {
      /// Widgets that have requested animation frames.
      anim_frame_requests: HashSet<WidgetId>,
      /// Widgets that have requested repaint.
      paint_requests: HashSet<WidgetId>,
      /// Deferred repaint (e.g., cursor blink after 530ms).
      /// Uses `Reverse<>` because `BinaryHeap` is a max-heap; wrapping in
      /// `Reverse` gives min-heap behavior (earliest `wake_at` first).
      deferred_repaints: BinaryHeap<Reverse<DeferredRepaint>>,
  }

  /// Ordered by `wake_at` ascending (implement `Ord` comparing `wake_at`).
  struct DeferredRepaint {
      widget_id: WidgetId,
      wake_at: Instant,
  }
  ```
- [ ] `request_anim_frame(widget_id)`: add to set
- [ ] `request_paint(widget_id)`: add to set
- [ ] `request_repaint_after(widget_id, duration)`: add deferred entry
- [ ] `has_pending_work(&self, now: Instant) -> bool`: true if any requests pending
- [ ] `next_wake_time(&self) -> Option<Instant>`: earliest deferred repaint time
  (feeds into event loop `ControlFlow::WaitUntil`)
- [ ] `drain_anim_frames(&mut self) -> Vec<WidgetId>`: clear and return
- [ ] Integrate with event loop: when `has_pending_work()` is false, use
  `ControlFlow::Wait`. When true, use `ControlFlow::Poll` or `WaitUntil`.
- [ ] **Event loop integration point**: The existing event loop control flow is computed
  by `compute_control_flow()` in `oriterm/src/app/event_loop_helpers/`. This pure
  function determines `ControlFlow` based on cursor blink state and whether animations
  are running. The `RenderScheduler` must feed into this function:
  - `RenderScheduler::has_pending_work()` → `animations_running = true`
  - `RenderScheduler::next_wake_time()` → feeds into `WaitUntil` computation
  The current `ctx.animations_running: &Cell<bool>` pattern in `DrawCtx` is replaced
  by the scheduler. `compute_control_flow()` queries the scheduler instead.
- [ ] **Replacing `animations_running: &Cell<bool>`**: The existing `DrawCtx` field
  `animations_running: &Cell<bool>` is set by widgets during `draw()`. The new model:
  widgets call `ctx.request_anim_frame()` during `anim_frame()` or `paint()` calls.
  The scheduler tracks which widgets have pending requests. The `DrawCtx` field is
  removed in Section 08 when the trait is finalized.

---

## 05.6 Completion Checklist

- [ ] `request_anim_frame()` → `AnimFrame(delta)` pipeline works end-to-end
- [ ] `AnimProperty<T>` replaces `AnimatedValue<T>` with optional `AnimBehavior`
  (`AnimProperty::new()` = instant changes, `AnimProperty::with_behavior()` = auto-animate)
- [ ] Properties with `AnimBehavior` auto-animate on `set()` without widget cooperation
- [ ] `Transaction` allows overriding animation curves for a block of state changes
- [ ] `Spring` physics converges correctly for critically/under/overdamped configurations
- [ ] Spring handled via `AnimCurve::Spring` (separate from `Easing`, not added as an `Easing` variant)
- [ ] `RenderScheduler` correctly tracks animation/paint requests
- [ ] Event loop sleeps when no animations are active
- [ ] `request_repaint_after(Duration)` works for deferred wakeups (cursor blink)
- [ ] Existing `AnimatedValue` usage migrated to `AnimProperty`. Known usages:
  - `ButtonWidget::hover_progress: AnimatedValue<f32>` (button/mod.rs)
  - `ToggleWidget::toggle_progress: AnimatedValue<f32>` (toggle/mod.rs) — thumb slide
  - `WindowControlButton::hover_progress: AnimatedValue<f32>` (window_chrome/controls.rs)
  - `TabBarWidget::hover_progress: Vec<AnimatedValue<f32>>` (tab_bar/widget/mod.rs)
  - `TabBarWidget::close_btn_opacity: Vec<AnimatedValue<f32>>` (tab_bar/widget/mod.rs)
  - `TabBarWidget::width_multipliers: Vec<AnimatedValue<f32>>` (tab_bar/widget/mod.rs)
  - Note: CheckboxWidget, DropdownWidget, and SliderWidget do NOT currently use
    `AnimatedValue` — they will get it through the VisualStateAnimator in Section 06.
  - Note: `AnimatedValue` is also used by `AnimationGroup`, `AnimationSequence`,
    `AnimationDelegate`, and `AnimationBuilder`. These higher-level constructs may
    remain as-is or be updated to use `AnimProperty` — evaluate during implementation.
- [ ] `AnimatedValue<T>` type retained for backward compatibility during migration but
  marked `#[deprecated]`. Remove after all usages are migrated (Section 08).
- [ ] Test files: `oriterm_ui/src/animation/tests.rs` (expand existing), plus
  new test files for new submodules (`behavior/tests.rs`, `spring/tests.rs`,
  `transaction/tests.rs`, `scheduler/tests.rs`)
- [ ] `./test-all.sh` green, `./clippy-all.sh` green, `./build-all.sh` green

**Exit Criteria:** A test widget with `AnimProperty<f32>` and `AnimBehavior::ease_out(150)`
receives AnimFrame events, interpolates smoothly from 0.0 to 1.0 over 150ms, then stops
requesting frames. Event loop returns to sleep after animation completes.
