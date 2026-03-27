---
section: "15"
title: "Mouse Cursor Icons"
status: not-started
reviewed: false
third_party_review:
  status: none
  updated: null
goal: "Dialog widgets change the OS mouse cursor to match the mockup — pointer for clickable elements, not-allowed for disabled, crosshair for color pickers"
depends_on: ["12", "13"]
sections:
  - id: "15.1"
    title: "Cursor Request Plumbing"
    status: not-started
  - id: "15.2"
    title: "Widget Cursor Declarations"
    status: not-started
  - id: "15.3"
    title: "Dialog Cursor Application"
    status: not-started
  - id: "15.4"
    title: "Tests + Build Gate"
    status: not-started
---

# Section 15: Mouse Cursor Icons

## Problem

The mockup declares `cursor: pointer` on every clickable element — nav items, buttons, toggles,
selects, scheme cards, color cells, sliders, keybind rows, and range inputs. Disabled primary
buttons use `cursor: not-allowed`. The color picker area uses `cursor: crosshair`. The app
currently shows the default arrow cursor everywhere in the settings dialog.

The main window already has cursor support for URL hover (`CursorIcon::Pointer` in
`oriterm/src/app/cursor_hover.rs`) and floating pane drag (`CursorIcon::Grab`/`Grabbing` in
`oriterm/src/app/floating_drag.rs`). Both use winit's `window.set_cursor(CursorIcon::*)`.

The dialog window lacks any cursor management — it never calls `set_cursor`.

### Mockup Cursor Declarations

From `mockups/settings-brutal.html`:

| Selector | Cursor | Widget |
|----------|--------|--------|
| `.nav-item` | pointer | SidebarNavWidget items |
| `.sidebar-update` | pointer | Sidebar update link |
| `.toggle` | pointer | ToggleWidget |
| `select` | pointer | DropdownWidget |
| `.scheme-card` | pointer | SchemeCardWidget |
| `.btn-sm` | pointer | Small buttons |
| `.color-cell` | pointer | Color cells |
| `.keybind-row` | pointer | KeybindWidget |
| `.cursor-option` | pointer | CursorPickerWidget options |
| `.btn` | pointer | All buttons (Reset, Cancel, Save) |
| `.btn-primary:disabled` | not-allowed | Disabled Save button |
| `input[type="range"]::-webkit-slider-thumb` | pointer | SliderWidget thumb |
| `.color-area-wrap` | crosshair | Color picker area (future) |
| `.hue-strip-wrap` | pointer | Hue strip (future) |
| `.alpha-strip-wrap` | pointer | Alpha strip (future) |
| `.btn-test` | pointer | Test buttons |

### Existing Infrastructure

- `winit::window::CursorIcon` — the enum: `Default`, `Pointer`, `Grab`, `Grabbing`,
  `NotAllowed`, `Crosshair`, etc.
- `cursor_hover.rs` — main window cursor management. Returns `CursorHoverResult { cursor_icon }`
  from hit-test, then calls `window.set_cursor(result.cursor_icon)`.
- The dialog event handler (`dialog_context/event_handling/mod.rs`) already does hit-testing
  for hover state. The cursor icon should be resolved from the same hit path.

---

## 15.1 Cursor Request Plumbing

### Goal

Add a `cursor_icon()` method to the `Widget` trait so widgets can declare what cursor they want
when hovered. The hit-test path in the dialog event handler reads the leaf widget's cursor and
calls `window.set_cursor()`. All cursors are **native OS cursors** via winit — no custom cursor
rendering. winit's `CursorIcon` maps to native cursors on all three platforms (Windows, macOS,
Linux/Wayland/X11) automatically — no platform-specific code needed.

### Approach

1. Add `fn cursor_icon(&self) -> CursorIcon` to the `Widget` trait with default `CursorIcon::Default`
2. Widgets override to return `CursorIcon::Pointer` when clickable, etc.
3. The dialog cursor-move handler reads the cursor from the hit-test leaf widget

**Alternative considered:** Store cursor on `Sense` or `InteractionState`. Rejected because cursor
depends on widget state (disabled → not-allowed), which only the widget knows.

### Checklist

- [ ] Add `fn cursor_icon(&self) -> winit::window::CursorIcon` to `Widget` trait, default `Default`
- [ ] Export `winit::window::CursorIcon` as `oriterm_ui::CursorIcon` (or re-export from a shared location so widgets don't depend on winit directly — check crate boundaries)
- [ ] If winit is not a dependency of `oriterm_ui`, use a local enum `UiCursorIcon` and convert to winit at the app layer
- [ ] Verify `Widget` trait stays under method count limits (check existing method count)

---

## 15.2 Widget Cursor Declarations

### Goal

Every clickable widget returns `Pointer` (or `NotAllowed` when disabled) from `cursor_icon()`.

### Checklist

- [ ] `ButtonWidget` — `Pointer` when enabled, `Default` when disabled
- [ ] `IdOverrideButton` — delegate to inner button
- [ ] `ToggleWidget` — `Pointer` when enabled
- [ ] `DropdownWidget` — `Pointer` when enabled
- [ ] `SliderWidget` — `Pointer` (entire track, not just thumb)
- [ ] `CheckboxWidget` — `Pointer` when enabled
- [ ] `SidebarNavWidget` items — `Pointer` (may need per-item cursor, not per-widget)
- [ ] `SchemeCardWidget` — `Pointer`
- [ ] `KeybindWidget` — `Pointer`
- [ ] `CursorPickerWidget` — `Pointer` per option
- [ ] `NumberInputWidget` — `Text` (text cursor for text input fields)
- [ ] `TextInputWidget` — `Text` (text cursor for text input fields)
- [ ] All other widgets — leave as `Default`
- [ ] Add tests: `cursor_icon()` returns correct value for enabled/disabled states

---

## 15.3 Dialog Cursor Application

### Goal

The dialog's cursor-move handler reads the cursor from the hot widget and applies it to the
window. When no interactive widget is under the cursor, revert to `Default`.

### Files

- `oriterm/src/app/dialog_context/event_handling/mod.rs` — `handle_dialog_cursor_move()`

### Checklist

- [ ] After hit-testing in `handle_dialog_cursor_move`, resolve the leaf widget's `cursor_icon()`
- [ ] Call `ctx.window.window().set_cursor(cursor)` with the resolved cursor
- [ ] When cursor leaves all interactive widgets (empty hot path), set `CursorIcon::Default`
- [ ] When cursor leaves the dialog window entirely (`CursorLeft` event), set `CursorIcon::Default`
- [ ] Verify: hover over button → pointer, move to empty area → arrow, hover disabled → not-allowed

---

## 15.4 Tests + Build Gate

### Checklist

- [ ] Unit tests: each widget's `cursor_icon()` returns correct value
- [ ] Unit test: disabled button returns `Default` (not `NotAllowed` — matching CSS `not-allowed` only for `.btn-primary:disabled`)
- [ ] `./clippy-all.sh` passes
- [ ] `./test-all.sh` passes
- [ ] `./build-all.sh` passes
- [ ] Manual verification: hover cursor changes correctly on all interactive elements in settings dialog
