---
section: "10"
title: "Settings Panel Rebuild"
status: in-progress
goal: "Settings dialog matches mockups/settings.html — sidebar navigation, 8 pages, all controls wired to config"
inspired_by:
  - "mockups/settings.html — the design spec"
  - "Windows Terminal settings UI"
  - "Ghostty Config app"
depends_on: ["09"]
reviewed: true
sections:
  - id: "10.0"
    title: "Dual Rendering Mode: Overlay vs Dialog Window"
    status: not-started
  - id: "10.0a"
    title: "Legacy Form Builder Cleanup"
    status: not-started
  - id: "10.0b"
    title: "Dependency Ordering: Config Types Before Page Builders"
    status: complete
  - id: "10.1"
    title: "Dialog Layout & SettingsPanel Restructuring"
    status: not-started
  - id: "10.2"
    title: "Appearance Page"
    status: not-started
  - id: "10.3"
    title: "Colors Page"
    status: not-started
  - id: "10.4"
    title: "Font Page"
    status: not-started
  - id: "10.5"
    title: "Terminal Page"
    status: not-started
  - id: "10.6"
    title: "Keybindings Page"
    status: not-started
  - id: "10.7"
    title: "Window, Bell, Rendering Pages"
    status: not-started
  - id: "10.8"
    title: "Action Wiring & Config Persistence"
    status: not-started
  - id: "10.9"
    title: "Completion Checklist"
    status: not-started
---

# Section 10: Settings Panel Rebuild

**Status:** Not Started
**Goal:** The settings dialog is a 1:1 match of `mockups/settings.html`: left sidebar
navigation with 8 pages (Appearance, Colors, Font, Terminal, Keybindings, Window, Bell,
Rendering), each page fully wired to config read/write/save.

**Context:** The current settings panel has 5 sections (Appearance, Font, Behavior, Terminal,
Bell) in a single scrollable `FormLayout`. It can render as either a modal overlay
(`oriterm/src/app/settings_overlay/`) or a separate dialog window
(`oriterm/src/app/dialog_context/`). The form builder in
`settings_overlay/form_builder/mod.rs` produces a `FormLayout` + `SettingsIds` (10 fields),
and `settings_overlay/action_handler/mod.rs` dispatches widget actions to config updates.
The rebuild replaces this with a sidebar + pages layout using all the new widgets from
Section 09, expanding to 8 pages and 24 control IDs.

**Depends on:** Section 09 (New Widget Library — all widgets must exist).

---

## 10.0 Dual Rendering Mode: Overlay vs Dialog Window

The settings panel currently supports TWO rendering modes and both must continue to work
after the rebuild:

1. **Modal overlay** (`oriterm/src/app/settings_overlay/mod.rs`): `open_settings_overlay()`
   pushes a `SettingsPanel` into the focused terminal window's `OverlayManager`. The overlay
   path stores `settings_pending: Option<Config>` and `settings_ids: Option<SettingsIds>` on
   `App`, and routes actions through `try_dispatch_settings_action()` in
   `oriterm/src/app/keyboard_input/overlay_dispatch.rs`.

2. **Separate dialog window** (`oriterm/src/app/dialog_context/`): Creates a real OS window
   with its own GPU surface. `DialogContent::Settings` stores `panel: Box<SettingsPanel>`,
   `ids: SettingsIds`, `pending_config: Box<Config>`, and `original_config: Box<Config>`.
   Actions route through `handle_dialog_content_action()` in
   `oriterm/src/app/dialog_context/content_actions.rs`.

Both paths call into the same `handle_settings_action()` dispatcher and the same
`SettingsPanel` widget. The rebuild must:

- [ ] Update both `open_settings_overlay()` and the dialog window creation path (in
  `oriterm/src/app/dialog_management.rs` or equivalent) to call the new
  `build_settings_dialog()` with the `&UiTheme` parameter.
- [ ] Ensure `SettingsPanel::new()` (overlay) and `SettingsPanel::embedded()` (dialog) both
  accept the new sidebar + pages layout instead of a `FormLayout`.
- [ ] **Overlay dispatch path needs NO routing changes.** `try_dispatch_settings_action()` in
  `overlay_dispatch.rs` is called BEFORE the match block in `handle_overlay_result()`, so ALL
  delivered actions already pass through `handle_settings_action()`. Only the handler itself
  (in `action_handler/mod.rs`) needs new `ValueChanged`/`TextChanged` match arms.
- [ ] Keep the `#[allow(dead_code)]` on `open_settings_overlay()` — it is retained as
  fallback and must compile even if not the primary path.

---

## 10.0a Legacy Form Builder Cleanup

The existing form builder code must be replaced, not merely extended:

- [ ] **Delete** the existing section builder functions: `build_appearance_section()`,
  `build_font_section()`, `build_behavior_section()`, `build_terminal_section()`,
  `build_bell_section()` from `form_builder/mod.rs`.
- [ ] **Delete** the `OPACITY_VALUES`, `FONT_SIZE_VALUES`, `BELL_DURATION_VALUES` constants
  from `form_builder/mod.rs` (opacity moves to a RangeSlider, font size to a NumberInput;
  bell duration values may be kept if the dropdown is retained).
- [ ] **Rename** `build_settings_form()` to `build_settings_dialog()` (signature changes to
  accept `&UiTheme`).
- [ ] **Replace** the old `SettingsIds` struct (10 fields) with the new one (24 fields
  including `reset_defaults_id`).
- [ ] **Update** existing test file `form_builder/tests.rs` — all tests referencing
  `build_settings_form()`, 5 sections, 10 rows, and `collect_ids()` must be rewritten to
  match the new 8-page sidebar structure.
- [ ] **Update** `action_handler/tests.rs` — the `default_ids()` helper and all tests that
  construct `SettingsIds` via `build_settings_form()` must use the new builder and expanded
  IDs.

---

## 10.0b Dependency Ordering: Config Types Before Page Builders

**CRITICAL**: The new config types (`TabBarPosition`, `GpuBackend`, `RenderingConfig`) and new
`Config` fields (`font.line_height`, `window.tab_bar_position`, `window.grid_padding`,
`window.restore_session`, etc.) are referenced by the page builders in 10.4-10.7. They must
be defined BEFORE implementing the page builders, not in 10.8 where they currently appear.

Implementation order:
1. Add `PartialEq` derives to all config structs (needed for dirty detection in 10.1).
2. Define `TabBarPosition`, `GpuBackend`, `RenderingConfig` types in `oriterm/src/config/`.
3. Add new fields to `Config` structs with `#[serde(default)]`.
4. Add TOML round-trip tests in `oriterm/src/config/tests.rs`.
5. THEN implement page builders (10.2-10.7) that reference these fields.
6. THEN wire up action handlers (10.8).

- [x] Define `TabBarPosition`, `GpuBackend`, `RenderingConfig` types and add new `Config`
  fields before starting page builders (10.2-10.7).
- [x] Add `PartialEq` derives to `Config` and all nested config structs.
- [x] Add TOML round-trip tests for all new config types in `oriterm/src/config/tests.rs`.

---

## 10.1 Dialog Layout & SettingsPanel Restructuring

**File(s):** `oriterm/src/app/settings_overlay/form_builder/mod.rs` (dialog skeleton),
per-page submodules: `form_builder/appearance.rs`, `form_builder/colors.rs`,
`form_builder/font.rs`, `form_builder/terminal.rs`, `form_builder/keybindings.rs`,
`form_builder/window.rs`, `form_builder/bell.rs`, `form_builder/rendering.rs`.

**WARNING**: The current `form_builder/mod.rs` is 224 lines. Adding 8 page builder
functions inline would push it well past 500 lines. Split page builders into submodules.
Recommended: `form_builder/mod.rs` keeps the top-level `build_settings_dialog()` function
and the `SettingsIds` struct. Each page builder lives in its own file. If grouping, combine
the 3 smaller pages (window/bell/rendering) into `form_builder/advanced_pages.rs`.

**Test organization**: Per `test-organization.md`, each page builder submodule that has tests
needs its own sibling `tests.rs` file. For example, `form_builder/appearance.rs` becomes
`form_builder/appearance/mod.rs` + `form_builder/appearance/tests.rs` if it has tests.
Alternatively, keep page builder tests consolidated in `form_builder/tests.rs` (testing via
the top-level `build_settings_dialog()` function), which avoids the submodule proliferation.

Replace the current single-column form with sidebar + pages.

### Page Builder Function Signatures

Each page builder function lives in its own submodule and returns a `Box<dyn Widget>` (the
page content widget). Control `WidgetId`s are captured via the `ids` parameter. The pattern:

```rust
/// Builds the Appearance page content widget.
/// The page is wrapped in a ScrollWidget::vertical() so content taller than the
/// viewport scrolls. The scroll widget uses SizeSpec::Fill for height.
pub(super) fn build_appearance_page(
    config: &Config,
    ids: &mut SettingsIds,
) -> Box<dyn Widget>
```

The `ids: &mut SettingsIds` pattern avoids returning a tuple of IDs per page — instead, each
builder writes its control IDs directly into the pre-allocated struct fields. This requires
`SettingsIds` to be initialized with `WidgetId::placeholder()` before calling the builders.

**Recommended approach**: Initialize `SettingsIds` with placeholder IDs, then each page
builder overwrites the relevant fields. See the `build_settings_dialog()` code sample below
for the orchestration pattern.

**WARNING: `WidgetId` has no `Default` impl.** `WidgetId(u64)` does not derive `Default`
and has no `ZERO` or `PLACEHOLDER` constant. Add a `WidgetId::placeholder()` constructor
that returns `WidgetId(0)` (never matches real IDs since the counter starts at 1) to
`oriterm_ui/src/widget_id.rs`. Then implement `SettingsIds::placeholder()` using it.

- [ ] Add `WidgetId::placeholder()` constructor returning `WidgetId(0)` to
  `oriterm_ui/src/widget_id.rs`.
- [ ] Implement `build_settings_dialog()` replacing `build_settings_form()`:
  ```rust
  pub fn build_settings_dialog(config: &Config, theme: &UiTheme) -> (SettingsLayout, SettingsIds) {
      let sidebar = SidebarNavWidget::new(vec![
          NavSection {
              title: "General".into(),
              items: vec![
                  NavItem { label: "Appearance".into(), icon: Some(IconId::Sun), page_index: 0 },
                  NavItem { label: "Colors".into(), icon: Some(IconId::Palette), page_index: 1 },
                  NavItem { label: "Font".into(), icon: Some(IconId::Type), page_index: 2 },
                  NavItem { label: "Terminal".into(), icon: Some(IconId::Terminal), page_index: 3 },
                  NavItem { label: "Keybindings".into(), icon: Some(IconId::Keyboard), page_index: 4 },
                  NavItem { label: "Window".into(), icon: Some(IconId::Window), page_index: 5 },
              ],
          },
          NavSection {
              title: "Advanced".into(),
              items: vec![
                  NavItem { label: "Bell".into(), icon: Some(IconId::Bell), page_index: 6 },
                  NavItem { label: "Rendering".into(), icon: Some(IconId::Activity), page_index: 7 },
              ],
          },
      ], theme);

      let mut ids = SettingsIds::placeholder();
      let pages = PageContainerWidget::new(vec![
          appearance::build_page(config, &mut ids),
          colors::build_page(config, &mut ids),
          font::build_page(config, &mut ids),
          terminal::build_page(config, &mut ids),
          keybindings::build_page(config, &mut ids),
          window::build_page(config, &mut ids),
          bell::build_page(config, &mut ids),
          rendering::build_page(config, &mut ids),
      ]);

      // Top-level layout: sidebar (fixed 200px) | content (fill)
      // Footer: Reset to Defaults | Cancel | Save
  }
  ```
- [ ] Update dialog window size from `(720, 560)` to `(860, 620)` in
  `DialogKind::default_size()` in `oriterm/src/window_manager/types.rs`. If overlay mode is
  used, update overlay centering bounds. Update the private `PANEL_WIDTH` constant (currently
  `600.0`) in `oriterm_ui/src/widgets/settings_panel/mod.rs` to `660.0` (860 - 200 sidebar).
- [ ] Add footer buttons: "Reset to Defaults" (ghost), "Cancel" (ghost), "Save"
  (primary/accent).
- [ ] Add footer separator line above buttons.
- [ ] Add `WidgetAction::ResetDefaults` variant to `WidgetAction` enum in
  `oriterm_ui/src/action.rs`. The action handler creates a `Config::default()` and applies
  it. Add the reset button ID to `SettingsIds`.
  **WARNING: Cross-crate enum change.** Adding a variant to `WidgetAction` breaks every
  exhaustive match across both `oriterm_ui` and `oriterm`. An alternative is to have
  `SettingsPanel::on_action()` translate `Clicked(reset_id)` into `ResetDefaults` (like it
  already does for `Clicked(save_id) -> SaveSettings`). Either way, the variant must exist.
  The plan recommends adding the variant for consistency with `SaveSettings`/`CancelSettings`.

### `WidgetAction::ResetDefaults` Sync Points

Adding `ResetDefaults` to `WidgetAction` is a cross-crate enum change. Every exhaustive
`match` on `WidgetAction` must add the new variant. Known sites:

1. `oriterm/src/app/dialog_context/content_actions.rs` `handle_dialog_content_action()` —
   add `WidgetAction::ResetDefaults` arm that calls a new `reset_dialog_settings()` method.
2. `oriterm/src/app/keyboard_input/overlay_dispatch.rs` `handle_overlay_result()` — add
   `WidgetAction::ResetDefaults` arm in the Delivered match (parallel to `SaveSettings`).
3. `oriterm/src/app/chrome/mod.rs` — already uses `_ => false` wildcard catch-all, so
   `ResetDefaults` is handled automatically. **No change needed.**
4. `oriterm_ui/src/widgets/settings_panel/mod.rs` `on_action()` — add mapping from
   `Clicked(reset_id)` to `WidgetAction::ResetDefaults`.

The compiler will catch any missed exhaustive matches, but listing them here avoids surprise.

### Scroll Behavior for Content Pages

Several pages will exceed the 620px - header - footer content area:
- **Colors page**: SchemeCard grid (4+ cards at 120px each) + palette editor (~200px) =
  easily 600+ pixels of content.
- **Font page**: CodePreview (~120px) + 5 SettingRows (~220px) fits but is tight.
- **Keybindings page**: 12 KeybindRows at ~44px each = 528px, close to limit.
- **Window page**: 5 SettingRows in 3 sections, fits comfortably.

Each page builder wraps its content in `ScrollWidget::vertical()` with `SizeSpec::Fill`
height so the scroll widget takes the remaining space in the content area. The sidebar is
NOT scrollable (8 items fit in 620px). The `PageContainerWidget` should pass its full
bounds to the active page so the scroll widget knows its viewport height.

- [ ] Each page builder function wraps its content in `ScrollWidget::vertical()`.
- [ ] Fix `PageContainerWidget::layout()` to use `SizeSpec::Fill` (not `Hug`) for the
  active page so the scroll widget receives a bounded viewport, not infinity.
  **Current issue**: `PageContainerWidget::layout()` returns a `LayoutBox::leaf(w, h)` where
  `(w, h)` is computed from `active_page_size()` using unconstrained layout. This means the
  page container reports its natural (hugged) size, which breaks scroll — the scroll widget
  never gets a finite viewport to constrain against. Fix: return `SizeSpec::Fill` for both
  width and height, and include the active page's `LayoutBox` as a child so the solver
  constrains it.
- [ ] Test: Colors page with 10+ SchemeCards scrolls smoothly with scrollbar visible.

### Keyboard Navigation: Sidebar <-> Pages

The settings dialog must be keyboard-navigable:

- [ ] **Tab** cycles focus through all focusable controls on the active page (dropdowns,
  toggles, sliders, number inputs, text inputs) via `FocusController` and
  `FocusManager::focus_next()`/`focus_prev()`.
- [ ] **Arrow Up/Down** in the sidebar switches the active nav item. Add
  `KeyDown { key: ArrowUp/ArrowDown }` handling to `SidebarNavWidget::on_input()` (currently
  only handles `MouseDown`).
- [ ] Define sidebar-to-content focus transition: either **Ctrl+Tab** switches between
  sidebar and content focus zones, or Tab wraps from last control on current page into the
  sidebar, then back to first control on the next page. Choose one and implement it.
- [ ] Make the sidebar focusable: either make individual nav items focusable or make the
  sidebar focusable as a unit that accepts arrow key navigation (it has `sense: click` but
  no `is_focusable` override).

### Dirty State Tracking (Unsaved Changes Indicator)

The `DialogContent::Settings` variant already stores `original_config: Box<Config>` (marked
`dead_code` with reason "reserved for dirty detection"). Wire this up:

- [ ] After each settings action, compare `pending_config` with `original_config`. If they
  differ, the dialog is "dirty".
- [ ] Show a visual indicator when dirty: either a dot/asterisk in the dialog title bar, or
  enable/disable the Save button based on dirty state.
- [ ] **Cancel with unsaved changes**: When the user clicks Cancel or closes the dialog while
  dirty, simply discard silently for the initial implementation. Document this as a known
  limitation; a confirmation prompt ("Discard unsaved changes?") can be added later.
- [ ] `Config` must derive `PartialEq` for dirty comparison. Currently it derives `Clone`
  and `Debug` but NOT `PartialEq`. Add `#[derive(PartialEq)]` to `Config` and all nested
  config structs that lack it. Structs already deriving `PartialEq`: `FontConfig`,
  `BellConfig`, `ColorConfig`, `PasteWarning`, `ProcessModel`, `CursorStyle`, `Decorations`.
  Structs MISSING `PartialEq` (must add): `Config`, `TerminalConfig`, `WindowConfig`,
  `BehaviorConfig`, `PaneConfig`, `KeybindConfig`. **Note**: `PaneConfig` has `f32` fields
  (`divider_px`, `inactive_opacity`), so `PartialEq` derive uses `f32::eq` (bitwise
  NaN != NaN). This is acceptable for dirty detection since NaN values are clamped on read.

---

### `SettingsPanel` Widget Restructuring

`SettingsPanel` (`oriterm_ui/src/widgets/settings_panel/mod.rs`) currently takes a
`FormLayout` in its constructor and wraps it in a `ScrollWidget` + footer. It must be
refactored to accept the new sidebar + PageContainerWidget layout.

**WARNING: 500-line limit.** `settings_panel/mod.rs` is currently 387 lines. Adding sidebar
handling, horizontal row layout, and new constructor signatures will likely push it past 500.
Extract the `paint()` method's chrome/footer background drawing logic into
`settings_panel/paint_helpers.rs`, or extract the footer construction into
`settings_panel/footer.rs`.

- [ ] Change `SettingsPanel::new(form: FormLayout)` to
  `SettingsPanel::new(sidebar: SidebarNavWidget, pages: PageContainerWidget)` (or accept a
  single pre-built container widget).
- [ ] Change internal container structure from
  `[header?, scroll(FormLayout), separator, footer]` to
  `[header?, horizontal_row(sidebar | scroll(pages)), separator, footer]`.
  The sidebar is fixed-width (200px), the page content fills remaining width and is
  scrollable.
- [ ] `SettingsPanel::embedded()` must also accept the new parameters.
- [ ] `accept_action()` must propagate to both the sidebar and the PageContainerWidget. When
  the sidebar emits `Selected { index }`, the PageContainerWidget switches pages. The
  `SettingsPanel` must forward the action to the PageContainerWidget.
- [ ] `for_each_child_mut()` must visit both sidebar and PageContainerWidget children for
  widget registration and focus collection.
- [ ] Update `focusable_children()` to include sidebar (if made focusable) and active
  page controls.
- [ ] Update `SettingsPanel` tests in `settings_panel/tests.rs`.

---

## 10.2 Appearance Page

- [ ] Content header: "Appearance" title + description text
- [ ] Theme section:
  - SettingRow: "Color scheme" dropdown (all built-in schemes)
- [ ] Window section:
  - SettingRow: "Opacity" RangeSlider (30-100%)
  - SettingRow: "Blur behind" Toggle

---

## 10.3 Colors Page

- [ ] Content header: "Colors" title + description text
- [ ] Schemes section:
  - SchemeCard grid (AutoFill 240px) showing all built-in color schemes
  - Each card: terminal preview + swatch bar
  - Active card: accent border + "Active" StatusBadge
- [ ] Palette editor (below card grid):
  - Title: "Palette — {scheme name}"
  - Special colors row: Foreground, Background, Cursor, Selection (4-column grid of
    SpecialColorSwatches)
  - Normal ANSI colors: 8-column ColorSwatchGrid (colors 0-7)
  - Bright ANSI colors: 8-column ColorSwatchGrid (colors 8-15)

---

## 10.4 Font Page

- [ ] CodePreview widget: syntax-highlighted Rust code with current font settings
- [ ] Typeface section:
  - SettingRow: "Font family" dropdown
  - SettingRow: "Size" NumberInput (8-32, step 0.5)
  - SettingRow: "Weight" dropdown (300-700)
- [ ] Features section:
  - SettingRow: "Ligatures" Toggle
  - SettingRow: "Line height" NumberInput (0.8-2.0, step 0.05)

---

## 10.5 Terminal Page

- [ ] Content header: "Terminal" title + description text
- [ ] Cursor section:
  - CursorPicker: 3 visual options (Block, Bar, Underline)
  - SettingRow: "Cursor blink" Toggle
- [ ] Scrollback section:
  - SettingRow: "Maximum lines" NumberInput (0-100000, step 1000)
- [ ] Shell section:
  - SettingRow: "Default shell" TextInput (monospace font)
  - SettingRow: "Paste warning" dropdown (Always/Never)

---

## 10.6 Keybindings Page

- [ ] Content header: "Keybindings" title + description text
- [ ] Tabs & Panes section:
  - KeybindRows: New tab, Close tab, Split vertically, Split horizontally,
    Next tab, Previous tab
- [ ] Clipboard section:
  - KeybindRows: Copy, Paste
- [ ] Navigation section:
  - KeybindRows: Scroll up, Scroll down, Search, Settings

---

## 10.7 Window, Bell, Rendering Pages

- [ ] **Window page**:
  - Chrome section: "Tab bar position" dropdown (Top/Bottom/Hidden)
  - Padding section: "Grid padding" NumberInput (0-40, step 2)
  - Startup section: "Restore previous session" Toggle,
    "Initial columns" NumberInput, "Initial rows" NumberInput

- [ ] **Bell page**:
  - Visual Bell section: "Animation" dropdown (Ease Out/Linear/None),
    "Duration" dropdown (Off/50ms/150ms/300ms/500ms)

- [ ] **Rendering page**:
  - GPU section: "Backend" dropdown (Auto/Vulkan/DirectX 12/Metal).
    Show "restart required" label — GPU backend changes apply on next launch only.
  - Text section: "LCD subpixel rendering" Toggle (maps to `font.subpixel_mode`)

---

## 10.8 Action Wiring & Config Persistence

**File(s):** `oriterm/src/app/settings_overlay/action_handler/mod.rs`,
  `oriterm/src/app/dialog_context/content_actions.rs`

- [ ] Expand `SettingsIds` struct with all new control IDs (currently has 10 fields in
  `oriterm/src/app/settings_overlay/form_builder/mod.rs`; expanding to 24):
  ```rust
  pub struct SettingsIds {
      // Footer
      pub reset_defaults_id: WidgetId,        // new — "Reset to Defaults" button
      // Appearance (opacity_dropdown -> opacity_slider is a control type change)
      pub theme_dropdown: WidgetId,
      pub opacity_slider: WidgetId,           // was: opacity_dropdown
      pub blur_toggle: WidgetId,              // new
      // Colors
      pub scheme_card_grid: WidgetId,
      // Font
      pub font_family_dropdown: WidgetId,     // new (no family dropdown exists currently)
      pub font_size_input: WidgetId,          // was: font_size_dropdown (changed to NumberInput)
      pub font_weight_dropdown: WidgetId,     // existing
      pub ligatures_toggle: WidgetId,         // was: ligatures_checkbox (changed to Toggle)
      pub line_height_input: WidgetId,        // new
      // Terminal
      pub cursor_picker: WidgetId,            // was: cursor_style_dropdown (changed to CursorPicker)
      pub cursor_blink_toggle: WidgetId,
      pub scrollback_input: WidgetId,
      pub shell_input: WidgetId,
      pub paste_warning_dropdown: WidgetId,
      // Window
      pub tab_bar_position_dropdown: WidgetId,
      pub grid_padding_input: WidgetId,
      pub restore_session_toggle: WidgetId,
      pub initial_columns_input: WidgetId,
      pub initial_rows_input: WidgetId,
      // Bell
      pub bell_animation_dropdown: WidgetId,
      pub bell_duration_dropdown: WidgetId,
      // Rendering
      pub gpu_backend_dropdown: WidgetId,
      pub subpixel_toggle: WidgetId,          // maps to font.subpixel_mode, NOT a rendering field
  }
  ```
- [ ] Add `ValueChanged` and `TextChanged` to `handle_dialog_content_action()` match arm in
  `oriterm/src/app/dialog_context/content_actions.rs`. Currently only `Toggled` and
  `Selected` are forwarded to `dispatch_dialog_settings_action()`. Without this, sliders,
  NumberInputs, and TextInputs silently drop their actions in the dialog window path.

  **WARNING: 500-line limit.** `content_actions.rs` is currently 471 lines. Before adding
  the new match arms, extract `winit_key_to_input_event()` and `winit_mods_to_ui()`
  (lines 419-471, 53 lines) into a sibling module `dialog_context/key_conversion.rs`. This
  brings `content_actions.rs` under 420 lines and leaves headroom.

- [ ] Update `handle_settings_action()` to route all new control IDs to config fields.
  The current handler only matches `WidgetAction::Selected` and `WidgetAction::Toggled`.
  The new controls introduce additional action types:

  | Control Type | WidgetAction | Controls Using It |
  |---|---|---|
  | Dropdown | `Selected { id, index }` | theme, font_family, font_weight, tab_bar_position, paste_warning, bell_animation, bell_duration, gpu_backend |
  | Toggle | `Toggled { id, value }` | blur, ligatures, cursor_blink, restore_session, subpixel |
  | RangeSlider | `ValueChanged { id, value }` | opacity |
  | NumberInput | `ValueChanged { id, value }` | font_size, line_height, grid_padding, scrollback, initial_columns, initial_rows |
  | TextInput | `TextChanged { id, text }` | shell |
  | CursorPicker | `Selected { id, index }` | cursor_picker |
  | SchemeCard grid | `Selected { id, index }` | scheme_card_grid |

  Example match arms for the new action types:
  ```rust
  WidgetAction::ValueChanged { id, value } if *id == ids.opacity_slider => {
      config.window.opacity = (*value / 100.0).clamp(0.0, 1.0);
      true
  }
  WidgetAction::TextChanged { id, text } if *id == ids.shell_input => {
      config.terminal.shell = if text.is_empty() { None } else { Some(text.clone()) };
      true
  }
  ```

- [ ] **Overlay dispatch path is already correct**: `try_dispatch_settings_action()` in
  `overlay_dispatch.rs` is called BEFORE the match block in `handle_overlay_result()`, so ALL
  delivered actions (including `ValueChanged` and `TextChanged`) already pass through
  `handle_settings_action()`. **No routing changes needed** — only `handle_settings_action()`
  itself needs the new match arms.
- [ ] Add `Config` fields for new settings. **Already existing** (with current field names):
  - `window.blur: bool`
  - `terminal.scrollback: usize`
  - `terminal.shell: Option<String>`
  - `window.columns: usize`
  - `window.rows: usize`

  **New fields to add:**
  - `font.line_height: f32`
  - `window.tab_bar_position: TabBarPosition`
  - `window.grid_padding: f32`
  - `window.restore_session: bool`
  - `rendering.gpu_backend: GpuBackend` (requires new `RenderingConfig` section in `Config`)
- [ ] Update `apply_settings_change()` to handle new config fields.
- [ ] **Config field name mapping** (plan name -> actual Config field):

  | Plan Name | Config Path | Notes |
  |-----------|------------|-------|
  | blur_behind | `window.blur` | already exists as `bool` |
  | scrollback_lines | `terminal.scrollback` | already exists as `usize` |
  | default_shell | `terminal.shell` | already exists as `Option<String>` |
  | initial_columns | `window.columns` | already exists as `usize` |
  | initial_rows | `window.rows` | already exists as `usize` |
  | line_height | `font.line_height` | **NEW** — add to `FontConfig` |
  | tab_bar_position | `window.tab_bar_position` | **NEW** — add enum + field to `WindowConfig` |
  | grid_padding | `window.grid_padding` | **NEW** — add to `WindowConfig` (`f32`) |
  | restore_session | `window.restore_session` | **NEW** — add to `WindowConfig` (`bool`) |
  | gpu_backend | `rendering.gpu_backend` | **NEW** — add `RenderingConfig` section to `Config` |
  | subpixel_toggle | `font.subpixel_mode` | **EXISTS** — toggle maps `Some("rgb")` / `Some("none")` |

- [ ] Each new `Config` field needs:
  1. Field added to struct (with `#[serde(default)]`)
  2. Default impl updated
  3. TOML deserialization test (in `oriterm/src/config/tests.rs`)
  4. TOML serialization round-trip test (serialize -> deserialize -> compare)
  5. `apply_settings_change()` handler
- [ ] Test: change each setting, save, verify config.toml updated, verify app state changes.

### New Config Type Definitions

Adding `TabBarPosition`, `GpuBackend`, and `RenderingConfig` requires defining new types:

```rust
// In oriterm/src/config/mod.rs (or a new config/rendering.rs submodule)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum TabBarPosition {
    #[default]
    Top,
    Bottom,
    Hidden,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum GpuBackend {
    #[default]
    Auto,
    Vulkan,
    #[serde(alias = "dx12")]
    DirectX12,
    Metal,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct RenderingConfig {
    pub gpu_backend: GpuBackend,
}
```

**IMPORTANT — Do NOT add `rendering.subpixel: bool`.** `font.subpixel_mode` already exists
as `Option<String>` in `FontConfig` (with values `"rgb"`, `"bgr"`, `"none"`). The rendering
page's "LCD subpixel rendering" Toggle maps to this existing field: enable =
`Some("rgb".into())`, disable = `Some("none".into())`. The `subpixel_toggle` ID in
`SettingsIds` writes to `config.font.subpixel_mode`, not a rendering config field. Also note
`font.subpixel_positioning` (`bool`) is a separate feature — do not confuse them.

Similarly, `font.line_height` does not exist yet. Adding it to `FontConfig` means the
field must be consumed by the font rasterizer / shaper. Verify that the rendering pipeline
reads `font.line_height` and applies it to cell height calculation, or document that this
is a config-only field that takes effect on next restart.

### `apply_settings_change()` Expansion

The existing `apply_settings_change()` calls:
- `apply_font_changes()` — handles font size, weight, features
- `apply_color_changes()` — handles color scheme
- `apply_cursor_changes()` — handles cursor style, blink
- `apply_window_changes()` — handles opacity, blur

New settings require new or expanded apply methods:
- [ ] `apply_font_changes()`: add `line_height` and `font_family` handling.
- [ ] `apply_window_changes()`: add `tab_bar_position`, `grid_padding`, `restore_session`,
  `columns`, `rows` handling.
- [ ] New `apply_rendering_changes()`: handle `gpu_backend` change. **WARNING: complexity.**
  Changing the GPU backend at runtime requires recreating the wgpu `Instance`, `Adapter`,
  `Device`, `Queue`, and all surfaces — a full GPU teardown and rebuild. For the initial
  implementation, show a "restart required" label next to the GPU backend dropdown and apply
  the change on next launch only. Do NOT attempt hot GPU backend switching in this section.
- [ ] Subpixel toggle writes to `config.font.subpixel_mode`, handled by existing
  `apply_font_changes()` (which already triggers font re-rasterization).
- [ ] `apply_cursor_changes()`: CursorPicker uses the same `CursorStyle` enum — no change
  needed.
- [ ] Shell change (`terminal.shell`) only takes effect for new panes — add description text
  "Takes effect for new terminal tabs" in the UI.
- [ ] Scrollback change (`terminal.scrollback`): decide whether to apply to existing panes
  (by resizing the scrollback buffer) or defer to new panes. Document the chosen behavior.

---

## 10.9 Completion Checklist

### Layout & Navigation
- [ ] Settings dialog opens at 860x620 with sidebar + content layout
- [ ] Sidebar shows 8 nav items in 2 sections (General: 6 items, Advanced: 2 items)
- [ ] Clicking a nav item switches to the corresponding page
- [ ] Active nav item has accent highlight
- [ ] All 8 pages render with correct content matching mockup
- [ ] Both rendering modes work: overlay (modal in terminal window) and dialog (separate
  OS window) both show the new sidebar + pages layout correctly
- [ ] Keyboard Tab cycles through controls on the active page
- [ ] Arrow keys navigate the sidebar when it has focus

### Controls
- [ ] All controls are functional (dropdowns, toggles, RangeSlider, NumberInputs, pickers)
- [ ] SchemeCards show terminal preview + swatch bar, selection works
- [ ] Palette editor shows SpecialColorSwatches + ANSI ColorSwatchGrids for active scheme
- [ ] CodePreview updates when font settings change
- [ ] CursorPicker shows 3 visual options with selection
- [ ] Keybindings page shows all shortcuts with KbdBadge styling

### Scroll
- [ ] Colors page scrolls when content exceeds viewport
- [ ] Scrollbar is visible on pages with overflow content
- [ ] Scroll position resets to top when switching pages

### Persistence & Actions
- [ ] Save persists all settings to config.toml
- [ ] Cancel discards changes
- [ ] Reset to Defaults restores all settings to `Config::default()`
- [ ] Footer buttons styled correctly (ghost for Reset/Cancel, primary for Save)
- [ ] `ValueChanged` and `TextChanged` actions route correctly in both overlay and dialog
  paths (not silently dropped)
- [ ] Dirty state comparison works (`pending_config != original_config`)

### Config
- [ ] All new `Config` fields have `#[serde(default)]` and round-trip TOML tests
- [ ] `Config` and all nested config structs derive `PartialEq` (needed for dirty detection)
- [ ] `TabBarPosition` serializes as `"top"` / `"bottom"` / `"hidden"`
- [ ] `GpuBackend` serializes as `"auto"` / `"vulkan"` / `"dx12"` / `"metal"`
- [ ] Existing config files without new fields load correctly (serde defaults)

### Tests
- [ ] `form_builder/tests.rs` rewritten for the new 8-page sidebar structure
- [ ] `action_handler/tests.rs` has tests for all new control IDs (`ValueChanged` for
  RangeSlider/NumberInputs, `TextChanged` for shell TextInput, `Selected` for new dropdowns
  and CursorPicker)
- [ ] If page builders are extracted to submodules, each submodule with tests needs its own
  `tests.rs` sibling per test-organization.md rules
- [ ] TOML round-trip tests for every new config field in `oriterm/src/config/tests.rs`
- [ ] Test `WidgetAction::ResetDefaults` dispatch resets all config fields to defaults
- [ ] Test that `SettingsPanel::new()` (overlay) and `SettingsPanel::embedded()` (dialog)
  both produce valid layouts with sidebar + pages
- [ ] `./test-all.sh` green, `./clippy-all.sh` green, `./build-all.sh` green

### File Size Compliance
- [ ] `content_actions.rs` stays under 500 lines (extract `winit_key_to_input_event` first)
- [ ] `settings_panel/mod.rs` stays under 500 lines (extract paint helpers or footer)
- [ ] Each page builder submodule stays under 500 lines
- [ ] `form_builder/mod.rs` stays under 500 lines after `SettingsIds` expansion

### Legacy Code Cleanup
- [ ] Old `build_appearance_section()`, `build_font_section()`, `build_behavior_section()`,
  `build_terminal_section()`, `build_bell_section()` deleted from `form_builder/mod.rs`
- [ ] Old `OPACITY_VALUES`, `FONT_SIZE_VALUES` constants removed (replaced by RangeSlider
  and NumberInput controls)
- [ ] No references to old `FormLayout`-based settings form remain (except in `_old/`)

**Exit Criteria:** The settings dialog visually matches `mockups/settings.html` in layout,
colors, spacing, and interactivity. All controls read from and write to `Config`. Save
persists to disk. Both overlay and dialog rendering modes work. No regressions in terminal
functionality.
