---
section: "11"
title: "Visual Fidelity: Content Area + Typography"
status: complete
reviewed: true
third_party_review:
  status: resolved
  updated: 2026-03-26
goal: "The settings content area matches the mockup's typography and rhythm across all pages: page headers, section headers, optional section descriptions, setting-row labels, inline status tags, and the shared body spacing/padding all render from real shared primitives instead of ad hoc per-page constants"
depends_on: ["01", "02", "03", "04", "05", "06"]
sections:
  - id: "11.1"
    title: "Shared Content Typography Boundary"
    status: complete
  - id: "11.2"
    title: "Page Headers"
    status: complete
  - id: "11.3"
    title: "Section Headers + Descriptions"
    status: complete
  - id: "11.4"
    title: "Setting Rows + Status Tags"
    status: complete
  - id: "11.5"
    title: "Body Spacing + Section Rhythm"
    status: complete
  - id: "11.6"
    title: "Tests"
    status: complete
  - id: "11.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "11.7"
    title: "Build & Verify"
    status: complete
---

# Section 11: Visual Fidelity - Content Area + Typography

## Problem

The draft treated Section 11 as mostly a verification pass, but the current implementation still has
real gaps in the shared content typography system.

What the tree actually shows today:

- `build_settings_page(...)`, `build_page_header(...)`, and `section_title(...)` live in
  `appearance.rs`, even though they are the shared content-layout path for all settings pages.
- The page header values are close to the mockup, but `build_page_header` relies on callers to pass
  already-uppercased titles (all 8 callers pass e.g. `"APPEARANCE"`) instead of applying
  `TextTransform::Uppercase` itself.
- Section-header spacing is wrong in multiple places:
  - the mockup uses `margin-bottom: 12px`
  - `appearance.rs` uses `TITLE_ROW_GAP = 8.0` for one section
  - most other sections use `ROW_GAP = 2.0` directly below the title
- The current section-title implementation renders `"// TITLE"` as one label with letter spacing
  applied to the entire string. The mockup resets the `//` prefix to normal spacing and applies
  the wider tracking only to the title text.
- `FontWeight::MEDIUM` (500) is available from Section 02, but `section_title` still uses the
  `LabelStyle::from_theme` default (`FontWeight::NORMAL` / 400).
- The mockup includes `.section-desc` blocks on several pages (`Font`, `Bell`, `Rendering`), but
  the current shared builder path has no section-description primitive at all.
- `SettingRowWidget` gets the basic label typography mostly right, but it cannot express inline
  status tags such as `Restart` or `Advanced`.
- Shared content spacing is still off:
  - content body bottom padding is currently `0`, not `28`
  - inter-section spacing is `24`, not `28`
  - intra-section row spacing is implemented as a global `2px` gap, even though the mockup rows
    are stacked directly and rely on their own internal height/padding

Section 11 therefore needs to be a shared content-typography rewrite, not a checklist that says
"already matches" while the actual primitives are still missing.

## Corrected Scope

1. Move shared content/header helpers out of `appearance.rs`
2. Make page and section headings semantic shared primitives instead of ad hoc labels
3. Add optional section descriptions and inline setting tags
4. Fix the shared content-body spacing constants so every page inherits the right rhythm

This section should not invent page-specific typography one file at a time. The mockup is highly
consistent across the settings pages, so the solution should be shared.

---

## 11.1 Shared Content Typography Boundary

### Goal

Move the shared content-area typography/layout helpers from `appearance.rs` into a dedicated shared
module so they have proper ownership.

### Files

- `oriterm/src/app/settings_overlay/form_builder/appearance.rs`
- `oriterm/src/app/settings_overlay/form_builder/*.rs` (all 7 other page builders)
- new: `oriterm/src/app/settings_overlay/form_builder/shared/mod.rs`
- new: `oriterm/src/app/settings_overlay/form_builder/shared/tests.rs`

### Current Boundary Problem

`appearance.rs` currently owns these shared primitives:

**Exported `pub(super)`:** `PAGE_PADDING`, `SECTION_GAP`, `ROW_GAP`, `TITLE_FONT_SIZE`,
`DESC_FONT_SIZE`, `SECTION_LETTER_SPACING`, `build_settings_page(...)`, `section_title(...)`

**Private:** `TITLE_ROW_GAP`, `SECTION_FONT_SIZE`, `TITLE_LETTER_SPACING`, `build_page_header(...)`

All 7 other page builders import from `super::appearance::` via
`use super::appearance::{ROW_GAP, build_settings_page, section_title}`.

### Required Restructure

Create `form_builder/shared/mod.rs` and move all shared items there.

**Visibility note:** `pub(super)` from `shared/mod.rs` makes items visible to `form_builder` and all
its submodules (`appearance`, `window`, etc.). This is the correct scope — no need for the broader
`pub(in crate::app)`.

### Checklist

- [x] Create `form_builder/shared/mod.rs` with `#[cfg(test)] mod tests;` at the bottom
- [x] Create `form_builder/shared/tests.rs` (empty initially)
- [x] Add `mod shared;` to `form_builder/mod.rs`
- [x] Move functions from `appearance.rs` to `shared/mod.rs`: `build_settings_page`, `build_page_header`, `section_title`
- [x] Move constants from `appearance.rs` to `shared/mod.rs`: `PAGE_PADDING`, `SECTION_GAP`, `ROW_GAP`, `TITLE_FONT_SIZE`, `DESC_FONT_SIZE`, `SECTION_LETTER_SPACING`, `TITLE_ROW_GAP`, `SECTION_FONT_SIZE`, `TITLE_LETTER_SPACING`
- [x] Set visibility to `pub(super)` on all moved items in `shared/mod.rs`
- [x] Move the `oriterm_ui` imports that these helpers need from `appearance.rs` to `shared/mod.rs`
- [x] Update all 7 page builders to import from `super::shared::` instead of `super::appearance::`
- [x] Update `appearance.rs` to import from `super::shared::` for its own section builders
- [x] Verify `appearance.rs` no longer has any `pub(super)` items (only page-specific code remains)
- [x] Add `//!` module doc comment at top of `shared/mod.rs`
- [x] Verify all page builders compile after the import path change

---

## 11.2 Page Headers

### Goal

Make the page-header helper own the uppercase transform semantically, rather than relying on callers
to pre-uppercase their titles.

### Files

- `form_builder/shared/mod.rs` (from 11.1)
- all 8 page builder files

### Mockup Facts

- title `18px`, weight `700`, uppercase, letter spacing `0.9px` (`0.05em * 18px`)
- subtitle `12px`, muted color
- `4px` title-to-subtitle gap
- `20px` gap after subtitle before first section
- side padding `28px`, top padding `24px`

### Current State

The current `build_page_header` matches the mockup's raw values closely:

- `TITLE_FONT_SIZE = 18.0`, `FontWeight::BOLD` (700), `TITLE_LETTER_SPACING = 0.9`
- `DESC_FONT_SIZE = 12.0`, `theme.fg_secondary`
- padding `Insets::tlbr(24.0, 28.0, 20.0, 28.0)`, gap `4.0`

Two things are wrong:

1. `build_page_header` does not apply `TextTransform::Uppercase` — callers pass pre-uppercased
   strings like `"APPEARANCE"`.
2. The helper lives in the wrong module (fixed by 11.1).

### Checklist

- [x] Apply `TextTransform::Uppercase` inside the helper's `LabelStyle`
- [x] Update all 8 callers to pass mixed-case titles:
  - `appearance.rs`: `"APPEARANCE"` -> `"Appearance"`
  - `colors.rs`: `"COLORS"` -> `"Colors"`
  - `font.rs`: `"FONT"` -> `"Font"`
  - `terminal.rs`: `"TERMINAL"` -> `"Terminal"`
  - `keybindings.rs`: `"KEYBINDINGS"` -> `"Keybindings"`
  - `window.rs`: `"WINDOW"` -> `"Window"`
  - `bell.rs`: `"BELL"` -> `"Bell"`
  - `rendering.rs`: `"RENDERING"` -> `"Rendering"`
- [x] Keep page title at `18px`, weight `700`, `0.9px` tracking, bright color
- [x] Keep subtitle at `12px`, muted color
- [x] Preserve `24 / 28 / 20` effective page-header spacing
- [x] Verify `dialog_builds_without_panic` test still passes

---

## 11.3 Section Headers + Descriptions

### Goal

Replace `section_title(...)` with richer shared helpers that match the mockup exactly, including
optional section descriptions.

### Files

- `form_builder/shared/mod.rs`
- all 8 page builder files (19 call sites total)

### Current Gaps

The mockup section heading system:

- title text `11px`, weight `500`, uppercase, `0.15em` tracking (`1.65px`)
- `//` prefix has `0` tracking
- title row has `12px` bottom spacing
- some sections include `.section-desc`: `12px`, muted, `line-height: 1.5`, effective `4px`
  gap between title and description, `12px` gap after description before first row

The current `section_title(text, theme)` helper does not match this:

- One combined `LabelWidget` for `"// {text}"` — a single label with the `//` and title
  concatenated, so the `//` receives the same letter spacing as the title
- Letter spacing `SECTION_LETTER_SPACING = 1.6px` (mockup: `1.65px`)
- Weight `FontWeight::NORMAL` (mockup: `FontWeight::MEDIUM`)
- No section-description support
- Bottom spacing inconsistent: `build_theme_section` uses `TITLE_ROW_GAP = 8.0`, all other
  sections use `ROW_GAP = 2.0` — neither matches mockup's `12px`

### Required Structure

Replace the current single-label row:
```
ContainerWidget::row() [gap=10, align=Center, width=Fill]
  +-- LabelWidget("// TITLE")     <- all same letter-spacing
  +-- SeparatorWidget::horizontal()
```

With separate labels:
```
ContainerWidget::row() [gap=10, align=Center, width=Fill]
  +-- LabelWidget("//")            <- letter_spacing=0.0, weight=500, color=fg_faint
  +-- LabelWidget("TITLE")         <- letter_spacing=1.65, weight=500, uppercase, color=fg_faint
  +-- SeparatorWidget::horizontal() <- thickness=2, color=theme.border (unchanged)
```

### Section Description Support

Add `build_section_header_with_description(title, desc, theme)` for sections that need it.

Without description:
```
ContainerWidget::column() [gap=0]
  +-- title row (from above)
  +-- SpacerWidget::fixed(0.0, 12.0)    <- title-to-rows gap
```

With description:
```
ContainerWidget::column() [gap=0]
  +-- title row
  +-- SpacerWidget::fixed(0.0, 4.0)     <- title-to-desc gap
  +-- LabelWidget(desc)                 <- 12px font, muted, line_height=1.5
  +-- SpacerWidget::fixed(0.0, 12.0)    <- desc-to-rows gap
```

Both helpers return `Box<dyn Widget>` (same as current `section_title`).

### Transition Strategy

The rename from `section_title(...)` to `build_section_header(...)` affects 19 call sites across
8 files. To avoid a broken intermediate state:

1. Create `build_section_header` and `build_section_header_with_description` in `shared/mod.rs`.
2. Keep `section_title` as a thin wrapper delegating to `build_section_header`.
3. Update all 19 call sites in a single pass.
4. Remove the old `section_title` function.

Each section builder also needs to change from `.with_gap(ROW_GAP)` to `.with_gap(0.0)` — this
must happen simultaneously with switching to the new helpers (which bake in the `12px` spacer).
Otherwise the gap stacks incorrectly.

### Checklist

- [x] Split `"// TITLE"` into separate prefix and title labels with distinct letter spacing
- [x] Apply `FontWeight::MEDIUM` (500) to both prefix and title
- [x] Apply `letter_spacing: 0.0` to prefix, `1.65` to title (update `SECTION_LETTER_SPACING` from `1.6`)
- [x] Preserve `TextTransform::Uppercase` on the title label
- [x] Preserve `SeparatorWidget::horizontal()` with `thickness: 2.0` and `theme.border`
- [x] Add `build_section_header(title, theme)` returning column with title row + 12px spacer
- [x] Add `build_section_header_with_description(title, desc, theme)` with 4px title-desc gap, description label (`12px`, muted, `line_height: Some(1.5)`), and 12px spacer
- [x] Both helpers return `Box<dyn Widget>` and accept `&str` (not `&'static str` — `colors.rs` passes a dynamic `format!` string)
- [x] Update all 19 call sites across 8 files to use `build_section_header`
- [x] Remove old `section_title` function after migration

---

## 11.4 Setting Rows + Status Tags

### Goal

Extend `SettingRowWidget` to support inline status tags on the name line.

### Files

- `oriterm_ui/src/widgets/setting_row/mod.rs`
- `oriterm_ui/src/widgets/setting_row/tests.rs`

### What Already Matches

`SettingRowWidget` correctly implements: row min height `44`, padding `10px 14px`, label/control
gap `24px`, name size `13px`, description size `11.5px`, name/description gap `2px`, hover
background `theme.bg_card`. These values stay.

### Missing: Inline Status Tags

The mockup `.setting-label .name` row is `display: flex; gap: 6px` with inline tags after the
name text. Tag styling (from `.tag`):

- font size `9px`, weight `700`, uppercase, letter spacing `0.54px` (`0.06em * 9px`)
- padding `2px 5px`, `1px` border using `currentColor`, `line-height: 1.3`
- Variant colors:
  - `New`: text `accent`, bg `accent_bg_strong`
  - `Restart`: text `warning`, bg `warning_bg`
  - `Advanced`: text `fg_secondary`, bg `bg_secondary`
  - `Experimental`: text `danger`, bg `danger_bg`

### Implementation

Add `SettingTag` and `SettingTagKind` in `setting_row/mod.rs` (small types, tightly coupled):

```rust
enum SettingTagKind { New, Restart, Advanced, Experimental }

struct SettingTag { kind: SettingTagKind, text: String }
```

`SettingTagKind::colors(&self, theme) -> (Color, Color)` maps variant to `(text_color, bg_color)`.

Add `tags: Vec<SettingTag>` field to `SettingRowWidget`, with `.with_tag(tag)` builder method.

Layout: when tags are present, the name leaf becomes a nested row with `gap: 6`. Each tag leaf
measures as `text_width + 12px` wide (5px padding each side + 1px border each side) and
`text_height + 6px` tall (2px padding each side + 1px border each side).

Paint: draw each tag chip via `scene.push_quad` with `RectStyle::filled(bg).with_border(1.0, text_color)`,
then render uppercase text at `9px`/`700`/`0.54px`/`line_height: 1.3` on top.

### Crate Boundary Note

Section 11.4 modifies `oriterm_ui` (the UI framework crate). Sections 11.1-11.3 and 11.5 modify
`oriterm` (the app crate). These are independent and can be implemented in any order. However,
form builders wanting to USE `.with_tag(...)` can only do so after 11.4 lands.

### Checklist

- [x] Add `SettingTag` struct and `SettingTagKind` enum in `setting_row/mod.rs`, exported as `pub`
- [x] Add `tags: Vec<SettingTag>` field, `.with_tag(tag)` builder, `tags(&self) -> &[SettingTag]` accessor
- [x] Implement `SettingTagKind::colors(&self, theme) -> (Color, Color)` with the 4-variant mapping
- [x] Update `build_layout_box` to nest a row for the name line when tags are present
- [x] Update `paint` to draw tag chips (background quad + border + text)
- [x] Tag typography: `9px`, weight `700`, uppercase, `0.54px` letter spacing, `line_height: 1.3`
- [x] Border color = text color (CSS `currentColor`)
- [x] Rows with zero tags render identically to current behavior
- [x] Keep compatible with Section 06 disabled-state opacity
- [x] Monitor file size: currently 259 lines, budget ~240 more before the 500-line limit. Extract `paint_tag_chip` helper if paint method grows beyond 50 lines.

---

## 11.5 Body Spacing + Section Rhythm

### Goal

Fix the shared content-area spacing so every settings page inherits the mockup rhythm automatically.

### Files

- `form_builder/shared/mod.rs` (from 11.1)
- all 8 page builder files

### Current Mismatches

| Property | Mockup | Current | Fix |
|----------|--------|---------|-----|
| Body bottom padding | `28px` | `0` | `Insets::tlbr(0.0, 28.0, 28.0, 28.0)` |
| Inter-section gap | `28px` | `SECTION_GAP = 24.0` | Change to `28.0` |
| Intra-row gap | `0` (rows stack by own padding) | `ROW_GAP = 2.0` | Remove constant |
| Title-to-row spacing | `12px` | `8.0` or `2.0` (inconsistent) | Owned by section-header helper |

### Required Changes

1. In `build_settings_page`: change body padding bottom from `0` to `28`, section gap from `24` to `28`.
2. Each section builder changes from `.with_gap(ROW_GAP)` to `.with_gap(0.0)` — rows stack by
   their own `min-height: 44px` and `padding: 10px 14px`.
3. Remove `ROW_GAP` and `TITLE_ROW_GAP` entirely. The section-header helper from 11.3 owns the
   `12px` title-to-rows spacing.

### Ordering Constraint

11.5 depends on 11.3 having landed the section-header helper. Removing `ROW_GAP` and
`TITLE_ROW_GAP` requires the helper to own the `12px` bottom spacing. Implement 11.3 first.

### Checklist

- [x] Fix content-body bottom padding from `0` to `28` in `build_settings_page`
- [x] Change `SECTION_GAP` from `24.0` to `28.0`
- [x] Remove `ROW_GAP = 2.0` constant entirely
- [x] Remove `TITLE_ROW_GAP = 8.0` constant
- [x] Update all section builders across all 8 page files to use `gap: 0` for intra-section rows (17 `.with_gap(ROW_GAP)` sites)
- [x] Verify `keybindings.rs` page: its `KeybindRow` widgets use `min_height: 36px` and `8px` vertical padding, not `44px`/`10px`. No inter-row gap is correct here too.
- [x] Verify `colors.rs` palette section compiles correctly — it uses a private `PALETTE_GAP = 12.0` (unaffected), but the schemes section uses `ROW_GAP` and must be updated
- [x] (TPR-11-008) Wire `build_section_header_with_description` into 3 pages: font.rs Fallback section ("Used when the primary font doesn't contain a glyph (emoji, CJK, symbols)."), bell.rs Throttle section ("Suppress repeated bells to avoid visual noise from programs that ring rapidly."), rendering.rs Performance section ("Tuning options for high-throughput scenarios. Defaults are correct for most users.")
- [x] (TPR-11-008) Remove `#[expect(dead_code)]` from `build_section_header_with_description`
- [x] (TPR-11-009) Fix `build_palette_section` in colors.rs — separate header from gapped content column so the header's built-in 12px spacer is the only title-to-content gap

---

## 11.6 Tests

### Goal

Add targeted regression coverage for shared content typography and tag support.

### Test Infrastructure Reality

**What Scene/TextRun CAN observe:** `TextRun.color`, `TextRun.shaped.width`, `TextRun.shaped.height`,
`TextRun.shaped.weight` (u16), `TextRun.shaped.glyphs.len()`, `TextRun.position`. Also `Quad.style`
(fill, border) and `Quad.bounds`.

**What Scene/TextRun CANNOT observe:** original text string, letter spacing, font size (logical px),
`TextStyle`, `TextTransform` setting. `ShapedText` only stores `glyphs`, `width`, `height`,
`baseline`, `size_q6`, `weight`. `MockMeasurer` always sets `size_q6 = 0`.

Test strategies must work within these constraints. Use glyph count to verify text transforms
(uppercase doesn't change count but proves shaping happened), weight field to verify font weight,
layout positions to verify spacing, and quad styles to verify tag rendering.

### Files

- `form_builder/shared/tests.rs` (new, from 11.1)
- `oriterm_ui/src/widgets/setting_row/tests.rs` (existing)
- `form_builder/tests.rs` (existing — integration guard)

### `shared/tests.rs` — page header tests

- `fn page_header_applies_uppercase()` — build page header with `"test"`, paint to Scene.
  `MockMeasurer` applies `text_transform` before shaping, so glyph count should equal character
  count of `"TEST"` (4). Verify the title text run's glyph count matches expected length.
- `fn page_header_weight_bold()` — paint to Scene, verify title text run's `shaped.weight == 700`.
- `fn page_header_title_subtitle_spacing()` — compute layout, assert 4px gap between title and
  subtitle `LayoutNode` rects.
- `fn page_header_padding()` — compute layout in a `600x400` viewport, verify header content rect
  starts at `x=28, y=24` (from `Insets::tlbr(24, 28, 20, 28)`).
- `fn page_header_font_sizes()` — build header, compute layout. `MockMeasurer` uses `style.size`
  for `line_height` calculation when `normalized_line_height` is `Some`. Verify title and subtitle
  `LayoutNode` heights differ (18px title vs 12px subtitle produce different measured heights).

### `shared/tests.rs` — section header tests

- `fn section_header_two_text_runs()` — paint section header to Scene, assert at least 2 text
  runs exist (prefix `"//"` and title are separate labels).
- `fn section_header_weight_medium()` — paint to Scene, verify text runs have `shaped.weight == 500`.
- `fn section_header_separator_present()` — paint to Scene, verify `scene.quads()` contains at
  least one quad (the separator line).
- `fn section_header_bottom_spacing_12()` — build section column with header + dummy child,
  compute layout, assert 12px gap between header bottom edge and child top edge.
- `fn section_description_gap_4_then_12()` — build section with description, compute layout,
  assert 4px gap between title row bottom and description top, 12px between description bottom
  and first row top.
- `fn section_description_weight()` — paint section-with-description, find the description text
  run (third text run after prefix and title), verify `shaped.weight == 400` (normal weight).

### `shared/tests.rs` — body spacing tests

- `fn body_spacing_bottom_padding_28()` — build a settings page, compute layout, verify body
  container has 28px bottom padding (compare `LayoutNode.content_rect` vs `LayoutNode.rect`).
- `fn body_spacing_section_gap_28()` — build settings page with 2 sections, compute layout,
  assert 28px gap between section 1 bottom and section 2 top.
- `fn body_spacing_no_intra_row_gap()` — build section with 2 setting rows, compute layout,
  assert row 2 top edge equals row 1 bottom edge (gap = 0).
- `fn settings_page_still_builds()` — call `build_settings_page` with title, desc, and empty
  section list. Assert non-zero widget ID.

### `setting_row/tests.rs` — tag support

- `fn setting_row_stores_tags()` — create row with `.with_tag(...)`, assert `tags().len() == 1`
  and correct kind.
- `fn setting_row_zero_tags_identical_output()` — create row with no tags, paint to Scene,
  assert exactly 3 text runs (name + desc + control label). Backward-compatibility guard.
- `fn setting_row_layout_with_tags()` — create row with one tag, compute layout, verify name-line
  node width exceeds same row without tags.
- `fn setting_row_paint_tag_text_run()` — create row with `Restart` tag, paint to Scene, find
  text run with `shaped.weight == 700` and glyph count matching `"RESTART"` length (7).
- `fn setting_row_paint_tag_quad()` — create row with `Restart` tag, paint to Scene, find quad
  whose `style` fill color matches `theme.warning_bg` and border color matches `theme.warning`.
- `fn setting_row_multiple_tags()` — create row with `[New, Experimental]` tags, paint to Scene,
  verify quads with both `accent_bg_strong` and `danger_bg` fill colors.
- `fn setting_row_tag_kind_colors()` — for each `SettingTagKind` variant, verify `colors(theme)`
  returns expected `(text_color, bg_color)` pair.
- `fn setting_row_with_tags_min_height()` — create row with two tags, compute layout, assert
  height >= 44px.

### `form_builder/tests.rs` — integration guard

No new tests. Verify existing tests still pass:
- `dialog_builds_without_panic`
- `settings_ids_all_distinct`
- `dialog_builds_with_update_info`

### Checklist

- [x] Add ~14 tests to `shared/tests.rs` covering page headers (5), section headers (4), descriptions (2), body spacing (3)
- [x] Add 8 tests to `setting_row/tests.rs` covering tag storage, backward compat, layout, paint, color mapping, min height
- [x] Verify the 8 existing `setting_row` tests still pass unchanged
- [x] Verify the 6 existing `form_builder/tests.rs` tests still pass unchanged
- [x] All tests assert specific numeric values or structural properties, not just "widget exists"

---

## 11.R Third Party Review Findings

### Open Findings

- [x] `[TPR-11-010][medium]` `oriterm/src/font/collection/loading.rs:101` — Section 11 marks the
  `500`-weight section headers complete, but the embedded UI font path still cannot realize that
  weight, so every `FontWeight::MEDIUM` header renders at the same visual weight as regular text.
  Resolved 2026-03-26: accepted and fixed. Added `medium: Option<FontData>` to `FontSet` and
  `medium: Option<FaceData>` to `FontCollection`. `FontSet::ui_embedded()` now wires
  `UI_FONT_MEDIUM`. `create_shaping_faces_for_weight()` substitutes the Medium face into the
  Regular slot for weights 500..700. `rasterize_with_weight()` likewise selects the Medium face.
  Removed `#[expect(dead_code)]` from `UI_FONT_MEDIUM`. Added 3 regression tests:
  `ui_embedded_has_medium_face`, `ui_embedded_collection_has_medium`, `terminal_embedded_has_no_medium`.

- [x] `[TPR-11-008][medium]` `oriterm/src/app/settings_overlay/form_builder/shared/mod.rs:128` — Section 11.3 is marked complete, but the new section-description path is still dead code and none of the live settings pages render the mockup's `.section-desc` content. **Accepted 2026-03-25**: Valid finding. Fix tasks added to 11.5 — wire `build_section_header_with_description` into font.rs (Fallback), bell.rs (Throttle), rendering.rs (Performance), and remove `#[expect(dead_code)]`.

- [x] `[TPR-11-009][low]` `oriterm/src/app/settings_overlay/form_builder/colors.rs:140` — The Colors palette section now stacks `PALETTE_GAP` on top of the shared header helper's built-in 12px spacer, doubling the title-to-content gap. **Accepted 2026-03-25**: Valid finding. Fix task added to 11.5 — restructure `build_palette_section` to separate header from gapped content column.

### Resolved Findings

- `TPR-11-001` The draft treated Section 11 as mostly verification, but the current shared content
  path still has real mismatches: wrong body bottom padding, wrong inter-section spacing, and wrong
  section-title spacing.
- `TPR-11-002` Shared content typography currently lives in `appearance.rs` even though it is used
  by all settings-page builders. Section 11 should move that shared logic to a dedicated module.
- `TPR-11-003` The current section-title helper cannot match the mockup exactly because it applies
  title letter spacing to the `//` prefix as well. The prefix and title need separate text runs.
- `TPR-11-004` The draft missed that section titles require medium (`500`) weight. `FontWeight::MEDIUM`
  is available from Section 02, but `section_title` still uses `FontWeight::NORMAL`.
- `TPR-11-005` The draft ignored `.section-desc`, but the mockup uses section descriptions on
  multiple pages. Section 11 needs a shared primitive for that text style and spacing.
- `TPR-11-006` The draft stopped at the existing `SettingRowWidget` constants and missed the
  mockup's inline status tags (`Restart`, `Advanced`, etc.).
- `TPR-11-007` The current `ROW_GAP = 2.0` creates a shared rhythm mismatch on every page builder
  that uses it directly under section titles and between rows.

---

## 11.7 Build & Verify

### Gate

```bash
timeout 150 ./build-all.sh
timeout 150 ./clippy-all.sh
timeout 150 ./test-all.sh
```

### Focused Verification

```bash
timeout 150 cargo test -p oriterm_ui setting_row::tests
timeout 150 cargo test -p oriterm settings_overlay::form_builder::shared::tests
timeout 150 cargo test -p oriterm settings_overlay::form_builder::tests
```

### File Size Verification

After all 11.x changes, verify these files stay under 500 lines:
- `form_builder/shared/mod.rs` (new — expected ~150-200 lines)
- `form_builder/appearance.rs` (currently 331 lines, should shrink to ~180 after move)
- `setting_row/mod.rs` (currently 259 lines, expected ~350-400 after tags)

### Manual Verification Checklist

- [x] Page headers match the mockup across all settings pages
- [x] Section headers show correct weight, spacing, prefix behavior, and divider fill
- [x] Optional section descriptions render with correct spacing and muted typography
- [x] Setting rows keep correct base metrics and render status tags correctly
- [x] Content body padding and inter-section spacing match the mockup
- [x] No dead `pub(super)` items remain in `appearance.rs` (clippy `dead_code = "deny"` catches this)
- [x] No unused imports remain in any page builder file after import path changes
- [x] `shared/mod.rs` has `//!` module doc and `#[cfg(test)] mod tests;` at bottom
- [x] `/tpr-review` passed — independent Codex review found no critical or major issues (or all findings triaged)
