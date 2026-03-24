---
section: "04"
title: "Line Height Control"
status: complete
reviewed: true
third_party_review:
  status: complete
  updated: 2026-03-24
goal: "TextStyle.line_height overrides logical UI text block height while preserving the current logical-height / physical-baseline render contract; cache keys and test measurers honor it; generic wrapped-text behavior remains out of scope until real wrapping exists"
inspired_by:
  - "CSS line-height (https://developer.mozilla.org/en-US/docs/Web/CSS/line-height)"
  - "GPUI line-height concepts (~/projects/reference_repos/gui_repos/zed/crates/gpui/src/text_system.rs)"
depends_on: ["01"]
sections:
  - id: "04.1"
    title: "TextStyle Line Height API"
    status: complete
  - id: "04.2"
    title: "Measurer, Baseline, and Cache Integration"
    status: complete
  - id: "04.3"
    title: "Consumer Boundaries"
    status: complete
  - id: "04.4"
    title: "Tests"
    status: complete
  - id: "04.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "04.5"
    title: "Build & Verify"
    status: complete
---

# Section 04: Line Height Control

## Problem

The current draft described the feature too broadly and mixed several different line-height
concepts together.

What the tree actually has today:

- [oriterm_ui/src/text/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/text/mod.rs) has no
  `TextStyle.line_height`.
- [oriterm/src/font/shaper/ui_measurer.rs](/home/eric/projects/ori_term/oriterm/src/font/shaper/ui_measurer.rs)
  returns `TextMetrics.height` and `ShapedText.height` in logical pixels, but leaves
  `ShapedText.baseline` in physical pixels for [scene_convert/text.rs](/home/eric/projects/ori_term/oriterm/src/gpu/scene_convert/text.rs).
- [oriterm/src/font/shaper/ui_text.rs](/home/eric/projects/ori_term/oriterm/src/font/shaper/ui_text.rs)
  still treats `TextOverflow::Wrap` as a placeholder and always returns `line_count: 1`.
- the app already has a separate terminal-grid config setting,
  [font_config.rs](/home/eric/projects/ori_term/oriterm/src/config/font_config.rs), where
  `config.font.line_height` adjusts terminal cell height. That is not the same feature as UI
  `TextStyle` line height.

The real missing capability is narrower:

1. UI text styles cannot request a custom line box height.
2. The cached measurer cannot distinguish text that differs only by line-height.
3. Widgets that want CSS-like vertical rhythm must currently fake it with fixed gaps or
   hand-authored per-line spacing.

## Corrected Scope

This section builds the framework capability for UI text line-height. It does not attempt to solve
generic wrapped-text layout across the widget system yet.

- Add `line_height` to `TextStyle`.
- Apply it in `UiFontMeasurer`, which already owns logical/physical conversion.
- Normalize invalid overrides (`<= 0.0`, `NaN`, infinities) back to metric-driven behavior at the
  measurer/cache boundary so public `TextStyle` construction cannot create nonsensical layout
  results.
- Adjust baseline in physical space so the existing renderer contract stays valid.
- Update `CachedTextMeasurer` and `MockMeasurer`.
- Leave generic multiline/wrap behavior out of scope until `TextOverflow::Wrap` is real.

## Representative Mockup Usage

The mockup uses line-height in several places, but not all vertical spacing comes from line-height.
The original section overstated this by attributing settings-row sizing to line-height when the
mockup mostly uses row `min-height`, padding, and gaps there.

Representative actual usages in [mockups/settings-brutal.html](/home/eric/projects/ori_term/mockups/settings-brutal.html):

| Element | CSS `font-size` | CSS `line-height` | Notes |
|---|---|---|---|
| `body` base text | `13px` | `1.5` | Global baseline for normal copy |
| `.section-desc` | `12px` | `1.5` | Section helper copy |
| `.setting-label .name .tag` | `9px` | `1.3` | Compact badges |
| `.num-stepper-btns button` | `8px` | `1` | Tight glyph box |
| `.scheme-terminal` | `11px` | `1.4` | Multiline preview |
| `.font-preview-text` | `15px` | `1.6` | Multiline font preview |
| `kbd` | `11px` | `1.4` | Inline keycap text |
| `.keybind-dialog-hint` | `11.5px` | `1.5` | Wrapped helper copy |
| `.info-callout p`, `.warning-callout p`, `.shell-preview` | `12px` | `1.5` | Informational paragraphs |

This confirms the capability is needed, but it also shows that broad consumer adoption belongs in
later fidelity sections. Section 04 should not claim that every one of these widgets is updated
here.

---

## 04.1 TextStyle Line Height API

### Goal

Add an optional UI-text line-height override without changing existing behavior for callers that do
not opt in.

### Files

- [oriterm_ui/src/text/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/text/mod.rs)
- [oriterm_ui/src/widgets/label/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/label/mod.rs)
- [oriterm_ui/src/text/tests.rs](/home/eric/projects/ori_term/oriterm_ui/src/text/tests.rs)

### Proposed Field

```rust
pub struct TextStyle {
    pub font_family: Option<String>,
    pub size: f32,
    pub weight: FontWeight,
    pub color: Color,
    pub align: TextAlign,
    pub overflow: TextOverflow,
    pub letter_spacing: f32,
    pub text_transform: TextTransform,
    pub line_height: Option<f32>,
}
```

Semantics:

- `None` = use natural font metrics from the shaped run
- `Some(multiplier)` where `multiplier.is_finite() && multiplier > 0.0` = logical line box height
  is `style.size * multiplier`
- invalid overrides (`<= 0.0`, `NaN`, infinities) are normalized to `None` by the measurer/cache
  path because `TextStyle` is a public struct and callers can bypass the builder

### Builder

```rust
#[must_use]
pub fn with_line_height(mut self, multiplier: f32) -> Self {
    self.line_height = Some(multiplier);
    self
}
```

`with_line_height()` may still use a `debug_assert!` for finite positive input, but the real
correctness boundary is the measurer/cache path, not the builder, because callers can always
construct `TextStyle` directly.

### Why `Option<f32>` Is Acceptable Here

The draft’s `Option<f32>` approach is still the pragmatic choice. This feature is an override on
top of existing metric-driven behavior, so `None` cleanly represents “don’t change anything.” A
full `LineHeight` enum is possible but not necessary for this section’s scope.

### LabelStyle Forwarding

Because settings headers and labels frequently go through `LabelStyle`, add a matching optional
field there as well and forward it when building a `TextStyle`.

`LabelWidget::text_style()` in [label/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/label/mod.rs)
currently builds a `TextStyle` via a builder chain:

```rust
TextStyle::new(self.style.font_size, self.style.color)
    .with_weight(self.style.weight)
    .with_overflow(self.style.overflow)
    .with_letter_spacing(self.style.letter_spacing)
    .with_text_transform(self.style.text_transform)
```

The simplest forwarding is to set the field directly after construction, since `line_height` is
`Option<f32>` and `None` is the default (no-op):

```rust
let mut ts = TextStyle::new(self.style.font_size, self.style.color)
    .with_weight(self.style.weight)
    .with_overflow(self.style.overflow)
    .with_letter_spacing(self.style.letter_spacing)
    .with_text_transform(self.style.text_transform);
ts.line_height = self.style.line_height;
ts
```

### Checklist

- [x] Add `line_height: Option<f32>` to `TextStyle` (after `text_transform` field) with `///` doc comment
- [x] Add `TextStyle::with_line_height(mut self, multiplier: f32) -> Self` builder method with `#[must_use]` and `///` doc comment
- [x] Update `TextStyle::default()` to set `line_height: None`
- [x] Update `TextStyle::new()` to set `line_height: None`
- [x] Add `TextStyle::normalized_line_height(&self) -> Option<f32>` validation method with `///` doc comment
- [x] Add `line_height: Option<f32>` to `LabelStyle` with `///` doc comment
- [x] Update `LabelStyle::from_theme()` to set `line_height: None`
- [x] Update `LabelStyle::Default` impl (delegates to `from_theme`) -- no separate change needed
- [x] Update `LabelWidget::text_style()` to forward `line_height` from `LabelStyle`
- [x] Update existing `text_style_default` test in `text/tests.rs` to assert `line_height` is `None`
- [x] Update existing `text_style_new` test in `text/tests.rs` to assert `line_height` is `None`
- [x] Verify `text/mod.rs` stays under 500 lines after changes (currently 296 lines, ~15 lines added)
- [x] Verify `label/mod.rs` stays under 500 lines after changes (currently 131 lines, ~5 lines added)

---

## 04.2 Measurer, Baseline, and Cache Integration

### Goal

Apply the override where unit conversion already happens, and keep the renderer’s current
baseline-space contract intact.

### Files

- [oriterm/src/font/shaper/ui_measurer.rs](/home/eric/projects/ori_term/oriterm/src/font/shaper/ui_measurer.rs)
- [oriterm/src/font/shaper/cached_measurer/mod.rs](/home/eric/projects/ori_term/oriterm/src/font/shaper/cached_measurer/mod.rs)
- [oriterm_ui/src/testing/mock_measurer.rs](/home/eric/projects/ori_term/oriterm_ui/src/testing/mock_measurer.rs)
- [oriterm/src/gpu/scene_convert/text.rs](/home/eric/projects/ori_term/oriterm/src/gpu/scene_convert/text.rs)
  for contract verification only; no code change should be required there

### Correct Boundary

Keep this logic in `UiFontMeasurer`, not `ui_text.rs`.

Why:

- `ui_text.rs` shapes in physical pixels and does not own the logical `style.size * multiplier`
  computation
- `UiFontMeasurer` already converts logical max width to physical and converts returned sizes back
- the current `ShapedText` contract is asymmetric: layout-facing `height` is logical by the time
  widgets see it, but render-facing `baseline` is still consumed as physical pixels

Use a single normalization helper before applying the override. This helper is needed in both
`UiFontMeasurer` (for measure/shape) and `CachedTextMeasurer` (for cache key construction).
Two placement options:

1. **On `TextStyle` itself** in `oriterm_ui/src/text/mod.rs` as a `pub` method — cleanest, since
   `TextStyle` is the type being normalized. Both `UiFontMeasurer` and `CachedTextMeasurer` can
   call `style.normalized_line_height()`.
2. **As a free function** in `oriterm/src/font/shaper/ui_measurer.rs` with `pub(super)` visibility
   — keeps it in the shaper module, accessible to both siblings.

Option 1 is preferred because it keeps validation adjacent to the type definition and is accessible
from any crate (including test code in `oriterm_ui`). This respects the crate boundary: `TextStyle`
is owned by `oriterm_ui`, and pure validation on that type belongs there:

```rust
// In oriterm_ui/src/text/mod.rs, on impl TextStyle:
/// Returns the line-height multiplier if valid (finite and positive), or `None`.
///
/// Invalid overrides (`<= 0.0`, `NaN`, infinities) normalize to `None`,
/// falling back to natural font metrics.
pub fn normalized_line_height(&self) -> Option<f32> {
    self.line_height
        .filter(|m| m.is_finite() && *m > 0.0)
}
```

### Measure Path

For `measure()`, the rule is straightforward for the current single-line behavior. The current
`UiFontMeasurer::measure()` calls `ui_text::shape_text()` and converts physical results to
logical:

```rust
fn measure(&self, text: &str, style: &TextStyle, _max_width: f32) -> TextMetrics {
    let collection = self.collection_for_style(style);
    let phys_spacing = style.letter_spacing.max(0.0) * self.scale;
    let shaped = ui_text::shape_text(text, style, f32::INFINITY, phys_spacing, collection);
    let height = match style.normalized_line_height() {
        Some(multiplier) => style.size * multiplier,
        None => shaped.height / self.scale,
    };
    TextMetrics {
        width: shaped.width / self.scale,
        height,
        line_count: 1,
    }
}
```

### Shape Path

For `shape()`, do the baseline math in physical space before finalizing the logical height:

```rust
fn shape(&self, text: &str, style: &TextStyle, max_width: f32) -> ShapedText {
    let collection = self.collection_for_style(style);
    let phys_spacing = style.letter_spacing.max(0.0) * self.scale;
    let mut shaped =
        ui_text::shape_text(text, style, max_width * self.scale, phys_spacing, collection);

    if let Some(multiplier) = style.normalized_line_height() {
        let target_logical_height = style.size * multiplier;
        let target_physical_height = target_logical_height * self.scale;
        // shaped.height is still in physical pixels at this point (from ui_text::shape_text).
        let half_leading_physical = (target_physical_height - shaped.height) / 2.0;
        shaped.baseline += half_leading_physical;
        shaped.height = target_logical_height;
    } else {
        shaped.height /= self.scale;
    }

    shaped.width /= self.scale;
    shaped
}
```

This is the critical correction from the original draft: do not add a logical-pixel half-leading
value directly to `shaped.baseline`, because baseline is still consumed in physical space by
[scene_convert/text.rs](/home/eric/projects/ori_term/oriterm/src/gpu/scene_convert/text.rs).

Note: `shaped.height` at the point of `half_leading_physical` computation is still in physical
pixels — `ui_text::shape_text()` returns `cell_metrics().height` (physical). The line-height branch
computes leading in physical space, adjusts baseline (physical), then sets height to the target
logical value. The else branch divides by scale as the current code does.

### Cache Key Update

`CachedTextMeasurer` must include line-height in `TextCacheKey` because it changes both measured
height and shaped baseline.

The current `TextCacheKey` struct (showing all existing fields plus the new one):

```rust
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct TextCacheKey {
    text: String,
    font_family: Option<String>,
    size_hundredths: u32,
    weight: FontWeight,
    overflow: TextOverflow,
    max_width_hundredths: u32,
    scale_hundredths: u32,
    letter_spacing_hundredths: u32,
    text_transform: TextTransform,
    line_height_hundredths: Option<u32>,  // NEW
}
```

That field should store the normalized finite-positive override, not the raw `TextStyle` value, so
invalid overrides collapse to the same cache key as `None`.

Update both `TextCacheKey::new()` and `TextCacheKey::for_measure()` to populate the new field.
Use `style.normalized_line_height()` to convert before calling `float_to_hundredths`,
mapping `None` or invalid values to `None` in the key:

```rust
line_height_hundredths: style.normalized_line_height().map(float_to_hundredths),
```

### MockMeasurer Update

`MockMeasurer` should honor the field too, otherwise widget tests cannot exercise the new style
knob. It does not need to emulate font internals perfectly, but it should at least:

- return overridden logical height from `measure()`
- shift shaped baseline by half-leading relative to its existing synthetic baseline model

**Name collision warning:** `MockMeasurer` already has a `pub line_height: f32` field (default
16.0) that represents the mock's natural line height — analogous to `cell_metrics().height` in
production. The CSS-style `TextStyle.line_height: Option<f32>` is a *multiplier* override. When
implementing the override in MockMeasurer:

- The mock's natural height = `self.line_height` (16.0 by default)
- The overridden height = `style.size * multiplier` (where `multiplier` comes from
  `style.normalized_line_height()`)
- The half-leading = `(overridden_height - self.line_height) / 2.0`
- The baseline shifts by half-leading: `self.line_height * 0.8 + half_leading`

**Important:** The mock has no scale factor, so "physical" and "logical" are the same for
MockMeasurer. The half-leading formula is simpler than in UiFontMeasurer. Use
`style.normalized_line_height()` (not raw `style.line_height`) so invalid overrides fall back to
the mock's natural behavior.

Concrete `measure()` change:

```rust
fn measure(&self, text: &str, style: &TextStyle, max_width: f32) -> TextMetrics {
    let transformed = style.text_transform.apply(text);
    let full_width = self.char_width * transformed.len() as f32;
    let effective_height = match style.normalized_line_height() {
        Some(multiplier) => style.size * multiplier,
        None => self.line_height,
    };
    if max_width.is_finite() && full_width > max_width {
        let line_count = (full_width / max_width).ceil() as u32;
        TextMetrics {
            width: max_width,
            height: effective_height * line_count as f32,
            line_count,
        }
    } else {
        TextMetrics {
            width: full_width,
            height: effective_height,
            line_count: 1,
        }
    }
}
```

Concrete `shape()` change:

```rust
fn shape(&self, text: &str, style: &TextStyle, _max_width: f32) -> ShapedText {
    let transformed = style.text_transform.apply(text);
    // ... glyph construction unchanged ...
    let width = self.char_width * transformed.len() as f32;
    let (height, baseline) = match style.normalized_line_height() {
        Some(multiplier) => {
            let target = style.size * multiplier;
            let half_leading = (target - self.line_height) / 2.0;
            (target, self.line_height * 0.8 + half_leading)
        }
        None => (self.line_height, self.line_height * 0.8),
    };
    ShapedText::new(glyphs, width, height, baseline, 0, style.weight.value())
}
```

Do not rename `MockMeasurer.line_height` -- it is a public field used by many existing tests.
Instead, use clear local variable names in the implementation (e.g. `target`,
`half_leading`) to avoid ambiguity.

### Physical/Logical Coordinate Verification

To verify the shape-path half-leading math is correct, trace the physical height source:

1. `ui_text::shape_text()` calls `shape_to_shaped_text()` which calls
   `collection.cell_metrics()` to get `metrics.height` and `metrics.baseline`.
2. These values are in **physical pixels** (the collection is loaded at the physical ppem size).
3. `ShapedText::new()` stores them directly -- no scale conversion inside `ui_text.rs`.
4. Back in `UiFontMeasurer::shape()`, `shaped.height` and `shaped.baseline` are both physical.
5. The line-height branch computes `target_physical_height = style.size * multiplier * self.scale`
   and subtracts `shaped.height` (physical) to get `half_leading_physical`.
6. It adds `half_leading_physical` to `shaped.baseline` (physical) -- correct.
7. It then sets `shaped.height = target_logical_height` (logical) -- matching the else branch.
8. `scene_convert/text.rs` consumes `shaped.baseline` as physical via
   `let baseline = shaped.baseline;` -- no change needed there.

This trace confirms the plan's shape-path snippet is architecturally sound.

### Out of Scope Here

Do not move line-height logic into `ui_text.rs`, and do not change `scene_convert` to reinterpret
baseline units. That would broaden this section into a rendering-contract refactor.

### Checklist

- [x] Apply `line_height` override in `UiFontMeasurer::measure()` (use `style.normalized_line_height()`)
- [x] Apply `line_height` override in `UiFontMeasurer::shape()` with baseline adjustment in physical space
- [x] Add `line_height_hundredths: Option<u32>` field to `TextCacheKey` struct with inline comment
- [x] Update `TextCacheKey::new()` to populate `line_height_hundredths` via `style.normalized_line_height().map(float_to_hundredths)`
- [x] Update `TextCacheKey::for_measure()` to populate `line_height_hundredths` the same way
- [x] Update `MockMeasurer::measure()` to use `style.normalized_line_height()` for effective height
- [x] Update `MockMeasurer::shape()` to adjust height and baseline when line-height override is set
- [x] Convert `mock_measurer.rs` to directory module: `testing/mock_measurer/mod.rs` + `testing/mock_measurer/tests.rs` (required by test-organization.md for sibling tests.rs pattern)
- [x] Verify `scene_convert/text.rs` still works unchanged (baseline stays physical)
- [x] Verify `ui_measurer.rs` stays under 500 lines after changes (currently 91 lines, ~15 lines added)
- [x] Verify `cached_measurer/mod.rs` stays under 500 lines after changes (currently 314 lines, ~5 lines added)
- [x] Verify `mock_measurer/mod.rs` stays under 500 lines after changes (currently 72 lines, ~15 lines added)

---

## 04.3 Consumer Boundaries

### Goal

Be explicit about what this section does and does not adopt immediately.

### Current Consumer Reality

Several widgets already stack multiple lines manually instead of relying on generic wrapped text:

- [setting_row/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/setting_row/mod.rs)
  matches the mockup's main settings rows more closely than a line-height feature does today: it
  uses a fixed name/description gap plus row `min-height`, not wrapped-text line boxes
- [dialog/rendering.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/dialog/rendering.rs)
  measures a single line and multiplies by line count for preview content
- [code_preview/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/code_preview/mod.rs)
  currently hard-codes `CODE_FONT_SIZE + 4.0` for line stepping
- [keybind/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/keybind/mod.rs) and
  [status_badge/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/status_badge/mod.rs)
  derive component height from measured text height plus padding

### Correct Promise

Section 04 should build the line-height capability and thread it through shared measurement and
shape caches. It should not promise all consumer adoption here.

The safe commitment is:

1. Any widget constructing a `TextStyle` can opt into line-height once the field exists.
2. Widgets that already stack lines manually can adopt it later by measuring a single styled line
   and using that returned height.
3. Generic `TextOverflow::Wrap` and true multi-line shaping stay out of scope until the text system
   actually supports wrapping.

### Important Non-Goal

Do not conflate this section with terminal grid line-height configuration. The existing
`config.font.line_height` setting in the Font page controls terminal cell spacing, not the UI text
styles in `oriterm_ui`.

### Checklist

- [x] Do not claim generic wrapped-text support in this section
- [x] Do not claim the existing terminal `config.font.line_height` setting exercises this feature
- [x] Limit this section to shared capability plus wrapper-style forwarding
- [x] Let later visual-fidelity sections choose which widgets adopt `TextStyle.line_height`

---

## 04.4 Tests

### Test Location Rules

Per `test-organization.md`, all tests go in sibling `tests.rs` files. Locations:

| Test category | File | Why here |
|---|---|---|
| `TextStyle` field/builder/validation | `oriterm_ui/src/text/tests.rs` | `TextStyle` owned by `oriterm_ui`; tests use `super::` |
| `TextCacheKey` differentiation | `oriterm/src/font/shaper/cached_measurer/tests.rs` | `TextCacheKey` owned by `cached_measurer`; tests use `super::` |
| `UiFontMeasurer` line-height math | `oriterm/src/font/shaper/tests.rs` | Requires real `FontCollection`; `shaper/tests.rs` already has `test_ui_measurer()` helper and existing `UiFontMeasurer` tests |
| `MockMeasurer` line-height behavior | `oriterm_ui/src/testing/mock_measurer/tests.rs` (NEW) | `MockMeasurer` owned by `oriterm_ui`; must convert `mock_measurer.rs` to directory module per test-organization.md |
| `LabelStyle` forwarding | `oriterm_ui/src/widgets/label/tests.rs` | Already has label layout tests; uses `MockMeasurer` |

### TextStyle Tests (`oriterm_ui/src/text/tests.rs`)

Existing tests `text_style_default` and `text_style_new` must be updated to assert `line_height`
is `None`. New tests:

1. **`text_style_with_line_height_sets_override`** -- `TextStyle::new(...).with_line_height(1.5)`
   sets `line_height` to `Some(1.5)`. Verifies the builder stores the value.

2. **`text_style_with_line_height_builder_chain`** -- chaining `with_line_height` with other
   builders (`with_weight`, `with_letter_spacing`) preserves all fields. Guards against accidental
   field reset.

3. **`normalized_line_height_valid_values`** -- `Some(1.0)`, `Some(1.5)`, `Some(0.5)` all pass
   through `normalized_line_height()` unchanged. Verifies valid inputs are not filtered.

4. **`normalized_line_height_filters_zero`** -- `Some(0.0)` normalizes to `None`.

5. **`normalized_line_height_filters_negative`** -- `Some(-1.0)` and `Some(-0.001)` normalize
   to `None`.

6. **`normalized_line_height_filters_nan`** -- `Some(f32::NAN)` normalizes to `None`.

7. **`normalized_line_height_filters_infinity`** -- `Some(f32::INFINITY)` and
   `Some(f32::NEG_INFINITY)` both normalize to `None`.

8. **`normalized_line_height_none_returns_none`** -- `line_height: None` returns `None` from
   `normalized_line_height()`. Verifies the passthrough.

9. **`text_style_debug_includes_line_height`** -- `format!("{:?}", style)` contains
    `"line_height"`. Verifies the derived `Debug` impl includes the new field.

10. **`text_style_partial_eq_distinguishes_line_height`** -- two styles that differ only by
    `line_height` are not equal. Verifies derived `PartialEq` includes the field.

### CachedTextMeasurer Tests (`oriterm/src/font/shaper/cached_measurer/tests.rs`)

Uses existing `DummyMeasurer` test helper. `DummyMeasurer` ignores style, so these tests verify
cache-key differentiation via hit/miss stats, not returned values. No changes to `DummyMeasurer`
are needed. New tests:

1. **`cache_key_changes_when_line_height_differs`** -- `TextCacheKey::new()` with
   `line_height: Some(1.5)` vs `line_height: None` produces different keys.

2. **`cache_key_same_when_line_height_same`** -- two styles with identical `Some(1.5)` produce
   the same key.

3. **`cache_key_invalid_line_height_same_as_none`** -- `Some(0.0)`, `Some(-1.0)`,
   `Some(f32::NAN)`, `Some(f32::INFINITY)` all produce the same key as `None`.
   Critical: invalid overrides must not pollute the cache with distinct entries.

4. **`cache_miss_when_line_height_changes`** -- use `CachedTextMeasurer` to measure same text
   with `None` then `Some(1.5)`. Second call must be a miss (not a stale hit).

5. **`cache_hit_same_line_height_across_frames`** -- populate cache with `Some(1.5)`, create
   new `CachedTextMeasurer` for next frame, same text+style hits.

### UiFontMeasurer Integration Tests (`oriterm/src/font/shaper/tests.rs`)

Uses existing `test_ui_measurer()` helper and `super::UiFontMeasurer`. These tests
exercise real font metrics, so exact pixel values are not asserted — use relational assertions.

1. **`measure_returns_styled_height_when_line_height_set`** -- `measure()` with
   `line_height: Some(1.5)` and `size: 13.0` returns `height == 19.5` (13.0 * 1.5).
   This is an exact assertion because the formula is `size * multiplier`, independent of font
   metrics.

2. **`shape_returns_same_logical_height_as_measure`** -- for the same text and style (with
   `line_height: Some(1.4)`), `shape().height == measure().height`. The shape path and measure
   path must agree on logical height.

3. **`width_unchanged_by_line_height`** -- measure the same text with `None` vs `Some(1.8)`.
   Width must be identical. Line-height only affects vertical metrics.

4. **`baseline_shifts_with_larger_line_height`** -- `shape()` with `Some(1.8)` produces a
   larger `baseline` than `shape()` with `None`. Larger line-height adds positive half-leading,
   pushing baseline downward.

5. **`baseline_shifts_with_smaller_line_height`** -- `shape()` with `Some(0.8)` produces a
   smaller `baseline` than `shape()` with `None`. Tighter line-height adds negative
   half-leading (target < natural), pulling baseline upward.

6. **`line_height_correct_at_scale_2`** -- construct `UiFontMeasurer` with `scale: 2.0`. Verify:
   - `measure().height == style.size * multiplier` (logical, independent of scale)
   - `shape().height == style.size * multiplier` (same logical height)
   - `shape().baseline` differs from the `scale: 1.0` case (physical baseline accounts for
     scale in the half-leading computation: `target_physical = logical * scale`)
   This is the regression test for TPR-04-001 (the original draft's logical/physical bug).

7. **`invalid_line_height_falls_back_to_natural`** -- for each of `Some(0.0)`, `Some(-1.0)`,
   `Some(f32::NAN)`, `Some(f32::INFINITY)`: `measure().height` and `shape().height` match the
   result from `line_height: None`. All four must be identical to the natural case.

8. **`line_height_one_produces_size_times_one`** -- `Some(1.0)` with `size: 13.0` produces
   `height == 13.0`. For real fonts, natural height is ~1.3x-1.5x the size (ascent + descent),
   so `Some(1.0)` is NOT the same as `None`. This verifies the multiplier math is
   `size * multiplier`, not `natural_height * multiplier`.

9. **`empty_text_with_line_height`** -- `shape("")` with `line_height: Some(1.5)` still returns
   the overridden height (not 0.0). Empty text should still report the styled line box height
   for layout purposes (consistent with CSS: an empty `<span>` with `line-height` still
   occupies vertical space).

10. **`line_height_with_letter_spacing`** -- combine `line_height: Some(1.5)` with
    `letter_spacing: 2.0`. Verify height is `size * 1.5` (unaffected by spacing) and width is
    greater than without spacing. Confirms the two features are orthogonal.

11. **`line_height_with_text_transform`** -- combine `line_height: Some(1.4)` with
    `text_transform: Uppercase`. Verify height is `size * 1.4` regardless of transform.
    Width may differ from lowercase due to different glyph widths.

### MockMeasurer Tests (`oriterm_ui/src/testing/mock_measurer/tests.rs`)

**Prerequisite:** convert `mock_measurer.rs` to `mock_measurer/mod.rs` + `mock_measurer/tests.rs`
per test-organization.md. Add `#[cfg(test)] mod tests;` at the bottom of `mock_measurer/mod.rs`.

Tests use `super::MockMeasurer` and `crate::text::TextStyle`. No font infrastructure needed.

1. **`mock_measure_line_height_overrides_height`** -- `MockMeasurer::STANDARD` with
   `style.line_height = Some(1.5)` and `style.size = 12.0` returns `height = 18.0`
   (12.0 * 1.5, not 16.0 the mock's natural height).

2. **`mock_measure_none_line_height_uses_natural`** -- `line_height: None` returns
   `self.line_height` (16.0 for STANDARD). Verifies backward compatibility.

3. **`mock_measure_invalid_line_height_uses_natural`** -- `Some(0.0)`, `Some(-1.0)`,
   `Some(f32::NAN)` all return 16.0 (natural). Verifies the mock calls
   `normalized_line_height()`.

4. **`mock_shape_baseline_shifts_with_line_height`** -- with `Some(1.5)` and `size = 12.0`:
   target = 18.0, natural = 16.0, half_leading = 1.0, expected baseline = 16.0 * 0.8 + 1.0
   = 13.8. Verifies the half-leading formula.

5. **`mock_shape_height_matches_measure_height`** -- for the same style, `shape().height`
   equals `measure().height`. The two paths must agree.

6. **`mock_measure_line_height_multiline`** -- with `line_height: Some(2.0)`, `size: 10.0`,
   and text long enough to wrap: each line uses the overridden height (20.0), so total
   `height = 20.0 * line_count`.

7. **`mock_shape_negative_half_leading`** -- with `Some(0.5)` and `size: 12.0`:
   target = 6.0, natural = 16.0, half_leading = (6.0 - 16.0) / 2.0 = -5.0.
   baseline = 16.0 * 0.8 + (-5.0) = 7.8. Verifies negative half-leading is not clamped.

### LabelStyle Forwarding Test (`oriterm_ui/src/widgets/label/tests.rs`)

1. **`label_style_line_height_forwarded_to_layout`** -- create a `LabelStyle` with
   `line_height: Some(1.5)` and `font_size: 12.0`. Build a `LabelWidget`, call `layout()`.
   The returned `intrinsic_height` must be 18.0 (12.0 * 1.5), not 16.0 (mock's natural).
   This verifies the full pipeline: `LabelStyle` -> `LabelWidget::text_style()` ->
   `TextStyle.line_height` -> `MockMeasurer::measure()` -> layout.

### Checklist

- [x] Update existing `text_style_default` test in `text/tests.rs` to assert `line_height` is `None`
- [x] Update existing `text_style_new` test in `text/tests.rs` to assert `line_height` is `None`
- [x] Add tests 1-10 above in `oriterm_ui/src/text/tests.rs`
- [x] Add tests 1-5 above in `oriterm/src/font/shaper/cached_measurer/tests.rs`
- [x] Add tests 1-11 above in `oriterm/src/font/shaper/tests.rs` (reuse `test_ui_measurer()` helper)
- [x] Convert `mock_measurer.rs` to directory module and add tests 1-7 in `oriterm_ui/src/testing/mock_measurer/tests.rs`
- [x] Add `label_style_line_height_forwarded_to_layout` in `oriterm_ui/src/widgets/label/tests.rs`
- [x] All tests pass: `timeout 150 cargo test -p oriterm_ui` and `timeout 150 cargo test -p oriterm`

---

## 04.R Third Party Review Findings

- [x] `[TPR-04-001][high]` [plans/ui-css-framework/section-04-line-height.md](/home/eric/projects/ori_term/plans/ui-css-framework/section-04-line-height.md) - The original draft added logical half-leading directly to `ShapedText.baseline` inside `UiFontMeasurer::shape()`, but the current renderer still consumes `baseline` in physical pixels. That would misplace text whenever the scale factor is not `1.0`. Resolved: the section now requires baseline math in physical space and keeps the current renderer contract unchanged on 2026-03-23.

- [x] `[TPR-04-002][medium]` [plans/ui-css-framework/section-04-line-height.md](/home/eric/projects/ori_term/plans/ui-css-framework/section-04-line-height.md) - The original section promised generic multiline behavior even though `TextOverflow::Wrap` is still effectively a placeholder in [ui_text.rs](/home/eric/projects/ori_term/oriterm/src/font/shaper/ui_text.rs) and `UiFontMeasurer::measure()` always reports `line_count: 1`. Resolved: the section now scopes itself to single-line shaped text plus manual multiline consumers, and defers true wrapped-text semantics until wrapping exists on 2026-03-23.

- [x] `[TPR-04-003][medium]` [plans/ui-css-framework/section-04-line-height.md](/home/eric/projects/ori_term/plans/ui-css-framework/section-04-line-height.md) - The original mockup usage table incorrectly treated settings-row layout height as a line-height feature, when the mockup actually gets most of that vertical rhythm from row `min-height`, padding, and gaps. Resolved: the section now lists the real line-height-driven mockup elements such as descriptions, badges, previews, callouts, and keybinding hints on 2026-03-23.

- [x] `[TPR-04-004][medium]` [plans/ui-css-framework/section-04-line-height.md](/home/eric/projects/ori_term/plans/ui-css-framework/section-04-line-height.md) - The original draft blurred UI text line-height with the existing terminal-grid `config.font.line_height` setting exposed on the Font page. Resolved: the section now explicitly scopes itself to `TextStyle`/`oriterm_ui` behavior and treats terminal grid line height as a separate subsystem on 2026-03-23.

- [x] `[TPR-04-005][medium]` [plans/ui-css-framework/section-04-line-height.md](/home/eric/projects/ori_term/plans/ui-css-framework/section-04-line-height.md) - The original draft suggested optional integration in `ui_text.rs`, but that layer does not own the logical `style.size * multiplier` calculation and is the wrong boundary for the current baseline contract. Resolved: the section now requires the override in `UiFontMeasurer` and cache/test helpers instead of broadening the shaping layer on 2026-03-23.

- [x] `[TPR-04-006][medium]` [plans/ui-css-framework/section-04-line-height.md](/home/eric/projects/ori_term/plans/ui-css-framework/section-04-line-height.md) - The corrected draft still left `Some(0.0)`, negative values, and non-finite `line_height` inputs unspecified even though `TextStyle` is a public struct. Without explicit normalization, layout and cache keys could observe zero/negative/NaN heights. Resolved: the section now requires finite-positive normalization at the measurer/cache boundary and adds test coverage for invalid overrides on 2026-03-23.

---

## 04.5 Build & Verify

### Gate

```bash
timeout 150 ./build-all.sh
timeout 150 ./clippy-all.sh
timeout 150 ./test-all.sh
```

### Focused Verification

```bash
timeout 150 cargo test -p oriterm_ui text
timeout 150 cargo test -p oriterm_ui label
timeout 150 cargo test -p oriterm_ui mock_measurer
timeout 150 cargo test -p oriterm font::shaper
timeout 150 cargo test -p oriterm cached_measurer
```

### Module Structure Verification

- `oriterm_ui/src/testing/mock_measurer/` is now a directory with `mod.rs` and `tests.rs`
- `oriterm_ui/src/testing/mock_measurer.rs` no longer exists (converted to directory module)
- No other file references the old `mock_measurer.rs` path

### Test Count Summary

| File | New tests | Updated tests |
|---|---|---|
| `oriterm_ui/src/text/tests.rs` | 10 | 2 (default, new) |
| `oriterm/src/font/shaper/cached_measurer/tests.rs` | 5 | 0 |
| `oriterm/src/font/shaper/tests.rs` | 11 | 0 |
| `oriterm_ui/src/testing/mock_measurer/tests.rs` | 7 | 0 |
| `oriterm_ui/src/widgets/label/tests.rs` | 1 | 0 |
| **Total** | **34** | **2** |

### Completion Criteria

- `TextStyle.line_height` exists and defaults to `None` (metric-driven behavior)
- `TextStyle::normalized_line_height()` filters invalid values to `None`
- `measure()` and `shape()` agree on logical height when line-height is set
- baseline shifts correctly under non-`1.0` scale factors (physical space math)
- invalid line-height overrides (`0.0`, negative, `NaN`, infinity) normalize back to natural metrics
- cache keys distinguish otherwise identical text/styles with different line-height values
- invalid line-height produces same cache key as `None` (no cache pollution)
- `MockMeasurer` honors line-height overrides via `normalized_line_height()`
- `MockMeasurer` negative half-leading is not clamped (tighter-than-natural line-height works)
- `LabelStyle` has `line_height: Option<f32>` and `LabelWidget::text_style()` forwards it
- `LabelWidget::layout()` returns overridden height when `line_height` is set
- empty text with `line_height` set returns the styled height (not 0.0)
- line-height and letter-spacing are orthogonal (height unaffected by spacing)
- existing tests in `text/tests.rs`, `cached_measurer/tests.rs`, and `label/tests.rs` still pass
- no source file exceeds 500 lines after changes
- no dead code introduced (clippy `dead_code = "deny"` passes)
- this section does not claim generic wrapping or blanket widget adoption beyond the shared
  capability it actually builds
