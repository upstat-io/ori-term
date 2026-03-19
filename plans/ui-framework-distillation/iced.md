# iced Deep-Dive: Elm Architecture for Rust GUI

## Architecture Summary

iced implements the **Elm Architecture**: `Application = Boot + Update + View`. State updates are pure functions that return `Task<Message>`. View is a pure function rebuilding the UI tree each frame. Tree diffing via `Tag` (TypeId) preserves widget state across rebuilds.

## Key Patterns

### 1. Message-Based Communication

**Files:** `src/application.rs`, `program/src/lib.rs`

Widgets publish messages to `Shell`. Messages collected in `Vec`, app calls `update()` for each. Update returns `Task` (monadic: `.map()`, `.then()`, `.chain()`, `.batch()`). No callbacks, no closures stored in widgets.

**Strengths:** Pure functional update, decoupled UI from logic, type-safe routing.
**Weaknesses:** Verbose for simple interactions, message enum grows large.

### 2. Tree-Based Widget State Persistence

**Files:** `core/src/widget.rs`, `core/src/widget/tree.rs`

```rust
pub struct Tree {
    pub tag: Tag,           // TypeId of widget state type
    pub state: State,       // Box<dyn Any>
    pub children: Vec<Tree>,
}
```

Reconciliation: compare `Tag` → if same type, call `widget.diff()` → if different, recreate subtree. Panics on type mismatch (catches bugs).

**Strengths:** Type-safe state, automatic reconciliation, survives reordering.

### 3. Widget Trait with Full Phase Separation

```rust
pub trait Widget<Message, Theme, Renderer> {
    fn size(&self) -> Size<Length>;
    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node;
    fn draw(&self, tree: &Tree, renderer: &mut Renderer, theme: &Theme, ...);
    fn update(&mut self, tree: &mut Tree, event: &Event, ..., shell: &mut Shell);
    fn children(&self) -> Vec<Tree>;
    fn diff(&self, tree: &mut Tree);
}
```

### 4. Shell as Event Publisher

```rust
pub struct Shell<'a, Message> {
    messages: &'a mut Vec<Message>,
    event_status: event::Status,     // Ignored or Captured
    is_layout_invalid: bool,
}
```

Widgets call `shell.publish(message)`, `shell.capture_event()`, `shell.invalidate_layout()`. Unified interface for all side effects.

### 5. Limits-Based Layout

`Limits { min_size, max_size }` propagated top-down. `Length` enum: `Fill | Shrink | Fixed(px)`. Helper DSL: `atomic()`, `sized()`, `contained()`, `padded()`, `flex()`.

### 6. Immutable Theme Data

```rust
pub struct Palette { primary, success, danger, warning, background, foreground: Color }
```

Pre-computed from seed. Passed by reference, never mutated at runtime. Status-based styling: `button::Status::Active | Hovered | Pressed | Disabled`.

### 7. Declarative Subscriptions

```rust
fn subscription(state: &State) -> Subscription<Message> {
    if state.timer_enabled {
        time::every(Duration::from_secs(1))
    } else {
        Subscription::none()
    }
}
```

Runtime hashes subscriptions. Different hash = kill old + create new. Same hash = keep running. Enables state-dependent streams.

### 8. Task Monads for Async

```rust
Task::none() | Task::done(value) | Task::perform(future, f) | Task::run(stream, f) | Task::batch(tasks)
task.map(f) | task.then(f) | task.chain(task2) | task.collect()
```

### 9. Testing: Headless Simulator

**Files:** `test/src/lib.rs`

```rust
let messages = ui.click("Increment")?;
let messages = ui.tap_key(Key::Enter)?;
assert!(ui.find("1").is_ok());
```

No mocking, natural selectors, pure testing.

## What ori_term Should Adopt

1. **Shell-like event propagation API** — unified interface for message publication, event capture, layout invalidation
2. **Tree-based state with Tag/TypeId** for widget identity across rebuilds
3. **Immutable theme data** — compute once, reference everywhere, pre-compute color variants
4. **Declarative subscriptions** for timer/IO/event streams with automatic lifecycle
5. **Explicit Task monads** for async operations

## Where ori_term Is Superior

1. Built-in animation engine (iced has none built-in)
2. GPU-first rendering (iced uses pluggable renderers)
3. Explicit InteractionManager vs implicit event handling
4. Already has controller decomposition (iced widgets handle everything in `update()`)
