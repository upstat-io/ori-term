---
section: "05"
title: "Animation Engine"
status: complete
goal: "Unified animation system with AnimFrameEvent timing, AnimProperty/AnimBehavior, transactions, and springs"
inspired_by:
  - "Druid request_anim_frame() / AnimFrame(delta) (druid/src/contexts.rs)"
  - "QML Behavior on property (Qt Quick)"
  - "SwiftUI Transaction-based animation (SwiftUI/Animations)"
  - "egui request_repaint() / request_repaint_after() (egui/src/context.rs)"
depends_on: []
reviewed: true
sections:
  - id: "05.1"
    title: "AnimFrame Integration"
    status: complete
  - id: "05.2"
    title: "Property Behaviors"
    status: complete
  - id: "05.3"
    title: "Transactions"
    status: complete
  - id: "05.4"
    title: "Spring Physics"
    status: complete
  - id: "05.5"
    title: "Render Scheduling"
    status: complete
  - id: "05.6"
    title: "Completion Checklist"
    status: complete
---

# Section 05: Animation Engine

**Status:** In Progress
**Goal:** Widgets request animation frames via `ctx.request_anim_frame()` and receive
`AnimFrameEvent` timing pulses. Properties can have `AnimBehavior` declarations that
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

**Module declaration**: Add `pub mod anim_frame;` to `oriterm_ui/src/animation/mod.rs`.
No change needed in `lib.rs` (the `animation` module is already declared).

Replace the current `animations_running: &Cell<bool>` flag (set per-frame in `DrawCtx`) with
explicit animation frame requests.

**Rendering discipline note**: Setting `anim_frame_requested = true` during `draw()` is an
output flag on the context struct, not a mutation of Grid/Tab/App state. The existing
`animations_running: &Cell<bool>` pattern in `DrawCtx` is the same category: a signal
FROM the widget TO the framework. The exception is limited to boolean output flags on
context structs.

- [x] Add `request_anim_frame()` and `request_paint()` methods to `EventCtx`, `DrawCtx`,
  and `ControllerCtx`. This replaces the current `ctx.animations_running.set(true)`
  pattern in `DrawCtx`.
  `EventCtx` and `DrawCtx` are defined in `oriterm_ui/src/widgets/mod.rs`;
  `ControllerCtx` is defined in `oriterm_ui/src/controllers/mod.rs`:
  ```rust
  /// Request an animation frame on the next vsync. The widget will receive
  /// an AnimFrame event with the time delta since the last frame.
  pub fn request_anim_frame(&mut self) {
      self.anim_frame_requested = true;
  }

  /// Request a repaint without an animation frame.
  pub fn request_paint(&mut self) {
      self.paint_requested = true;
  }
  ```
  **Deviation:** Instead of separate `bool` fields, implemented via shared
  `Option<&'a FrameRequestFlags>` on both contexts. `FrameRequestFlags` holds
  two `Cell<bool>` fields. Shared reference avoids per-container merge logic and
  matches the existing `animations_running: &Cell<bool>` pattern. Methods are
  `&self` (not `&mut self`) since `Cell::set` works through shared refs.
- [x] **Wire flag propagation from context structs to `RenderScheduler`** (05.5). The
  `anim_frame_requested` and `paint_requested` fields on
  contexts are *output flags*, not inputs. The framework reads them after calling the
  widget method and feeds them into the `RenderScheduler` (Section 05.5). The plumbing:

  **For `DrawCtx`:** After `widget.draw(ctx)` returns, the framework checks
  `ctx.anim_frame_requested` and `ctx.paint_requested`. If set, it calls
  `scheduler.request_anim_frame(widget_id)` / `scheduler.request_paint(widget_id)`.
  The `for_child()` method must propagate child flags upward: after the child draw
  returns, merge `child_ctx.anim_frame_requested` into `parent_ctx.anim_frame_requested`
  (bitwise OR). Alternatively, pass a `&RenderScheduler` reference into contexts so
  widgets write directly to the scheduler — but this couples contexts to the scheduler.
  The flag-merge approach keeps contexts lightweight and scheduler-agnostic.

  **For `EventCtx`:** Same pattern. After `widget.handle_mouse()` / `handle_key()`
  returns, the framework reads flags and feeds the scheduler.

  **For `ControllerCtx`:** Already solved — `ControllerRequests::ANIM_FRAME` and
  `ControllerRequests::PAINT` exist in the bitmask (Section 04). The dispatch function
  `dispatch_to_controllers()` returns `DispatchOutput` with accumulated `requests`.
  The framework reads `requests.contains(ANIM_FRAME)` and feeds the scheduler. No
  new work needed for `ControllerCtx` — this is already wired.

- [x] Define `AnimFrameEvent`:
  ```rust
  /// Timing pulse delivered to widgets that requested an animation frame.
  #[derive(Debug, Clone, Copy)]
  pub struct AnimFrameEvent {
      /// Nanoseconds since the last AnimFrame delivered to this widget.
      /// 0 on the first frame after transitioning from idle to animating.
      pub delta_nanos: u64,
      /// Absolute timestamp for this frame.
      pub now: Instant,
  }
  ```
- [x] **Do NOT add `Widget::anim_frame()` in this section.** The `AnimFrameEvent` type
  is defined here (Section 05) but the `Widget::anim_frame()` trait method and its
  `AnimCtx` context are added in Section 08. This section provides the building blocks;
  Section 08 wires them into the widget trait. During Section 05 testing, validate
  `AnimFrameEvent`, `AnimProperty`, `Spring`, and `RenderScheduler` via unit tests
  without requiring a Widget trait change.
- [x] **Verify the per-frame animation lifecycle** (documented here, implemented via
  `RenderScheduler` in 05.5 and `Widget::anim_frame()` in Section 08):
  1. `RenderScheduler` tracks which widgets requested anim frames.
  2. On next frame, deliver `AnimFrameEvent` to all requesting widgets.
  3. Widget processes animation, calls `request_paint()` if visual change.
  4. Widget calls `request_anim_frame()` again if animation is still running.
  5. If no widgets request another frame, event loop returns to sleep.
- [x] **File size**: `anim_frame.rs` is a flat file containing `AnimFrameEvent` and
  `FrameRequestFlags` (~83 lines). Well within the 500-line limit.

---

## 05.2 Property Behaviors

**File(s):**
- `oriterm_ui/src/animation/behavior/mod.rs` (new file, ~150 lines — AnimBehavior, AnimCurve)
- `oriterm_ui/src/animation/behavior/tests.rs` (new file — sibling tests)
- `oriterm_ui/src/animation/property/mod.rs` (new file, ~200 lines — AnimProperty, ActiveTransition)
- `oriterm_ui/src/animation/property/tests.rs` (new file — sibling tests)

**Module declarations**: Add `pub mod behavior;` and `pub mod property;` to
`oriterm_ui/src/animation/mod.rs`. Add `#[cfg(test)] mod tests;` at the bottom
of both `behavior/mod.rs` and `property/mod.rs`.

QML-inspired: declare "this property animates when changed."

- [x] Define `AnimBehavior`:
  ```rust
  /// Declares how a property transitions when its target value changes.
  #[derive(Debug, Clone, Copy)]
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
- [x] Define `ActiveTransition<T>` — the in-flight animation state stored by `AnimProperty`.
  Both easing and spring produce a scalar progress value (0.0 to 1.0), which is fed
  to `Lerp` to compute the actual value. The velocity tracks the rate of change of
  this normalized progress, not the value's coordinate space. This means a single
  `f32` velocity works for any `Lerp` type (Point, Size, Rect, etc.):
  ```rust
  /// In-flight transition state for an AnimProperty.
  #[derive(Debug, Clone, Copy)]
  struct ActiveTransition<T: Lerp> {
      /// Value at the start of the transition.
      from: T,
      /// Value at the end of the transition (same as AnimProperty::target).
      to: T,
      /// When the transition started.
      start: Instant,
      /// Current progress through the transition (0.0 = from, 1.0 = to).
      /// For easing: computed lazily from elapsed time.
      /// For springs: advanced by tick() each frame.
      progress: f32,
      /// For spring-based transitions: velocity of the progress value.
      /// Tracks rate of change of the normalized progress (0.0 to 1.0),
      /// NOT velocity in the value's coordinate space. This means a single
      /// f32 velocity works for any Lerp type (Point, Size, Rect, etc.)
      /// because the spring operates on the scalar progress dimension.
      /// Unused for easing-based transitions (easing computes progress
      /// from elapsed time).
      velocity: f32,
  }
  ```
  **Spring statefulness**: Spring animations are stateful — they need to store
  velocity between frames. `ActiveTransition` stores `velocity: f32` which is
  updated each time `tick()` is called. The spring operates on the `progress`
  field (0.0 to 1.0), not on the raw value. `T::lerp(from, to, progress)` maps
  progress to the actual interpolated value. This means spring-based `AnimProperty`
  requires a `tick(&mut self, now: Instant)` call each frame (during `anim_frame()`),
  NOT lazy evaluation in `get()`. The `get()` method returns
  `T::lerp(from, to, progress)` without advancing the simulation.
- [x] Define `AnimProperty<T: Lerp>` in `property.rs`:
  ```rust
  /// A value that optionally transitions smoothly when changed.
  ///
  /// Replaces `AnimatedValue<T>`. When `behavior` is `None`, changes are instant.
  /// When `behavior` is `Some`, `set()` starts a transition using the behavior's curve.
  #[derive(Debug, Clone)]
  pub struct AnimProperty<T: Lerp> {
      /// The target (resting) value.
      target: T,
      /// The current interpolated value (updated by `tick()` for springs,
      /// computed lazily for easing-based transitions).
      current: T,
      /// Optional animation behavior — None means instant changes.
      behavior: Option<AnimBehavior>,
      /// In-flight transition, if any.
      transition: Option<ActiveTransition<T>>,
  }

  impl<T: Lerp> AnimProperty<T> {
      /// Creates an instantly-changing property (no animation).
      pub fn new(value: T) -> Self { /* behavior: None */ }

      /// Creates a property with auto-animation on `set()`.
      pub fn with_behavior(value: T, behavior: AnimBehavior) -> Self { /* auto-animate */ }

      /// Set the target value. If a behavior is set (and no Transaction overrides
      /// it to instant), starts an animation from the current interpolated value.
      /// If no behavior (or Transaction::instant()), changes instantly.
      /// Requires `now` to compute the current interpolated value for smooth
      /// interruption (starting the new animation from mid-flight).
      pub fn set(&mut self, value: T, now: Instant) { ... }

      /// Set without animation (even if behavior exists).
      pub fn set_immediate(&mut self, value: T) { ... }

      /// Get the current interpolated value.
      ///
      /// For easing-based transitions: computes the value from elapsed time.
      /// For spring-based transitions: returns the last value computed by `tick()`.
      pub fn get(&self, now: Instant) -> T { ... }

      /// Advance spring-based transitions by one frame.
      ///
      /// Must be called each frame for spring animations (during `anim_frame()`).
      /// No-op for easing-based transitions (those are computed lazily in `get()`).
      /// No-op if no transition is active.
      pub fn tick(&mut self, now: Instant) { ... }

      /// Is an animation currently running?
      pub fn is_animating(&self, now: Instant) -> bool { ... }

      /// Returns the final resting value.
      pub fn target(&self) -> T { ... }
  }
  ```
- [x] `AnimProperty` replaces `AnimatedValue` — same concept but with optional `Behavior`.
  Existing `AnimatedValue` already provides: `new(value, duration, easing)`, `set(value, now)`,
  `set_immediate(value)`, `get(now)`, `is_animating(now)`, `target()`. `AnimProperty` adds
  the `Behavior` as optional (None = instant, Some = animated). Existing `AnimatedValue`
  usage migrates to `AnimProperty::with_behavior()`.
- [x] Implement `Lerp` for additional types needed by new widgets:
  - `Color` (already implemented in `color/mod.rs`)
  - `f32` (already implemented in `animation/mod.rs`)
  - `Point<U>` (already implemented in `animation/mod.rs`)
  - `Size<U>` (already implemented in `animation/mod.rs`)
  - `Rect<U>` (already implemented in `animation/mod.rs`)
  - `Transform2D` (already implemented in `animation/mod.rs`)
  - `Insets` (**not yet implemented** — add to `animation/mod.rs`, per-field lerp:
    `top`, `right`, `bottom`, `left`)

---

## 05.3 Transactions

**File(s):**
- `oriterm_ui/src/animation/transaction/mod.rs` (new file, ~100 lines)
- `oriterm_ui/src/animation/transaction/tests.rs` (new file — sibling tests)

**Module declaration**: Add `pub mod transaction;` to `oriterm_ui/src/animation/mod.rs`.
Add `#[cfg(test)] mod tests;` at the bottom of `transaction/mod.rs`.

SwiftUI-inspired: animation metadata that travels with state changes.

- [x] Define `Transaction`:
  ```rust
  /// Animation metadata attached to a state change.
  /// When a Transaction is active, all AnimProperty::set() calls
  /// within it use the Transaction's animation curve instead of
  /// the property's default behavior.
  #[derive(Debug, Clone, Copy)]
  pub struct Transaction {
      pub animation: Option<AnimBehavior>,
  }

  impl Transaction {
      /// No animation — changes are instant regardless of property behavior.
      pub fn instant() -> Self { Self { animation: None } }
      /// Override with a specific animation curve.
      pub fn animated(behavior: AnimBehavior) -> Self {
          Self { animation: Some(behavior) }
      }
  }
  ```
- [x] Implement thread-local `Transaction` stack using `Cell` (`Transaction` is `Copy`):
  ```rust
  use std::cell::Cell;

  thread_local! {
      static CURRENT_TRANSACTION: Cell<Option<Transaction>> = const { Cell::new(None) };
  }

  /// Execute `f` with the given transaction active. All `AnimProperty::set()`
  /// calls within `f` use the transaction's animation curve instead of each
  /// property's default behavior.
  ///
  /// Transactions nest: the inner transaction overrides the outer one.
  /// The previous transaction is restored after `f` returns (including on panic,
  /// via a drop guard).
  pub fn with_transaction<F, R>(tx: Transaction, f: F) -> R
  where
      F: FnOnce() -> R,
  {
      CURRENT_TRANSACTION.with(|cell| {
          let prev = cell.get();
          cell.set(Some(tx));
          // Drop guard restores `prev` even if `f()` panics.
          struct RestoreGuard<'a> {
              cell: &'a Cell<Option<Transaction>>,
              prev: Option<Transaction>,
          }
          impl Drop for RestoreGuard<'_> {
              fn drop(&mut self) {
                  self.cell.set(self.prev);
              }
          }
          let guard = RestoreGuard { cell, prev };
          let result = f();
          drop(guard);
          result
      })
  }

  /// Read the current transaction (called by AnimProperty::set()).
  pub(crate) fn current_transaction() -> Option<Transaction> {
      CURRENT_TRANSACTION.with(Cell::get)
  }
  ```
- [x] **Verify threading model compatibility.** Thread-local storage is compatible with the single-threaded
  event loop architecture. The winit event loop, widget tree traversal, and all
  `AnimProperty::set()` calls happen on the main thread. Background threads (PTY
  reader, font rasterizer) never call `AnimProperty::set()`. If the architecture
  ever moves to multi-threaded widget updates, `thread_local!` must be replaced
  with a context-parameter approach (passing `&Transaction` through contexts).
  For now, thread-local is correct and matches SwiftUI's design.

  **Testing hazard**: Thread-local state persists across tests in the same thread.
  If a test calls `with_transaction()` and panics before the guard restores the
  previous transaction, subsequent tests on that thread will see stale transaction
  state. Mitigation: (1) the `RestoreGuard` drop pattern handles panics, and
  (2) tests that directly test `current_transaction()` should explicitly call
  `with_transaction(Transaction::instant(), || { ... })` to reset state. No
  test should rely on `current_transaction()` being `None` at test entry without
  explicitly ensuring it.
- [x] When `AnimProperty::set()` is called inside a `with_transaction()` block,
  it checks `current_transaction()`. If a transaction is active:
  - `Transaction::animated(behavior)` → use the transaction's `behavior` instead
    of the property's own `behavior`.
  - `Transaction::instant()` (animation is `None`) → set immediately with no
    animation, regardless of the property's behavior.
  If no transaction is active (`current_transaction()` returns `None`), the property
  uses its own behavior as usual.
  This enables "animate all state changes in this block with spring()" or
  "make all changes instant" without touching individual properties.
- [x] **Edge case**: `with_transaction(Transaction::instant(), ...)` during initial
  widget construction ensures first-paint values are set without animation, even
  if properties have behaviors. This prevents "animate from zero" on widget creation.

---

## 05.4 Spring Physics

**File(s):** `oriterm_ui/src/animation/spring/mod.rs` (new file, ~120 lines),
`oriterm_ui/src/animation/spring/tests.rs` (new file — sibling tests)

**Module declaration**: Add `pub mod spring;` to `oriterm_ui/src/animation/mod.rs`.
Add `#[cfg(test)] mod tests;` at the bottom of `spring/mod.rs`.

First-class spring model for natural-feeling motion.

- [x] Define `Spring` parameters:
  ```rust
  /// Damped harmonic oscillator parameters for spring animations.
  ///
  /// Uses the second-order system model: converts `response` and `damping`
  /// to angular frequency (omega) and damping coefficient for the ODE solver.
  #[derive(Debug, Clone, Copy, PartialEq)]
  pub struct Spring {
      /// How quickly the spring responds (lower = faster). Default: 0.55.
      /// Corresponds to the period of oscillation in seconds.
      pub response: f32,
      /// Damping ratio. 1.0 = critically damped (no overshoot). Default: 0.825.
      /// < 1.0 = underdamped (overshoot), > 1.0 = overdamped (slow approach).
      pub damping: f32,
      /// Velocity threshold at which animation is considered complete. Default: 0.001.
      /// When `|velocity| < epsilon` AND `|current - target| < epsilon`, done.
      pub epsilon: f32,
  }

  impl Default for Spring {
      fn default() -> Self {
          Self { response: 0.55, damping: 0.825, epsilon: 0.001 }
      }
  }
  ```
- [x] Implement spring simulation:
  ```rust
  impl Spring {
      /// Given current value, target, velocity, and delta_time,
      /// return (new_value, new_velocity, is_done).
      ///
      /// Uses semi-implicit Euler integration of the damped harmonic oscillator:
      ///   omega = 2 * PI / response
      ///   acceleration = omega^2 * (target - current) - 2 * damping * omega * velocity
      ///   velocity' = velocity + acceleration * dt
      ///   current' = current + velocity' * dt
      ///   is_done = |velocity'| < epsilon && |current' - target| < epsilon
      pub fn step(&self, current: f32, target: f32, velocity: f32, dt: f32)
          -> (f32, f32, bool) { ... }
  }
  ```
- [x] **Stability guard in `Spring::step()`**: Clamp `dt` to a maximum of `1/30` seconds (33ms). If a frame
  takes longer (e.g., due to stutter), large `dt` values can cause the spring to
  overshoot wildly or diverge. Clamping ensures stability at the cost of slightly
  slower animation during frame drops.
- [x] Handle Spring separately from `Easing` -- do NOT add an `Easing::Spring` variant.
  Springs are velocity-based and stateful (need per-frame `Spring::step()`), while
  `Easing::apply(t) -> f32` is stateless and time-fraction-based. These are
  fundamentally incompatible APIs.

  Instead, introduce an `AnimCurve` enum that wraps both approaches:
  ```rust
  /// Unifies duration-based easing and velocity-based springs.
  #[derive(Debug, Clone, Copy)]
  pub enum AnimCurve {
      /// Duration-based easing (stateless, fraction-based).
      Easing { easing: Easing, duration: Duration },
      /// Velocity-based spring (stateful, per-frame step).
      Spring(Spring),
  }
  ```
  Update `AnimBehavior` to hold `AnimCurve` instead of separate `duration` + `easing`
  fields. For `AnimProperty` dispatch:
  - **Easing**: `get(now)` computes elapsed fraction and calls `easing.apply(t)`.
    Purely time-based, no state mutation needed.
  - **Spring**: `tick(now)` calls `Spring::step()` with stored velocity, updates
    `current` and `velocity` in `ActiveTransition`. `get(now)` returns the last
    computed `current` value without advancing. This is why `tick()` is required
    for spring animations (see 05.2).

  **Sync point**: `Easing` is in `oriterm_ui/src/animation/mod.rs` (line ~84).
  The existing `Easing` enum and its `apply()` method remain unchanged. `AnimCurve`
  lives in `animation/behavior.rs` alongside `AnimBehavior`.
- [x] `AnimProperty::tick()` with a spring-based `AnimBehavior` must call
  `Spring::step()` each frame and continue requesting animation frames until
  `is_done` returns true. When done, clear `ActiveTransition` to stop animation.
- [x] Unit tests in `oriterm_ui/src/animation/spring/tests.rs`:
  - [x] `spring_converges_to_target` — after enough steps, value reaches target within epsilon
  - [x] `spring_critically_damped_no_overshoot` — damping=1.0, value never exceeds target
  - [x] `spring_underdamped_overshoots` — damping=0.5, value exceeds target at some point
  - [x] `spring_overdamped_no_overshoot` — damping=2.0, value approaches slowly without overshoot
  - [x] `spring_zero_dt_no_change` — step with dt=0.0 returns same value and velocity
  - [x] `spring_large_dt_clamped` — step with dt=1.0 is clamped, doesn't diverge
  - [x] `spring_at_rest_is_done` — when current==target and velocity==0, reports done
  - [x] `spring_default_parameters_reasonable` — default Spring produces smooth motion
  - [x] `spring_negative_velocity_direction` — spring from above target converges correctly

---

## 05.5 Render Scheduling

**File(s):** `oriterm_ui/src/animation/scheduler/mod.rs` (new file, ~180 lines),
`oriterm_ui/src/animation/scheduler/tests.rs` (new file — sibling tests)

**Module declaration**: Add `pub mod scheduler;` to `oriterm_ui/src/animation/mod.rs`.
Add `#[cfg(test)] mod tests;` at the bottom of `scheduler/mod.rs`.

Centralized tracking of which widgets need animation frames and repaints.

- [x] Define `DeferredRepaint` with manual `Ord` (comparing only `wake_at`, required
  by `BinaryHeap`):
  ```rust
  /// A deferred repaint request, ordered by wake time.
  /// WidgetId is NOT included in the ordering — only `wake_at` matters
  /// for the min-heap. Ties are broken arbitrarily.
  #[derive(Debug, Clone, Copy, Eq, PartialEq)]
  struct DeferredRepaint {
      widget_id: WidgetId,
      wake_at: Instant,
  }

  impl Ord for DeferredRepaint {
      fn cmp(&self, other: &Self) -> std::cmp::Ordering {
          self.wake_at.cmp(&other.wake_at)
      }
  }

  impl PartialOrd for DeferredRepaint {
      fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
          Some(self.cmp(other))
      }
  }
  ```
  **Note:** `WidgetId` does not derive `Eq`/`Ord` — only `PartialEq`, `Eq`, `Hash`,
  and `Copy`. The `Ord` impl on `DeferredRepaint` compares only `wake_at` (which is
  `Ord`), so `WidgetId` does not need `Ord`.
- [x] Define `RenderScheduler`:
  ```rust
  /// Centralized tracking of animation frame and repaint requests.
  ///
  /// Owned by the application layer (one per window context). Widgets
  /// signal requests via context flags; the framework reads those flags
  /// after each widget call and forwards them to the scheduler.
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
  ```
- [x] **Ownership**: `RenderScheduler` lives on the per-window context in the
  `oriterm` crate (alongside `layer_animator`, `overlays`, etc.). It is NOT
  in `oriterm_ui` — it is a concrete type defined in `oriterm_ui` but owned
  and driven by the application layer. The `oriterm_ui` crate defines the struct;
  `oriterm/src/app/` owns instances. One scheduler per window context.
  Constructor: `RenderScheduler::new()` with empty sets and empty heap.
- [x] `request_anim_frame(widget_id)`: add to set
- [x] `request_paint(widget_id)`: add to set
- [x] `request_repaint_after(widget_id, duration, now)`: add deferred entry
  (`wake_at = now + duration`)
- [x] `has_pending_work(&self, now: Instant) -> bool`: true if any anim_frame requests
  OR any paint requests OR any deferred repaints with `wake_at <= now`
- [x] `next_wake_time(&self) -> Option<Instant>`: earliest deferred repaint time
  (feeds into event loop `ControlFlow::WaitUntil`)
- [x] `take_anim_frames(&mut self) -> HashSet<WidgetId>`: move the set out
  via `std::mem::take()` (zero-alloc if the set was empty; reuses existing
  allocation on the caller side). The scheduler's field becomes an empty
  `HashSet` with zero capacity; next `request_anim_frame()` may allocate,
  but the idle path does not. Alternatively, accept a `&mut Vec<WidgetId>`
  scratch buffer owned by the caller (event loop), clear + extend from the
  set, then clear the set. Either approach avoids per-frame allocation.
- [x] `take_paint_requests(&mut self) -> HashSet<WidgetId>`: same pattern
  as `take_anim_frames()`. Uses `std::mem::take()` to avoid allocation.
- [x] `promote_deferred(&mut self, now: Instant)`: move deferred repaints with
  `wake_at <= now` into `paint_requests`. Called at the start of each frame
  before draining.
- [x] `remove_widget(&mut self, widget_id: WidgetId)`: remove all pending requests
  for a widget (called on widget removal / deregistration).
  **Implementation note**: Use lazy removal for `deferred_repaints` — do not
  rebuild the heap. Instead, `promote_deferred()` skips entries whose
  `widget_id` is no longer in the active widget set. This avoids O(n) heap
  rebuild on widget removal. The `remove_widget()` method only removes from
  `anim_frame_requests` and `paint_requests` (both O(1) HashSet remove).
- [x] Integrate with event loop: when `has_pending_work()` is false, use
  `ControlFlow::Wait`. When true, use `ControlFlow::Poll` or `WaitUntil`.
- [x] **Event loop integration point**: The existing event loop control flow is computed
  by `compute_control_flow()` in `oriterm/src/app/event_loop_helpers/`. This pure
  function determines `ControlFlow` based on cursor blink state and whether animations
  are running. The `RenderScheduler` must feed into this function:
  - `RenderScheduler::has_pending_work()` → `has_animations = true` in
    `ControlFlowInput`. This causes `WaitUntil(now + 16ms)` — a vsync-aligned
    wakeup for the next animation frame. (The current `has_animations` field is
    fed by `layer_animator.is_any_animating()`. The scheduler adds a second
    source: `has_animations = layer_animator.is_any_animating() || scheduler.has_pending_work(now)`.)
  - `RenderScheduler::next_wake_time()` → feeds into `WaitUntil` computation
    (compared against other wake sources like cursor blink, choosing the earliest).
  - `compute_control_flow()` gains a new input: **`scheduler_wake: Option<Instant>`**
    on `ControlFlowInput`. This feeds `RenderScheduler::next_wake_time()` into the
    control flow decision. The function picks `min(next_toggle, scheduler_wake)` for
    `WaitUntil` when neither `has_animations` nor `any_dirty` are active.
  - **Existing tests must be updated**: The tests in
    `oriterm/src/app/event_loop_helpers/tests.rs` construct `ControlFlowInput`
    directly. Adding `scheduler_wake` requires updating every test's struct literal
    (set to `None` for existing tests, add new tests for `Some(instant)` cases).
    The `idle_input()` helper gets `scheduler_wake: None`.
  - The caller in `event_loop.rs` computes `scheduler_wake` from
    `scheduler.next_wake_time()` when building `ControlFlowInput`.
- [x] **Replacing `animations_running: &Cell<bool>`**: The existing `DrawCtx` field
  `animations_running: &Cell<bool>` is set by widgets during `draw()`. The new model:
  widgets call `ctx.request_anim_frame()` during `anim_frame()` or `draw()`/`paint()`.
  The framework reads the flag after the call returns and forwards to the scheduler.
  The `DrawCtx` field is removed in Section 08 when the trait is finalized.
  **During Section 05 (before Section 08)**: Both mechanisms coexist. The scheduler
  is available but the old `animations_running` Cell still works. This allows
  incremental migration — widgets can be migrated one at a time. The event loop
  consults both: `has_animations = layer_animator.is_any_animating() || scheduler.has_pending_work(now)`
  and separately: `if animations_running.get() { ctx.dirty = true; }` (unchanged).
- [x] Unit tests in `oriterm_ui/src/animation/scheduler/tests.rs`:
  - [x] `scheduler_empty_has_no_pending_work`
  - [x] `scheduler_anim_frame_request_has_pending_work`
  - [x] `scheduler_paint_request_has_pending_work`
  - [x] `scheduler_take_anim_frames_clears_set`
  - [x] `scheduler_deferred_repaint_before_wake_time_not_pending`
  - [x] `scheduler_deferred_repaint_after_wake_time_is_pending`
  - [x] `scheduler_next_wake_time_returns_earliest`
  - [x] `scheduler_promote_deferred_moves_to_paint`
  - [x] `scheduler_remove_widget_clears_all_requests`
  - [x] `scheduler_multiple_deferred_ordered_correctly`

---

## 05.6 Completion Checklist

### Core functionality
- [x] `AnimFrameEvent` type defined and exported
- [x] `request_anim_frame()` and `request_paint()` output flags on `EventCtx` and `DrawCtx`
- [x] `ControllerRequests::ANIM_FRAME` and `PAINT` already exist (Section 04 — no new work)
- [x] `AnimProperty<T>` replaces `AnimatedValue<T>` with optional `AnimBehavior`
  (`AnimProperty::new()` = instant changes, `AnimProperty::with_behavior()` = auto-animate)
- [x] Properties with `AnimBehavior` auto-animate on `set()` without widget cooperation
- [x] `AnimProperty::tick()` advances spring-based transitions each frame
- [x] `Transaction` allows overriding animation curves for a block of state changes
- [x] `Spring` physics converges correctly for critically/under/overdamped configurations
- [x] Spring handled via `AnimCurve::Spring` (separate from `Easing`, not added as an `Easing` variant)
- [x] `RenderScheduler` correctly tracks animation/paint requests
- [x] Event loop sleeps when no animations are active
- [x] `request_repaint_after(Duration)` works for deferred wakeups (cursor blink)

### Module declarations
- [x] `oriterm_ui/src/animation/mod.rs` declares: `pub mod anim_frame;`, `pub mod behavior;`,
  `pub mod property;`, `pub mod spring;`, `pub mod transaction;`, `pub mod scheduler;`
- [x] Each directory module's `mod.rs` ends with `#[cfg(test)] mod tests;`
  (behavior, property, spring, transaction, scheduler)
- [x] `anim_frame.rs` is a flat file (no tests needed — only type definitions)
- [x] Re-exports in `animation/mod.rs`: `pub use` for `AnimFrameEvent`, `AnimBehavior`,
  `AnimCurve`, `AnimProperty`, `Spring`, `Transaction`, `with_transaction`,
  `RenderScheduler` — so consumers can `use oriterm_ui::animation::AnimProperty`

### File size compliance
- [x] `animation/mod.rs` stays under 500 lines (currently 356 — adding re-exports and
  `Lerp for Insets` brings it to ~380. If it exceeds 500, extract `Lerp` impls into
  `animation/lerp_impls.rs`)
- [x] Each new `mod.rs` file stays under 500 lines:
  - `anim_frame.rs` (~83 lines — flat file with FrameRequestFlags, no tests)
  - `behavior/mod.rs` (~95 lines — AnimBehavior, AnimCurve)
  - `property/mod.rs` (~220 lines — AnimProperty, ActiveTransition)
  - `spring/mod.rs` (~87 lines — Spring)
  - `transaction/mod.rs` (~83 lines — Transaction, with_transaction, RestoreGuard)
  - `scheduler/mod.rs` (~165 lines — RenderScheduler, DeferredRepaint)

### Derive traits
- [x] `AnimBehavior`: `Debug, Clone, Copy` (only field is `AnimCurve` which is `Copy`)
- [x] `AnimCurve`: `Debug, Clone, Copy` (all variants contain only `Copy` fields:
  `Easing` + `Duration` for the easing variant, `Spring` for the spring variant)
- [x] `AnimProperty<T>`: `Debug, Clone` (where `T: Lerp + Debug`; `Clone` is implied
  by `Lerp: Copy`)
- [x] `ActiveTransition<T>`: `Debug, Clone, Copy` (all fields are `Copy`)
- [x] `Spring`: `Debug, Clone, Copy, PartialEq`
- [x] `AnimFrameEvent`: `Debug, Clone, Copy`
- [x] `Transaction`: `Debug, Clone, Copy`
- [x] `RenderScheduler`: `Debug` (not `Clone` — heap is not cheaply cloneable)
- [x] `DeferredRepaint`: `Debug, Clone, Copy, Eq, PartialEq` + manual `Ord`/`PartialOrd`

### Migration (deferred to Section 08)
- [x] Existing `AnimatedValue` usage migrated to `AnimProperty`. Known usages:
  - `ButtonWidget` — migrated to `VisualStateAnimator` (no `AnimatedValue`)
  - `ToggleWidget::toggle_progress: AnimProperty<f32>` — migrated from `AnimatedValue`
  - `WindowControlButton` — migrated to `VisualStateAnimator` (no `AnimatedValue`)
  - `TabBarWidget::hover_progress: Vec<AnimProperty<f32>>` — migrated
  - `TabBarWidget::close_btn_opacity: Vec<AnimProperty<f32>>` — migrated
  - `TabBarWidget::width_multipliers: Vec<AnimProperty<f32>>` — migrated
- [x] `AnimatedValue<T>` type retained for backward compatibility during migration but
  marked `#[deprecated]`. Tests in `animation/tests.rs` still exercise it.

### Test files and test cases
- [x] `oriterm_ui/src/animation/tests.rs` — expand with `Lerp for Insets` tests:
  - [x] `lerp_insets_at_boundaries`
  - [x] `lerp_insets_at_midpoint`
- [x] `oriterm_ui/src/animation/behavior/tests.rs` — tests for `AnimBehavior` and `AnimCurve`:
  - [x] `anim_behavior_ease_out_creates_easing_curve`
  - [x] `anim_behavior_spring_creates_spring_curve`
  - [x] `anim_curve_easing_debug_format`
- [x] `oriterm_ui/src/animation/property/tests.rs` — tests for `AnimProperty`:
  - [x] `anim_property_new_has_no_behavior`
  - [x] `anim_property_with_behavior_animates_on_set`
  - [x] `anim_property_new_set_is_instant`
  - [x] `anim_property_set_immediate_bypasses_behavior`
  - [x] `anim_property_get_returns_interpolated_value`
  - [x] `anim_property_is_animating_during_transition`
  - [x] `anim_property_not_animating_after_completion`
  - [x] `anim_property_smooth_interruption` — set during active animation starts from current pos
  - [x] `anim_property_target_returns_final_value`
  - [x] `anim_property_tick_advances_spring`
  - [x] `anim_property_tick_noop_for_easing`
- [x] `oriterm_ui/src/animation/transaction/tests.rs` — tests for `Transaction`:
  - [x] `transaction_instant_overrides_behavior`
  - [x] `transaction_animated_overrides_behavior`
  - [x] `transaction_nesting_inner_overrides_outer`
  - [x] `transaction_no_transaction_uses_property_behavior`
  - [x] `transaction_panic_restores_previous` — verify RestoreGuard on panic
- [x] `oriterm_ui/src/animation/spring/tests.rs` — tests for `Spring` (listed in 05.4)
- [x] `oriterm_ui/src/animation/scheduler/tests.rs` — tests for `RenderScheduler` (listed in 05.5)
- [x] `oriterm/src/app/event_loop_helpers/tests.rs` — update existing tests for new
  `scheduler_wake` field, add new tests for scheduler-driven wake
- [x] `./test-all.sh` green, `./clippy-all.sh` green, `./build-all.sh` green

**Exit Criteria:**
1. Unit test: `AnimProperty<f32>` with `AnimBehavior::ease_out(150)` — `set(1.0, now)`,
   `get(now + 75ms)` returns ~0.5 (eased), `get(now + 150ms)` returns 1.0,
   `is_animating(now + 150ms)` returns false.
2. Unit test: `AnimProperty<f32>` with `AnimBehavior::spring()` — `set(1.0, now)`, call
   `tick()` repeatedly with 16ms steps, value converges to 1.0 within epsilon.
3. Unit test: `RenderScheduler` — `request_anim_frame(id)` makes `has_pending_work()` true,
   `take_anim_frames()` returns the id, subsequent `has_pending_work()` returns false.
4. Unit test: `with_transaction(Transaction::instant(), ...)` — `AnimProperty::set()` inside
   the block sets immediately despite having a behavior.
5. All builds, clippy, and tests pass.
