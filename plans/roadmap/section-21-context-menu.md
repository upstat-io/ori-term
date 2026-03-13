---
section: 21
title: Context Menu & Window Controls
status: in-progress
reviewed: true
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
    status: complete
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
     - [x] Settings (opens settings dialog)
     - [x] Separator
     - [x] About
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
- [x] `oriterm/src/app/context_menu/tests.rs`: dropdown menu builder (entries, actions, out-of-bounds resolve)
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
  - [x] Used by settings dialog Save button to persist user changes
  - [x] Handle write errors gracefully (log warning, don't crash)

---

## 21.3 Settings UI

Full-featured settings panel with form controls for font, color scheme, cursor, window, and keybinding settings. Implemented as both a modal overlay and a dialog window, GPU-rendered for consistent styling.

**Files:**
- `oriterm/src/app/settings_overlay/mod.rs` — `open_settings_overlay()` (modal overlay path, retained as fallback)
- `oriterm/src/app/settings_overlay/form_builder/mod.rs` — `build_settings_form()`, `SettingsIds` (maps widget IDs to config fields)
- `oriterm/src/app/settings_overlay/form_builder/tests.rs` — form builder tests
- `oriterm/src/app/settings_overlay/action_handler/mod.rs` — `handle_settings_action()` (dispatches widget actions to pending config)
- `oriterm/src/app/settings_overlay/action_handler/tests.rs` — action handler tests
- `oriterm/src/app/dialog_management.rs` — `open_settings_dialog()`, `close_dialog()`, dialog window lifecycle
- `oriterm/src/app/dialog_rendering.rs` — dialog frame rendering
- `oriterm/src/app/dialog_context.rs` — `DialogWindowContext`, `DialogContent` (dialog state)
- `oriterm_ui/src/widgets/settings_panel/mod.rs` — `SettingsPanel` widget
- `oriterm_ui/src/widgets/dialog/mod.rs` — `DialogWidget` (generic dialog frame)

**Reference:** `_old/src/app/settings_ui.rs`, `_old/src/gpu/render_settings.rs`

### App state changes

- [x] Settings uses `settings_ids: Option<SettingsIds>` + `settings_pending: Option<Config>` on `App` — working copy of config, mutated by control changes, applied on Save
- [x] Dialog windows stored in `App.dialogs: HashMap<WindowId, DialogWindowContext>` (separate from `App.windows`)
- [x] `DialogWindowContext` struct (in `dialog_context.rs`): window, surface, renderer, kind, content, scale, chrome
- [x] `DialogContent::Settings { panel, ids, pending_config, original_config }` — settings-specific dialog content
- [x] Overlay path retained as fallback: `open_settings_overlay()` in `settings_overlay/mod.rs` (marked `#[allow(dead_code)]`)
- [x] Event routing: dialog windows use `WindowManager` for kind-based routing, separate from terminal window dispatch

### Settings dialog lifecycle

- [x] `open_settings_dialog(event_loop)` (in `dialog_management.rs`):
  - [x] Prevents duplicates: `has_dialog_of_kind(DialogKind::Settings)` — focuses existing dialog if open
  - [x] Creates frameless dialog window centered on parent via `create_dialog_window()`
  - [x] Sets `min_inner_size(600x400)` for settings dialog
  - [x] Platform ownership: `platform_ops().set_owner()` + `set_window_type(WindowKind::Dialog(kind))`
  - [x] GPU surface + `WindowRenderer::new_ui_only()` (reuses shared `GpuPipelines`, UI font collection only)
  - [x] Builds `SettingsPanel` form via `form_builder::build_settings_form(&config)` with aligned label widths
  - [x] Registers in `WindowManager`, stores in `App.dialogs`, installs platform chrome, renders first frame
- [x] `close_dialog(winit_id)`:
  - [x] Clears platform modal state, unregisters from `WindowManager`, removes `DialogWindowContext`
- [x] Dialog chrome: `install_dialog_chrome()` enables OS-level hit testing (close button, caption drag, resize edges)

### Wiring from ContextAction::Settings

- [x] `ContextAction::Settings` arm in `overlay_dispatch.rs` sends `TermEvent::OpenSettings` through event proxy
- [x] `TermEvent::OpenSettings` variant in `oriterm/src/event.rs`
- [x] `user_event` handler calls `self.open_settings_dialog(event_loop)`
- [x] Also triggered by `Action::OpenSettings` keybinding (Ctrl+, on Windows/Linux, Cmd+, on macOS)

### Settings form and controls

- [x] `build_settings_form()` constructs a `FormWidget` with grouped controls:
  - [x] Font settings: size, family, weight, hinting, subpixel mode
  - [x] Color settings: scheme selector (dropdown), theme override
  - [x] Cursor settings: style, blink interval
  - [x] Window settings: opacity, blur
- [x] `SettingsIds` struct: maps `WidgetId` for each control to its config field
- [x] Dropdown controls: emit `WidgetAction::OpenDropdown` → popup overlay with `MenuWidget`
- [x] Pending config pattern: control changes mutate `settings_pending` (working copy), `self.config` untouched until Save

### Save/Cancel flow (in `overlay_dispatch.rs`)

- [x] `WidgetAction::SaveSettings` → `save_settings()`:
  - [x] Takes pending config, swaps into `self.config`
  - [x] Calls `apply_settings_change(old_config)` which applies font/color/cursor/window deltas
  - [x] Persists to disk via `self.config.save()`
  - [x] Dismisses overlay/dialog
- [x] `WidgetAction::CancelSettings` → `cancel_settings()`:
  - [x] Discards `settings_pending`, dismisses overlay/dialog
- [x] `apply_settings_change()`: temporarily swaps old config back so delta-comparison methods work correctly, then restores new config

**Tests (21.3):**
- [x] `oriterm/src/app/settings_overlay/form_builder/tests.rs`: `SettingsIds` field uniqueness, form construction
- [x] `oriterm/src/app/settings_overlay/action_handler/tests.rs`: action dispatch to pending config fields
- [x] Note: `open_settings_dialog`, `close_dialog`, dialog rendering require GPU/winit — manual verification

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
- `oriterm/src/platform/jump_list/mod.rs` — `JumpListTask` struct (not platform-gated), `build_jump_list_tasks()` (not platform-gated), `#[cfg(target_os = "windows")] submit_jump_list()` (COM code, Windows-only), `#[cfg(target_os = "windows")] exe_path()`. Module declaration in `platform/mod.rs` is unconditional (`pub mod jump_list;`) so that the data model compiles and tests run on all platforms.
- `oriterm/src/platform/jump_list/tests.rs` — sibling test file (tests `build_jump_list_tasks` on all platforms)

**WARNING — file size discipline:** The `mod.rs` file will contain the cross-platform data model (~30 lines), `build_jump_list_tasks` (~15 lines), and the `#[cfg(windows)]` COM functions (`submit_jump_list` ~80 lines, `exe_path` ~10 lines, COM helpers ~40 lines). Estimated total: ~200-250 lines including docs and imports. Well within the 500-line limit. If `submit_jump_list` grows beyond 50 lines, extract COM helper functions (`create_shell_link`, `create_task_collection`) as private helpers within the same file — do NOT let the function exceed 50 lines (code-hygiene.md).

**Reference:** Windows Terminal `Jumplist.cpp` (COM-based, profile entries), WezTerm `app.rs` (`applicationDockMenu` — "New Window"), Ghostty `AppDelegate.swift` (dock menu — "New Window" + "New Tab")

**Scope:** Windows Jump List only. macOS Dock Menu and Linux Desktop Actions are deferred to a future section (requires multi-platform build/test infrastructure that does not yet exist).

### Windows — Jump List

Win32 COM API: `ICustomDestinationList` + `IShellLinkW`. Items appear in the taskbar right-click menu and Start menu pin.

**Crate dependency:**
- [x] Add `windows` crate (Windows-only) to `oriterm/Cargo.toml` under `[target.'cfg(windows)'.dependencies]` with features: `"Win32_UI_Shell"`, `"Win32_UI_Shell_PropertiesSystem"`, `"Win32_System_Com"`, `"Win32_System_Com_StructuredStorage"`. The `windows` crate provides COM interface wrappers (`ICustomDestinationList`, `IShellLinkW`, `IObjectCollection`) with safe Rust APIs. The existing `windows-sys` crate only provides function declarations and structs, not COM vtable wrappers.
- [x] `IPropertyStore` and `PKEY_Title` live in `windows::Win32::UI::Shell::PropertiesSystem` (requires the `Win32_UI_Shell_PropertiesSystem` feature). `PROPVARIANT` is in `windows::Win32::System::Com::StructuredStorage` (requires `Win32_System_Com_StructuredStorage`).
- [x] Verify cross-compilation: the `windows` crate must compile for `x86_64-pc-windows-gnu` (the project's cross-compile target from WSL). Run `./build-all.sh` after adding the dependency to confirm.
- [x] Alternatively, use `windows-sys` with manual raw COM vtable FFI (not recommended — error-prone and verbose).

**WARNING — `unsafe` code required:**
- [x] With the `windows` crate, COM interface method calls (`BeginList`, `AddUserTasks`, `CommitList`, `SetPath`, `SetArguments`, etc.) are **safe** — the crate handles vtable dispatch internally. Only `CoCreateInstance` and `SetCurrentProcessExplicitAppUserModelID` require `unsafe`.
- [x] Since the module is unconditional (data model is cross-platform), use `#[allow(unsafe_code, reason = "COM FFI for Jump List construction")]` on the individual `#[cfg(target_os = "windows")]` functions that call COM APIs (`submit_jump_list`, `create_shell_link`) rather than module-level `#![allow(unsafe_code)]`. This keeps the lint active for cross-platform data model code.
- [x] `SetCurrentProcessExplicitAppUserModelID` can use the **existing** `windows-sys` crate (already has `Win32_UI_Shell` feature enabled). It lives in `main.rs`, not in the `jump_list` module, so it uses the existing `#[allow(unsafe_code)]` pattern already present in `cli::attach_console()`.
- [x] Minimize the unsafe surface: wrap each raw COM call in a safe helper function that handles `HRESULT` → `Result` conversion. Keep the unsafe blocks as small as possible.

**COM initialization prerequisites:**
- [x] `CoInitializeEx(COINIT_APARTMENTTHREADED)` must be called on the thread before any COM object creation. Winit calls `OleInitialize` during **window creation** (`winit-0.30.12/src/platform_impl/windows/window.rs:1168`), not during event loop creation. Since `submit_jump_list` runs in `main()` before `run_app()` (and thus before any window is created), an explicit `CoInitializeEx` call is required. The subsequent winit `OleInitialize` (which itself calls `CoInitializeEx`) will harmlessly return `S_FALSE` (already initialized). Use `CoInitializeEx` from the new `windows` crate's `Win32_System_Com` feature (added for COM interfaces).
- [x] If Jump List construction is moved to a background thread (e.g., for async profile discovery), that thread MUST call `CoInitializeEx` before any COM calls and `CoUninitialize` on exit. Use an RAII guard: `struct ComGuard; impl Drop for ComGuard { fn drop(&mut self) { CoUninitialize(); } }`.
- [x] All COM interface pointers (`ICustomDestinationList`, `IShellLinkW`, `IObjectCollection`, `IPropertyStore`) must be released (dropped) before `CoUninitialize`. Rust's drop order handles this naturally if the guard is declared first. The `windows` crate handles `Release` automatically via `Drop`.
- [x] `SetCurrentProcessExplicitAppUserModelID(L"Ori.Terminal")` should be called early in `main()` (before window creation) to ensure consistent taskbar grouping and Jump List association. Without this, Windows infers the model ID from the executable path, which breaks if the binary is renamed or moved. Use `windows_sys::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID` with a `PCWSTR` (already available via the existing `Win32_UI_Shell` feature in `windows-sys`).
- [x] Place the call inside a `#[cfg(windows)]` block in `main()`, after `init_logger()` but before `build_event_loop()`. Construct the wide string using the project's existing pattern: `OsStr::new("Ori.Terminal").encode_wide().chain(Some(0)).collect::<Vec<u16>>()` (see `platform/url/mod.rs` for example). The `HRESULT` return should be logged but not fatal (`S_OK` on success, `E_INVALIDARG` if the ID exceeds 128 chars).

### Architecture: data model vs. COM submission

- [x] `JumpListTask` struct (pure data, no COM dependency):
  - [x] `label: String` — display name in the jump list
  - [x] `arguments: String` — command-line arguments (e.g., `--new-window`)
  - [x] `description: String` — tooltip text
- [x] `build_jump_list_tasks() -> Vec<JumpListTask>` — pure function that builds the task list from config. This is unit-testable without COM.
- [x] `exe_path() -> std::io::Result<std::path::PathBuf>` — helper to resolve the path to the running `oriterm` binary via `std::env::current_exe()`. Called inside `submit_jump_list` for `IShellLinkW::SetPath`. Must handle the case where `current_exe()` fails (e.g., `/proc/self/exe` unreadable) by returning an error that `submit_jump_list` propagates.
- [x] `submit_jump_list(tasks: &[JumpListTask]) -> windows::core::Result<()>` — COM submission wrapper. Split into helpers to stay under 50 lines per function (code-hygiene.md). Recommended split:
  - `submit_jump_list()` — orchestrates the COM transaction (< 30 lines)
  - `create_shell_link(exe: &Path, task: &JumpListTask) -> windows::core::Result<IShellLinkW>` — creates and configures one shell link (< 30 lines)
  Step-by-step COM transaction (across the helper functions):
  1. `CoCreateInstance::<ICustomDestinationList>(CLSID_DestinationList)` — create destination list
  2. `dest_list.BeginList()` — returns `(max_slots, IObjectArray)` of removed items (must be queried even if ignored)
  3. `CoCreateInstance::<IObjectCollection>(CLSID_EnumerableObjectCollection)` — create task collection
  4. For each `JumpListTask`, call `create_shell_link()`:
     a. `CoCreateInstance::<IShellLinkW>(CLSID_ShellLink)` — create shell link
     b. `link.SetPath(exe_path)` — set executable path
     c. `link.SetArguments(task.arguments)` — set CLI args
     d. `link.SetDescription(task.description)` — set tooltip
     e. `link.cast::<IPropertyStore>()` — QI for property store
     f. `prop_store.SetValue(&PKEY_Title, &PROPVARIANT::from(task.label))` — set display name
     g. `prop_store.Commit()` — flush property store
     h. `collection.AddObject(&link)` — add to collection
  5. `dest_list.AddUserTasks(&collection)` — add tasks to "Tasks" category
  6. `dest_list.CommitList()` — commit the jump list
- [x] All string arguments to COM methods must be converted to wide strings (`HSTRING` or `PCWSTR`). The `windows` crate's `HSTRING` type handles this. For `PROPVARIANT`, use `PROPVARIANT::from(&HSTRING::from(label))` or construct manually with `VT_LPWSTR`.

- [x] Jump list initialization on app startup:
  - [x] Build tasks via `build_jump_list_tasks()`
  - [x] Submit via `submit_jump_list()`
  - [x] Log result (success or COM error)
  - [x] **Call site:** Do NOT add to `App::try_init()` — that function is already long (208 lines with `#[expect(clippy::too_many_lines)]`) and handles window/GPU/font/mux/renderer/tab creation. Instead, place both calls in `main()`:
    - `SetCurrentProcessExplicitAppUserModelID` goes in `main()` after `init_logger()` and before `build_event_loop()` (flat Win32 API, no COM needed).
    - `submit_jump_list` goes in `main()` after `build_event_loop()` but before `event_loop.run_app()`, preceded by an explicit `CoInitializeEx(COINIT_APARTMENTTHREADED)` call. Winit does not initialize COM until window creation (inside `App::resumed`/`try_init`), so COM is not yet available at this point. The explicit init is harmless — winit's subsequent `OleInitialize` returns `S_FALSE` (already initialized). This is the pattern Windows Terminal uses.
- [x] Built-in tasks (always present):
  - [x] **New Window** — launches `oriterm.exe --new-window` (flag already exists in `Cli` struct)
  - [x] **New Tab** — launches `oriterm.exe --new-tab` (flag does NOT exist yet — see dependency note below)
- [x] Error handling: Jump list APIs may fail (Explorer not running, COM init failure, `current_exe()` failure) — log and continue, never crash. The `submit_jump_list` return value is logged at `warn` level, not propagated to the caller.
- [x] **Dependency:** Jump List entries launch `oriterm --new-tab` / `--new-window`. The `--new-window` flag already exists in `oriterm/src/cli/mod.rs` (clap-based `Cli` struct). However, `--new-tab` does not exist yet — it must be added to the CLI and dispatched in `main()`. This is a prerequisite for the "New Tab" jump list entry.
- [x] **`--new-tab` CLI flag** — add to `Cli` struct in `oriterm/src/cli/mod.rs`:
  - [x] `#[arg(long)] pub new_tab: bool` — mirrors the existing `new_window` flag pattern (see line 49 of `cli/mod.rs`)
  - [x] In `main()`: add `if args.new_tab { log::info!("--new-tab requested"); }` after the existing `if args.new_window` block (line 45-47 of `main.rs`). The actual tab-in-existing-window behavior requires IPC (Section 34). For now, `--new-tab` launches a new window with one tab (same as default behavior). This is the same approach WezTerm takes before its mux daemon is running.
  - [x] The `dead_code = "deny"` workspace lint will fire if `new_tab` is added to `Cli` but never read. The `log::info!` in `main()` satisfies this.
  - [x] Add the `--new-tab` CLI flag FIRST (before jump list code), since `build_jump_list_tasks()` references `"--new-tab"` as an argument string. The flag must exist and be tested before the jump list code that refers to it.

### Profile entries (FUTURE — no profile system yet)

Profile quick-launch entries are deferred until the profile system is implemented. When that happens:
- [ ] One `JumpListTask` per configured profile
- [ ] Display name: profile name (e.g., "PowerShell", "Ubuntu")
- [ ] Arguments: `--profile {profile_name}`
- [ ] Icon: profile icon path if configured, otherwise app icon
- [ ] Grouped under custom "Profiles" category (via `ICustomDestinationList::AppendCategory`)
- [ ] Update triggers on profile add/remove/rename and on config reload
- [ ] `IShellLinkW::SetIconLocation()` for per-profile icons (requires `.ico` file path)
- [ ] Auto-detect shells: PowerShell, Command Prompt, WSL distros (dynamic profile sources)

**Reference:** Windows Terminal `Jumplist.cpp` uses `ActiveProfiles()` from its settings model. Profiles are auto-discovered via dynamic profile sources (WSL, PowerShell Core, Visual Studio). Jump list is rebuilt lazily on settings change (hash comparison). Uses "Tasks" category (not "Destinations"). Full replacement on each update (clear + re-add) rather than delta tracking. See `~/projects/reference_repos/console_repos/terminal/src/cascadia/TerminalApp/Jumplist.cpp`.

### Implementation order

The items above must be implemented in this order due to compile-time dependencies:

1. **`--new-tab` CLI flag** — add field to `Cli`, read in `main()`, add tests in `cli/tests.rs`. Verify: `./build-all.sh && ./test-all.sh`.
2. **`windows` crate dependency** — add to `Cargo.toml`. Verify: `./build-all.sh` (cross-compile must succeed).
3. **`jump_list/mod.rs` + `jump_list/tests.rs`** — create module with `JumpListTask`, `build_jump_list_tasks()`, `#[cfg(windows)]` COM functions. Add `pub mod jump_list;` to `platform/mod.rs`. Verify: `./build-all.sh && ./test-all.sh`.
4. **`SetCurrentProcessExplicitAppUserModelID` in `main.rs`** — early in `main()`, after `init_logger()`, before `build_event_loop()`. Uses existing `windows-sys` (no new dependency).
5. **`submit_jump_list` call in `main.rs`** — after step 4, with explicit `CoInitializeEx(COINIT_APARTMENTTHREADED)` since winit has not initialized COM yet at this point (winit calls `OleInitialize` during window creation, not event loop creation).
6. **Final verification** — `./build-all.sh && ./clippy-all.sh && ./test-all.sh`.

### `IShellLinkW` construction detail (inside `create_shell_link` helper)

- [x] `SetPath()` → absolute path to `oriterm.exe` from `exe_path()`
- [x] `SetArguments()` → command-line args for the action (e.g., `--new-window`)
- [x] `SetDescription()` → tooltip text (e.g., "Open a new terminal window")
- [x] `IPropertyStore::SetValue(PKEY_Title)` → display name (e.g., "New Window")
- [x] `IPropertyStore::Commit()` — required after `SetValue` to flush the property store
- [x] Note: `PKEY_AppUserModel_ID` is NOT needed per-link — the process-level `SetCurrentProcessExplicitAppUserModelID` in `main()` handles taskbar grouping. Per-link model IDs are only needed for cross-process scenarios.

### macOS — Dock Menu (DEFERRED)

Deferred to a future section. Requires macOS build/test infrastructure.

### Linux — Desktop Actions (DEFERRED)

Deferred to a future section. The `.desktop` file is an install-time packaging artifact, not runtime code.

**Tests:** `oriterm/src/platform/jump_list/tests.rs`
- [x] `build_jump_list_tasks` returns 2 built-in tasks ("New Window", "New Tab") with correct arguments (`--new-window`, `--new-tab`)
- [x] `build_jump_list_tasks` returns tasks with correct labels and descriptions (human-readable, not empty)
- [x] `JumpListTask` fields are correctly populated (label, arguments, description) — verify each field is non-empty
- [x] Task argument strings match CLI flag names exactly (`--new-window` not `--new_window`, `--new-tab` not `--new_tab`) — the jump list launches a new process via CLI
- [x] Note: `submit_jump_list` requires Windows COM runtime and cannot be unit tested. Cover via manual verification on Windows or a `#[cfg(target_os = "windows")] #[ignore]` integration test.
- [x] Note: `exe_path()` cannot be meaningfully unit tested (depends on `/proc/self/exe` or equivalent). Tested indirectly via `submit_jump_list` integration test.
- [x] Module structure: `mod.rs` ends with `#[cfg(test)] mod tests;` (sibling test file pattern per `test-organization.md`)

---

## 21.6 Section Completion

Verification that all sub-sections (21.1-21.5) are complete and integrated.

### Sync Points — Settings UI (21.3, COMPLETE)

These were required for 21.3 and are already implemented:

- [x] `oriterm/src/event.rs`: `TermEvent::OpenSettings` variant
- [x] `oriterm/src/app/event_loop.rs`: `TermEvent::OpenSettings` match arm calls `self.open_settings_dialog(event_loop)`
- [x] `oriterm/src/app/event_loop.rs`: dialog window event routing via `WindowManager` kind checks
- [x] `oriterm/src/app/keyboard_input/overlay_dispatch.rs`: `ContextAction::Settings` sends `TermEvent::OpenSettings`
- [x] `oriterm/src/app/mod.rs`: `settings_ids: Option<SettingsIds>` + `settings_pending: Option<Config>` on `App`
- [x] `oriterm/src/app/mod.rs`: `mod settings_overlay;` + `mod dialog_management;` declarations
- [x] `oriterm/src/app/settings_overlay/mod.rs`: `open_settings_overlay()` (overlay fallback path)
- [x] `oriterm/src/app/settings_overlay/form_builder/mod.rs`: `build_settings_form()`, `SettingsIds`
- [x] `oriterm/src/app/settings_overlay/action_handler/mod.rs`: `handle_settings_action()`
- [x] `oriterm/src/app/dialog_management.rs`: `open_settings_dialog()`, `close_dialog()`, `DialogWindowContext`
- [x] `oriterm/src/app/constructors.rs`: `settings_ids: None` + `settings_pending: None` in both `App::new()` and `App::new_daemon()`
- [x] `oriterm/src/config/io.rs`: `Config::save()` is actively used (no `#[allow(dead_code)]`)

### Sync Points — Jump List (21.5, COMPLETE)

All sync points for 21.5 have been implemented:

**Cargo.toml:**
- [x] `oriterm/Cargo.toml`: add `windows` crate under `[target.'cfg(windows)'.dependencies]` with features: `"Win32_UI_Shell"`, `"Win32_UI_Shell_PropertiesSystem"`, `"Win32_System_Com"`, `"Win32_System_Com_StructuredStorage"`

**New files:**
- [x] `oriterm/src/platform/jump_list/mod.rs`: `JumpListTask` struct (unconditional), `build_jump_list_tasks()` (unconditional), `#[cfg(target_os = "windows")] submit_jump_list()` + `create_shell_link()` helper (with per-function `#[allow(unsafe_code)]`), `#[cfg(target_os = "windows")] exe_path()`; ends with `#[cfg(test)] mod tests;`
- [x] `oriterm/src/platform/jump_list/tests.rs`: sibling test file (runs on all platforms)

**Modified files (must update ALL):**
- [x] `oriterm/src/platform/mod.rs`: add `pub mod jump_list;` — unconditional module declaration (data model is cross-platform; COM code is `#[cfg(windows)]` inside the module)
- [x] `oriterm/src/main.rs`: add `#[cfg(windows)]` block after `init_logger()` calling `SetCurrentProcessExplicitAppUserModelID` via `windows_sys` (existing dependency, no new crate needed for this call — `Win32_UI_Shell` feature already enabled)
- [x] `oriterm/src/main.rs`: add `#[cfg(windows)]` block in `main()` with explicit `CoInitializeEx(COINIT_APARTMENTTHREADED)` + `submit_jump_list(build_jump_list_tasks())` call. Place after `init_logger()` and the `SetCurrentProcessExplicitAppUserModelID` call. Log result at `warn` level on failure, do not fail startup on error. The explicit COM init is needed because winit does not call `OleInitialize` until window creation (inside `App::resumed`).
- [x] `oriterm/src/cli/mod.rs`: add `#[arg(long)] pub new_tab: bool` to `Cli` struct
- [x] `oriterm/src/main.rs`: add `if args.new_tab { log::info!("--new-tab requested"); }` after the existing `if args.new_window` block (prevents `dead_code` lint)
- [x] `oriterm/src/cli/tests.rs`: add tests mirroring the existing `--new-window` test pattern (see `new_window_flag_parses`, `new_window_flag_defaults_to_false` in `cli/tests.rs`): `new_tab_flag_parses`, `new_tab_flag_defaults_to_false`, `completions_contain_new_tab_flag`

### Feature Checklist

- [ ] All 21.1–21.5 items complete
- [x] Context menu: 3 menu types (tab, grid, dropdown), GPU-rendered, keyboard navigation, full action dispatch chain
- [x] Config reload: broadcast to all panes/windows, `FontCollection` rebuild, grid reflow, file watcher with 200ms debounce
- [x] Settings UI: dialog window with `DialogWindowContext`, form-based settings panel (font, color, cursor, window), Save/Cancel flow, persists to config via `Config::save()`
- [x] Settings UI: `TermEvent::OpenSettings` wiring, `Action::OpenSettings` keybinding (Ctrl+,/Cmd+,), dialog event routing
- [x] Window controls: platform-specific rendering, Aero Snap, frameless drag, keyboard accessibility (Alt+F4, Win+Arrow)
- [x] Jump List (Windows): data model (`JumpListTask`) + COM submission via `windows` crate, app user model ID, `--new-tab` CLI flag
- [ ] Dock Menu (macOS): DEFERRED — requires macOS build infrastructure
- [ ] Desktop Actions (Linux): DEFERRED — install-time packaging artifact

### Build Verification

- [x] `./build-all.sh` — clean build (cross-compile `x86_64-pc-windows-gnu` + host). Must verify that the new `windows` crate dependency compiles for the cross-compile target.
- [x] `./clippy-all.sh` — no warnings (workspace-wide, both targets). Watch for: `dead_code` on `new_tab` field if not read in `main()`, `unsafe_code` deny on COM functions (must have per-function `#[allow(unsafe_code)]`), clippy `too_many_lines` — `submit_jump_list` and `create_shell_link` must each be < 50 lines.
- [x] `./test-all.sh` — all tests pass (workspace-wide). The `jump_list` tests run on the host (Linux) because `JumpListTask` and `build_jump_list_tasks()` are not platform-gated. COM tests (`submit_jump_list`) are `#[cfg(target_os = "windows")]` and only run on Windows CI.

### Manual Verification (Windows only)

- [ ] **Context menu test**: right-click tab, grid, and dropdown button — each menu renders, keyboard-navigates, and dispatches actions correctly
- [ ] **Config reload test**: edit config file while running — font, color scheme, cursor, keybinding, and opacity changes apply to all open panes/windows within 200ms
- [ ] **Settings dialog test**: open settings (Ctrl+, or dropdown menu), change settings, Save — verify all terminal windows update, Cancel — verify no changes applied. Reopen — no orphaned windows, no GPU resource leak
- [ ] **Jump List test** (Windows): right-click taskbar icon — "New Window" and "New Tab" entries appear and launch correctly
- [ ] **Jump List error resilience** (Windows): verify that Jump List COM failure does not prevent app startup — intentionally break COM (e.g., corrupt the AppUserModelID) and confirm the app still launches with a log warning
- [ ] **`--new-tab` CLI test**: run `oriterm --new-tab` from terminal — verify it launches (new window with one tab for now; proper IPC dispatch deferred to Section 34)
- [ ] **Cross-compile smoke test**: after adding `windows` crate, run `cargo build --target x86_64-pc-windows-gnu` to confirm the new dependency links correctly under MinGW

**Exit Criteria:** All three menu contexts (tab, grid, dropdown) work with GPU rendering, keyboard navigation, and full action dispatch. Config reload broadcasts to all panes/windows with font rebuild and grid reflow. Settings dialog opens with form controls for all major config sections, Save persists to disk and applies changes, Cancel discards. Window controls (minimize, maximize, close) render platform-specifically with Aero Snap support. Jump List provides "New Window" and "New Tab" entries in the Windows taskbar. `SetCurrentProcessExplicitAppUserModelID` is called early in `main()` (before event loop) for consistent taskbar grouping. `submit_jump_list` is called in `main()` with explicit `CoInitializeEx` before `run_app()`. `--new-tab` CLI flag is recognized (dispatches to default behavior until IPC lands in Section 34). All source files < 500 lines, all functions < 50 lines. Clean build on both host and cross-compile target, zero clippy warnings, all tests pass.
