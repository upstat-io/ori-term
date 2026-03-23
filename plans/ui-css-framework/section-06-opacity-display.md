---
section: "06"
title: "Opacity + Display Control"
status: not-started
reviewed: true
third_party_review:
  status: none
  updated: null
goal: "Widgets can be faded (opacity modulation) and hidden (display:none equivalent) — disabled controls render at 40% opacity, inactive icons at 70%, and page switching hides non-active pages with zero layout cost"
inspired_by:
  - "CSS opacity property"
  - "CSS display: none"
  - "CSS pointer-events: none"
depends_on: []
sections:
  - id: "06.1"
    title: "Opacity Modulation"
    status: not-started
  - id: "06.2"
    title: "Display/Visibility Toggle"
    status: not-started
  - id: "06.3"
    title: "Pointer Events Control"
    status: not-started
  - id: "06.4"
    title: "Tests"
    status: not-started
  - id: "06.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "06.5"
    title: "Build & Verify"
    status: not-started
---

# Section 06: Opacity + Display Control

**Goal:** Add two CSS-equivalent controls to the UI framework: opacity modulation (fade widgets without removing them from the tree) and display toggling (remove from layout entirely). These are foundational building blocks used by multiple mockup patterns: disabled controls at 40% opacity, inactive sidebar icons at 70%, and settings page switching where non-active pages occupy zero space.

**References:**
- `oriterm_ui/src/draw/scene/stacks.rs` — Scene state stacks (clip, offset, layer bg)
- `oriterm_ui/src/draw/scene/paint.rs` — `push_quad()`, `push_text()`, `push_line()`, `push_icon()` methods
- `oriterm_ui/src/widgets/contexts.rs` — `DrawCtx` struct (bounds, scene, measurer, theme, etc.)
- `oriterm_ui/src/layout/layout_box.rs` — `LayoutBox` struct (disabled field exists, no visible/hidden field)
- `oriterm/src/gpu/scene_convert/mod.rs` — `convert_scene()` already accepts `opacity: f32` parameter
- `oriterm_ui/src/widgets/mod.rs` — `Widget` trait definition

---

## 06.1 Opacity Modulation

### Current State

The GPU pipeline already supports opacity modulation. `convert_scene()` in `oriterm/src/gpu/scene_convert/mod.rs` accepts an `opacity: f32` parameter and multiplies all emitted primitive colors by it via `color_to_linear_with_opacity()`. This is the compositor-level opacity used for window-level fading.

What is missing is widget-level opacity: the ability for a single widget subtree to render at reduced opacity. The Scene does not have an opacity stack.

### Approach: Scene Opacity Stack

Add an opacity stack to `Scene`, analogous to the existing clip and offset stacks.

**File:** `oriterm_ui/src/draw/scene/stacks.rs`

Add `opacity_stack: Vec<f32>` and `cumulative_opacity: f32` fields to `Scene`:

```rust
// In Scene struct:
opacity_stack: Vec<f32>,
cumulative_opacity: f32,  // product of all stacked opacities
```

```rust
impl Scene {
    /// Push an opacity multiplier onto the stack.
    ///
    /// All primitives pushed while this opacity is active will have their
    /// alpha multiplied by `opacity`. Opacities compose multiplicatively:
    /// pushing 0.5 then 0.5 yields 0.25 effective opacity.
    pub fn push_opacity(&mut self, opacity: f32) {
        self.opacity_stack.push(self.cumulative_opacity);
        self.cumulative_opacity *= opacity.clamp(0.0, 1.0);
    }

    /// Pop the most recent opacity from the stack.
    pub fn pop_opacity(&mut self) {
        if let Some(prev) = self.opacity_stack.pop() {
            self.cumulative_opacity = prev;
        }
    }

    /// Returns the current cumulative opacity.
    pub fn current_opacity(&self) -> f32 {
        self.cumulative_opacity
    }

    /// Whether the opacity stack is balanced (empty).
    pub fn opacity_stack_is_empty(&self) -> bool {
        self.opacity_stack.is_empty()
    }
}
```

Initialize `cumulative_opacity` to `1.0` in `Scene::new()` and `Scene::clear()`.

Add `opacity_stack_is_empty()` to the `build_scene()` debug_assert.

### ContentMask Extension

**File:** `oriterm_ui/src/draw/scene/content_mask.rs`

Add an `opacity` field to `ContentMask`:

```rust
pub struct ContentMask {
    pub clip: Rect,
    /// Opacity multiplier (0.0-1.0). Applied during GPU conversion.
    pub opacity: f32,
}
```

Update `ContentMask::unclipped()` to set `opacity: 1.0`.

Update `current_content_mask()` in `paint.rs` to include `self.cumulative_opacity`.

### GPU Conversion

**File:** `oriterm/src/gpu/scene_convert/mod.rs`

The `opacity` parameter in `convert_scene()` is the compositor opacity. Widget-level opacity from `ContentMask` is a second multiplier. Update `convert_quad()` (line ~98) and its siblings:

```rust
fn convert_quad(quad: &Quad, writer: &mut InstanceWriter, scale: f32, opacity: f32, clip: [f32; 4]) {
    let effective_opacity = opacity * quad.content_mask.opacity;
    convert_rect_clipped(quad.bounds, &quad.style, writer, scale, effective_opacity, clip);
}
```

Same pattern for all 4 primitive conversion functions:
- `convert_quad()` (line ~98) — quads/rects
- `convert_scene_line()` (line ~109) — border lines, separators
- `convert_scene_text()` (line ~122) — text runs
- `convert_scene_icon()` (line ~142) — icon primitives

Note: `clip_from_mask()` (line ~85) currently only extracts the clip rect from `ContentMask`. It does NOT need to extract opacity — opacity is applied separately in each `convert_*` function by reading `content_mask.opacity` directly.

**Migration note:** Existing tests in `scene/tests.rs` assert on `ContentMask` equality against `ContentMask::unclipped()`. Adding the `opacity` field will break 6+ assertions. All must be updated to include the expected opacity value.

### Widget Usage

Widgets apply opacity by pushing it before painting:

```rust
// In a disabled control's paint():
if self.is_disabled {
    ctx.scene.push_opacity(0.4);
}
self.paint_contents(ctx);
if self.is_disabled {
    ctx.scene.pop_opacity();
}
```

This naturally composes — a disabled control inside a 70%-opacity panel renders at `0.4 * 0.7 = 0.28`.

### Convenience on DrawCtx

Optionally, add a helper on `DrawCtx` to scope opacity:

```rust
impl DrawCtx<'_> {
    /// Paint a closure with modified opacity.
    pub fn with_opacity(&mut self, opacity: f32, f: impl FnOnce(&mut DrawCtx<'_>)) {
        self.scene.push_opacity(opacity);
        f(self);
        self.scene.pop_opacity();
    }
}
```

### Use Cases from Mockup

| Element | Opacity | Notes |
|---------|---------|-------|
| Disabled slider/toggle/dropdown | 0.4 | CSS `opacity: 0.4` on `.setting-row.disabled` |
| Inactive sidebar icon | 0.7 | Non-active nav items have subdued icons |
| Placeholder text in search input | 0.5 | Placeholder text is semi-transparent |

---

## 06.2 Display/Visibility Toggle

### Problem

The settings dialog has 8 pages. Only one page is visible at a time. Currently, page switching likely requires either rebuilding the widget tree (expensive) or keeping all pages in the tree and skipping their paint (but they still consume layout space).

CSS `display: none` removes an element from layout entirely. We need an equivalent.

### Approach: LayoutBox Visibility

**File:** `oriterm_ui/src/layout/layout_box.rs`

Add a `visible` field to `LayoutBox`:

```rust
pub struct LayoutBox {
    // ... existing fields ...

    /// When false, this box and its children produce zero-size layout output
    /// and are skipped during painting and hit testing. Equivalent to CSS
    /// `display: none`. Default: true.
    pub visible: bool,
}
```

Default value: `true` (all existing code unaffected).

Add a builder method:

```rust
impl LayoutBox {
    /// Sets visibility. When false, the box produces zero layout size.
    #[must_use]
    pub fn with_visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }
}
```

### Layout Solver Integration

**File:** `oriterm_ui/src/layout/solver.rs`

In the flex solver's child layout loop, when a child `LayoutBox` has `visible: false`, skip it entirely — do not include it in the flex main-axis total, do not allocate space for it, and produce a `LayoutNode` with `Rect::ZERO`:

```rust
if !lb.visible {
    return LayoutNode {
        rect: Rect::ZERO,
        children: vec![],
        widget_id: lb.widget_id,
        // ... other fields defaulted
    };
}
```

**Integration points in the solver** (all must be updated):
1. Main-axis size accumulation — invisible children contribute 0 to the total.
2. Cross-axis max calculation — invisible children are excluded.
3. Gap calculation — gaps are not added between invisible children (a gap should only separate two visible siblings).
4. `FillPortion` distribution — invisible children's fill portions are excluded from the total.

This is the most efficient approach: invisible widgets cost only a single `visible` check in the solver. No layout computation, no child recursion, no paint, no hit testing.

### Paint Skip

**File:** Widget `paint()` implementations or the tree-walk paint system.

The paint system already respects layout bounds. A zero-size rect means nothing is visible within the clip. However, to be explicit and avoid wasted work, the tree-walk paint should skip nodes whose layout rect is zero-size:

```rust
// In the tree-walk paint loop:
if node.rect.width() <= 0.0 || node.rect.height() <= 0.0 {
    continue; // Skip invisible (display:none) widgets.
}
```

### Hit Test Skip

**File:** `oriterm_ui/src/hit_test/mod.rs`

The hit-test tree walk should similarly skip zero-size nodes. Since `Rect::ZERO.contains(point)` is always false, this happens naturally. But an explicit early-out avoids recursing into children:

```rust
if node.rect.is_empty() {
    return None; // No hit in zero-size node.
}
```

### Widget Usage

In the settings page container, each page widget reports visibility based on the active page:

```rust
impl Widget for SettingsPageContainer {
    fn layout(&self, ctx: &LayoutCtx) -> LayoutBox {
        let page_boxes: Vec<LayoutBox> = self.pages.iter().enumerate()
            .map(|(i, page)| {
                page.layout(ctx).with_visible(i == self.active_page_index)
            })
            .collect();
        LayoutBox::flex(Direction::Column, page_boxes)
    }
}
```

Only the active page computes layout and receives paint. All other pages are zero-cost.

### Alternative Considered: `Widget::visible()` Trait Method

Instead of putting visibility on `LayoutBox`, we could add `fn visible(&self) -> bool` to the `Widget` trait. This would be checked before calling `layout()`, `paint()`, and `on_input()`. However, this is less flexible — it requires the widget itself to know whether it should be visible, rather than letting the parent decide. The `LayoutBox` approach lets parents control visibility of children (matching CSS where the parent can set `display: none` on any child).

---

## 06.3 Pointer Events Control

### Current State

`LayoutBox` already has a `disabled: bool` field. When true, it is treated as `Sense::none()` during hit testing (from the hit test code). This means no hover, no click, no interaction.

### Relationship to Opacity

In CSS, `pointer-events: none` and `opacity` are independent properties. A faded element can still be interactive (tooltip on hover), and a full-opacity element can be non-interactive.

In our framework, the common case is that reduced opacity implies non-interactive: disabled controls are both faded and non-interactive. We should not couple these — keep `disabled` and `push_opacity()` as independent mechanisms.

### Convenience Pattern

Widgets that need the "disabled" visual treatment should apply both:

```rust
impl Widget for SliderWidget {
    fn layout(&self, ctx: &LayoutCtx) -> LayoutBox {
        let lb = self.build_layout(ctx);
        if self.is_disabled {
            lb.with_disabled(true)
        } else {
            lb
        }
    }

    fn paint(&self, ctx: &mut DrawCtx) {
        if self.is_disabled {
            ctx.scene.push_opacity(0.4);
        }
        self.draw_contents(ctx);
        if self.is_disabled {
            ctx.scene.pop_opacity();
        }
    }
}
```

### No New Fields Needed

The `disabled` field on `LayoutBox` already serves as `pointer-events: none`. No new types or fields are required for pointer event control. This subsection is about documenting the pattern and ensuring disabled widgets consistently apply opacity.

### Widget Audit

Review all interactive widgets to ensure they respect the disabled pattern:

| Widget | Has `disabled` support | Applies opacity when disabled |
|--------|----------------------|------------------------------|
| ButtonWidget | Check | Should apply 0.4 opacity |
| ToggleWidget | Check | Should apply 0.4 opacity |
| SliderWidget | Check | Should apply 0.4 opacity |
| DropdownWidget | Check | Should apply 0.4 opacity |
| TextInputWidget | Check | Should apply 0.4 opacity |
| CheckboxWidget | Check | Should apply 0.4 opacity |
| NumberInputWidget | Check | Should apply 0.4 opacity |

---

## 06.4 Tests

### Opacity Stack Tests

**File:** `oriterm_ui/src/draw/scene/tests.rs`

```rust
#[test]
fn opacity_stack_composes_multiplicatively() {
    let mut scene = Scene::new();
    assert_eq!(scene.current_opacity(), 1.0);

    scene.push_opacity(0.5);
    assert!((scene.current_opacity() - 0.5).abs() < f32::EPSILON);

    scene.push_opacity(0.5);
    assert!((scene.current_opacity() - 0.25).abs() < f32::EPSILON);

    scene.pop_opacity();
    assert!((scene.current_opacity() - 0.5).abs() < f32::EPSILON);

    scene.pop_opacity();
    assert_eq!(scene.current_opacity(), 1.0);
}

#[test]
fn opacity_clamps_to_0_1() {
    let mut scene = Scene::new();
    scene.push_opacity(1.5);
    assert_eq!(scene.current_opacity(), 1.0);

    scene.pop_opacity();
    scene.push_opacity(-0.5);
    assert_eq!(scene.current_opacity(), 0.0);
}

#[test]
fn opacity_applied_to_quad_content_mask() {
    let mut scene = Scene::new();
    scene.push_opacity(0.4);
    scene.push_quad(Rect::new(0.0, 0.0, 10.0, 10.0), RectStyle::filled(Color::RED));
    scene.pop_opacity();

    let quad = &scene.quads()[0];
    assert!((quad.content_mask.opacity - 0.4).abs() < f32::EPSILON);
}
```

### Visibility Tests

**File:** `oriterm_ui/src/layout/tests.rs`

```rust
#[test]
fn invisible_layout_box_produces_zero_size() {
    let visible = LayoutBox::leaf(100.0, 50.0);
    let invisible = LayoutBox::leaf(100.0, 50.0).with_visible(false);

    let container = LayoutBox::flex(Direction::Column, vec![visible, invisible]);
    let node = compute_layout(&container, Rect::new(0.0, 0.0, 200.0, 200.0));

    // Container should only be as tall as the visible child.
    assert!((node.rect.height() - 50.0).abs() < f32::EPSILON);
}

#[test]
fn invisible_child_not_hit_tested() {
    let invisible = LayoutBox::leaf(100.0, 50.0)
        .with_visible(false)
        .with_widget_id(WidgetId::next());

    let container = LayoutBox::flex(Direction::Column, vec![invisible]);
    let node = compute_layout(&container, Rect::new(0.0, 0.0, 200.0, 200.0));

    let hit = hit_test(&node, Point::new(50.0, 25.0));
    assert!(hit.is_none()); // Invisible widget not hittable.
}
```

### GPU Conversion Tests

**File:** `oriterm/src/gpu/scene_convert/tests.rs`

**NOTE:** The byte-offset test below assumes a specific instance record layout (`bg_a` at bytes[60..64]). Verify the actual offset at implementation time against `instance_writer.rs`. Prefer a higher-level helper that extracts the bg color from the instance record if one exists.

```rust
#[test]
fn content_mask_opacity_multiplies_with_compositor_opacity() {
    let mut scene = Scene::new();
    scene.push_opacity(0.5);
    scene.push_quad(Rect::new(0.0, 0.0, 10.0, 10.0), RectStyle::filled(Color::WHITE));
    scene.pop_opacity();

    let mut writer = InstanceWriter::new();
    convert_scene(&scene, &mut writer, None, 1.0, 0.8); // compositor opacity 0.8

    // Effective opacity = 0.8 * 0.5 = 0.4
    // White fill alpha should be 0.4
    // NOTE: Byte offset 60-64 is the bg_color alpha — verify against actual instance layout.
    let bytes = writer.as_bytes();
    let bg_a = f32::from_le_bytes(bytes[60..64].try_into().unwrap());
    assert!((bg_a - 0.4).abs() < 0.01);
}
```

---

## 06.R Third Party Review Findings

Reserved for findings from `/review-plan` or external review. Not actionable until populated.

---

## 06.5 Build & Verify

### Gate

```bash
./build-all.sh
./clippy-all.sh
./test-all.sh
```

### Verification Steps

1. `cargo build --target x86_64-pc-windows-gnu` — cross-compile succeeds
2. `cargo clippy --target x86_64-pc-windows-gnu` — no new warnings
3. `cargo test -p oriterm_ui` — opacity stack and visibility tests pass
4. `cargo test -p oriterm` — scene convert opacity multiplication tests pass
5. Visual: disabled slider renders at 40% opacity
6. Visual: page switching hides non-active pages completely

### Checklist

- [ ] `./build-all.sh` passes
- [ ] `./clippy-all.sh` passes
- [ ] `./test-all.sh` passes
- [ ] Scene opacity stack balanced after `build_scene()` (debug_assert)
- [ ] No regression in existing widget rendering (all existing opacity values unchanged)
- [ ] Existing scene tests updated for new `ContentMask.opacity` field
