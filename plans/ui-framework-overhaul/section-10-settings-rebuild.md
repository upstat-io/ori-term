---
section: "10"
title: "Settings Panel Rebuild"
status: not-started
goal: "Settings dialog matches mockups/settings.html — sidebar navigation, 8 pages, all controls wired to config"
inspired_by:
  - "mockups/settings.html — the design spec"
  - "Windows Terminal settings UI"
  - "Ghostty Config app"
depends_on: ["09"]
reviewed: false
sections:
  - id: "10.1"
    title: "Dialog Layout"
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
Section 09, expanding to 8 pages and ~22 control IDs.

**Depends on:** Section 09 (New Widget Library — all widgets must exist).

---

## 10.1 Dialog Layout

**File(s):** `oriterm/src/app/settings_overlay/form_builder/mod.rs` (dialog skeleton),
`oriterm/src/app/settings_overlay/form_builder/pages.rs` or per-page files:
`appearance.rs`, `colors.rs`, `font.rs`, `terminal.rs`, `keybindings.rs`,
`window.rs`, `bell.rs`, `rendering.rs`.

**WARNING**: The current `form_builder/mod.rs` is 224 lines. Adding 8 page builder
functions inline would push it well past 500 lines. Split page builders into submodules.
Recommended: `form_builder/mod.rs` keeps the top-level `build_settings_dialog()` function
and the `SettingsIds` struct. Each page builder lives in its own file. If grouping, combine
the 3 smaller pages (window/bell/rendering) into `form_builder/advanced_pages.rs`.

Replace the current single-column form with sidebar + pages.

- [ ] Rebuild `build_settings_form()`:
  ```rust
  pub fn build_settings_dialog(config: &Config) -> (SettingsLayout, SettingsIds) {
      let sidebar = SidebarNavWidget::new(vec![
          // General
          NavSection::new("General", vec![
              NavItem::new("Appearance", IconId::Sun, 0),
              NavItem::new("Colors", IconId::Palette, 1),
              NavItem::new("Font", IconId::Type, 2),
              NavItem::new("Terminal", IconId::Terminal, 3),
              NavItem::new("Keybindings", IconId::Keyboard, 4),
              NavItem::new("Window", IconId::Window, 5),
          ]),
          // Advanced
          NavSection::new("Advanced", vec![
              NavItem::new("Bell", IconId::Bell, 6),
              NavItem::new("Rendering", IconId::Activity, 7),
          ]),
      ]);

      let pages = PageContainerWidget::new(vec![
          build_appearance_page(config),
          build_colors_page(config),
          build_font_page(config),
          build_terminal_page(config),
          build_keybindings_page(config),
          build_window_page(config),
          build_bell_page(config),
          build_rendering_page(config),
      ]);

      // Top-level layout: sidebar (fixed 200px) | content (fill)
      // Footer: Reset to Defaults | Cancel | Save
  }
  ```
- [ ] Dialog window size: 860x620 logical pixels (matches mockup).
  **Implementation**: Dialog window creation is in `oriterm/src/app/dialog_management.rs`.
  Update `create_dialog_window()` (or equivalent) to request inner size 860x620 logical.
  If the overlay mode is used instead, update the overlay centering bounds.
  Also update `SettingsPanel::PANEL_WIDTH` from 600.0 to the full content width
  (860 - 200 sidebar = 660px content area).
- [ ] Footer buttons: "Reset to Defaults" (ghost), "Cancel" (ghost), "Save" (primary/accent)
- [ ] Footer separator line above buttons
- [ ] "Reset to Defaults" button: add `WidgetAction::ResetDefaults` variant to
  `WidgetAction` enum in `widgets/mod.rs`. The action handler creates a `Config::default()`
  and applies it. Add the reset button ID to `SettingsIds`.

---

## 10.2 Appearance Page

- [ ] Content header: "Appearance" title + description
- [ ] Theme section:
  - SettingRow: "Color scheme" dropdown (all built-in schemes)
- [ ] Window section:
  - SettingRow: "Opacity" range slider (30-100%)
  - SettingRow: "Blur behind" toggle

---

## 10.3 Colors Page

- [ ] Content header: "Colors" title + description
- [ ] Schemes section:
  - SchemeCard grid (AutoFill 240px) showing all built-in color schemes
  - Each card: terminal preview + swatch bar
  - Active card: accent border + "Active" badge
- [ ] Palette editor (below card grid):
  - Title: "Palette — {scheme name}"
  - Special colors row: Foreground, Background, Cursor, Selection (4-column grid)
  - Normal ANSI colors: 8-column ColorSwatchGrid (colors 0-7)
  - Bright ANSI colors: 8-column ColorSwatchGrid (colors 8-15)

---

## 10.4 Font Page

- [ ] CodePreview: syntax-highlighted Rust code with current font
- [ ] Typeface section:
  - SettingRow: "Font family" dropdown
  - SettingRow: "Size" number input (8-32, step 0.5)
  - SettingRow: "Weight" dropdown (300-700)
- [ ] Features section:
  - SettingRow: "Ligatures" toggle
  - SettingRow: "Line height" number input (0.8-2.0, step 0.05)

---

## 10.5 Terminal Page

- [ ] Content header: "Terminal" title + description
- [ ] Cursor section:
  - CursorPicker: 3 visual options (Block, Bar, Underline)
  - SettingRow: "Cursor blink" toggle
- [ ] Scrollback section:
  - SettingRow: "Maximum lines" number input (0-100000, step 1000)
- [ ] Shell section:
  - SettingRow: "Default shell" text input (monospace)
  - SettingRow: "Paste warning" dropdown (Always/Never)

---

## 10.6 Keybindings Page

- [ ] Content header: "Keybindings" title + description
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
  - Padding section: "Grid padding" number input (0-40, step 2)
  - Startup section: "Restore previous session" toggle,
    "Initial columns" number input, "Initial rows" number input

- [ ] **Bell page**:
  - Visual Bell section: "Animation" dropdown (Ease Out/Linear/None),
    "Duration" dropdown (Off/50ms/150ms/300ms/500ms)

- [ ] **Rendering page**:
  - GPU section: "Backend" dropdown (Auto/Vulkan/DirectX 12/Metal)
  - Text section: "LCD subpixel rendering" toggle

---

## 10.8 Action Wiring & Config Persistence

**File(s):** `oriterm/src/app/settings_overlay/action_handler/mod.rs`,
  `oriterm/src/app/dialog_context/content_actions.rs`

- [ ] Expand `SettingsIds` struct with all new control IDs (currently has 10 fields in
  `oriterm/src/app/settings_overlay/form_builder/mod.rs`; expanding to ~22):
  ```rust
  pub struct SettingsIds {
      // Appearance (opacity_dropdown → opacity_slider is a control type change)
      pub theme_dropdown: WidgetId,
      pub opacity_slider: WidgetId,      // was: opacity_dropdown
      pub blur_toggle: WidgetId,         // new
      // Colors
      pub scheme_card_grid: WidgetId,
      // Font
      pub font_family_dropdown: WidgetId,     // new (was part of font_size_dropdown)
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
      pub subpixel_toggle: WidgetId,
  }
  ```
- [ ] Update `handle_settings_action()` to route all new control IDs to config fields
- [ ] Add `Config` fields for new settings. **Already existing** (with current field names):
  - `window.blur: bool` (plan calls it `blur_behind` but it's `blur` in `WindowConfig`)
  - `terminal.scrollback: usize` (plan calls it `scrollback_lines: u32` but it's `scrollback: usize`)
  - `terminal.shell: Option<String>` (plan calls it `default_shell: String` but it's `shell: Option<String>`)
  - `window.columns: usize` (plan calls it `initial_columns: u16` but it's `columns: usize`)
  - `window.rows: usize` (plan calls it `initial_rows: u16` but it's `rows: usize`)

  **New fields to add:**
  - `font.line_height: f32`
  - `window.tab_bar_position: TabBarPosition`
  - `window.grid_padding: f32`
  - `window.restore_session: bool`
  - `rendering.gpu_backend: GpuBackend` (may need a new `RenderingConfig` section)
  - `rendering.subpixel: bool`
- [ ] Update `apply_settings_change()` for new config fields
- [ ] **Config field name mapping** (plan name → actual Config field):
  | Plan Name | Config Path | Notes |
  |-----------|------------|-------|
  | blur_behind | `window.blur` | already exists as `bool` |
  | scrollback_lines | `terminal.scrollback` | already exists as `usize` |
  | default_shell | `terminal.shell` | already exists as `Option<String>` |
  | initial_columns | `window.columns` | already exists as `usize` |
  | initial_rows | `window.rows` | already exists as `usize` |
  | line_height | `font.line_height` | **NEW** — add to `FontConfig` |
  | tab_bar_position | `window.tab_bar_position` | **NEW** — add enum + field to `WindowConfig` |
  | grid_padding | `window.grid_padding` | **NEW** — add to `WindowConfig` (f32) |
  | restore_session | `window.restore_session` | **NEW** — add to `WindowConfig` (bool) |
  | gpu_backend | — | **NEW** — add `RenderingConfig` section to `Config` |
  | subpixel | — | **NEW** — add to `RenderingConfig` (bool) |
- [ ] Each new `Config` field needs:
  1. Field added to struct (with `#[serde(default)]`)
  2. Default impl updated
  3. TOML deserialization test
  4. `apply_settings_change()` handler
- [ ] Test: change each setting, save, verify config.toml updated, verify app state changes

---

## 10.9 Completion Checklist

- [ ] Settings dialog opens at 860x620 with sidebar + content layout
- [ ] Sidebar shows 8 nav items in 2 sections (General, Advanced)
- [ ] Clicking nav item switches to corresponding page
- [ ] Active nav item has accent highlight
- [ ] All 8 pages render with correct content matching mockup
- [ ] All controls are functional (dropdowns, toggles, sliders, number inputs, pickers)
- [ ] Scheme cards show terminal preview + swatch bar, selection works
- [ ] Palette editor shows special colors + ANSI color grids for active scheme
- [ ] Font preview updates when font settings change
- [ ] Cursor picker shows 3 visual options with selection
- [ ] Keybindings page shows all shortcuts with KbdBadge styling
- [ ] Save persists all settings to config.toml
- [ ] Cancel discards changes
- [ ] Reset to Defaults restores all settings
- [ ] Footer buttons styled correctly (ghost + primary)
- [ ] Test file: `oriterm/src/app/settings_overlay/form_builder/tests.rs` (expand existing)
- [ ] If page builders are extracted to submodules, each submodule with tests needs its own
  `tests.rs` sibling per test-organization.md rules
- [ ] `./test-all.sh` green, `./clippy-all.sh` green, `./build-all.sh` green

**Exit Criteria:** The settings dialog visually matches `mockups/settings.html` in layout,
colors, spacing, and interactivity. All controls read from and write to `Config`. Save
persists to disk. No regressions in terminal functionality.
