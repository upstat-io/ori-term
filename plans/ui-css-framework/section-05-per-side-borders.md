---
section: "05"
title: "Per-Side Borders"
status: not-started
reviewed: true
third_party_review:
  status: none
  updated: null
goal: "RectStyle supports per-side border widths and colors — sidebar right border, nav active left border, and footer top border render correctly"
inspired_by:
  - "CSS box model border-top/right/bottom/left"
depends_on: []
sections:
  - id: "05.1"
    title: "BorderSides Type"
    status: not-started
  - id: "05.2"
    title: "RectStyle Integration"
    status: not-started
  - id: "05.3"
    title: "GPU Rect Shader Update"
    status: not-started
  - id: "05.4"
    title: "Widget Adoption"
    status: not-started
  - id: "05.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "05.5"
    title: "Build & Verify"
    status: not-started
---

# Section 05: Per-Side Borders

**Goal:** Enable per-side border widths and colors on `RectStyle`, replacing the current uniform-only `Option<Border>`. The mockup uses per-side borders in at least three places: sidebar right border (vertical separator), nav active item left border (3px accent indicator), and footer top border (horizontal separator). The current `Border` struct only supports a single uniform width+color, which cannot express these patterns without hacks (e.g. drawing a separate `push_line()` adjacent to the rect).

**References:**
- `oriterm_ui/src/draw/border.rs` — current `Border { width: f32, color: Color }`
- `oriterm_ui/src/draw/rect_style.rs` — `RectStyle { border: Option<Border>, ... }`
- `oriterm/src/gpu/shaders/ui_rect.wgsl` — SDF fragment shader with uniform `border_width: f32`
- `oriterm/src/gpu/instance_writer/mod.rs` — instance record layout, `push_ui_rect()` method
- `oriterm/src/gpu/pipelines/` — UI rect pipeline vertex attribute layout
- `oriterm/src/gpu/scene_convert/mod.rs` — `convert_rect_clipped()` extracts `b.width` from `Option<Border>`

---

## 05.1 BorderSides Type

**File:** `oriterm_ui/src/draw/border.rs`

Add a `BorderSides` type that holds per-side border specs. Keep the existing `Border` struct as the per-side value type (width + color pair).

### New Types

```rust
/// Per-side border specification.
///
/// Each side is optional. `None` means no border on that side.
/// Use `BorderSides::uniform()` for a single width+color on all sides.
/// Use `BorderSides::only_left()` etc. for single-side borders.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BorderSides {
    pub top: Option<Border>,
    pub right: Option<Border>,
    pub bottom: Option<Border>,
    pub left: Option<Border>,
}
```

### Constructors and Helpers

```rust
impl BorderSides {
    /// All four sides with the same width and color.
    pub fn uniform(width: f32, color: Color) -> Self { ... }

    /// Only the top side.
    pub fn only_top(width: f32, color: Color) -> Self { ... }

    /// Only the right side.
    pub fn only_right(width: f32, color: Color) -> Self { ... }

    /// Only the bottom side.
    pub fn only_bottom(width: f32, color: Color) -> Self { ... }

    /// Only the left side.
    pub fn only_left(width: f32, color: Color) -> Self { ... }

    /// Whether all four sides are `None`.
    pub fn is_empty(&self) -> bool { ... }

    /// Whether all set sides share the same width and color.
    ///
    /// Returns the shared `Border` if uniform, `None` if sides differ.
    /// Used by the GPU path to select the uniform fast path (single SDF
    /// inset) vs. the per-side path (4 edge distance checks).
    pub fn as_uniform(&self) -> Option<Border> { ... }

    /// Per-side widths as `[top, right, bottom, left]`.
    ///
    /// `None` sides are `0.0`. Used by the GPU instance writer.
    pub fn widths(&self) -> [f32; 4] { ... }

    /// Returns the color for the border. When sides have different colors,
    /// returns the first non-None side's color. Returns `None` if all sides
    /// are `None`.
    ///
    /// The GPU shader currently supports a single border color (same as
    /// existing behavior). Per-side colors would require either multiple
    /// draw calls or a more complex shader. For now, we enforce uniform
    /// color across all set sides.
    pub fn color(&self) -> Option<Color> { ... }
}
```

### Design Decision: Single Border Color

The mockup uses the same color for all per-side borders (the theme border color or the accent color). The GPU shader uses `fg_color` for border color and there is only one `fg_color` per instance. Rather than expanding the instance record to carry 4 border colors (adding 48 bytes per instance), we enforce a single color and vary only the widths per side. This matches CSS practice where per-side colors are rare in terminal UIs.

If per-side colors become needed later, the correct approach would be to emit multiple rect instances (one per border side), not to expand the instance record.

### Validation

`BorderSides` validates at construction that if multiple sides are set, they all share the same color. The `only_*` constructors trivially satisfy this. `uniform()` satisfies this. A general `new(top, right, bottom, left)` constructor should assert color uniformity in debug builds.

---

## 05.2 RectStyle Integration

**File:** `oriterm_ui/src/draw/rect_style.rs`

### Field Change

Replace `border: Option<Border>` with `border: BorderSides`:

```rust
pub struct RectStyle {
    pub fill: Option<Color>,
    /// Per-side border specification.
    pub border: BorderSides,
    pub corner_radius: [f32; 4],
    pub shadow: Option<Shadow>,
    pub gradient: Option<Gradient>,
}
```

`BorderSides::default()` is all-`None` (no border), which is equivalent to `Option<Border>::None`, so the `Default` derive on `RectStyle` continues to work.

### Builder Methods

Keep the existing `with_border(width, color)` as a uniform shorthand:

```rust
impl RectStyle {
    /// Adds a uniform border on all sides.
    pub fn with_border(mut self, width: f32, color: Color) -> Self {
        self.border = BorderSides::uniform(width, color);
        self
    }

    /// Adds a border on the top side only.
    pub fn with_border_top(mut self, width: f32, color: Color) -> Self {
        self.border.top = Some(Border { width, color });
        self
    }

    /// Adds a border on the right side only.
    pub fn with_border_right(mut self, width: f32, color: Color) -> Self {
        self.border.right = Some(Border { width, color });
        self
    }

    /// Adds a border on the bottom side only.
    pub fn with_border_bottom(mut self, width: f32, color: Color) -> Self {
        self.border.bottom = Some(Border { width, color });
        self
    }

    /// Adds a border on the left side only.
    pub fn with_border_left(mut self, width: f32, color: Color) -> Self {
        self.border.left = Some(Border { width, color });
        self
    }
}
```

### Migration of Existing Call Sites

All existing `style.border.map_or(...)` and `style.border.unwrap_or(...)` patterns must update. Search for all references to `border` field on `RectStyle`:

- `oriterm/src/gpu/scene_convert/mod.rs` — `convert_rect_clipped()`: extract border width/color from `BorderSides`
- `oriterm_ui/src/draw/scene/mod.rs` — if any scene code reads border
- All widget files that construct `RectStyle::with_border(...)` — these continue working unchanged since `with_border` remains the same signature

The critical change is in `convert_rect_clipped()`:

```rust
// Before:
let (border_color, border_width) = style.border.map_or(([0.0; 4], 0.0), |b| {
    (color_to_linear_with_opacity(b.color, opacity), b.width)
});

// After — uniform fast path:
let (border_color, border_width) = if let Some(b) = style.border.as_uniform() {
    (color_to_linear_with_opacity(b.color, opacity), b.width)
} else if style.border.is_empty() {
    ([0.0; 4], 0.0)
} else {
    // Per-side: handled by the per-side shader path (05.3)
    ...
};
```

---

## 05.3 GPU Rect Shader Update

**WARNING: High-risk subsection.** GPU shader changes require careful validation. Test incrementally: first verify the uniform path still works, then add per-side rendering one border at a time. Use wgpu's validation layer to catch layout mismatches.

### Instance Record Layout Change

The current 96-byte instance record has a single `f32` at offset 76 for `border_width`. For per-side borders, we need 4 widths. Two options:

**Option A: Repurpose existing fields.** The UI rect pipeline does not use the `uv` field (all zeros for rects — UV is only used by glyph instances). We can repurpose the 16-byte `uv` slot (offset 16, `vec4<f32>`) to carry `border_widths: [top, right, bottom, left]`. The `border_width` field at offset 76 becomes a flag: `0.0` = no border, `> 0.0` = check `uv` for per-side widths. This avoids changing the instance record size.

**Option B: Expand instance size.** Change offset 76 from `f32` to `vec4<f32>` (16 bytes), shifting clip rect from offset 80 to 92. This would increase record size from 96 to 108 bytes and requires updating every pipeline layout.

**Recommendation: Option A.** The UI rect shader already has its own pipeline and vertex layout (`UI_RECT_ATTRS`). The `uv` field is always `[0, 0, 0, 0]` for UI rects (no texture sampling). Repurposing it for border widths is zero-cost and avoids any instance size change.

### Instance Writer Changes

**File:** `oriterm/src/gpu/instance_writer/mod.rs`

Add a new method or modify `push_ui_rect()`:

```rust
/// Push a styled UI rectangle with per-side border widths.
///
/// `border_widths` is `[top, right, bottom, left]` in logical pixels.
/// All four values are written to the UV field (offsets 16-31), which
/// the UI rect shader repurposes for per-side border widths.
pub fn push_ui_rect_per_side(
    &mut self,
    rect: ScreenRect,
    fill: [f32; 4],
    border_color: [f32; 4],
    corner_radius: f32,
    border_widths: [f32; 4],  // [top, right, bottom, left]
    clip: [f32; 4],
) { ... }
```

The existing `push_ui_rect()` with a single `border_width: f32` can call through to the per-side version with `[w, w, w, w]`.

### Scene Convert Changes

**File:** `oriterm/src/gpu/scene_convert/mod.rs`

In `convert_rect_clipped()`, detect uniform vs. per-side and call the appropriate writer method:

```rust
if let Some(b) = style.border.as_uniform() {
    // Uniform: single SDF inset (existing fast path).
    writer.push_ui_rect(screen, fill, border_color, radius, b.width * scale, clip);
} else if !style.border.is_empty() {
    // Per-side: write widths into UV slots.
    let widths = style.border.widths();
    let scaled = [
        widths[0] * scale,
        widths[1] * scale,
        widths[2] * scale,
        widths[3] * scale,
    ];
    let color = style.border.color()
        .map_or([0.0; 4], |c| color_to_linear_with_opacity(c, opacity));
    writer.push_ui_rect_per_side(screen, fill, color, radius, scaled, clip);
} else {
    writer.push_ui_rect(screen, fill, [0.0; 4], radius, 0.0, clip);
}
```

### WGSL Shader Changes

**File:** `oriterm/src/gpu/shaders/ui_rect.wgsl`

The fragment shader currently uses a single `border_width` and computes an inner SDF inset uniformly. For per-side borders, we switch to edge-distance checks.

The current `VertexOutput` uses locations 0-7 (clip_max at @location(7)). We add a new interpolant:

```wgsl
// In VertexOutput, add:
@location(8) border_widths: vec4<f32>,  // [top, right, bottom, left]

// In vs_main, pass through the UV field (repurposed for UI rects):
out.border_widths = instance.uv;
```

In `fs_main`, detect per-side mode: when `border_width > 0.0` AND any component of `border_widths` differs from the others, use per-side path. Otherwise use existing uniform SDF inset.

```wgsl
if input.border_width > 0.0 {
    let bw = input.border_widths;
    let is_uniform = all(bw == vec4<f32>(bw.x));

    if is_uniform {
        // Existing uniform SDF inset path (unchanged).
        let inner_radius = max(input.corner_radius - input.border_width, 0.0);
        let inner_half = input.half_size - input.border_width;
        let d_inner = sd_rounded_box(input.local_pos, inner_half, inner_radius);
        let inner_alpha = 1.0 - smoothstep(-0.5, 0.5, d_inner);
        color = mix(input.border_color, input.fill_color, inner_alpha);
    } else {
        // Per-side edge distance checks.
        // local_pos is offset from center; half_size is half-extents.
        // dist_from_top_edge = (local_pos.y + half_size.y) = pixels from top edge
        let dist_top    = bw.x - (input.local_pos.y + input.half_size.y);
        let dist_bottom = bw.z - (input.half_size.y - input.local_pos.y);
        let dist_left   = bw.w - (input.local_pos.x + input.half_size.x);
        let dist_right  = bw.y - (input.half_size.x - input.local_pos.x);

        let in_top    = smoothstep(-0.5, 0.5, dist_top);
        let in_bottom = smoothstep(-0.5, 0.5, dist_bottom);
        let in_left   = smoothstep(-0.5, 0.5, dist_left);
        let in_right  = smoothstep(-0.5, 0.5, dist_right);

        let in_border = max(max(in_top, in_bottom), max(in_left, in_right));
        color = mix(input.fill_color, input.border_color, in_border);
    }
}
```

**Corner interaction:** When both per-side borders and corner radius are active, the outer rounded-box SDF still clips everything. The per-side edge checks define the border zone; in corner regions the outer SDF anti-aliasing handles the rounding naturally.

**Performance note:** The per-side check adds 4 `smoothstep` calls and `max` operations. For UI rects (dozens per frame, not thousands), this is negligible. The uniform-detection fast path (`all(bw == vec4<f32>(bw.x))`) avoids the per-side math for the common case.

### Pipeline Layout

**File:** `oriterm/src/gpu/pipeline/mod.rs`

No changes needed. The `uv` field at location 2 is already declared as `vec4<f32>` in `UI_RECT_ATTRS`. The shader reads `instance.uv` which is already wired up. The vertex output just needs the new `border_widths` interpolant.

---

## 05.4 Widget Adoption

### Sidebar Right Border

The sidebar container should have a right border separating it from the content area.

```rust
// In sidebar container rendering:
let sidebar_style = RectStyle::filled(theme.sidebar_bg)
    .with_border_right(1.0, theme.border);
```

### Footer Top Border

The footer (unsaved changes bar) should have a top border separating it from the scroll content.

```rust
// In footer rendering:
let footer_style = RectStyle::filled(theme.surface)
    .with_border_top(1.0, theme.border);
```

### Nav Active Item Left Border

The active navigation item in the sidebar should have a left accent border as a selection indicator.

```rust
// In SidebarNavWidget active item rendering:
if is_active {
    let active_style = RectStyle::filled(theme.accent_bg)
        .with_border_left(3.0, theme.accent);
    ctx.scene.push_quad(item_bounds, active_style);
}
```

### Existing Call Sites

All existing `with_border()` calls continue to work unchanged. The method signature is identical. The internal representation changes from `Option<Border>` to `BorderSides::uniform()`, but the builder pattern hides this.

---

## 05.R Third Party Review Findings

*To be populated after review.*

---

## 05.5 Build & Verify

### Checklist

- [ ] `./build-all.sh` passes (all 3 targets)
- [ ] `./clippy-all.sh` passes (no new warnings)
- [ ] `./test-all.sh` passes (existing tests unbroken)
- [ ] Convert `oriterm_ui/src/draw/border.rs` to directory module `border/mod.rs` + `border/tests.rs` (per test-organization.md)
- [ ] New unit tests in `oriterm_ui/src/draw/border/tests.rs`:
  - `BorderSides::uniform()` round-trips through `as_uniform()`
  - `BorderSides::only_left()` has correct `widths()` array
  - `BorderSides::default().is_empty()` is true
  - `as_uniform()` returns `None` for mixed widths
- [ ] New unit tests in `oriterm/src/gpu/scene_convert/tests.rs`:
  - Per-side border rect writes correct UV values to instance buffer
  - Uniform border still uses the uniform fast path
- [ ] Visual verification: sidebar right border, footer top border, and nav active left border render at correct positions with correct colors
- [ ] No regression in uniform border rendering (buttons, dialogs, text inputs)
