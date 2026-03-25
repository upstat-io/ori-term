---
section: "09"
title: "Settings Content Completeness"
status: complete
reviewed: true
third_party_review:
  status: resolved
  updated: 2026-03-25
goal: "The Appearance page matches the mockup's setting content, and the newly exposed controls for unfocused opacity, window decorations, and tab bar style drive real config-backed behavior across the shared settings form pipeline instead of dead UI"
depends_on: ["01", "02", "03", "05", "06", "07", "08"]
sections:
  - id: "09.1"
    title: "Appearance Page Content Surface"
    status: complete
  - id: "09.2"
    title: "Shared Settings Pipeline Integration"
    status: complete
  - id: "09.3"
    title: "Unfocused Window Opacity"
    status: complete
  - id: "09.4"
    title: "Window Decorations Behavior"
    status: complete
  - id: "09.5"
    title: "Tab Bar Style Behavior"
    status: complete
  - id: "09.6"
    title: "Tests"
    status: complete
  - id: "09.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "09.7"
    title: "Build & Verify"
    status: complete
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

- [x] Appearance page builds three sections instead of two
- [x] `Unfocused opacity` row is added to the `Window` section
- [x] `Decorations` section is added below `Window`
- [x] `SettingsIds` captures the three new control IDs
- [x] Form-builder ID inventory tests are updated

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

- [x] `handle_settings_action()` maps all three new controls into pending config changes
- [x] Dirty state changes when any new control diverges from the original config
- [x] Reset-to-defaults rebuilds the new controls correctly
- [x] Save applies the new fields through the normal settings-apply flow

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

- [x] Add `window.unfocused_opacity` to config with default and clamp helper
- [x] Use focus-aware effective opacity in window creation and apply paths
- [x] Update focus event handling to reapply window transparency on focus changes
- [x] Update redraw paths so frame opacity matches the focused/unfocused value
- [x] Add config tests for defaults, clamping, and round-trip
- [x] Add blur teardown path for opacity transitions <!-- TPR-09-010 -->
  - `apply_transparency()` early-returns for `opacity >= 1.0`, skipping any blur disable
  - Only `set_blur(true)` exists; zero `set_blur(false)` / `clear_vibrancy` / `clear_acrylic` calls
  - [x] Remove or restructure the `opacity >= 1.0` early return so blur state is always managed
  - [x] Add explicit blur disable: Linux `set_blur(false)`, Windows `clear_acrylic()`, macOS `clear_vibrancy()`
  - [x] `window-vibrancy` v0.7 has full disable support — no limitation to document
  - [x] Add regression test: `focus_changes_select_correct_opacity` + `blur_disabled_when_config_blur_false`

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

- [x] Main-window creation path uses `window.decorations`
- [x] Save/apply and config reload respond to decoration changes <!-- TPR-09-009 -->
  - `config_reload/mod.rs` `apply_window_changes()` only checks opacity/blur, ignores decorations entirely
  - [x] Add decoration change detection to `apply_window_changes()`
  - [x] Apply decoration changes to existing windows (`set_decorations(bool)` for decorated/frameless toggle; macOS titlebar modes logged as requiring restart)
- [x] Bool-only decoration plumbing is widened where necessary <!-- TPR-09-009 -->
  - `init/mod.rs` and `window_management.rs` collapse enum via `is_decorated()` to a bool
  - macOS hardcodes `with_decorations(true)` in `window/mod.rs`
  - [x] Carry `Decorations` enum through `oriterm_ui::window::WindowConfig` as `DecorationMode` enum instead of `decorated: bool`
  - [x] Map `Transparent` and `Buttonless` into platform-specific window attributes (macOS: transparent titlebar / hide traffic lights via `with_titlebar_buttons_hidden`; other platforms: equivalent to Frameless)
  - [x] Remove macOS `with_decorations(true)` hardcode; routed through `resolve_winit_decorations()` per-platform
- [x] Appearance dropdown reflects the supported decoration modes accurately
- [x] Decoration behavior stays separated between main windows and dialogs

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

- [x] Add a real tab-bar style enum/config field
- [x] Map the Appearance-page `Hidden` option onto the existing position model coherently
- [x] Replace hardcoded single-style tab-bar metrics with style-driven metrics <!-- TPR-09-008 -->
  - `TabBarMetrics` struct exists but `TabBarLayout::compute()` uses `TAB_MIN_WIDTH`, `TAB_MAX_WIDTH`, `TAB_PADDING` constants directly
  - [x] Update `TabBarLayout::compute()` to accept `&TabBarMetrics` and use its fields instead of constants
  - [x] Update all `recompute_layout()` call sites to pass the widget's metrics
  - [x] Update `max_text_width()` to use `metrics.tab_padding` instead of `TAB_PADDING` constant
- [x] Thread tab-bar style through app init, new-window creation, and settings apply <!-- TPR-09-008 -->
  - `set_metrics()` exists but has zero callers; `with_theme()` hardcodes `TabBarMetrics::DEFAULT`
  - [x] Build `TabBarMetrics` from `config.window.tab_bar_style` in `create_tab_bar_widget()`
  - [x] Call `set_metrics()` + trigger relayout when config reload changes `tab_bar_style`
  - [x] Call `set_metrics()` + trigger relayout on settings save/apply
- [x] Keep the Window page `Tab bar position` control functional and consistent
- [x] Thread `TabBarPosition::Hidden` through runtime to suppress tab bar <!-- TPR-09-007 -->
  - Config mutation works (action_handler maps Hidden correctly) but zero runtime consumers exist
  - [x] `compute_window_layout()` in `chrome/mod.rs`: skip tab bar height allocation when position is Hidden
  - [x] `cursor_in_tab_bar()` in `chrome/mod.rs`: return false when position is Hidden
  - [x] `update_tab_bar_hover()` in `chrome/mod.rs`: early return when position is Hidden
  - [x] Redraw path (`redraw/mod.rs`): skip `draw_tab_bar()` when position is Hidden
  - [x] Init path: thread position through so initial window respects Hidden
  - [x] Config reload/settings apply: update tab bar visibility on position change

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

- [x] Builder tests cover new IDs and counts
- [x] Action-handler tests cover the new controls
- [x] Config serde/default tests cover the new fields
- [x] App/tab-bar tests cover runtime behavior, not just config mutation <!-- TPR-09-007, TPR-09-008 -->
  - Only config-mutation tests exist (`tab_bar_style_hidden_maps_to_position_hidden`, `tab_bar_style_default_preserves_position`)
  - Missing: `focus_changes_select_correct_opacity`, `compact_tab_bar_metrics_differ_from_default`, `hidden_tab_bar_suppresses_layout`
  - [x] Add `hidden_tab_bar_suppresses_layout()` — prove Hidden position produces zero tab-bar height in layout
  - [x] Add `compact_tab_bar_metrics_differ_from_default()` — prove Compact metrics produce different dimensions
  - [x] Add `focus_changes_select_correct_opacity()` — prove focus state selects correct opacity value
  - [x] Add blur-teardown regression test (`blur_disabled_when_config_blur_false`)

---

## 09.R Third Party Review Findings

### Open Findings

- [x] `[TPR-09-011][medium]` `oriterm/src/app/settings_overlay/form_builder/appearance.rs:232` — The new Appearance decorations dropdown still cannot represent `Decorations::Buttonless`, even though Section 09 kept that runtime mode alive.
  Resolved 2026-03-25: accepted and fixed. Added conditional `Buttonless` option on macOS (4th dropdown item). On non-macOS, Buttonless maps to Transparent index since behavior is identical. Action handler maps index 3 → Buttonless on macOS. Added `decorations_dropdown_buttonless_on_macos` and `decorations_dropdown_transparent_roundtrip` tests.

- [x] `[TPR-09-012][low]` `oriterm_ui/src/widgets/tab_bar/widget/mod.rs:1` — Section 09 pushes `TabBarWidget` past the repository's 500-line source-file limit instead of splitting the style/runtime additions into submodules as the section itself required.
  Resolved 2026-03-25: accepted and fixed. Extracted animation/interaction lifecycle methods into `animation.rs` submodule (224 lines). `mod.rs` now 320 lines — well under 500.

- [x] `[TPR-09-013][low]` `oriterm/src/app/config_reload/mod.rs:1` — The Section 09 runtime wiring also pushes `config_reload/mod.rs` over the same hard 500-line limit.
  Resolved 2026-03-25: accepted and fixed. Extracted font config helpers (apply_font_config, resolve_hinting, resolve_subpixel_mode, rebuild_ui_font_sizes) into `font_config.rs` submodule (183 lines). `mod.rs` now 409 lines — well under 500.

- [x] `[TPR-09-007][high]` `oriterm/src/app/chrome/mod.rs:116` — `TabBarPosition` is still
  config-only, so the Appearance `Hidden` option and the Window-page `Tab bar position` control do
  not suppress chrome layout or hit testing.
  Resolved 2026-03-25: accepted. Concrete implementation tasks added to §09.5 checklist. The config
  mutation path works (action_handler correctly maps Hidden), but `compute_window_layout()`,
  `cursor_in_tab_bar()`, `update_tab_bar_hover()`, and the redraw path all unconditionally allocate
  and render the tab bar. `tab_bar_position` has zero consumers outside config/settings code.

- [x] `[TPR-09-008][high]` `oriterm/src/app/init/mod.rs:274` — `TabBarStyle::Compact` never
  reaches the runtime tab bar, so the new Appearance dropdown only mutates config state.
  Resolved 2026-03-25: accepted. Concrete implementation tasks added to §09.5 checklist.
  `with_theme()` hardcodes `TabBarMetrics::DEFAULT`, `set_metrics()` is dead code (zero callers),
  and `TabBarLayout::compute()` uses constants directly instead of the metrics struct. The
  infrastructure exists but nothing connects config → widget.

- [x] `[TPR-09-009][medium]` `oriterm/src/app/init/mod.rs:34` — Decorations support was not
  widened beyond a bool, so `Transparent`/`Buttonless` remain unrepresentable and saves/reloads do
  not update live windows.
  Resolved 2026-03-25: accepted. Concrete implementation tasks added to §09.4 checklist.
  `init/mod.rs` and `window_management.rs` collapse the enum via `is_decorated()`,
  `config_reload/mod.rs` ignores decorations entirely, and macOS hardcodes `with_decorations(true)`.
  The enum exists in config but only `Full` vs everything-else is representable at runtime.

- [x] `[TPR-09-010][medium]` `oriterm/src/app/event_loop.rs:154` — Focus-aware opacity can turn
  blur on for unfocused windows but has no path to turn it back off.
  Resolved 2026-03-25: accepted. Concrete implementation tasks added to §09.3 checklist.
  `apply_transparency()` has an early return for `opacity >= 1.0` that prevents any blur teardown,
  and the only `set_blur` call in the codebase is `set_blur(true)`. Zero occurrences of
  `set_blur(false)`, `clear_vibrancy`, or `clear_acrylic` exist anywhere.

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
