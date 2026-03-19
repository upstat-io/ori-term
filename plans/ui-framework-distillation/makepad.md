# makepad Deep-Dive: GPU-Native Live Design Framework

Makepad takes a **radically different approach** — GPU-native from the ground up with a custom script DSL for live design. Most relevant for GPU rendering patterns.

## Architecture Summary

GPU-native rendering via **instanced draw calls**. Turtle-based incremental layout. Declarative **Animator state machines** drive shader uniforms. Area-based hit testing tied to draw regions. LiveId (FNV-1a hashed u64) for all identifiers. Custom script DSL for widget declaration, shaders, and animation.

## Key Patterns

### 1. Instanced GPU Rendering

**Files:** `/draw/src/draw_list_2d.rs`, `/draw/src/cx_2d.rs`

Widgets don't make individual draw calls. They append **instance data** (C-repr structs) to a draw list. All instances of the same shader rendered in ONE GPU draw call.

```rust
#[repr(C)]
pub struct DrawQuad {
    pub rect_pos: Vec2f,
    pub rect_size: Vec2f,
    pub draw_clip: Vec4f,
    pub draw_depth: f32,
}
```

Button doesn't "create a quad" — it sets `self.draw_bg.rect_pos/size` and calls `draw()` which appends to the pooled buffer.

**Performance:** O(1) draw calls for any number of widgets using the same shader. GPU-native batching.

**Directly applicable to ori_term:** Terminal grid cells could be instanced — 5000 cells rendered in one draw call instead of per-cell.

### 2. Turtle Layout (Incremental Walk)

**Files:** `/draw/src/turtle.rs`

Instead of measure-then-position two-pass, the Turtle is a **stateful cursor**:

```rust
pub struct Walk {
    pub abs_pos: Option<Vec2d>,
    pub margin: Inset,
    pub width: Size,   // Fill | Fit | Fixed(f64)
    pub height: Size,
}
```

As you "walk" widgets, the turtle computes their final rectangles immediately. Layout is incremental, not deferred.

### 3. Animator State Machines → Shader Uniforms

**Files:** `/widgets/src/animator.rs`

Animators are **declarative state machines**, not property animators:

```
animator: Animator {
    hover: {
        default: @off
        off: { from: {all: Forward {duration: 0.1}}, apply: { draw_bg: {hover: 0.0} } }
        on:  { from: {all: Forward {duration: 0.1}}, apply: { draw_bg: {hover: 1.0} } }
    }
}
```

On `FingerHoverIn` → `animator_play(cx, ids!(hover.on))`. Animator interpolates shader uniforms. GPU does the blending:

```glsl
get_color: fn() {
    return self.color.mix(self.color_hover, self.hover)
}
```

**Key insight:** Animations aren't side-car computations — they're baked into state transitions. The state machine drives both the visual transition and shader uniforms simultaneously.

### 4. Area-Based Hit Testing

Every drawn region produces an `Area` (draw_list_id + rect). Events check `event.hits(cx, self.area)` — returns `Hit` enum:

```rust
pub enum Hit {
    FingerDown(FingerDownEvent),
    FingerMove(FingerMoveEvent),
    FingerHoverIn(FingerHoverEvent),
    FingerHoverOut(FingerHoverEvent),
    FingerUp(FingerUpEvent),
    ...
}
```

Hit testing is baked into event handling — no separate interaction manager.

### 5. LiveId — Zero-Allocation Identifiers

**Files:** `/libs/live_id/src/live_id.rs`

`LiveId(u64)` = FNV-1a hash of identifier string. No heap strings anywhere. Global interner for reverse mapping (debug only).

### 6. UID-Based Widget Registry

**Files:** `/widgets/src/widget_tree.rs`

`WidgetUid` (AtomicU64 counter). Widget tree maintains `HashMap<WidgetUid, GraphNode>`. Queries by path: `root.child("name").child("subname")` → BFS lookup.

Weak refs + UIDs. Survives redraws, O(log n) lookup.

### 7. Draw List Pooling + Lazy Redraw

Draw lists reused across frames. Widget's `draw_bg` instance data cleared and re-appended, not reallocated. `redraw_id` tracks if widget was touched this frame. Untouched = reuse draw list as-is.

### 8. Terminal Widget

**Files:** `/libs/terminal_core/src/`

Makepad has a full terminal emulator widget! VT100 parser, grid storage, PTY management. Terminal is just another widget — implements `Widget`, responds to events. Grid cells rendered as glyphs via `DrawGlyph` shader.

### 9. Script DSL for Everything

Widget declaration, shader code, animation config, and theme all in the same DSL:

```
mod.widgets.ButtonFlat = set_type_default() do mod.widgets.ButtonBase {
    text: "Button"
    width: Fit
    draw_text +: {
        color: theme.color_label_inner
        get_color: fn() { return self.color.mix(self.color_hover, self.hover) }
    }
}
```

**Strengths:** Hot-reloadable, unified declaration.
**Weakness:** Custom language, steep learning curve, tooling dependency.

## What ori_term Should Adopt

### High Impact
1. **Instanced rendering for grid cells** — Single draw call for thousands of cells via instance buffers. `#[repr(C)]` cell struct → GPU memory directly.
2. **State machine animators** — Declarative state transitions that drive shader uniforms, not separate animation engine ticking.
3. **Draw list pooling** — Reuse GPU buffers across frames, only update changed instance data.
4. **Lazy redraw** — Track per-widget `redraw_id`, skip untouched widgets.

### Medium Impact
5. **Area-based hit testing** — Tie interaction regions to drawn areas naturally.
6. **UID-based widget lookup** — Stable, hash-friendly, survives redraws.
7. **FNV-1a hashed identifiers** — Zero-allocation LiveId pattern for widget/style/action names.

### Design Insights
- GPU is the primary compute target, CPU orchestrates but doesn't do the heavy work
- Animations belong in the state machine, not a separate engine
- Hit testing should be a natural consequence of drawing, not a parallel system
- Incremental layout (turtle) can be simpler than two-pass measure+position

## Where ori_term Is Superior

1. Standard Rust throughout (no custom DSL)
2. Cross-platform without custom shader compiler
3. Explicit controller decomposition (makepad mixes everything in event handlers)
4. Formal event propagation pipeline (capture/bubble phases)
5. More principled separation of concerns
