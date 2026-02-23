---
section: 5C
title: Window Chrome (Title Bar + Controls)
status: complete
tier: 2
goal: Render a visible title bar with minimize/maximize/close controls, wire platform integration (Aero Snap, drag), offset the terminal grid below the caption bar
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

- [x] `constants.rs` — layout constants (CAPTION_HEIGHT, CAPTION_HEIGHT_MAXIMIZED, CONTROL_BUTTON_WIDTH, RESIZE_BORDER_WIDTH, CLOSE_HOVER_COLOR, SYMBOL_SIZE, SYMBOL_STROKE_WIDTH)
- [x] `layout.rs` — `ControlKind` enum (Minimize, MaximizeRestore, Close)
- [x] `ControlRect` struct (kind + rect)
- [x] `ChromeLayout` struct (caption_height, title_rect, controls, interactive_rects, visible)
- [x] `ChromeLayout::compute()` — pure geometry from window_width, is_maximized, is_fullscreen
- [x] `ChromeLayout::hidden()` — fullscreen returns zero-height invisible layout

## 5C.2 WindowControlButton Widget

- [x] `controls.rs` — `WindowControlButton` implementing `Widget`
- [x] Three kinds: Minimize, MaximizeRestore, Close
- [x] Geometric symbol drawing (minimize dash, maximize square, restore overlapping squares, close X)
- [x] Hover animation via `AnimatedValue<f32>` (100ms, EaseOut)
- [x] Close button: red hover bg (#C42B1C), white foreground on hover
- [x] Emits `WidgetAction::WindowMinimize/WindowMaximize/WindowClose`
- [x] `is_pressed()` accessor for parent container routing

## 5C.3 WindowChromeWidget Container

- [x] `mod.rs` — `WindowChromeWidget` composing title + 3 control buttons
- [x] `with_theme()` constructor derives colors from `UiTheme`
- [x] Active/inactive caption background colors (darken helper)
- [x] Caption background rect + title text drawing
- [x] Mouse event routing to control buttons
- [x] `update_hover()` — hover enter/leave to control buttons
- [x] `caption_height()`, `interactive_rects()`, `is_visible()` accessors
- [x] `set_title()`, `set_active()`, `set_maximized()`, `set_fullscreen()`, `set_window_width()` state updates

## 5C.4 App Integration — Init + Redraw

- [x] Create `WindowChromeWidget` in `init/mod.rs` after renderer
- [x] Wire `enable_snap()` on Windows with scaled caption height
- [x] Wire `set_client_rects()` on Windows with scaled interactive rects
- [x] Grid height reduced by caption_px: `grid_h = h.saturating_sub(caption_px)`
- [x] Grid bounds offset: `Rect::new(0.0, caption_height, ...)`
- [x] Store chrome widget in `App.chrome`
- [x] `NullMeasurer` stub for chrome drawing (no text measurement needed)
- [x] `draw_chrome()` method — creates DrawList, builds DrawCtx, draws chrome
- [x] `append_ui_draw_list()` on GpuRenderer — converts DrawList to GPU instances
- [x] Removed `#[allow(dead_code)]` from `convert_draw_list()` (now actively used)

## 5C.5 App Integration — Events + Resize

- [x] `chrome.rs` — `handle_chrome_action()` dispatching WindowMinimize/Maximize/Close
- [x] `handle_resize()` extracted method — recomputes chrome layout, grid offset, platform rects
- [x] `update_chrome_hover()` — converts physical cursor position to logical, routes hover
- [x] `try_chrome_mouse()` — intercepts mouse clicks in caption area, routes to chrome buttons
- [x] `WindowEvent::Focused` handler — sets chrome active/inactive state
- [x] Added `WidgetAction::WindowMinimize`, `WindowMaximize`, `WindowClose` variants
- [x] Wired chrome mouse check before `handle_mouse_input` in `MouseInput` event
- [x] Wired `update_chrome_hover` in `CursorMoved` event

## 5C.6 Platform Polish

- [x] Windows: `enable_snap()` wired with border_width and caption_height
- [x] Windows: `set_client_rects()` updated on resize
- [x] Windows-specific code properly gated with `#[cfg(target_os = "windows")]`
- [x] Scale factor handling: logical pixels for layout, physical pixels for platform APIs
- [x] Active/inactive: `Focused`/`Unfocused` winit events → chrome color change

## 5C.7 Tests + Verification

- [x] ChromeLayout tests: restored/maximized caption height, fullscreen hidden, three controls, button positions, interactive rects, title rect, narrow window
- [x] WindowControlButton tests: kind, not focusable, press state on mouse down
- [x] WindowChromeWidget tests: caption height, fullscreen invisible, maximized height, interactive rects count, resize updates layout, set title, active/inactive
- [x] `./clippy-all.sh` — no warnings (Windows cross-compile + host)
- [x] `./test-all.sh` — all 670 tests pass (23 new chrome tests)
- [x] `./build-all.sh` — cross-compilation succeeds

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

## Files Modified

| File | Change |
|------|--------|
| `oriterm_ui/src/widgets/mod.rs` | Added `pub mod window_chrome;` + 3 WidgetAction variants |
| `oriterm/src/app/mod.rs` | Added `mod chrome;`, `chrome` field, hover/mouse/resize/focus handlers |
| `oriterm/src/app/init/mod.rs` | Chrome widget creation, `enable_snap()`, grid offset |
| `oriterm/src/app/redraw.rs` | `NullMeasurer`, `draw_chrome()`, chrome drawing in render pipeline |
| `oriterm/src/gpu/renderer/mod.rs` | `append_ui_draw_list()` method |
| `oriterm/src/gpu/draw_list_convert/mod.rs` | Removed `#[allow(dead_code)]` |
