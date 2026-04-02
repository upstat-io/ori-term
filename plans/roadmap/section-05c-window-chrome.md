---
section: 5C
title: Window Chrome (Title Bar + Controls)
status: complete
reviewed: true
last_verified: "2026-03-31"
tier: 2
goal: Render a visible title bar with minimize/maximize/close controls, wire platform integration (Aero Snap, drag), offset the terminal grid below the caption bar
third_party_review:
  status: none
  updated: null
sections:
  - id: "5C.1"
    title: ChromeLayout (Pure Geometry)
    status: complete
  - id: "5C.2"
    title: WindowControlButton Widget
    status: complete
  - id: "5C.3"
    title: WindowChromeWidget Container
    status: complete
  - id: "5C.4"
    title: "App Integration — Init + Redraw"
    status: complete
  - id: "5C.5"
    title: "App Integration — Events + Resize"
    status: complete
  - id: "5C.6"
    title: Platform Polish
    status: complete
  - id: "5C.7"
    title: Tests + Verification
    status: complete
---

# Section 05C: Window Chrome (Title Bar + Controls)

**Status:** Complete
**Goal:** Render a visible title bar with minimize/maximize/close controls, wire platform integration (Aero Snap on Windows, drag on all platforms), and offset the terminal grid below the caption bar. Bridges the gap between "frameless window" (Section 5) and "tab bar with drag/animation" (Section 16).

**Crate:** `oriterm_ui` (widget layer) + `oriterm` (app integration)
**Dependencies:** Section 5 (Window + GPU), Section 7 (2D UI Framework — Widget trait, DrawList, themes)

---

## 5C.1 ChromeLayout (Pure Geometry)

- [x] `constants.rs` — layout constants (CAPTION_HEIGHT, CAPTION_HEIGHT_MAXIMIZED, CONTROL_BUTTON_WIDTH, RESIZE_BORDER_WIDTH, SYMBOL_SIZE) (verified 2026-03-29 — CLOSE_HOVER_COLOR and SYMBOL_STROKE_WIDTH not present; symbols are now icon-atlas-based, close hover color is theme-derived via UiTheme.close_hover_bg)
- [x] `layout.rs` — `ControlKind` enum (Minimize, MaximizeRestore, Close) (verified 2026-03-29 — layout.rs:17-25)
- [x] `ControlRect` struct (kind + rect) (verified 2026-03-29 — layout.rs:38-43)
- [x] `ChromeLayout` struct (caption_height, title_rect, controls, interactive_rects, visible) (verified 2026-03-29 — layout.rs:49-66, also includes `mode` and `ChromeMode` enum for dialog support)
- [x] `ChromeLayout::compute()` — pure geometry from window_width, is_maximized, is_fullscreen (verified 2026-03-29 — layout.rs:73, 11 tests)
- [x] `ChromeLayout::hidden()` — fullscreen returns zero-height invisible layout (verified 2026-03-29 — layout.rs:156-165)

## 5C.2 WindowControlButton Widget

- [x] `controls.rs` — `WindowControlButton` implementing `Widget` (verified 2026-03-29 — 296 lines)
- [x] Three kinds: Minimize, MaximizeRestore, Close (verified 2026-03-29 — action() maps all three at lines 152-156)
- [x] Geometric symbol drawing (minimize dash, maximize square, restore overlapping squares, close X) (verified 2026-03-29 — evolved to icon-atlas-based via IconId::Minimize/Maximize/Restore/WindowClose at lines 161-213)
- [x] Hover animation via `AnimatedValue<f32>` (100ms, EaseOut) (verified 2026-03-29 — line 79, HOVER_DURATION=100ms)
- [x] Close button: red hover bg (#C42B1C), white foreground on hover (verified 2026-03-29 — close_hover_bg field, current_fg lerps to Color::WHITE at lines 123-129)
- [x] Emits `WidgetAction::WindowMinimize/WindowMaximize/WindowClose` (verified 2026-03-29 — lines 151-157)
- [x] `is_pressed()` accessor for parent container routing (verified 2026-03-29 — line 96)

## 5C.3 WindowChromeWidget Container

- [x] `mod.rs` — `WindowChromeWidget` composing title + 3 control buttons (verified 2026-03-29 — 444 lines, now used only for dialogs; main window uses TabBarWidget with embedded WindowControlButtons)
- [x] `with_theme()` constructor derives colors from `UiTheme` (verified 2026-03-29 — lines 67-69)
- [x] Active/inactive caption background colors (darken helper) (verified 2026-03-29 — current_caption_bg at lines 203-209)
- [x] Caption background rect + title text drawing (verified 2026-03-29 — draw() at lines 234-275)
- [x] Mouse event routing to control buttons (verified 2026-03-29 — handle_mouse at lines 277-316)
- [x] `update_hover()` — hover enter/leave to control buttons (verified 2026-03-29 — lines 354-405)
- [x] `caption_height()`, `interactive_rects()`, `is_visible()` accessors (verified 2026-03-29 — lines 124-139)
- [x] `set_title()`, `set_active()`, `set_maximized()`, `set_fullscreen()`, `set_window_width()` state updates (verified 2026-03-29 — lines 145-173)

## 5C.4 App Integration — Init + Redraw

- [x] Create `WindowChromeWidget` in `init/mod.rs` after renderer (verified 2026-03-29 — install_chrome() at init/mod.rs:274 with ChromeMode::Main)
- [x] Wire `enable_snap()` on Windows with scaled caption height (verified 2026-03-29 — evolved to NativeChromeOps::install_chrome() trait method)
- [x] Wire `set_client_rects()` on Windows with scaled interactive rects (verified 2026-03-29 — evolved to NativeChromeOps::set_interactive_rects())
- [x] Grid height reduced by caption_px: `grid_h = h.saturating_sub(caption_px)` (verified 2026-03-29 — compute_window_layout at chrome/mod.rs:118-176)
- [x] Grid bounds offset: `Rect::new(0.0, caption_height, ...)` (verified 2026-03-29 — grid_origin_y at chrome/mod.rs:89-91)
- [x] Store chrome widget in `App.chrome` (verified 2026-03-29)
- [x] `NullMeasurer` stub for chrome drawing (no text measurement needed) (verified 2026-03-29 — STALE: superseded by CachedTextMeasurer/UiFontMeasurer pipeline, not a defect)
- [x] `draw_chrome()` method — creates DrawList, builds DrawCtx, draws chrome (verified 2026-03-29 — STALE: evolved into unified tab bar drawing pipeline; chrome drawn as part of tab bar widget paint)
- [x] `append_ui_draw_list()` on GpuRenderer — converts DrawList to GPU instances (verified 2026-03-29 — exists at gpu/window_renderer/draw_list.rs)
- [x] Removed `#[allow(dead_code)]` from `convert_draw_list()` (now actively used) (verified 2026-03-29)

## 5C.5 App Integration — Events + Resize

- [x] `chrome.rs` — `handle_chrome_action()` dispatching WindowMinimize/Maximize/Close (verified 2026-03-29 — chrome/mod.rs:183-208, gated #[cfg(not(target_os = "macos"))])
- [x] `handle_resize()` extracted method — recomputes chrome layout, grid offset, platform rects (verified 2026-03-29 — resize.rs:103-189)
- [x] `update_chrome_hover()` — converts physical cursor position to logical, routes hover (verified 2026-03-29 — renamed to update_tab_bar_hover() at chrome/mod.rs:243)
- [x] `try_chrome_mouse()` — intercepts mouse clicks in caption area, routes to chrome buttons (verified 2026-03-29 — evolved to tab bar hit test pipeline, cursor_in_tab_bar() at line 228)
- [x] `WindowEvent::Focused` handler — sets chrome active/inactive state (verified 2026-03-29)
- [x] Added `WidgetAction::WindowMinimize`, `WindowMaximize`, `WindowClose` variants (verified 2026-03-29 — oriterm_ui/src/widgets/mod.rs:181-185)
- [x] Wired chrome mouse check before `handle_mouse_input` in `MouseInput` event (verified 2026-03-29 — evolved to tab bar hover hit routing)
- [x] Wired `update_chrome_hover` in `CursorMoved` event (verified 2026-03-29 — update_tab_bar_hover is the CursorMoved handler)

## 5C.6 Platform Polish

- [x] Windows: `enable_snap()` wired with border_width and caption_height (verified 2026-03-29 — install_chrome passes border_width and caption_height to platform trait)
- [x] Windows: `set_client_rects()` updated on resize (verified 2026-03-29 — refresh_platform_rects in resize.rs:196-209)
- [x] Windows-specific code properly gated with `#[cfg(target_os = "windows")]` (verified 2026-03-29 — NativeChromeOps trait with WindowsNativeOps/MacosNativeOps/LinuxNativeOps)
- [x] Scale factor handling: logical pixels for layout, physical pixels for platform APIs (verified 2026-03-29 — grid_origin_y multiplies by scale and rounds)
- [x] Active/inactive: `Focused`/`Unfocused` winit events → chrome color change (verified 2026-03-29 — set_active on both WindowChromeWidget and TabBarWidget)

## 5C.7 Tests + Verification

- [x] ChromeLayout tests: restored/maximized caption height, fullscreen hidden, three controls, button positions, interactive rects, title rect, narrow window (verified 2026-03-29 — 11 layout tests all pass)
- [x] WindowControlButton tests: kind, not focusable, press state on mouse down (verified 2026-03-29 — 3 button tests all pass)
- [x] WindowChromeWidget tests: caption height, fullscreen invisible, maximized height, interactive rects count, resize updates layout, set title, active/inactive (verified 2026-03-29 — container tests removed; main window chrome now routes through TabBarWidget. Tab bar tests cover equivalent functionality: interactive_rects_count, positions, controls, hover enter/leave)
- [x] `./clippy-all.sh` — no warnings (Windows cross-compile + host) (verified 2026-03-29)
- [x] `./test-all.sh` — all 670 tests pass (23 new chrome tests) (verified 2026-03-29 — actual workspace count now 1,104 tests; 28 chrome-related tests)
- [x] `./build-all.sh` — cross-compilation succeeds (verified 2026-03-29)

---

## Files Created

| File | Purpose | Lines |
|------|---------|-------|
| `oriterm_ui/src/widgets/window_chrome/constants.rs` | Layout constants | ~38 |
| `oriterm_ui/src/widgets/window_chrome/layout.rs` | ChromeLayout computation | ~120 |
| `oriterm_ui/src/widgets/window_chrome/controls.rs` | WindowControlButton widget | ~260 |
| `oriterm_ui/src/widgets/window_chrome/mod.rs` | WindowChromeWidget container | ~360 |
| `oriterm_ui/src/widgets/window_chrome/tests.rs` | Unit tests | ~218 |
| `oriterm/src/app/chrome.rs` | App-level chrome action dispatch | ~50 |

## Known Issues

- [x] **BUG:** Dialog close button hold-drag — clicking and holding the close button on a dialog allows dragging the window; the dialog then closes on mouse-up. The close button correctly returns `HTCLIENT` via `WM_NCHITTEST` (interactive rect), but the dialog's app-level event handling may be interpreting the press+drag as a caption drag simultaneously. Minor UX issue. Discovered during chrome plan verification (2026-03-10). **Fixed:** Added interactive rect check in `route_dialog_click()` — mirrors the platform hit test logic. Click on a control button no longer initiates drag (2026-03-10).

---

## Files Modified

| File | Change |
|------|--------|
| `oriterm_ui/src/widgets/mod.rs` | Added `pub mod window_chrome;` + 3 WidgetAction variants |
| `oriterm/src/app/mod.rs` | Added `mod chrome;`, `chrome` field, hover/mouse/resize/focus handlers |
| `oriterm/src/app/init/mod.rs` | Chrome widget creation, `enable_snap()`, grid offset |
| `oriterm/src/app/redraw.rs` | `NullMeasurer`, `draw_chrome()`, chrome drawing in render pipeline |
| `oriterm/src/gpu/renderer/mod.rs` | `append_ui_draw_list()` method |
| `oriterm/src/gpu/draw_list_convert/mod.rs` | Removed `#[allow(dead_code)]` |

## Verification Notes (2026-03-29)

### Test Coverage Gaps
- [x] No tests for `ChromeMode::Dialog` layout. Added 3 Dialog-mode tests: `layout_dialog_mode_single_close_button`, `layout_dialog_close_at_right_edge`, `layout_dialog_title_wider_than_full`. Done 2026-04-01.
- [x] No test for control button click cycle (mouse-up action emission). Added `click_cycle_emits_clicked_action` — verifies mouse-down produces no action, mouse-up emits `Clicked`. Done 2026-04-01.
- [x] Fullscreen hidden test (`layout_fullscreen_hidden`) — fixed: replaced vacuous `.all()` on empty Vec with `assert!(interactive_rects.is_empty())`. Done 2026-03-31.
- [x] No test for `WindowChromeWidget` draw output. Added `chrome_paint_produces_scene_output` — verifies paint produces caption background rects via WidgetTestHarness. Done 2026-04-01.

### Plan/Reality Divergences (Non-Defects)
- `SYMBOL_STROKE_WIDTH` and `CLOSE_HOVER_COLOR` constants listed in 5C.1 do not exist. Symbols are now icon-atlas-based; close hover color is theme-derived.
- `NullMeasurer` listed in 5C.4 no longer exists. Superseded by `CachedTextMeasurer`/`UiFontMeasurer`.
- `draw_chrome()` method listed in 5C.4 no longer exists as a standalone method. Drawing is integrated into the tab bar widget paint pipeline.
- WindowChromeWidget container tests listed in 5C.7 were removed. The main window chrome is now unified into the tab bar. Dialog chrome uses `WindowChromeWidget` but has no dedicated unit tests.
- 23 chrome tests claimed in 5C.7; actual count is 28 (evolved with additional layout engine tests).
- 670 total tests claimed; actual workspace count is 1,104.
