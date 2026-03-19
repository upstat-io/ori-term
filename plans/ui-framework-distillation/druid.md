# druid Deep-Dive: The Original Inspiration

ori_term's Widget trait was originally inspired by druid. Focus: what we adopted poorly, what we evolved beyond, and why druid declined.

## Architecture Summary

Retained-mode widget tree. 5-method Widget trait: `event()`, `lifecycle()`, `update()`, `layout()`, `paint()`. `WidgetPod` wraps widgets with state tracking. Generic `T` data parameter everywhere. Lens system for data binding.

## Key Patterns

### 1. Five-Method Widget Trait

**Files:** `druid/src/widget/mod.rs`

```rust
pub trait Widget<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env);
    fn lifecycle(&mut self, ctx: &mut LifecycleCtx, event: &LifeCycle, data: &T, env: &Env);
    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env);
    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size;
    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env);
}
```

Clear phase separation. Each context type limits what widgets can do.

**Strengths:** Proven design, easy to reason about.
**Weakness:** Generic `T` everywhere causes type parameter spam and confusing compiler errors.

### 2. WidgetPod — Automatic State Tracking

**Files:** `druid/src/core.rs`

WidgetPod wraps widgets with: layout rect, origin, paint insets, `is_hot`, `is_active`, `has_focus`, `needs_layout`, `needs_paint`. Automatically computes hot state from mouse position + layout geometry. Emits `HotChanged`, `FocusChanged` lifecycle events.

**Strengths:** Widgets don't track interaction state manually. Framework handles it.
**Key insight for ori_term:** Hot state is computed automatically by the pod, not by widgets.

### 3. BoxConstraints Layout

**Files:** `druid/src/box_constraints.rs`

Same model as Flutter. `BoxConstraints { min, max }`. Widgets shrink constraints for children, children report size, parents position children.

### 4. Context-Based Capability Limits

**Files:** `druid/src/contexts.rs`

- `EventCtx`: can `set_active()`, `request_focus()`, `request_paint()`
- `LifecycleCtx`: can `register_for_focus()`
- `UpdateCtx`: can `request_layout()`, `request_paint()`
- `LayoutCtx`: can compute sizes
- `PaintCtx`: can draw (read-only state)

Compile-time enforcement of what each phase can do.

### 5. Region-Based Invalidation

Widgets call `ctx.request_paint()` or `ctx.request_paint_rect(rect)`. Framework tracks dirty regions, only repaints affected areas.

### 6. Lens System

```rust
#[derive(Lens)]
struct AppState { name: String, count: u32 }
Flex::column()
    .with_child(TextBox::new().lens(AppState::name))
    .with_child(Slider::new().lens(AppState::count))
```

Lenses narrow `AppState` to individual fields. Each widget only sees its piece of data.

**Strengths:** Compositional data binding.
**Weakness:** Extremely verbose, hard to learn, boilerplate for nested structures.

### 7. Command/Selector System

Cross-widget communication via typed `Command`s dispatched through the widget tree. Better than callbacks for global operations (save, undo, focus).

### 8. Testing Harness

**Files:** `druid/src/tests/harness.rs`

Widget testing without a window. Simulate events, assert on paint output.

## Why Druid Declined

1. **Generic `T` everywhere** — Every widget, every container, every adapter needs matching type parameters. Compiler errors are inscrutable.
2. **Lens boilerplate** — Workaround for generics is verbose (every subtree needs `LensWrap`)
3. **Clone-heavy data model** — `Data` trait requires `Clone + PartialEq`. Expensive for large structures.
4. **No virtual scrolling** — Creates all widgets even if off-screen
5. **Steep learning curve** vs React/Vue
6. **Team moved on** — Linebender shifted to masonry/xilem

## What ori_term Should Learn

### Keep (things druid got right that we should preserve)
1. **5-method trait shape** — Clear phase separation, proven pattern
2. **Context-based capability limits** — Compile-time enforcement of phase rules
3. **Automatic hot/active/focus** — Framework computes, widgets query
4. **Region-based invalidation** — Only repaint what changed

### Don't Repeat (druid's mistakes)
1. **No generic `T` for widget data** — ori_term's single AppState approach is better
2. **No Lens system** — Too much boilerplate for the benefit
3. **Don't clone data on every update** — Use references or Arc
4. **Add virtual scrolling** — Critical for performance with many widgets

### Improve On
1. **Formalize test harness** — Druid's Harness is a good starting point
2. **Synthesize lifecycle events** — HotChanged, FocusChanged are valuable but should be separate from structural lifecycle
