# egui Deep-Dive: Immediate-Mode GPU UI

## Architecture Summary

egui is an **immediate-mode** framework (~170k LOC). No retained widget tree — widgets are stateless builders, state lives in a global `Memory` map keyed by `Id`. Each frame: accumulate shapes → tessellate to GPU meshes → render.

## Key Patterns

### 1. IdTypeMap — Heterogeneous Per-Widget State

**Files:** `src/memory/mod.rs`, `src/util/id_type_map.rs`

State stored in `Memory.data: IdTypeMap` — a `HashMap<(Id, TypeId), Box<dyn Any>>`. Widgets don't hold state; they look it up by ID each frame.

- `Id` is `NonZeroU64` (niche optimization: `Option<Id>` is still 8 bytes)
- IDs derived via hashing: `Id::new("label")`, `parent_id.with("child")`
- Generation counters for GC of stale entries

**Strengths:** Zero per-widget overhead, elegant persistence (serialize Memory to disk), type-safe downcast.
**Weaknesses:** Global state pollution, ID collisions silent, no structural relationships.

### 2. Sense-Based Interaction Declaration

**Files:** `src/sense.rs`, `src/hit_test.rs`, `src/interaction.rs`, `src/response.rs`

Widgets declare capabilities via `Sense` bitflags (`CLICK | DRAG | FOCUSABLE`). Framework computes `InteractionSnapshot` from hit test + previous state. Widgets get a `Response` object with `.hovered()`, `.clicked()`, `.dragged()`.

Hit testing pipeline: transform pointer → find widgets within search_radius → prune by layer occlusion → separate hits for hover/click/drag.

**Strengths:** Declarative, centralized hit testing, simple state machine (pure data, no callbacks).
**Weaknesses:** Single pointer per frame, frame-latency on drag detection.

### 3. Implicit Frame-Scoped Animation

**Files:** `src/animation_manager.rs`

`ctx.animate_bool(id, duration, condition)` → returns 0.0..1.0. State stored in `IdMap`. If condition changes mid-animation, smoothly continues from current position. No explicit animator objects.

**Strengths:** Zero boilerplate, smooth transitions, no allocation.
**Weaknesses:** Linear only (no easing/spring), no animation groups, no coordination.

### 4. One-Pass Layout (Placer + Region)

**Files:** `src/layout.rs`, `src/placer.rs`, `src/ui.rs`

`Ui` wraps a `Placer` with a directional cursor. Layout is one-pass: constraints flow down (max width), sizes flow up (child reports size). Direction enum: `LeftToRight`, `TopDown`, etc. Second pass only for first-shown windows.

**Weaknesses:** No flex weights, no aspect ratio, wrapping is heuristic.

### 5. Shape → Tessellator → Mesh → GPU

**Files:** `crates/epaint/src/tessellator.rs`, `crates/epaint/src/mesh.rs`

Painter accumulates `Shape` enums (Rect, Circle, Path, Text). Tessellator converts to triangle meshes per texture. Re-tessellated every frame (no caching).

### 6. Frame Cache

**Files:** `src/cache/frame_cache.rs`

Expensive computations cached by key hash, evicted if not used this frame. Good for text shaping, glyph rasterization.

### 7. Nohash Optimization

`IdMap<V>` = `nohash_hasher::IntMap<Id, V>`. Since IDs are already high-entropy u64, skip double-hashing.

## Testing

`egui_kittest` crate: headless `Harness`, visual snapshot regression testing, queryable widgets by ID/text/role.

## What ori_term Should Adopt

1. **FrameCache pattern** for expensive computations (text shaping, layout) — cache by key, evict stale
2. **Niche-optimized IDs** (`NonZeroU64`) for `Option<WidgetId>` = 8 bytes
3. **Builder pattern with immutable self** for controller configuration
4. **Visual snapshot testing** for widget regression

## Where ori_term Is Superior

1. GPU mesh caching (retained, not re-tessellated per frame)
2. Spring physics and easing curves
3. Modular interaction controllers
4. Property transactions for atomic updates
5. Damage tracking to minimize GPU work
