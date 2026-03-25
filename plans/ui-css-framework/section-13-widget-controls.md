---
section: "13"
title: "Visual Fidelity: Widget Controls"
status: not-started
reviewed: false
third_party_review:
  status: resolved
  updated: 2026-03-23
goal: "The shared settings controls match the mockup across all current control families: slider, toggle, dropdown trigger and popup, number input stepper, text input, cursor picker, and scheme cards. Control sizing variants, typography, geometry, and state behavior come from shared primitives instead of page-specific hacks."
depends_on: ["01", "02", "03", "04", "11"]
sections:
  - id: "13.1"
    title: "Shared Control Contract + Size Variants"
    status: not-started
  - id: "13.2"
    title: "Slider + Toggle Fidelity"
    status: not-started
  - id: "13.3"
    title: "Dropdown Trigger + Popup"
    status: not-started
  - id: "13.4"
    title: "Number + Text Inputs"
    status: not-started
  - id: "13.5"
    title: "Selection Controls"
    status: not-started
  - id: "13.6"
    title: "Tests"
    status: not-started
  - id: "13.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "13.7"
    title: "Build & Verify"
    status: not-started
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

- `oriterm_ui/src/widgets/slider/mod.rs`
- `oriterm_ui/src/widgets/toggle/mod.rs`
- `oriterm_ui/src/widgets/dropdown/mod.rs`
- `oriterm_ui/src/widgets/number_input/mod.rs`
- `oriterm_ui/src/widgets/text_input/mod.rs`
- `oriterm_ui/src/widgets/cursor_picker/mod.rs`
- `oriterm_ui/src/widgets/scheme_card/mod.rs`
- `oriterm/src/app/settings_overlay/form_builder/*.rs`

### Current Boundary Problem

Most control widgets currently hardcode one default geometry:

- `DropdownStyle.min_width = 140`
- `NumberInputWidget` fixed width `80`
- `TextInputStyle.min_width = 120`
- `CursorPickerWidget` fixed card gap/size
- `SchemeCardWidget` fixed width `200`

But the mockup uses multiple size variants and context-specific sizing:

- dropdowns at `140`, `160`, and `180`
- number steppers with inner input widths `56` and `44`
- text inputs at `200`
- scheme cards in an auto-fill grid with `minmax(240px, 1fr)`

### Required Shared Variant Surface

Section 13 should add shared, widget-level sizing/style APIs rather than per-page magic constants.

Examples:

- `DropdownWidget`
  - `with_min_width(...)` or a `DropdownSize` variant
- `NumberInputWidget`
  - stepper width variants for default vs compact pair usage
- `TextInputWidget`
  - fixed-width settings-field variants
- `SchemeCardWidget`
  - card width derived from the actual grid contract rather than a stale fixed width

The exact API shape can vary, but the important property is ownership: page builders should choose
from explicit supported control variants rather than cloning style structs field-by-field.

### Relationship To Section 11

Section 11 owns row rhythm and shared content spacing. Section 13 should only own control geometry
and control-specific spacing inside each widget, not global row gaps between settings rows.

### Checklist

- [ ] Add shared size/style variants for settings controls
- [ ] Remove control sizing assumptions from page-builder ad hoc constants where possible
- [ ] Keep row-spacing ownership in Section 11, not Section 13
- [ ] Make shared variants cover the actual settings consumers already present in the tree

---

## 13.2 Slider + Toggle Fidelity

### Goal

Match the mockup slider and toggle controls exactly, including geometry and value presentation.

### Files

- `oriterm_ui/src/widgets/slider/mod.rs`
- `oriterm_ui/src/widgets/slider/widget_impl.rs`
- `oriterm_ui/src/widgets/toggle/mod.rs`
- `mockups/settings-brutal.html`

### Slider Gaps

The draft overstated how close the slider already is. The current track/thumb are close, but the
value presentation is still wrong for the mockup:

- mockup slider group:
  - track/value gap `10px`
  - value label `min-width: 32px`
  - value label monospace
  - opacity sliders display `100%`, not plain `100`
- current slider:
  - `VALUE_GAP = 12`
  - `VALUE_LABEL_WIDTH = 48`
  - value text uses the default UI font/color path
  - generic formatting emits raw numeric text without suffix support

### Slider Rewrite

Keep the slider as a shared widget, but add a configurable value-display model, for example:

- `None`
- raw numeric
- percentage
- numeric with suffix

Avoid closures. A small enum-based display strategy is the feasible path for this codebase.

The slider should also allow the value label typography to match the mockup:

- `12px`
- muted color
- monospace family
- right-aligned within a compact label area

### Toggle Gaps

The draft already noticed the thumb mismatch but got stuck in CSS arithmetic. The real actionable
finding is simple:

- current code computes `thumb_size = height - 2 * thumb_padding = 14`
- mockup thumb is explicitly `12x12`
- current travel math happens to produce `18px`, but only because it uses the wrong thumb size

Section 13 should stop deriving the thumb from a simplistic formula and instead encode the actual
mockup geometry directly:

- outer size `38x20`
- thumb size `12`
- off position `left = 3`
- on translation `18`

If that requires representing thumb size independently from track height and padding, do that.

### Checklist

- [ ] Add slider value-display support for `%` and other compact formats
- [ ] Change slider value label geometry to match the mockup (`10px` gap, `32px` min width)
- [ ] Render slider value labels in monospace
- [ ] Fix toggle thumb geometry to actual `12px` size with `18px` travel
- [ ] Keep slider/toggle colors and borders aligned with current theme tokens

---

## 13.3 Dropdown Trigger + Popup

### Goal

Make dropdown controls match the mockup in both closed and open states.

### Files

- `oriterm_ui/src/widgets/dropdown/mod.rs`
- `oriterm_ui/src/widgets/menu/mod.rs`
- `oriterm_ui/src/widgets/menu/widget_impl.rs`
- `oriterm/src/app/dialog_context/overlay_actions.rs`
- `oriterm/src/app/keyboard_input/overlay_dispatch.rs`

### Trigger Gaps

The closed trigger is close but still incomplete:

- correct:
  - `2px` border
  - `12px` font size
  - `6px 30px 6px 10px` padding
  - `140px` default minimum width
- missing or inaccurate:
  - no explicit width-variant support for `160px` / `180px`
  - indicator is a Unicode `▾`, not the mockup chevron glyph/path
  - text typography is still tied to the default shared control text surface

### Popup Boundary

In this codebase, opening a dropdown goes through `WidgetAction::OpenDropdown` and the popup is a
`MenuWidget`. If Section 13 only restyles `DropdownWidget`, the open-state control still will not
match the mockup interaction.

Section 13 should therefore cover:

- `DropdownWidget` trigger fidelity
- the popup list surface used for dropdown choices

The mockup does not show an open dropdown screenshot, so do not invent a new visual language. The
popup should stay consistent with the brutal settings theme and with the trigger dimensions.

### Required Work

- add dropdown trigger width variants
- replace the Unicode indicator with proper control geometry
  - chevron path, dedicated primitive, or shared tiny icon
- ensure the popup menu style harmonizes with the same border/background/hover language

### Checklist

- [ ] Add dropdown trigger width variants for current settings consumers
- [ ] Replace the Unicode triangle indicator with actual geometry
- [ ] Cover both trigger and popup menu surfaces in this section
- [ ] Keep popup styling aligned with the brutal theme instead of leaving it as a generic menu

---

## 13.4 Number + Text Inputs

### Goal

Bring numeric and text entry controls up to the mockup's actual structure and behavior.

### Files

- `oriterm_ui/src/widgets/number_input/mod.rs`
- `oriterm_ui/src/widgets/text_input/mod.rs`
- `oriterm_ui/src/widgets/text_input/widget_impl.rs`
- `oriterm/src/app/settings_overlay/form_builder/font.rs`
- `oriterm/src/app/settings_overlay/form_builder/window.rs`
- `oriterm/src/app/settings_overlay/form_builder/terminal.rs`
- `mockups/settings-brutal.html`

### Number Input Gaps

`NumberInputWidget` is the biggest functional mismatch in the current section.

Mockup:

- real `input[type="number"]`
- wrapped in `.num-stepper`
- single `2px` outer border
- hover and focus change the wrapper border color
- inner text field width variants (`56`, `44`)
- right-side stepper column with:
  - `22px` width
  - `2px` left divider
  - `1px` horizontal divider
  - hover/active button states

Current widget:

- fixed width `80`
- fixed height `32`
- `1px` border
- font size `13`
- no text editing
- no hover border state
- no variant widths
- custom arrows painted as text, not a composed stepper model

Section 13 should not try to paper over this with paint tweaks. It needs a real stepper/input
widget model, likely by composing a numeric text-entry field with a stepper button column.

### Text Input Gaps

`TextInputWidget` already provides the right behavior family, but the default settings styling is
wrong for the mockup's general text fields:

- border is `1px`, not `2px`
- font size defaults to `13`, not `12`
- no hover border color path
- width behavior is generic `min_width = 120`, not a settings-field width contract

Section 13 should add a settings-text-input style variant rather than leaving each consumer to patch
fields individually.

### Consumer Mapping

Current settings consumers already need both widgets:

- `font.rs`
  - size and line-height numeric controls
- `window.rs`
  - padding/rows/columns numeric controls
- `terminal.rs`
  - scrollback numeric control
  - shell text input

This is shared control work, not a one-page fix.

### Checklist

- [ ] Replace the faux numeric display widget with a real stepper/input model
- [ ] Add number-input width variants for default and paired layouts
- [ ] Add settings-text-input styling for `2px` border, `12px` text, and hover/focus borders
- [ ] Keep numeric/text entry behavior shared across all current settings consumers

---

## 13.5 Selection Controls

### Goal

Bring the card-style controls used in settings pages up to mockup fidelity: cursor picker and scheme
cards.

### Files

- `oriterm_ui/src/widgets/cursor_picker/mod.rs`
- `oriterm_ui/src/widgets/scheme_card/mod.rs`
- `oriterm/src/app/settings_overlay/form_builder/colors.rs`
- `oriterm/src/app/settings_overlay/form_builder/terminal.rs`
- `mockups/settings-brutal.html`

### Cursor Picker Gaps

Mockup cursor picker:

- cards use `bg-raised` and `2px` border at rest
- hover uses `bg-hover` and `border-strong`
- active uses `accent-bg` and `accent` border
- card gap `24px`
- padding `12px 20px`
- demo font size `16`
- label font size `11`

Current `CursorPickerWidget`:

- card gap `10`
- fixed `80x72` card box with transparent normal background
- inactive border `1px`
- no proper hover background/border treatment
- demo font size `18`
- label font size `10`

Section 13 should treat this as a real fidelity rewrite, not a cosmetic tune-up.

### Scheme Card Gaps

The current scheme card is also substantially off:

- normal mockup card has persistent `bg-raised` and `2px` border
- current normal card is transparent with no border unless hovered
- mockup grid min width is `240`, current card width is `200` and grid min width is `210`
- mockup title row uses medium-weight name text
- mockup selected badge is a real chip with padding, border, uppercase, and tracking
- current selected badge is just plain `"Active"` text

Section 13 should upgrade both the widget and its consumer grid contract in `colors.rs`.

### Checklist

- [ ] Match cursor picker card spacing, typography, and active/hover states to the mockup
- [ ] Match scheme card base background/border behavior to the mockup
- [ ] Upgrade scheme card badge from plain text to a real chip
- [ ] Update scheme card grid sizing to the mockup's `240px` minimum width

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
  - `fn slider_value_label_width_and_gap()` — value-label gap is `10px`, min-width is `32px`
  - `fn slider_monospace_value_formatting()` — value label uses monospace family
  - `fn slider_percent_display_mode()` — percent mode shows `100%` not `100`
  - `fn slider_value_at_min_shows_correct_format()` — slider at minimum value formats correctly (e.g., `0%`)
  - `fn slider_value_at_max_shows_correct_format()` — slider at maximum value formats correctly (e.g., `100%`)
  - `fn slider_suffix_display_mode()` — suffix mode appends configured suffix text
- toggle (`oriterm_ui/src/widgets/toggle/tests.rs`)
  - `fn toggle_thumb_size_is_12px()` — thumb size matches mockup `12x12`
  - `fn toggle_travel_is_18px()` — on-to-off translation is `18px`
  - `fn toggle_off_position()` — off position has thumb at `left = 3`
  - `fn toggle_on_position()` — on position has thumb at `left = 3 + 18 = 21`
  - `fn toggle_outer_size_38x20()` — outer track size matches `38x20`
- dropdown (`oriterm_ui/src/widgets/dropdown/tests.rs`)
  - `fn dropdown_indicator_is_geometry_not_unicode()` — indicator is rendered geometry, not Unicode `▾`
  - `fn dropdown_width_variant_140()` — default min-width is `140px`
  - `fn dropdown_width_variant_160()` — variant supports `160px` min-width
  - `fn dropdown_width_variant_180()` — variant supports `180px` min-width
  - `fn dropdown_popup_uses_theme_style()` — popup menu respects brutal theme styling
- number input (`oriterm_ui/src/widgets/number_input/tests.rs`)
  - `fn number_input_text_is_editable()` — editable text behavior for the refactored input
  - `fn number_input_stepper_divider_geometry()` — wrapper/divider geometry matches mockup (`22px` stepper width, `2px` left divider)
  - `fn number_input_compact_width_variant()` — compact width variant (`44px`) differs from default (`56px`)
  - `fn number_input_stepper_buttons_hover()` — stepper up/down buttons have hover/active states
- text input (`oriterm_ui/src/widgets/text_input/tests.rs`)
  - `fn text_input_settings_style_border_width()` — settings-style variant has `2px` border
  - `fn text_input_settings_style_hover_border()` — hover changes border color
  - `fn text_input_settings_style_focus_border()` — focus changes border color to accent
  - `fn text_input_fixed_width_variant()` — fixed-width `200px` variant
- cursor picker (`oriterm_ui/src/widgets/cursor_picker/tests.rs`)
  - `fn cursor_picker_card_normal_background()` — normal state has `bg-raised` background and `2px` border
  - `fn cursor_picker_card_hover_state()` — hover uses `bg-hover` and `border-strong`
  - `fn cursor_picker_card_active_state()` — active uses `accent-bg` and `accent` border
  - `fn cursor_picker_card_gap()` — gap between cards is `24px`
- scheme card (`oriterm_ui/src/widgets/scheme_card/tests.rs`)
  - `fn scheme_card_normal_persistent_background()` — normal card has persistent `bg-raised` and `2px` border
  - `fn scheme_card_selected_badge_is_chip()` — selected badge renders as a styled chip with padding, border, uppercase
  - `fn scheme_card_grid_min_width_240()` — grid uses `240px` minimum card width

The current tests are too shallow for a fidelity section. They mostly verify widget existence,
fixed layout dimensions, or simple action emission.

### Checklist

- [ ] Add geometry-level tests for slider/toggle/dropdown
- [ ] Add real structure/state tests for number and text inputs
- [ ] Add card-state tests for cursor picker and scheme card widgets
- [ ] Add coverage strong enough to catch paint regressions, not just constructor regressions

---

## 13.R Third Party Review Findings

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
cargo test -p oriterm_ui slider::tests
cargo test -p oriterm_ui toggle::tests
cargo test -p oriterm_ui dropdown::tests
cargo test -p oriterm_ui number_input::tests
cargo test -p oriterm_ui text_input::tests
cargo test -p oriterm_ui cursor_picker::tests
cargo test -p oriterm_ui scheme_card::tests
```

Manual verification checklist:

- [ ] Sliders match track/thumb/value presentation from the mockup
- [ ] Toggles match `38x20` / `12px` thumb geometry and state colors
- [ ] Dropdown triggers and popup lists match the brutal settings theme
- [ ] Number and text inputs match mockup sizing, borders, and interaction states
- [ ] Cursor picker and scheme cards match the mockup's card-based control styling
