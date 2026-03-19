# masonry Deep-Dive: Druid's Successor — Hard-Won Lessons

Masonry is druid's deliberate fork by the Linebender team. It represents lessons learned from druid's mistakes. Everything that changed, changed for a reason.

## Architecture Summary

Retained-mode widget tree. **Separated event streams** (PointerEvent, TextEvent, StatusChange, LifeCycle). WidgetPod with Bloom filter child tracking. Safety rails that panic in debug if widgets break invariants. `declare_widget!` macro for mutation wrappers. RenderRoot decoupled from app layer.

## Key Changes from Druid

### 1. Separated Event Types

**Before (druid):** Single `event()` method with giant `Event` enum (MouseDown, MouseUp, KeyDown, Wheel, Timer, Command...).

**After (masonry):**
```rust
fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &PointerEvent);
fn on_text_event(&mut self, ctx: &mut EventCtx, event: &TextEvent);
fn on_status_change(&mut self, ctx: &mut LifeCycleCtx, event: &StatusChange);
fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle);
```

**Why:** Widgets that only handle pointer events don't need to match on keyboard variants. Status changes (HotChanged, FocusChanged) are framework-generated state notifications, not user events.

### 2. StatusChange — Formalized State Notifications

```rust
pub enum StatusChange {
    HotChanged(bool),
    FocusChanged(bool),
}
```

Sent **before** the event that caused them. All ancestors with widget in their layout rect also get hot. Button gets it first (innermost), then parent, then grandparent.

**Hot state feedback loop solution:** `update_hot_state()` called during event handling AND during `place_child()` in layout. If a button shrinks on hover causing mouse to leave, StatusChange fires inline.

### 3. Bloom Filter Child Tracking

**Files:** `src/widget/widget_state.rs`

```rust
pub struct WidgetState {
    pub(crate) children: Bloom<WidgetId>,
    ...
}
```

When you call `child.on_event(ctx, ...)`, masonry marks that child as visited. If a child isn't visited by end of method → **debug panic**. Catches the #1 container bug: forgetting to recurse to a child.

### 4. Safety Rails

**Files:** `src/widget/tests/safety_rails.rs` (370 lines of validation tests)

Runtime assertions in debug mode:
- All declared children visited in every pass
- `place_child()` called for every child in layout
- No widget visited twice in same pass
- Stashed widgets never laid out or painted
- New widgets receive `WidgetAdded` before any other event
- Disabled children receive `DisabledChanged`

**Smart skips:** If child is stashed, event is handled, or child is out of bounds — skip is allowed.

### 5. declare_widget! Macro — Auto-Invalidating Mutation

```rust
pub struct Button { label: WidgetPod<Label> }
crate::declare_widget!(ButtonMut, Button);

impl<'a> ButtonMut<'a> {
    pub fn set_text(&mut self, text: impl Into<ArcStr>) {
        self.label_mut().set_text(text.into());
        // Auto-invalidates: WidgetMut::drop calls merge_up()
    }
}
```

When `WidgetMut` drops, it propagates invalidation flags to parent. Can't forget to call `request_paint()`.

### 6. WidgetRef for Safe Read-Only Access

```rust
pub struct WidgetRef<'w, W: Widget + ?Sized> {
    widget_state: &'w WidgetState,
    widget: &'w W,
}
```

Methods: `state()` (layout rect, flags), `deref()` (widget), `downcast()`, `find_widget_by_id()`, `find_widget_at_pos()`. Debug impl prints widget tree.

### 7. RenderRoot — Framework Decoupled from App

```rust
pub struct RenderRoot {
    root: WidgetPod<Box<dyn Widget>>,
    state: RenderRootState,
}

pub enum RenderRootSignal {
    Action(Action, WidgetId),
    RequestRedraw,
    RequestAnimFrame,
    SetCursor(CursorIcon),
    ...
}
```

RenderRoot doesn't know about app data. It routes events, schedules redraws, emits signals. The app layer (Xilem or whatever) consumes signals.

### 8. Simple Actions (Not Commands)

```rust
pub enum Action { ButtonPressed, TextChanged(String), CheckboxChecked(bool), Other(Arc<dyn Any>) }
```

Pure data. App decides what to do. No RPC semantics like druid's Commands.

### 9. PromiseToken for Async

```rust
pub struct PromiseToken<T>(PromiseTokenId, PhantomData<T>);
```

Widget creates token, gives to async work. When complete, app submits `PromiseResult`. Widget pattern-matches on its token. No runtime dependency.

### 10. Context Capability Macro

```rust
impl_context_method!(
    WidgetCtx<'_>, EventCtx<'_>, LifeCycleCtx<'_>, PaintCtx<'_>, LayoutCtx<'_>,
    { pub fn widget_id(&self) -> WidgetId { self.widget_state.id } }
);
```

Shared methods across contexts without code duplication. Phase-specific methods only on their context.

### 11. Test Harness

**Files:** `src/testing/harness.rs` (670 lines)

```rust
let mut harness = TestHarness::create(Button::new("Click"));
harness.mouse_move_to(button_id);
assert!(harness.get_widget(button_id).state().is_hot);
harness.mouse_button_press(MouseButton::Left);
assert_eq!(harness.pop_action(), Some((Action::ButtonPressed, button_id)));
```

Snapshot testing: `assert_render_snapshot!(harness, "button_hover")`.

## What ori_term Should Adopt

### High Priority
1. **Separated event types** — Split PointerEvent, TextEvent, StatusChange. Our event system currently mixes these.
2. **StatusChange notifications** — HotChanged/FocusChanged as explicit framework events, not widget-tracked state.
3. **Safety rails** — Debug assertions validating all children visited, no double-visits, stashed handling.
4. **Context capability limits** — Each phase gets its own context type with restricted methods.
5. **Test harness** — Headless widget testing with input simulation and state inspection.

### Medium Priority
6. **declare_widget! equivalent** — Auto-invalidating mutation wrappers for widgets with settable properties.
7. **WidgetRef for introspection** — Safe read-only access + tree printing for debugging.
8. **Bloom filter for children** — Cheap validation of tree traversal completeness.
9. **RenderRoot signals** — Decouple framework from app; emit signals instead of direct mutation.

### Design Principles to Internalize
- Hot state is state, not an event — deserves its own notification mechanism
- Safety rails catch bugs early — debug assertions on tree traversal save hours
- Separate framework from app — RenderRoot doesn't know about app data
- Capability-based API — can only call methods valid in your phase
- Testing as first-class concern — harness in core, not separate crate
