---
section: "04"
title: "Line Height Control"
status: not-started
reviewed: true
third_party_review:
  status: resolved
  updated: 2026-03-23
goal: "TextStyle.line_height overrides logical UI text block height while preserving the current logical-height / physical-baseline render contract; cache keys and test measurers honor it; generic wrapped-text behavior remains out of scope until real wrapping exists"
inspired_by:
  - "CSS line-height (https://developer.mozilla.org/en-US/docs/Web/CSS/line-height)"
  - "GPUI line-height concepts (~/projects/reference_repos/gui_repos/zed/crates/gpui/src/text_system.rs)"
depends_on: ["01"]
sections:
  - id: "04.1"
    title: "TextStyle Line Height API"
    status: not-started
  - id: "04.2"
    title: "Measurer, Baseline, and Cache Integration"
    status: not-started
  - id: "04.3"
    title: "Consumer Boundaries"
    status: not-started
  - id: "04.4"
    title: "Tests"
    status: not-started
  - id: "04.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "04.5"
    title: "Build & Verify"
    status: not-started
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

### Checklist

- [ ] Add `line_height: Option<f32>` to `TextStyle`
- [ ] Add `TextStyle::with_line_height()`
- [ ] Update `TextStyle::default()` and `TextStyle::new()`
- [ ] Add a small normalization helper for finite positive line-height overrides
- [ ] Add `line_height: Option<f32>` to `LabelStyle`
- [ ] Update `LabelStyle::from_theme()` and `Default`

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

Use a single normalization helper before applying the override:

```rust
fn normalized_line_height(style: &TextStyle) -> Option<f32> {
    style
        .line_height
        .filter(|multiplier| multiplier.is_finite() && *multiplier > 0.0)
}
```

### Measure Path

For `measure()`, the rule is straightforward for the current single-line behavior:

```rust
let phys = ui_text::measure_text_styled(text, style, self.collection);
let height = match normalized_line_height(style) {
    Some(multiplier) => style.size * multiplier,
    None => phys.height / self.scale,
};
```

### Shape Path

For `shape()`, do the baseline math in physical space before finalizing the logical height:

```rust
let mut shaped = ui_text::shape_text(text, style, max_width * self.scale, self.collection);

if let Some(multiplier) = normalized_line_height(style) {
    let target_logical_height = style.size * multiplier;
    let target_physical_height = target_logical_height * self.scale;
    let half_leading_physical = (target_physical_height - shaped.height) / 2.0;
    shaped.baseline += half_leading_physical;
    shaped.height = target_logical_height;
} else {
    shaped.height /= self.scale;
}

shaped.width /= self.scale;
```

This is the critical correction from the original draft: do not add a logical-pixel half-leading
value directly to `shaped.baseline`, because baseline is still consumed in physical space by
[scene_convert/text.rs](/home/eric/projects/ori_term/oriterm/src/gpu/scene_convert/text.rs).

### Cache Key Update

`CachedTextMeasurer` must include line-height in `TextCacheKey` because it changes both measured
height and shaped baseline.

```rust
struct TextCacheKey {
    text: String,
    font_family: Option<String>,
    size_hundredths: u32,
    weight: FontWeight,
    overflow: TextOverflow,
    max_width_hundredths: u32,
    scale_hundredths: u32,
    letter_spacing_hundredths: u32,
    line_height_hundredths: Option<u32>,
}
```

That field should store the normalized finite-positive override, not the raw `TextStyle` value, so
invalid overrides collapse to the same cache key as `None`.

### MockMeasurer Update

`MockMeasurer` should honor the field too, otherwise widget tests cannot exercise the new style
knob. It does not need to emulate font internals perfectly, but it should at least:

- return overridden logical height from `measure()`
- shift shaped baseline by half-leading relative to its existing synthetic baseline model

### Out of Scope Here

Do not move line-height logic into `ui_text.rs`, and do not change `scene_convert` to reinterpret
baseline units. That would broaden this section into a rendering-contract refactor.

### Checklist

- [ ] Apply `line_height` override in `UiFontMeasurer::measure()`
- [ ] Apply `line_height` override in `UiFontMeasurer::shape()` with baseline math in physical space
- [ ] Normalize invalid overrides before measurement, shaping, and cache-key construction
- [ ] Add `line_height_hundredths` to `TextCacheKey`
- [ ] Update `TextCacheKey::new()` and `TextCacheKey::for_measure()`
- [ ] Update `MockMeasurer` to honor `line_height`
- [ ] Leave `scene_convert` unchanged; it should keep consuming physical baseline values

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

- [ ] Do not claim generic wrapped-text support in this section
- [ ] Do not claim the existing terminal `config.font.line_height` setting exercises this feature
- [ ] Limit this section to shared capability plus wrapper-style forwarding
- [ ] Let later visual-fidelity sections choose which widgets adopt `TextStyle.line_height`

---

## 04.4 Tests

### Unit Tests

Files:

- [oriterm_ui/src/text/tests.rs](/home/eric/projects/ori_term/oriterm_ui/src/text/tests.rs)
- [oriterm/src/font/shaper/cached_measurer/tests.rs](/home/eric/projects/ori_term/oriterm/src/font/shaper/cached_measurer/tests.rs)

Add tests for:

```rust
#[test]
fn text_style_line_height_default_is_none() { ... }

#[test]
fn text_style_with_line_height_sets_override() { ... }

#[test]
fn cache_key_changes_when_line_height_changes() { ... }
```

### Shaper/Measurer Integration Tests

File:

- [oriterm/src/font/shaper/tests.rs](/home/eric/projects/ori_term/oriterm/src/font/shaper/tests.rs)

Keep the focused `UiFontMeasurer` assertions in the existing shaper test module unless
`ui_measurer.rs` is deliberately converted into a directory module with a sibling `tests.rs`.
That avoids a file-organization refactor just to add coverage.

Add focused tests for:

1. `measure()` returns `style.size * multiplier` when line-height is set.
2. `shape()` returns the same logical height as `measure()`.
3. width is unchanged by line-height overrides.
4. baseline moves downward for larger line-height and upward for tighter line-height.
5. the behavior is correct at a scale factor other than `1.0`, so the physical/logical baseline
   conversion is covered.
6. invalid overrides (`0.0`, negative, `NaN`, infinities) fall back to natural metrics.

### MockMeasurer Coverage

Add a widget-level or testing helper test that confirms `MockMeasurer` respects the override. That
prevents the UI test suite from silently diverging from production behavior.

### Checklist

- [ ] `TextStyle` default and builder tests exist
- [ ] `CachedTextMeasurer` key coverage exists for line-height
- [ ] `UiFontMeasurer` height/baseline behavior is covered at scale `1.0`
- [ ] `UiFontMeasurer` height/baseline behavior is covered at a non-`1.0` scale
- [ ] Invalid override normalization is covered in production and mock measurers
- [ ] `MockMeasurer` line-height behavior is covered

---

## 04.R Third Party Review Findings

- [x] `[TPR-04-001][high]` [plans/ui-css-framework/section-04-line-height.md](/home/eric/projects/ori_term/plans/ui-css-framework/section-04-line-height.md) - The original draft added logical half-leading directly to `ShapedText.baseline` inside `UiFontMeasurer::shape()`, but the current renderer still consumes `baseline` in physical pixels. That would misplace text whenever the scale factor is not `1.0`. Resolved: the section now requires baseline math in physical space and keeps the current renderer contract unchanged on 2026-03-23.

- [x] `[TPR-04-002][medium]` [plans/ui-css-framework/section-04-line-height.md](/home/eric/projects/ori_term/plans/ui-css-framework/section-04-line-height.md) - The original section promised generic multiline behavior even though `TextOverflow::Wrap` is still effectively a placeholder in [ui_text.rs](/home/eric/projects/ori_term/oriterm/src/font/shaper/ui_text.rs) and `measure_text_styled()` always reports `line_count: 1`. Resolved: the section now scopes itself to single-line shaped text plus manual multiline consumers, and defers true wrapped-text semantics until wrapping exists on 2026-03-23.

- [x] `[TPR-04-003][medium]` [plans/ui-css-framework/section-04-line-height.md](/home/eric/projects/ori_term/plans/ui-css-framework/section-04-line-height.md) - The original mockup usage table incorrectly treated settings-row layout height as a line-height feature, when the mockup actually gets most of that vertical rhythm from row `min-height`, padding, and gaps. Resolved: the section now lists the real line-height-driven mockup elements such as descriptions, badges, previews, callouts, and keybinding hints on 2026-03-23.

- [x] `[TPR-04-004][medium]` [plans/ui-css-framework/section-04-line-height.md](/home/eric/projects/ori_term/plans/ui-css-framework/section-04-line-height.md) - The original draft blurred UI text line-height with the existing terminal-grid `config.font.line_height` setting exposed on the Font page. Resolved: the section now explicitly scopes itself to `TextStyle`/`oriterm_ui` behavior and treats terminal grid line height as a separate subsystem on 2026-03-23.

- [x] `[TPR-04-005][medium]` [plans/ui-css-framework/section-04-line-height.md](/home/eric/projects/ori_term/plans/ui-css-framework/section-04-line-height.md) - The original draft suggested optional integration in `ui_text.rs`, but that layer does not own the logical `style.size * multiplier` calculation and is the wrong boundary for the current baseline contract. Resolved: the section now requires the override in `UiFontMeasurer` and cache/test helpers instead of broadening the shaping layer on 2026-03-23.

- [x] `[TPR-04-006][medium]` [plans/ui-css-framework/section-04-line-height.md](/home/eric/projects/ori_term/plans/ui-css-framework/section-04-line-height.md) - The corrected draft still left `Some(0.0)`, negative values, and non-finite `line_height` inputs unspecified even though `TextStyle` is a public struct. Without explicit normalization, layout and cache keys could observe zero/negative/NaN heights. Resolved: the section now requires finite-positive normalization at the measurer/cache boundary and adds test coverage for invalid overrides on 2026-03-23.

---

## 04.5 Build & Verify

### Gate

```bash
./build-all.sh
./clippy-all.sh
./test-all.sh
```

### Focused Verification

1. `cargo test -p oriterm_ui text`
2. `cargo test -p oriterm_ui label`
3. `cargo test -p oriterm_ui testing`
4. `cargo test -p oriterm font::shaper`
5. `cargo test -p oriterm cached_measurer`
6. Manual verification at a scale factor other than `1.0` if available

### Completion Criteria

- `TextStyle.line_height` exists and defaults to metric-driven behavior
- `measure()` and `shape()` agree on logical height when line-height is set
- baseline shifts correctly under non-`1.0` scale factors
- invalid line-height overrides normalize back to natural metrics instead of producing impossible
  layout values
- cache keys distinguish otherwise identical text/styles with different line-height values
- this section does not claim generic wrapping or blanket widget adoption beyond the shared
  capability it actually builds
