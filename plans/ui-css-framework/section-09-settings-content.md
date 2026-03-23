---
section: "09"
title: "Settings Content Completeness"
status: not-started
reviewed: true
third_party_review:
  status: resolved
  updated: 2026-03-23
goal: "The Appearance page matches the mockup's setting content, and the newly exposed controls for unfocused opacity, window decorations, and tab bar style drive real config-backed behavior across the shared settings form pipeline instead of dead UI"
depends_on: ["01", "02", "03", "05", "06", "07", "08"]
sections:
  - id: "09.1"
    title: "Appearance Page Content Surface"
    status: not-started
  - id: "09.2"
    title: "Shared Settings Pipeline Integration"
    status: not-started
  - id: "09.3"
    title: "Unfocused Window Opacity"
    status: not-started
  - id: "09.4"
    title: "Window Decorations Behavior"
    status: not-started
  - id: "09.5"
    title: "Tab Bar Style Behavior"
    status: not-started
  - id: "09.6"
    title: "Tests"
    status: not-started
  - id: "09.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "09.7"
    title: "Build & Verify"
    status: not-started
---

# Section 09: Settings Content Completeness

## Problem

The draft correctly noticed that the Appearance page is missing content from the mockup, but it
treated Section 09 as a small form-builder patch. The current tree shows the missing controls also
require real behavior work.

What the code actually has today:

- [mockups/settings-brutal.html](/home/eric/projects/ori_term/mockups/settings-brutal.html)
  shows three Appearance sections:
  `Theme`, `Window`, and `Decorations`.
- [appearance.rs](/home/eric/projects/ori_term/oriterm/src/app/settings_overlay/form_builder/appearance.rs)
  currently builds only `Theme` and `Window`, with just `Color scheme`, `Opacity`, and
  `Blur behind`.
- The mockup's Appearance page includes:
  `Unfocused opacity`, `Window decorations`, and `Tab bar style`.
- [form_builder/mod.rs](/home/eric/projects/ori_term/oriterm/src/app/settings_overlay/form_builder/mod.rs)
  already routes eight settings pages through a shared `SettingsIds` map and shared builder.
- [action_handler/mod.rs](/home/eric/projects/ori_term/oriterm/src/app/settings_overlay/action_handler/mod.rs)
  is the real pending-config update layer for settings controls.
- [dialog_context/content_actions.rs](/home/eric/projects/ori_term/oriterm/src/app/dialog_context/content_actions.rs)
  owns reset/rebuild, dirty tracking, and save application for the real dialog-window settings
  path.
- [config/mod.rs](/home/eric/projects/ori_term/oriterm/src/config/mod.rs) already has
  `window.decorations` and `window.tab_bar_position`, but it has no `window.unfocused_opacity`
  field and no `tab_bar_style` concept.
- `window.decorations` exists in config but is not used by startup window creation, extra-window
  creation, or config-apply paths today.
- [tab_bar/constants.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/tab_bar/constants.rs)
  and [tab_bar/widget/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/tab_bar/widget/mod.rs)
  hardcode one tab-bar geometry. There is no compact-vs-default variant system.

There is also a real model conflict the draft missed:

- the mockup's `Tab bar style` dropdown includes `Hidden`
- the existing Window page already exposes `Tab bar position = Top / Bottom / Hidden`

Section 09 therefore cannot just add a second hidden-state field. It needs an explicit model that
keeps style and position coherent.

## Corrected Scope

Section 09 should keep the full mockup goal intact, but fulfill it properly:

1. add the missing Appearance-page rows and Decorations section
2. wire them through the shared settings form/action pipeline
3. implement the runtime behavior those controls imply

That means Section 09 owns more than the page builder:

- `appearance.rs` and `SettingsIds` for the visible content
- `action_handler` and shared dialog rebuild flow for config updates
- focused/unfocused window opacity behavior in the app/window render path
- real decoration-mode application for main terminal windows
- a real tab-bar style system, not just a dead dropdown

This section should not fake completeness by adding UI that stores values no runtime code reads.

---

## 09.1 Appearance Page Content Surface

### Goal

Make the Appearance page content match the mockup's sections and control inventory.

### Files

- [appearance.rs](/home/eric/projects/ori_term/oriterm/src/app/settings_overlay/form_builder/appearance.rs)
- [form_builder/mod.rs](/home/eric/projects/ori_term/oriterm/src/app/settings_overlay/form_builder/mod.rs)
- [mockups/settings-brutal.html](/home/eric/projects/ori_term/mockups/settings-brutal.html)

### Current vs Required Structure

Current Appearance page:

- `Theme`
  - `Color scheme`
- `Window`
  - `Opacity`
  - `Blur behind`

Mockup Appearance page:

- `Theme`
  - `Color scheme`
- `Window`
  - `Opacity`
  - `Blur behind`
  - `Unfocused opacity`
- `Decorations`
  - `Window decorations`
  - `Tab bar style`

### Builder Changes

Update `build_page()` to construct three sections:

- `build_theme_section(...)`
- `build_window_section(...)`
- `build_decorations_section(...)`

Extend `build_window_section(...)` with:

- `Unfocused opacity`
  - slider
  - `30..=100`
  - `step = 1`
  - label text and description matched to the mockup meaning, not a guessed fallback string

Add `build_decorations_section(...)` with two dropdown-backed rows:

- `Window decorations`
- `Tab bar style`

### SettingsIds

Add concrete IDs for:

- `unfocused_opacity_slider`
- `decorations_dropdown`
- `tab_bar_style_dropdown`

Update the ID inventory tests in
[form_builder/tests.rs](/home/eric/projects/ori_term/oriterm/src/app/settings_overlay/form_builder/tests.rs)
so the fixed-count expectation matches the new controls.

### Checklist

- [ ] Appearance page builds three sections instead of two
- [ ] `Unfocused opacity` row is added to the `Window` section
- [ ] `Decorations` section is added below `Window`
- [ ] `SettingsIds` captures the three new control IDs
- [ ] Form-builder ID inventory tests are updated

---

## 09.2 Shared Settings Pipeline Integration

### Goal

Wire the new controls into the existing shared settings pipeline instead of inventing a second
save/reset path.

### Files

- [action_handler/mod.rs](/home/eric/projects/ori_term/oriterm/src/app/settings_overlay/action_handler/mod.rs)
- [dialog_context/content_actions.rs](/home/eric/projects/ori_term/oriterm/src/app/dialog_context/content_actions.rs)
- [dialog_management.rs](/home/eric/projects/ori_term/oriterm/src/app/dialog_management.rs)
- [keyboard_input/overlay_dispatch.rs](/home/eric/projects/ori_term/oriterm/src/app/keyboard_input/overlay_dispatch.rs)

### Correct Integration Boundary

The draft pointed at a generic "save handler" in the abstract, but the live implementation already
has a concrete split:

- `form_builder/*` creates widgets and IDs
- `settings_overlay::action_handler` maps widget actions into pending config edits
- `dialog_context/content_actions` handles dialog reset, rebuild, dirty state, and Save

Section 09 should extend that existing pipeline.

### Required Action Wiring

Add new action-handler branches for:

- `unfocused_opacity_slider` → pending config unfocused opacity
- `decorations_dropdown` → pending config decorations mode
- `tab_bar_style_dropdown` → pending config tab-bar style / hidden mapping

### Reset / Dirty / Rebuild

The real dialog reset path already rebuilds the full settings panel from `Config::default()`.
Section 09 should rely on that instead of hand-writing new reset logic per widget.

What must still be updated:

- dirty tracking must reflect the new fields because `pending_config != original_config`
- form rebuilds must preserve the current page and repopulate the new controls
- save application must trigger the new runtime behavior work from Sections 09.3-09.5

Because the form builder and action handler are shared, the overlay fallback path inherits the new
controls automatically. Do not fork the builder for dialogs vs overlays.

### Checklist

- [ ] `handle_settings_action()` maps all three new controls into pending config changes
- [ ] Dirty state changes when any new control diverges from the original config
- [ ] Reset-to-defaults rebuilds the new controls correctly
- [ ] Save applies the new fields through the normal settings-apply flow

---

## 09.3 Unfocused Window Opacity

### Goal

Implement the mockup's `Unfocused opacity` as real main-window behavior, not a dead config field or
an alias for pane dimming.

### Files

- [config/mod.rs](/home/eric/projects/ori_term/oriterm/src/config/mod.rs)
- [init/mod.rs](/home/eric/projects/ori_term/oriterm/src/app/init/mod.rs)
- [window_management.rs](/home/eric/projects/ori_term/oriterm/src/app/window_management.rs)
- [event_loop.rs](/home/eric/projects/ori_term/oriterm/src/app/event_loop.rs)
- [config_reload/mod.rs](/home/eric/projects/ori_term/oriterm/src/app/config_reload/mod.rs)
- [window/mod.rs](/home/eric/projects/ori_term/oriterm/src/window/mod.rs)
- [redraw/mod.rs](/home/eric/projects/ori_term/oriterm/src/app/redraw/mod.rs)
- [redraw/multi_pane/mod.rs](/home/eric/projects/ori_term/oriterm/src/app/redraw/multi_pane/mod.rs)

### Data Model

Add:

```rust
pub unfocused_opacity: f32
```

to `WindowConfig`, with:

- default `1.0`
- clamped effective value in `0.3..=1.0`
- serde round-trip coverage

Do not reuse:

- `window.opacity`
- `pane.inactive_opacity`

Those are different features.

### Runtime Model

Add a helper at the window-config boundary for "effective opacity given focus state", then use it
consistently in both:

- OS/window transparency application
- frame/render opacity setup

The implementation should treat main-window focus as the input signal:

- focused main window → `window.opacity`
- unfocused main window → `window.unfocused_opacity`

Dialog windows should keep their existing opacity behavior; this setting is for terminal windows.

### Apply Points

Section 09 should thread the focused/unfocused opacity through:

- initial window creation
- new-window creation
- config save/apply
- config reload
- focus-in / focus-out handling
- frame palette opacity in single-pane and multi-pane redraw

That removes the current single-opacity assumption spread across init, redraw, and config-reload
code.

### Checklist

- [ ] Add `window.unfocused_opacity` to config with default and clamp helper
- [ ] Use focus-aware effective opacity in window creation and apply paths
- [ ] Update focus event handling to reapply window transparency on focus changes
- [ ] Update redraw paths so frame opacity matches the focused/unfocused value
- [ ] Add config tests for defaults, clamping, and round-trip

---

## 09.4 Window Decorations Behavior

### Goal

Turn the existing `window.decorations` config enum into real main-window behavior and expose it on
the Appearance page.

### Files

- [config/mod.rs](/home/eric/projects/ori_term/oriterm/src/config/mod.rs)
- [init/mod.rs](/home/eric/projects/ori_term/oriterm/src/app/init/mod.rs)
- [window_management.rs](/home/eric/projects/ori_term/oriterm/src/app/window_management.rs)
- [config_reload/mod.rs](/home/eric/projects/ori_term/oriterm/src/app/config_reload/mod.rs)
- [window_manager/types.rs](/home/eric/projects/ori_term/oriterm/src/window_manager/types.rs)
- [window_manager/platform/mod.rs](/home/eric/projects/ori_term/oriterm/src/window_manager/platform/mod.rs)
- [oriterm_ui/src/window/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/window/mod.rs)

### Current State

`Decorations` already exists in config with:

- `None`
- `Full`
- `Transparent`
- `Buttonless`

But current main-window creation only feeds a boolean decoration concept through
`oriterm_ui::window::WindowConfig`, and the current settings apply/reload path does not use the
enum at all.

### Required Runtime Work

Section 09 should promote decorations to a real application/window contract.

That likely requires:

- widening the app/window configuration surface beyond a bool-only decorations flag
- mapping `Decorations` variants into platform-aware creation/apply behavior
- ensuring main windows use the configured mode at startup and when new windows are created
- applying saved/reloaded decoration changes to existing windows in place where possible, and
  recreating windows when a platform cannot safely mutate the mode live

Dialog windows should keep their dialog-specific chrome rules; do not accidentally apply main-window
decoration settings to the settings dialog itself.

### Dropdown Values

The Appearance-page dropdown should at minimum cover the mockup's visible values:

- `None (frameless)`
- `Full`
- `Transparent`

If Section 09 keeps `Buttonless` as a supported runtime mode, expose it conditionally where that
platform behavior is meaningful rather than leaving it config-only.

### Checklist

- [ ] Main-window creation path uses `window.decorations`
- [ ] Save/apply and config reload respond to decoration changes
- [ ] Bool-only decoration plumbing is widened where necessary
- [ ] Appearance dropdown reflects the supported decoration modes accurately
- [ ] Decoration behavior stays separated between main windows and dialogs

---

## 09.5 Tab Bar Style Behavior

> **HIGH COMPLEXITY WARNING**: This subsection touches the tab bar layout, rendering, hit testing, and platform interactive-rect calculation. The existing `tab_bar/widget/mod.rs` is already at 497 lines, so a file split is mandatory before adding style-variant logic. The model correction (style vs position overlap) also requires careful testing of all style/position combinations.

### Goal

Implement the mockup's `Tab bar style` as a real tab-bar variant system while resolving the overlap
with the existing `Tab bar position` setting.

### Files

- [config/mod.rs](/home/eric/projects/ori_term/oriterm/src/config/mod.rs)
- [appearance.rs](/home/eric/projects/ori_term/oriterm/src/app/settings_overlay/form_builder/appearance.rs)
- [window.rs](/home/eric/projects/ori_term/oriterm/src/app/settings_overlay/form_builder/window.rs)
- [action_handler/mod.rs](/home/eric/projects/ori_term/oriterm/src/app/settings_overlay/action_handler/mod.rs)
- [tab_bar/constants.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/tab_bar/constants.rs)
- [tab_bar/widget/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/tab_bar/widget/mod.rs)
- [tab_bar/layout.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/tab_bar/layout.rs)
- [chrome/mod.rs](/home/eric/projects/ori_term/oriterm/src/app/chrome/mod.rs)
- [init/mod.rs](/home/eric/projects/ori_term/oriterm/src/app/init/mod.rs)
- [window_management.rs](/home/eric/projects/ori_term/oriterm/src/app/window_management.rs)

### Model Correction

The draft proposed a new `TabBarStyle` with only `Default` and `Compact`, but the mockup dropdown
actually shows:

- `Default`
- `Compact`
- `Hidden`

At the same time, the existing Window page already exposes:

- `Top`
- `Bottom`
- `Hidden`

Section 09 must not introduce a second unrelated hidden state.

### Recommended State Model

Use:

- a real style enum for visual density, for example `Default | Compact`
- the existing `TabBarPosition` for placement and hidden state

Then map the Appearance-page dropdown like this:

- `Default` → style `Default`, preserve current non-hidden position
- `Compact` → style `Compact`, preserve current non-hidden position
- `Hidden` → set `tab_bar_position = Hidden`

This keeps the mockup surface while avoiding duplicate visibility state.

### Runtime Tab-Bar Work

`Compact` cannot be a no-op. The current tab bar hardcodes one geometry through constants like:

- `TAB_BAR_HEIGHT`
- `TAB_PADDING`
- `CLOSE_BUTTON_WIDTH`

Section 09 should replace the one-size-fits-all geometry with a style-driven metrics object used by:

- top-level window layout
- tab-bar layout and hit testing
- tab-bar drawing
- platform interactive rect calculation

Because [tab_bar/widget/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/tab_bar/widget/mod.rs)
is already 497 lines, do not pile style-variant logic into that file. Extract a dedicated
tab-bar metrics/style module first so this stays within the repository's file-size rule.

### Checklist

- [ ] Add a real tab-bar style enum/config field
- [ ] Map the Appearance-page `Hidden` option onto the existing position model coherently
- [ ] Replace hardcoded single-style tab-bar metrics with style-driven metrics
- [ ] Thread tab-bar style through app init, new-window creation, and settings apply
- [ ] Keep the Window page `Tab bar position` control functional and consistent

---

## 09.6 Tests

### Goal

Cover the new controls at the builder, action, config, and runtime-behavior layers.

### Required Coverage

In [form_builder/tests.rs](/home/eric/projects/ori_term/oriterm/src/app/settings_overlay/form_builder/tests.rs):

- Appearance page IDs include the three new controls
- total distinct-ID count is updated

In [action_handler/tests.rs](/home/eric/projects/ori_term/oriterm/src/app/settings_overlay/action_handler/tests.rs):

- `unfocused_opacity_slider` updates pending config
- `decorations_dropdown` updates pending config
- `tab_bar_style_dropdown` updates config state and the `Hidden` mapping correctly

In [config/tests.rs](/home/eric/projects/ori_term/oriterm/src/config/tests.rs):

- `fn unfocused_opacity_default_is_one()` — defaults to `1.0`
- `fn unfocused_opacity_clamps_low()` — values below `0.3` clamp to `0.3`
- `fn unfocused_opacity_clamps_high()` — values above `1.0` clamp to `1.0`
- `fn unfocused_opacity_at_boundaries()` — `0.3` and `1.0` pass through unchanged
- `fn unfocused_opacity_serde_roundtrip()` — serialize/deserialize preserves the value
- `fn tab_bar_style_default()` — defaults to `Default` variant
- `fn tab_bar_style_serde_roundtrip()` — serialize/deserialize preserves the variant
- `fn decorations_serde_roundtrip()` — existing + any new decoration modes round-trip correctly

In the relevant app/tab-bar tests:

- `fn focus_changes_select_correct_opacity()` — focus changes produce the expected focused/unfocused opacity choice
- `fn compact_tab_bar_metrics_differ_from_default()` — compact tab-bar metrics differ from default metrics in the intended way
- `fn hidden_tab_bar_suppresses_layout()` — hidden tab bar still suppresses layout/rendering through the existing position path
- `fn tab_bar_style_hidden_maps_to_position_hidden()` — selecting `Hidden` in the style dropdown maps to `tab_bar_position = Hidden`
- `fn tab_bar_style_default_preserves_position()` — selecting `Default` style preserves the current non-hidden position

### Checklist

- [ ] Builder tests cover new IDs and counts
- [ ] Action-handler tests cover the new controls
- [ ] Config serde/default tests cover the new fields
- [ ] App/tab-bar tests cover runtime behavior, not just config mutation

---

## 09.R Third Party Review Findings

### Resolved Findings

- `TPR-09-001` The draft treated Section 09 as a form-only patch, but the missing controls imply
  real runtime work: focus-aware opacity, actual decoration application, and tab-bar style
  behavior.
- `TPR-09-002` The draft mislocated the integration points. The live settings flow is shared across
  `form_builder`, `settings_overlay::action_handler`, and
  [dialog_context/content_actions.rs](/home/eric/projects/ori_term/oriterm/src/app/dialog_context/content_actions.rs),
  not a single abstract save handler.
- `TPR-09-003` `window.decorations` already exists in config, but it is currently unused by window
  creation and config-apply code. Section 09 must implement behavior, not duplicate config.
- `TPR-09-004` The draft's `Tab bar style = Default | Compact` plan was inaccurate. The mockup
  includes `Hidden`, and that overlap with `tab_bar_position = Hidden` must be resolved explicitly.
- `TPR-09-005` The current tab bar has no style-variant system and hardcodes one geometry through
  constants. A live `Compact` mode requires tab-bar metrics/layout/render changes, not just a new
  enum.
- `TPR-09-006` [tab_bar/widget/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/tab_bar/widget/mod.rs)
  is already near the file-size limit, so Section 09 must split style/metrics work into submodules
  instead of expanding that file further.

---

## 09.7 Build & Verify

After implementation:

- run the full repository verification gate required by the repo rules
- confirm the Appearance page now shows `Theme`, `Window`, and `Decorations`
- confirm the new controls update pending settings, reset correctly, and save correctly
- verify unfocused main windows use the configured opacity
- verify decoration mode changes and tab-bar style changes produce visible runtime behavior

Required commands:

```bash
./build-all.sh
./clippy-all.sh
./test-all.sh
```
