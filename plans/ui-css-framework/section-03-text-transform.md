---
section: "03"
title: "Text Transform + Letter Spacing"
status: complete
reviewed: true
third_party_review:
  status: resolved
  updated: 2026-03-24
goal: "TextTransform survives through measurement, shaping, and caching; letter spacing stays a single logical-pixel style value and is applied consistently for metrics, overflow, and rendering"
inspired_by:
  - "CSS text-transform (https://developer.mozilla.org/en-US/docs/Web/CSS/text-transform)"
  - "CSS letter-spacing (https://developer.mozilla.org/en-US/docs/Web/CSS/letter-spacing)"
depends_on: []
sections:
  - id: "03.1"
    title: "TextTransform API"
    status: complete
  - id: "03.2"
    title: "Pipeline + Cache Integration"
    status: complete
  - id: "03.3"
    title: "Letter Spacing Semantics + Mockup Adoption"
    status: complete
  - id: "03.4"
    title: "Tests"
    status: complete
  - id: "03.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "03.5"
    title: "Build & Verify"
    status: complete
---

# Section 03: Text Transform + Letter Spacing

## Problem

The current tree already contains part of the letter-spacing work, but the section draft did not
match that reality:

- [oriterm_ui/src/text/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/text/mod.rs) already
  has `TextStyle.letter_spacing` in logical pixels.
- [oriterm_ui/src/widgets/label/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/label/mod.rs)
  already forwards `LabelStyle.letter_spacing` into `TextStyle`.
- [oriterm/src/font/shaper/ui_measurer.rs](/home/eric/projects/ori_term/oriterm/src/font/shaper/ui_measurer.rs)
  currently applies spacing in both `measure()` and `shape()`, but it uses different counting
  bases: `measure()` uses `text.chars().count()` while `shape()` uses `shaped.glyphs.len()`.
- [oriterm/src/app/settings_overlay/form_builder/appearance.rs](/home/eric/projects/ori_term/oriterm/src/app/settings_overlay/form_builder/appearance.rs)
  and [oriterm_ui/src/widgets/sidebar_nav/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/sidebar_nav/mod.rs)
  still uppercase text manually with `.to_uppercase()`.

That leaves two real gaps:

1. There is still no style-level `TextTransform` capability.
2. Letter spacing is not owned by the shaping layer, so overflow/truncation and measurement do not
   share one source of truth.

## Corrected Scope

This section should build the shared text capability, not force every widget to implement string
mutation by hand.

- `TextTransform` belongs on `TextStyle`, with `LabelStyle` forwarding it where needed.
- Transform application belongs in the shared text pipeline before overflow and shaping.
- `letter_spacing` remains a single logical-pixel field on `TextStyle`.
- Mockup `em` values are converted to logical pixels at style-construction sites; this section does
  not add a second persisted `letter_spacing_em` field.

Button-specific typography adoption is not mandatory here. `ButtonStyle` does not currently expose
`text_transform` or `letter_spacing`, so full footer/button fidelity remains in Section 12 unless
that API is explicitly broadened there.

## Mockup Usage

| Element | CSS `text-transform` | CSS `letter-spacing` | Current repo state |
|---|---|---|---|
| Page title (`APPEARANCE`) | `uppercase` | `0.05em` | Title is already passed as uppercase literal; spacing is approximated with `0.9px` |
| Section title (`Theme`) | `uppercase` | `0.15em` | Currently uses manual `.to_uppercase()` plus `1.6px` spacing |
| Sidebar section header (`General`) | `uppercase` | `0.15em` | Currently uses manual `.to_uppercase()` plus `1.5px` spacing |
| Button label (`Save`) | `uppercase` | `0.06em` | Buttons are currently authored as uppercase string literals |
| Footer unsaved badge | `uppercase` | `0.06em` | Footer fidelity belongs in Section 12 |

---

## 03.1 TextTransform API

### Goal

Add a style-level text-transform field that consumers can opt into without mutating source strings.

### Files

- [oriterm_ui/src/text/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/text/mod.rs)
- [oriterm_ui/src/widgets/label/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/label/mod.rs)
- [oriterm_ui/src/text/tests.rs](/home/eric/projects/ori_term/oriterm_ui/src/text/tests.rs)

### Required API

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextTransform {
    #[default]
    None,
    Uppercase,
    Lowercase,
}

impl TextTransform {
    pub fn apply<'a>(self, text: &'a str) -> std::borrow::Cow<'a, str> {
        match self {
            Self::None => std::borrow::Cow::Borrowed(text),
            Self::Uppercase => std::borrow::Cow::Owned(text.to_uppercase()),
            Self::Lowercase => std::borrow::Cow::Owned(text.to_lowercase()),
        }
    }
}
```

Add the field to `TextStyle`:

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
}
```

Add a builder:

```rust
#[must_use]
pub fn with_text_transform(mut self, transform: TextTransform) -> Self {
    self.text_transform = transform;
    self
}
```

### Scope Guard

Do not require `Capitalize` in this section. The mockup does not use it, and a faithful CSS-style
capitalize implementation is locale- and boundary-sensitive. Adding a naive whitespace-based helper
would create a misleading API surface without a real consumer.

### LabelStyle Integration

`LabelStyle` is the only dedicated wrapper type in the current settings stack that already exposes
text-specific styling knobs. Add `text_transform: TextTransform` there and forward it when building
`TextStyle`.

### Checklist

- [x] Add `TextTransform::{None, Uppercase, Lowercase}` to `TextStyle`
- [x] Add `TextStyle::with_text_transform()`
- [x] Update `TextStyle::default()` and `TextStyle::new()`
- [x] Add `text_transform` to `LabelStyle`
- [x] Update `LabelStyle::from_theme()` and `Default`

---

## 03.2 Pipeline + Cache Integration

### Goal

Apply transforms once in the shared text pipeline so layout, painting, overflow, and caching all
agree on the same text.

### Files

- [oriterm/src/font/shaper/ui_text.rs](/home/eric/projects/ori_term/oriterm/src/font/shaper/ui_text.rs)
- [oriterm/src/font/shaper/ui_measurer.rs](/home/eric/projects/ori_term/oriterm/src/font/shaper/ui_measurer.rs)
- [oriterm/src/font/shaper/cached_measurer/mod.rs](/home/eric/projects/ori_term/oriterm/src/font/shaper/cached_measurer/mod.rs)
- [oriterm_ui/src/testing/mock_measurer.rs](/home/eric/projects/ori_term/oriterm_ui/src/testing/mock_measurer.rs)
- [oriterm/src/app/settings_overlay/form_builder/appearance.rs](/home/eric/projects/ori_term/oriterm/src/app/settings_overlay/form_builder/appearance.rs)
- [oriterm_ui/src/widgets/sidebar_nav/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/sidebar_nav/mod.rs)

### Correct Ordering

```text
caller text + TextStyle{text_transform, letter_spacing}
  -> CachedTextMeasurer key(original text + transform + spacing + other style)
  -> ui_text::apply transform
  -> overflow / ellipsis on transformed text
  -> shaping
  -> letter-spacing adjustment
  -> ShapedText / TextMetrics
```

The key correction is that transform must happen before overflow handling. Otherwise values like
`Uppercase` can change the text length (`ß -> SS`) after truncation decisions have already been
made.

### Shared Integration Strategy

The original draft applied transforms in each widget's `layout()` and `paint()` methods. That is
the wrong boundary for this repo:

- it duplicates logic across every text-bearing widget
- it makes cache behavior depend on whether a widget remembered to transform before calling the
  measurer
- it violates the overview's principle that widgets declare style and the framework realizes it

Instead:

1. Add a shared helper in `ui_text.rs` or `oriterm_ui::text` that applies `style.text_transform`.
2. Make `shape_text()` and `measure_text_styled()` use that helper.
3. Leave `UiFontMeasurer` responsible only for logical/physical unit conversion.
4. Update `MockMeasurer` to apply the same transform so widget tests remain representative.

### Cache Key Update

`CachedTextMeasurer` must include `text_transform` in `TextCacheKey`. The key already carries
`letter_spacing_hundredths`, `size_hundredths`, `weight`, `overflow`, `scale_hundredths`,
`max_width_hundredths`, `font_family`, and `text` — so only `text_transform` is new.

Keep the key based on the original caller text plus the style field rather than eagerly allocating
transformed text just to build cache keys.

```rust
struct TextCacheKey {
    text: String,
    font_family: Option<String>,
    size_hundredths: u32,
    weight: FontWeight,
    overflow: TextOverflow,
    text_transform: TextTransform, // ← new field
    max_width_hundredths: u32,
    scale_hundredths: u32,
    letter_spacing_hundredths: u32, // already present in current codebase
}
```

### Consumer Cleanup

After the shared pipeline exists, remove manual uppercase conversion from the current settings
consumers that actually need it:

- `section_title()` in `appearance.rs`
- sidebar section titles in `sidebar_nav/mod.rs`

Keep the `"// "` prefix handling separate; that is pseudo-element content, not a case transform.

### Checklist

- [x] Apply `text_transform` inside the shared text pipeline, not per widget
- [x] Update `CachedTextMeasurer::TextCacheKey` to include `text_transform`
- [x] Update `MockMeasurer` to honor `TextStyle.text_transform`
- [x] Remove manual `.to_uppercase()` calls from settings section-title/sidebar-title code
- [x] Verify transformed style output matches explicitly transformed text

---

## 03.3 Letter Spacing Semantics + Mockup Adoption

### Goal

Keep one spacing field (`TextStyle.letter_spacing` in logical pixels), and make shaping,
measurement, and overflow use the same spacing-aware result.

### Files

- [oriterm/src/font/shaper/ui_text.rs](/home/eric/projects/ori_term/oriterm/src/font/shaper/ui_text.rs)
- [oriterm/src/font/shaper/ui_measurer.rs](/home/eric/projects/ori_term/oriterm/src/font/shaper/ui_measurer.rs)
- [oriterm/src/font/shaper/cached_measurer/mod.rs](/home/eric/projects/ori_term/oriterm/src/font/shaper/cached_measurer/mod.rs)
- [oriterm_ui/src/text/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/text/mod.rs)
- [oriterm/src/app/settings_overlay/form_builder/appearance.rs](/home/eric/projects/ori_term/oriterm/src/app/settings_overlay/form_builder/appearance.rs)

### Corrected Design

Do not add `letter_spacing_em` as a persisted field on `TextStyle`. The repo already stores text
metrics in logical pixels for `size`, padding, border widths, and spacing. A second stored unit
would create unnecessary API and cache complexity for no real gain.

Instead:

1. Keep `TextStyle.letter_spacing` as the only stored spacing field.
2. Convert CSS mockup values to logical pixels at style-construction sites.
3. Move spacing application out of `UiFontMeasurer` and into the shared shaping path so
   `measure_text_styled()` and `shape_text()` use the same glyph basis.

### Why the Current Implementation Is Wrong

The worktree's current letter-spacing patch fixed scale conversion but not ownership:

- `measure()` adds spacing using `text.chars().count()`
- `shape()` adds spacing using `shaped.glyphs.len()`
- `truncate_with_ellipsis()` still reasons about width without any style-aware spacing budget

That means combining marks, ligatures, or any transform that changes character count can still
produce disagreement between measured width and shaped width.

### Required Integration

Use one shared helper after shaping:

```rust
fn apply_letter_spacing(shaped: &mut ShapedText, logical_spacing: f32, scale: f32) {
    if logical_spacing <= 0.0 || shaped.glyphs.is_empty() {
        return;
    }

    let phys_spacing = logical_spacing * scale;
    for glyph in &mut shaped.glyphs {
        glyph.x_advance += phys_spacing;
    }
    shaped.width += phys_spacing * shaped.glyphs.len() as f32;
}
```

Then:

- `shape_text()` (or a shared internal helper it owns) applies spacing once
- `measure_text_styled()` derives width from the same shaped result
- `UiFontMeasurer` only converts width/height units

### Overflow Interaction

If `TextOverflow::Ellipsis` stays supported together with letter spacing, truncation must be
spacing-aware. The simplest correct option is to replace the current style-blind width budgeting
with a style-aware fitting helper that measures candidate strings using the same transform and
spacing rules the final shape path uses.

### Mockup Values

Convert the mockup's `em` values to logical pixels where the styles are authored:

| Element | CSS | Logical px to store in `letter_spacing` |
|---|---|---|
| Page title, 18px @ `0.05em` | `0.05 * 18` | `0.9` |
| Section title, 11px @ `0.15em` | `0.15 * 11` | `1.65` (round as needed, current tree uses `1.6`) |
| Sidebar title, 10px @ `0.15em` | `0.15 * 10` | `1.5` |
| Footer/button labels, 12px @ `0.06em` | `0.06 * 12` | `0.72` |

This section only needs to preserve or clean up the existing page/section/sidebar title constants.
Button/footer adoption is deferred unless Section 12 expands `ButtonStyle`.

### Checklist

- [x] Remove letter-spacing arithmetic from `UiFontMeasurer`
- [x] Apply spacing in one shared shaping/measurement path
- [x] Make ellipsis truncation respect spacing when spacing is non-zero
- [x] Keep `TextStyle.letter_spacing` as the only persisted spacing field
- [x] Keep current mockup-converted pixel constants for title/sidebar styles

---

## 03.4 Tests

### Unit Tests

Files:

- [oriterm_ui/src/text/tests.rs](/home/eric/projects/ori_term/oriterm_ui/src/text/tests.rs)
- [oriterm/src/font/shaper/cached_measurer/tests.rs](/home/eric/projects/ori_term/oriterm/src/font/shaper/cached_measurer/tests.rs)

Add tests for:

```rust
#[test]
fn text_transform_default_is_none() { ... }

#[test]
fn text_transform_none_borrows() {
    // Verify TextTransform::None returns Cow::Borrowed (zero allocation)
}

#[test]
fn text_transform_uppercase_matches_explicit_text() { ... }

#[test]
fn text_transform_lowercase_matches_explicit_text() { ... }

#[test]
fn text_transform_uppercase_multibyte_expansion() {
    // "straße" -> "STRASSE" (German sharp s expands from 1 to 2 chars)
    // Verifies that transform handles string length changes correctly
}

#[test]
fn text_transform_empty_string() {
    // Empty string stays empty for all transform variants
}

#[test]
fn cache_key_changes_when_text_transform_changes() { ... }

#[test]
fn cache_key_same_when_text_transform_same() {
    // Two cache keys with identical text and TextTransform::Uppercase are equal
}
```

### Integration Tests

File:

- [oriterm/src/font/shaper/tests.rs](/home/eric/projects/ori_term/oriterm/src/font/shaper/tests.rs)

Add focused shaper tests for:

1. `measure()` and `shape()` return the same width for transformed text.
2. `measure()` and `shape()` return the same width for non-zero letter spacing.
3. A combining-mark or multi-codepoint sample still satisfies width equality with spacing enabled.
4. Transform-before-overflow: `TextTransform::Uppercase` with ellipsis produces the same visible
   result as explicitly uppercasing the source before shaping.

### Widget-Level Test Coverage

File:

- [oriterm_ui/src/widgets/label/tests.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/label/tests.rs)

Add a forwarding test that `LabelStyle.text_transform` is propagated into the `TextStyle` built by
`LabelWidget`.

### Checklist

- [x] `TextTransform` unit tests exist in `oriterm_ui`
- [x] Cache-key coverage exists for `text_transform`
- [x] `UiFontMeasurer` width consistency is tested with letter spacing
- [x] At least one transformed + ellipsized case is covered
- [x] `LabelStyle` forwarding is covered

---

## 03.R Third Party Review Findings

- [x] `[TPR-03-007][medium]` `oriterm/src/font/shaper/ui_text.rs:133` — ellipsis fitting still budgets letter spacing per Unicode scalar while the final shaped width adds spacing per glyph.
  Resolved 2026-03-24: accepted and fixed. Two changes: (1) `shape_text()` now shapes the full text first and checks actual width (including per-glyph letter spacing) before deciding to truncate — this eliminates false truncation from ligatures/combining marks. (2) `truncate_with_ellipsis()` now counts only visible characters (nonzero unicode width) for the letter-spacing budget, matching the shaping output where zero-width marks don't produce glyphs. Added regression tests: `truncate_combining_marks_spacing_not_inflated` and `shape_text_ellipsis_shapes_first_to_avoid_false_truncation`.

- [x] `[TPR-03-008][medium]` `oriterm_ui/src/testing/mock_measurer.rs:36` — `MockMeasurer` still measures transformed text by UTF-8 byte length instead of character/glyph count.
  Resolved 2026-03-24: accepted and fixed. Changed both `measure()` and `shape()` in `MockMeasurer` to use `transformed.chars().count()` instead of `transformed.len()`. Added regression tests: `mock_measurer_non_ascii_width_matches_glyph_count` and `mock_measurer_transform_multibyte_consistent`.

- [x] `[TPR-03-006][medium]` `oriterm/src/font/shaper/ui_text.rs:129` — the ellipsis path is still style-blind for letter spacing, so a spaced UI label can be truncated against the wrong width budget and then widened afterward.
  Resolved 2026-03-24: accepted and fixed. Added `phys_letter_spacing` parameter to `shape_text()` and `truncate_with_ellipsis()`. Letter spacing is now applied inside `shape_text()` (to glyph advances) and accounted for in `truncate_with_ellipsis()` (per-character and ellipsis width budgets). Removed post-hoc spacing from `UiFontMeasurer::shape()` — all spacing logic is centralized in `shape_text`. Added 3 regression tests: `truncate_with_ellipsis_respects_letter_spacing`, `truncate_with_ellipsis_spacing_short_text_unchanged`, `shape_text_ellipsis_with_spacing_stays_within_budget`.

- [x] `[TPR-03-001][high]` [plans/ui-css-framework/section-03-text-transform.md](/home/eric/projects/ori_term/plans/ui-css-framework/section-03-text-transform.md) - The original plan applied text transforms in each widget's `layout()` and `paint()` methods, which would have duplicated logic across every text-bearing widget and made cache behavior depend on caller discipline. Resolved: the section now requires transform handling in the shared text pipeline and cache model on 2026-03-23.

- [x] `[TPR-03-002][medium]` [plans/ui-css-framework/section-03-text-transform.md](/home/eric/projects/ori_term/plans/ui-css-framework/section-03-text-transform.md) - The original `letter_spacing_em` field duplicated the repo's existing logical-pixel `letter_spacing` field and would have forced unnecessary cache-key and measurer churn. Resolved: the section now keeps one stored pixel spacing value and converts mockup `em` values at style-construction sites on 2026-03-23.

- [x] `[TPR-03-003][medium]` [plans/ui-css-framework/section-03-text-transform.md](/home/eric/projects/ori_term/plans/ui-css-framework/section-03-text-transform.md) - The original section promised button adoption even though [oriterm_ui/src/widgets/button/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/button/mod.rs) does not currently expose transform or spacing fields. Resolved: the section now limits mandatory adoption to the shared text pipeline plus current label/sidebar consumers and defers full button/footer typography to Section 12 unless that API is widened there on 2026-03-23.

- [x] `[TPR-03-004][medium]` [plans/ui-css-framework/section-03-text-transform.md](/home/eric/projects/ori_term/plans/ui-css-framework/section-03-text-transform.md) - The original `Capitalize` helper used naive whitespace semantics despite CSS capitalize behavior being locale- and boundary-sensitive, and the mockup does not need it. Resolved: the required scope now covers only `None`, `Uppercase`, and `Lowercase`, with `Capitalize` deferred until a real consumer justifies the added complexity on 2026-03-23.

- [x] `[TPR-03-005][medium]` [plans/ui-css-framework/section-03-text-transform.md](/home/eric/projects/ori_term/plans/ui-css-framework/section-03-text-transform.md) - The original draft did not account for transform and letter-spacing interactions with overflow handling, so truncation could still be computed against the wrong string or width budget. Resolved: the section now requires transform-before-overflow ordering and spacing-aware truncation on 2026-03-23.

---

## 03.5 Build & Verify

### Gate

```bash
./build-all.sh
./clippy-all.sh
./test-all.sh
```

### Focused Verification

1. `cargo test -p oriterm_ui text`
2. `cargo test -p oriterm_ui label`
3. `cargo test -p oriterm font::shaper`
4. `cargo test -p oriterm cached_measurer`
5. Manual verification of the settings sidebar and section headers at the active scale factor

### Completion Criteria

- `TextStyle.text_transform` is honored by measurement, shaping, and caching
- manual uppercase calls are removed from the existing settings title/sidebar consumers
- `measure()` and `shape()` agree on width when letter spacing is enabled
- non-zero letter spacing does not silently bypass ellipsis width calculations
- this section does not claim footer/button typography fidelity beyond the widgets whose APIs
  actually expose the required style knobs
