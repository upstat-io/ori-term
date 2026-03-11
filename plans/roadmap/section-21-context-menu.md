---
section: 21
title: Context Menu & Window Controls
status: in-progress
reviewed: false
tier: 4
goal: GPU-rendered context menus, config reload broadcasting, settings UI, window controls, taskbar jump list
sections:
  - id: "21.1"
    title: Context Menu
    status: complete
  - id: "21.2"
    title: Config Reload Broadcasting
    status: complete
  - id: "21.3"
    title: Settings UI
    status: complete
  - id: "21.4"
    title: Window Controls
    status: complete
  - id: "21.5"
    title: Taskbar Jump List & Dock Menu
    status: not-started
  - id: "21.6"
    title: Section Completion
    status: not-started
---

# Section 21: Context Menu & Window Controls

**Status:** In Progress (4 of 6 sub-sections complete)
**Goal:** GPU-rendered context menus, config reload broadcasting, settings UI, window controls, taskbar jump list.

**Crates:** `oriterm` (binary), `oriterm_ui` (widget library)

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
  - [x] Measure max label width using `TextMeasurer` (backed by `UiFontMeasurer`)
  - [x] If any `Check` entry exists: left margin includes checkmark width + gap
  - [x] `width = (left_margin + max_label_w + extra_width).max(min_width)`
  - [x] `height = padding_y * 2 + sum(entry_height for each entry)`
  - [x] Entry heights: `item_height` for Item/Check, `separator_height` for Separator
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
- [x] Menu style constants (in `MenuStyle` struct, derived via `MenuStyle::from_theme(&UiTheme)`):
  - [x] `item_height: f32` — height per clickable item
  - [x] `padding_y: f32` — vertical padding inside menu
  - [x] `padding_x: f32` — horizontal padding for labels
  - [x] `min_width: f32` — minimum menu width
  - [x] `extra_width: f32` — extra padding beyond widest label
  - [x] `separator_height: f32` — separator entry height
  - [x] `corner_radius: f32` — corner radius for menu shape
  - [x] `hover_inset: f32` — inset of hover highlight from menu edges (also doubles as separator margin)
  - [x] `hover_radius: f32` — corner radius for hover highlight
  - [x] `checkmark_size: f32` — check mark area width/height
  - [x] `checkmark_gap: f32` — gap between check mark and label text
  - [x] Color fields: `bg`, `fg`, `hover_bg`, `separator_color`, `border_color`, `check_color`, `shadow_color`
  - [x] `border_width: f32`, `font_size: f32`
- [x] Action dispatch chain (complete flow from click to effect):
  1. [x] User clicks menu item → `MenuWidget::handle_mouse` emits `WidgetAction::Selected { id, index }`
  2. [x] Overlay system delivers event → `handle_overlay_result()` in `overlay_dispatch.rs`
  3. [x] `dispatch_context_action(index)` resolves index via `ContextMenuState::resolve()`
  4. [x] Dismisses menu overlay, then matches on `ContextAction` variant to execute
  5. [x] Each action delegates to existing `App` methods (`copy_selection`, `paste_from_clipboard`, `close_tab_at_index`, etc.)
- [x] Edge case: Copy with no selection — handled at build time: `build_grid_context_menu(has_selection)` omits the Copy entry entirely when `has_selection` is false (tested in `grid_context_menu_without_selection`)
- [x] Edge case: CloseTab from grid context menu uses placeholder index 0 — the dispatch in `overlay_dispatch.rs` calls `close_tab_at_index(0)` but this works because the grid context menu always applies to the active tab
- [x] Keyboard navigation within open menu:
  - [x] Arrow Down/Up: navigate between clickable items (skips separators, wraps around)
  - [x] Enter/Space: activate hovered item (emit `Selected`)
  - [x] Escape: dismiss overlay (emit `DismissOverlay`)
  - [x] Requires focus — `is_focusable()` returns `true`, unfocused menu ignores keys

**Tests (21.1):**
- [x] `oriterm/src/app/context_menu/tests.rs`: dropdown menu builder (entries, actions, empty schemes, out-of-bounds resolve)
- [x] `oriterm/src/app/context_menu/tests.rs`: tab context menu builder (entries, actions with tab index)
- [x] `oriterm/src/app/context_menu/tests.rs`: grid context menu builder (with/without selection, action coverage)
- [x] `oriterm_ui/src/widgets/menu/tests.rs`: layout (min width, height, empty menu, wide labels, check entries)
- [x] `oriterm_ui/src/widgets/menu/tests.rs`: mouse interaction (click emits selected, separator not clickable, hover tracking, hover leave)
- [x] `oriterm_ui/src/widgets/menu/tests.rs`: keyboard navigation (arrow down/up, enter, escape, space, wrapping, consecutive separators)
- [x] `oriterm_ui/src/widgets/menu/tests.rs`: edge cases (single item, not focused ignores keys, right-click ignored, out-of-bounds Y)

---

## 21.2 Config Reload Broadcasting

When the config file changes (detected by `ConfigMonitor` file watcher in `oriterm/src/config/monitor/mod.rs`), changes are applied to ALL panes and ALL windows consistently. Some changes (font) require expensive atlas rebuilds and grid reflow.

**File:** `oriterm/src/app/config_reload.rs`

**Reference:** `_old/src/app/config_reload.rs`

- [x] `apply_config_reload(&mut self)`:
  - [x] Load new config from disk via `Config::try_load()` — if parse fails, log warning and return (keep current config)
  - [x] **Color scheme changes** (`apply_color_changes`): if `new.colors != old.colors`:
    - [x] Resolve theme via `new.colors.resolve_theme()`
    - [x] Build palette via `build_palette_from_config()` which calls `scheme::resolve_scheme()` (not `palette::find_scheme`)
    - [x] Apply to ALL panes via `mux.set_pane_theme(pane_id, theme, palette)`
  - [x] **Font changes** (`apply_font_changes`): if any of `size`, `family`, `features`, `fallback`, `weight`, `hinting`, `subpixel_mode`, `variations`, `codepoint_map` changed:
    - [x] Load new `FontSet`, prepend user fallbacks
    - [x] For each window: build `FontCollection` at window-specific DPI, call `renderer.replace_font_collection()`
    - [x] Sync grid layout for all windows via `self.sync_grid_layout()` (handles cell dimension changes, terminal resize, PTY resize)
    - [x] Log: `"config reload: font size={:.1}, cell={}x{}"`
  - [x] **Cursor style changes** (`apply_cursor_changes`): if `new.terminal.cursor_style != old.terminal.cursor_style`:
    - [x] Parse new cursor shape via `new.terminal.cursor_style.to_shape()`
    - [x] Apply to ALL panes via `mux.set_cursor_shape(pane_id, shape)`
  - [x] **Cursor blink interval changes**: if `new.terminal.cursor_blink_interval_ms` changed:
    - [x] Update `self.cursor_blink.set_interval()`
  - [x] **Keybinding changes** (`apply_keybinding_changes`):
    - [x] Rebuild binding table: `self.bindings = keybindings::merge_bindings(&new.keybind)`
  - [x] **Window changes** (`apply_window_changes`): if opacity or blur changed:
    - [x] Apply to ALL windows via `ctx.window.set_transparency(opacity, blur)`
  - [x] **Behavior changes** (`apply_behavior_changes`): if `bold_is_bright` changed:
    - [x] Mark all panes dirty via `mux.mark_all_dirty(pane_id)`
  - [x] **Image changes** (`apply_image_changes`): if image protocol config changed:
    - [x] CPU-side: `mux.set_image_config()` for all panes
    - [x] GPU-side: `renderer.set_image_gpu_memory_limit()` for all windows
  - [x] **Bell changes**: if `new.bell != old.bell`, log info (bell config is read from `self.config` at usage sites, so storing the new config is sufficient — no active broadcasting needed)
  - [x] Store new config: `self.config = new_config`
  - [x] Update UI theme if changed, apply to all tab bars
  - [x] Invalidate pane render caches, mark all windows dirty
- [x] Config fields intentionally not hot-reloaded (require restart):
  - `process_model` — daemon vs. embedded is determined at startup, cannot change at runtime
  - `terminal.shell` — only affects new pane creation (existing panes keep their shell)
  - `terminal.scrollback` — existing panes retain their scrollback size; changing only affects new panes (resizing an active scrollback ring buffer mid-session is destructive and complex)
  - `window.columns`, `window.rows` — initial window size only; current window size is user-controlled
  - `window.decorations` — frameless vs. native titlebar cannot be toggled at runtime on Windows (requires window recreation)
  - `window.resize_increments` — initial window hint only
  - `pane.divider_px`, `pane.min_cells`, `pane.dim_inactive`, `pane.inactive_opacity`, `pane.divider_color`, `pane.focus_border_color` — read from `self.config` at render/resize sites, so storing the new config is sufficient. No explicit broadcast step, but all panes pick up changes on next render.
- [x] File watcher mechanism (`ConfigMonitor` in `oriterm/src/config/monitor/mod.rs`):
  - [x] Uses `notify` crate (`recommended_watcher`) to watch the config directory
  - [x] Also watches `themes/` subdirectory for `.toml` scheme files
  - [x] 200ms debounce: drains rapid-fire events from editors (write-tmp, rename, etc.)
  - [x] Fires `on_change` callback → sends `TermEvent::ConfigReload` via `EventLoopProxy`
  - [x] Event loop dispatches to `App::apply_config_reload()` in the `user_event` handler
  - [x] RAII cleanup: dropping `ConfigMonitor` signals shutdown, drops watcher, joins thread
- [x] `Config::save()` — persist config changes to disk:
  - [x] Write current config to TOML file at `config_path()` (in `oriterm/src/config/io.rs`)
  - [x] Used by dropdown menu scheme selection (and future settings UI) to persist user choices
  - [x] Handle write errors gracefully (log warning, don't crash)
  - [x] Note: `Config::save()` is currently `#[allow(dead_code, reason = "...")]` — it will be used when settings UI (21.3) lands and when scheme selection persists

---

## 21.3 Settings UI

Separate frameless settings window (not an overlay). Displays color scheme selector. GPU-rendered for consistent styling.

**Files (new):**
- `oriterm/src/app/settings_ui/mod.rs` — `SettingsState` struct, lifecycle (`open`, `close`, `is_settings`), constants
- `oriterm/src/app/settings_ui/rendering.rs` — `render_settings_frame()` (pure computation: builds draw primitives, no state mutation)
- `oriterm/src/app/settings_ui/mouse.rs` — `handle_settings_mouse()`, `update_settings_hover()`
- `oriterm/src/app/settings_ui/scheme.rs` — `apply_scheme_to_all_panes()`
- `oriterm/src/app/settings_ui/tests.rs` — sibling test file

**Reference:** `_old/src/app/settings_ui.rs`, `_old/src/gpu/render_settings.rs`

### App state changes

- [x] Add `settings_state: Option<SettingsState>` field on `App` — `None` if settings not open
- [x] `SettingsState` struct (in `settings_ui/mod.rs`):
  - [x] `winit_id: winit::window::WindowId` — the OS window ID
  - [x] `window: Arc<Window>` — raw winit window (not TermWindow — settings has no session identity)
  - [x] `renderer: WindowRenderer` — per-window GPU renderer (fonts, atlases, instance buffers)
  - [x] `hovered_row: Option<usize>` — currently hovered scheme row index
  - [x] `dirty: bool` — redraw needed
- [x] The settings window is NOT stored in `App.windows` — it has no `WindowContext`, no `TabBarWidget`, no `TerminalGridWidget`, no overlay system, no pane cache. It is a separate lightweight window with its own state struct.
- [x] Event routing: the `window_event` handler checks `self.is_settings_window(window_id)` before dispatching to the normal terminal path.

### Settings window lifecycle

- [x] `open_settings_window(event_loop)`:
  - [x] If already open (`settings_state.is_some()`), focus the existing window and return (prevents duplicates — mirrors old prototype behavior)
  - [x] Create a small frameless, non-resizable OS window (~300x350px) via `WindowConfig` with `resizable: false`
  - [x] Create GPU surface directly via `gpu.create_surface(&window)` (raw window, not TermWindow — settings has no session)
  - [x] Create `WindowRenderer` via `create_settings_renderer()` (reuses shared `GpuPipelines`)
  - [x] Build `SettingsState` and store as `self.settings_state = Some(state)`
  - [x] Clear-render initial frame (dark background) before making visible to prevent white flash
  - [x] `window.set_visible(true)`
- [x] `close_settings_window()`:
  - [x] Drop `SettingsState` (releases GPU surface, renderer), set `settings_state = None`
  - [x] Transfer focus back to the most recent terminal window
- [x] `is_settings_window(window_id) -> bool` — check if `settings_state` is `Some` with matching winit ID

### Wiring from ContextAction::Settings

- [x] In `oriterm/src/app/keyboard_input/overlay_dispatch.rs`, the `ContextAction::Settings` arm currently logs `"settings action not yet implemented"`. Replace with:
  - [x] Send `TermEvent::OpenSettings` through the event proxy (settings window creation requires `ActiveEventLoop` which is only available in the `user_event` handler, not during overlay dispatch)
  - [x] Add `TermEvent::OpenSettings` variant to `TermEvent` enum in `oriterm/src/event.rs`
  - [x] Handle in the `user_event` match arm: call `self.open_settings_window(event_loop)`

### Event routing for settings window

- [x] Add `handle_settings_window_event(&mut self, window_id, event)` method (in `settings_ui/mod.rs`):
  - [x] `WindowEvent::CloseRequested` → call `close_settings_window()`
  - [x] `WindowEvent::KeyboardInput` → only Escape (dismiss) is handled; all other keys consumed
  - [x] `WindowEvent::CursorMoved` → call `update_settings_hover()`
  - [x] `WindowEvent::MouseInput` (Left, Pressed) → dispatch to `handle_settings_mouse()`
  - [x] `WindowEvent::RedrawRequested` → mark dirty (rendering handled in `about_to_wait`)
  - [x] `WindowEvent::Resized`, `WindowEvent::ScaleFactorChanged` → handle surface resize + mark dirty
  - [x] All other events → consumed without action (settings window has no terminal)
- [x] In `event_loop.rs` `window_event`, add early guard before the existing match:
  ```rust
  if self.is_settings_window(window_id) {
      self.handle_settings_window_event(window_id, event);
      return;
  }
  ```

### Settings window content

- [x] Title bar: "Theme" label + close button (top-right corner, 30x30px)
- [x] Color scheme list: rows of ~40px height each:
  - [x] Color swatch: 16x16px square showing scheme's background color (with 1px border)
  - [x] Scheme name: text label 40px from left
  - [x] Active indicator: checkmark icon if this is the current scheme
  - [x] Hover highlight: rounded rect across full row width (4px inset from edges)

### Mouse handling (in `settings_ui/mouse.rs`)

- [x] `handle_settings_mouse(&mut self, x: f32, y: f32)`:
  - [x] Top-right 30x30px: close button → `close_settings_window()`
  - [x] Top 50px: title area — supports window drag via `window.drag_window()`
  - [x] Below: scheme rows. `row_idx = (y - TITLE_HEIGHT) / ROW_HEIGHT`
  - [x] Bounds check: `row_idx < scheme_count`
  - [x] Click on row: `apply_settings_scheme(row_idx)`
- [x] `update_settings_hover(&mut self, x: f32, y: f32)`:
  - [x] Compute hovered row index from cursor position
  - [x] Update `settings_state.hovered_row`, mark dirty if changed

### Scheme application (in `settings_ui/scheme.rs`)

- [x] `apply_settings_scheme(&mut self, row_idx: usize)`:
  - [x] Update `self.config.colors.scheme` via `clone_from`
  - [x] Build palette via `palette_from_scheme()` with resolved theme
  - [x] Apply to ALL panes: `mux.set_pane_theme(pane_id, theme, palette)` for each pane
  - [x] Persist to config file: `self.config.save()`
  - [x] `Config::save()` `#[allow(dead_code)]` removed (no longer dead code)
  - [x] Mark all terminal windows dirty + settings window dirty

### GPU rendering (in `settings_ui/rendering.rs`)

- [x] `render_settings_frame(&mut self)` — reads `SettingsState` + `config` immutably to build instance buffers, then submits the frame:
  - [x] Full-window background (dark, derived from palette — uses `darken_rgb(bg, 0.20)`)
  - [x] 1px border on all edges (using palette-derived border color)
  - [x] Title text "Theme" rendered at (16, centered-in-50px-title) using `UiFontMeasurer` (backed by `WindowRenderer::active_ui_collection()`)
  - [x] Close button icon (vector X) in top-right corner
  - [x] Per-row rendering: swatch + name + optional checkmark, with hover highlight for `hovered_row`
  - [x] Color derivation from palette: `darken(bg, 0.20)` for window bg, `lighten(bg, 0.15)` for hover, etc.
  - [x] Uses `SettingsState.renderer`'s draw pipeline (same shaders as terminal windows)
- [x] **Rendering discipline**: This function borrows state immutably to compute draw primitives. No mutation of `config`, `hovered_row`, scheme selection, or any other state. All state changes happen in event handlers (`handle_settings_mouse`, `update_settings_hover`).

### Stretch goal note

This sub-section (21.3) is a stretch goal. The dropdown menu already provides scheme selection with the same functionality. The settings window adds a more polished UX but can be deferred past initial feature parity without blocking 21.6.

**Tests (21.3):** `oriterm/src/app/settings_ui/tests.rs`
- [x] `darken_rgb` identity, black, partial tests
- [x] `rgb_to_color` conversion test
- [x] `lighten_color` identity and white tests
- [x] `build_scheme_list` non-empty test
- [x] `update_settings_hover` row index calculation from Y coordinate
- [x] Scheme row index computation edge cases: y < title height returns None, y at exact row boundary
- [x] Note: `is_settings_window`, `close_settings_window`, `handle_settings_mouse`, `open_settings_window`, and `render_settings_frame` require GPU/winit and cannot be unit tested. Cover via manual verification.

---

## 21.4 Window Controls

Custom window controls for the frameless window, integrated into the tab bar. Platform-specific rendering (rectangular on Windows, circular on macOS/Linux).

**File:** `oriterm_ui/src/widgets/window_chrome/` (control button widgets), `oriterm_ui/src/widgets/tab_bar/widget/controls_draw.rs` (tab bar integration), `oriterm_ui/src/platform_windows/` (Aero Snap subclass)

**Reference:** `_old/src/gpu/render_tab_bar.rs`, `_old/src/tab_bar.rs`

- [x] Three buttons in top-right corner of tab bar:
  - [x] Minimize (─): emits `WidgetAction::WindowMinimize`
  - [x] Maximize (□ / ⧉): emits `WidgetAction::WindowMaximize` — icon changes based on `is_maximized`
  - [x] Close (×): emits `WidgetAction::WindowClose`
- [x] Platform-specific rendering (geometric drawing — no font glyphs needed):
  - [x] **Windows**: Three rectangular buttons, each `CONTROL_BUTTON_WIDTH` (46px) wide:
    - [x] Minimize: horizontal line icon
    - [x] Maximize: single square icon (when not maximized) or two overlapping squares with erase-out (when maximized/restored)
    - [x] Close: X icon (two diagonal lines)
    - [x] Close button hover: red background with white icon
    - [x] Other buttons hover: subtle background change
    - [x] Animated hover transitions (100ms `AnimatedValue`, `EaseOut`)
  - [x] **Linux/macOS**: Circular buttons with themed colors
- [x] Window dragging:
  - [x] Double-click on `DragArea` (empty tab bar space): toggle maximize
  - [x] Click + drag on `DragArea`: `window.drag_window()` — OS handles movement
  - [x] Aero Snap on Windows: handled by OS via `drag_window()` when custom WndProc subclass is installed
- [x] Aero Snap subclass (Windows-specific, `oriterm_ui/src/platform_windows/`):
  - [x] `enable_snap()` installs `SetWindowSubclass` handler with per-window `SnapData`
  - [x] Custom `WndProc` that handles `WM_NCHITTEST` — returns `HTCAPTION` for drag areas, `HTCLIENT` for interactive areas
  - [x] Also handles `WM_DPICHANGED` — stores new DPI via `AtomicU32`, queried via `get_current_dpi()`
  - [x] `set_client_rects()` updates interactive regions on tab bar layout changes
  - [x] OS drag session support for tab tear-off: `begin_os_drag()`, `WM_MOVING` correction, merge detection
  - [x] Modal loop timer (60 FPS `SetTimer`) for rendering during `DragWindow`/`ResizeWindow`
- [x] Keyboard accessibility:
  - [x] `Alt+F4` / `Cmd+Q`: handled by the OS for frameless windows on Windows (winit passes `WM_CLOSE` through). The custom `WndProc` subclass does NOT intercept `WM_SYSCOMMAND`/`SC_CLOSE`, so `Alt+F4` works natively. On macOS, `Cmd+Q` is handled by the AppKit menu system.
  - [x] `Win+Up` (maximize), `Win+Down` (restore/minimize), `Win+Left`/`Win+Right` (snap): all handled by the OS via the Aero Snap subclass. The custom `WndProc` returns `HTCAPTION` for drag areas, which enables the OS's built-in `Win+Arrow` behavior. The `WM_SIZE` / `Resized` event handler picks up the resulting size change.
  - [x] Fullscreen toggle: handled via `Action::ToggleFullscreen` keybinding (F11 by default), dispatched through `execute_action` → `ctx.window.set_fullscreen(!is_fs)`.

---

## 21.5 Taskbar Jump List & Dock Menu

OS-level quick-action menus that appear when the user right-clicks the app icon in the Windows taskbar or macOS dock. These provide fast access to common actions (new tab, new window, profiles) without first focusing the app window.

**Files (new):**
- `oriterm/src/platform/jump_list/mod.rs` — Jump List construction and update (Windows-only, `#[cfg(target_os = "windows")]` at module declaration in `platform/mod.rs`)
- `oriterm/src/platform/jump_list/tests.rs` — sibling test file

**Reference:** Windows Terminal `Jumplist.cpp` (COM-based, profile entries), WezTerm `app.rs` (`applicationDockMenu` — "New Window"), Ghostty `AppDelegate.swift` (dock menu — "New Window" + "New Tab")

**Scope:** Windows Jump List only. macOS Dock Menu and Linux Desktop Actions are deferred to a future section (requires multi-platform build/test infrastructure that does not yet exist).

### Windows — Jump List

Win32 COM API: `ICustomDestinationList` + `IShellLinkW`. Items appear in the taskbar right-click menu and Start menu pin.

**WARNING — `unsafe` code required:**
- [ ] COM FFI calls (`CoCreateInstance`, `ICustomDestinationList` vtable calls, `IShellLinkW` methods, `SetCurrentProcessExplicitAppUserModelID`) are inherently `unsafe`. The `jump_list` module must use `#![allow(unsafe_code, reason = "COM FFI for Jump List construction")]` at the module level. This follows the same pattern as `oriterm_ui/src/platform_windows/` which already allows unsafe for Win32 subclassing.
- [ ] Minimize the unsafe surface: wrap each COM call in a safe helper function that handles `HRESULT` → `Result` conversion. Keep the unsafe blocks as small as possible.

**COM initialization prerequisites:**
- [ ] `CoInitializeEx(COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE)` must be called on the thread that creates Jump List COM objects. The winit event loop thread on Windows already calls `OleInitialize` (which implies `CoInitializeEx(COINIT_APARTMENTTHREADED)`), so Jump List construction on the main thread is safe without an explicit `CoInitialize` call.
- [ ] If Jump List construction is moved to a background thread (e.g., for async profile discovery), that thread MUST call `CoInitializeEx` before any COM calls and `CoUninitialize` on exit. Use an RAII guard: `struct ComGuard; impl Drop for ComGuard { fn drop(&mut self) { CoUninitialize(); } }`.
- [ ] All COM interface pointers (`ICustomDestinationList`, `IShellLinkW`, `IObjectCollection`, `IPropertyStore`) must be released (dropped) before `CoUninitialize`. Rust's drop order handles this naturally if the guard is declared first.
- [ ] `SetCurrentProcessExplicitAppUserModelID(L"Ori.Terminal")` should be called early in `main()` (before window creation) to ensure consistent taskbar grouping and Jump List association. Without this, Windows infers the model ID from the executable path, which breaks if the binary is renamed or moved.

### Architecture: data model vs. COM submission

- [ ] `JumpListTask` struct (pure data, no COM dependency):
  - [ ] `label: String` — display name in the jump list
  - [ ] `arguments: String` — command-line arguments (e.g., `--new-tab`)
  - [ ] `description: String` — tooltip text
- [ ] `build_jump_list_tasks() -> Vec<JumpListTask>` — pure function that builds the task list from config. This is unit-testable without COM.
- [ ] `submit_jump_list(tasks: &[JumpListTask]) -> Result<()>` — COM submission wrapper. Creates `ICustomDestinationList`, constructs `IShellLinkW` per task, commits. This is an integration test only (requires Windows COM runtime).

- [ ] Jump list initialization on app startup:
  - [ ] Build tasks via `build_jump_list_tasks()`
  - [ ] Submit via `submit_jump_list()`
  - [ ] Log result (success or COM error)
- [ ] Built-in tasks (always present):
  - [ ] **New Tab** — launches `ori_term.exe --new-tab` (or reuses running instance via IPC when Section 34 lands)
  - [ ] **New Window** — launches `ori_term.exe --new-window`
- [ ] Profile quick-launch entries (when profile system exists):
  - [ ] One `JumpListTask` per configured profile
  - [ ] Display name: profile name (e.g., "PowerShell", "Ubuntu")
  - [ ] Arguments: `--profile {profile_name}`
  - [ ] Icon: profile icon path if configured, otherwise app icon
  - [ ] Grouped under custom "Profiles" category
- [ ] `IShellLinkW` construction per item (inside `submit_jump_list`):
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
- [ ] **Dependency:** Jump List entries launch `ori_term.exe --new-tab` / `--new-window`. This requires command-line argument parsing in `main()` to be implemented (not yet present). Without it, the launched process would open a default window regardless of arguments. This is a prerequisite or must be co-implemented.

### macOS — Dock Menu (DEFERRED)

Deferred to a future section. Requires macOS build/test infrastructure.

### Linux — Desktop Actions (DEFERRED)

Deferred to a future section. The `.desktop` file is an install-time packaging artifact, not runtime code.

**Tests:** `oriterm/src/platform/jump_list/tests.rs`
- [ ] `build_jump_list_tasks` returns 2 built-in tasks ("New Tab", "New Window") with correct arguments
- [ ] `build_jump_list_tasks` with N profiles returns N + 2 tasks
- [ ] `JumpListTask` fields are correctly populated (label, arguments, description)
- [ ] Note: `submit_jump_list` requires Windows COM runtime and cannot be unit tested. Cover via manual verification on Windows or a `#[cfg(target_os = "windows")] #[ignore]` integration test.

---

## 21.6 Section Completion

Verification that all sub-sections (21.1-21.5) are complete and integrated.

### Sync Points — New Types and Registrations

When implementing 21.3 (Settings UI), the following locations must be updated together:

- [ ] `oriterm/src/event.rs`: add `TermEvent::OpenSettings` variant
- [ ] `oriterm/src/app/event_loop.rs`: add `TermEvent::OpenSettings` match arm in `user_event` handler
- [ ] `oriterm/src/app/event_loop.rs`: add early guard in `window_event` for settings window (before existing match)
- [ ] `oriterm/src/app/event_loop.rs`: in `about_to_wait`, check `settings_state.dirty` alongside terminal window dirty flags
- [ ] `oriterm/src/app/keyboard_input/overlay_dispatch.rs`: replace `ContextAction::Settings` stub with `TermEvent::OpenSettings` send
- [ ] `oriterm/src/app/mod.rs`: add `settings_state: Option<settings_ui::SettingsState>` field to `App` struct
- [ ] `oriterm/src/app/mod.rs`: add `mod settings_ui;` declaration
- [ ] `oriterm/src/app/settings_ui/mod.rs`: `SettingsState` struct, `open_settings_window`, `close_settings_window`, `is_settings_window`, `handle_settings_window_event`
- [ ] `oriterm/src/app/settings_ui/rendering.rs`: `render_settings_frame`
- [ ] `oriterm/src/app/settings_ui/mouse.rs`: `handle_settings_mouse`, `update_settings_hover`
- [ ] `oriterm/src/app/settings_ui/scheme.rs`: `apply_scheme_to_all_panes`
- [ ] `oriterm/src/app/settings_ui/tests.rs`: sibling test file
- [ ] `oriterm/src/app/constructors.rs`: initialize `settings_state: None` in both `App::new()` and `App::new_daemon()`
- [ ] `oriterm/src/config/io.rs`: remove `#[allow(dead_code, reason = "...")]` from `Config::save()` once settings UI calls it

### Feature Checklist

- [ ] All 21.1–21.5 items complete
- [x] Context menu: 3 menu types, GPU-rendered, checkmark entries, shadow rendering, keyboard navigation, full action dispatch chain
- [x] Config reload: broadcast to all panes/windows, `FontCollection` rebuild, grid reflow, file watcher with 200ms debounce
- [ ] Settings UI: separate window with `SettingsState` (not `WindowContext`), color scheme selector, persist to config
- [ ] Settings UI: `TermEvent::OpenSettings` wiring, event routing guard, `about_to_wait` dirty integration
- [x] Window controls: platform-specific rendering, Aero Snap, frameless drag, keyboard accessibility (Alt+F4, Win+Arrow)
- [ ] Jump List (Windows): data model (`JumpListTask`) + COM submission, app user model ID, CLI arg parsing dependency
- [ ] Dock Menu (macOS): DEFERRED — requires macOS build infrastructure
- [ ] Desktop Actions (Linux): DEFERRED — install-time packaging artifact
- [ ] `./build-all.sh` — clean build (cross-compile + host)
- [ ] `./clippy-all.sh` — no warnings (workspace-wide, both targets)
- [ ] `./test-all.sh` — all tests pass (workspace-wide)
- [ ] **Context menu test**: right-click tab, grid, and dropdown button — each menu renders, keyboard-navigates, and dispatches actions correctly
- [ ] **Config reload test**: edit config file while running — font, color scheme, cursor, keybinding, and opacity changes apply to all open panes/windows within 200ms
- [ ] **Settings window test**: open settings, change scheme, verify all terminal windows update colors, close settings, reopen — no orphaned windows, no GPU resource leak
- [ ] **Jump List test** (Windows): right-click taskbar icon — "New Tab" and "New Window" entries appear and launch correctly

**Exit Criteria:** All three menu contexts (tab, grid, dropdown) work with GPU rendering, keyboard navigation, and full action dispatch. Config reload broadcasts to all panes/windows with font rebuild and grid reflow. Settings UI opens as a separate window with color scheme selection that persists to config. Window controls (minimize, maximize, close) render platform-specifically with Aero Snap support. Jump List provides "New Tab" and "New Window" entries in the Windows taskbar. Clean build, zero clippy warnings, all tests pass.
