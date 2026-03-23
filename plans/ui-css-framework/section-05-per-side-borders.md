---
section: "05"
title: "Per-Side Borders"
status: not-started
reviewed: false
third_party_review:
  status: resolved
  updated: 2026-03-23
goal: "RectStyle and the dedicated UI-rect renderer support per-side border widths and colors, including rounded-corner boxes, while preserving a fast path for the common uniform-border case"
inspired_by:
  - "CSS box model border-top/right/bottom/left"
  - "CSS border painting and corner joins"
depends_on: []
sections:
  - id: "05.1"
    title: "BorderSides Type"
    status: not-started
  - id: "05.2"
    title: "RectStyle Integration"
    status: not-started
  - id: "05.3"
    title: "Dedicated UI Rect Instance Path"
    status: not-started
  - id: "05.4"
    title: "Shader Geometry and Corner Ownership"
    status: not-started
  - id: "05.5"
    title: "Consumer Boundaries"
    status: not-started
  - id: "05.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "05.6"
    title: "Build & Verify"
    status: not-started
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
  converts every styled rect to one UI-rect instance with one `border_width`.
- [oriterm/src/gpu/shaders/ui_rect.wgsl](/home/eric/projects/ori_term/oriterm/src/gpu/shaders/ui_rect.wgsl)
  only understands one border width and one border color.
- [oriterm/src/gpu/prepared_frame/mod.rs](/home/eric/projects/ori_term/oriterm/src/gpu/prepared_frame/mod.rs)
  already keeps `ui_rects` and `overlay_rects` in dedicated buffers, separate from glyph and
  terminal-background buffers.
- [oriterm/src/gpu/pipeline/mod.rs](/home/eric/projects/ori_term/oriterm/src/gpu/pipeline/mod.rs)
  already has a dedicated UI-rect pipeline and buffer layout.

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

    pub fn is_empty(&self) -> bool { ... }

    /// Returns `Some(border)` only when all four sides are present and identical.
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

- [ ] Convert `border.rs` into `border/mod.rs` + `border/tests.rs`
- [ ] Add `BorderSides`
- [ ] Re-export `BorderSides` from `draw/mod.rs`
- [ ] Add uniform and single-side constructors
- [ ] Add `is_empty()`, `as_uniform()`, `widths()`, and `colors()`
- [ ] Normalize invalid widths to "no border"

---

## 05.2 RectStyle Integration

### Goal

Thread `BorderSides` through the public style API while keeping uniform-border call sites simple.

### Files

- [oriterm_ui/src/draw/rect_style.rs](/home/eric/projects/ori_term/oriterm_ui/src/draw/rect_style.rs)
- [oriterm_ui/src/draw/tests.rs](/home/eric/projects/ori_term/oriterm_ui/src/draw/tests.rs)

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

### Checklist

- [ ] Change `RectStyle.border` to `BorderSides`
- [ ] Keep `with_border(width, color)` as the uniform shorthand
- [ ] Add top/right/bottom/left border builders
- [ ] Update draw tests to expect `BorderSides`

---

## 05.3 Dedicated UI Rect Instance Path

### Goal

Move full per-side border support onto the dedicated UI-rect renderer instead of overloading the
shared 96-byte instance format used by unrelated pipelines.

### Files

- [oriterm/src/gpu/prepared_frame/mod.rs](/home/eric/projects/ori_term/oriterm/src/gpu/prepared_frame/mod.rs)
- [oriterm/src/gpu/window_renderer/mod.rs](/home/eric/projects/ori_term/oriterm/src/gpu/window_renderer/mod.rs)
- [oriterm/src/gpu/window_renderer/render.rs](/home/eric/projects/ori_term/oriterm/src/gpu/window_renderer/render.rs)
- [oriterm/src/gpu/window_renderer/helpers.rs](/home/eric/projects/ori_term/oriterm/src/gpu/window_renderer/helpers.rs)
- [oriterm/src/gpu/pipeline/mod.rs](/home/eric/projects/ori_term/oriterm/src/gpu/pipeline/mod.rs)
- [oriterm/src/gpu/pipeline/tests.rs](/home/eric/projects/ori_term/oriterm/src/gpu/pipeline/tests.rs)

The files above are already large:

- [pipeline/mod.rs](/home/eric/projects/ori_term/oriterm/src/gpu/pipeline/mod.rs) is already
  over the 500-line limit
- [window_renderer/mod.rs](/home/eric/projects/ori_term/oriterm/src/gpu/window_renderer/mod.rs)
  is already over the 500-line limit
- [window_renderer/render.rs](/home/eric/projects/ori_term/oriterm/src/gpu/window_renderer/render.rs)
  is already over the 500-line limit

So this section should extract the UI-rect-specific buffer layout and upload helpers into
submodules rather than growing those files further.

### New Dedicated Writer

Introduce a dedicated writer for UI rects, for example:

- `oriterm/src/gpu/ui_rect_writer/mod.rs` (with `#[cfg(test)] mod tests;` at bottom)
- `oriterm/src/gpu/ui_rect_writer/tests.rs`

`PreparedFrame.ui_rects` and `PreparedFrame.overlay_rects` should switch from `InstanceWriter` to
this dedicated writer. The shared background/glyph/cursor writers stay unchanged.

### Proposed UI Rect Instance Layout

Use a UI-rect-specific stride, e.g. 144 bytes:

```text
0   pos: vec2<f32>
8   size: vec2<f32>
16  clip: vec4<f32>
32  fill_color: vec4<f32>
48  border_widths: vec4<f32>   // [top, right, bottom, left]
64  corner_radii: vec4<f32>    // [top_left, top_right, bottom_right, bottom_left]
80  border_top: vec4<f32>
96  border_right: vec4<f32>
112 border_bottom: vec4<f32>
128 border_left: vec4<f32>
```

That gives the shader every value it needs directly, without stealing fields from glyph instances
or creating hidden dual meanings for `uv`, `kind`, or `atlas_page`.

### Why This Boundary Is Correct

- UI rects already render through a dedicated `ui_rect_pipeline` and dedicated `ui_rect_buffer`
  (verified in `render.rs` draw dispatch via `record_draw`).
- The shared glyph/background writer does not need to know anything about border sides.
- Upload helpers already operate on raw byte slices, so a new writer type is a localized change.
- The renderer can keep a uniform fast path while still supporting full feature data.

### Render Path Impact

Switching from `InstanceWriter` (96-byte stride) to a dedicated writer (144-byte stride) requires
updating more than `PreparedFrame` field types:

- **`render.rs`**: The `upload!` macro and `record_draw()` calls for `ui_rect_buffer` and
  `overlay_rects` must use the new writer's byte slice and instance count.
- **`pipeline/mod.rs`**: The `ui_rect_buffer_layout()` must match the new stride and attribute
  offsets. The current layout assumes the shared 96-byte format.
- **`PreparedFrame` API surface**: `extend_from()`, `clear()`, `maybe_shrink()`, `is_empty()`,
  `len()`, and `clear_ephemeral_tiers()` all operate on the `ui_rects` and `overlay_rects` fields.
  The new writer must expose a compatible interface or those methods must be updated.
- **`OverlayDrawRange`**: The `rects` range is currently based on `InstanceWriter::len()`. The
  new writer must provide the same `len()` semantics (instance count, not byte count).

### Checklist

- [ ] Add a dedicated UI-rect writer module with its own tests
- [ ] Switch `PreparedFrame.ui_rects` and `overlay_rects` to the new writer
- [ ] Extract UI-rect buffer layout code out of oversized pipeline/window-renderer files
- [ ] Add a dedicated `ui_rect_buffer_layout()` for the new stride
- [ ] Update `render.rs` draw dispatch for the new writer's byte layout and instance count
- [ ] Update `OverlayDrawRange` and `PreparedFrame` helper methods for the new writer API
- [ ] Keep the shared `InstanceWriter` and non-UI pipelines unchanged

---

## 05.4 Shader Geometry and Corner Ownership

> **HIGH COMPLEXITY WARNING**: This subsection is the most technically demanding work in the entire plan. It requires coordinated changes to WGSL shader code, Rust GPU pipeline configuration, scene conversion, and a custom SDF implementation. Budget extra time and expect iteration. The corner ownership math should be prototyped and unit-tested on the CPU side before porting to WGSL. Subsections 05.3 and 05.4 must be implemented together in a single pass: the new buffer layout from 05.3 and the shader updates from 05.4 are tightly coupled and neither will compile or run correctly without the other.

### Goal

Render the full feature set correctly:

- per-side widths
- per-side colors
- rounded corners
- deterministic joins where adjacent border sides meet

### Files

- [oriterm/src/gpu/scene_convert/mod.rs](/home/eric/projects/ori_term/oriterm/src/gpu/scene_convert/mod.rs)
- [oriterm/src/gpu/shaders/ui_rect.wgsl](/home/eric/projects/ori_term/oriterm/src/gpu/shaders/ui_rect.wgsl)
- [oriterm/src/gpu/scene_convert/tests.rs](/home/eric/projects/ori_term/oriterm/src/gpu/scene_convert/tests.rs)

### Scene Conversion

`convert_scene()` currently takes `&mut InstanceWriter` as its second parameter (the UI rect
writer). Once `PreparedFrame.ui_rects` changes to the dedicated writer type, `convert_scene()`
and `convert_rect_clipped()` must also change their parameter types to match. Both functions live
in `scene_convert/mod.rs`.

`convert_rect_clipped()` should populate the dedicated UI-rect instance directly:

- fill color
- per-side widths
- per-side colors
- all four corner radii
- clip rect

Shadow conversion can stay as an extra fill-only UI-rect instance with zero border widths.

### Shader Responsibilities

The shader needs three pieces of logic:

1. **Outer shape mask**
   Use a true four-corner rounded-rect SDF instead of collapsing radii with the current
   `uniform_radius()` behavior. Note: `uniform_radius()` returns `max()` of all four radii.
   The four-corner data is already stored on `RectStyle.corner_radius: [f32; 4]` and passed
   through `convert_rect_clipped()`, but is collapsed to one scalar by `uniform_radius()`
   before reaching the shader.

2. **Inner content mask**
   Compute the inset shape from per-side border widths. Because left/right and top/bottom widths can
   differ, the inner content box is offset relative to the outer center. The inner corner geometry
   must be derived from the adjacent side insets, not from a single scalar border width.

3. **Border-side ownership**
   Choose top/right/bottom/left border color per fragment, including the corner regions where two
   adjacent sides meet.

### Corner Ownership Rule

Do not fall back to "first non-transparent side wins" or "left side owns the full corner." That is
the kind of shortcut that makes mixed-color rounded borders look wrong immediately.

Instead, define an explicit corner ownership rule and use it consistently:

- each edge band owns its straight edge region
- each rounded corner is split between its two adjacent sides
- the split follows the corner wedge from the outer arc toward the inner arc, matching CSS-style
  adjacent-side ownership closely enough that browser and mockup comparisons agree

The exact math can be implemented either as:

- a direct per-fragment corner-wedge test in WGSL, or
- a small Rust helper mirrored in WGSL so the same ownership cases are testable on the CPU side

Either is acceptable. The non-negotiable part is that the rule must be explicit and testable.

### Uniform Fast Path

Preserve a fast path inside the UI-rect path when:

- all four border widths are equal
- all four border colors are equal
- all four corner radii are equal

That lets the common button/input/dialog case stay on the simple branch even after the dedicated
UI-rect format grows.

### Checklist

- [ ] Replace the current single-radius/single-border shader logic with full per-side data flow
- [ ] Implement four-corner rounded-rect outer masking
- [ ] Implement asymmetric inset inner masking
- [ ] Implement explicit side ownership for corner regions
- [ ] Preserve a fast uniform branch for common cases

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

- [ ] Do not claim existing sidebar/footer dividers are blocked on this section
- [ ] Limit this section to shared style and renderer capability
- [ ] Let later fidelity sections choose concrete widget migrations

---

## 05.R Third Party Review Findings

- [x] `[TPR-05-001][high]` [plans/ui-css-framework/section-05-per-side-borders.md](/home/eric/projects/ori_term/plans/ui-css-framework/section-05-per-side-borders.md) - The original draft said the goal was per-side widths and colors, then later restricted the implementation to one shared border color. That is incompatible with the mockup's callouts, which combine a subtle perimeter with a stronger left accent on the same box. Resolved: the section now treats per-side colors as first-class data on 2026-03-23.

- [x] `[TPR-05-002][medium]` [plans/ui-css-framework/section-05-per-side-borders.md](/home/eric/projects/ori_term/plans/ui-css-framework/section-05-per-side-borders.md) - The previous revision scoped rounded and mixed-color cases out of the section and replaced them with a square-corner CPU decomposition plan. That makes the plan easier, but it removes framework capability the user explicitly asked to support properly. Resolved: the section now keeps those cases in scope and routes them through the dedicated UI-rect renderer on 2026-03-23.

- [x] `[TPR-05-003][medium]` [plans/ui-css-framework/section-05-per-side-borders.md](/home/eric/projects/ori_term/plans/ui-css-framework/section-05-per-side-borders.md) - Earlier drafts treated the shared 96-byte `InstanceWriter` as the only available implementation path. That was the wrong boundary: UI rects already have dedicated buffers and a dedicated pipeline. Resolved: the section now introduces a dedicated UI-rect writer and buffer layout instead of overloading the glyph/background path on 2026-03-23.

- [x] `[TPR-05-004][medium]` [plans/ui-css-framework/section-05-per-side-borders.md](/home/eric/projects/ori_term/plans/ui-css-framework/section-05-per-side-borders.md) - The renderer still collapses four corner radii with `uniform_radius()`, so any plan that claims proper rounded asymmetric borders without addressing the shader geometry is incomplete. Resolved: the section now explicitly includes four-corner outer masking, asymmetric inner masking, and corner ownership in the UI-rect shader work on 2026-03-23.

- [x] `[TPR-05-005][medium]` [plans/ui-css-framework/section-05-per-side-borders.md](/home/eric/projects/ori_term/plans/ui-css-framework/section-05-per-side-borders.md) - [pipeline/mod.rs](/home/eric/projects/ori_term/oriterm/src/gpu/pipeline/mod.rs), [window_renderer/mod.rs](/home/eric/projects/ori_term/oriterm/src/gpu/window_renderer/mod.rs), and [window_renderer/render.rs](/home/eric/projects/ori_term/oriterm/src/gpu/window_renderer/render.rs) are already over the repository's 500-line source-file limit. A realistic plan must include extraction work instead of silently growing them further. Resolved: the section now requires UI-rect-specific submodules as part of the implementation on 2026-03-23.

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
4. Manual or screenshot-based verification against the mockup for:
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

### Tests

In `oriterm_ui/src/draw/border/tests.rs`:
- `fn border_sides_default_is_empty()` — `BorderSides::default().is_empty()` is true
- `fn border_sides_uniform_all_sides_equal()` — `uniform()` sets all four sides identically
- `fn border_sides_as_uniform_returns_some_when_identical()` — `as_uniform()` returns `Some` for uniform borders
- `fn border_sides_as_uniform_returns_none_when_different()` — `as_uniform()` returns `None` when sides differ
- `fn border_sides_only_top_leaves_others_none()` — `only_top()` sets top, leaves others `None`
- `fn border_sides_widths_returns_correct_array()` — `widths()` returns `[top, right, bottom, left]` with 0.0 for absent sides
- `fn border_sides_colors_uses_transparent_for_absent()` — `colors()` uses transparent for absent sides
- `fn border_sides_normalizes_invalid_width()` — NaN, negative, and zero widths normalize to "no border"
- `fn border_sides_composition()` — `uniform().with_border_left()` overrides only left side

In `oriterm_ui/src/draw/tests.rs`:
- `fn rect_style_with_border_creates_uniform()` — `with_border()` creates `BorderSides::uniform()`
- `fn rect_style_with_border_top_only_sets_top()` — per-side builder sets only the addressed side
- `fn rect_style_composable_border_sides()` — `with_border().with_border_left()` composes correctly

In `oriterm/src/gpu/ui_rect_writer/tests.rs`:
- `fn ui_rect_instance_stride_is_144_bytes()` — byte layout matches the declared stride
- `fn ui_rect_writer_count_matches_instances()` — `len()` returns instance count, not byte count
- `fn ui_rect_writer_clear_resets_to_zero()` — `clear()` resets count but retains capacity

In `oriterm/src/gpu/scene_convert/tests.rs`:
- `fn convert_rect_uniform_border()` — uniform border produces equal per-side widths and colors
- `fn convert_rect_per_side_widths()` — asymmetric widths propagate through conversion
- `fn convert_rect_mixed_per_side_colors()` — different per-side colors propagate through conversion
- `fn convert_rect_four_corner_radii()` — four distinct corner radii propagate correctly
- `fn convert_rect_no_border()` — empty `BorderSides` produces zero border widths

### Checklist

- [ ] `./build-all.sh` passes
- [ ] `./clippy-all.sh` passes
- [ ] `./test-all.sh` passes
- [ ] `border` module has dedicated `BorderSides` tests (see test list above)
- [ ] `RectStyle` tests cover uniform and side-specific builders
- [ ] dedicated UI-rect writer tests cover byte layout and count semantics
- [ ] pipeline tests cover the new UI-rect stride and attribute offsets
- [ ] scene-convert tests cover uniform, per-side width, per-side color, and corner radii cases
- [ ] visual verification exists for at least one rounded mixed-color border case before the
  section is marked complete
