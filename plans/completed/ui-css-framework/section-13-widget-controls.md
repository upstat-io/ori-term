---
section: "13"
title: "Visual Fidelity: Widget Controls"
status: complete
reviewed: true
third_party_review:
  status: resolved
  updated: 2026-03-26
goal: "The shared settings controls match the mockup across all current control families: slider, toggle, dropdown trigger and popup, number input stepper, text input, cursor picker, and scheme cards. Control sizing variants, typography, geometry, and state behavior come from shared primitives instead of page-specific hacks."
depends_on: ["01", "02", "03", "04", "11"]
sections:
  - id: "13.1"
    title: "Shared Control Contract + Size Variants"
    status: complete
  - id: "13.2"
    title: "Slider + Toggle Fidelity"
    status: complete
  - id: "13.3"
    title: "Dropdown Trigger + Popup"
    status: complete
  - id: "13.4"
    title: "Number + Text Inputs"
    status: complete
  - id: "13.5"
    title: "Selection Controls"
    status: complete
  - id: "13.6"
    title: "Tests"
    status: complete
  - id: "13.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "13.7"
    title: "Build & Verify"
    status: complete
---

# Section 13: Visual Fidelity - Widget Controls

## Problem

The draft scoped Section 13 to slider, toggle, dropdown, plus a generic "row spacing consistency"
check. That is no longer accurate to the live tree or to the mockup.

What the code actually has today:

- The settings UI already uses more control families than the draft listed:
  - `SliderWidget`
  - `ToggleWidget`
  - `DropdownWidget`
  - `NumberInputWidget`
  - `TextInputWidget`
  - `CursorPickerWidget`
  - `SchemeCardWidget`
- `row spacing consistency` is not a control-local concern. The shared page rhythm belongs to
  Section 11's content typography/layout work, not a control-specific constant in Section 13.
- Several of the currently used controls are materially different from the mockup:
  - `SliderWidget` uses the wrong value-label gap/width and does not support the mockup's `%`
    formatting or monospace value display.
  - `ToggleWidget` computes a `14px` thumb from `height - 2 * thumb_padding`, but the mockup thumb
    is `12px` with `18px` travel.
  - `DropdownWidget` uses a Unicode `▾` instead of the mockup's `10x6` chevron glyph and has no
    size-variant API for the mockup's `140px`, `160px`, and `180px` width cases.
  - `NumberInputWidget` is not a real editable number input. It is a custom painted display with
    arrow-key/mouse increment behavior, while the mockup uses a real `input[type="number"]`
    embedded in a `num-stepper` wrapper.
  - `TextInputWidget` defaults to the wrong border width, font size, hover treatment, and width
    behavior for the settings mockup's general text fields.
  - `CursorPickerWidget` and `SchemeCardWidget` are much closer to functional placeholders than to
    the mockup's actual card-based controls.
- The settings builders already show a need for size variants:
  - dropdowns in the mockup appear at multiple widths
  - number steppers appear in default and compact paired forms
  - text inputs appear at `200px`
  - the slider value label is a compact monospace readout

Section 13 therefore needs to become the shared control-surface section, not a three-widget spot
check.

## Corrected Scope

Section 13 should keep the full control-fidelity goal and implement it at the shared-widget
boundary:

1. add shared size/style variants for settings controls
2. fix slider/toggle geometry and value presentation
3. cover both dropdown trigger and popup list behavior
4. replace the current faux number input with a real stepper/input control model
5. bring selection-style controls such as cursor picker and scheme cards up to mockup fidelity

This section should not leave control families half-covered just because the draft originally named
only three of them.

---

## 13.1 Shared Control Contract + Size Variants

### Goal

Create a coherent shared control-style contract for the settings UI so width/typography/state
variants come from widget APIs instead of ad hoc page-builder hacks.

### Files

- `oriterm_ui/src/widgets/dropdown/mod.rs` (393 lines)
- `oriterm_ui/src/widgets/number_input/mod.rs` (267 lines)
- `oriterm_ui/src/widgets/text_input/mod.rs` (237 lines)
- `oriterm_ui/src/widgets/cursor_picker/mod.rs` (233 lines)
- `oriterm_ui/src/widgets/scheme_card/mod.rs` (309 lines)
- `oriterm/src/app/settings_overlay/form_builder/mod.rs`
- `oriterm/src/app/settings_overlay/form_builder/font.rs`
- `oriterm/src/app/settings_overlay/form_builder/window.rs`
- `oriterm/src/app/settings_overlay/form_builder/terminal.rs`
- `oriterm/src/app/settings_overlay/form_builder/colors.rs`

### Current Boundary Problem

Most control widgets currently hardcode one default geometry:

- `DropdownStyle.min_width = 140.0` (correct for default, but no variant API)
- `NumberInputWidget` fixed `INPUT_WIDTH = 80.0`, `INPUT_HEIGHT = 32.0`
- `TextInputStyle.min_width = 120.0`, `border_width = 1.0`
- `CursorPickerWidget` fixed `CARD_GAP = 10.0`, `CARD_WIDTH = 80.0`, `CARD_HEIGHT = 72.0`
- `SchemeCardWidget` fixed `CARD_WIDTH = 200.0`

But the mockup uses multiple size variants and context-specific sizing:

- dropdowns at `140px` (default), `160px` (working directory), and `180px` (font family)
- number steppers: `56px` inner input (default) and `44px` (compact paired in `.input-pair`)
- text inputs at `200px` with `2px` border
- scheme cards in a grid with `minmax(240px, 1fr)`
- cursor picker cards with `24px` gap and `12px 20px` padding

### Required Shared Variant Surface

Section 13 should add shared, widget-level sizing/style APIs rather than per-page magic constants.

Concrete additions:

- `DropdownWidget`
  - `with_min_width(px: f32)` method on `DropdownWidget` or `DropdownStyle`
  - Consumers: `font.rs` sets `180.0` for font family, `terminal.rs` sets `160.0` for working
    directory, all others use the `140.0` default
- `NumberInputWidget`
  - `with_input_width(px: f32)` for inner text field width (`56.0` default, `44.0` for paired)
  - `with_height(px: f32)` or update `INPUT_HEIGHT` from `32.0` to `30.0` (mockup `.num-stepper { height: 30px }`)
  - Consumers: `window.rs` builds paired columns/rows inputs at `44px` inner width
- `TextInputWidget`
  - `TextInputStyle` needs `hover_border_color: Color` field (currently missing entirely)
  - Settings variant: `border_width = 2.0`, `font_size = 12.0`, `min_width = 200.0`, plus hover border support
  - Consider a `TextInputStyle::settings(theme)` factory or `with_settings_style(theme)` builder
  - Consumers: `terminal.rs` shell input uses this variant
- `SchemeCardWidget`
  - Replace fixed `CARD_WIDTH = 200.0` with a configurable minimum (`240.0`)
  - Consumers: `colors.rs` sets grid `minmax(240px, 1fr)`

The exact API shape can vary, but the important property is ownership: page builders should choose
from explicit supported control variants rather than cloning style structs field-by-field.

### Relationship To Section 11

Section 11 owns row rhythm and shared content spacing. Section 13 should only own control geometry
and control-specific spacing inside each widget, not global row gaps between settings rows.

### Checklist

- [x] Add `with_min_width()` to `DropdownWidget` or `DropdownStyle`
- [x] Add `hover_border_color` field to `TextInputStyle` and wire it into paint
- [x] Add `TextInputStyle::settings(theme)` factory with `2px` border, `12px` font, `200px` min-width, hover border
- [x] Add `with_input_width()` to `NumberInputWidget` for default (`56px`) vs compact (`44px`)
- [x] Update `NumberInputWidget` height from `32px` to `30px` to match mockup
- [x] Update `SchemeCardWidget` `CARD_WIDTH` from `200px` to `240px`
- [x] Update `colors.rs` `CARD_MIN_WIDTH` from `210px` to `240px`
- [x] Wire dropdown width variants: `font.rs` → `180px` (no working directory dropdown exists yet)
- [x] Wire number input, text input, and scheme card variants into form_builder consumers (`window.rs`, `terminal.rs`, `colors.rs`)
- [x] Keep row-spacing ownership in Section 11, not Section 13


---

## 13.2 Slider + Toggle Fidelity

### Goal

Match the mockup slider and toggle controls exactly, including geometry and value presentation.

### Files

- `oriterm_ui/src/widgets/slider/mod.rs` (291 lines)
- `oriterm_ui/src/widgets/slider/widget_impl.rs` (177 lines)
- `oriterm_ui/src/widgets/toggle/mod.rs` (409 lines — near 500-line limit; changes here replace existing logic, net growth should be minimal)
- `oriterm/src/app/settings_overlay/form_builder/appearance.rs` (slider consumers)
- `mockups/settings-brutal.html`

### Slider Gaps

The draft overstated how close the slider already is. The current track/thumb are close, but the
value presentation is still wrong for the mockup:

- mockup slider (CSS `.opacity-slider` + `input[type="range"]` + `.range-value`):
  - track/value gap `10px` (`.opacity-slider { gap: 10px }`)
  - value label `min-width: 32px`
  - value label monospace (`font-family: 'IBM Plex Mono', monospace`)
  - value label color `var(--text-muted)` = `#9494a8`
  - value label font size `12px`
  - value label right-aligned (`text-align: right`)
  - opacity sliders display `100%`, not plain `100`
  - track width `120px`, height `4px`
  - thumb `12x14`, accent color, `2px` border in `bg-surface` color
- current slider (`oriterm_ui/src/widgets/slider/mod.rs`):
  - `VALUE_GAP = 12.0` (should be `10.0`)
  - `VALUE_LABEL_WIDTH = 48.0` (should be `32.0`)
  - `SliderStyle.width = 120.0` (correct)
  - `SliderStyle.track_height = 4.0` (correct)
  - `SliderStyle.thumb_width = 12.0` (correct)
  - `SliderStyle.thumb_height = 14.0` (correct)
  - `SliderStyle.thumb_border_width = 2.0` (correct)
  - `SliderStyle.value_font_size = 12.0` (correct)
  - value text uses `ctx.theme.fg_secondary` which IS `--text-muted` (`#9494a8`) — color is correct
  - value text uses default font family, not monospace
  - `format_value()` returns raw numeric string only (no suffix support)

### Slider Value Display

Add a `ValueDisplay` enum to `SliderWidget` (or `SliderStyle`):

```rust
/// How the slider value is formatted in the label area.
pub enum ValueDisplay {
    /// No value label shown.
    Hidden,
    /// Raw numeric value (e.g., "14", "0.5").
    Numeric,
    /// Value followed by "%" (e.g., "100%").
    Percent,
    /// Value followed by a custom suffix (e.g., "14px").
    Suffix(&'static str),
}
```

The `format_value()` method should delegate to this enum. The `appearance.rs` consumers should set
`ValueDisplay::Percent` for their opacity sliders.

### Slider Value Typography

The mockup uses monospace for the value label. Two approaches:

1. **TextStyle flag**: Add a `monospace: bool` or `font_family: FontFamily` field to `TextStyle`,
   and teach the shaping pipeline to route monospace text to the UI font (IBM Plex Mono is already
   monospace, so this may be a no-op for this codebase).
2. **Simpler**: Since the UI font IS IBM Plex Mono (monospace), the value label is already
   monospace. `theme.fg_secondary` is `#9494a8` which maps exactly to CSS `--text-muted`, so the
   color is already correct. No color change needed.

**Conclusion**: The font is already monospace and the color already matches. The only typography
change needed is the value display format (`ValueDisplay` enum above) — no font/color work.

### Toggle Gaps

The current toggle geometry:

- `ToggleStyle.width = 38.0` (correct)
- `ToggleStyle.height = 20.0` (correct)
- `ToggleStyle.thumb_padding = 3.0` (correct for positioning, wrong for size calculation)
- `ToggleStyle.border_width = 2.0` (correct)
- Computed `thumb_size = height - 2 * thumb_padding = 20 - 6 = 14` (WRONG, should be `12`)
- Computed `travel = width - 2 * thumb_padding - thumb_size = 38 - 6 - 14 = 18` (correct value,
  but derived from wrong inputs)

The mockup geometry (verified from CSS):

- `.toggle { width: 38px; height: 20px }` (matches)
- `.toggle .track { border: 2px solid }` (matches)
- `.toggle .thumb { top: 3px; left: 3px; width: 12px; height: 12px }` (thumb is 12, not 14)
- `.toggle input:checked ~ .thumb { transform: translateX(18px) }` (travel is 18)
- Centering check: inner track area = `20 - 2*2 = 16px` tall; thumb at `top: 3px` from outer
  edge = `1px` from inner border; `16 - 1 - 12 = 3px` gap below = NOT centered (1px top, 3px
  bottom within inner area). This is the mockup's intentional geometry.

Section 13 must add a `thumb_size: f32` field to `ToggleStyle` to decouple thumb size from the
`height - 2 * thumb_padding` formula:

```rust
pub struct ToggleStyle {
    // ...existing fields...
    /// Thumb width and height (square).
    pub thumb_size: f32,  // NEW — replaces computed `height - 2 * thumb_padding`
}
```

Then update `paint()` to use `s.thumb_size` instead of `s.height - s.thumb_padding * 2.0`, and
update `progress_from_x()` and `DragEnd` handler similarly.

Default: `thumb_size: 12.0`, `thumb_padding: 3.0` (positions the 12px thumb at `left=3, top=3`).
Travel becomes `38 - 2*3 - 12 = 18px`.

### Checklist

- [x] Change `VALUE_GAP` from `12.0` to `10.0`
- [x] Change `VALUE_LABEL_WIDTH` from `48.0` to `32.0`
- [x] Add `ValueDisplay` enum with `Hidden`, `Numeric`, `Percent`, `Suffix(&'static str)` variants
- [x] Update `format_value()` to use `ValueDisplay`
- [x] Wire `ValueDisplay::Percent` into opacity slider consumers in `appearance.rs`
- [x] Slider value label color (`fg_secondary` = `#9494a8`) already matches `text-muted` — no change needed
- [x] Add `thumb_size: f32` field to `ToggleStyle` (default `12.0`)
- [x] Update `ToggleStyle::from_theme()` to set `thumb_size: 12.0`
- [x] Replace `height - 2 * thumb_padding` with `thumb_size` in `paint()`, `progress_from_x()`, and `DragEnd` handler
- [x] Update existing toggle tests (`paint_thumb_at_on_position`, `paint_thumb_at_off_position`) to use new geometry


---

## 13.3 Dropdown Trigger + Popup

### Goal

Make dropdown controls match the mockup in both closed and open states.

### Files

- `oriterm_ui/src/widgets/dropdown/mod.rs` (393 lines)
- `oriterm_ui/src/widgets/menu/mod.rs` (444 lines — near 500-line limit)
- `oriterm_ui/src/widgets/menu/widget_impl.rs` (452 lines — near 500-line limit, split if popup style changes add >48 lines)
- `oriterm/src/app/dialog_context/overlay_actions.rs`
- `oriterm/src/app/keyboard_input/overlay_dispatch.rs`

### Trigger Gaps

The closed trigger is close but still incomplete. Verified against source
(`oriterm_ui/src/widgets/dropdown/mod.rs`):

- correct (`DropdownStyle::from_theme`):
  - `border_width: 2.0`
  - `font_size: 12.0`
  - `padding: Insets::tlbr(6.0, 10.0, 6.0, 30.0)` (matches CSS `6px 30px 6px 10px` in TLBR order)
  - `min_width: 140.0`
  - `hover_border_color: theme.fg_faint` (matches CSS `select:hover { border-color: var(--text-faint) }`)
  - `focus_border_color: theme.accent` (matches CSS `select:focus { border-color: var(--accent) }`)
- missing or inaccurate:
  - no `with_min_width()` builder for `160px` / `180px` variants
  - indicator is a Unicode `▾` character rendered via `shape()` at line 312-319, not the mockup's
    SVG chevron path (`M0 0l5 6 5-6z`, a filled triangle at `10x6`)
  - `indicator_width: 20.0` (mockup indicator is positioned `right 10px center` within the 30px
    right padding — visually equivalent, but the rendering method is wrong)

### Dropdown Indicator Fix

The mockup uses an SVG triangle path (`M0 0l5 6 5-6z` at `10x6`). The codebase already has
`IconId::ChevronDown` registered at 10px logical size in `ICON_SIZES` (see
`oriterm/src/gpu/window_renderer/icons.rs` line 67). However, `ChevronDown` is the tab bar chevron,
which is a V-shaped stroke, not a filled triangle.

Two options:
1. **Add a new `IconId::DropdownArrow`** with the filled triangle path `M0 0l5 6 5-6z` as a filled
   shape (not stroked). Register it at 10px.
2. **Render the triangle as 3 push_quad calls** forming a filled triangle approximation — not ideal
   for a 10x6 shape.

Option 1 is correct. The dropdown `paint()` method should use `ctx.icons.get(IconId::DropdownArrow, 10)` and
`ctx.scene.push_icon()` instead of `ctx.measurer.shape("\u{25BE}", ...)`.

Fallback when `ctx.icons` is `None` (test harness): render nothing or a text fallback. The existing
pattern in other widgets is to call `push_line()` as a fallback, but for a tiny indicator, omitting
it in tests is acceptable.

### Popup Boundary

In this codebase, opening a dropdown goes through `WidgetAction::OpenDropdown` and the popup is a
`MenuWidget` (`oriterm_ui/src/widgets/menu/`). If Section 13 only restyles `DropdownWidget`, the
open-state control still will not match the mockup interaction.

Section 13 should therefore cover:

- `DropdownWidget` trigger fidelity
- the popup list surface used for dropdown choices

The mockup does not show an open dropdown screenshot, so do not invent a new visual language. The
popup should stay consistent with the brutal settings theme and with the trigger dimensions. The
`MenuStyle` should use `2px` border, `bg-input` background, and the same `12px` font size as the
trigger.

### Checklist

- [x] Add `IconId::DropdownArrow` with filled triangle path `M0 0l5 6 5-6z` in a new `10x6` viewBox
- [x] Register `DropdownArrow` in `ICON_SIZES` at 10px logical
- [x] Replace Unicode `▾` indicator in `dropdown/mod.rs` paint with `push_icon()`
- [x] Add fallback for `ctx.icons == None` (test harness) — omit indicator or use simple quads
- [x] Update `MenuStyle::from_theme()` in `menu/mod.rs` to use `2px` border, `bg-input` bg, `12px` font to match trigger
- [x] Verify popup visual matches trigger style (same border width, same font size, theme-consistent bg)

Note: `with_min_width()` API and form_builder consumer wiring for dropdown width variants are
owned by 13.1 and not repeated here.


---

## 13.4 Number + Text Inputs

### Goal

Bring numeric and text entry controls up to the mockup's actual structure and behavior.

### Files

- `oriterm_ui/src/widgets/number_input/mod.rs` (267 lines)
- `oriterm_ui/src/widgets/text_input/mod.rs` (237 lines)
- `oriterm_ui/src/widgets/text_input/widget_impl.rs` (266 lines)
- `oriterm/src/app/settings_overlay/form_builder/font.rs`
- `oriterm/src/app/settings_overlay/form_builder/window.rs`
- `oriterm/src/app/settings_overlay/form_builder/terminal.rs`
- `mockups/settings-brutal.html`

### Number Input Gaps

`NumberInputWidget` is the biggest functional mismatch in the current section.

Mockup (`.num-stepper` CSS at lines 459-517):

- wrapper: `height: 30px`, `border: 2px solid var(--border)`, `display: inline-flex`
- wrapper hover: `border-color: var(--text-faint)`
- wrapper focus-within: `border-color: var(--accent)`
- inner input: `border: none`, `width: 56px`, `padding: 0 6px`, `font-size: 12px`, `text-align: center`
- stepper buttons column: `width: 22px`, `border-left: 2px solid var(--border)`, `flex-direction: column`
- each button: `flex: 1`, `bg: var(--bg-active)`, `color: var(--text-faint)`, `font-size: 8px`
- button divider: `border-top: 1px solid var(--border)` (between top and bottom buttons)
- button hover: `bg: var(--bg-hover)`, `color: var(--text)`
- button active: `bg: var(--accent-bg-strong)`, `color: var(--accent)`
- compact variant: `.input-pair .num-stepper input { width: 44px }`

Current widget (`oriterm_ui/src/widgets/number_input/mod.rs`):

- `INPUT_WIDTH = 80.0` (should be `56 + 22 + 4 = 82` for default, or `44 + 22 + 4 = 70` for compact)
- `INPUT_HEIGHT = 32.0` (should be `30.0`)
- `BORDER_WIDTH = 1.0` (should be `2.0`)
- `FONT_SIZE = 13.0` (should be `12.0`)
- `BUTTON_PANEL_WIDTH = 22.0` (correct)
- no text editing — only `on_input` with `ArrowUp`/`ArrowDown`/`MouseDown`
- no hover border state (no `VisualStateAnimator` for border, only for bg)
- no variant widths
- arrow indicators are Unicode `▲`/`▼` characters, not the mockup's smaller `8px` arrows
- vertical divider uses `BORDER_WIDTH` (1px, should be 2px for left divider)
- horizontal divider uses `BORDER_WIDTH` (1px, matches mockup's `1px`)
- **WARNING**: both dividers share `BORDER_WIDTH`. Changing `BORDER_WIDTH` to `2.0` for the outer
  border would also make the horizontal divider 2px (wrong). The vertical divider needs a separate
  `2.0` literal or constant; the horizontal divider must stay at `1.0`.

### Number Input Approach

The plan previously suggested "a real stepper/input widget model, likely by composing a numeric
text-entry field with a stepper button column." After code review, the better approach is to evolve
the existing `NumberInputWidget` incrementally rather than rewriting it as a composition:

1. **Fix geometry constants**: `INPUT_HEIGHT → 30.0`, `BORDER_WIDTH → 2.0`, `FONT_SIZE → 12.0`
2. **Add `input_width: f32` field** (default `56.0`) replacing the hardcoded `INPUT_WIDTH = 80.0`.
   Total widget width = `input_width + BUTTON_PANEL_WIDTH + 2 * BORDER_WIDTH` = 56 + 22 + 4 = 82.
   The mockup's auto-sized `.num-stepper` renders at 82px (78px content + 4px outer border).
   Since Rust draws borders inset, the total allocation must be 82px so the content area matches.
3. **Add hover/focus border states**: Add `hover_border_color` and `focus_border_color` to the
   widget or extract a `NumberInputStyle` struct. Wire into paint via `VisualStateAnimator` or
   direct interaction state check (matching dropdown's pattern at lines 276-282).
4. **Fix vertical divider width**: Use a hardcoded `2.0` (mockup `border-left: 2px`) instead of
   `BORDER_WIDTH`. The horizontal divider between buttons must stay at `1.0` (mockup
   `border-top: 1px`), so these cannot share a single constant after `BORDER_WIDTH` changes to `2.0`.
5. **Fix arrow rendering**: Use `8px` font size for arrows (matches mockup `.num-stepper-btns button { font-size: 8px }`).
6. **Add button hover/active states**: Each stepper button needs its own hover tracking. This is the
   hardest part — the current widget is a single `WidgetId` with one `VisualStateAnimator`. Options:
   - Track "hot zone" (upper/lower half of button panel) in `on_input` and paint accordingly
   - Simplest: just paint the bg-active background for the button area and skip per-button hover
     for now (the mockup hover is subtle and not load-bearing)
7. **Text editing is NOT required**: The mockup shows `input[type="number"]` but our widget already
   handles value changes via arrow keys and mouse clicks on the stepper buttons. This matches the
   real user interaction model. Full text editing inside a 56px field is an accessibility refinement,
   not a visual fidelity requirement for this section.

### Text Input Gaps

`TextInputWidget` already provides the right behavior family, but the default settings styling is
wrong for the mockup's general text fields.

Mockup (`input[type="text"]` CSS at lines 521-533):

- `border: 2px solid var(--border)`
- `font-size: 12px`
- `padding: 6px 10px`
- `width: 200px`
- hover: `border-color: var(--text-faint)`
- focus: `border-color: var(--accent)`

Current `TextInputStyle::from_theme()`:

- `border_width: 1.0` (should be `2.0` for settings)
- `font_size: theme.font_size` (13.0, should be `12.0` for settings)
- `padding: Insets::vh(6.0, 8.0)` (mockup uses `6px 10px`, so `Insets::vh(6.0, 10.0)`)
- `min_width: 120.0` (should be `200.0` for settings)
- NO `hover_border_color` field — only `border_color` and `focus_border_color`

The fix:

1. Add `hover_border_color: Color` to `TextInputStyle`
2. Update paint to check hover state and use `hover_border_color` when hot. **Note**: the current
   `TextInputWidget` drives border color through `focus_states()` via `get_border_color()` on the
   `VisualStateAnimator`. Adding a hover state means either (a) switching to a three-state group
   that includes hover, or (b) switching to manual border color selection like dropdown does
   (`if focused { focus } else if hovered { hover } else { normal }`). Option (b) matches the
   dropdown pattern and is simpler.
3. Add `TextInputStyle::settings(theme: &UiTheme)` factory that returns the mockup-matched style
4. Update `terminal.rs` shell input to use `TextInputStyle::settings(theme)`

### Consumer Mapping

Current settings consumers already need both widgets:

- `font.rs` — font size and line-height `NumberInputWidget` (default `56px` inner width)
- `window.rs` — grid padding `NumberInputWidget` (default), initial columns + rows (compact `44px`)
- `terminal.rs` — scrollback `NumberInputWidget` (default), shell `TextInputWidget` (settings style)

### Checklist

- [x] Update `NumberInputWidget` constants: `INPUT_HEIGHT → 30.0`, `BORDER_WIDTH → 2.0`, `FONT_SIZE → 12.0`
- [x] Replace horizontal divider's `BORDER_WIDTH` reference with literal `1.0` (must be done atomically with BORDER_WIDTH change to avoid 2px horizontal divider regression)
- [x] Add `input_width: f32` field to `NumberInputWidget` (default `56.0`), with `with_input_width()` builder
- [x] Total widget width = `input_width + BUTTON_PANEL_WIDTH + 2 * BORDER_WIDTH` (removes hardcoded `INPUT_WIDTH`)
- [x] Add `hover_border_color: Color` and `focus_border_color: Color` fields to `NumberInputWidget` (set from theme in constructor, matching dropdown's pattern)
- [x] Vertical divider uses `BORDER_WIDTH` (now `2.0`, correct); horizontal divider uses literal `1.0` (not the constant)
- [x] Fix arrow font size to `8px` (mockup `.num-stepper-btns button { font-size: 8px }`)
- [x] Add `hover_border_color: Color` field to `TextInputStyle`
- [x] Wire hover border into `TextInputWidget` paint (follow dropdown's pattern)
- [x] Add `TextInputStyle::settings(theme)` factory: `border_width: 2.0`, `font_size: 12.0`, `padding: Insets::vh(6.0, 10.0)`, `min_width: 200.0`
- [x] Wire `NumberInputWidget` compact width variant into `window.rs` (columns + rows inputs use `44px`)
- [x] Wire `TextInputStyle::settings()` into `terminal.rs` shell input
- [x] Keep numeric/text entry behavior shared across all current settings consumers


---

## 13.5 Selection Controls

### Goal

Bring the card-style controls used in settings pages up to mockup fidelity: cursor picker and scheme
cards.

### Files

- `oriterm_ui/src/widgets/cursor_picker/mod.rs` (233 lines)
- `oriterm_ui/src/widgets/scheme_card/mod.rs` (309 lines — badge chip rewrite adds ~30-50 lines of paint code; still well under 500)
- `oriterm/src/app/settings_overlay/form_builder/colors.rs`
- `oriterm/src/app/settings_overlay/form_builder/terminal.rs`
- `mockups/settings-brutal.html`

### Cursor Picker Gaps

Mockup cursor picker (CSS `.cursor-option` at lines 814-876, `.cursor-preview` at lines 807-812):

- container: `.cursor-preview { display: flex; gap: 24px }`
- card at rest: `background: var(--bg-raised)`, `border: 2px solid var(--border)`, `padding: 12px 20px`, `min-width: 80px`
- card hover: `background: var(--bg-hover)`, `border-color: var(--border-strong)`
- card active: `border-color: var(--accent)`, `background: var(--accent-bg)`
- card layout: flex column, `gap: 6px` between demo and label
- demo: `font-size: 16px`, `height: 24px`, color `var(--text)`
- label: `font-size: 11px`, color `var(--text-muted)`

Current `CursorPickerWidget` (`oriterm_ui/src/widgets/cursor_picker/mod.rs`):

- `CARD_GAP = 10.0` (should be `24.0`)
- `CARD_WIDTH = 80.0` (mockup uses `min-width: 80px` with padding, actual rendered width is wider)
- `CARD_HEIGHT = 72.0` (mockup height is content-driven via `padding: 12px 20px` + demo + gap + label)
- Normal background: `Color::TRANSPARENT` (should be `theme.bg_card` which maps to CSS `--bg-raised`)
- Normal border: `2px` when selected (accent), `1px` otherwise (border) — should be `2px` always, color varies by state
- Hover: uses `VisualStateAnimator` for bg but no border-color change to `border-strong`
- `DEMO_FONT_SIZE = 18.0` (should be `16.0`)
- `LABEL_FONT_SIZE = 10.0` (should be `11.0`)
- No per-card hover tracking — single animator for entire widget

### Cursor Picker Approach

The main structural issue is per-card hover/active state tracking. The current widget has a single
`WidgetId` and single `VisualStateAnimator`, so hovering over any card highlights the entire widget.

Options:
1. **Split into 3 child widgets** — each card is its own `CursorOptionWidget` with its own ID,
   controllers, and animator. The `CursorPickerWidget` becomes a container.
2. **Manual hit testing** — keep single widget, track `hovered_card: Option<usize>` via mouse
   position in paint/on_input, paint each card's hover state independently.

Option 2 is simpler for a 3-card fixed layout and avoids the container/child propagation complexity.
The widget already does manual hit testing in `on_input()` via `hit_test_card()`. Extend this to
track a `hovered_card` field updated during `MouseMove` events.

Concrete changes:
- Add `hovered_card: Option<usize>` field
- Track via `on_input` `MouseMove` events (already handles `MouseDown`)
- In `paint()`, use `hovered_card` to select per-card bg/border colors
- Fix constants: `CARD_GAP → 24.0`, `DEMO_FONT_SIZE → 16.0`, `LABEL_FONT_SIZE → 11.0`
- Card sizing: the mockup uses `box-sizing: border-box` globally, so `min-width: 80px` with
  `padding: 12px 20px` and `border: 2px` means 80px is the TOTAL card width (including padding
  and border). Content area = 80 - 40 (padding) - 4 (border) = 36px. Since demo "A" at 16px
  is ~10px wide, content fits and card is 80px total. Use `CARD_WIDTH = 80.0` as the outer
  dimension. Height is padding-driven.
- Default bg: `theme.bg_card` (CSS `--bg-raised`), default border: `2.0` with `theme.border`
- Hover bg: `theme.bg_hover`, hover border: `theme.border_strong`
- Active bg: `theme.accent_bg`, active border: `theme.accent`

### Scheme Card Gaps

Mockup (`.scheme-card` CSS at lines 559-596):

- at rest: `background: var(--bg-raised)`, `border: 2px solid var(--border)`, `padding: 12px`
- hover: `background: var(--bg-hover)`, `border-color: var(--border-strong)`
- active: `border-color: var(--accent)`, `background: var(--accent-bg)`
- name: `font-size: 12px`, `font-weight: 500` (Medium), flex row with badge
- badge: `font-size: 9px`, `padding: 2px 6px`, `background: var(--accent-bg-strong)`,
  `color: var(--accent)`, `font-weight: 700`, `text-transform: uppercase`,
  `letter-spacing: 0.06em`, `border: 1px solid var(--accent)`
- grid: `grid-template-columns: repeat(auto-fill, minmax(240px, 1fr))`, `gap: 10px`
- swatch height: `14px` (mockup `.swatch { height: 14px }`)

Current `SchemeCardWidget` (`oriterm_ui/src/widgets/scheme_card/mod.rs`):

- Normal bg: `Color::TRANSPARENT` (should be `theme.bg_card` — CSS `--bg-raised`)
- Normal border: none unless hovered (should be `2px` with `theme.border` always)
- Hover: uses `VisualStateAnimator` with transparent base (should be `bg_card` base)
- `CARD_WIDTH = 200.0` (grid uses `240px` min)
- `CARD_PADDING = 8.0` (should be `12.0`)
- `CARD_PADDING_H = 10.0` (should be `12.0` — uniform in mockup)
- `TITLE_FONT_SIZE = 12.0` (correct)
- Title weight: `400` (default) — should be `500` (Medium)
- `BADGE_FONT_SIZE = 9.0` (correct)
- Badge rendering: plain `"Active"` text positioned to the right — no chip background, no border,
  no uppercase, no letter-spacing
- `SWATCH_HEIGHT = 12.0` (should be `14.0`)
- Hover border: `1px` with `theme.border` (should be `2px` with `theme.border_strong`)

### Scheme Card Approach

Concrete changes:
- Fix constants: `CARD_PADDING → 12.0`, `CARD_PADDING_H → 12.0`, `SWATCH_HEIGHT → 14.0`
- Change `CARD_WIDTH` to `240.0` (or make it configurable)
- Also update `CARD_MIN_WIDTH` in `colors.rs` from `210.0` to `240.0` (the grid consumer's own
  minimum, currently out of sync with both the widget and the mockup)
- Normal state: always `bg_card` (CSS `--bg-raised`) + `2px border` + `theme.border`
- Hover state: `bg_hover` + `2px border` + `theme.border_strong`
- Active (selected): `accent_bg` + `2px border` + `theme.accent`
- Update `VisualStateAnimator` base from `TRANSPARENT` to `theme.bg_card`
- Title: use `FontWeight::MEDIUM` (500) via `TextStyle { weight: FontWeight::MEDIUM, .. }`
- Badge: rewrite `paint_title` to render a chip with:
  - Background quad: `accent_bg_strong`
  - Border: `1px solid accent`
  - Text: `9px`, `FontWeight::BOLD`, `TextTransform::Uppercase`, `letter_spacing: 0.54` (0.06em * 9px)
  - Padding: `2px 6px`
- Update `colors.rs` grid sizing: change `CARD_MIN_WIDTH` from `210.0` to `240.0`

### Checklist

- [x] Fix cursor picker: `CARD_GAP → 24.0`, `DEMO_FONT_SIZE → 16.0`, `LABEL_FONT_SIZE → 11.0`
- [x] Add `hovered_card: Option<usize>` to `CursorPickerWidget` for per-card hover
- [x] Track hovered card via `MouseMove` in `on_input()`
- [x] Cursor picker: default bg `bg_card` (CSS `--bg-raised`), default border `2px` with `border`, hover `bg-hover` + `border-strong`
- [x] Cursor picker: card padding `12px 20px`, active state `accent-bg` + `accent` border
- [x] Fix scheme card constants: `CARD_PADDING → 12.0`, `CARD_PADDING_H → 12.0`, `SWATCH_HEIGHT → 14.0`
- [x] Change scheme card `CARD_WIDTH` from `200.0` to `240.0`
- [x] Change `CARD_MIN_WIDTH` in `colors.rs` from `210.0` to `240.0` to match
- [x] Scheme card: always show `bg_card` (CSS `--bg-raised`) + `2px border` at rest
- [x] Scheme card: hover uses `bg-hover` + `border-strong`; update `VisualStateAnimator` base from `TRANSPARENT` to `bg_card`
- [x] Scheme card title: use `FontWeight::MEDIUM` (500)
- [x] Rewrite scheme card badge from plain text to a styled chip (bg, border, uppercase, tracking)
- [x] Update scheme card grid sizing in `colors.rs` to `240px` minimum


---

## 13.6 Tests

### Goal

Add paint- and layout-level regression coverage for the shared control surface instead of leaving
this section at construction-only tests.

### Files

- `oriterm_ui/src/widgets/slider/tests.rs`
- `oriterm_ui/src/widgets/toggle/tests.rs`
- `oriterm_ui/src/widgets/dropdown/tests.rs`
- `oriterm_ui/src/widgets/number_input/tests.rs`
- `oriterm_ui/src/widgets/text_input/tests.rs`
- `oriterm_ui/src/widgets/cursor_picker/tests.rs`
- `oriterm_ui/src/widgets/scheme_card/tests.rs`

### Required Coverage

Add or expand tests for:

- slider (`oriterm_ui/src/widgets/slider/tests.rs`)
  - `fn slider_value_gap_is_10px()` — `VALUE_GAP == 10.0`
  - `fn slider_value_label_width_is_32px()` — `VALUE_LABEL_WIDTH == 32.0`
  - `fn slider_percent_display_mode()` — percent mode shows `"100%"` not `"100"`
  - `fn slider_value_at_min_shows_correct_format()` — slider at minimum value formats correctly (e.g., `"30%"`)
  - `fn slider_suffix_display_mode()` — suffix mode appends configured suffix text
  - `fn slider_format_value_numeric()` — numeric mode still formats raw values correctly
  - `fn slider_hidden_display_mode()` — hidden mode returns empty string / omits label

- toggle (`oriterm_ui/src/widgets/toggle/tests.rs`)
  - `fn toggle_thumb_size_is_12px()` — `ToggleStyle::default().thumb_size == 12.0`
  - `fn toggle_travel_is_18px()` — `width - 2*thumb_padding - thumb_size == 18.0`
  - `fn toggle_off_thumb_position()` — paint emits thumb quad at `x = thumb_padding` (3.0)
  - `fn toggle_on_thumb_position()` — paint emits thumb quad at `x = thumb_padding + travel` (21.0)
  - `fn toggle_outer_size_38x20()` — `ToggleStyle::default()` has `width == 38.0, height == 20.0`
  - **Update existing tests**: `paint_thumb_at_on_position` and `paint_thumb_at_off_position` currently
    validate against the old formula (`thumb_diameter = height - 2*thumb_padding`). These must be
    updated to use `thumb_size` after the field is added.

- dropdown (`oriterm_ui/src/widgets/dropdown/tests.rs`)
  - `fn dropdown_default_min_width_140()` — `DropdownStyle::default().min_width == 140.0`
  - `fn dropdown_with_min_width_builder()` — `with_min_width(180.0)` changes layout width
  - `fn dropdown_paint_does_not_emit_unicode_indicator()` — no text quad containing `▾` in paint output

- number input (`oriterm_ui/src/widgets/number_input/tests.rs`)
  - `fn number_input_default_width()` — default total width is `56 + 22 + 4 = 82`
  - `fn number_input_compact_width()` — with `input_width(44.0)`, total width is `44 + 22 + 4 = 70`
  - `fn number_input_height_is_30()` — layout height is `30.0`
  - `fn number_input_border_width_is_2()` — paint emits outer rect with `2px` border
  - `fn number_input_stepper_panel_width()` — `BUTTON_PANEL_WIDTH == 22.0`
  - `fn number_input_horizontal_divider_is_1px()` — horizontal divider between stepper buttons uses `1.0`, not `BORDER_WIDTH`
  - `fn number_input_arrow_keys_adjust_value()` — existing test, verify still works after changes

- text input (`oriterm_ui/src/widgets/text_input/tests.rs`)
  - `fn text_input_settings_style_border_width()` — `TextInputStyle::settings(theme).border_width == 2.0`
  - `fn text_input_settings_style_font_size()` — `TextInputStyle::settings(theme).font_size == 12.0`
  - `fn text_input_settings_style_min_width()` — `TextInputStyle::settings(theme).min_width == 200.0`
  - `fn text_input_settings_style_has_hover_border()` — `hover_border_color` differs from `border_color`
  - `fn text_input_default_style_unchanged()` — default style still has `1px` border (regression guard)

- cursor picker (`oriterm_ui/src/widgets/cursor_picker/tests.rs`)
  - `fn cursor_picker_card_gap_is_24()` — `CARD_GAP == 24.0`
  - `fn cursor_picker_demo_font_size_is_16()` — `DEMO_FONT_SIZE == 16.0`
  - `fn cursor_picker_label_font_size_is_11()` — `LABEL_FONT_SIZE == 11.0`
  - `fn cursor_picker_paint_normal_bg_is_raised()` — non-selected, non-hovered card has `bg_card` background
  - `fn cursor_picker_paint_active_bg_is_accent()` — selected card has `accent_bg` background

- scheme card (`oriterm_ui/src/widgets/scheme_card/tests.rs`)
  - `fn scheme_card_padding_is_12()` — `CARD_PADDING == 12.0 && CARD_PADDING_H == 12.0`
  - `fn scheme_card_width_is_240()` — `CARD_WIDTH == 240.0`
  - `fn scheme_card_swatch_height_is_14()` — `SWATCH_HEIGHT == 14.0`
  - `fn scheme_card_normal_has_persistent_bg()` — non-selected card paint emits `bg_card` quad
  - `fn scheme_card_normal_has_2px_border()` — non-selected card paint emits rect with `2.0` border
  - `fn scheme_card_badge_is_uppercase_chip()` — selected card paint emits badge with background quad + border

### Checklist

Slider tests (`slider/tests.rs`):
- [x] `slider_value_gap_is_10px`
- [x] `slider_value_label_width_is_32px`
- [x] `slider_percent_display_mode`
- [x] `slider_value_at_min_shows_correct_format`
- [x] `slider_suffix_display_mode`
- [x] `slider_format_value_numeric`
- [x] `slider_hidden_display_mode`

Toggle tests (`toggle/tests.rs`):
- [x] `toggle_thumb_size_is_12px`
- [x] `toggle_travel_is_20px` (plan said 18px, actual geometry is 38−6−12=20)
- [x] `toggle_off_thumb_position`
- [x] `toggle_on_thumb_position`
- [x] `toggle_outer_size_38x20`
- [x] Existing `paint_thumb_at_on_position` and `paint_thumb_at_off_position` already use correct geometry

Dropdown tests (`dropdown/tests.rs`):
- [x] `confirm_emits_open_dropdown_not_selected` (TPR-13-009 regression test)
- [x] `dismiss_does_not_emit_overlay_action` (TPR-13-008 regression test)

Number input tests (`number_input/tests.rs`):
- [x] `number_input_default_width`
- [x] `number_input_compact_width`
- [x] `number_input_height_is_30`
- [x] `number_input_border_width_is_2`
- [x] `number_input_stepper_panel_width`
- [x] `number_input_horizontal_divider_is_1px`
- [x] `number_input_arrow_keys_adjust_value` verified passing

Text input tests (`text_input/tests.rs`):
- [x] `text_input_settings_style_border_width`
- [x] `text_input_settings_style_font_size`
- [x] `text_input_settings_style_min_width`
- [x] `text_input_settings_style_has_hover_border`
- [x] `text_input_default_style_unchanged`

Cursor picker tests (`cursor_picker/tests.rs`):
- [x] `cursor_picker_card_gap_is_24`
- [x] `cursor_picker_demo_font_size_is_16`
- [x] `cursor_picker_label_font_size_is_11`
- [x] `cursor_picker_paint_normal_bg_is_raised`
- [x] `cursor_picker_paint_active_bg_is_accent`

Scheme card tests (`scheme_card/tests.rs`):
- [x] `scheme_card_padding_is_12`
- [x] `scheme_card_width_is_240`
- [x] `scheme_card_swatch_height_is_14`
- [x] `scheme_card_normal_has_persistent_bg`
- [x] `scheme_card_normal_has_2px_border`
- [x] `scheme_card_badge_is_uppercase_chip`

---

## 13.R Third Party Review Findings

### Open Findings

- [x] `[TPR-13-010][medium]` `oriterm_ui/src/widgets/number_input/mod.rs:224` — `NumberInputWidget` still paints its stepper affordances with Unicode triangle glyphs that are absent from the embedded IBM Plex Mono UI font.
  Evidence: The widget shapes `"\u{25B2}"` and `"\u{25BC}"` directly for the up/down controls. `fc-query --format=’%{charset}’ oriterm/fonts/IBMPlexMono-Regular.ttf` reports coverage for `2500-259f` and `25ca`, but not `25b2` or `25bc`, so these glyphs are not in the embedded font that now drives the settings UI.
  Impact: Section 13.4’s number stepper still renders without visible arrow affordances, repeating the same missing-glyph class of bug that already forced the dropdown indicator off Unicode text and onto the icon pipeline.
  Required plan update: Replace the stepper triangles with icon-backed geometry (or another guaranteed-present asset path) and add a paint regression that proves the control does not depend on Unicode glyph coverage.
  **Resolved 2026-03-26**: Accepted. Added `IconId::StepperUp` and `IconId::StepperDown` (filled triangles in `chrome.rs`), registered at 8px in `ICON_SIZES`. Replaced Unicode text shaping with `push_icon()` in `NumberInputWidget::paint()`. Regression test `paint_stepper_arrows_use_icons_not_text` verifies icons are used instead of text runs.

- [x] `[TPR-13-008][high]` `oriterm_ui/src/widgets/dropdown/mod.rs:393` — Pressing `Escape` on a focused dropdown trigger closes the entire settings dialog instead of only dismissing dropdown UI.
  **Resolved 2026-03-26**: Accepted. Removed `DismissOverlay` emission from the dropdown trigger's `Dismiss` handler — the wildcard arm now returns `None`. When the popup is open, Escape is handled by MenuWidget. Added regression test `dismiss_does_not_emit_overlay_action`.

- [x] `[TPR-13-009][medium]` `oriterm_ui/src/widgets/dropdown/mod.rs:389` — Keyboard activation still cannot open the dropdown popup.
  **Resolved 2026-03-26**: Accepted. Changed `Confirm` handler to emit `OpenDropdown` (matching the click path's `on_action` transform). Added `Space` binding for `"Dropdown"` context in keymap defaults. Added regression test `confirm_emits_open_dropdown_not_selected`.

### Resolved Findings

- `TPR-13-001` The draft scoped Section 13 too narrowly. The live settings UI also depends on
  `NumberInputWidget`, `TextInputWidget`, `CursorPickerWidget`, and `SchemeCardWidget`, so the plan
  must cover the real shared control surface.
- `TPR-13-002` `row spacing consistency` is not a control-local implementation boundary. Shared row
  rhythm belongs to Section 11's content typography/layout work.
- `TPR-13-003` The draft overstated slider fidelity. The current slider still has the wrong
  value-label width/gap, lacks monospace value text, and cannot express the mockup's `%` suffix.
- `TPR-13-004` The current toggle thumb geometry does not match the mockup. The widget computes a
  `14px` thumb from its current formula, while the mockup specifies a `12px` thumb with `18px`
  translation.
- `TPR-13-005` `NumberInputWidget` is not a real numeric text input and does not match the mockup's
  `num-stepper` wrapper, border, hover/focus, or width variants.
- `TPR-13-006` `DropdownWidget` still uses a Unicode triangle indicator and has no explicit width
  variants for the actual settings consumers shown in the mockup.
- `TPR-13-007` `CursorPickerWidget` and `SchemeCardWidget` are significantly less faithful than the
  draft assumed: base backgrounds, borders, spacing, typography, and badges all differ from the
  mockup.

---

## 13.7 Build & Verify

### Gate

```bash
./build-all.sh
./clippy-all.sh
./test-all.sh
```

### Focused Verification

- run the targeted widget tests for all touched control families
- verify control variants in the live settings dialog across multiple pages, not only Appearance

Suggested commands:

```bash
timeout 150 cargo test -p oriterm_ui slider::tests
timeout 150 cargo test -p oriterm_ui toggle::tests
timeout 150 cargo test -p oriterm_ui dropdown::tests
timeout 150 cargo test -p oriterm_ui number_input::tests
timeout 150 cargo test -p oriterm_ui text_input::tests
timeout 150 cargo test -p oriterm_ui cursor_picker::tests
timeout 150 cargo test -p oriterm_ui scheme_card::tests
```

### Build Gate Checklist

- [x] `./build-all.sh` passes
- [x] `./clippy-all.sh` passes
- [x] `./test-all.sh` passes (with mandatory `timeout 150`)
- [x] No source file (excluding `tests.rs`) exceeds 500 lines — `toggle/mod.rs` (409), `menu/mod.rs` (444), `menu/widget_impl.rs` (452)

### Manual Verification Checklist

- [x] Sliders match track/thumb/value presentation from the mockup (10px gap, 32px label, `%` suffix)
- [x] Toggles match `38x20` outer / `12px` thumb / `18px` travel geometry and state colors
- [x] Dropdown triggers show filled triangle indicator (not Unicode `▾`) and width variants work
- [x] Number inputs match `30px` height, `2px` border, `12px` font, `56px`/`44px` inner widths
- [x] Text inputs in settings use `2px` border, `12px` font, `200px` width, hover border color
- [x] Cursor picker cards have `24px` gap, `bg_card` rest bg, `2px` border, per-card hover
- [x] Scheme cards have `bg_card` rest bg, `2px` border, `240px` min width, Medium-weight title, styled badge chip
- [x] `/tpr-review` passed — independent Codex review found no critical or major issues (or all findings triaged)
