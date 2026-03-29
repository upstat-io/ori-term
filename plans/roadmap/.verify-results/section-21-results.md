# Section 21: Context Menu & Window Controls — Verification Results

**Verified by:** verify-roadmap agent
**Date:** 2026-03-29
**Section status:** in-progress (21.6 Section Completion is not-started)
**Branch:** dev

## Context Loaded

- `/home/eric/projects/ori_term/.claude/worktrees/verify-roadmap/CLAUDE.md` — full project rules
- `.claude/rules/code-hygiene.md` — file organization, style, 500-line limit
- `.claude/rules/impl-hygiene.md` — module boundaries, data flow, error handling
- `.claude/rules/test-organization.md` — sibling tests.rs pattern
- `plans/roadmap/section-21-context-menu.md` — full section plan (519 lines, read in 4 chunks)

---

## 21.1 Context Menu — VERIFIED COMPLETE

### Files Examined
- `oriterm_ui/src/widgets/menu/mod.rs` (376 lines) — MenuWidget, MenuEntry, MenuStyle
- `oriterm_ui/src/widgets/menu/widget_impl.rs` (335 lines) — Widget trait impl
- `oriterm_ui/src/widgets/menu/tests.rs` (580 lines) — 27 tests
- `oriterm/src/app/context_menu/mod.rs` (145 lines) — ContextAction, ContextMenuState, builders
- `oriterm/src/app/context_menu/tests.rs` (105 lines) — 7 tests

### Tests Run
- `cargo test -p oriterm_ui -- menu`: **27 passed**, 0 failed
- `cargo test -p oriterm -- context_menu`: **7 passed**, 0 failed

### Evidence of Correctness

**MenuEntry enum:** Three variants verified in `mod.rs:19-26` — `Item { label }`, `Check { label, checked }`, `Separator`. Matches plan exactly.

**MenuStyle:** All 20 fields from plan verified present in `mod.rs:44-90`. Includes `from_theme(&UiTheme)` constructor at line 94. Field names and types match plan: `item_height`, `padding_y`, `padding_x`, `min_width`, `extra_width`, `separator_height`, `corner_radius`, `hover_inset`, `hover_radius`, `checkmark_size`, `checkmark_gap`, `bg`, `fg`, `hover_bg`, `separator_color`, `border_color`, `check_color`, `shadow_color`, `border_width`, `font_size`.

**Three menu contexts:**
- `build_dropdown_menu()` at `context_menu/mod.rs:49` — Settings, Separator, About. Verified by test `dropdown_menu_entries` which asserts 3 entries and correct action mapping.
- `build_tab_context_menu(tab_index, tab_id)` at line 70 — Close Tab, Duplicate Tab, Separator, Move to New Window. Verified by `tab_context_menu_entries` and `tab_context_menu_actions` tests.
- `build_grid_context_menu(has_selection)` at line 99 — Copy conditionally included. Verified by `grid_context_menu_with_selection` (8 entries) and `grid_context_menu_without_selection` (7 entries, no Copy).

**Layout calculation:** `total_height()` at line 194, `entry_at_y()` at line 273. Tests verify: min width enforced (`layout_min_width_enforced`), height correct (`layout_height_includes_all_entries` — 3 items + 1 separator + 2 padding_y), empty menu height is padding-only (`layout_empty_menu`), wide labels exceed min width (`layout_wide_label_exceeds_min_width`), check entries add width (`check_entries_affect_layout`).

**Hit testing:** `entry_at_y()` iterates entries, skips separators. Tests: `separator_not_clickable`, `mouse_y_above_padding_clears_hover`, `mouse_y_below_entries_clears_hover`.

**Dismiss conditions:** Escape emits `DismissOverlay` (test `escape_emits_dismiss`). Click selection emits `Selected` (test `click_emits_selected`). Outside-click handled by overlay system (not widget responsibility).

**Keyboard navigation:** Arrow Down/Up skip separators, wrap around. Verified by: `arrow_down_navigates` (skips separator at index 2), `arrow_up_navigates` (wraps to last, skips separator), `keyboard_wraps_around`, `consecutive_separators_skipped`, `all_separators_menu_navigate_returns_false`. Enter and Space activate: `enter_emits_selected`, `space_key_activates`. Focus gate: `not_focused_ignores_keys`.

**Action dispatch chain:** Verified by grepping `overlay_dispatch.rs` — all 9 `ContextAction` variants handled (lines 240-270): Settings, About, CloseTab, DuplicateTab, MoveToNewWindow, Copy, Paste, SelectAll, NewTab.

### Hygiene Check
- All files under 500 lines. Test file at 580 is exempt.
- Sibling `tests.rs` pattern: `mod.rs` ends with `#[cfg(test)] mod tests;` at line 376 (menu) and line 145 (context_menu). Tests use `super::` imports.
- No `unwrap()` in library code. `MenuWidget::new()` is infallible.
- `//!` module doc on all files.

### Verdict: PASS

---

## 21.2 Config Reload Broadcasting — VERIFIED COMPLETE

### Files Examined
- `oriterm/src/app/config_reload.rs` (490 lines) — `apply_config_reload`, delta methods
- `oriterm/src/config/monitor/mod.rs` — ConfigMonitor file watcher
- `oriterm/src/config/monitor/tests.rs` — 7 tests
- `oriterm/src/config/io.rs` — `Config::save()` confirmed at line 48

### Tests Run
- `cargo test -p oriterm -- config::monitor`: **7 passed**, 0 failed

### Evidence of Correctness

**`apply_config_reload()`** at `config_reload.rs:22`: Loads via `Config::try_load()`, returns on error (line 24-26). Applies deltas in order: font, color, cursor, window, behavior, image, keybindings. Stores new config at line 45. Updates UI theme if changed (line 48-54). Invalidates caches and marks dirty (lines 57-62).

**Color changes:** `apply_color_changes()` at line 174. Checks `new.colors == self.config.colors`. Resolves theme via `new.colors.resolve_theme()`. Builds palette via `build_palette_from_config()` (line 183). Applies to ALL panes via iteration (lines 187-189).

**Font changes:** `apply_font_changes()` at line 71. Detects change across 9 fields (size, family, weight, features, fallback, hinting, subpixel_mode, variations, codepoint_map). Loads new FontSet, prepends user fallbacks (line 105). Iterates ALL windows for per-DPI FontCollection creation (lines 114-150). Calls `sync_grid_layout` for grid reflow (lines 156-167). Log message matches plan: `"config reload: font size={:.1}, cell={}x{}"`.

**Cursor changes:** `apply_cursor_changes()` at line 195. Checks style change, applies to all panes. Checks blink interval change, updates `self.cursor_blink.set_interval()`.

**Keybinding changes:** `apply_keybinding_changes()` at line 292. Rebuilds via `keybindings::merge_bindings(&new.keybind)`.

**Window changes:** `apply_window_changes()` at line 219. Checks opacity and blur. Applies to ALL windows.

**Image changes:** `apply_image_changes()` at line 259. CPU-side `mux.set_image_config()` for all panes. GPU-side `renderer.set_image_gpu_memory_limit()` for all windows.

**File watcher:** `ConfigMonitor` in `config/monitor/mod.rs`. Uses `notify` crate. 7 tests verify: `is_theme_file` matching (TOML in themes dir), sender disconnection, debounce timeout.

**Config::save():** Confirmed at `config/io.rs:48`. Used by settings dialog Save button.

### Hygiene Check
- `config_reload.rs` is 490 lines — under 500-line limit.
- No inline test modules (config reload has no sibling tests.rs, but it's integration code requiring App state).

### Verdict: PASS

---

## 21.3 Settings UI — VERIFIED COMPLETE

### Files Examined
- `oriterm/src/app/settings_overlay/mod.rs` (72 lines) — `open_settings_overlay()` fallback
- `oriterm/src/app/settings_overlay/form_builder/mod.rs` — `build_settings_form`, `SettingsIds`
- `oriterm/src/app/settings_overlay/form_builder/tests.rs` (103 lines) — 10 tests
- `oriterm/src/app/settings_overlay/action_handler/mod.rs` — `handle_settings_action`
- `oriterm/src/app/settings_overlay/action_handler/tests.rs` (134 lines) — 10 tests
- `oriterm/src/app/dialog_management.rs` (390 lines) — `open_settings_dialog`, `close_dialog`
- `oriterm/src/app/dialog_rendering.rs` (176 lines) — dialog frame rendering
- `oriterm/src/app/dialog_context/mod.rs` — `DialogWindowContext`, `DialogContent`
- `oriterm_ui/src/widgets/settings_panel/mod.rs` — `SettingsPanel` widget
- `oriterm_ui/src/widgets/dialog/mod.rs` — `DialogWidget`

### Tests Run
- `cargo test -p oriterm -- settings_overlay`: **20 passed**, 0 failed
  - 10 form_builder tests: section count (5), section names (Appearance/Font/Behavior/Terminal/Bell), row counts (2/3/1/2/2=10), IDs distinct, all expanded
  - 10 action_handler tests: theme, opacity, font size, font weight, ligatures on/off, cursor blink, bell duration, unknown ID, paste warning, cursor style

### Evidence of Correctness

**Dialog lifecycle:** `open_settings_dialog()` at `dialog_management.rs:46`. Prevents duplicates via `has_dialog_of_kind(DialogKind::Settings)` (line 98). Creates frameless window centered on parent. Sets `min_inner_size(600x400)` (line 54). Platform ownership via `set_owner()` and `set_window_type()` (lines 132-136). GPU surface + `WindowRenderer::new_ui_only()` (line 149, line 304).

**DialogWindowContext:** Confirmed in `dialog_context/mod.rs:34-67`. Has: `window`, `surface`, `surface_config`, `renderer`, `kind`, `content`, `chrome`, `overlays`, `layer_tree`, `layer_animator`, `text_cache`, `draw_list`, `scale_factor`, `dirty`.

**DialogContent::Settings:** Lines 74-80 confirm `{ panel, ids, pending_config, original_config }`.

**close_dialog():** At line 204. Clears platform modal state, hides window, unregisters from WindowManager, removes context.

**Wiring:** `ContextAction::Settings` in `overlay_dispatch.rs` sends `TermEvent::OpenSettings` (line 242). `event_loop.rs:333` dispatches to `open_settings_dialog()`. `Action::OpenSettings` keybinding confirmed via `action_dispatch.rs:234`.

**Form builder:** 5 sections (Appearance, Font, Behavior, Terminal, Bell) with 10 total rows. All IDs distinct (test `settings_ids_all_distinct`).

**Save/Cancel:** `WidgetAction::SaveSettings` and `CancelSettings` confirmed in `oriterm_ui/src/widgets/mod.rs:177-179`.

**Overlay fallback:** `open_settings_overlay()` at `settings_overlay/mod.rs:24` marked `#[allow(dead_code, reason = "retained for overlay fallback path")]`.

### Hygiene Check
- Dialog management: 390 lines, under limit.
- Sibling tests.rs files for both form_builder and action_handler.

### Verdict: PASS

---

## 21.4 Window Controls — VERIFIED COMPLETE

### Files Examined
- `oriterm_ui/src/widgets/window_chrome/mod.rs` (444 lines) — WindowChromeWidget
- `oriterm_ui/src/widgets/window_chrome/controls.rs` (296 lines) — WindowControlButton
- `oriterm_ui/src/widgets/window_chrome/layout.rs` (166 lines) — ChromeLayout
- `oriterm_ui/src/widgets/window_chrome/constants.rs` (30 lines) — CAPTION_HEIGHT, CONTROL_BUTTON_WIDTH
- `oriterm_ui/src/widgets/window_chrome/tests.rs` (178 lines) — 14 tests
- `oriterm_ui/src/platform_windows/mod.rs` — Aero Snap subclass
- `oriterm_ui/src/platform_windows/subclass.rs` — WndProc handler

### Tests Run
- `cargo test -p oriterm_ui -- window_chrome`: **14 passed**, 0 failed

### Evidence of Correctness

**Three buttons:** `ChromeLayout::compute()` creates 3 controls: Minimize, MaximizeRestore, Close. Verified by test `layout_three_control_buttons` (asserts len=3, correct kinds).

**Widget actions:** `controls.rs:153-155` — Minimize emits `WidgetAction::WindowMinimize`, MaximizeRestore emits `WindowMaximize`, Close emits `WindowClose`.

**Platform rendering:** `WindowChromeWidget::with_theme_and_mode()` creates buttons with themed colors (lines 86-101). Close button has separate `close_hover_bg` and `close_pressed_bg` colors. Full mode = 3 buttons, Dialog mode = 1 button (close only).

**Layout tests:** `layout_restored_caption_height` (CAPTION_HEIGHT), `layout_maximized_caption_height` (CAPTION_HEIGHT_MAXIMIZED), `layout_fullscreen_hidden` (height=0, visible=false), `layout_close_button_at_right_edge` (close.right() == window width), `layout_buttons_ordered_right_to_left`, `layout_buttons_span_full_caption_height`, `layout_title_rect_before_buttons`, `layout_interactive_rects_match_controls`.

**Aero Snap:** `platform_windows/mod.rs` installs `SetWindowSubclass`. `subclass.rs` handles `WM_NCHITTEST` (maps to HTCAPTION/HTCLIENT), `WM_DPICHANGED`, `WM_ENTERSIZEMOVE`/`WM_EXITSIZEMOVE` (modal timer at 60 FPS), `WM_MOVING` (OS drag correction), `WM_NCCALCSIZE` (removes non-client area for frameless).

**Keyboard accessibility:** Plan states Alt+F4 and Win+Arrow handled by OS through WndProc returning HTCAPTION. Subclass confirmed at `subclass.rs` — `map_hit_result` maps `HitTestResult::Caption` to `HTCAPTION`. Fullscreen toggle via `Action::ToggleFullscreen`.

### Hygiene Check
- All files well under 500 lines. `mod.rs` is 444 lines.
- Sibling `tests.rs` at bottom of `mod.rs`.
- Constants extracted to `constants.rs`.
- `#[must_use]` on builder methods confirmed (e.g., `with_style`, `with_selected_index` on MenuWidget).

### Verdict: PASS

---

## 21.5 Taskbar Jump List & Dock Menu — PARTIALLY COMPLETE

### Files Examined
- `oriterm/src/platform/jump_list/mod.rs` (43 lines) — JumpListTask, build_jump_list_tasks
- `oriterm/src/platform/jump_list/windows_impl.rs` (139 lines) — submit_jump_list, create_shell_link
- `oriterm/src/platform/jump_list/tests.rs` (69 lines) — 7 tests
- `oriterm/src/main.rs` — SetCurrentProcessExplicitAppUserModelID, submit_jump_list_on_startup
- `oriterm/src/cli/mod.rs` (438 lines) — Cli struct (no `new_tab` field)
- `oriterm/Cargo.toml` — `windows` crate dependency confirmed

### Tests Run
- `cargo test -p oriterm -- jump_list`: **7 passed**, 0 failed

### Evidence of Correctness

**JumpListTask struct:** `jump_list/mod.rs:14-21` — `label`, `arguments`, `description`. Matches plan.

**build_jump_list_tasks():** Returns Vec with 1 task (New Window only). Test `build_returns_one_built_in_task` asserts `len() == 1`.

**COM submission:** `windows_impl.rs:32-67` — `submit_jump_list()` orchestrates COM transaction: `CoCreateInstance<ICustomDestinationList>`, `BeginList`, `IObjectCollection`, per-task `create_shell_link`, `AddUserTasks`, `CommitList`. Per-function `#[allow(unsafe_code)]` with reason. Helper `set_link_title()` uses `IPropertyStore` and `PKEY_Title`.

**exe_path():** Line 128, uses `std::env::current_exe()`.

**main.rs wiring:** `set_app_user_model_id()` called at line 57 (after `init_logger`, before event loop). `submit_jump_list_on_startup()` called at line 67 (after `build_event_loop`). Confirmed explicit `CoInitializeEx(COINIT_APARTMENTTHREADED)` at line 230.

**Cargo.toml:** `windows` crate with features: `Win32_UI_Shell`, `Win32_UI_Shell_Common`, `Win32_UI_Shell_PropertiesSystem`, `Win32_System_Com`, `Win32_System_Com_StructuredStorage`, `Win32_System_Variant`, `Win32_Storage_EnhancedStorage`.

### DISCREPANCY: --new-tab CLI flag and "New Tab" jump list task MISSING

The plan marks all these as [x] (complete):
- `--new-tab` CLI flag on `Cli` struct
- `if args.new_tab { log::info!("--new-tab requested"); }` in `main()`
- `new_tab_flag_parses`, `new_tab_flag_defaults_to_false`, `completions_contain_new_tab_flag` tests in `cli/tests.rs`
- "New Tab" entry in `build_jump_list_tasks()` returning 2 tasks

**Reality:**
- `Cli` struct at `cli/mod.rs:26-65` has NO `new_tab` field (only `new_window`, `embedded`, `profile`, `connect`, `window`)
- `main.rs` has NO `args.new_tab` check (only `args.new_window` at line 53)
- `cli/tests.rs` has NO `new_tab` tests (confirmed via grep)
- `build_jump_list_tasks()` returns only 1 task ("New Window"), not 2. Test explicitly asserts `len() == 1`.
- The plan's test spec at line 440 says "returns 2 built-in tasks ('New Window', 'New Tab')" but the actual test says "returns one built-in task"

This is a clear plan-vs-code mismatch. The plan checks all `--new-tab` items as done, but the implementation was deliberately scoped to only "New Window". The jump list tests were written to match the actual implementation (1 task), not the plan (2 tasks).

### Hygiene Check
- All files under 500 lines. `submit_jump_list` is 35 lines, `create_shell_link` is 17 lines — well under 50-line limit.
- Per-function `#[allow(unsafe_code, reason = "...")]` instead of module-level — correct.
- `#[cfg_attr(not(windows), allow(dead_code))]` on cross-platform data model — correct.
- Sibling `tests.rs` pattern followed.

### Verdict: PARTIAL PASS — Jump List works for "New Window" only. `--new-tab` CLI flag and "New Tab" jump list entry are NOT implemented despite plan marking them [x].

---

## 21.6 Section Completion — NOT STARTED

Plan status is `not-started`. Feature checklist at line 492 has 2 unchecked items:
- `[ ] All 21.1-21.5 items complete` — cannot be checked due to 21.5 --new-tab gap
- `[ ] Dock Menu (macOS): DEFERRED`
- `[ ] Desktop Actions (Linux): DEFERRED`

Manual verification items (lines 510-516) are all unchecked — expected since they require a running Windows environment.

---

## Summary

| Sub-section | Plan Status | Verified Status | Tests | Issues |
|---|---|---|---|---|
| 21.1 Context Menu | complete | **PASS** | 34 (27 menu + 7 context) | None |
| 21.2 Config Reload | complete | **PASS** | 7 (monitor) | No unit tests for reload logic itself (requires App) |
| 21.3 Settings UI | complete | **PASS** | 20 (10 form + 10 action) | Dialog lifecycle not unit-testable (GPU/winit) |
| 21.4 Window Controls | complete | **PASS** | 14 (chrome layout + button) | None |
| 21.5 Jump List | complete | **PARTIAL** | 7 (jump list data model) | --new-tab missing; plan says [x] but not implemented |
| 21.6 Completion | not-started | NOT STARTED | N/A | Blocked by 21.5 gap |

**Total tests:** 82 passing, 0 failing

### Critical Findings

1. **Plan-code mismatch on --new-tab (21.5):** The plan marks 4 items as [x] complete that do not exist in code:
   - `Cli.new_tab` field
   - `main()` reading `args.new_tab`
   - 3 CLI tests (new_tab_flag_parses, new_tab_flag_defaults_to_false, completions_contain_new_tab_flag)
   - "New Tab" entry in `build_jump_list_tasks()` (returns 1 task, not 2)

   The plan's sync point at line 486 (`cli/mod.rs: add new_tab: bool`) is unchecked in reality. Either the plan needs to be updated to reflect the deliberate scoping decision (1 task only), or the `--new-tab` flag needs to be implemented.

2. **No code hygiene violations found.** All source files under 500 lines. All functions under 50 lines. Sibling tests.rs pattern followed everywhere. Module docs present. No unwrap() in library code.

3. **Test coverage is solid for testable components.** Config reload, dialog lifecycle, and rendering are integration-level (require App/GPU) and are noted in the plan as manual verification.
