# GPUI (Zed) Deep-Dive: GPU-Accelerated Editor Framework

**Most relevant to ori_term** — built for a GPU-accelerated code editor.

## Architecture Summary

GPUI uses a **centralized entity ownership model** where `App` is the sole owner of all state. Access mediated through `Context<T>` handles. 3-pass rendering: `request_layout` → `prepaint` → `paint`. Taffy for layout. Actions + keymap for keyboard dispatch. Arena allocator for frame-scoped elements.

## Key Patterns

### 1. Entity System — Centralized Ownership

**Files:** `app.rs`, `app/entity_map.rs`, `app/context.rs`

```rust
pub struct Entity<T> { entity_id: EntityId, rc: Weak<AtomicUsize> }

counter.update(cx, |counter: &mut Counter, cx: &mut Context<Counter>| {
    counter.count += 1;
    cx.notify(); // Signal observers
});
```

**App owns all state.** Entities are handles (ID + refcount). Access always through context. Lifecycle: Reserve → Insert → Update → Release.

**Strengths:** Eliminates Arc<Mutex> friction, no deadlocks, simple borrow checking.
**Weaknesses:** All access requires App context, harder to test in isolation.

### 2. Three-Pass Element Rendering

**Files:** `element.rs`

```rust
pub trait Element: IntoElement {
    type RequestLayoutState;
    type PrepaintState;
    fn request_layout(&mut self, ...) -> (LayoutId, Self::RequestLayoutState);
    fn prepaint(&mut self, bounds, layout, ...) -> Self::PrepaintState;
    fn paint(&mut self, bounds, layout, prepaint, ...);
}
```

Phase 1: Taffy layout computation. Phase 2: Commit hitboxes, prepare GPU resources. Phase 3: Render primitives to Scene.

**Key:** State flows between phases via associated types. No allocation needed between phases.

### 3. Element State Persistence

```rust
window.with_element_state(global_id, |state: &mut Option<HoverState>, window| {
    let mut state = state.unwrap_or_default();
    // Update hover...
    (result, state)
});
```

State keyed by `GlobalElementId`, persists across frames if ID stable. Hover/drag state lives here, not in widget structs.

### 4. Action Dispatch + Keymap System

**Files:** `key_dispatch.rs`, `input.rs`

```rust
actions!(editor, [MoveUp, MoveDown, Undo, Redo]);

Keymap::new(vec![
    KeyBinding::new("cmd-z", Editor::undo, Some("Editor")),
])
```

**DispatchTree** built during render. Actions bubble from focused element upward. `KeyContext` tags gate which bindings apply. Actions are data (rebindable, replayable), not code.

**Strengths:** Runtime rebinding, macro support, accessibility, testable.

### 5. StyleRefinement — Partial Style Merging

**Files:** `style.rs`, `styled.rs`

`#[derive(Refineable)]` generates sparse `StyleRefinement` struct. Builder methods modify only changed fields:

```rust
div().flex().flex_col().gap_2().p_4().bg(Color::red())
```

Each method only sets one `Option<T>` field. Final `Style` computed by merging refinements.

### 6. Scene-Based GPU Rendering

**Files:** `scene.rs`

```rust
pub struct Scene {
    pub quads: Vec<Quad>,
    pub paths: Vec<Path>,
    pub monochrome_sprites: Vec<MonochromeSprite>,
    pub subpixel_sprites: Vec<SubpixelSprite>,
    ...
}
```

Elements don't render to GPU directly. They push primitives to Scene. Scene sorted by z-order, submitted to GPU backend.

### 7. BoundsTree for Z-Order and Damage

**Files:** `bounds_tree.rs`

R-tree variant that assigns z-order automatically from overlap. Enables incremental rendering (only repaint dirty regions).

### 8. Arena Allocator for Elements

**Files:** `arena.rs`

Custom arena: elements allocated per-frame, cleared at frame end. No allocation fragmentation. Chunks reused.

### 9. Global Pattern for Shared Config

**Files:** `global.rs`

```rust
pub trait Global: 'static {}
let colors = Colors::global(cx);
T::update_global(cx, |global, cx| { ... });
```

Globals with observer callbacks. Theme, palette, settings without deep parameter passing.

### 10. Focus System

Single `FocusHandle` per window. `handle.focus(cx)`, `handle.is_focused(cx)`. Keyboard routes to focused element first.

### 11. Testing Infrastructure

**Files:** `app/test_context.rs`

Full app instances without real window. Deterministic time, single-threaded executors, manual input injection. `#[gpui::test]` macro.

## What ori_term Should Adopt

### High Impact
1. **Entity system** — Move from `Arc<Mutex<Pane>>` to centralized `Entity<Pane>` owned by app. Eliminates deadlock risk.
2. **DispatchTree for input** — Build tree during render, route mouse/keyboard through capture/bubble phases. Automatic focus/precedence.
3. **Split layout from painting** — 3-pass (layout → prepaint → paint) enables caching and damage tracking.
4. **Scene struct** — Collect paint primitives, sort by z-order once. GPU consumes sorted scene.
5. **Element state via GlobalElementId** — Hover/drag state in map, not widget structs.

### Medium Impact
6. **StyleRefinement** — Partial style merging for ergonomic builder pattern
7. **Action/Keymap system** — Separate actions from handlers, enable runtime rebinding
8. **Global pattern** — Theme/settings without deep parameter passing
9. **Arena allocator** — For frame-scoped widget instances

### Lower Impact
10. BoundsTree for z-order computation
11. Visual test infrastructure
12. Cross-platform inspector

## Where ori_term Is Superior

1. Dedicated animation engine with spring physics (GPUI's is minimal)
2. Explicit controller decomposition (GPUI mixes everything in event handlers)
3. Visual state manager with declarative state groups
4. Already has Sense declarations (GPUI doesn't have equivalent)
