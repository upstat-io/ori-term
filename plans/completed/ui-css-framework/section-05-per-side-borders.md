---
section: "05"
title: "Per-Side Borders"
status: complete
reviewed: true
third_party_review:
  status: resolved
  updated: 2026-03-24
goal: "RectStyle and the dedicated UI-rect renderer support per-side border widths and colors, including rounded-corner boxes, while preserving a fast path for the common uniform-border case"
inspired_by:
  - "CSS box model border-top/right/bottom/left"
  - "CSS border painting and corner joins"
depends_on: []
sections:
  - id: "05.1"
    title: "BorderSides Type"
    status: complete
  - id: "05.2"
    title: "RectStyle Integration + Consumer Migration"
    status: complete
  - id: "05.3"
    title: "Dedicated UI Rect Writer + Pipeline"
    status: complete
  - id: "05.4"
    title: "Scene Conversion + Shader Geometry"
    status: complete
  - id: "05.5"
    title: "Consumer Boundaries"
    status: complete
  - id: "05.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "05.6"
    title: "Build & Verify"
    status: complete
---

# Section 05: Per-Side Borders

## Problem

The original draft was directionally right that `RectStyle` needs per-side borders, but it
proposed the wrong implementation boundary and later collapsed the feature back down to a
single-color or square-corner subset.

What the tree actually has today:

- [oriterm_ui/src/draw/border.rs](/home/eric/projects/ori_term/oriterm_ui/src/draw/border.rs)
  defines only a uniform `Border { width, color }`.
- [oriterm_ui/src/draw/rect_style.rs](/home/eric/projects/ori_term/oriterm_ui/src/draw/rect_style.rs)
  stores `border: Option<Border>`.
- [oriterm/src/gpu/scene_convert/mod.rs](/home/eric/projects/ori_term/oriterm/src/gpu/scene_convert/mod.rs)
  converts every styled rect to one UI-rect instance with one `border_width` via
  `push_ui_rect()`. **Also converts `LinePrimitive`s into UI-rect instances** via
  `convert_line_clipped()` — lines use the same writer as quads.
- [oriterm/src/gpu/shaders/ui_rect.wgsl](/home/eric/projects/ori_term/oriterm/src/gpu/shaders/ui_rect.wgsl)
  only understands one border width (`border_width: f32`) and one border color (`fg_color`).
  The SDF function `sd_rounded_box()` takes a single scalar `r` for corner radius.
- [oriterm/src/gpu/prepared_frame/mod.rs](/home/eric/projects/ori_term/oriterm/src/gpu/prepared_frame/mod.rs)
  already keeps `ui_rects` and `overlay_rects` in dedicated `InstanceWriter` buffers, separate
  from glyph and terminal-background buffers. `SavedTerminalTier` does NOT contain UI rect
  fields, so incremental rendering is unaffected by this change.
- [oriterm/src/gpu/pipeline/mod.rs](/home/eric/projects/ori_term/oriterm/src/gpu/pipeline/mod.rs)
  already has a dedicated `UI_RECT_ATTRS` (10 attributes) and `ui_rect_buffer_layout()`, but
  they share the 96-byte `INSTANCE_STRIDE` with the general instance format.
- [oriterm/src/gpu/pipelines.rs](/home/eric/projects/ori_term/oriterm/src/gpu/pipelines.rs)
  stores `ui_rect_pipeline: RenderPipeline` in the shared `GpuPipelines` struct. The pipeline
  is created with `ui_rect_buffer_layout()` at startup.

- [oriterm/src/gpu/window_renderer/scene_append.rs](/home/eric/projects/ori_term/oriterm/src/gpu/window_renderer/scene_append.rs)
  calls `convert_scene()` with `&mut self.prepared.ui_rects` (chrome tier) and
  `&mut self.prepared.overlay_rects` (overlay tier). Both sites must change when the writer
  type changes.

That last point is the key correction: proper support does not have to distort the shared
`InstanceWriter` used by backgrounds and glyphs. The renderer already has a UI-rect-only path.

The mockup uses more than uniform borders:

- sidebar search input uses a uniform border and already works today
- sidebar active nav items use a left accent indicator and already work today via a separate quad
- the sidebar right divider and footer separators already work today via dedicated quads or
  [SeparatorWidget](/home/eric/projects/ori_term/oriterm_ui/src/widgets/separator/mod.rs)
- `.info-callout` and `.warning-callout` need mixed border colors on one box
- `.font-preview-meta` and `.keybind-dialog-buttons` need top-only borders
- the framework should support rounded asymmetric borders correctly instead of declaring them out of
  scope just because the current shader does not yet support them

The real missing capability is:

1. `RectStyle` cannot represent per-side border widths and colors.
2. The UI-rect pipeline cannot render per-side widths and colors.
3. The framework has no proper story for corner ownership when adjacent rounded sides use
   different colors.

## Corrected Scope

Section 05 should build the full renderer support instead of narrowing the feature away.

- Add `BorderSides` to `oriterm_ui` with independent width and color per side.
- Preserve `RectStyle::with_border(width, color)` as the ergonomic shorthand for uniform borders.
- Introduce a dedicated UI-rect instance format and writer for `ui_rects` and `overlay_rects`.
- Leave the shared 96-byte `InstanceWriter` for terminal backgrounds, cursors, and glyphs alone.
- Update the UI-rect shader to consume per-side widths, per-side colors, and four corner radii.
- Keep a fast path for the common uniform-border case inside the dedicated UI-rect path.
- Allow existing dedicated quads and `SeparatorWidget` uses to remain until later fidelity
  sections choose to migrate them.

This is broader than the previous revision, but it is the correct architecture boundary: the UI
rect renderer is already isolated from the rest of the instance pipeline, so this is where full
support belongs.

---

## 05.1 BorderSides Type

### Goal

Represent the feature honestly in `oriterm_ui`: each side can have its own width and its own
color.

### Files

- [oriterm_ui/src/draw/border.rs](/home/eric/projects/ori_term/oriterm_ui/src/draw/border.rs)
- [oriterm_ui/src/draw/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/draw/mod.rs)

Because this type needs dedicated unit tests, convert `border.rs` into a directory module
(`border/mod.rs` + `border/tests.rs`) to satisfy the repository's test-organization rule.

### Proposed Type

```rust
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BorderSides {
    pub top: Option<Border>,
    pub right: Option<Border>,
    pub bottom: Option<Border>,
    pub left: Option<Border>,
}
```

Keep `Border` as the side value type:

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Border {
    pub width: f32,
    pub color: Color,
}
```

### Constructors and Helpers

```rust
impl BorderSides {
    pub fn uniform(width: f32, color: Color) -> Self { ... }
    pub fn only_top(width: f32, color: Color) -> Self { ... }
    pub fn only_right(width: f32, color: Color) -> Self { ... }
    pub fn only_bottom(width: f32, color: Color) -> Self { ... }
    pub fn only_left(width: f32, color: Color) -> Self { ... }

    /// True when no side has a visible border (all absent or all invalid widths).
    pub fn is_empty(&self) -> bool { ... }

    /// Returns `Some(border)` only when all four sides are present, have valid widths
    /// (finite > 0.0), and are identical (same width and color). Used by the scene
    /// conversion fast path. Sides with invalid widths are treated as absent.
    pub fn as_uniform(&self) -> Option<Border> { ... }

    /// Per-side widths as `[top, right, bottom, left]`, normalized for rendering.
    pub fn widths(&self) -> [f32; 4] { ... }

    /// Per-side colors as `[top, right, bottom, left]`, using transparent for absent sides.
    pub fn colors(&self) -> [Color; 4] { ... }
}
```

### Normalization

Widths should be normalized before rendering:

- finite `> 0.0` widths remain visible
- `<= 0.0`, `NaN`, and infinities behave as "no border on that side"

That normalization belongs in the helper/render boundary, not only in builders, because callers
can construct `Border` and `BorderSides` directly.

### Checklist

- [x] Convert `border.rs` into `border/mod.rs` + `border/tests.rs`
- [x] Add `BorderSides`
- [x] Re-export `BorderSides` from `draw/mod.rs`
- [x] Add uniform and single-side constructors
- [x] Add `is_empty()`, `as_uniform()`, `widths()`, and `colors()`
- [x] Normalize invalid widths to "no border"
- [x] Verify `border/mod.rs` stays under 500 lines (currently 16 lines + ~80 new lines)

---

## 05.2 RectStyle Integration + Consumer Migration

### Goal

Thread `BorderSides` through the public style API while keeping uniform-border call sites simple.
Update all consumers of `RectStyle.border` to compile with the new type.

### Files

- [oriterm_ui/src/draw/rect_style.rs](/home/eric/projects/ori_term/oriterm_ui/src/draw/rect_style.rs)
- [oriterm_ui/src/draw/tests.rs](/home/eric/projects/ori_term/oriterm_ui/src/draw/tests.rs)
- [oriterm_ui/src/draw/damage/hash_primitives.rs](/home/eric/projects/ori_term/oriterm_ui/src/draw/damage/hash_primitives.rs)
- [oriterm/src/widgets/terminal_preview/mod.rs](/home/eric/projects/ori_term/oriterm/src/widgets/terminal_preview/mod.rs)

- [oriterm/src/gpu/scene_convert/mod.rs](/home/eric/projects/ori_term/oriterm/src/gpu/scene_convert/mod.rs)

### Field Change

Replace `Option<Border>` with `BorderSides`:

```rust
pub struct RectStyle {
    pub fill: Option<Color>,
    pub border: BorderSides,
    pub corner_radius: [f32; 4],
    pub shadow: Option<Shadow>,
    pub gradient: Option<Gradient>,
}
```

`BorderSides::default()` remains "no border", so `RectStyle::default()` stays invisible by
default.

### Builder Methods

Keep the existing uniform shorthand and add per-side setters:

```rust
impl RectStyle {
    pub fn with_border(mut self, width: f32, color: Color) -> Self {
        self.border = BorderSides::uniform(width, color);
        self
    }

    pub fn with_border_top(mut self, width: f32, color: Color) -> Self { ... }
    pub fn with_border_right(mut self, width: f32, color: Color) -> Self { ... }
    pub fn with_border_bottom(mut self, width: f32, color: Color) -> Self { ... }
    pub fn with_border_left(mut self, width: f32, color: Color) -> Self { ... }
}
```

The side setters should only overwrite the addressed side so callers can compose:

```rust
RectStyle::filled(bg)
    .with_border(2.0, subtle)
    .with_border_left(3.0, accent)
```

That composition pattern is required for the mockup's callouts.

Also add a `with_border_sides(sides: BorderSides)` setter on `RectStyle` for callers that
construct `BorderSides` programmatically (e.g. from theme config or dynamic style computation):

```rust
#[must_use]
pub fn with_border_sides(mut self, sides: BorderSides) -> Self {
    self.border = sides;
    self
}
```

### Consumer Migration

The following call sites reference `RectStyle.border` as `Option<Border>` and must be updated:

1. **`oriterm_ui/src/draw/tests.rs` line 13**: `assert!(s.border.is_none())` becomes
   `assert!(s.border.is_empty())`.
2. **`oriterm_ui/src/draw/tests.rs` lines 40-45**: `assert_eq!(s.border, Some(Border { ... }))`
   becomes `assert_eq!(s.border, BorderSides::uniform(2.0, Color::WHITE))`.

3. **`oriterm/src/widgets/terminal_preview/mod.rs` line 84**: `border: None` becomes
   `border: BorderSides::default()`.
4. **`oriterm_ui/src/draw/damage/hash_primitives.rs` lines 157-164**:
   `match &s.border { Some(b) => { mix(hash,1); hash_f32(hash, b.width); hash_color(hash, b.color); } None => mix(hash, 0) }`
   must change to hash all four sides. **Use `s.border.widths()` and `s.border.colors()`** (not
   raw field access) so the hash is computed over the normalized representation. This ensures
   semantically identical borders produce the same hash regardless of how they were constructed.
   The damage tracker uses hashes to detect visual changes — if two `BorderSides` values with
   different per-side data hash the same, the renderer skips repainting incorrectly.

5. **`oriterm/src/gpu/scene_convert/mod.rs` line 201**: `style.border.map_or(([0.0; 4], 0.0), |b| ...)`
   must change to read `BorderSides`. This is the bridge to 05.3/05.4 — for now it can use
   `border.as_uniform()` to keep the existing uniform-only behavior until the dedicated writer
   replaces the conversion path. **Degradation**: non-uniform borders silently render as no
   border until 05.3/05.4. This is safe because no widget constructs non-uniform borders yet.
   Add a `// TODO(05.4): replace as_uniform() with full per-side conversion` comment.

**Note**: The `RectStyle` field type change from `Option<Border>` to `BorderSides` also breaks the
derived `PartialEq` comparison in `oriterm_ui/src/draw/scene/tests.rs` at line 51
(`assert_eq!(quad.style, style)`) — but that test constructs via `with_border()` builder, so it
compiles as long as `BorderSides` derives `PartialEq`. No code change needed there, but verify
compilation.

### Checklist

- [x] Change `RectStyle.border` to `BorderSides`
- [x] Keep `with_border(width, color)` as the uniform shorthand
- [x] Add `with_border_sides(sides)` programmatic setter

- [x] Add top/right/bottom/left border builders
- [x] Update draw tests to expect `BorderSides` (both `is_none()` at line 13 and `== Some(Border{..})`
  at line 40 — two separate assertions break)

- [x] Update `terminal_preview/mod.rs` struct literal

- [x] Update `damage/hash_primitives.rs` `hash_rect_style()` to hash all four sides of
  `BorderSides` instead of matching `Option<Border>`

- [x] Update `scene_convert/mod.rs` `convert_rect_clipped()` to bridge with `BorderSides` API
  (interim: use `as_uniform()` fallback until 05.3/05.4 add full per-side conversion)

- [x] Verify `oriterm_ui/src/draw/scene/tests.rs` still compiles (uses `RectStyle` equality via
  `assert_eq!(quad.style, style)` — requires `BorderSides: PartialEq`)

- [x] Verify `./build-all.sh` passes after this subsection (the project uses `dead_code = "deny"`,
  so any missed consumer will fail compilation)

---

## 05.3 Dedicated UI Rect Writer + Pipeline

### Goal

Introduce a dedicated UI-rect instance writer with a 144-byte stride that carries per-side
border data. Update the UI-rect pipeline's vertex buffer layout to match. Leave the shared
96-byte `InstanceWriter` for terminal backgrounds, cursors, and glyphs unchanged.

### Files

- [oriterm/src/gpu/prepared_frame/mod.rs](/home/eric/projects/ori_term/oriterm/src/gpu/prepared_frame/mod.rs) (430 lines)

- [oriterm/src/gpu/pipeline/mod.rs](/home/eric/projects/ori_term/oriterm/src/gpu/pipeline/mod.rs) (512 lines — over 500-line limit)
- [oriterm/src/gpu/pipeline/tests.rs](/home/eric/projects/ori_term/oriterm/src/gpu/pipeline/tests.rs)
- [oriterm/src/gpu/pipelines.rs](/home/eric/projects/ori_term/oriterm/src/gpu/pipelines.rs)

- [oriterm/src/gpu/window_renderer/render.rs](/home/eric/projects/ori_term/oriterm/src/gpu/window_renderer/render.rs) (706 lines — over 500-line limit)
- [oriterm/src/gpu/window_renderer/scene_append.rs](/home/eric/projects/ori_term/oriterm/src/gpu/window_renderer/scene_append.rs)

- [oriterm/src/gpu/window_renderer/mod.rs](/home/eric/projects/ori_term/oriterm/src/gpu/window_renderer/mod.rs) (483 lines — under 500-line limit)

### Corrected File Size Claims

The previous draft claimed `window_renderer/mod.rs` was over the 500-line limit. It is 483
lines — under the limit. The files that ARE over are:
- `pipeline/mod.rs` at 512 lines
- `window_renderer/render.rs` at 706 lines

**Mandatory pre-work**: `pipeline/mod.rs` is already at 512 lines — over the 500-line limit.
Before adding a new `UI_RECT_ATTRS` array and stride constant, extract UI-rect-specific pipeline
code into a submodule (e.g. `pipeline/ui_rect.rs`) to bring `pipeline/mod.rs` under the limit.
The extraction should include `UI_RECT_ATTRS`, `ui_rect_buffer_layout()`, and
`create_ui_rect_pipeline()`. Re-export from `pipeline/mod.rs`.

### New Dedicated Writer

Introduce a dedicated writer for UI rects:

- `oriterm/src/gpu/ui_rect_writer/mod.rs` (with `#[cfg(test)] mod tests;` at bottom)
- `oriterm/src/gpu/ui_rect_writer/tests.rs`

The writer must expose the same API surface as `InstanceWriter` for the `PreparedFrame` methods
that operate on it:
- `new()`, `clear()`, `is_empty()`, `len()`, `as_bytes()`, `byte_len()`
- `maybe_shrink()`, `extend_from()`, `swap_buf()` (if needed — but `SavedTerminalTier` does
  NOT include UI rects, so `swap_buf` is not needed for incremental rendering)

`PreparedFrame.ui_rects` and `PreparedFrame.overlay_rects` should switch from `InstanceWriter` to
this dedicated writer. The shared background/glyph/cursor writers stay unchanged.

### Proposed UI Rect Instance Layout

144 bytes per instance, 10 vertex attributes (same count as current `UI_RECT_ATTRS`, within
wgpu's 16-attribute limit):

```text
 0  pos: vec2<f32>            (@location(0))
 8  size: vec2<f32>           (@location(1))
16  clip: vec4<f32>           (@location(2))
32  fill_color: vec4<f32>     (@location(3))
48  border_widths: vec4<f32>  (@location(4))  // [top, right, bottom, left]
64  corner_radii: vec4<f32>   (@location(5))  // [tl, tr, br, bl]
80  border_top: vec4<f32>     (@location(6))  // RGBA linear
96  border_right: vec4<f32>   (@location(7))  // RGBA linear
112 border_bottom: vec4<f32>  (@location(8))  // RGBA linear
128 border_left: vec4<f32>    (@location(9))  // RGBA linear
```

That gives the shader every value it needs directly, without stealing fields from glyph instances
or creating hidden dual meanings for `uv`, `kind`, or `atlas_page`.

Key design notes:

- **`uv`, `kind`, and `atlas_page` are dropped**: The current UI rect shader (`ui_rect.wgsl`)
  reads these from the instance input but does not use them in the SDF fragment shader. The UI
  rect pipeline does not bind any atlas texture (only the uniform bind group). Removing these
  fields is correct.
- **Border colors are in linear RGBA `[f32; 4]`**: The scene conversion layer applies
  `srgb_f32_to_linear()` and opacity multiplication before writing, matching the current
  `color_to_linear_with_opacity()` pattern used by `convert_rect_clipped()`.
- **`corner_radii` is vec4**: Passes all four radii directly. The current `uniform_radius()`
  collapse happens in `convert_rect_clipped()` — that function will be updated in 05.4 to
  pass through all four radii instead.

### Push Method

The writer needs a single push method that both `convert_rect_clipped()` (styled quads) and
`convert_line_clipped()` (line segments) will call:

```rust
impl UiRectWriter {
    pub fn push_ui_rect(
        &mut self,
        rect: ScreenRect,
        fill: [f32; 4],
        border_widths: [f32; 4],
        corner_radii: [f32; 4],
        border_colors: [[f32; 4]; 4],  // [top, right, bottom, left]
        clip: [f32; 4],
    ) { ... }
}
```

Lines call this with `border_widths = [0.0; 4]`, `corner_radii = [0.0; 4]`, and all border
colors transparent — same as the current `push_ui_rect` call pattern for lines.

**`ScreenRect` reuse**: `scene_convert/mod.rs` currently imports `ScreenRect` from
`instance_writer`. The new `UiRectWriter` should reuse the same `ScreenRect` type (import from
`instance_writer`) rather than defining a duplicate. `ScreenRect` is a simple value type
(`{x, y, w, h}` with a `scaled()` method) and is not conceptually owned by either writer.

### Pipeline Layout Update

The `ui_rect_buffer_layout()` function in `pipeline/mod.rs` must return a `VertexBufferLayout`
with:
- `array_stride: 144` (was 96)
- 10 `VertexAttribute`s with the new offsets shown above

The `UI_RECT_ATTRS` array must be replaced with the new attribute definitions. The old
`UI_RECT_ATTRS` shared locations 0-6 with `INSTANCE_ATTRS` — the new layout is fully
independent and does NOT share attribute definitions with the terminal instance format.

**Important**: The current `ui_rect_buffer_layout()` uses `INSTANCE_STRIDE` (96 bytes) for
`array_stride`. The new layout needs its own stride constant (e.g.
`pub const UI_RECT_INSTANCE_SIZE: usize = 144;`) declared in the `ui_rect_writer` module and
used by both the writer and the pipeline layout. Do NOT reuse `INSTANCE_STRIDE` or
`INSTANCE_SIZE` — those belong to the shared terminal instance format and must stay at 96 bytes.

`create_ui_rect_pipeline()` already calls `ui_rect_buffer_layout()`, so the pipeline
automatically picks up the new stride. The pipeline in `GpuPipelines.ui_rect_pipeline` is
rebuilt at startup — no runtime reconfiguration needed.

### Render Path Impact

The `upload!` macro in `render.rs` calls `.as_bytes()` and `.len()` on each writer. The new
writer must expose these same methods. Key observations:

- **`record_draw()` and `record_draw_range()`**: These use `pass.draw(0..4, 0..instance_count)`.
  wgpu reads instances using the pipeline's `array_stride` — so changing stride from 96 to 144
  only requires that the vertex buffer layout matches. The draw call dispatch code does NOT
  hardcode stride values; it just passes instance counts. **No structural changes needed.**
- **`upload_buffer()` in `helpers.rs`**: Takes raw `&[u8]` — stride-agnostic. Works as-is.
- **`OverlayDrawRange.rects` ranges**: Based on `len()` (instance count). The new writer's
  `len()` must return instance count (byte_len / 144), not byte count. The overlay range
  snapshot code in `scene_append.rs` (lines 64, 91) reads `self.prepared.overlay_rects.len()`
  — this works unchanged if `len()` semantics are preserved.
- **`upload_overlay_and_cursor_buffers()`**: Also uses the `upload!` macro pattern on
  `overlay_rects`. Works unchanged with new writer.

### Checklist

- [x] Extract UI-rect pipeline code from `pipeline/mod.rs` into `pipeline/ui_rect.rs` to bring
  `pipeline/mod.rs` under the 500-line limit (move `UI_RECT_ATTRS`, `ui_rect_buffer_layout()`,
  `create_ui_rect_pipeline()`, re-export from `pipeline/mod.rs`)

- [x] Add a dedicated `UiRectWriter` module at `oriterm/src/gpu/ui_rect_writer/mod.rs` + `tests.rs`
- [x] Declare `pub(crate) mod ui_rect_writer;` in `oriterm/src/gpu/mod.rs`

- [x] Add `pub const UI_RECT_INSTANCE_SIZE: usize = 144;` to `ui_rect_writer/mod.rs`

- [x] Implement `push_ui_rect()`, `new()`, `clear()`, `len()`, `is_empty()`, `as_bytes()`,
      `byte_len()`, `maybe_shrink()`, `extend_from()`
- [x] Switch `PreparedFrame.ui_rects` and `overlay_rects` from `InstanceWriter` to `UiRectWriter`
  (both `new()` and `with_capacity()` constructors initialize these fields)

- [x] Update `PreparedFrame` methods: `clear()`, `clear_ephemeral_tiers()`, `extend_from()`,
      `maybe_shrink()`, `is_empty()`, `total_instances()`, `count_draw_calls()`
- [x] Replace `UI_RECT_ATTRS` in `pipeline/mod.rs` with the new 144-byte attribute array
- [x] Update `ui_rect_buffer_layout()` to return stride 144
- [x] Verify `create_ui_rect_pipeline()` still compiles (it calls `ui_rect_buffer_layout()`)
- [x] Update `scene_append.rs` — `convert_scene()` calls now pass `&mut UiRectWriter` instead
      of `&mut InstanceWriter`

- [x] Verify `render.rs` `upload!` macro and `record_draw` calls compile — they access
  `.as_bytes()` and `.len()` on `self.prepared.ui_rects` / `overlay_rects` via macro expansion
  (no code change needed if `UiRectWriter` exposes the same method names, but verify compilation)

- [x] Update pipeline tests in `pipeline/tests.rs` for the new attribute layout
- [x] Keep the shared `InstanceWriter` and non-UI pipelines unchanged

---

## 05.3+05.4 Implementation Constraint

**05.3 and 05.4 MUST be implemented as a single atomic unit.** The pipeline vertex buffer
layout (05.3) and the WGSL shader `InstanceInput` struct (05.4) must match exactly. After
completing 05.3 alone, the shader expects the old 96-byte layout while the buffer provides
144-byte records — the pipeline will fail validation or render garbage. Do not attempt to
leave the codebase in a "05.3 done, 05.4 pending" state. Implement them together, test
together.

---

## 05.4 Scene Conversion + Shader Geometry

> **HIGH COMPLEXITY**: This subsection requires coordinated changes to WGSL shader code, Rust
> GPU pipeline configuration, scene conversion, and a custom SDF implementation. Budget extra
> time and expect iteration. The corner ownership math should be prototyped and unit-tested on
> the CPU side before porting to WGSL.

### Goal

Update scene conversion to populate the new 144-byte instance format and render the full
feature set correctly in the shader:

- per-side widths
- per-side colors
- four independent corner radii
- deterministic joins where adjacent border sides meet

### Files

- [oriterm/src/gpu/scene_convert/mod.rs](/home/eric/projects/ori_term/oriterm/src/gpu/scene_convert/mod.rs) (339 lines)
- [oriterm/src/gpu/scene_convert/tests.rs](/home/eric/projects/ori_term/oriterm/src/gpu/scene_convert/tests.rs)
- [oriterm/src/gpu/shaders/ui_rect.wgsl](/home/eric/projects/ori_term/oriterm/src/gpu/shaders/ui_rect.wgsl) (128 lines)

### Scene Conversion Changes

`convert_scene()` signature currently takes `&mut InstanceWriter` as `ui_writer`:

```rust
// Current:
pub fn convert_scene(
    scene: &Scene,
    ui_writer: &mut InstanceWriter,
    text_ctx: Option<&mut TextContext<'_>>,
    scale: f32,
    opacity: f32,
)

// New:
pub fn convert_scene(
    scene: &Scene,
    ui_writer: &mut UiRectWriter,     // changed type
    text_ctx: Option<&mut TextContext<'_>>,
    scale: f32,
    opacity: f32,
)
```

The following functions inside `scene_convert/mod.rs` pass the writer through and must also
change their parameter type:

- `convert_quad()` (line 96)
- `convert_rect_clipped()` (line 164)
- `convert_scene_line()` (line 107)
- `convert_line_clipped()` (line 225)

**`convert_rect_clipped()`** must be updated to populate the full 144-byte instance:

```rust
fn convert_rect_clipped(
    rect: Rect,
    style: &RectStyle,
    writer: &mut UiRectWriter,
    scale: f32,
    opacity: f32,
    clip: [f32; 4],
) {
    // fill color (unchanged)
    // shadow instance: zero border widths, zero border colors, corner_radii = outer + expand
    //   — same as today but with the new push_ui_rect signature
    // main instance:
    //   - border_widths from style.border.widths() * scale  (each element scaled)
    //   - corner_radii from style.corner_radius * scale (pass all four, NOT uniform_radius())
    //   - border_colors from style.border.colors() through color_to_linear_with_opacity()
}
```

Note: the shadow instance currently uses `uniform_radius(&style.corner_radius) + expand` (line 193
in `convert_rect_clipped`). With the new writer, pass all four radii each expanded by `expand`:
`[tl + expand, tr + expand, br + expand, bl + expand]`, each multiplied by `scale`. This ensures
shadows around rects with per-corner radii produce correctly rounded shadows.

Key change: **remove the `uniform_radius()` call** and pass all four corner radii directly.
The current `uniform_radius()` takes `max()` of all four radii — this was a workaround for the
single-radius shader. With the new four-corner SDF, pass `[tl, tr, br, bl] * scale` directly.

**`convert_line_clipped()`** continues to write zero-border-width instances. Its only change is
the writer type and calling `push_ui_rect()` with the new signature (all border data zeroed).

**Text and icon conversion are unaffected.** `convert_text()` and `convert_icon()` write to
`TextContext`'s `mono_writer`, `subpixel_writer`, and `color_writer` — all of which remain
`InstanceWriter` (96-byte). Only the `ui_writer` parameter of `convert_scene()` changes type.
The extensive text/icon tests in `scene_convert/tests.rs` (57 tests) remain structurally
unchanged — they construct their own `InstanceWriter` for glyph output. However, they will
need the `convert_scene()` first argument changed from `&mut InstanceWriter` to
`&mut UiRectWriter` since that parameter type changes in the signature.

### Shader Responsibilities

The shader (`ui_rect.wgsl`) needs a complete rewrite of its instance input struct and fragment
shader logic. The vertex shader changes are minimal (pos/size/clip math is the same).

**New `InstanceInput` struct:**

```wgsl
struct InstanceInput {
    @location(0) pos: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) clip: vec4<f32>,
    @location(3) fill_color: vec4<f32>,
    @location(4) border_widths: vec4<f32>,   // [top, right, bottom, left]
    @location(5) corner_radii: vec4<f32>,    // [tl, tr, br, bl]
    @location(6) border_top: vec4<f32>,      // RGBA linear
    @location(7) border_right: vec4<f32>,
    @location(8) border_bottom: vec4<f32>,
    @location(9) border_left: vec4<f32>,
}
```

**Fragment shader needs three pieces of logic:**

1. **Outer shape mask** — four-corner rounded-rect SDF
   Replace the current `sd_rounded_box(p, half_size, r)` (single scalar `r`) with a
   four-corner variant. The standard approach: select the radius for the quadrant the fragment
   is in, then evaluate the quarter-circle SDF for that corner. This is a well-known SDF
   technique (see Inigo Quilez, Evan Wallace/Figma SDF article).

2. **Inner content mask** — asymmetric inset
   The inner shape is the outer shape inset by per-side border widths. Because
   left/right and top/bottom widths can differ, the inner box is NOT centered on the outer
   box's center. Compute `inner_half_size` and `inner_radii` from the outer rect and per-side
   widths. The inner SDF must be evaluated at `local_pos - center_offset` where
   `center_offset = vec2((left - right) * 0.5, (top - bottom) * 0.5)` to account for the
   shifted inner rect center. Inner half-size is
   `half_size - vec2((left + right) * 0.5, (top + bottom) * 0.5)`. Inner corner radii are
   `max(outer_radius - max(adjacent_width_1, adjacent_width_2), 0.0)` per CSS spec.

3. **Border-side ownership** — per-fragment color selection
   For each fragment in the border ring (between outer and inner SDF), determine which side
   "owns" it. The straight edge regions are trivial (top band, right band, etc.). Corner
   regions where two adjacent sides meet must be split.

### Corner Ownership Rule

Do not fall back to "first non-transparent side wins" or "left side owns the full corner." That is
the kind of shortcut that makes mixed-color rounded borders look wrong immediately.

Instead, define an explicit corner ownership rule and use it consistently:

- each edge band owns its straight edge region
- each rounded corner is split between its two adjacent sides along a 45-degree diagonal
  from the corner of the outer rect toward the inner rect

- the diagonal split matches CSS browser behavior closely enough that mockup comparisons agree
- for square corners (radius = 0), the split is a simple miter line at 45 degrees

The math: for a fragment at position `p` relative to the rect center, compute
`quadrant = sign(p)`. In the top-left quadrant, the split between top and left ownership is
along the line `y - center_y == x - center_x` (relative to the corner origin). Fragments
above the diagonal belong to the top side, below to the left side. The same pattern applies to
all four corners.

### `VertexOutput` Changes

The vertex shader must pass through all per-side data to the fragment shader. The current
`VertexOutput` has 8 `@location` outputs. The new version will have more:

```wgsl
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) fill_color: vec4<f32>,
    @location(1) local_pos: vec2<f32>,
    @location(2) half_size: vec2<f32>,
    @location(3) corner_radii: vec4<f32>,
    @location(4) border_widths: vec4<f32>,
    @location(5) border_top: vec4<f32>,
    @location(6) border_right: vec4<f32>,
    @location(7) border_bottom: vec4<f32>,
    @location(8) border_left: vec4<f32>,
    @location(9) clip_min: vec2<f32>,
    @location(10) clip_max: vec2<f32>,
}
```

That is 11 inter-stage `@location` slots. The WebGPU spec guarantees
`maxInterStageShaderVariables >= 16` (locations) and `maxInterStageShaderComponents >= 60`
(scalar components). These 11 locations use 36 scalar components
(4+2+2+4+4+4+4+4+4+2+2), well within both limits.

### Uniform Fast Path

Preserve a fast path inside the fragment shader when:

- all four border widths are equal
- all four border colors are equal
- all four corner radii are equal

That lets the common button/input/dialog case stay on the simple branch even after the dedicated
UI-rect format grows. The check can be a single `all()` comparison in WGSL:

```wgsl
let bw = input.border_widths;
let is_uniform = bw.x == bw.y && bw.y == bw.z && bw.z == bw.w
    && all(input.border_top == input.border_right)
    && all(input.border_right == input.border_bottom)
    && all(input.border_bottom == input.border_left)
    && input.corner_radii.x == input.corner_radii.y
    && input.corner_radii.y == input.corner_radii.z
    && input.corner_radii.z == input.corner_radii.w;
```

### Checklist

- [x] Update `convert_scene()` signature to take `&mut UiRectWriter`
- [x] Update `scene_convert/mod.rs` imports: add `UiRectWriter` (from `ui_rect_writer`), keep
  `ScreenRect` (from `instance_writer`)
- [x] Update `convert_quad()`, `convert_rect_clipped()`, `convert_scene_line()`,
      `convert_line_clipped()` signatures

- [x] Remove `uniform_radius()` function — replace with direct four-corner radii passthrough
- [x] Update shadow instance in `convert_rect_clipped()` to pass four per-corner expanded radii
  instead of `uniform_radius() + expand`

- [x] Update `convert_rect_clipped()` to populate all 144-byte fields from `BorderSides`
  (this replaces the `as_uniform()` interim bridge from 05.2 — remove the TODO comment)
- [x] Update `convert_line_clipped()` to call new `push_ui_rect()` with zero border data

- [x] Rewrite `ui_rect.wgsl` `InstanceInput` struct with 10 new attributes
- [x] Rewrite `ui_rect.wgsl` `VertexOutput` struct with per-side data passthrough
- [x] Implement four-corner rounded-rect outer SDF in WGSL
- [x] Implement asymmetric inset inner SDF in WGSL
- [x] Implement explicit side ownership for corner regions (diagonal split)
- [x] Preserve a fast uniform branch for common cases
- [x] Update `scene_convert/tests.rs` to use `UiRectWriter` instead of `InstanceWriter`.
  Existing tests (`filled_rect_produces_one_instance`, `rect_with_border_writes_border_fields`,
  etc.) verify byte offsets based on the 96-byte layout (e.g., fill at offset 48, kind at offset
  64). These must be rewritten to verify the 144-byte layout offsets (fill at offset 32, border
  widths at offset 48, etc.). Also add new per-side border tests listed in 05.6.

- [x] Preserve all 57 existing `scene_convert/tests.rs` test scenarios under new byte offsets.
  Key tests that verify byte offsets and must be rewritten: `filled_rect_produces_one_instance`,
  `rect_with_border_writes_border_fields`, `rect_with_shadow_produces_two_instances`,
  `horizontal_line_converts_to_rect`, `invisible_rect_still_writes_instance`,
  `uniform_radius_picks_max_of_four_corners`, `all_corners_zero_is_sharp_rect`,
  `radius_larger_than_half_dimension_passes_through`. Tests that only check `len()` or
  `is_empty()` (most text/icon tests) only need the `ui_writer` type changed. Count test
  functions before and after migration to ensure no coverage is lost.

- [x] Rewrite `uniform_radius_picks_max_of_four_corners` to verify all four radii pass through
  individually to the 144-byte instance (at `corner_radii` offset 64, a `vec4<f32>` with
  `[tl, tr, br, bl]`). The old test asserted `max()` collapse — the new test must assert all
  four values are preserved exactly.

- [x] Rewrite `all_corners_zero_is_sharp_rect` and `radius_larger_than_half_dimension_passes_through`
  to read from the new 144-byte `corner_radii` field instead of the old single `corner_radius` offset.

---

## 05.5 Consumer Boundaries

### Goal

Clarify what Section 05 adds and what existing widgets may continue doing until later fidelity
sections choose to migrate them.

### Current Consumer Reality

Some mockup border effects already render correctly today with dedicated primitives:

- [sidebar_nav/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/sidebar_nav/mod.rs)
  already paints the sidebar right divider as a separate quad
- [sidebar_nav/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/sidebar_nav/mod.rs)
  already paints the active nav indicator as a separate accent quad
- [settings_panel/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/settings_panel/mod.rs)
  already paints footer separators with [SeparatorWidget](/home/eric/projects/ori_term/oriterm_ui/src/widgets/separator/mod.rs)

Those do not need to be rewritten immediately just because `RectStyle` becomes more expressive.

### Correct Promise

Section 05 should build the shared capability. Later fidelity sections can decide when to migrate
real widgets to a single styled rect.

That means:

1. Existing dedicated lines/quads may stay where they are already correct.
2. Widgets that need one box with asymmetric borders can adopt `RectStyle` once the renderer
   supports it.
3. Rounded multi-color boxes are in scope for the renderer capability even if no current widget
   adopts them immediately.

### Checklist

- [x] Do not claim existing sidebar/footer dividers are blocked on this section
- [x] Limit this section to shared style and renderer capability
- [x] Let later fidelity sections choose concrete widget migrations

---

## 05.R Third Party Review Findings

- [x] `[TPR-05-010][medium]` `oriterm/src/gpu/prepared_frame/mod.rs:339` — `PreparedFrame::extend_from()` shifts overlay draw ranges after it has already appended the overlay buffers, so merged overlay ranges point past the newly appended instances.
  Evidence: the helper appends `overlay_rects`/`overlay_*glyphs` first, then derives `bases` from the post-append lengths at [prepared_frame/mod.rs](/home/eric/projects/ori_term/oriterm/src/gpu/prepared_frame/mod.rs). If `self.overlay_rects.len() == 2` and `other.overlay_draw_ranges[0].rects == (0, 1)`, appending `other.overlay_rects` makes the new base `3`, so the shifted range becomes `(3, 4)` even though the appended rect lives at index `2`.
  Impact: any caller that composes a `PreparedFrame` containing overlays will record out-of-bounds draw ranges, so the appended overlay rect/glyph passes are skipped or misindexed. `prepared_frame/tests.rs` currently has no regression coverage for `extend_from()` with `overlay_draw_ranges`, so this contract bug can land unnoticed.
  Required plan update: capture the overlay buffer base lengths before any `extend_from()` calls, then add a unit test that merges two frames with non-empty `overlay_draw_ranges` and verifies the shifted ranges still address the appended instances.
  Resolved 2026-03-24: accepted. Moved `bases` capture before the `extend_from()` calls so indices reflect pre-append buffer lengths. Added regression test `extend_from_shifts_overlay_draw_ranges_correctly` in `prepared_frame/tests.rs`.

- [x] `[TPR-05-009][medium]` `oriterm_ui/src/draw/damage/hash_primitives.rs:156` — per-side border hashing still treats colors on zero-width/invalid sides as meaningful changes, so semantically invisible border edits spuriously dirty widgets.
  Resolved 2026-03-24: accepted. Fixed `BorderSides::side_color()` to return `Color::TRANSPARENT` for sides with invalid widths (zero, negative, NaN, infinite), matching the `widths()` normalization. Added regression test `color_change_on_zero_width_border_produces_no_damage` in `damage/tests.rs` and unit test `border_sides_colors_transparent_for_zero_width` in `border/tests.rs`.

- [x] `[TPR-05-008][medium]` `plans/ui-css-framework/section-05-per-side-borders.md:843` — Section 05 still marks the file-size TPR resolved even though `window_renderer/render.rs` remains over the repository hard limit in the current tree.
  Resolved 2026-03-24: accepted and fixed — extracted upload methods and draw-pass recording helpers from `render.rs` (706 lines) into `render_helpers.rs` (404 lines). Remaining `render.rs` is 327 lines. Both under the 500-line limit. Module registered in `window_renderer/mod.rs`.

- [x] `[TPR-05-006][high]` `oriterm/src/gpu/shaders/ui_rect.wgsl:147` — the new UI-rect shader still does not implement the required diagonal corner split, so mixed-color rounded borders render the wrong side in corner regions.
  Evidence: `fs_main()` delegates border ownership to `select_border_color()` (`oriterm/src/gpu/shaders/ui_rect.wgsl:146-154`), but that helper never computes the diagonal described in the plan. It compares only nearest-edge distances (`oriterm/src/gpu/shaders/ui_rect.wgsl:177-196`), and the corner fallback always returns the top or bottom color. Border widths are not part of the ownership decision, so an absent side can still win a rounded corner.
  Impact: the exact cases Section 05 claims to unlock — rounded mixed-color callouts, rounded top-only separators, and other asymmetric borders — still paint incorrect corner wedges or transparent gaps instead of CSS-like joins.
  Required plan update: implement an explicit per-corner diagonal ownership rule that respects the two sides adjacent to that corner, and add shader/CPU regressions for mixed-color rounded corners and one-sided rounded borders.
  Resolved 2026-03-24: accepted. Rewrote `select_border_color()` with CSS 2.1 diagonal corner ownership: the split line from outer corner to inner corner is determined by the ratio of adjacent border widths (`near_v * h_bw` vs `near_h * v_bw`), so wider sides claim proportionally more of the corner. Zero-width sides never win ownership.

- [x] `[TPR-05-007][low]` `oriterm/src/gpu/shaders/ui_rect.wgsl:116` — Section 05 marks the uniform fast path complete, but the shader still only has the general per-side branch.
  Evidence: the section explicitly requires an `is_uniform` fast path before the general border logic (`plans/ui-css-framework/section-05-per-side-borders.md:711-754`), yet `fs_main()` only checks `has_border` and always runs the asymmetric inner-mask plus `select_border_color()` path for bordered rects (`oriterm/src/gpu/shaders/ui_rect.wgsl:116-157`). No uniform-branch predicate exists in the WGSL.
  Impact: common uniform-border widgets (buttons, inputs, dialogs) pay the more expensive general path even though the checklist records that optimization as done, so the implementation and plan state disagree.
  Required plan update: either add the promised uniform fast branch with coverage or reopen the completed checklist item until it exists.
  Resolved 2026-03-24: accepted. Added `is_uniform` predicate in `fs_main()` that checks all four widths equal and all four colors equal. Uniform borders skip `select_border_color()` entirely and use `input.border_top` directly.

- [x] `[TPR-05-001][high]` [plans/ui-css-framework/section-05-per-side-borders.md](/home/eric/projects/ori_term/plans/ui-css-framework/section-05-per-side-borders.md) - The original draft said the goal was per-side widths and colors, then later restricted the implementation to one shared border color. That is incompatible with the mockup's callouts, which combine a subtle perimeter with a stronger left accent on the same box. Resolved: the section now treats per-side colors as first-class data on 2026-03-23.

- [x] `[TPR-05-002][medium]` [plans/ui-css-framework/section-05-per-side-borders.md](/home/eric/projects/ori_term/plans/ui-css-framework/section-05-per-side-borders.md) - The previous revision scoped rounded and mixed-color cases out of the section and replaced them with a square-corner CPU decomposition plan. That makes the plan easier, but it removes framework capability the user explicitly asked to support properly. Resolved: the section now keeps those cases in scope and routes them through the dedicated UI-rect renderer on 2026-03-23.

- [x] `[TPR-05-003][medium]` [plans/ui-css-framework/section-05-per-side-borders.md](/home/eric/projects/ori_term/plans/ui-css-framework/section-05-per-side-borders.md) - Earlier drafts treated the shared 96-byte `InstanceWriter` as the only available implementation path. That was the wrong boundary: UI rects already have dedicated buffers and a dedicated pipeline. Resolved: the section now introduces a dedicated UI-rect writer and buffer layout instead of overloading the glyph/background path on 2026-03-23.

- [x] `[TPR-05-004][medium]` [plans/ui-css-framework/section-05-per-side-borders.md](/home/eric/projects/ori_term/plans/ui-css-framework/section-05-per-side-borders.md) - The renderer still collapses four corner radii with `uniform_radius()`, so any plan that claims proper rounded asymmetric borders without addressing the shader geometry is incomplete. Resolved: the section now explicitly includes four-corner outer masking, asymmetric inner masking, and corner ownership in the UI-rect shader work on 2026-03-23.

- [x] `[TPR-05-005][medium]` [plans/ui-css-framework/section-05-per-side-borders.md](/home/eric/projects/ori_term/plans/ui-css-framework/section-05-per-side-borders.md) - [pipeline/mod.rs](/home/eric/projects/ori_term/oriterm/src/gpu/pipeline/mod.rs) (512 lines) and [window_renderer/render.rs](/home/eric/projects/ori_term/oriterm/src/gpu/window_renderer/render.rs) (706 lines) are already over the repository's 500-line source-file limit. (`window_renderer/mod.rs` at 483 lines is under the limit — the original report was wrong.) A realistic plan must include extraction work instead of silently growing them further. Resolved: the section now requires UI-rect-specific submodules as part of the implementation on 2026-03-23.

---

## 05.6 Build & Verify

### Gate

```bash
./build-all.sh
./clippy-all.sh
./test-all.sh
```

### Focused Verification

1. `cargo test -p oriterm_ui draw`
2. `cargo test -p oriterm scene_convert`
3. `cargo test -p oriterm pipeline`
4. `cargo test -p oriterm ui_rect_writer`

5. Manual or screenshot-based verification against the mockup for:
   - mixed-color left-accent callouts
   - top-only separators on filled boxes
   - rounded asymmetric-border synthetic cases

### Completion Criteria

- `RectStyle` can represent top/right/bottom/left borders independently
- the UI-rect renderer accepts per-side widths, per-side colors, and four corner radii
- mixed-color borders on one box render correctly
- rounded asymmetric borders render correctly
- uniform-border widgets still render correctly and stay on a simple fast branch
- the implementation does not expand the shared glyph/background instance format
- `convert_line_clipped()` still produces valid UI rect instances (zero-border)

### Tests

In `oriterm_ui/src/draw/border/tests.rs`:
- `fn border_sides_default_is_empty()` — `BorderSides::default().is_empty()` is true
- `fn border_sides_uniform_all_sides_equal()` — `uniform()` sets all four sides identically
- `fn border_sides_as_uniform_returns_some_when_identical()` — `as_uniform()` returns `Some` for uniform borders
- `fn border_sides_as_uniform_returns_none_when_different()` — `as_uniform()` returns `None` when sides differ
- `fn border_sides_as_uniform_returns_none_when_colors_differ()` — `as_uniform()` returns `None` when widths match but colors differ

- `fn border_sides_only_top_leaves_others_none()` — `only_top()` sets top, leaves others `None`
- `fn border_sides_only_right_leaves_others_none()` — same pattern for `only_right()`
- `fn border_sides_only_bottom_leaves_others_none()` — same pattern for `only_bottom()`
- `fn border_sides_only_left_leaves_others_none()` — same pattern for `only_left()`

- `fn border_sides_widths_returns_correct_array()` — `widths()` returns `[top, right, bottom, left]` with 0.0 for absent sides
- `fn border_sides_colors_uses_transparent_for_absent()` — `colors()` uses transparent for absent sides
- `fn border_sides_normalizes_invalid_width()` — NaN, negative, and zero widths normalize to "no border"
- `fn border_sides_normalizes_infinity_width()` — `f32::INFINITY` width normalizes to "no border"

- `fn border_sides_partial_eq_distinguishes_sides()` — two `BorderSides` differing only on one side are not equal (verifies derived `PartialEq`)

In `oriterm_ui/src/draw/tests.rs`:
- `fn rect_style_default_border_is_empty()` — update existing `rect_style_default_is_invisible` to assert `s.border.is_empty()` instead of `s.border.is_none()`

- `fn rect_style_with_border_creates_uniform()` — `with_border()` creates `BorderSides::uniform()`
- `fn rect_style_with_border_top_only_sets_top()` — per-side builder sets only the addressed side
- `fn rect_style_composable_border_sides()` — `with_border().with_border_left()` composes correctly
- `fn rect_style_with_border_overrides_previous()` — `with_border_left(3, accent).with_border(2, subtle)` replaces all sides (uniform after per-side)

In `oriterm_ui/src/draw/damage/tests.rs` (existing file):
- `fn hash_rect_style_distinguishes_border_sides()` — two `RectStyle` values with different per-side border data produce different damage hashes. Create two scenes with identical quads (same widget ID) except for border sides, verify `DamageTracker` detects the change (second frame reports `has_damage()`).
- `fn hash_rect_style_same_border_produces_same_hash()` — identical `BorderSides` values produce the same hash across frames (second frame reports no damage).
- `fn hash_rect_style_uniform_vs_explicit_same_hash()` — `BorderSides::uniform(2.0, Color::WHITE)` and manually constructing all four sides as `Some(Border { width: 2.0, color: Color::WHITE })` produce the same hash (semantically identical borders must not cause false damage).

In `oriterm/src/gpu/ui_rect_writer/tests.rs`:
- `fn ui_rect_instance_stride_is_144_bytes()` — byte layout matches the declared stride
- `fn ui_rect_writer_count_matches_instances()` — `len()` returns instance count, not byte count
- `fn ui_rect_writer_clear_resets_to_zero()` — `clear()` resets count but retains capacity
- `fn ui_rect_writer_maybe_shrink_honors_4x_threshold()` — verify `maybe_shrink()` follows the
  project's shrink discipline: `if capacity > 4 * len && capacity > 4096 -> shrink_to(len * 2)`

- `fn ui_rect_writer_push_writes_correct_offsets()` — verify pos, size, clip, fill, border_widths, corner_radii, and border color fields at expected byte offsets

- `fn ui_rect_writer_push_uniform_border_writes_equal_sides()` — push a uniform border, verify all four border color slots contain the same values and all four widths are equal

- `fn ui_rect_writer_push_zero_border_all_transparent()` — push with zero border widths, verify border color slots are all [0,0,0,0]

- `fn ui_rect_writer_extend_from_appends_correctly()` — `extend_from()` appends another writer's bytes correctly and `len()` reflects the sum

In `oriterm/src/gpu/scene_convert/tests.rs`:
- `fn convert_rect_uniform_border()` — uniform border produces equal per-side widths and colors
- `fn convert_rect_per_side_widths()` — asymmetric widths propagate through conversion
- `fn convert_rect_mixed_per_side_colors()` — different per-side colors propagate through conversion
- `fn convert_rect_four_corner_radii()` — four distinct corner radii propagate correctly (NOT collapsed by `uniform_radius()`)

- `fn convert_rect_no_border()` — empty `BorderSides` produces zero border widths
- `fn convert_rect_shadow_uses_per_corner_radii()` — a rect with shadow and per-corner radii
  produces a shadow instance where each corner radius is expanded by `spread + blur_radius`,
  NOT collapsed to a single `uniform_radius() + expand`

- `fn convert_rect_border_widths_scaled_by_scale_factor()` — border widths are multiplied by the
  scale factor (e.g. scale=2.0 doubles all border widths in the output instance)

- `fn convert_line_writes_zero_border()` — line conversion produces a UI rect instance with zero border widths and transparent border colors

- `fn convert_line_diagonal_still_produces_stepping_rects()` — diagonal line decomposition still
  emits per-step UI rect instances through the new writer (regression test for
  `convert_line_clipped()` pixel-stepping logic)

In `oriterm/src/gpu/pipeline/tests.rs`:
- **Remove** `ui_rect_attrs_share_first_seven_with_instance_attrs` — no longer valid since the
  new layout is fully independent from `INSTANCE_ATTRS`

- Update `ui_rect_ten_attributes` to verify the new 10-attribute array (count stays 10)
- Rewrite `ui_rect_attribute_offsets_and_locations` with the new 144-byte layout offsets:
  `[(0,0), (8,1), (16,2), (32,3), (48,4), (64,5), (80,6), (96,7), (112,8), (128,9)]`
- Rewrite `ui_rect_last_attribute_fits_within_stride` to check against 144 instead of 96
- Rewrite `ui_rect_buffer_layout_uses_instance_step_mode` to verify `array_stride == 144`
- Update `ui_rect_attributes_are_contiguous` — still valid pattern but offsets change
- `fn ui_rect_stride_is_144()` — verify new `UI_RECT_INSTANCE_SIZE` constant equals 144

### Checklist

- [x] `./build-all.sh` passes
- [x] `./clippy-all.sh` passes
- [x] `./test-all.sh` passes
- [x] `border` module has dedicated `BorderSides` tests (see test list above)
- [x] `RectStyle` tests cover uniform and side-specific builders
- [x] dedicated UI-rect writer tests cover byte layout, field offsets, and count semantics
- [x] pipeline tests cover the new UI-rect stride and attribute offsets
- [x] scene-convert tests cover uniform, per-side width, per-side color, corner radii, and line cases
- [x] damage tracker tests verify per-side border changes are detected (not silently ignored)

- [x] visual verification exists for at least one rounded mixed-color border case before the
  section is marked complete
- [x] `/tpr-review` passed — independent Codex review found no critical or major issues (or all findings triaged)
