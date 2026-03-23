---
section: "12"
title: "Visual Fidelity: Footer + Buttons"
status: not-started
reviewed: true
third_party_review:
  status: resolved
  updated: 2026-03-23
goal: "The settings footer matches the mockup structurally and visually: it lives only in the right content column, the unsaved group and Reset/Cancel/Save button cluster are laid out correctly, the shared button primitive can express the required typography and disabled state, and footer dirty-state behavior stays synchronized with the real settings pipeline"
depends_on: ["01", "02", "03", "05", "08", "10"]
sections:
  - id: "12.1"
    title: "Right Column Footer Structure"
    status: not-started
  - id: "12.2"
    title: "Shared Button Typography + States"
    status: not-started
  - id: "12.3"
    title: "Unsaved Indicator + Dirty State"
    status: not-started
  - id: "12.4"
    title: "Semantic Actions + Tests"
    status: not-started
  - id: "12.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "12.5"
    title: "Build & Verify"
    status: not-started
---

# Section 12: Visual Fidelity - Footer + Buttons

## Problem

The draft framed Section 12 as a footer styling pass, but the current implementation has deeper
layout and state-model problems.

What the tree actually shows today:

- `SettingsPanel` currently appends a full-width footer row below the entire settings content.
  That means the sidebar stops above the footer, which does not match the mockup's full-height
  sidebar.
- `SettingsPanel::paint()` then draws an opaque footer background hack across the full panel width,
  including the sidebar area.
- The mockup footer HTML is:
  - `footer-left` unsaved group
  - `Reset to Defaults`
  - `Cancel`
  - `Save`
  with the left group consuming the `margin-right: auto` slot.
- The current footer layout is:
  - `Reset`
  - fill spacer
  - `Cancel`
  - fixed `8px`
  - `Save`
  and the unsaved indicator is painted as an overlay, not part of layout.
- Because the unsaved indicator is painted at the same left inset where the Reset button is laid
  out, the current implementation can overlap the Reset button.
- The current unsaved indicator is text only. The mockup requires a `14px` warning icon, `6px`
  icon/text gap, uppercase tracked text, and left-group layout.
- The current button primitive cannot express the full footer typography:
  - no font weight in `ButtonStyle`
  - no letter spacing in `ButtonStyle`
  - no button-level text-transform support
  - no correct disabled-state border/opacity handling for `.btn-primary:disabled`
- The draft also got one mockup fact wrong: `.btn-primary` is `font-weight: 700`, not `500`.
- There is a real dirty-state sync gap today: after `ResetDefaults`, the dialog title bullet is
  recomputed, but the rebuilt `SettingsPanel` is not sent `SettingsUnsaved(dirty)`, so the footer
  state can desynchronize from the actual pending-vs-original config state.
- `oriterm_ui/src/widgets/settings_panel/mod.rs` is already `487` lines, so adding more footer
  logic there is not a maintainable path.

Section 12 therefore needs a structural rewrite, not just more `ButtonStyle` fields and visual
verification.

## Corrected Scope

Section 12 should keep the full mockup goal and implement it at the right boundaries:

1. move footer ownership out of the full-panel bottom bar and into the right content column
2. make the unsaved group a real layout participant instead of paint-time overlay text
3. extend the shared button primitive so footer buttons can match the mockup exactly
4. keep footer dirty-state and semantic button actions synchronized with the real dialog pipeline

This section should not preserve the current full-panel footer hack and try to patch around it with
more manual paint math.

---

## 12.1 Right Column Footer Structure

### Goal

Make the footer live only in the right content column so the sidebar remains full-height and the
footer layout matches the mockup's actual DOM structure.

### Files

- `oriterm_ui/src/widgets/settings_panel/mod.rs`
- `oriterm/src/app/settings_overlay/form_builder/mod.rs`
- `oriterm_ui/src/widgets/page_container/mod.rs`
- new footer widget/module in `oriterm_ui/src/widgets/`

### Current Structural Mismatch

The mockup's footer belongs to the right pane, not the whole panel.

Current tree:

- `SettingsPanel`
  - header
  - body row (`sidebar + pages`)
  - footer separator
  - footer row

Mockup structure:

- panel
  - sidebar rail, full height
  - right column
    - page content
    - footer

The current structure is why the sidebar does not extend to the bottom of the panel and why the
footer background has to be overpainted manually.

### Required Structure

Introduce a dedicated settings footer widget and compose it inside the right content column:

```text
settings panel chrome
  content row
    sidebar
    right column
      page container (fill)
      settings footer (fixed height)
```

Recommended ownership:

- `SettingsPanel`
  - panel chrome
  - header / close button
  - no knowledge of Reset/Cancel/Save footer layout
- `SettingsFooterWidget`
  - unsaved group
  - Reset / Cancel / Save buttons
  - footer separator/background
  - semantic action translation for those buttons
- `build_settings_dialog(...)`
  - compose `sidebar + right_column(page_container + footer)`

This removes the current full-width footer bar hack and brings the layout in line with the mockup.

### Why This Is Also The Feasible Path

`settings_panel/mod.rs` is already at `487` lines. Keeping all footer layout, dirty state, button
action mapping, and footer paint logic inside that file is likely to push it over the repository
limit immediately. Extracting a dedicated footer widget solves both the structural mismatch and the
maintainability problem.

### Checklist

- [ ] Create dedicated footer widget module (e.g., `oriterm_ui/src/widgets/settings_footer/mod.rs` with `#[cfg(test)] mod tests;` and `tests.rs`)
- [ ] Move footer ownership out of `SettingsPanel` and into the dedicated right-column footer widget
- [ ] Compose the footer under the page container, not under the full panel
- [ ] Keep the sidebar full-height relative to the settings content area
- [ ] Remove the full-panel footer background overpaint hack
- [ ] Keep `settings_panel/mod.rs` under the repository file-size limit

---

## 12.2 Shared Button Typography + States

### Goal

Extend the shared button primitive so the settings footer buttons match the mockup's typography and
disabled behavior without turning footer code into a one-off paint fork.

### Files

- `oriterm_ui/src/widgets/button/mod.rs`
- `oriterm_ui/src/widgets/button/tests.rs`
- new footer widget/module from Section 12.1
- `mockups/settings-brutal.html`

### Mockup Facts

Common `.btn` typography:

- font size `12px`
- uppercase
- letter spacing `0.04em`
- padding `6px 16px`
- border width `2px`

Variant details:

- `btn-danger-ghost`
  - text muted at rest
  - danger border/text/background on hover
  - weight `500`
- `btn-ghost`
  - text muted at rest
  - stronger border and bright text on hover
  - weight `500`
- `btn-primary`
  - accent bg/border
  - dark text
  - hover accent-hover
  - weight `700`
  - disabled opacity `0.4`

### Current Shared Primitive Gap

`ButtonStyle` currently lacks:

- font weight
- letter spacing
- text transform
- disabled border handling
- a good fit for CSS-like disabled opacity

Section 12 should add those capabilities at the shared button boundary, not hardcode them inside the
settings footer.

### Required `ButtonStyle` Upgrade

Add shared typography/state fields, for example:

- `font_weight`
- `letter_spacing`
- `text_transform` if Section 03 exposes a shared transform surface
- either:
  - `disabled_border_color` plus explicit disabled colors, or
  - a shared `disabled_opacity` path that is applied consistently to text/background/border

The exact disabled-state API can vary, but it must let the footer's primary Save button match the
mockup's disabled appearance instead of remaining fully accented with only a foreground/background
swap.

### Footer Button Variants

Use the shared button primitive to build three explicit footer styles:

- Reset
  - muted rest
  - danger-hover treatment
  - medium weight
- Cancel
  - muted rest
  - hover surface/bright text/stronger border
  - medium weight
- Save
  - accent-filled primary
  - bold weight
  - correct disabled state when clean

Manual uppercase labels are acceptable as a temporary bridge only if Section 03's text-transform
surface is not available yet. Once it exists, the transform should move into `ButtonStyle` so the
footer and other dialogs do not depend on pre-uppercased strings.

### Checklist

- [ ] Add font weight support to `ButtonStyle`
- [ ] Add letter spacing support to `ButtonStyle`
- [ ] Use shared text-transform support for buttons when available
- [ ] Add correct disabled-state styling for primary buttons
- [ ] Implement footer Reset/Cancel/Save variants from shared button style fields

---

## 12.3 Unsaved Indicator + Dirty State

### Goal

Make the unsaved indicator a real footer-left group that matches the mockup visually and stays
synchronized with the actual pending-config dirty state.

### Files

- new footer widget/module from Section 12.1
- `oriterm/src/app/dialog_context/content_actions.rs`
- `oriterm_ui/src/icons/mod.rs`
- `oriterm/src/gpu/icon_rasterizer/mod.rs`

### Current Gaps

The current implementation is missing most of the mockup behavior:

- no left-group layout
- no icon
- no `6px` icon/text gap
- no tracked/weighted text style
- no fade/visible state semantics
- dirty-state desync after `ResetDefaults`

### Required Footer-Left Model

Build a real left-side footer group:

- left group occupies the `margin-right: auto` slot
- Reset / Cancel / Save stay grouped on the right
- when there are no unsaved changes:
  - the group should collapse or render hidden without disturbing button alignment
- when there are unsaved changes:
  - show warning icon + label as one aligned group

That means the unsaved indicator must be part of the footer widget layout, not drawn after the fact
in `paint()`.

### Indicator Visual Requirements

Match the mockup:

- icon size `14px`
- icon/text gap `6px`
- label size `11px`
- medium weight
- uppercase
- `0.06em` tracking
- warning color

For the icon, add a shared warning glyph through the icon pipeline rather than inventing a local
one-off primitive inside the footer widget. Section 08 covered sidebar fidelity, but this section
should still use the shared icon system for the new footer glyph.

### Dirty-State Synchronization

Current bug to fix:

- after `ResetDefaults`, the dialog rebuilds the panel and recomputes the title bullet
- but it never reapplies `SettingsUnsaved(dirty)` to the rebuilt settings widget tree

Section 12 must fix that flow so the footer indicator and Save disabled state remain correct after:

- ordinary setting edits
- page switches
- reset-to-defaults rebuilds
- save/apply
- cancel/close

### Save Button Enabled State

The mockup provides a disabled style for `.btn-primary:disabled`. The footer should use that:

- `Save` enabled when `pending_config != original_config`
- `Save` disabled when clean

This should be driven by the same dirty-state source as the unsaved indicator. Do not maintain two
independent booleans.

### Checklist

- [ ] Make the unsaved indicator a real footer-left layout group
- [ ] Add the warning icon through the shared icon system
- [ ] Match the mockup's `11px` / medium / tracked warning label style
- [ ] Drive indicator visibility and Save enabled state from one dirty-state source
- [ ] Reapply dirty state after reset rebuilds so footer state does not desynchronize

---

## 12.4 Semantic Actions + Tests

### Goal

Keep footer semantics clean and add regression coverage for the layout/state issues the current draft
missed.

### Files

- new footer widget/module from Section 12.1
- `oriterm_ui/src/widgets/settings_panel/tests.rs`
- `oriterm_ui/src/widgets/button/tests.rs`
- `oriterm/src/app/dialog_context/content_actions.rs`

### Semantic Actions

The dedicated footer widget should translate its internal button clicks into the existing semantic
actions:

- `SaveSettings`
- `CancelSettings`
- `ResetDefaults`

That keeps the dialog action dispatcher unchanged at the semantic layer and avoids leaking internal
button IDs outside the footer widget.

### Required Tests

Add footer-specific tests in the new footer widget's `tests.rs`:

- layout structure:
  - `fn footer_exists_only_in_right_column()` — footer exists only in the right content column, not spanning full panel width
  - `fn sidebar_remains_full_height_with_footer()` — sidebar remains full-height when footer is present
  - `fn unsaved_group_does_not_overlap_reset()` — unsaved group and Reset button do not overlap in layout
  - `fn footer_separator_above_buttons()` — top separator renders above the button row
- semantic actions:
  - `fn reset_button_emits_reset_defaults()` — Reset emits `ResetDefaults` action
  - `fn cancel_button_emits_cancel_settings()` — Cancel emits `CancelSettings` action
  - `fn save_button_emits_save_settings()` — Save emits `SaveSettings` action
- dirty-state behavior:
  - `fn unsaved_true_shows_indicator_enables_save()` — `SettingsUnsaved(true)` shows indicator and enables Save
  - `fn unsaved_false_hides_indicator_disables_save()` — `SettingsUnsaved(false)` hides indicator and disables Save
  - `fn reset_rebuild_reapplies_dirty_state()` — reset rebuild reapplies dirty state correctly so footer does not desync
  - `fn footer_widget_id_stable_across_rebuilds()` — footer widget IDs remain stable after dialog rebuild to prevent stale action routing
- shared button styling (in `oriterm_ui/src/widgets/button/tests.rs`):
  - `fn button_style_font_weight_affects_measurement()` — weight field affects text measurement
  - `fn button_style_letter_spacing_affects_width()` — letter spacing increases measured width
  - `fn button_style_disabled_primary_renders_at_reduced_opacity()` — disabled primary styling renders at reduced opacity

The current `settings_panel` tests only verify click-to-semantic mapping and “draws without panic.”
That is not enough for a footer whose current implementation has structural overlap and dirty-state
sync bugs.

### Checklist

- [ ] Keep semantic action variants stable
- [ ] Add layout tests that catch the current overlap/placement bug
- [ ] Add dirty-state tests for clean/dirty/reset transitions
- [ ] Expand shared button tests for typography and disabled-state styling

---

## 12.R Third Party Review Findings

### Resolved Findings

- `TPR-12-001` The draft treated the footer as a full-panel bottom bar, but the mockup footer lives
  only in the right content column while the sidebar remains full-height.
- `TPR-12-002` The draft's claimed button order was wrong. The mockup layout is `footer-left`
  unsaved group first, then `Reset`, `Cancel`, `Save` as a right-aligned cluster.
- `TPR-12-003` The current unsaved indicator is painted at the same left inset where the Reset
  button is laid out, so it can overlap the button. This must be fixed structurally, not by more
  paint offsets.
- `TPR-12-004` The draft stated all buttons use `font-weight: 500`, but the mockup's
  `.btn-primary` uses `700`.
- `TPR-12-005` The current button primitive cannot express the footer's required typography or
  disabled primary state because `ButtonStyle` lacks weight, tracking, and correct disabled border /
  opacity support.
- `TPR-12-006` `settings_panel/mod.rs` is already near the repository file-size limit, so adding
  more footer-specific logic there is not maintainable. Footer ownership should be extracted.
- `TPR-12-007` After `ResetDefaults`, the dialog title dirty marker is recomputed but the rebuilt
  panel is not sent `SettingsUnsaved(dirty)`, so footer dirty visuals can desynchronize from the
  real config state.

---

## 12.5 Build & Verify

### Gate

```bash
./build-all.sh
./clippy-all.sh
./test-all.sh
```

### Focused Verification

- run the targeted footer/settings-panel tests
- run button widget tests
- verify the live settings dialog in both clean and dirty states

Suggested commands:

```bash
cargo test -p oriterm_ui settings_panel::tests
cargo test -p oriterm_ui button::tests
cargo test -p oriterm dialog_context::content_actions
```

Manual verification checklist:

- [ ] Footer appears only in the right content column
- [ ] Sidebar remains full-height and visually continuous to the bottom
- [ ] Unsaved group appears on the left without overlapping buttons
- [ ] Reset, Cancel, and Save form a right-aligned cluster with correct spacing
- [ ] Save disables correctly when there are no unsaved changes
- [ ] Reset, Cancel, Save, and unsaved visuals match the mockup
