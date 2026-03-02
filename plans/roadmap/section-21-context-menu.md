---
section: 21
title: Context Menu & Window Controls
status: in-progress
tier: 4
goal: GPU-rendered context menus, config reload broadcasting, settings UI, window controls
sections:
  - id: "21.1"
    title: Context Menu
    status: complete
  - id: "21.2"
    title: Config Reload Broadcasting
    status: not-started
  - id: "21.3"
    title: Settings UI
    status: not-started
  - id: "21.4"
    title: Window Controls
    status: not-started
  - id: "21.5"
    title: Taskbar Jump List & Dock Menu
    status: not-started
  - id: "21.6"
    title: Section Completion
    status: not-started
---

# Section 21: Context Menu & Window Controls

**Status:** 📋 Planned
**Goal:** GPU-rendered context menus, config reload broadcasting, settings UI, window controls. This is the final feature parity section — completing it means the rebuild matches the old prototype's full capability.

**Crate:** `oriterm` (binary only — no core changes)

**Reference:** `_old/src/context_menu.rs`, `_old/src/gpu/render_overlay.rs`, `_old/src/app/config_reload.rs`, `_old/src/app/settings_ui.rs`, `_old/src/gpu/render_settings.rs`, `_old/src/gpu/render_tab_bar.rs`, `_old/src/tab_bar.rs`

---

## 21.1 Context Menu

GPU-rendered context menus (not OS native) for consistent cross-platform styling. Three distinct menu types depending on what was right-clicked.

**File:** `oriterm_ui/src/widgets/menu/mod.rs` (MenuWidget), `oriterm/src/app/context_menu/mod.rs` (ContextAction, ContextMenuState, builders)

**Reference:** `_old/src/context_menu.rs`, `_old/src/gpu/render_overlay.rs`

- [x] `MenuWidget` struct (plan called this `MenuOverlay` — position/size managed by overlay system):
  - [x] `entries: Vec<MenuEntry>` — menu items
  - [x] Position managed by overlay anchoring (not stored on widget — cleaner separation)
  - [x] `hovered: Option<usize>` — currently hovered entry index (None if not hovering any item)
  - [x] Width/height computed dynamically in `layout()` and `total_height()` (not cached — correct for overlay resizing)
  - [x] Scale handled by overlay system (DPI-independent widget)
- [x] `MenuEntry` enum:
  - [x] `Item { label: String }` — clickable item (action decoupled via `ContextMenuState`)
  - [x] `Check { label: String, checked: bool }` — item with checkmark indicator (action decoupled)
  - [x] `Separator` — horizontal line divider
- [x] `ContextAction` enum + `ContextMenuState` — maps entry indices to actions (cleaner than embedding actions in entries)
- [x] Three menu contexts:
  1. [x] **Tab context menu** (right-click on a tab):
     - [x] Close Tab
     - [x] Duplicate Tab
     - [x] Move to New Window
  2. [x] **Grid context menu** (right-click in terminal area):
     - [x] Copy (enabled only if selection exists)
     - [x] Paste
     - [x] Select All
     - [x] Separator
     - [x] New Tab
     - [x] Close Tab
     - [x] Separator
     - [x] Settings
  3. [x] **Dropdown menu** (click dropdown button in tab bar):
     - [x] Settings (opens settings window)
     - [x] Separator
     - [x] Color scheme selector: list all built-in schemes with `Check` entries (active scheme has checkmark)
- [x] Layout calculation:
  - [x] Measure max label width using UI font collection
  - [x] If any `Check` entry exists: add checkmark icon width + gap
  - [x] `width = max_label_width + 2 * ITEM_PADDING_X + MENU_EXTRA_WIDTH`, clamped to `MENU_MIN_WIDTH`
  - [x] `height = 2 * MENU_PADDING_Y + sum(entry_height for each entry)`
  - [x] Entry heights: `ITEM_HEIGHT` for Item/Check, `SEPARATOR_HEIGHT` for Separator
- [x] Hit testing:
  - [x] `entry_at_y(y: f32) -> Option<usize>` (overlay handles bounds check, widget does Y mapping)
  - [x] Iterate entries, accumulate Y offset
  - [x] Return entry index if clickable (skip separators)
  - [x] Return None if outside or on separator
- [x] Dismiss conditions:
  - [x] Click outside menu rect (overlay system)
  - [x] Escape key (`WidgetAction::DismissOverlay`)
  - [x] Any action selected and executed (`WidgetAction::Selected`)
- [x] GPU rendering (overlay pass, topmost):
  - [x] Shadow rectangle (2px offset down-right, rounded corners, semi-transparent)
  - [x] Menu background rectangle (rounded corners, border)
  - [x] Per-entry:
    - [x] **Item**: text label at left margin from left
    - [x] **Check**: checkmark icon (if checked) + label indented past icon
    - [x] **Separator**: horizontal line with left/right margins
    - [x] Hover highlight: rounded rectangle with inset, lighter background
- [x] Menu style constants (in `MenuStyle` struct, derived from `UiTheme`):
  - [x] `padding_y: f32` — vertical padding inside menu
  - [x] `padding_x: f32` — horizontal padding for labels
  - [x] `hover_inset: f32` doubles as separator margin (separator drawn between hover insets)
  - [x] `separator_height: f32` — separator entry height
  - [x] `item_height: f32` — height per clickable item
  - [x] `min_width: f32` — minimum menu width
  - [x] `extra_width: f32` — extra padding for checkmark column
  - [x] `corner_radius: f32` — corner radius for menu shape
  - [x] `hover_inset: f32` — inset of hover highlight from menu edges
  - [x] `hover_radius: f32` — corner radius for hover highlight

---

## 21.2 Config Reload Broadcasting

When the config file changes (detected by file watcher), changes must be applied to ALL tabs and ALL windows consistently. Some changes (font) require expensive atlas rebuilds and grid reflow.

**File:** `oriterm/src/app/config_reload.rs`

**Reference:** `_old/src/app/config_reload.rs`

- [ ] `apply_config_reload(&mut self)`:
  - [ ] Load new config from disk via `Config::try_load()` — if parse fails, log error and return (keep current config)
  - [ ] **Color scheme changes**: if `new.colors.scheme != old.colors.scheme`:
    - [ ] Resolve scheme from built-in list: `palette::find_scheme(&name)`
    - [ ] Apply to ALL tabs: `tab.apply_color_config(scheme, &colors, bold_is_bright)`
    - [ ] Mark all grids dirty
  - [ ] **Font changes**: if any of `size`, `family`, `features`, `fallback`, `weight`, `tab_bar_font_weight`, `tab_bar_font_family` changed:
    - [ ] Rebuild main font collection at `new_size * scale_factor`
    - [ ] Rebuild UI font collection at `new_size * scale_factor * UI_FONT_SCALE`
    - [ ] Rebuild glyph atlas (expensive — clears all cached glyphs)
    - [ ] **Resize ALL tabs in ALL windows** — cell dimensions changed, grids need reflow:
      - [ ] For each window (skipping settings window):
        - [ ] For each tab: `tab.clear_selection()`, `tab.resize(new_cols, new_rows, ...)`
    - [ ] Log: `"config reload: font size={}, cell={}x{}, tab_bar_weight={}"`
  - [ ] **Cursor style changes**: if `new.terminal.cursor_style != old.terminal.cursor_style`:
    - [ ] Parse new cursor shape
    - [ ] Apply to ALL tabs: `tab.set_cursor_shape(new_cursor)`
  - [ ] **Keybinding changes**:
    - [ ] Rebuild binding table: `self.bindings = keybindings::merge_bindings(&new.keybind)`
  - [ ] **Opacity changes**: mark all windows for redraw (compositor effect may need update)
  - [ ] Store new config: `self.config = new_config`
  - [ ] Mark `tab_bar_dirty = true`, all grids dirty
  - [ ] Request redraw on all windows
- [ ] `Config::save()` — persist config changes to disk:
  - [ ] Write current config to TOML file at `config_path()`
  - [ ] Used by dropdown menu scheme selection (and future settings UI) to persist user choices
  - [ ] Handle write errors gracefully (log warning, don't crash)

---

## 21.3 Settings UI

Separate frameless settings window (not an overlay). Displays color scheme selector with live preview. GPU-rendered for consistent styling.

**File:** `oriterm/src/app/settings_ui.rs`, `oriterm/src/gpu/render_settings.rs`

**Reference:** `_old/src/app/settings_ui.rs`, `_old/src/gpu/render_settings.rs`

- [ ] `settings_window: Option<WindowId>` on App — None if settings not open
- [ ] Settings window lifecycle:
  - [ ] `open_settings_window(event_loop)` — create separate small window (~300×350px), init GPU surface
  - [ ] `close_settings_window()` — remove from windows map, set `settings_window = None`
  - [ ] Only Escape key works in settings window (all other input consumed)
- [ ] Settings window content:
  - [ ] Title bar: "Theme" label + close button (top-right corner, 30×30px)
  - [ ] Color scheme list: rows of ~40px height each:
    - [ ] Color swatch: 16×16px square showing scheme's background color
    - [ ] Scheme name: text label 40px from left
    - [ ] Active indicator: checkmark icon if this is the current scheme
    - [ ] Hover highlight: rounded rect across full row width
- [ ] Mouse handling:
  - [ ] Top-right 30×30px: close button
  - [ ] Top 50px: title area (no interaction)
  - [ ] Below: scheme rows. `row_idx = (y - 50) / 40`
  - [ ] Click on row: `apply_scheme_to_all_tabs(scheme)`
- [ ] Scheme application:
  - [ ] Update `self.active_scheme`
  - [ ] Apply to ALL tabs: `tab.apply_color_config(scheme, &config.colors, bold_is_bright)`
  - [ ] Persist to config file: `self.config.colors.scheme = scheme.name; self.config.save()`
  - [ ] Request redraw on all windows
- [ ] GPU rendering:
  - [ ] Full-window background (dark, derived from palette)
  - [ ] 1px border on all edges
  - [ ] Per-row rendering with color derivation from palette
  - [ ] This is a stretch goal — can be deferred past initial feature parity

---

## 21.4 Window Controls

Custom window controls for the frameless window, integrated into the tab bar. Platform-specific rendering (rectangular on Windows, circular on macOS/Linux).

**File:** `oriterm/src/chrome/tab_bar.rs` (integrated with tab bar rendering)

**Reference:** `_old/src/gpu/render_tab_bar.rs`, `_old/src/tab_bar.rs`

- [ ] Three buttons in top-right corner of tab bar:
  - [ ] Minimize (─): `window.set_minimized(true)`
  - [ ] Maximize (□ / ⧉): toggle `window.set_maximized()` — icon changes based on `is_maximized`
  - [ ] Close (×): close window
- [ ] Platform-specific rendering:
  - [ ] **Windows**: Three rectangular buttons, each `CONTROL_BUTTON_WIDTH` (58px) wide:
    - [ ] Minimize: horizontal line icon
    - [ ] Maximize: single square icon (when not maximized) or two overlapping squares with erase-out (when maximized/restored)
    - [ ] Close: X icon (two diagonal small rectangles)
    - [ ] Close button hover: red background (`CONTROL_CLOSE_HOVER_BG`) with white icon (`CONTROL_CLOSE_HOVER_FG`)
    - [ ] Other buttons hover: subtle background change (`control_hover_bg`)
  - [ ] **Linux/macOS**: Three circular buttons:
    - [ ] Diameter: 24px
    - [ ] Spacing: 8px between buttons
    - [ ] Margins: 12px from edges
    - [ ] Colored circles with icons on hover
- [ ] Window dragging:
  - [ ] Double-click on `DragArea` (empty tab bar space): toggle maximize
  - [ ] Click + drag on `DragArea`: `window.drag_window()` — OS handles movement
  - [ ] Aero Snap on Windows: handled by OS via `drag_window()` when custom WndProc subclass is installed
- [ ] Aero Snap subclass (Windows-specific):
  - [ ] Custom `WndProc` that handles `WM_NCHITTEST` — returns `HTCAPTION` for drag areas, `HTCLIENT` for interactive areas
  - [ ] Also handles `WM_DPICHANGED` — stores new DPI for `handle_resize()` to read
  - [ ] Required because frameless windows don't get Snap behavior by default

---

## 21.5 Taskbar Jump List & Dock Menu

OS-level quick-action menus that appear when the user right-clicks the app icon in the Windows taskbar or macOS dock. These provide fast access to common actions (new tab, new window, profiles) without first focusing the app window.

**File:** `oriterm/src/platform/jump_list.rs` (new — Windows), `oriterm/src/platform/dock_menu.rs` (new — macOS)

**Reference:** Windows Terminal `Jumplist.cpp` (COM-based, profile entries), WezTerm `app.rs` (`applicationDockMenu` — "New Window"), Ghostty `AppDelegate.swift` (dock menu — "New Window" + "New Tab")

### Windows — Jump List

Win32 COM API: `ICustomDestinationList` + `IShellLinkW`. Items appear in the taskbar right-click menu and Start menu pin.

- [ ] Jump list initialization on app startup:
  - [ ] Create `ICustomDestinationList` instance (`CLSID_DestinationList`)
  - [ ] `BeginList()` → get max slots, removed objects
  - [ ] Build task collection via `IObjectCollection`
  - [ ] `CommitList()` to publish
- [ ] Built-in tasks (always present):
  - [ ] **New Tab** — launches `ori_term.exe --new-tab` (or reuses running instance via IPC when Section 34 lands)
  - [ ] **New Window** — launches `ori_term.exe --new-window`
- [ ] Profile quick-launch entries (when profile system exists):
  - [ ] One `IShellLinkW` per configured profile
  - [ ] Display name: profile name (e.g., "PowerShell", "Ubuntu")
  - [ ] Arguments: `--profile {profile_name}`
  - [ ] Icon: profile icon path if configured, otherwise app icon
  - [ ] Grouped under custom "Profiles" category
- [ ] `IShellLinkW` construction per item:
  - [ ] `SetPath()` → path to `ori_term.exe`
  - [ ] `SetArguments()` → command-line args for the action
  - [ ] `SetDescription()` → tooltip text
  - [ ] `IPropertyStore::SetValue(PKEY_Title)` → display name
  - [ ] `IPropertyStore::SetValue(PKEY_AppUserModel_ID)` → app user model ID (for taskbar grouping)
- [ ] Update triggers:
  - [ ] On startup (always rebuild)
  - [ ] On profile add/remove/rename (when profile system exists)
  - [ ] On config reload (if profile list changed)
- [ ] Error handling: Jump list APIs may fail (Explorer not running, COM init failure) — log and continue, never crash

### macOS — Dock Menu

Cocoa API: implement `applicationDockMenu(_:)` on `NSApplicationDelegate`, return `NSMenu`.

- [ ] Dock menu setup:
  - [ ] Implement `applicationDockMenu:` delegate method (via objc2 / cocoa crate)
  - [ ] Return cached `NSMenu` instance (rebuilt when menu items change)
- [ ] Menu items:
  - [ ] **New Window** — `NSMenuItem` with action selector → spawns new window
  - [ ] **New Tab** — `NSMenuItem` with action selector → adds tab to frontmost window
  - [ ] Separator
  - [ ] Profile entries (when profile system exists): one item per profile
- [ ] Update triggers:
  - [ ] Rebuild menu on profile change
  - [ ] Menu is queried lazily by AppKit (no push needed — just update the cached `NSMenu`)

### Linux — Desktop Actions (Static)

`.desktop` file actions — defined at install time, not dynamically updated.

- [ ] `.desktop` file entries:
  ```ini
  [Desktop Action new-window]
  Name=New Window
  Exec=ori_term --new-window

  [Desktop Action new-tab]
  Name=New Tab
  Exec=ori_term --new-tab
  ```
- [ ] Reference in main `[Desktop Entry]` section: `Actions=new-window;new-tab;`
- [ ] No runtime API needed — desktop environments read `.desktop` file at install/login
- [ ] Dynamic quicklists (Ubuntu Unity `com.canonical.unity.launcher`) — stretch goal, low priority

**Tests:**
- [ ] Windows: Jump list builds without COM errors (mock `ICustomDestinationList` or integration test)
- [ ] Windows: correct number of `IShellLinkW` items created for N profiles + 2 built-in tasks
- [ ] macOS: `NSMenu` returned by dock menu contains expected items
- [ ] Linux: `.desktop` file validates with `desktop-file-validate`
- [ ] All platforms: graceful degradation when API unavailable (log warning, no crash)

---

## 21.6 Section Completion

This is the **final feature parity checkpoint**. Completing this section means the rebuild matches the old prototype's full capability.

- [ ] All 21.1–21.5 items complete
- [ ] Context menu: 3 menu types, GPU-rendered, checkmark entries, shadow rendering
- [ ] Config reload: broadcast to all tabs/windows, font atlas rebuild, grid reflow
- [ ] Settings UI: separate window, color scheme selector, live preview, persist to config
- [ ] Window controls: platform-specific rendering, Aero Snap, frameless drag
- [ ] Jump List (Windows): "New Tab", "New Window", profile entries in taskbar right-click
- [ ] Dock Menu (macOS): "New Window", "New Tab" in dock right-click
- [ ] Desktop Actions (Linux): `.desktop` file with new-window/new-tab actions
- [ ] Tab struct: clean ownership, lock-free mode cache, background thread cleanup
- [ ] Tab management: create, close, duplicate, cycle, reorder, CWD inheritance
- [ ] Tab bar layout: DPI-aware, width lock, platform-specific control zone
- [ ] Tab bar rendering: separators with suppression, bell pulse, dragged tab overlay, animation offsets
- [ ] Hit testing: correct priority order, close button inset, platform-specific controls
- [ ] Drag: 10px threshold, center-based insertion, tear-off with directional thresholds, mouse offset preservation
- [ ] OS drag + merge: WM_MOVING detection, seamless drag continuation, synthesized mouse-down, stale button-up suppression
- [ ] Multi-window: shared GPU, flat tab storage, cross-window tab movement
- [ ] Window lifecycle: no-flash startup, DPI-aware resize, ConPTY-safe cleanup, exit-before-drop
- [ ] Coordinate systems: pixel → cell, tab bar layout, grid padding, side detection
- [ ] Event routing: 7-layer keyboard dispatch, 7-layer mouse dispatch, search/menu interception
- [ ] Render scheduling: about_to_wait coalescing, 8ms frame budget, cursor blink scheduling
- [ ] Shell integration: 5 shell injection mechanisms, two-parser strategy, CWD tracking, prompt state machine, title priority
- [ ] `cargo build -p oriterm --target x86_64-pc-windows-gnu --release` — clean build
- [ ] `cargo clippy -p oriterm -p oriterm_core --target x86_64-pc-windows-gnu` — no warnings
- [ ] `cargo test -p oriterm_core` — all tests pass
- [ ] Deploy to Windows: full feature parity with old prototype
- [ ] **Stress test**: multiple tabs, htop in one, vim in another, heavy output in third — no lockup, no starvation
- [ ] **Drag stress test**: rapid drag reorder across multiple windows, tear-off and merge in quick succession — no crash, no orphaned tabs
- [ ] **Close stress test**: rapidly close many tabs while hovering tab bar — close buttons don't shift unexpectedly (tab width lock works)

**Exit Criteria:** Feature parity with the old prototype. Clean architecture, clean threading, no god objects, no contention issues. Every intricacy from the old prototype is preserved: ConPTY deadlock avoidance, seamless drag-merge, mode cache, CWD inheritance, tab width lock, bell pulse animation, prompt state deferred marking, keyboard mode stack swap on alt screen. The terminal emulator is ready for daily use.
