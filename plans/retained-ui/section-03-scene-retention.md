---
section: "03"
title: "Scene Retention"
status: complete
goal: "Unchanged widget subtrees reuse cached draw command sequences — only dirty subtrees rebuild their DrawList segment. The final frame is composed from retained + rebuilt pieces."
inspired_by:
  - "Flutter Scene / Layer compositing (compositor_context.cc)"
  - "Chromium PaintController / DisplayItemList (paint/paint_controller.cc)"
depends_on: ["02"]
sections:
  - id: "03.1"
    title: "SceneNode Abstraction"
    status: complete
  - id: "03.2"
    title: "Per-Widget DrawList Caching"
    status: complete
  - id: "03.3"
    title: "Scene Composition"
    status: complete
  - id: "03.4"
    title: "Container Draw with Selective Rebuild"
    status: complete
  - id: "03.5"
    title: "Dialog and Chrome Integration"
    status: complete
  - id: "03.6"
    title: "Completion Checklist"
    status: complete
reviewed: true
---

# Section 03: Scene Retention

**Status:** Not Started
**Goal:** Each widget subtree owns a cached draw command sequence. When the subtree is unchanged (per the InvalidationTracker from Section 02), its cached commands are reused verbatim. Only dirty subtrees regenerate their DrawList segment. A composition pass merges cached + rebuilt segments into the final DrawList for GPU conversion.

**Context:** Today `DrawList::clear()` is called at the top of every render pass (dialog_rendering.rs:58, redraw/draw_helpers.rs:48 for chrome). Every widget then re-emits its entire draw command sequence via `Widget::draw()`. This means a Settings dialog with 70 widgets emits ~300 draw commands per frame even when nothing changed. With Section 02's invalidation tracking, we know which widgets are dirty. This section makes the framework act on that knowledge by skipping `draw()` for clean subtrees and replaying their cached commands instead.

**Reference implementations:**
- **Flutter** `flow/layers/container_layer.cc`: Each `Layer` caches its `DisplayList`. Composition merges layers. Dirty layers rebuild; clean layers replay.
- **Chromium** `cc/paint/paint_controller.cc`: `CachedDisplayItems` per client. Subsequences are cached and replayed when the client reports no changes.

**Depends on:** Section 02 (InvalidationTracker tells us which subtrees are dirty).

**Co-implementation requirement with Section 02:** These sections must land together. Without scene retention, the InvalidationTracker just gates the top-level render decision — individual widgets still all draw. Without invalidation, scene retention has no signal for what to rebuild.

---

## 03.1 SceneNode Abstraction

**File(s):** new `oriterm_ui/src/draw/scene_node.rs`

**Module registration:** Add `mod scene_node;` to `oriterm_ui/src/draw/mod.rs` (after `mod shadow;` at line 10). Add `pub use scene_node::SceneNode;` to the re-export block (after line 16). Without this, the new file won't compile.

A `SceneNode` is a cached draw command slice for one widget subtree. It stores the DrawList segment range (start..end indices) from the last successful draw, along with the bounds that produced it.

- [x] Define `SceneNode`:
  ```rust
  /// Cached draw output for a widget subtree.
  pub struct SceneNode {
      /// Widget that owns this cache.
      widget_id: WidgetId,
      /// Cached draw commands from the last draw.
      commands: Vec<DrawCommand>,
      /// Bounds that produced these commands (layout output).
      bounds: Rect,
      /// Whether this cache is valid.
      valid: bool,
  }

  impl SceneNode {
      pub fn new(widget_id: WidgetId) -> Self { ... }
      pub fn is_valid(&self) -> bool { self.valid }
      pub fn invalidate(&mut self) { self.valid = false; }
      pub fn update(&mut self, commands: Vec<DrawCommand>, bounds: Rect) { ... }
      pub fn commands(&self) -> &[DrawCommand] { &self.commands }
      pub fn bounds(&self) -> Rect { self.bounds }
  }
  ```

- [x] `SceneNode` does NOT own its children -- the widget tree already provides the hierarchy. The scene node is a flat per-widget cache, not a parallel tree. This avoids maintaining two trees.

- [x] **Clone cost awareness:** `SceneNode::commands()` returns `&[DrawCommand]`. When replaying cached commands, the caller appends them to the output `DrawList`. This requires cloning each `DrawCommand`, including `DrawCommand::Text { shaped: ShapedText { glyphs: Vec<ShapedGlyph> } }`. For a widget with 5 text commands each containing 20 glyphs, that's 5 vec clones of ~20 elements each. This is still much cheaper than re-shaping (rustybuzz is ~10x more expensive than a vec clone), but consider extending `DrawList` with `extend_from_slice()` that clones in bulk. If Section 01's decision is option (b) (`Rc<ShapedText>` in `DrawCommand`), the clone cost becomes trivial (`Rc::clone`).

---

## 03.2 Per-Widget DrawList Caching

**File(s):** `oriterm_ui/src/widgets/mod.rs`, `oriterm_ui/src/draw/draw_list.rs`

Each widget that participates in scene retention needs a `SceneNode`. For leaf widgets (Label, Button, Checkbox, etc.), the node caches their individual draw commands. For containers, the node caches the composite output of all children.

- [x] Add `SceneNode` to `DrawCtx`:
  ```rust
  pub struct DrawCtx<'a> {
      // ...existing fields...
      /// Scene cache for the current widget. `None` during uncached draws.
      scene_cache: Option<&'a mut HashMap<WidgetId, SceneNode>>,
  }
  ```

- [x] The scene cache is a flat `HashMap<WidgetId, SceneNode>` owned by the host (`WindowContext` or `DialogWindowContext`), passed into the draw context. Widgets don't need to know about caching — the container draw logic checks the cache.

- [x] Alternative: put the cache on `ContainerWidget` instead of `DrawCtx`. **Trade-off:** Container ownership is simpler but means leaf widgets at the root (not inside a container) can't be cached. Since all UI is composed via containers, this is acceptable. **Recommendation:** Cache on the host context, passed via `DrawCtx`, for maximum flexibility.

- [x] **Widget identity in draw context:** `DrawCtx` (widgets/mod.rs:197) does not currently carry a `WidgetId`. The `compose_scene` function needs to know which widget is being drawn to look up its `SceneNode`. Two options:
  - **(a) `ContainerWidget::draw()` performs the cache lookup** (recommended): The container already knows each child's `id()`. It checks `scene_cache[child.id()]` before calling `child.draw()`. This keeps `draw()` signatures unchanged — the cache check is in the container, not in individual widgets.
  - **(b) Add `widget_id: WidgetId` to `DrawCtx`**: Each widget sets it before drawing. More general but adds a field to every draw context.
  - **Recommendation:** Option (a) — containers drive the cache. `compose_scene()` is called by `ContainerWidget::draw()`, not by leaf widgets. Leaf widgets don't need to know about caching.

---

## 03.3 Scene Composition

**File(s):** new `oriterm_ui/src/draw/scene_compose.rs`

**Module registration:** Add `mod scene_compose;` to `oriterm_ui/src/draw/mod.rs`. Add `pub use scene_compose::compose_scene;` to re-exports.

A function that builds the final `DrawList` by replaying cached segments for clean subtrees and collecting new segments for dirty subtrees.

- [x] Define `compose_scene()`:
  ```rust
  /// Compose a final DrawList from a widget tree using scene caches.
  ///
  /// For each widget in the tree:
  /// - If the widget's SceneNode is valid and bounds match, append
  ///   cached commands to the output.
  /// - If invalid or bounds changed, call widget.draw() into a
  ///   temporary DrawList, store it in the SceneNode, and append
  ///   to the output.
  pub fn compose_scene(
      root: &dyn Widget,
      ctx: &mut DrawCtx<'_>,
      tracker: &InvalidationTracker,
      cache: &mut HashMap<WidgetId, SceneNode>,
  ) -> DrawList { ... }
  ```

- [x] Clip state must be correctly managed across cached/uncached boundaries. If a parent container has `clip_children: true`, the `PushClip`/`PopClip` commands must be emitted even when replaying cached child content. Solution: the parent's scene node includes the clip commands wrapping the children.

- [x] Layer stack (`PushLayer`/`PopLayer`) must similarly be maintained. The bg_hint for subpixel text compositing depends on the layer stack state, which is baked into the cached `DrawCommand::Text` entries. This is correct because the layer push happens in the parent's draw, and the child text already captured the bg_hint at push_text time.

- [x] **Background color invalidation:** If a parent widget changes its background color (e.g. container hover changes its `PushLayer` bg color), all child text commands' `bg_hint` values become stale — they were captured from the old layer stack. When a container's paint state changes and it pushes a different `PushLayer { bg }`, all children's scene nodes must be invalidated (not just the container itself). This is a paint-level invalidation that cascades to children.
  - Rule: `DirtyKind::Paint` on a widget that emits `PushLayer` must invalidate all descendant scene nodes, not just the widget's own node.
  - Implementation: `compose_scene` checks whether the parent's `PushLayer` bg matches the bg at the time children were cached. Mismatch → invalidate children.

---

## 03.4 Container Draw with Selective Rebuild

**File(s):** `oriterm_ui/src/widgets/container/mod.rs`

**File size warning:** `container/mod.rs` is currently 413 lines. The cache-check logic added to `draw()` will add approximately 20-30 lines. Monitor carefully -- if the file approaches 480 lines, extract the scene-aware draw path into a `container/scene_draw.rs` submodule.

The `ContainerWidget::draw()` method (container/mod.rs:330-363) currently iterates all children and calls `child.draw()` for each. Modify this to skip children whose `SceneNode` is valid.

- [x] In `ContainerWidget::draw()`, for each child:
  1. Check if `scene_cache[child.id()]` is valid and `bounds` matches.
  2. If valid: append cached commands to `ctx.draw_list`.
  3. If invalid: call `child.draw()` into a temporary `DrawList`, store in cache, append to output.

- [x] The visibility culling already in `draw()` (container/mod.rs:343 — `if !child_node.rect.intersects(visible_bounds)`) continues to work — culled children never enter the cache check.

- [x] `needs_layout` and `needs_paint` flags on `ContainerWidget` (container/mod.rs:50-52) should invalidate the corresponding children's `SceneNode`s. When `needs_layout` is true, all children's nodes are invalidated (because positions changed). When only `needs_paint` is true, only the specific dirty child's node is invalidated.

---

## 03.5 Dialog and Chrome Integration

**File(s):** `oriterm/src/app/dialog_rendering.rs`, `oriterm/src/app/redraw/draw_helpers.rs`

Wire scene composition into the actual render paths.

- [x] `render_dialog()` — Replace the current pattern:
  ```rust
  // Before:
  ctx.draw_list.clear();
  ctx.chrome.draw(&mut draw_ctx);
  ctx.content.content_widget().draw(&mut draw_ctx);

  // After:
  ctx.draw_list.clear();
  compose_scene(&ctx.chrome, &mut draw_ctx, &tracker, &mut ctx.scene_cache);
  compose_scene(ctx.content.content_widget(), &mut draw_ctx, &tracker, &mut ctx.scene_cache);
  ```

- [x] Chrome rendering in `draw_tab_bar()` — The tab bar is a single `TabBarWidget` with child tab items. Its scene node caches the entire tab bar draw output. Only invalidated when tabs change (add, remove, rename, reorder, switch) or on hover/drag. Static tab bar across frames → zero draw calls.

- [x] Overlay rendering in `draw_overlays()` — Each overlay is a separate scene. Overlay open/close/animation invalidates the overlay's node. Stable overlays (e.g. open dropdown list without hover change) reuse cached commands.

- [x] Add `scene_cache: HashMap<WidgetId, SceneNode>` to `WindowContext` and `DialogWindowContext`. Clear on resize, theme change, font change (same triggers as text cache invalidation from Section 01).

- [x] **Sync point:** Adding `scene_cache` field requires updating:
  - `WindowContext::new()` (window_context.rs:95) — initialize `HashMap::new()`
  - `DialogWindowContext::new()` (dialog_context/mod.rs:135) — initialize `HashMap::new()`
  - `DialogWindowContext::resize_surface()` (dialog_context/mod.rs:171) — clear scene cache on resize
  - All theme/font change handlers that clear caches — must also clear scene cache

---

## 03.6 Completion Checklist

**Tests:** `scene_node.rs` and `scene_compose.rs` should each have sibling test files. Convert each to a directory module if tests are substantial, or add `#[cfg(test)] mod tests;` with sibling `tests.rs` files. Specifically:
- `scene_node/mod.rs` + `scene_node/tests.rs` — test `SceneNode` invalidation, update, bounds tracking
- `scene_compose/mod.rs` + `scene_compose/tests.rs` — test compose with all-clean, all-dirty, mixed, nested clips, nested layers

- [x] `SceneNode` is defined and exported from `oriterm_ui::draw`
- [x] `compose_scene()` correctly handles clip and layer stack across cached/uncached boundaries
- [x] `ContainerWidget::draw()` skips clean children and replays cached commands
- [x] `render_dialog()` uses scene composition — verified by counting `Widget::draw()` calls
- [x] Tab bar uses scene composition — static tab bar produces zero draw calls after first frame
- [x] Overlay rendering uses scene composition
- [x] Scene cache invalidates correctly on resize, theme change, font change
- [x] Scene cache invalidates correctly when `InvalidationTracker` marks a widget dirty
- [x] `DrawList` output is identical whether rendered via scene composition or full rebuild (behavioral equivalence test)
- [x] `./build-all.sh` green
- [x] `./clippy-all.sh` green
- [x] `./test-all.sh` green

**Exit Criteria:** Hovering a single button in the Settings dialog calls `Widget::draw()` on exactly that button (and its container chain for clip/layer management). All other widgets' draw commands are replayed from cache. Verified by instrumenting `Widget::draw()` with a call counter.
