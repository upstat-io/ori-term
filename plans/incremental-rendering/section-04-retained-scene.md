---
section: "04"
title: "Retained Scene & Dirty Regions"
status: not-started
reviewed: true
goal: "Scene retains per-widget paint output across frames. Only dirty widgets repaint their scene fragments; clean widgets reuse their previous output. Full scene.clear() is eliminated."
inspired_by:
  - "Druid retained render tree (per-widget paint caching)"
  - "GPUI (Zed) element painting (dirty tracking per element)"
  - "Flutter RepaintBoundary (subtree paint caching)"
depends_on: ["02"]
sections:
  - id: "04.1"
    title: "Per-Widget Scene Fragments"
    status: not-started
  - id: "04.2"
    title: "Fragment Storage"
    status: not-started
  - id: "04.3"
    title: "Scene Patching"
    status: not-started
  - id: "04.4"
    title: "Dirty Region Collection"
    status: not-started
  - id: "04.5"
    title: "Tests"
    status: not-started
  - id: "04.6"
    title: "Completion Checklist"
    status: not-started
---

# Section 04: Retained Scene & Dirty Regions

**Status:** Not Started
**Goal:** The Scene is no longer cleared and rebuilt from scratch every frame. Instead, each widget's paint output is stored as a "fragment" keyed by widget ID. On subsequent frames, only dirty widgets repaint their fragments; clean widgets' fragments are reused verbatim. The Scene becomes a retained data structure that is patched, not rebuilt.

**Context:** Currently, `App::compose_dialog_widgets()` calls `scene.clear()` then `widget.paint()` on the entire tree. This discards ALL previous paint output and rebuilds it from scratch — even if only one widget's hover color changed. For a dialog with 37+ widgets and 200+ primitives, this is wasteful. The `DamageTracker` (`oriterm_ui/src/draw/damage/mod.rs`) already hashes per-widget primitives (grouped by `Scene::current_widget_id`) and compares against the previous frame — but this is post-hoc damage detection AFTER the full repaint, not a pre-repaint optimization that avoids redundant paint calls.

**Reference implementations:**
- **Druid** `druid/src/core.rs`: `InternalLifeCycle::RouteWidgetAdded` + `should_propagate_to_hidden()` gates lifecycle propagation. The `needs_paint` flag on widget state controls repaint gating.
- **Flutter** `rendering/proxy_box.dart`: `RepaintBoundary` creates a separate compositing layer for subtrees, enabling independent repaint
- **GPUI (Zed)** `crates/gpui/src/window.rs`: `paint()` only called on elements with `PrepaintStateIndex` changes; `reuse_prepaint()` skips unchanged ranges

**Depends on:** Section 02 (per-widget dirty set provides the "which widgets changed" signal).

**Scope clarification:** This section optimizes **hover, focus, and animation** frame times — cases where one widget's visual state changes but nothing moves. It does NOT optimize **scroll** frame times, because scroll changes every visible widget's absolute position (baked into primitives at paint time), invalidating all fragments. Scroll optimization requires Section 05 (GPU-side texture blit). The retained scene still provides the infrastructure Section 05 builds on.

---

## 04.1 Per-Widget Scene Fragments

**File(s):** `oriterm_ui/src/draw/scene/mod.rs` (struct definition), new `oriterm_ui/src/draw/scene/fragment.rs` (fragment tracking), new `oriterm_ui/src/draw/fragment_cache.rs` (cache)

A "fragment" is the contiguous slice of Scene primitives produced by a single widget's `paint()` call. Currently, every primitive already carries `widget_id: Option<WidgetId>` (set by `DrawCtx::for_child()` → `Scene::set_widget_id()`), but they are interleaved in the Scene arrays with no way to extract a widget's contribution efficiently. To support per-widget caching, we need contiguous range tracking.

- [ ] Add fragment tracking to Scene:
  ```rust
  /// A contiguous range of primitives in the Scene's typed arrays,
  /// produced by a single widget's paint() call.
  #[derive(Debug, Clone)]
  pub struct SceneFragment {
      pub widget_id: WidgetId,
      pub quads: Range<usize>,
      pub text_runs: Range<usize>,
      pub lines: Range<usize>,
      pub icons: Range<usize>,
      pub images: Range<usize>,
  }
  ```

- [ ] Track fragment boundaries during paint:
  ```rust
  impl Scene {
      /// Begin a new fragment for the given widget.
      /// Records the current lengths of all primitive arrays.
      pub fn begin_fragment(&mut self, widget_id: WidgetId) { ... }

      /// End the current fragment, returning it.
      /// The fragment spans from the begin snapshot to current lengths.
      pub fn end_fragment(&mut self) -> SceneFragment { ... }
  }
  ```

- [ ] Integrate with `DrawCtx::for_child()` -- begin/end fragment around each widget's paint call.
  **Important:** `DrawCtx` is a plain struct (not Drop-based). Fragment begin/end must be explicit, not RAII. Options:
  (a) **Explicit calls in ContainerWidget::paint():** Call `scene.begin_fragment(child_id)` before `child.paint()` and `scene.end_fragment()` after.
  (b) **Wrapper method on DrawCtx:** Add `DrawCtx::paint_child()` that wraps begin/end around the child's paint call.
  (c) **Paint-with-fragment closure:** `scene.with_fragment(id, || child.paint(&mut ctx))`.
  **Recommended: Option (b)** — keeps the API clean and prevents mismatched begin/end.
  ```rust
  impl DrawCtx<'_> {
      /// Paint a child widget, tracking its Scene fragment.
      pub fn paint_child(&mut self, child: &dyn Widget, child_id: WidgetId, bounds: Rect) {
          self.scene.begin_fragment(child_id);
          let mut child_ctx = self.for_child(child_id, bounds);
          child.paint(&mut child_ctx);
          self.scene.end_fragment();
      }
  }
  ```

---

### Stack State and ContentMask

**Critical design constraint:** The Scene resolves clip/offset stacks into each primitive's `ContentMask` at push time (see `scene/paint.rs` -- `push_quad`, `push_text`, etc. apply `self.apply_offset(bounds)` and `self.current_content_mask()`). This means:

1. **Good news:** Cached fragments are self-contained — no stack state restoration needed for replay.
2. **Bad news:** Fragments contain ABSOLUTE positions and clip rects. If a widget's position changes (scroll, resize, layout shift), its cached fragment is invalid. Scroll offset changes invalidate ALL visible content fragments because the scroll offset is baked into every primitive via `push_offset()`.
3. **Implication for scroll:** Fragment caching helps for hover/focus changes (widget doesn't move, only colors change) but does NOT help for scroll (every visible widget moves). GPU-side scroll (Section 05) is needed to avoid repainting during scroll.

- [ ] Document this constraint in the fragment cache: fragments are valid only when the widget's position hasn't changed.
- [ ] On scroll offset change: invalidate all content fragments (fall back to full content repaint, or defer to Section 05's GPU texture approach).
- [ ] On hover/focus change: only invalidate the affected widget's fragment (position stable, only visual state changed).

---

## 04.2 Fragment Storage

**File(s):** `oriterm_ui/src/draw/fragment_cache.rs` (new file)

Store the previous frame's fragments so clean widgets can reuse them.

- [ ] Add a `FragmentCache` that stores per-widget primitive snapshots:
  ```rust
  pub struct FragmentCache {
      /// Previous frame's fragments, keyed by widget ID.
      fragments: HashMap<WidgetId, CachedFragment>,
  }

  // All 5 typed arrays from Scene (quads, text_runs, lines, icons, images).
  struct CachedFragment {
      quads: Vec<Quad>,
      text_runs: Vec<TextRun>,
      lines: Vec<LinePrimitive>,
      icons: Vec<IconPrimitive>,
      images: Vec<ImagePrimitive>,
  }
  ```

- [ ] After each frame, snapshot dirty widgets' fragments into the cache
- [ ] Evict fragments for widgets that no longer exist (deregistered)

---

## 04.3 Scene Patching

**File(s):** `oriterm_ui/src/draw/scene/mod.rs`

Instead of `scene.clear()` + full repaint, the new pipeline:
1. For each dirty widget: remove its old fragment, call `paint()` to produce new fragment
2. For each clean widget: copy its cached fragment into the Scene
3. Maintain correct z-order (paint order) across fragments

- [ ] Implement `Scene::patch_widget()`:
  ```rust
  /// Replace a widget's fragment in the Scene.
  /// Removes the old fragment's primitives and inserts the new ones.
  pub fn patch_widget(&mut self, widget_id: WidgetId, fragment: CachedFragment) { ... }
  ```

- [ ] Handle z-ordering: fragments must appear in paint order. Options:
  (a) **Append + sort** — append all fragments, sort by paint order index
  (b) **Stable slots** — pre-allocate slots per widget in paint order, overwrite in place
  (c) **Rebuild from cache** — assemble the full Scene from cached fragments + freshly painted dirty fragments in paint order

  **Recommended: Option (c)** — simplest, still avoids redundant paint calls. The Scene is "assembled" from fragments rather than "painted from scratch."

- [ ] Fallback: if too many widgets are dirty (>50%), fall back to full repaint (simpler, avoids overhead of fragment management)

- [ ] **Do NOT use `build_scene()`** for the retained path. `build_scene()` calls `scene.clear()` unconditionally (it's the immediate-mode entry point). The retained path assembles the scene from fragments. `compose_dialog_widgets()` already paints directly (not through `build_scene`), so this is consistent — but any future callers that use `build_scene()` must be updated.

**WARNING: High complexity.** This is the hardest design decision in Section 04.

- [ ] **ContainerWidget::paint() and nested widgets:** Fragment tracking must work recursively. When ContainerWidget paints its children, each child gets its own fragment. But the ContainerWidget itself may also push primitives (background, border) before/after children. The ContainerWidget's "own" fragment is the primitives it pushes directly, while child fragments are separate. This means fragment tracking is hierarchical, not flat — a widget's fragment excludes its children's fragments.

- [ ] **Design decision: flat vs hierarchical fragments.** Two options:
  - **(a) Exclusive ranges:** A container's fragment contains ONLY its own primitives (background, borders), excluding children. Requires `begin_fragment` for parent before children, `pause_fragment` before each child, `resume_fragment` after each child, `end_fragment` after all children. More complex but enables fine-grained caching.
  - **(b) Leaf-only caching:** Only cache fragments for LEAF widgets (widgets with no children). Containers always repaint their own primitives but skip clean children's paint calls. Simpler, still captures most benefit since leaves (labels, icons, color swatches) are the majority of paint work.
  - **Recommended: Option (b)** for initial implementation — simpler, lower risk, captures ~80% of the benefit. Upgrade to option (a) only if profiling shows container self-painting is a bottleneck.

---

## 04.4 Dirty Region Collection

**File(s):** `oriterm_ui/src/draw/mod.rs`, `oriterm_ui/src/invalidation/mod.rs`

Collect the bounding rects of dirty widgets' fragments to produce damage rects for the GPU.

- [ ] After scene patching, compute damage rects:
  ```rust
  pub fn compute_damage_rects(
      dirty_widgets: &HashSet<WidgetId>,
      fragment_cache: &FragmentCache,
      bounds_map: &HashMap<WidgetId, Rect>,
  ) -> Vec<Rect> {
      dirty_widgets.iter()
          .filter_map(|id| bounds_map.get(id))
          .copied()
          .collect()
  }
  ```

- [ ] Merge overlapping damage rects to reduce GPU overdraw
- [ ] Pass damage rects to GPU renderer (future: scissor rect per damage region)

---

## 04.5 Tests

**File(s):** `oriterm_ui/src/draw/scene/tests.rs` (fragment tracking tests), `oriterm_ui/src/draw/fragment_cache/tests.rs` (cache tests)

**Note:** If `fragment_cache.rs` needs tests, convert it to `fragment_cache/mod.rs` + `fragment_cache/tests.rs` per the sibling test file convention.

- [ ] Test: `fragment_tracking_records_widget_primitives` — paint a button, verify fragment contains its quads + text runs
- [ ] Test: `clean_widget_reuses_cached_fragment` — paint, mark clean, assemble scene, verify same primitives without calling paint()
- [ ] Test: `dirty_widget_repaints_fragment` — paint, mark dirty, assemble scene, verify paint() is called and new primitives replace old
- [ ] Test: `scene_assembly_preserves_paint_order` — paint A, B, C; dirty B; assemble; verify A before B before C in scene
- [ ] Test: `fragment_eviction_on_deregister` — deregister widget, verify its fragment is removed from cache

---

## 04.6 Completion Checklist

- [ ] Scene tracks per-widget fragments with begin/end markers
- [ ] `DrawCtx::paint_child()` wraps begin/end fragment (explicit, not Drop-based)
- [ ] FragmentCache stores previous frame's per-widget primitives
- [ ] Scene assembly from fragments produces correct output (matches full repaint)
- [ ] Only dirty widgets call `paint()` — clean widgets reuse cached fragments
- [ ] Fragment strategy decided: leaf-only caching (recommended) or hierarchical container/child separation
- [ ] Position-change detection: scroll offset change invalidates all content fragments
- [ ] Damage rects computed from dirty widget bounds
- [ ] Fallback to full repaint when >50% widgets dirty
- [ ] `build_scene()` NOT used for retained path (it calls `scene.clear()`)
- [ ] No regressions — `./test-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./build-all.sh` green

**Exit Criteria:** A hover on one SettingRow in a 37-widget dialog calls `paint()` on 1 widget. The Scene contains all 37 widgets' primitives (assembled from cache + fresh paint). Verified by counting paint calls via a test widget.
