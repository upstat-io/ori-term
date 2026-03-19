---
section: "03"
title: "Scene Abstraction & Damage Tracking"
status: not-started
reviewed: false
goal: "Introduce a PaintScene abstraction between widget paint and GPU rendering that enables damage tracking (only repaint dirty regions) and automatic z-order sorting."
inspired_by:
  - "GPUI Scene (src/scene.rs) — flat primitive list, sorted by z-order, GPU backend consumes sorted scene"
  - "GPUI BoundsTree (src/bounds_tree.rs) — R-tree for z-order computation from overlap"
  - "makepad DrawList — instanced draw call batching, lazy redraw via redraw_id"
depends_on: []
sections:
  - id: "03.1"
    title: "PaintScene Primitive Types"
    status: not-started
  - id: "03.2"
    title: "Damage Region Tracking"
    status: not-started
  - id: "03.3"
    title: "Z-Order Computation"
    status: not-started
  - id: "03.4"
    title: "Integration with Existing DrawList"
    status: not-started
  - id: "03.5"
    title: "Completion Checklist"
    status: not-started
---

# Section 03: Scene Abstraction & Damage Tracking

**Status:** Not Started
**Goal:** A `PaintScene` struct that collects paint primitives from widgets, tracks dirty regions, and enables the GPU renderer to skip unchanged areas. Currently, our `DrawList` is a flat `Vec<DrawCommand>` with no damage tracking -- every frame repaints everything.

**Context:** Our existing `SceneCache` (`draw/scene_node/mod.rs`) caches per-widget draw commands and invalidates via containment tracking. The `compose_scene()` function (`draw/scene_compose/mod.rs`) uses `InvalidationTracker` (`invalidation/mod.rs`) to invalidate dirty nodes before redrawing. This is good but operates at the widget level -- if any widget in a subtree is dirty, the entire subtree repaints. A scene-level abstraction enables region-based damage: only repaint the pixel area that actually changed.

GPUI collects all paint operations into a `Scene` struct with typed primitive lists (quads, paths, text, sprites). At frame end, the scene is sorted by z-order and submitted to the GPU backend. This separation enables:
1. Damage comparison: diff this frame's scene against last frame's
2. Z-order sorting: no manual layer indices
3. Batching: group primitives by shader/texture for fewer draw calls

**Relationship to existing systems:**
- **SceneCache + compose_scene():** Widget-level caching and invalidation propagation. Decides WHICH widgets to redraw. PaintScene adds REGION-level damage on top: after compose_scene produces a DrawList, PaintScene wraps it and tracks which screen regions changed. These are complementary, not competing.
- **LayerTree + LayerAnimator compositor:** Handles per-LAYER opacity and transforms (overlay fade-in/out). PaintScene operates at the widget/primitive level WITHIN a single compositor layer. The GPU renderer will: (1) for each compositor layer, check if its PaintScene has damage, (2) if dirty, re-render the layer's texture from its PaintScene, (3) compose layers via the existing compositor. No changes to the compositor itself.

**Reference implementations:**
- **GPUI** `src/scene.rs`: `Scene { quads, paths, underlines, monochrome_sprites, ... }`. PaintOperation enum with StartLayer/EndLayer for clipping.
- **makepad** draw lists: Instance batching, `redraw_id` per widget, lazy redraw.

**Depends on:** None (can be built alongside existing DrawList).

---

## 03.1 PaintScene Primitive Types

**File(s):** `oriterm_ui/src/draw/paint_scene.rs` (~200 lines)

PaintScene wraps the existing DrawList rather than creating parallel primitive types. This preserves all existing DrawList functionality (clip stacks, translate stacks, layer bg for subpixel compositing) without reimplementation.

- [ ] Define `PaintScene` that adds scene-level metadata on top of `DrawList`:
  ```rust
  /// A frame's worth of paint output with scene-level metadata.
  ///
  /// Wraps the existing `DrawList` (which handles clipping, transforms,
  /// layer bg stacks) and adds z-order assignment and widget provenance
  /// tracking for damage comparison.
  pub struct PaintScene {
      /// The underlying draw commands (full DrawList with clip/translate stacks).
      pub draw_list: DrawList,
      /// Per-command metadata: which widget emitted it, z-order.
      pub metadata: Vec<PrimitiveMeta>,
      /// Widget IDs whose paint output is included (for damage diffing).
      pub widget_ids: HashSet<WidgetId>,
  }

  pub struct PrimitiveMeta {
      pub z_order: u32,
      pub widget_id: Option<WidgetId>,
      /// Bounding rect of this primitive (for region-based damage).
      pub bounds: Rect,
  }
  ```

- [ ] Implement builder methods on `PaintScene` that delegate to `DrawList` and record metadata:
  ```rust
  impl PaintScene {
      /// Pushes a rect to the inner DrawList and records metadata.
      pub fn push_rect(&mut self, rect: Rect, style: RectStyle, widget_id: Option<WidgetId>) {
          self.draw_list.push_rect(rect, style);
          self.metadata.push(PrimitiveMeta {
              z_order: self.next_z_order(),
              widget_id,
              bounds: rect,
          });
          if let Some(id) = widget_id {
              self.widget_ids.insert(id);
          }
      }
      // Same pattern for push_text, push_line, push_icon, push_image.
      // All 11 DrawCommand variants are handled: Rect, Line, Text, Image,
      // Icon, PushClip, PopClip, PushTranslate, PopTranslate, PushLayer, PopLayer.
  }
  ```

---

## 03.2 Damage Region Tracking

**File(s):** `oriterm_ui/src/draw/damage.rs` (~150 lines)

Damage tracking uses per-widget hash comparison to detect changes between frames. This catches all changes (color, text, bounds) at O(total_widgets) cost.

- [ ] Define `DamageTracker`:
  ```rust
  pub struct DamageTracker {
      /// Regions that changed since last frame.
      dirty_regions: Vec<Rect>,
      /// Per-widget hash of draw commands from last frame (for diff).
      prev_hashes: HashMap<WidgetId, u64>,
  }
  ```
- [ ] Implement frame diffing to determine changed regions:
  1. For each widget in the current `PaintScene.widget_ids`, hash its draw commands.
  2. Compare against `prev_hashes`. If different (or new widget), add the widget's
     bounding rect to `dirty_regions`. If a widget was in `prev_hashes` but not this
     frame, add its old bounding rect (widget removed = region dirty).
  3. Merge overlapping dirty rects to reduce region count.
- [ ] Provide query API for the GPU renderer:
  ```rust
  impl DamageTracker {
      pub fn is_region_dirty(&self, rect: Rect) -> bool {
          self.dirty_regions.iter().any(|r| r.intersects(rect))
      }
      pub fn dirty_regions(&self) -> &[Rect] { &self.dirty_regions }
  }
  ```

---

## 03.3 Z-Order Computation

Z-order is trivially the paint order counter -- a monotonically increasing `u32` assigned as each DrawCommand is pushed to PaintScene. No sorting is needed because paint order IS z-order (depth-first tree walk). Overlays are rendered to separate compositor layers (via existing LayerTree), so z-order is per-layer, not cross-layer.

- [ ] Assign z-order automatically via a monotonically increasing counter per `push_*` call on `PaintScene`
- [ ] Document that overlay widgets use separate compositor layers (no z-order interaction with the main widget tree)

---

## 03.4 Integration with Existing DrawList

Migration is non-breaking: PaintScene wraps DrawList, so widget paint code is unchanged.

- [ ] `PaintScene` wraps `DrawList` internally, so `DrawCtx` continues writing to
  `&mut DrawList` unchanged. The `PaintScene` is constructed at the frame level
  (not per-widget), and the `DrawList` inside it is passed to `DrawCtx`.
- [ ] Widget paint code is UNCHANGED -- all existing `ctx.draw_list.push_rect()` calls
  work as before. PaintScene metadata is recorded by the framework (in `compose_scene`
  or equivalent), not by individual widgets.
- [ ] GPU renderer migration path:
  1. **Phase 1:** `PaintScene` wraps `DrawList`. GPU renderer continues consuming
     `DrawList` as before. Damage tracking runs but results are LOGGED, not acted on.
  2. **Phase 2:** GPU renderer uses `DamageTracker.dirty_regions()` to scissor-clip
     rendering to dirty areas. Full functionality.
- [ ] Scene cache interop: `compose_scene()` continues to use `SceneCache` for
  widget-level caching. The `PaintScene` sits above `compose_scene`: it wraps the
  final `DrawList` output and adds damage metadata for the GPU layer.

---

## 03.5 Completion Checklist

- [ ] Tests for PaintScene and DamageTracker follow sibling tests.rs pattern
- [ ] `PaintScene` wraps `DrawList` and records per-command metadata (z-order, widget_id, bounds)
- [ ] All 11 `DrawCommand` variants handled (Rect, Line, Text, Image, Icon, PushClip, PopClip, PushTranslate, PopTranslate, PushLayer, PopLayer)
- [ ] `DamageTracker` identifies changed regions between frames via per-widget hash comparison
- [ ] Z-order assigned automatically from paint order (monotonic counter)
- [ ] Existing widgets work without modification -- `DrawCtx` writes to `DrawList` inside `PaintScene`
- [ ] Relationship to `SceneCache` / `compose_scene` documented and tested
- [ ] Relationship to `LayerTree` compositor documented
- [ ] `./build-all.sh` and `./test-all.sh` pass
- [ ] `./clippy-all.sh` clean

**Exit Criteria:** `PaintScene` wraps the `DrawList` from a widget tree paint pass, `DamageTracker` computes dirty regions by diffing per-widget hashes against the previous frame, and the GPU renderer can query `dirty_regions()` to restrict rendering to changed areas. All existing widget paint code continues to work without modification.
