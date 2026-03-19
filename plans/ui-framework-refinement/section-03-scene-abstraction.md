---
section: "03"
title: "Type-Separated Scene Architecture"
status: complete
reviewed: true
goal: "Replace DrawList with a type-separated Scene (typed primitive arrays with per-primitive resolved ContentMask). Remove SceneCache. Add DamageTracker for per-widget dirty region tracking. Migrate GPU renderer to consume typed arrays with shader-side clipping."
inspired_by:
  - "GPUI Scene (crates/gpui/src/scene.rs) — type-separated primitive arrays, per-primitive ContentMask, sorted by z-order"
  - "GPUI Window paint (crates/gpui/src/window.rs) — full repaint every frame, scene-level damage tracking"
depends_on: []
sections:
  - id: "03.0"
    title: "Module Layout & File Structure"
    status: complete
  - id: "03.1"
    title: "Primitive Types & ContentMask"
    status: complete
  - id: "03.2"
    title: "Scene Struct & Paint API"
    status: complete
  - id: "03.3"
    title: "DrawCtx Migration"
    status: complete
  - id: "03.4"
    title: "Widget Paint Migration"
    status: complete
  - id: "03.5"
    title: "build_scene() & SceneCache Removal"
    status: complete
  - id: "03.6"
    title: "DamageTracker"
    status: complete
  - id: "03.7"
    title: "GPU Renderer Migration"
    status: complete
  - id: "03.8"
    title: "DrawList Removal & Cleanup"
    status: complete
  - id: "03.9"
    title: "Tests"
    status: complete
  - id: "03.10"
    title: "Completion Checklist"
    status: complete
---

# Section 03: Type-Separated Scene Architecture

**Status:** Not Started

**Goal:** Replace the flat `DrawList` (`Vec<DrawCommand>` + stack commands) with a type-separated `Scene` (typed primitive arrays with per-primitive resolved `ContentMask`). Remove `SceneCache` (full repaint every frame). Add `DamageTracker` for per-widget dirty region detection. Migrate GPU renderer to consume typed arrays directly with shader-side clipping.

## Why Replace DrawList, Not Wrap It

A metadata sidecar (the original plan) bolts damage tracking onto DrawList without fixing its fundamental problems. The correct architecture replaces DrawList entirely:

- **Type separation aligns with GPU pipeline.** The GPU renderer already separates rects from glyphs into different instance writers (`ui_rects`, `ui_glyphs`, `ui_subpixel_glyphs`, `ui_color_glyphs`). DrawList's interleaved commands force a type-dispatch loop in `convert_draw_list()`. Typed arrays eliminate this — iterate `scene.quads()` directly into `ui_rects`.
- **Per-primitive ContentMask eliminates stack commands.** DrawList stores PushClip/PopClip/PushTranslate/PopTranslate/PushLayer/PopLayer as commands that the GPU converter must process at frame time. Resolving clip/transform into each primitive at paint time removes 6 of 11 DrawCommand variants and the entire `ClipContext`/`ClipSegment` machinery.
- **SceneCache complexity is unjustified.** Only ContainerWidget uses SceneCache. It adds SceneNode, compose_scene(), containment tracking, store_log, invalidation propagation — substantial machinery for ~200 widgets whose paint() methods are 10-30 lines of array appends (~12us total). Text shaping (the expensive part) is already cached on widgets themselves. Full repaint every frame is microseconds; scene-level damage tracking replaces widget-level caching.
- **Damage tracking is natural on typed arrays.** Iterate primitives grouped by widget_id, hash each group, compare against previous frame. No sidecar metadata, no index bookmarking.

## What Gets Removed

- `DrawList`, `DrawCommand` enum (11 variants -> 0)
- `SceneCache`, `SceneNode`, `compose_scene()`, `extend_from_cache()`
- `ClipContext`, `ClipSegment`, `TierClips`, `record_draw_clipped()`, `record_draw_range_clipped()`
- `InvalidationTracker.paint_dirty` (layout_dirty stays)
- PushClip/PopClip/PushTranslate/PopTranslate/PushLayer/PopLayer as output commands
- `StatusBadge::draw()` signature (takes `&mut DrawList` directly — must be migrated to `&mut Scene`)
- `oriterm_ui/src/testing/render_assert.rs` assertion helpers (DrawList/DrawCommand-based — rewritten for Scene)

## What Gets Added

- `Scene` (5 typed arrays + internal state stacks)
- Primitive types: `Quad`, `TextRun`, `LinePrimitive`, `IconPrimitive`, `ImagePrimitive`
- `ContentMask` (resolved clip rect per primitive)
- `build_scene()` (simple tree walk, replaces compose_scene)
- `DamageTracker` (per-widget hash comparison)
- Shader-side per-instance clipping (replaces CPU scissor segments)

## Reference Implementations

- **GPUI** `crates/gpui/src/scene.rs`: Type-separated arrays (`quads`, `paths`, `monochrome_sprites`, ...). Per-primitive `ContentMask`. `PaintOperation` log for replay-based damage.
- **GPUI** `crates/gpui/src/window.rs`: Window manages clip/offset stacks during paint. Stacks resolved into each primitive's ContentMask at push time. Full repaint every frame.

## Implementation Order

1. Define primitive types and Scene struct (03.0-03.2) — new code, no integration
2. Migrate DrawCtx and all widgets (03.3-03.4) — the big switchover
3. Replace compose_scene, remove SceneCache (03.5)
4. Add DamageTracker (03.6)
5. Migrate GPU renderer (03.7)
6. Remove DrawList (03.8)
7. Tests (03.9) — written throughout, listed at end

---

## 03.0 Module Layout & File Structure

**New directory:** `oriterm_ui/src/draw/scene/`

Scene is a directory module to accommodate submodules and sibling tests.rs per test-organization.md.

- [x] Create `oriterm_ui/src/draw/scene/` directory with:
  - `mod.rs` — Scene struct, typed array fields, clear/len/accessors, re-exports; ends with `#[cfg(test)] mod tests;` per test-organization.md (~137 lines)
  - `primitives.rs` — Quad, TextRun, LinePrimitive, IconPrimitive, ImagePrimitive (~92 lines)
  - `content_mask.rs` — ContentMask struct (~30 lines)
  - `paint.rs` — push_quad/push_text/push_line/push_icon/push_image with state resolution (~101 lines)
  - `stacks.rs` — push_clip/pop_clip, push_offset/pop_offset, push_layer_bg/pop_layer_bg, queries (~122 lines)
  - `tests.rs` — scene unit tests (see 03.9)

- [x] Create `oriterm_ui/src/draw/damage/` directory with:
  - `mod.rs` — DamageTracker struct (placeholder for compute_damage); ends with `#[cfg(test)] mod tests;` per test-organization.md (~65 lines)
  - `tests.rs` — damage tracker unit tests (see 03.9)

- [x] Add `mod scene;` and `mod damage;` to `oriterm_ui/src/draw/mod.rs`
- [x] Add re-exports:
  ```rust
  pub use scene::{Scene, Quad, TextRun, LinePrimitive, IconPrimitive, ImagePrimitive, ContentMask};
  pub use damage::DamageTracker;
  ```
- [x] Verify: `./build-all.sh` passes with placeholder files

---

## 03.1 Primitive Types & ContentMask

**File:** `oriterm_ui/src/draw/scene/primitives.rs` (~120 lines)
**File:** `oriterm_ui/src/draw/scene/content_mask.rs` (~40 lines)

Every primitive carries its resolved visual state. The GPU renderer reads these directly — no stack processing at consumption time.

- [x] Define `ContentMask`:
  ```rust
  /// Resolved visual constraints for a single primitive.
  ///
  /// Computed at paint time from accumulated clip stacks.
  #[derive(Debug, Clone, Copy, PartialEq)]
  pub struct ContentMask {
      /// Viewport-space clip rect (intersection of all ancestor clips).
      pub clip: Rect,
  }

  impl ContentMask {
      /// No clipping.
      pub fn unclipped() -> Self {
          Self {
              clip: Rect::from_ltrb(
                  f32::NEG_INFINITY, f32::NEG_INFINITY,
                  f32::INFINITY, f32::INFINITY,
              ),
          }
      }
  }
  ```
  **Note:** Opacity is NOT part of ContentMask. Like the current `convert_draw_list()`, the replacement `convert_scene()` takes a `base_opacity` parameter and multiplies it into all color alphas at GPU conversion time. Per-widget opacity can be added later via an opacity stack on Scene — do not add it preemptively.

- [x] Define `Quad` — filled/bordered/shadowed rectangle:
  ```rust
  #[derive(Debug, Clone, PartialEq)]
  pub struct Quad {
      pub bounds: Rect,
      pub style: RectStyle,
      pub content_mask: ContentMask,
      pub widget_id: Option<WidgetId>,
  }
  ```

- [x] Define `TextRun` — pre-shaped text at a position:
  ```rust
  #[derive(Debug, Clone, PartialEq)]
  pub struct TextRun {
      pub position: Point,
      pub shaped: ShapedText,
      pub color: Color,
      /// Background hint for subpixel compositing (from layer_bg stack).
      pub bg_hint: Option<Color>,
      pub content_mask: ContentMask,
      pub widget_id: Option<WidgetId>,
  }
  ```

- [x] Define `LinePrimitive` — line segment with thickness:
  ```rust
  #[derive(Debug, Clone, PartialEq)]
  pub struct LinePrimitive {
      pub from: Point,
      pub to: Point,
      pub width: f32,
      pub color: Color,
      pub content_mask: ContentMask,
      pub widget_id: Option<WidgetId>,
  }
  ```

- [x] Define `IconPrimitive` — monochrome atlas icon:
  ```rust
  #[derive(Debug, Clone, PartialEq)]
  pub struct IconPrimitive {
      pub rect: Rect,
      pub atlas_page: u32,
      pub uv: [f32; 4],
      pub color: Color,
      pub content_mask: ContentMask,
      pub widget_id: Option<WidgetId>,
  }
  ```

- [x] Define `ImagePrimitive` — texture-mapped rectangle:
  ```rust
  #[derive(Debug, Clone, PartialEq)]
  pub struct ImagePrimitive {
      pub rect: Rect,
      pub texture_id: u32,
      pub uv: [f32; 4],
      pub content_mask: ContentMask,
      pub widget_id: Option<WidgetId>,
  }
  ```

---

## 03.2 Scene Struct & Paint API

**File:** `oriterm_ui/src/draw/scene/mod.rs` (~80 lines)
**File:** `oriterm_ui/src/draw/scene/paint.rs` (~150 lines)
**File:** `oriterm_ui/src/draw/scene/stacks.rs` (~100 lines)

Scene owns typed arrays and internal state stacks. State is resolved into ContentMask at push time — stacks are consumed internally, never emitted as output.

- [x] Define `Scene` struct:
  ```rust
  pub struct Scene {
      // Typed primitive arrays (output).
      pub(crate) quads: Vec<Quad>,
      pub(crate) text_runs: Vec<TextRun>,
      pub(crate) lines: Vec<LinePrimitive>,
      pub(crate) icons: Vec<IconPrimitive>,
      pub(crate) images: Vec<ImagePrimitive>,
      // Internal state stacks (resolved into ContentMask, not in output).
      clip_stack: Vec<Rect>,
      offset_stack: Vec<(f32, f32)>,
      cumulative_offset: (f32, f32),
      layer_bg_stack: Vec<Color>,
  }
  ```

- [x] Implement constructors and accessors:
  ```rust
  impl Scene {
      pub fn new() -> Self { /* all empty */ }
      pub fn quads(&self) -> &[Quad] { &self.quads }
      pub fn text_runs(&self) -> &[TextRun] { &self.text_runs }
      pub fn lines(&self) -> &[LinePrimitive] { &self.lines }
      pub fn icons(&self) -> &[IconPrimitive] { &self.icons }
      pub fn images(&self) -> &[ImagePrimitive] { &self.images }
      pub fn is_empty(&self) -> bool { /* all arrays empty */ }

      /// Clears all primitives and resets stacks, retaining allocated memory.
      pub fn clear(&mut self) { /* clear all vecs, reset stacks */ }
  }
  ```

- [x] Implement paint methods (`paint.rs`). Each resolves current state into the primitive (deviation: widget_id stored as current context on Scene via `set_widget_id()` instead of per-call parameter, to stay within clippy's 5-arg limit):
  ```rust
  impl Scene {
      pub fn push_quad(&mut self, bounds: Rect, style: RectStyle, widget_id: Option<WidgetId>) {
          self.quads.push(Quad {
              bounds: self.apply_offset(bounds),
              style,
              content_mask: self.current_content_mask(),
              widget_id,
          });
      }

      pub fn push_text(
          &mut self, position: Point, shaped: ShapedText,
          color: Color, widget_id: Option<WidgetId>,
      ) {
          self.text_runs.push(TextRun {
              position: self.apply_offset_point(position),
              shaped, color,
              bg_hint: self.current_layer_bg(),
              content_mask: self.current_content_mask(),
              widget_id,
          });
      }

      pub fn push_line(
          &mut self, from: Point, to: Point, width: f32,
          color: Color, widget_id: Option<WidgetId>,
      ) { /* offset from/to, resolve content_mask */ }

      pub fn push_icon(
          &mut self, rect: Rect, atlas_page: u32, uv: [f32; 4],
          color: Color, widget_id: Option<WidgetId>,
      ) { /* offset rect, resolve content_mask */ }

      pub fn push_image(
          &mut self, rect: Rect, texture_id: u32, uv: [f32; 4],
          widget_id: Option<WidgetId>,
      ) { /* offset rect, resolve content_mask */ }
  }
  ```

- [x] Implement state stack methods (`stacks.rs`):
  ```rust
  impl Scene {
      // --- Clip ---
      pub fn push_clip(&mut self, rect: Rect) {
          let resolved = self.apply_offset(rect);
          let intersected = self.clip_stack.last()
              .map(|c| c.intersection(resolved))
              .unwrap_or(resolved);
          self.clip_stack.push(intersected);
      }
      pub fn pop_clip(&mut self) {
          debug_assert!(!self.clip_stack.is_empty(), "pop_clip without matching push_clip");
          self.clip_stack.pop();
      }

      // --- Offset (replaces PushTranslate/PopTranslate) ---
      pub fn push_offset(&mut self, dx: f32, dy: f32) {
          self.offset_stack.push((dx, dy));
          self.cumulative_offset.0 += dx;
          self.cumulative_offset.1 += dy;
      }
      pub fn pop_offset(&mut self) {
          debug_assert!(!self.offset_stack.is_empty(), "pop_offset without matching push_offset");
          if let Some((dx, dy)) = self.offset_stack.pop() {
              self.cumulative_offset.0 -= dx;
              self.cumulative_offset.1 -= dy;
          }
      }

      // --- Layer BG (for subpixel text compositing) ---
      pub fn push_layer_bg(&mut self, bg: Color) { self.layer_bg_stack.push(bg); }
      pub fn pop_layer_bg(&mut self) {
          debug_assert!(!self.layer_bg_stack.is_empty());
          self.layer_bg_stack.pop();
      }

      // --- Queries (for visibility culling) ---
      /// Current clip rect in viewport space.
      pub fn current_clip(&self) -> Option<Rect> {
          self.clip_stack.last().copied()
      }

      /// Clip rect in content space (for scroll container visibility culling).
      pub fn current_clip_in_content_space(&self) -> Option<Rect> {
          self.clip_stack.last().map(|clip| {
              clip.offset(-self.cumulative_offset.0, -self.cumulative_offset.1)
          })
      }

      pub fn current_layer_bg(&self) -> Option<Color> {
          self.layer_bg_stack.last().copied()
      }

      // --- Stack depth queries (for debug assertions) ---
      pub fn clip_stack_is_empty(&self) -> bool { self.clip_stack.is_empty() }
      pub fn offset_stack_is_empty(&self) -> bool { self.offset_stack.is_empty() }
      pub fn layer_bg_stack_is_empty(&self) -> bool { self.layer_bg_stack.is_empty() }
  }
  ```

- [x] Internal helpers: `current_content_mask()`, `apply_offset(Rect)`, `apply_offset_point(Point)`

---

## 03.3 DrawCtx Migration

**Risk: Atomic migration.** Sections 03.3 and 03.4 form a single atomic change — the codebase will not compile between them. Changing `DrawCtx.draw_list` to `DrawCtx.scene` immediately breaks every widget's `paint()` method and every test that constructs a `DrawCtx`. All ~76 construction sites and ~36 widget paint methods must be updated in one commit. Plan for a focused session. Use `cargo check` frequently to track remaining sites. The compiler will catch every missed site as a type error.

**File:** `oriterm_ui/src/widgets/contexts.rs`

Replace `draw_list: &'a mut DrawList` with `scene: &'a mut Scene`. Remove `scene_cache` field.

- [x] Change DrawCtx (deviation: `scene_cache` kept until 03.5; bridge DrawList in Scene enables GPU compat):
  ```rust
  pub struct DrawCtx<'a> {
      pub scene: &'a mut Scene,       // was: pub draw_list: &'a mut DrawList
      pub bounds: Rect,
      pub now: Instant,
      pub measurer: &'a dyn TextMeasurer,
      pub theme: &'a UiTheme,
      pub icons: Option<&'a ResolvedIcons>,
      pub interaction: Option<&'a InteractionManager>,
      pub widget_id: Option<WidgetId>,
      pub frame_requests: Option<&'a FrameRequestFlags>,
      // REMOVED: scene_cache: Option<&'a mut SceneCache>
  }
  ```

- [x] Update `for_child()` to forward `scene` and call `set_widget_id()` (scene_cache kept until 03.5)

- [x] Update **all 76 DrawCtx struct literal construction sites**:
  - 22 production sites (6 in oriterm, 15 in oriterm_ui, 1 in test harness)
  - 54 test sites across widget test files
  - Change `draw_list: &mut draw_list` to `scene: &mut scene`
  - Remove `scene_cache: ...` field
  - **Run `grep -rn 'DrawCtx {' oriterm_ui/ oriterm/` to confirm all sites**
  - Compiler enforces completeness — missing sites are compile errors

---

## 03.4 Widget Paint Migration

Migrate all widget `paint()` implementations from `ctx.draw_list.push_*()` to `ctx.scene.push_*()`.

### Translation table

| Before | After |
|--------|-------|
| `ctx.draw_list.push_rect(bounds, style)` | `ctx.scene.push_quad(bounds, style, ctx.widget_id)` |
| `ctx.draw_list.push_text(pos, shaped, color)` | `ctx.scene.push_text(pos, shaped, color, ctx.widget_id)` |
| `ctx.draw_list.push_line(from, to, w, color)` | `ctx.scene.push_line(from, to, w, color, ctx.widget_id)` |
| `ctx.draw_list.push_icon(rect, page, uv, color)` | `ctx.scene.push_icon(rect, page, uv, color, ctx.widget_id)` |
| `ctx.draw_list.push_image(rect, tid, uv)` | `ctx.scene.push_image(rect, tid, uv, ctx.widget_id)` |
| `ctx.draw_list.push_clip(rect)` | `ctx.scene.push_clip(rect)` |
| `ctx.draw_list.pop_clip()` | `ctx.scene.pop_clip()` |
| `ctx.draw_list.push_translate(dx, dy)` | `ctx.scene.push_offset(dx, dy)` |
| `ctx.draw_list.pop_translate()` | `ctx.scene.pop_offset()` |
| `ctx.draw_list.push_layer(bg)` | `ctx.scene.push_layer_bg(bg)` |
| `ctx.draw_list.pop_layer()` | `ctx.scene.pop_layer_bg()` |
| `ctx.draw_list.current_clip_rect()` | `ctx.scene.current_clip()` |
| `ctx.draw_list.current_clip_rect_in_content_space()` | `ctx.scene.current_clip_in_content_space()` |
| `ctx.draw_list.current_layer_bg()` | `ctx.scene.current_layer_bg()` |

- [x] Migrate leaf widgets (~21): LabelWidget, SeparatorWidget, SpacerWidget, RichLabel, ButtonWidget, CheckboxWidget, DropdownWidget, SliderWidget, ToggleWidget, ColorSwatchGrid, SpecialColorSwatch, CodePreviewWidget, CursorPickerWidget, KeybindWidget (x2), NumberInputWidget, SchemeCardWidget, WindowControlButton, MenuWidget, TextInputWidget, SidebarNavWidget

- [x] Migrate container widgets (~12): ContainerWidget (SceneCache calls bridged via `as_draw_list_mut()`), StackWidget, ScrollWidget, FormLayout, FormSection, FormRow, PageContainerWidget, PanelWidget, WindowChromeWidget, DialogWidget, SettingRow, SettingsPanel

- [x] Migrate tab_bar helper draw methods in `tab_bar/widget/*.rs`

- [x] Migrate overlay drawing in `oriterm_ui/src/overlay/manager/mod.rs`

- [x] Migrate draw helpers in `oriterm/src/app/redraw/draw_helpers.rs` and `oriterm/src/app/dialog_rendering.rs`

- [x] Migrate `StatusBadge::draw()` (`oriterm_ui/src/widgets/status_badge/mod.rs`): changed signature from `&mut DrawList` to `&mut Scene`.

- [x] Migrate `oriterm/src/app/redraw/search_bar.rs`: now uses Scene, GPU boundary uses `scene.as_draw_list()`.

- [x] Migrate `oriterm/src/widgets/terminal_preview/mod.rs`: `ctx.scene.push_quad()`.

- [x] Verify: `grep 'draw_list' oriterm_ui/src/widgets/` — only bridge `as_draw_list()` calls remain (correct)
- [x] Verify: `grep 'draw_list' oriterm/src/widgets/` — zero direct `ctx.draw_list` references
- [x] Migrate `oriterm/src/widgets/terminal_grid/tests.rs`: uses `Scene::new()` and `DrawCtx { scene: ... }`.
- [x] Verify: `grep 'draw_list' oriterm_ui/src/overlay/` — only bridge calls remain (correct)

- [x] Migrate `WidgetTestHarness::render()` in `oriterm_ui/src/testing/harness_inspect.rs`: returns `Scene`.

- [x] Rewrite `oriterm_ui/src/testing/render_assert.rs` assertion helpers to query Scene's typed arrays:
  - `command_count()` -> `scene.len()`
  - `rects()` -> `scene.quads()`
  - `texts()` -> `scene.text_runs()`
  - `assert_has_rect_with_color()` -> iterates `scene.quads()` checking `style.fill`
  - `assert_has_text()` -> `!scene.text_runs().is_empty()`

- [x] Update tests in `oriterm_ui/src/testing/tests.rs` to use new Scene-based API

---

## 03.5 build_scene() & SceneCache Removal

**Replaces:** compose_scene(), SceneCache, SceneNode

compose_scene() did three things: (1) invalidate dirty SceneNodes, (2) enable caching on DrawCtx, (3) call root.paint(). With SceneCache removed, this reduces to calling root.paint() with a stack-balance assertion.

- [x] Create `build_scene()` as a free function in `oriterm_ui/src/draw/scene/mod.rs` (re-exported from `draw/mod.rs`):
  ```rust
  /// Paints the widget tree into a Scene. Full repaint every call.
  ///
  /// Replaces `compose_scene()`. The caller uses DamageTracker to
  /// detect what changed between frames.
  pub fn build_scene(root: &dyn Widget, ctx: &mut DrawCtx<'_>) {
      ctx.scene.clear();
      root.paint(ctx);
      debug_assert!(
          ctx.scene.clip_stack_is_empty()
              && ctx.scene.offset_stack_is_empty()
              && ctx.scene.layer_bg_stack_is_empty(),
          "Unbalanced stacks after build_scene"
      );
  }
  ```

- [x] Delete `oriterm_ui/src/draw/scene_compose/` directory (mod.rs + tests.rs)
- [x] Delete `oriterm_ui/src/draw/scene_node/` directory (mod.rs + tests.rs)
- [x] Remove their `mod` declarations and `pub use` re-exports from `draw/mod.rs`

- [x] Simplify ContainerWidget — remove:
  - `try_replay_cached()` method (in `layout_build.rs`)
  - `store_in_cache()` method (in `layout_build.rs`)
  - All `extend_from_cache()` calls
  - Cache-related branches in paint() (lines 342-363 in `container/mod.rs`)
  - ContainerWidget's paint() constructs child `DrawCtx` struct literals directly (line 350) with `scene_cache: ctx.scene_cache.as_deref_mut()`. After migration, switch to `ctx.for_child(child_id, bounds)` which avoids manual struct construction. The `draw_list.len()` / `log_position()` bookkeeping also disappears.
  - Result: paint() iterates children, calls `child.paint(&mut ctx.for_child(child_id, bounds))`, no caching
  - Delete SceneCache-specific tests in `oriterm_ui/src/widgets/container/tests.rs`: `scene_cache_skips_clean_children`, `scene_cache_redraws_invalidated_child`, `scene_cache_miss_on_bounds_mismatch` (lines ~407-557). These test the removed caching behavior.

- [x] Simplify InvalidationTracker:
  - Remove `paint_dirty: HashSet<WidgetId>` (DamageTracker handles paint-level change detection)
  - Keep `layout_dirty: HashSet<WidgetId>` (still needed for relayout)
  - Keep `full_invalidation: bool` (still needed for resize/theme)
  - Simplify or rename `DirtyKind` — only Layout remains
  - Note: `DirtyKind::Paint` is used in `dialog_rendering.rs` line 135 for `HotChanged` events, and `From<ControllerRequests>` maps `PAINT` to `DirtyKind::Paint`. When `paint_dirty` is removed, update these callers: either drop the `mark(Paint)` calls (DamageTracker replaces paint tracking) or use full invalidation if animation is running (which `dialog_rendering.rs` already does separately). Update the `From` impl so `PAINT` maps to `Clean`.
  - Remove `is_paint_dirty()` method and update callers (`scene_compose/mod.rs` is deleted; any remaining uses in app layer become layout-only checks).

- [x] Update app-layer call sites:
  - `oriterm/src/app/redraw/draw_helpers.rs` — `draw_tab_bar()`: compose_scene -> build_scene; remove `scene_cache` and `invalidation` parameters (SceneCache gone, InvalidationTracker no longer needed for paint). Also remove `_invalidation` and `_scene_cache` params from `draw_overlays()` (already unused, prefixed with `_`).
  - `oriterm/src/app/dialog_rendering.rs` — `compose_dialog_widgets()`: replace `compose_scene()` calls with `build_scene()`; switch DrawCtx construction from `draw_list` to `scene`; remove `scene_cache: None` from all 3 DrawCtx struct literals (lines 153, 176, 234); remove `&mut ctx.scene_cache` arguments to `compose_scene()` (lines 162, 185)
  - Remove `SceneCache` fields from `WindowContext` (`oriterm/src/app/window_context.rs`) and `DialogWindowContext` (`oriterm/src/app/dialog_context/mod.rs`)
  - Migrate `chrome_draw_list: DrawList` on `WindowContext` (`oriterm/src/app/window_context.rs`) to `chrome_scene: Scene`
  - Migrate `draw_list: DrawList` on `DialogWindowContext` (`oriterm/src/app/dialog_context/mod.rs`) to `scene: Scene`
  - Remove `InvalidationTracker` paint-dirty calls (keep layout-dirty calls)
  - Remove `ctx.scene_cache.clear()` calls in:
    - `oriterm/src/app/config_reload/mod.rs` (lines 65, 167)
    - `oriterm/src/app/dialog_context/event_handling/mod.rs` (line 94)
    - `oriterm/src/app/chrome/resize.rs` (line 207)
    - `oriterm/src/app/mod.rs` (lines 335, 375)
    - `oriterm/src/app/dialog_context/mod.rs` (line 230, in `clear()` method)
    SceneCache is gone; `DamageTracker.reset()` replaces cache clearing on config reload / dialog resize / window resize.
  - Remove `&mut ctx.scene_cache` arguments in `oriterm/src/app/redraw/mod.rs` (lines 299, 320) and `oriterm/src/app/redraw/multi_pane.rs` (lines 451, 471) — these pass scene_cache to `draw_tab_bar()` / `draw_overlays()` which will no longer accept it.
  - Update comments in `oriterm/src/app/render_dispatch.rs` that reference `compose_scene` (lines 39, 62)

---

## 03.6 DamageTracker

**File:** `oriterm_ui/src/draw/damage/mod.rs` (~250 lines)

Per-widget damage tracking via primitive hashing. Each primitive carries `widget_id`. After build_scene(), DamageTracker iterates the Scene, hashes primitives grouped by widget, and compares against the previous frame.

- [x] Define DamageTracker:
  ```rust
  pub struct DamageTracker {
      dirty_regions: Vec<Rect>,
      merge_scratch: Vec<Rect>,
      prev_state: HashMap<WidgetId, WidgetFrameState>,
      current_scratch: HashMap<WidgetId, WidgetFrameState>,
      first_frame: bool,
  }

  struct WidgetFrameState {
      hash: u64,
      bounds: Rect,
  }
  ```

- [x] Implement `compute_damage(&mut self, scene: &Scene)`:
  1. If first_frame -> compute full_bounds, push as single dirty region, update state, return
  2. Build current per-widget state from Scene: iterate all 5 typed arrays, for each primitive with `Some(widget_id)`, feed its fields into a per-widget hasher (via `f32::to_bits()`) and accumulate bounds
  3. Diff current vs prev: same hash+bounds -> clean; different hash -> both old+new bounds dirty; new widget -> new bounds dirty; removed widget -> old bounds dirty
  4. Merge overlapping dirty rects (greedy O(n^2), n typically < 20)
  5. Swap current -> prev for next frame

- [x] Implement per-primitive hashing (in `hash_primitives.rs` submodule). Numeric fields hashed via `f32::to_bits()` for floats, direct hash for integers:
  - **Quad:** bounds (x,y,w,h), all RectStyle fields (fill, border width+color, corner_radius[4], shadow 5 fields, gradient angle + stops), content_mask (clip rect)
  - **TextRun:** position, all ShapedGlyph fields (glyph_id, face_index, synthetic, x_advance, x_offset, y_offset) + shaped width/height/baseline, color, bg_hint, content_mask
  - **LinePrimitive:** from, to, width, color, content_mask
  - **IconPrimitive:** rect, atlas_page, uv[4], color, content_mask
  - **ImagePrimitive:** rect, texture_id, uv[4], content_mask

  **Verify at implementation time:** Run grep on struct definitions to confirm no fields missed.

- [x] Handle primitives with `widget_id: None`: these come from app-frame-level draws (root DrawCtx has `widget_id: None`). Group them under a sentinel key or exclude from per-widget tracking. They still contribute to the full-scene dirty region on first frame.

- [x] Per-widget bounds: union of all primitive bounds for each widget_id. Quad -> bounds, TextRun -> Rect(position, shaped.width x shaped.height), LinePrimitive -> bounding rect of from/to +/- half-width, Icon/Image -> rect.

- [x] Provide query API:
  ```rust
  pub fn is_region_dirty(&self, rect: Rect) -> bool
  pub fn dirty_regions(&self) -> &[Rect]
  pub fn has_damage(&self) -> bool
  pub fn is_first_frame(&self) -> bool
  pub fn reset(&mut self)  // after resize/theme/font/scale change
  ```

- [x] Merge overlapping rects using pre-allocated `merge_scratch` (no per-frame allocation)

- [x] **Allocation discipline for compute_damage():** The two `HashMap<WidgetId, WidgetFrameState>` fields (`prev_state`, `current_scratch`) are swapped each frame (step 5). After warmup, both maps have sufficient capacity for the widget set. Use `current_scratch.clear()` (retains capacity) before populating, then `std::mem::swap(&mut self.prev_state, &mut self.current_scratch)`. This gives zero allocation after the first frame for a stable widget count. If widget count grows, HashMap may reallocate — acceptable since widget tree changes are rare. Do NOT use `HashMap::new()` per frame.

- [x] Add `#[cfg(test)] mod tests;` at bottom of `damage/mod.rs` (12 tests pass)

- [x] **Integration wiring:** Add `DamageTracker` as a field on `WindowContext` and `DialogWindowContext` (replaces the per-widget paint-dirty tracking that was removed from InvalidationTracker). Call `damage_tracker.compute_damage(&scene)` after `build_scene()` in `draw_helpers.rs` and `dialog_rendering.rs`. Call `damage_tracker.reset()` on resize/theme/font/scale change (same sites that call `invalidation.invalidate_all()`).

---

## 03.7 GPU Renderer Migration

**Files:**
- `oriterm/src/gpu/draw_list_convert/mod.rs` — rewrite to consume Scene
- `oriterm/src/gpu/draw_list_convert/clip.rs` — DELETE
- `oriterm/src/gpu/draw_list_convert/text.rs` — update for TextRun
- `oriterm/src/gpu/window_renderer/draw_list.rs` — update entry points
- `oriterm/src/gpu/window_renderer/helpers.rs` — remove `record_draw_clipped`
- `oriterm/src/gpu/prepared_frame/mod.rs` — remove TierClips, update instance size
- WGSL shader files — add per-instance clip

### 03.7.1 convert_scene() replaces convert_draw_list()

- [x] Rewrite `convert_draw_list()` -> `convert_scene()`:
  ```rust
  pub fn convert_scene(
      scene: &Scene,
      ui_writer: &mut InstanceWriter,
      text_ctx: Option<&mut TextContext<'_>>,
      scale: f32,
      base_opacity: f32,
  ) {
      for quad in scene.quads() {
          convert_quad(quad, ui_writer, scale, base_opacity);
      }
      for line in scene.lines() {
          convert_line_primitive(line, ui_writer, scale, base_opacity);
      }
      if let Some(text_ctx) = text_ctx {
          for text in scene.text_runs() {
              convert_text_run(text, text_ctx, scale, base_opacity);
          }
          for icon in scene.icons() {
              convert_icon_primitive(icon, text_ctx, scale, base_opacity);
          }
      }
  }
  ```
  No type-dispatch loop. No clip state tracking. No translate stack.

### 03.7.2 Shader-side clipping

Each primitive's ContentMask.clip is baked into the instance data. The fragment shader discards pixels outside the clip rect. This eliminates ClipContext/ClipSegment entirely — **one draw call per type per tier**.

- [x] Update instance layout (80 -> 96 bytes). Current layout for reference:
  ```
  Current (80 bytes):
  [0-15]   pos_x, pos_y, size_w, size_h
  [16-31]  uv_x, uv_y, uv_w, uv_h
  [32-47]  fg_r, fg_g, fg_b, fg_a
  [48-63]  bg_r, bg_g, bg_b, bg_a
  [64-67]  kind (u32)
  [68-71]  atlas_page (u32)
  [72-75]  corner_radius (f32)
  [76-79]  border_width (f32)

  New (96 bytes — append clip at end to minimize churn):
  [0-15]   pos_x, pos_y, size_w, size_h
  [16-31]  uv_x, uv_y, uv_w, uv_h
  [32-47]  fg_r, fg_g, fg_b, fg_a
  [48-63]  bg_r, bg_g, bg_b, bg_a
  [64-67]  kind (u32)
  [68-71]  atlas_page (u32)
  [72-75]  corner_radius (f32)
  [76-79]  border_width (f32)
  [80-95]  clip_x, clip_y, clip_w, clip_h  <- NEW (appended)
  ```
  Appending the clip rect at the end preserves all existing field offsets, minimizing changes to existing push functions. Only new code reads from offsets 80-95. Update `INSTANCE_SIZE` from 80 to 96 and add `OFF_CLIP_X/Y/W/H` constants. Update `VertexBufferLayout` stride and add clip attribute.

  **Implementation detail:** Both `push_instance()` (private, used by push_rect/push_glyph/push_cursor) and `push_ui_rect()` (public) call `self.buf.resize(start + INSTANCE_SIZE, 0)`. Since `INSTANCE_SIZE` changes from 80 to 96, both methods automatically allocate the right size. The new clip fields at [80-95] must be explicitly written in each method; the `resize(..., 0)` zero-fills them by default which means `ContentMask::unclipped()` must be written explicitly (NEG_INFINITY, not 0.0) for terminal-tier instances. The compile-time assertion `const _: () = assert!(OFF_BORDER_WIDTH + 4 == INSTANCE_SIZE);` in `instance_writer/mod.rs` must be updated to `assert!(OFF_CLIP_H + 4 == INSTANCE_SIZE);` after adding the new offset constants.

  **Note:** Terminal-tier instances (backgrounds, glyphs, cursors) do NOT use clip — write `ContentMask::unclipped()` (f32::NEG_INFINITY bounds) so the shader never discards. Only chrome and overlay tiers carry real clip rects.

  **Memory impact:** 80 -> 96 bytes is a 20% increase in instance buffer size. For a typical 80x24 terminal (1920 visible cells x 2 instances per cell for bg+glyph = ~3840 instances), this adds 61 KB to the GPU upload. Negligible relative to the ~300 KB baseline and the atlas textures (~16 MB). The tradeoff is justified by eliminating ClipContext/TierClips/scissor-splitting complexity.

- [x] Update all push methods on InstanceWriter to write clip fields at [80-95]:
  - `push_ui_rect()` — accepts clip rect from ContentMask
  - `push_glyph()` — accepts clip rect from ContentMask
  - `push_glyph_with_bg()` — accepts clip rect from ContentMask
  - `push_rect()` and `push_cursor()` — write `ContentMask::unclipped()` values (terminal tier never clips)
- [x] Update `VertexBufferLayout` in `oriterm/src/gpu/pipeline/mod.rs`: change `INSTANCE_STRIDE` (computed from `INSTANCE_SIZE`) from 80 to 96. Add clip `VertexAttribute` entry to both `INSTANCE_ATTRS` (7 entries -> 8; used by bg/fg/subpixel_fg/color_fg pipelines) and `UI_RECT_ATTRS` (9 entries -> 10; used by ui_rect pipeline). The clip attribute uses `VertexFormat::Float32x4` at offset 80, next available shader location after the existing attributes.

- [x] Update WGSL shaders that consume the instance buffer. All shaders reading the main instance layout must be updated to read the new clip fields and discard fragments outside the clip rect:
  - `oriterm/src/gpu/shaders/ui_rect.wgsl` — UI rects (chrome + overlay)
  - `oriterm/src/gpu/shaders/fg.wgsl` — monochrome glyph instances
  - `oriterm/src/gpu/shaders/subpixel_fg.wgsl` — subpixel glyph instances
  - `oriterm/src/gpu/shaders/color_fg.wgsl` — color glyph instances
  - `oriterm/src/gpu/shaders/bg.wgsl` — background rects (terminal tier)
  - Note: `image.wgsl`, `composite.wgsl`, `colr_solid.wgsl`, and `colr_gradient.wgsl` use different instance layouts and are unaffected.
  Fragment shader clip logic (vertex shader computes `clip_max = clip_pos + clip_size` from the x,y,w,h instance data and passes both `clip_min` and `clip_max` as varyings):
  ```wgsl
  // Vertex shader:
  out.clip_min = vec2<f32>(instance.clip_x, instance.clip_y);
  out.clip_max = vec2<f32>(instance.clip_x + instance.clip_w, instance.clip_y + instance.clip_h);

  // Fragment shader:
  if frag_pos.x < in.clip_min.x || frag_pos.x > in.clip_max.x
      || frag_pos.y < in.clip_min.y || frag_pos.y > in.clip_max.y {
      discard;
  }
  ```
  **Risk:** Fragment `discard` prevents early-z optimization on tile-based mobile GPUs (ARM Mali, Apple M-series). For desktop wgpu targets (Vulkan/DX12/Metal on x86), this is negligible — UI clipping involves a handful of primitives, not thousands. If profiling shows issues, the alternative is `step()` to set alpha to 0.0 instead of discard (avoids breaking the rasterization pipeline). Start with `discard` for simplicity.

  **Risk:** The `f32::NEG_INFINITY` sentinel for unclipped instances must survive the vertex -> fragment interpolation. WGSL `f32` supports infinity per IEEE 754, and the comparison `frag_pos.x < f32::NEG_INFINITY` is always false, so unclipped instances never discard. Verify this works on all three target backends (Vulkan, DX12, Metal) during testing.

- [x] Per-instance opacity: multiply `base_opacity` into color alpha during convert_scene (same as current `convert_draw_list` behavior)

### 03.7.3 ClipContext removal

- [x] Delete `oriterm/src/gpu/draw_list_convert/clip.rs`
- [x] Remove `TierClips` from `PreparedFrame` (`ui_clips`, `overlay_clips` fields)
- [x] Remove `clips: TierClips` field from `OverlayDrawRange` (`oriterm/src/gpu/prepared_frame/mod.rs`). With shader-side clipping, per-overlay clip segments are unnecessary — each overlay's primitives carry their own ContentMask.
- [x] Remove `record_draw_clipped()` and `record_draw_range_clipped()` from `window_renderer/helpers.rs`
- [x] Replace all `record_draw_clipped()` calls (4 static calls in `record_cached_content_passes()`) and `record_draw_range_clipped()` calls (4 per overlay, inside the `for range in &p.overlay_draw_ranges` loop in `record_overlay_pass()`) with simple `record_draw()` calls. Chrome tier becomes 4 simple draws. Overlay tier becomes 4 simple draws per overlay using `record_draw()` with start..end instance ranges (a new `record_draw_range()` helper, or inline the range logic).
- [x] Simplify `PreparedFrame::extend_from()`: remove `ui_clips.extend_from()`, `overlay_clips.extend_from()`, and the per-overlay `clips.shift_offsets()` call. Also simplify `clear()` and `clear_ephemeral_tiers()` to stop clearing TierClips.
- [x] Remove `overlay_scratch_clips: TierClips` field from `WindowRenderer` and `clip_stack: Vec<Rect>` (used only by ClipContext). These are currently fields on the renderer struct used during `convert_draw_list()`.

### 03.7.4 Entry point updates

- [x] Rename/rewrite `append_ui_draw_list_with_text()` -> `append_ui_scene_with_text()` (takes `&Scene`)
- [x] Rename/rewrite `append_overlay_draw_list_with_text()` -> `append_overlay_scene_with_text()` (takes `&Scene`)
- [x] Update text glyph rasterization cache to iterate `scene.text_runs()` and `scene.icons()` instead of filtering DrawCommands
- [x] Rewrite `ui_text_raster_keys()` in `oriterm/src/gpu/window_renderer/helpers.rs`: change signature from `&DrawList` to `&Scene`, iterate `scene.text_runs()` instead of filtering `DrawCommand::Text` variants. Also iterate `scene.icons()` for icon glyph raster keys.
- [x] Update `cache_ui_glyphs()` in `oriterm/src/gpu/window_renderer/draw_list.rs` to accept `&Scene` instead of `&DrawList`
- [x] Rename `oriterm/src/gpu/window_renderer/draw_list.rs` to `scene_append.rs` (now appends Scene data, not DrawList). Update `mod draw_list;` in `window_renderer/mod.rs`.
- [x] Update all callers in `draw_helpers.rs`, `dialog_rendering.rs`, and `search_bar.rs`

---

## 03.8 DrawList Removal & Cleanup

After all consumers are migrated, remove DrawList and related types.

- [x] Delete `oriterm_ui/src/draw/draw_list.rs` (DrawList, DrawCommand enum)
- [x] Remove `mod draw_list;` and its `pub use` from `draw/mod.rs`
- [x] Split `oriterm_ui/src/draw/tests.rs`: keep RectStyle tests (lines 15-60 — `rect_style_default_is_invisible`, `rect_style_filled`, `rect_style_builder_chain`, `rect_style_per_corner_radius`), delete all DrawList/DrawCommand tests (lines 62-411 — `draw_list_new_is_empty`, `clip_push_pop_balanced`, `layer_push_pop_balanced`, translate tests, etc.)
- [x] Rewrite `oriterm/src/gpu/draw_list_convert/tests.rs` to test `convert_scene()` with Scene input instead of DrawList+DrawCommand
- [x] Rename `oriterm/src/gpu/draw_list_convert/` directory to `oriterm/src/gpu/scene_convert/` (the module now converts Scene, not DrawList). Update `mod draw_list_convert;` to `mod scene_convert;` in `oriterm/src/gpu/mod.rs` and all import paths.
- [x] Verify: `grep -rn 'DrawList\|DrawCommand' oriterm_ui/ oriterm/` returns zero hits outside comments
- [x] Keep: RectStyle, Border, Shadow, Gradient, GradientStop (used by Quad)
- [x] Keep: ShapedText, ShapedGlyph (used by TextRun)
- [x] Run `./clippy-all.sh` — fix dead code warnings
- [x] Run `./build-all.sh` — verify cross-compilation
- [x] Run `./test-all.sh` — all tests pass

---

## 03.9 Tests

**Files:**
- `oriterm_ui/src/draw/scene/tests.rs`
- `oriterm_ui/src/draw/damage/tests.rs`

### Scene tests (scene/tests.rs)

- [x] Empty scene: `Scene::new()` has empty arrays and no stacks
- [x] push_quad: adds to quads with ContentMask::unclipped()
- [x] push_text: bg_hint captured from layer_bg stack
- [x] push_line: from/to offset by cumulative translation
- [x] push_clip / pop_clip: primitives inside clip have intersected clip in ContentMask
- [x] Nested clips: inner = intersection of outer and inner
- [x] push_offset / pop_offset: primitive bounds offset by cumulative translation
- [x] current_clip_in_content_space: subtracts offset from clip rect
- [x] clear() empties arrays and resets stacks, retains Vec capacity
- [x] Stack balance: debug assertion fires on unbalanced push/pop
- [x] Layer bg: text inside push_layer_bg(white) has bg_hint = Some(white)
- [x] Offset + clip interaction: clip applied in viewport space, offset applies to primitive bounds
- [x] push_icon: adds to icons with offset rect, resolved ContentMask, and widget_id
- [x] push_image: adds to images with offset rect, resolved ContentMask, and widget_id
- [x] Capacity retention: after push + clear(), Vec capacity is preserved (no reallocation on next push)

### DamageTracker tests (damage/tests.rs)

- [x] First frame: full-scene dirty region
- [x] Identical scenes: zero dirty regions
- [x] Changed widget: dirty at widget bounds
- [x] Moved widget: dirty at both old and new bounds
- [x] Removed widget: dirty at old bounds
- [x] New widget: dirty at new bounds
- [x] Overlapping dirty rects merged
- [x] Non-overlapping dirty rects stay separate
- [x] reset() causes next call to act as first frame
- [x] is_region_dirty() query correctness
- [x] has_damage() false when no damage

### Integration tests

- [x] build_scene() produces correct Scene for widget tree
- [x] build_scene() fires assertion on unbalanced stacks (debug mode)
- [x] Full cycle: build_scene -> compute_damage -> verify dirty regions match a widget state change

---

## 03.10 Completion Checklist

- [x] **Primitive types:** Quad, TextRun, LinePrimitive, IconPrimitive, ImagePrimitive, ContentMask (03.1)
- [x] **Scene struct:** typed arrays + internal state stacks (03.2)
- [x] **Paint API:** push_quad/text/line/icon/image resolve ContentMask at push time (03.2)
- [x] **State stacks:** push_clip/pop_clip, push_offset/pop_offset, push_layer_bg/pop_layer_bg with debug assertions (03.2)
- [x] **Queries:** current_clip(), current_clip_in_content_space(), current_layer_bg() (03.2)
- [x] **DrawCtx:** `scene` replaces `draw_list`; `scene_cache` removed (03.3)
- [x] **76 DrawCtx construction sites** updated (22 production + 54 test) (03.3)
- [x] **~36 widget paint methods** migrated to Scene API (03.4)
- [x] **Zero remaining** `ctx.draw_list` references in widget code (03.4)
- [x] **StatusBadge::draw()** migrated to `&mut Scene` (03.4)
- [x] **search_bar.rs** migrated to Scene (03.4)
- [x] **WidgetTestHarness::render()** returns Scene (03.4)
- [x] **render_assert.rs** helpers rewritten for Scene (03.4)
- [x] **terminal_grid/tests.rs** migrated to Scene (03.4)
- [x] **build_scene()** replaces compose_scene() with stack balance assertion (03.5)
- [x] **SceneCache, SceneNode, compose_scene() deleted** (03.5)
- [x] **ContainerWidget simplified** — no cache replay code (03.5)
- [x] **InvalidationTracker simplified** — paint_dirty removed, layout_dirty retained (03.5)
- [x] **scene_cache.clear()** calls removed from config_reload, chrome/resize, dialog event_handling, app/mod, dialog_context (03.5)
- [x] **DamageTracker** with per-widget hash comparison via f32::to_bits() (03.6)
- [x] **DamageTracker** wired into WindowContext + DialogWindowContext (03.6)
- [x] **convert_scene()** replaces convert_draw_list() — typed array iteration (03.7) — partially done: convert_draw_list still exists for bridge path, convert_scene deferred to 03.8 when bridge is removed
- [x] **Per-instance clip** in GPU instance data (96-byte instances), shader-side discard (03.7)
- [x] **ClipContext, ClipSegment, TierClips, record_draw_clipped(), record_draw_range_clipped() deleted** (03.7)
- [x] **OverlayDrawRange** simplified — no TierClips field (03.7)
- [x] **InstanceWriter** layout updated to 96 bytes with clip fields (03.7)
- [x] **All WGSL shaders** updated with per-instance clip discard (03.7)
- [x] **DrawList, DrawCommand deleted** (03.8)
- [x] **All source files** under 500 lines (03.0)
- [x] **Scene + DamageTracker + integration tests** pass (03.9)
- [x] `./build-all.sh` passes
- [x] `./test-all.sh` passes
- [x] `./clippy-all.sh` clean

**Exit Criteria:** The entire paint pipeline uses Scene (typed arrays with per-primitive ContentMask) instead of DrawList. Widgets paint via `ctx.scene.push_quad()` etc. SceneCache is removed — full repaint every frame. DamageTracker computes per-widget dirty regions by hashing scene primitives. GPU renderer consumes typed arrays with shader-side clipping. No DrawList, DrawCommand, SceneCache, or ClipSegment references remain.
