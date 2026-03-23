---
section: "11"
title: "Visual Fidelity: Content Area + Typography"
status: not-started
reviewed: true
third_party_review:
  status: resolved
  updated: 2026-03-23
goal: "The settings content area matches the mockup's typography and rhythm across all pages: page headers, section headers, optional section descriptions, setting-row labels, inline status tags, and the shared body spacing/padding all render from real shared primitives instead of ad hoc per-page constants"
depends_on: ["01", "02", "03", "04", "06"]
sections:
  - id: "11.1"
    title: "Shared Content Typography Boundary"
    status: not-started
  - id: "11.2"
    title: "Page Headers"
    status: not-started
  - id: "11.3"
    title: "Section Headers + Descriptions"
    status: not-started
  - id: "11.4"
    title: "Setting Rows + Status Tags"
    status: not-started
  - id: "11.5"
    title: "Body Spacing + Section Rhythm"
    status: not-started
  - id: "11.6"
    title: "Tests"
    status: not-started
  - id: "11.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "11.7"
    title: "Build & Verify"
    status: not-started
---

# Section 11: Visual Fidelity - Content Area + Typography

## Problem

The draft treated Section 11 as mostly a verification pass, but the current implementation still has
real gaps in the shared content typography system.

What the tree actually shows today:

- `build_settings_page(...)`, `build_page_header(...)`, and `section_title(...)` live in
  `appearance.rs`, even though they are the shared content-layout path for all settings pages.
- The page header values are close to the mockup, but the current helper still relies on callers to
  pass already-uppercased titles instead of owning the transform itself.
- Section-header spacing is wrong in multiple places:
  - the mockup uses `margin-bottom: 12px`
  - `appearance.rs` uses `TITLE_ROW_GAP = 8.0` for one section
  - most other sections use `ROW_GAP = 2.0` directly below the title
- The current section-title implementation renders `"// TITLE"` as one label with letter spacing
  applied to the entire string. The mockup explicitly resets the `//` prefix to normal spacing and
  applies the wider tracking only to the title text.
- The current `FontWeight` enum still exposes only `Regular` and `Bold`, so the mockup's `500`
  medium-weight section headers and inline tags are not expressible yet.
- The mockup includes `.section-desc` blocks on several pages (`Font`, `Bell`, `Rendering`), but
  the current shared builder path has no section-description primitive at all.
- `SettingRowWidget` gets the basic label typography mostly right, but it cannot express inline
  status tags such as `Restart` or `Advanced`, even though those are part of the mockup's content
  typography system.
- Shared content spacing is still off:
  - content body bottom padding is currently `0`, not `28`
  - inter-section spacing is `24`, not `28`
  - intra-section row spacing is implemented as a global `2px` gap, even though the mockup rows
    are stacked directly and rely on their own internal height/padding

Section 11 therefore needs to be a shared content-typography rewrite, not a checklist that says
"already matches" while the actual primitives are still missing.

## Corrected Scope

Section 11 should keep the full mockup goal and implement the shared content typography surface
properly:

1. move shared content/header helpers out of `appearance.rs`
2. make page and section headings semantic shared primitives instead of ad hoc labels
3. add optional section descriptions and inline setting tags
4. fix the shared content-body spacing constants so every page inherits the right rhythm

This section should not invent page-specific typography one file at a time. The mockup is highly
consistent across the settings pages, so the solution should be shared.

---

## 11.1 Shared Content Typography Boundary

### Goal

Put the shared content-area typography/layout logic in a shared module instead of leaving it under
the Appearance page builder.

### Files

- `oriterm/src/app/settings_overlay/form_builder/appearance.rs`
- `oriterm/src/app/settings_overlay/form_builder/*.rs`
- new shared module under `oriterm/src/app/settings_overlay/form_builder/`

### Current Boundary Problem

`appearance.rs` currently owns these shared primitives:

- `build_settings_page(...)`
- `build_page_header(...)`
- `section_title(...)`
- `PAGE_PADDING`
- `SECTION_GAP`
- `ROW_GAP`

Those are used by `window.rs`, `font.rs`, `terminal.rs`, `keybindings.rs`, `bell.rs`, `colors.rs`,
and `rendering.rs`. That is no longer a clean ownership boundary.

### Required Restructure

Move the shared content-layout and typography helpers into a dedicated module such as:

```text
oriterm/src/app/settings_overlay/form_builder/
    shared/mod.rs
    shared/tests.rs
```

The module must follow the sibling `tests.rs` pattern: `#[cfg(test)] mod tests;` at the bottom of `mod.rs`.

Recommended ownership:

- page-header helper
- section-header helper
- optional section-description helper
- shared content-body spacing constants
- small content-typography data structs if needed

Then update all page builders to import those helpers from the shared module rather than from
`appearance.rs`.

### Checklist

- [ ] Move shared content typography/layout helpers out of `appearance.rs`
- [ ] Keep shared spacing and typography constants in one module
- [ ] Update every page builder to import the shared helper module
- [ ] Stop using the Appearance page as the accidental owner of all content typography

---

## 11.2 Page Headers

### Goal

Keep the page header visually aligned to the mockup while making it a robust shared primitive across
all settings pages.

### Files

- shared form-builder module from Section 11.1
- `oriterm/src/app/settings_overlay/form_builder/*.rs`
- `oriterm_ui/src/widgets/label/mod.rs`

### Mockup Facts

The mockup page header is:

- title `18px`
- weight `700`
- uppercase
- letter spacing `0.05em` (`0.9px` at `18px`)
- bright text color
- `4px` title-to-subtitle gap
- `20px` gap after the subtitle before the first section
- header side padding `28px`
- top padding `24px`

### Current State

The current helper is close on raw values:

- title size `18`
- subtitle size `12`
- title weight `Bold`
- title letter spacing `0.9`
- side padding `28`
- top padding `24`

But two details still need cleanup:

1. the helper is shared but lives in the wrong module
2. callers currently pass strings like `"APPEARANCE"` and `"WINDOW"` already uppercased, so the
   transform is not actually owned by the shared typography primitive

### Required Work

Make the page-header helper semantic:

- accept normal page titles like `"Appearance"`
- apply uppercase transform inside the helper
- keep the exact mockup typography values at the helper boundary

The current composition of `gap = 4` plus bottom spacing of `20` is already close enough to the
mockup's `margin-bottom` behavior. Section 11 does not need to mimic CSS margins literally; it just
needs the same visual result from the shared helper.

### Checklist

- [ ] Keep page title at `18px`, weight `700`, uppercase, `0.9px` tracking
- [ ] Keep subtitle at `12px` and muted color
- [ ] Apply uppercase transform inside the helper, not in each caller
- [ ] Preserve `24 / 28 / 20` effective page-header spacing across all pages

---

## 11.3 Section Headers + Descriptions

### Goal

Make section headers and optional section descriptions match the mockup exactly and come from a real
shared primitive.

### Files

- shared form-builder module from Section 11.1
- `oriterm_ui/src/widgets/label/mod.rs`
- `oriterm_ui/src/widgets/separator/mod.rs`
- `oriterm_ui/src/text/mod.rs`
- `mockups/settings-brutal.html`

### Current Gaps

The mockup section heading system is richer than the current helper:

- title text is `11px`, weight `500`, uppercase, `0.15em` tracking
- the `//` prefix has normal tracking
- the title row has `12px` bottom spacing
- some sections also include a `.section-desc` block:
  - font size `12px`
  - muted color
  - line height `1.5`
  - effective `4px` gap below the title row
  - `12px` gap below the description before the first row

The current helper does not match this:

- it uses one combined `LabelWidget` for `"// TITLE"`
- it applies `1.6px` tracking to the slashes and the title together
- it leaves weight at the default regular weight
- it has no section-description support at all
- its bottom spacing is inconsistent across consumers (`8px` in one place, `2px` in others)

### Required Header Rewrite

Replace the current `section_title(...)` helper with a richer shared primitive, for example:

- `build_section_header(title, theme)`
- `build_section_header_with_description(title, desc, theme)`

The important part is the structure:

- prefix text `"//"` rendered separately
- title text rendered separately
- separator line fills the remaining width

That split is necessary so only the title text receives uppercase tracking while the prefix keeps
normal spacing, matching the mockup's `::before` behavior.

### Section Description Support

Add a real optional description primitive for sections that need it.

The mockup already uses this pattern on several pages:

- `Font` fallback section
- `Bell` throttle section
- `Rendering` performance section

Section 11 should provide the shared typography/layout primitive now, even if some of those content
sections are introduced or expanded by other plan sections later.

Because the current `TextStyle` surface does not yet carry line-height, the section-description
helper should explicitly consume the line-height support defined in Section 04 rather than faking it
with arbitrary spacer constants.

### Checklist

- [ ] Replace the single combined `"// TITLE"` label with separate prefix and title runs
- [ ] Apply medium (`500`) weight to section titles once Section 02 lands
- [ ] Keep title tracking on the title text only, not on the prefix
- [ ] Add a shared optional section-description primitive
- [ ] Match the mockup's `12px` title-bottom spacing and description spacing

---

## 11.4 Setting Rows + Status Tags

### Goal

Keep the strong parts of `SettingRowWidget`, but extend it so it can express the full mockup label
system.

### Files

- `oriterm_ui/src/widgets/setting_row/mod.rs`
- `oriterm_ui/src/widgets/setting_row/tests.rs`
- `oriterm/src/app/settings_overlay/form_builder/*.rs`
- `oriterm_ui/src/text/mod.rs`

### What Already Matches

`SettingRowWidget` already matches several core row metrics:

- row min height `44`
- row padding `10px 14px`
- label/control gap `24px`
- name size `13px`
- description size `11.5px`
- name/description gap `2px`
- hover background `theme.bg_card`

Those values should stay.

### Real Missing Features

The draft stopped at those constants, but the mockup includes richer label content:

- inline status tags on the name line
  - examples: `Restart`, `Advanced`
- tag styling:
  - font size `9px`
  - weight `700`
  - uppercase
  - letter spacing `0.06em`
  - padding `2px 5px`
  - `1px` border using the current text color
  - variant colors for accent, warning, and danger

The current `SettingRowWidget` only stores:

- `name: String`
- `description: String`
- `control: Box<dyn Widget>`

That is not enough to represent the mockup.

### Required Row Contract Upgrade

Extend `SettingRowWidget` to accept a richer label model, for example:

- plain name + description + zero or more tags, or
- a dedicated `SettingLabel` struct with:
  - name
  - description
  - tags

Recommended tag model:

```rust
enum SettingTagKind {
    New,
    Restart,
    Experimental,
}
```

with a tag payload that supports label text and style variant.

Do not solve this with a single rich-text string. The more feasible approach is to render the name
line as a small row of widgets or text/rect primitives:

- name text
- zero or more tag chips after it

That keeps measurement and hover behavior tractable inside the existing widget model.

### Disabled Rows

The mockup also defines `.setting-row.disabled { opacity: 0.4; pointer-events: none; }`.
Behavioral ownership for that belongs with Section 06, but Section 11 should make sure the richer
setting-row label/tag model remains compatible with disabled-state opacity and hit suppression.

### Checklist

- [ ] Preserve the existing correct row metrics and typography constants
- [ ] Extend `SettingRowWidget` with a real tag/badge model
- [ ] Render name-line tags with mockup typography and color variants
- [ ] Keep the row compatible with Section 06 disabled-state behavior
- [ ] Avoid rich-text string hacks for mixed label/tag content

---

## 11.5 Body Spacing + Section Rhythm

### Goal

Fix the shared content-area spacing so every settings page inherits the mockup rhythm automatically.

### Files

- shared form-builder module from Section 11.1
- `oriterm/src/app/settings_overlay/form_builder/*.rs`
- `mockups/settings-brutal.html`

### Current Mismatches

The mockup content-body structure is:

- body padding `0 28px 28px`
- section margin-bottom `28px`
- no trailing margin after the last section
- section title bottom spacing `12px`
- row stack uses the rows' own height/padding, not a global extra `2px` inter-row gap

The current shared constants do not match:

- body bottom padding is effectively `0`
- `SECTION_GAP = 24.0`
- `ROW_GAP = 2.0`
- title-to-first-row spacing is inconsistent (`8px` in one place, `2px` elsewhere)

### Required Spacing Rewrite

Update the shared body rhythm to reflect the mockup directly:

- content-body padding:
  - top `0`
  - left/right `28`
  - bottom `28`
- inter-section gap `28`
- no generic row gap inside a section
- title/description-to-row spacing handled by the section-header primitive from Section 11.3

This is important because the current `ROW_GAP = 2.0` leaks mockup-inaccurate spacing into every
page builder that imports the shared constants from `appearance.rs`.

`ContainerWidget::column().with_gap(...)` already gives the correct "no trailing gap after the last
section" behavior, so Section 11 only needs to set the right shared values and remove the wrong
ones.

### Checklist

- [ ] Fix content-body bottom padding from `0` to `28`
- [ ] Change inter-section spacing from `24` to `28`
- [ ] Remove the generic `2px` intra-section row gap
- [ ] Let section-header primitives own title/description spacing instead of per-page constants

---

## 11.6 Tests

### Goal

Add targeted regression coverage for shared content typography instead of relying on a visual audit.

### Files

- new shared form-builder tests if the shared module is added
- `oriterm_ui/src/widgets/setting_row/tests.rs`
- `oriterm/src/app/settings_overlay/form_builder/tests.rs`

### Required Coverage

Add tests for the shared content typography path:

- `fn page_header_title_subtitle_spacing()` — title/subtitle gap matches mockup `4px`
- `fn page_header_applies_uppercase_transform()` — title receives uppercase transform internally
- `fn page_header_letter_spacing_correct()` — title tracking is `0.9px`
- `fn section_header_prefix_separate_from_title()` — prefix `"//"` and title are separate text runs
- `fn section_header_title_only_receives_tracking()` — letter spacing applies only to the title, not the prefix
- `fn section_header_separator_fills_width()` — separator line fills remaining width
- `fn section_description_composition()` — optional section-description composes below title with `4px` gap
- `fn section_description_uses_line_height()` — section-description text style sets `line_height: Some(1.5)` via Section 04's capability
- `fn body_spacing_bottom_padding_28()` — content-body bottom padding is `28`
- `fn body_spacing_section_gap_28()` — inter-section gap is `28`
- `fn body_spacing_no_intra_row_gap()` — no generic `2px` gap between rows within a section

Expand `SettingRowWidget` tests for:

- `fn setting_row_stores_tags()` — tag model storage
- `fn setting_row_layout_with_tags()` — layout with one or more tags adds to name-line width
- `fn setting_row_paint_includes_tag_chips()` — paint output includes tag chips with correct colors
- `fn setting_row_multiple_tags()` — row with two tags renders both
- `fn setting_row_disabled_with_tags()` — disabled-state opacity compatibility with tags if Section 06 lands first

The current `setting_row` tests only verify that three text runs are produced for a simple row.
That is not enough once tags and richer label structure exist.

### Checklist

- [ ] Add shared content-typography tests for headers, sections, and spacing
- [ ] Expand `setting_row` tests to cover tags and richer label layout
- [ ] Keep existing basic construction/layout tests
- [ ] Add coverage strong enough to catch spacing regressions, not just widget existence

---

## 11.R Third Party Review Findings

### Resolved Findings

- `TPR-11-001` The draft treated Section 11 as mostly verification, but the current shared content
  path still has real mismatches: wrong body bottom padding, wrong inter-section spacing, and wrong
  section-title spacing.
- `TPR-11-002` Shared content typography currently lives in `appearance.rs` even though it is used
  by all settings-page builders. Section 11 should move that shared logic to a dedicated module.
- `TPR-11-003` The current section-title helper cannot match the mockup exactly because it applies
  title letter spacing to the `//` prefix as well. The prefix and title need separate text runs.
- `TPR-11-004` The draft missed that section titles require medium (`500`) weight, which the
  current `FontWeight` surface cannot yet express.
- `TPR-11-005` The draft ignored `.section-desc`, but the mockup uses section descriptions on
  multiple pages. Section 11 needs a shared primitive for that text style and spacing.
- `TPR-11-006` The draft stopped at the existing `SettingRowWidget` constants and missed the
  mockup's inline status tags (`Restart`, `Advanced`, etc.), which require a richer row-label model.
- `TPR-11-007` The current `ROW_GAP = 2.0` is not a harmless approximation; it creates a shared
  rhythm mismatch on every page builder that uses it directly under section titles and between rows.

---

## 11.7 Build & Verify

### Gate

```bash
./build-all.sh
./clippy-all.sh
./test-all.sh
```

### Focused Verification

- run the targeted `setting_row` tests
- run the settings form-builder tests
- verify typography and spacing in the live settings overlay across more than one page, not just
  `Appearance`

Suggested commands:

```bash
cargo test -p oriterm_ui setting_row::tests
cargo test -p oriterm settings_overlay::form_builder::tests
```

Manual verification checklist:

- [ ] Page headers match the mockup across all settings pages
- [ ] Section headers show correct weight, spacing, prefix behavior, and divider fill
- [ ] Optional section descriptions render with correct spacing and muted typography
- [ ] Setting rows keep correct base metrics and render status tags correctly
- [ ] Content body padding and inter-section spacing match the mockup
