# GUI Framework Architecture Research

Research into event systems, animation systems, hover/focus handling, and state management
across 8 major GUI frameworks. Focus: architectural patterns for ori_term's UI layer.

---

## 1. SwiftUI (Apple, Declarative)

### Event System

**No bubbling.** SwiftUI does not bubble events like UIKit. Only one view ultimately receives
a given event. Hit testing uses the **layout frame**, not the visual shape -- invisible views
(opacity=0) can still receive input events.

**Gesture competition with priority modifiers.** Multiple gestures can observe the same touch,
but only one wins. The deepest child gesture wins by default, but this can be overridden:
- `.gesture()` -- standard priority, child wins
- `.highPriorityGesture()` -- overrides child precedence
- `.simultaneousGesture()` -- both parent and child fire

Gestures exist only where the view is hit-tested. No size = no gesture.

**No mouse capture concept** in the web/UIKit sense. Drag gestures receive all movement once
recognized through the gesture system. The gesture recognizer owns the sequence.

### Animation System

**Transaction-based.** Every state change carries a Transaction containing the animation curve.
`withAnimation(.spring()) { state.toggle() }` is syntactic sugar for
`withTransaction(Transaction(animation: .spring())) { state.toggle() }`.

**Implicit vs explicit:**
- Implicit: `.animation(.easeInOut)` modifies the Transaction for the current subtree
- Explicit: `withAnimation {}` sets the Transaction from the root, affecting all views that
  depend on the changed state

**How it works internally:** When state changes, SwiftUI takes a before-snapshot and
after-snapshot of the view tree, then interpolates between them using the Transaction's
animation curve. Animatable components (conforming to `Animatable` protocol) extract the
curve from the Transaction and compute interpolated values each frame.

**matchedGeometryEffect:** Assigns the same ID to views in different states. During transition,
SwiftUI interpolates between their frames, creating "hero animation" illusions where one view
morphs into another -- even when they're actually separate views being shown/hidden.

**Spring physics:** First-class. `.spring(response:dampingFraction:blendDuration:)` is the
recommended default animation. SwiftUI's spring model is critically-damped by default for
natural-feeling motion.

### Hover/Focus System

**Hover:** `.onHover { isHovering in ... }` per-view modifier. Platform-specific (macOS/iPadOS
only). Known reliability issue: on macOS, `.onHover` closure is NOT always called on mouse
exit with high cursor velocity. Internally backed by NSView enter/exit tracking.

`.onContinuousHover(coordinateSpace:perform:)` provides precise pointer location within the
view's bounds via `HoverPhase` enum.

**Focus:** `@FocusState` property wrapper tracks which view has keyboard focus. Can be bound
to a Boolean (single field) or enum (multiple fields). Hover and focus are **completely
separate systems**.

### State Management

**Property wrapper hierarchy:**
- `@State` -- local mutable state owned by the view. Changes trigger re-render.
- `@Binding` -- two-way reference to parent's state.
- `@ObservedObject` / `@StateObject` -- external model via `ObservableObject` protocol.
  `@Published` properties trigger observation notifications.
- `@EnvironmentObject` -- dependency injection through the view tree.

**Push model.** State changes propagate automatically. When a `@Published` property changes,
all observing views are re-rendered. SwiftUI tracks dependencies at the property level.

### KEY INSIGHT

**Transactions as animation metadata.** Animations aren't a separate system bolted onto state
changes -- they ARE metadata on state changes. Every state mutation flows through a Transaction
that says "here's how to interpolate." This means any state change can be animated with zero
widget cooperation. The widget doesn't need to know about animation; it just declares what its
state-dependent properties are.

**What to steal:** The Transaction concept. When ori_term's UI state changes, the change itself
should carry animation metadata. Widgets declare animatable properties. The framework handles
interpolation. No per-widget animation controllers.

---

## 2. Flutter (Google, Widget Tree)

### Event System

**Gesture Arena for disambiguation.** Flutter's unique contribution. When a pointer-down occurs,
all gesture recognizers whose widgets contain the pointer enter a "gesture arena." They observe
pointer-move events. Recognition rules:
- A recognizer can eliminate itself (leave the arena)
- If one recognizer is left, it wins by default
- A recognizer can declare itself winner, ejecting all others
- GestureArenaTeam: groups of recognizers that cooperate (one wins on behalf of the team)

**Hit testing pipeline:**
Physical Touch -> Platform -> Flutter Engine (PointerEvents) -> Hit Test (walk RenderObject
tree) -> Gesture Arena -> Gesture Recognizers -> Widget callbacks

**HitTestBehavior** controls event pass-through:
- `deferToChild` (default) -- passes to children first
- `opaque` -- absorbs, prevents background widgets
- `translucent` -- allows background widgets to also receive

**No bubbling in the DOM sense.** Events flow through the hit test list. The arena
disambiguates. Parent/child gesture conflicts are resolved by arena competition, not
propagation. The child GestureDetector wins by default (first to enter arena).

### Animation System

**Explicit by default.** AnimationController manages timing (0.0 to 1.0 over a Duration).
Tween maps that range to actual values. CurvedAnimation applies easing. The controller is
bound to a TickerProvider (usually `SingleTickerProviderStateMixin`).

**Ticker integration:** A Ticker fires once per vsync. The SchedulerBinding manages three
callback types:
- **Transient** (onBeginFrame) -- Tickers and AnimationControllers fire here
- **Persistent** (after transient) -- the rendering pipeline
- **Post-frame** -- cleanup, rarely used

**Implicit animations:** `AnimatedContainer`, `AnimatedOpacity`, etc. manage their own
AnimationController internally. You just change properties and the widget animates.

**Physics simulations:** `SpringSimulation`, `FrictionSimulation`, `GravitySimulation`. Used
via `AnimationController.animateWith(simulation)`. The simulation provides position/velocity
at each time step until `isDone` returns true.

**How a widget says "keep rendering":** The Ticker keeps calling `scheduleFrame()` every
vsync while the AnimationController is running. When the animation completes, the Ticker
stops, and no more frames are scheduled.

### Hover/Focus System

**MouseRegion widget:** Wraps a subtree to receive mouse enter/exit/hover events. Separate
from GestureDetector. Provides `onEnter`, `onExit`, `onHover` callbacks.

**FocusNode tree (parallel to widget tree):**
- `FocusNode` -- long-lived, persists between builds. Holds focus state.
- `FocusScopeNode` -- grouping mechanism. Focus traversal stays within scope unless
  explicitly broken. Keeps history of focused nodes within its subtree.
- `FocusableActionDetector` -- combines Focus + MouseRegion + Actions + Shortcuts into one
  widget. This is what Flutter's built-in controls use internally.

**Focus traversal:** Tab-order navigation. Customizable via `FocusTraversalPolicy`.

### State Management

**setState() -> rebuild.** Calling `setState()` marks the Element dirty. The framework
rebuilds it on the next frame. Only the subtree rooted at that StatefulWidget rebuilds.

**InheritedWidget:** Data propagation down the tree. Descendants call
`context.dependOnInheritedWidgetOfExactType<T>()` to subscribe. When the InheritedWidget
updates, only subscribed descendants rebuild. `updateShouldNotify()` controls whether
subscribers are actually notified.

**Push model.** setState() is imperative. The framework doesn't know which properties changed;
it rebuilds the whole widget subtree and diffs the resulting Element tree.

### KEY INSIGHT

**Gesture Arena.** The single most elegant gesture disambiguation system in any framework.
Instead of bubbling/capture phases with `stopPropagation()`, gestures *compete*. A tap and a
drag can both observe the same pointer, and the arena resolves who wins based on evidence
(distance moved, time elapsed). This eliminates the entire class of "parent ate my click"
bugs.

**What to steal:** The arena concept for ori_term's split-pane resize handles vs. text
selection vs. scrollbar interactions. When the user mouse-downs on a split boundary, a resize
recognizer and a selection recognizer can both enter the arena. If the user drags
horizontally, resize wins. If vertically (on a horizontal split), selection wins.

---

## 3. egui (Rust, Immediate Mode)

### Event System

**Frame-delayed interaction.** egui's most distinctive trait: interactions at Frame N use widget
rectangles from Frame N-1. The framework maintains double-buffered `PassState` per viewport --
at each pass end, `this_pass` and `prev_pass` swap.

**Hit testing:** `hit_test()` produces a `WidgetHits` struct. An `interact_radius` (default
5px) extends hit areas, making small widgets easier to click (especially on touch). Hit testing
runs against the *previous frame's* layout.

**No propagation model.** Every widget calls `ui.interact(rect, id, sense)` or returns a
`Response`. The Response tells you directly: `.hovered()`, `.clicked()`, `.dragged()`,
`.double_clicked()`, etc. There's no event routing tree. The immediate-mode loop IS the
dispatch.

**Sense enum:** Declares what interactions a widget cares about:
- `Sense::click()` -- clicks and hover
- `Sense::drag()` -- drags and hover
- `Sense::click_and_drag()` -- all three
- `Sense::focusable()` -- keyboard focus only (for screen readers)

**Mouse capture:** Implicit. When you start dragging a widget (identified by its Id), egui
routes all subsequent pointer events to that widget's Id until release. No explicit
`capture_mouse()` call needed.

### Animation System

**Minimal built-in support.** egui provides:
- `ctx.animate_bool(id, bool)` -- returns a 0.0..1.0 value that smoothly transitions
- `ctx.animate_value_with_time(id, target, duration)` -- interpolates any f32
- Both use easing (fast start, slow finish)

**Repaint model:**
- `ctx.request_repaint()` -- schedules another frame immediately
- `ctx.request_repaint_after(Duration)` -- schedules repaint after delay
- If no repaint requested, egui sleeps (zero CPU when idle)

**No spring physics.** No built-in transition system. Community crates (`egui_animation`,
`egui_transition_animation`) fill the gap.

**How a widget says "keep rendering":** Call `ctx.request_repaint()` in your UI code. If
you're animating, call it every frame. When done, stop calling it and egui returns to idle.

### Hover/Focus System

**Per-widget via Response.** `response.hovered()` checks if the pointer is over the widget.
`response.has_focus()` checks keyboard focus. No separate hover tracking system.

**No enter/leave events.** You check `hovered()` every frame and compare with your own stored
state if you need transitions.

**Focus:** Tab navigation is built in. Widgets with `Sense::focusable()` participate. Focus
is tracked centrally by `Memory`.

### State Management

**Immediate mode = no state by default.** Your application owns all state. egui is a pure
function: `fn ui(&mut self, ctx: &Context)` -- you read your state, emit widgets, check
responses, mutate your state.

**Persistent state via Memory:** For widget-internal state (scroll positions, collapsible
headers, window positions), egui provides `Memory` with `IdTypeMap` storage. Widgets store/load
by their unique `Id`. This is type-safe (keyed by `TypeId` + widget `Id`).

**No reactivity.** No signals, no subscriptions, no dependency tracking. The entire UI is
rebuilt every frame. The "diff" is implicit: if your widget code produces the same shapes, the
visual output doesn't change.

### KEY INSIGHT

**Frame-delayed hit testing with interact_radius.** By testing against the *previous* frame's
layout, egui eliminates the chicken-and-egg problem of "widget needs to know its bounds to
process input, but layout hasn't happened yet." Combined with interact_radius (extending hit
areas by 5px), this makes widgets feel responsive even when tiny or densely packed.

**What to steal:** The `Sense` model. ori_term widgets should declare upfront what
interactions they care about. A label: `Sense::none()`. A button: `Sense::click()`. A resize
handle: `Sense::drag()`. A scrollbar: `Sense::click_and_drag()`. The framework can then skip
hit-testing for widgets that don't care about the current event type.

Also steal: the `request_repaint()` / `request_repaint_after()` model for animation. Widgets
that are animating request the next frame. When animation is done, they stop. The event loop
can sleep.

---

## 4. Iced (Rust, Elm Architecture)

### Event System

**Unidirectional message flow.** No direct event handlers. Widgets produce `Message` values
that flow into the `update()` function. The framework maps low-level events to widget-specific
Messages.

**Event -> Message mapping:** Widgets like `Button` take a `Message` to emit on click. The
framework handles hit testing internally. You never see mouse coordinates in typical usage.

**For custom widgets:** The `Widget` trait has an `on_event()` method that receives `Event`
enum values (mouse, keyboard, touch, window). The widget can return `Status::Captured` or
`Status::Ignored`.

**Mouse hover:** `mouse_interaction()` method on Widget returns the cursor style. The
`hover()` widget function displays an overlay when the base widget is hovered. `MouseArea`
widget provides explicit hover/press/release Messages.

### Animation System

**Subscription-based.** Animations are driven by time subscriptions:
```
iced::time::every(Duration::from_millis(16))
```
This produces a `Message` every 16ms. Your `update()` function interpolates state values.

**Iced 0.14+ Animation struct:** A higher-level API. `Animation::new(value)` tracks state
changes and provides interpolated values through time, integrating with the reactive rendering
system.

**Reactive rendering (0.14+):** Only modified widgets issue GPU commands. The framework
tracks which widgets changed and skips redraw for unchanged regions.

**No spring physics built-in.** You'd implement it in `update()` using your own simulation.

**How a widget says "keep rendering":** Return a `Subscription` from the `subscription()`
method. While the subscription is active, messages keep flowing. When the animation completes,
remove the time subscription and the app goes idle.

### Hover/Focus System

**MouseArea widget:** Emits messages for press, release, enter, exit, move. Separate from
Button. You wire it yourself.

**Focus:** The framework supports focus and tab navigation, but it's less mature than Flutter's.
Widget::operate() can query/set focus.

### State Management

**Pure Elm architecture.** State is a plain struct. Messages are a plain enum. `update(&mut
self, message: Message)` is the only place state changes. `view(&self) -> Element` produces
the widget tree. No mutation during view.

**Immutable view, mutable update.** The view function takes `&self` (immutable). This
guarantees the widget tree is a pure function of state. State mutations only happen in
`update()`, which takes `&mut self`.

**Command for async.** `update()` can return a `Command` (now called `Task` in 0.13+) for
async operations. The runtime executes it and feeds the result back as a Message.

### KEY INSIGHT

**Subscriptions as declarative event sources.** Instead of registering callbacks, you return a
description of what you want to listen to. `time::every(16ms)` says "I want clock ticks."
`keyboard::listen()` says "I want key events." The runtime manages the actual subscriptions.
When your app stops returning a subscription, the source is automatically dropped.

**What to steal:** The Subscription pattern for ori_term's cursor blink timer and animation
ticks. Instead of manually managing timers, the UI layer declares: "I need a cursor blink
subscription" or "I need animation frames for this transition." The event loop manages the
actual timer. When the animation completes, the subscription disappears and the event loop
returns to sleep.

---

## 5. Druid / Xilem (Rust, Data-First)

### Druid: Event System

**Widget trait with typed contexts.** Every Widget method receives a specific context:
- `event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env)`
- `lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env)`
- `update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env)`
- `layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env)`
- `paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env)`

**Event propagation:** Top-down dispatch through widget tree. Events flow from root to target.
Widgets can `set_handled()` to stop propagation. No bubbling phase.

**Mouse capture via `set_active(true)`.** When active, the widget receives all mouse events
even when the pointer leaves its bounds. Essential for drag operations. Active must be
explicitly cleared (`set_active(false)`) on mouse-up; Druid does NOT auto-clear it.

### Druid: Hot/Active/Focus State Trifecta

This is Druid's most distinctive feature:

- **Hot (hover):** Automatically computed from layout rect. ALL ancestors whose rects contain
  the mouse are hot. `LifeCycle::HotChanged` fires BEFORE the triggering MouseMove. Read via
  `ctx.is_hot()`.

- **Active (pressed):** Manually managed. Widget calls `ctx.set_active(true)` on mouse-down.
  While active, receives all mouse events regardless of pointer position. `ctx.set_active(false)`
  on mouse-up. Read via `ctx.is_active()`.

- **Focus (keyboard):** Widget calls `ctx.request_focus()`. Only one widget focused at a time.
  Ancestors also receive keyboard events. `LifeCycle::FocusChanged` fires on gain/lose. Read
  via `ctx.is_focused()`.

**The elegance:** These three states are orthogonal and compose cleanly. A button's paint logic
is literally: `if is_active { pressed_style } else if is_hot { hover_style } else { normal }`.
No state machines, no boolean flags, no manual enter/exit tracking.

### Druid: Animation System

**AnimFrame event.** Widgets call `ctx.request_anim_frame()`. On the next frame, they receive
`Event::AnimFrame(nanos_since_last)`. The interval is 0 on the first frame after transitioning
from idle to animating.

**Key detail:** Receiving AnimFrame does NOT imply a paint will follow. The widget must
explicitly call `ctx.request_paint()` if visual update is needed.

**No built-in transitions or springs.** You implement interpolation in your AnimFrame handler.
The framework just provides the timing pulse.

### Druid: Lens System

**Lenses decompose state.** A `Lens<S, T>` provides read/write access to a field T within a
larger state S. Widgets bind to a lens, so they only see their relevant slice of state.

Benefits:
- **Performance:** Data changes outside the lens don't trigger the widget's `update()`.
- **Reuse:** A "color picker" widget works with any struct that has a color field, via lens.
- **Composition:** `lens!(AppState, sidebar.background.color)` chains field access.

### Xilem: Reactive Architecture

**Three parallel trees:**
1. **View tree** -- short-lived, rebuilt each cycle. Generated by app closure.
2. **Widget tree** -- long-lived, persisted. Updated by diffing successive view trees.
3. **View state tree** -- long-lived. Holds memoization data, previous view snapshots.

**Event dispatching via id-path.** Each node in the view tree has a stable ID. Events are
routed down the id-path, and at each node the handler receives `&mut AppState`. Adapt nodes
remap the state reference, enabling component composition where subcomponents see a different
state slice.

**Memoization:** When a subtree is a pure function of some data, Xilem compares the data to
the previous version. If unchanged, the entire subtree rebuild is skipped. This is the primary
performance optimization.

### KEY INSIGHT

**Druid's Hot/Active/Focus trifecta.** Three orthogonal boolean states that cover 90% of widget
interaction needs. Hot is automatic (computed from geometry). Active is manual (set by widget on
mouse-down). Focus is requested. The LifeCycle events fire in the right order (HotChanged
before MouseMove). No state machines needed.

**Xilem's Adapt node** is the functional programming answer to Druid's lenses -- but more
powerful because it can transform the state reference at any point in the tree.

**What to steal:** The Hot/Active/Focus model, verbatim. ori_term widgets should have:
- `is_hot()` -- automatically computed from mouse position + layout rect
- `is_active()` -- set on mouse-down, cleared on mouse-up, captures events
- `is_focused()` -- requested, exclusive, routes keyboard events

These three booleans eliminate an enormous amount of per-widget state tracking code.

Also steal: `request_anim_frame()` / `AnimFrame(delta)`. The widget asks for timing pulses. It
gets them with a delta-time. It calls `request_paint()` when it actually has visual changes.
Clean separation of animation logic from rendering.

---

## 6. WPF / WinUI (.NET, XAML)

### Event System

**Three routing strategies (the most complete model):**

1. **Tunneling (Preview):** Root to target. Event handlers on ancestors fire first. Names
   prefixed with "Preview" (e.g., `PreviewMouseDown`). Used for interception/filtering.

2. **Bubbling:** Target to root. Standard event handling. The default for most input events.

3. **Direct:** Only the target. No propagation. Used for non-input events.

**Every input event is a Preview+Bubble pair.** `PreviewMouseDown` tunnels, then `MouseDown`
bubbles. This is the most flexible propagation model of any framework: you can intercept
events before the target sees them (preview/tunnel) OR after (bubble).

**Routed event data:** Both `Source` (original raiser) and `sender` (current handler's
element) are available. Setting `e.Handled = true` stops propagation. But even handled events
can be observed via `AddHandler(event, handler, handledEventsToo: true)`.

**Mouse capture:** `element.CaptureMouse()` redirects all mouse input to that element until
`ReleaseMouseCapture()`. Fires `GotMouseCapture` / `LostMouseCapture` events. Essential for
drag operations. Force-capturing can interfere with drag-and-drop.

### Animation System

**Storyboard/Timeline architecture:**
- **Timeline** -- base class. Defines duration, repeat, auto-reverse.
- **Animation** -- interpolates a specific type (DoubleAnimation, ColorAnimation, etc.)
- **Storyboard** -- container of Timelines. Targets specific objects + properties via
  `Storyboard.TargetName` and `Storyboard.TargetProperty` attached properties.

**Property-driven.** Animations target dependency properties. A `DoubleAnimation` on
`Opacity` smoothly interpolates the value. The animation system *overlays* the animated value
on top of the property's base value with a strict precedence order:
1. Property system coercion (highest)
2. Active animations
3. Local values
4. Triggers/styles/templates
5. Inheritance
6. Default

**EventTrigger:** Connects routed events to Storyboards declaratively in XAML:
"When this event fires, play this animation."

**VisualStateManager (VSM):**
- Groups of mutually exclusive visual states (e.g., CommonStates: Normal, PointerOver,
  Pressed, Disabled).
- `VisualStateManager.GoToState(control, "Pressed", true)` transitions between states.
- Transitions between states are animated via Storyboards defined in the state group.
- States are defined in ControlTemplates, making them reusable across all instances.

### Hover/Focus System

**Automatic via dependency properties:**
- `IsMouseOver` -- read-only, automatically maintained. True when pointer is over the element
  or any of its children.
- `IsMouseDirectlyOver` -- true only when over the element itself, not children.
- `MouseEnter` / `MouseLeave` -- bubbling events.

**Focus:** `IsFocused`, `IsKeyboardFocused`, `IsKeyboardFocusWithin`. Multiple levels of
focus awareness. Tab navigation is automatic based on `TabIndex`.

### State Management

**Dependency Property system.** Not stored in the object -- stored in the WPF property system.
Value resolution follows a precedence hierarchy (animations > local > style > inheritance >
default). Coercion callbacks can clamp/modify values. Change notification is automatic:
when a dependency property changes, all bindings are notified.

**Data binding:** Two-way bindings between UI properties and data model. Changes propagate
automatically in either direction. Binding expressions support converters, validation, and
fallback values.

### KEY INSIGHT

**Dependency Property value precedence.** A single property can be simultaneously influenced by
animations, local values, styles, triggers, templates, and inheritance. The property system
resolves the "winner" through a fixed precedence order. Animations automatically override local
values while running, then gracefully yield back when complete. Coercion sits above everything,
providing hard constraints (e.g., a slider thumb can't exceed the track bounds).

This means a button can have a style-defined color, a hover-triggered color override, a press
animation, and a disabled state -- all on the SAME property -- and the framework resolves which
value "wins" at any given moment.

**Also:** The Preview/Bubble pair pattern. Having BOTH tunneling and bubbling for every input
event is supremely powerful. A parent can intercept (and optionally suppress) events before
children see them via Preview handlers. This is strictly more powerful than bubble-only
(Flutter/SwiftUI) or capture-bubble without paired naming (DOM).

**What to steal:** The VisualStateManager pattern. ori_term widgets should declare state groups:
`CommonStates { Normal, Hovered, Pressed, Disabled }`. Transitions between states carry
animation metadata. The widget's visual properties are defined per-state. The framework handles
interpolating between states. This eliminates manual `if is_hot { ... } else { ... }` paint
logic.

Also steal: The dependency property precedence concept. When an animation is running on a
property, it overrides the base value. When it stops, the base value returns. No manual cleanup.

---

## 7. GTK4 (C/GObject, Signal-Based)

### Event System

**Capture-Bubble-Target three-phase propagation:**
1. **Capture phase:** Event walks from root (GtkWindow) down to target. Controllers with
   `GTK_PHASE_CAPTURE` can intercept.
2. **Target phase:** Event reaches the target widget.
3. **Bubble phase:** Event walks back up from target to root. Controllers with
   `GTK_PHASE_BUBBLE` handle here.

**EventController architecture (GTK4's revolution over GTK3):** Input handling is decomposed
into standalone controller objects, not widget virtual methods:
- `GtkEventControllerMotion` -- pointer enter/leave/motion
- `GtkEventControllerKey` -- key press/release
- `GtkGestureClick` -- click recognition
- `GtkGestureDrag` -- drag recognition
- `GtkEventControllerFocus` -- focus in/out
- `GtkEventControllerScroll` -- scroll events

Multiple controllers can be attached to a single widget. Each controller declares its
propagation phase. A controller can mark an event as consumed to stop propagation.

**Signal system (GObject):** All communication is signal-based. `g_signal_connect()` registers
handlers. `g_signal_emit()` fires synchronously. Signals support:
- **Detail:** Extra qualifier for fine-grained subscription (e.g., "notify::visible")
- **Accumulators:** Combine return values from multiple handlers
- **Class handlers:** Default handlers defined by the widget class

### Animation System

**Tick callback:** `gtk_widget_add_tick_callback()` registers a function called before each
frame (at display refresh rate). Receives a `GdkFrameClock` for timing.

**Important:** The tick callback does NOT automatically cause a repaint. You must call
`gtk_widget_queue_draw()` or `gtk_widget_queue_resize()` explicitly.

**CSS transitions:** GTK4 supports CSS-like transitions on widget properties. When a property
changes (e.g., due to a state change from hover), the transition interpolates smoothly.

**GtkRevealer:** Higher-level animation widget. Animates child visibility with configurable
transition types (slide, crossfade, etc.).

**Frame timing:** `gdk_frame_clock_get_frame_time()` for continuous animations.
`gdk_frame_timings_get_predicted_presentation_time()` for frame-perfect isolated events.

### Hover/Focus System

**GtkEventControllerMotion provides two critical properties:**
- `contains-pointer` -- true if pointer is anywhere in the widget's subtree (widget or
  any descendant)
- `is-pointer` -- true only if pointer is directly over THIS widget, not a descendant

This distinction elegantly handles nested widgets. A parent container can know "the pointer is
somewhere inside me" (`contains-pointer`) vs. "the pointer is on me specifically"
(`is-pointer`).

**Enter/Leave signals:** The `enter` signal fires when the pointer enters the widget. The
`leave` signal fires when it exits. Property updates happen: `contains-pointer` is updated
*before* `enter` but *after* `leave`.

**Focus:** `GtkEventControllerFocus` tracks keyboard focus. `:focusable` property determines
if a widget can receive focus. Focus is per-window.

### State Management

**GObject properties with notification.** Properties are defined on GObject classes. Setting a
property automatically emits `notify::property-name` signal. Bindings can connect properties
between objects.

**No automatic re-render on state change.** Widgets must explicitly queue_draw() when their
visual state changes. The signal system provides the notification; the widget decides whether to
repaint.

### KEY INSIGHT

**EventController composition.** Instead of a monolithic Widget::event() method, input handling
is decomposed into reusable controller objects. A widget that needs drag gets a
`GtkGestureDrag`. A widget that needs hover gets a `GtkEventControllerMotion`. These are
**independent, composable objects** that can be mixed and matched without subclassing.

Combined with `contains-pointer` vs. `is-pointer`, this provides the cleanest nested hover
model of any framework.

**What to steal:** The EventController pattern. ori_term should NOT have a monolithic
`fn event(&mut self, event: Event)` on widgets. Instead, widgets should attach controller
objects: `HoverController`, `DragController`, `FocusController`, `ClickController`. Each
controller is reusable and independently testable. The widget composes behavior by combining
controllers.

Also steal: `contains-pointer` / `is-pointer` distinction. When the mouse is over a button
inside a panel, the button gets `is-pointer=true, contains-pointer=true` and the panel gets
`is-pointer=false, contains-pointer=true`. This is the correct mental model for nested hover.

---

## 8. Qt Quick / QML (C++, Property Binding)

### Event System

**Declarative event handling.** MouseArea is a transparent rectangle that receives mouse events:
- `onClicked`, `onPressed`, `onReleased`, `onDoubleClicked`
- `onPositionChanged` (during press, or always if `hoverEnabled: true`)
- `onEntered`, `onExited`
- `onPressAndHold`

**Event propagation:** QML items receive events based on stacking order (z-order, last child
painted = first to receive). `MouseArea.propagateComposedEvents` allows events to pass through.
`mouse.accepted = false` in a handler passes the event to items below.

**FocusScope:** Groups of focusable items. Within a FocusScope, one item has `focus: true`. When
the FocusScope itself receives active focus, its focused child gets active focus. This enables
component-level focus management.

**Signal/Slot:** The fundamental communication mechanism. Signals are emitted, slots (or
JavaScript functions) receive. Connections are type-safe and can cross thread boundaries.

### Animation System

**Declarative, property-targeted animations:**

```qml
Rectangle {
    Behavior on x { NumberAnimation { duration: 500 } }
}
```

This means: "whenever `x` changes, animate the change over 500ms." The `Behavior on` pattern
is the most concise implicit animation declaration of any framework.

**States and Transitions:**
```qml
states: [
    State { name: "pressed"; PropertyChanges { target: rect; color: "red" } }
]
transitions: [
    Transition { from: "*"; to: "pressed"; ColorAnimation { duration: 200 } }
]
```

States define property snapshots. Transitions define how to animate between snapshots. The
wildcard `"*"` matches any state.

**Animation types:** PropertyAnimation, NumberAnimation, ColorAnimation, RotationAnimation,
SequentialAnimation, ParallelAnimation, SpringAnimation, SmoothedAnimation.

**SpringAnimation:** First-class spring physics with `spring`, `damping`, `mass`, `epsilon`
properties.

### Hover/Focus System

**MouseArea with hoverEnabled:**
- `containsMouse` property: true when pointer is over the MouseArea
- By default, hover tracking only when a button is pressed
- `hoverEnabled: true` enables passive hover tracking
- `onEntered` / `onExited` signals for enter/leave

**HoverHandler (Qt 6+):** Newer, lighter alternative to MouseArea for pure hover detection.
Doesn't block events. Multiple HoverHandlers can be on the same item.

**Focus:** `FocusScope` groups focusable items. `focus: true` requests focus. `activeFocus`
is the read-only property indicating actual keyboard focus (FocusScope must also have focus).

### State Management

**Property bindings with automatic dependency tracking.** This is QML's defining feature:

```qml
Rectangle {
    width: parent.width / 2  // Binding: auto-updates when parent.width changes
}
```

**How it works internally:** When a binding expression is evaluated, the QML engine's V8
wrapper captures every property access. These become dependencies. When any dependency's
`notify` signal fires, the binding is marked dirty.

**Lazy evaluation (Qt 6.2+):** Changed properties don't immediately re-evaluate bindings.
Instead, bindings are lazily re-evaluated when the property is next read. This avoids
cascading re-evaluations when multiple dependencies change simultaneously.

**Change notification:** QProperty (C++ side) automatically emits change notifications. No
manual `emit` needed for Q_PROPERTY with BINDABLE.

### KEY INSIGHT

**`Behavior on` -- the implicit animation pattern.** Four words: `Behavior on x { ... }`. This
says "whenever this property changes, animate the change." It's the most concise and intuitive
animation declaration of any framework. You don't touch animation controllers. You don't manage
timers. You don't call interpolation functions. You just say "animate this property" and it
happens.

Combined with **lazy binding evaluation**, QML avoids the "cascade storm" problem where changing
one property triggers a chain of synchronous re-evaluations.

**What to steal:** The `Behavior on` concept. ori_term should support property-level animation
declarations. When a widget property (position, opacity, color, size) has an associated
behavior, any change to that property is automatically animated. No manual interpolation code
in the widget.

Also steal: Lazy binding evaluation. When multiple properties change in the same frame (e.g.,
window resize changes width, height, and aspect ratio), bindings should be lazily evaluated
once per frame, not eagerly after each individual change.

---

## Cross-Framework Synthesis: What ori_term Should Steal

### Event System

| Decision | Recommendation | Source |
|----------|---------------|--------|
| Propagation model | **Capture + Bubble** (two phases) | WPF, GTK4 |
| Hit testing | **Layout rect based**, with interact_radius for touch | egui, SwiftUI |
| Gesture disambiguation | **Arena pattern** for competing gestures | Flutter |
| Mouse capture | **Active state** on mouse-down, auto-routes events | Druid |
| Event decomposition | **Controller objects**, not monolithic event() | GTK4 |
| Interaction declaration | **Sense enum** per widget | egui |

### Animation System

| Decision | Recommendation | Source |
|----------|---------------|--------|
| Animation trigger | **Transaction on state change** (animation metadata travels with the mutation) | SwiftUI |
| Implicit animation | **Behavior on property** declarations | QML |
| Frame timing | **request_anim_frame() / AnimFrame(delta)** | Druid |
| Spring physics | First-class spring model | SwiftUI, QML |
| Render scheduling | **request_repaint() / request_repaint_after()** | egui |
| Visual states | **VisualStateManager** with animated transitions | WPF |
| Idle behavior | **Sleep when no animation active** | egui, Iced |

### Hover/Focus System

| Decision | Recommendation | Source |
|----------|---------------|--------|
| Hover model | **Hot/Active/Focus trifecta** (three orthogonal booleans) | Druid |
| Nested hover | **contains-pointer vs. is-pointer** distinction | GTK4 |
| Hover computation | **Automatic from layout rect** (not manual) | Druid |
| Focus scopes | **FocusScope grouping** for component-level focus | Flutter, QML |
| Lifecycle | **HotChanged fires before MouseMove** | Druid |

### State Management

| Decision | Recommendation | Source |
|----------|---------------|--------|
| Ownership | **App owns state**, framework owns presentation | Iced, egui |
| Change propagation | **Lazy evaluation** (batch per frame, not eager) | QML |
| Update trigger | **Explicit request_paint()**, not automatic on state change | Druid, GTK4 |
| Data decomposition | **Lens/Adapt pattern** for subcomponent state access | Druid, Xilem |

### The Architecture

Combining the best ideas:

1. **Widgets declare Sense** (what interactions they care about).
2. **EventControllers** (composable, reusable) handle the actual event processing.
3. **Hot/Active/Focus** states are maintained by the framework, not widgets.
4. **State changes carry Transactions** (animation metadata).
5. **Properties have optional Behaviors** (implicit animation on change).
6. **VisualStateGroups** define appearance per-state with animated transitions.
7. **request_anim_frame()** for explicit animation; the framework sleeps when idle.
8. **Gesture Arena** resolves competing interactions (resize handle vs. text selection).
9. **Lazy evaluation** batches binding updates per frame.
10. **Capture + Bubble** propagation for maximum flexibility.
