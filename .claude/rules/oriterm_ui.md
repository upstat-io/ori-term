---
paths:
  - "oriterm_ui/src/**"
  - "oriterm_ui/tests/**"
---

# oriterm_ui — UI Framework

The canonical home for the widget framework: widget trait + implementations, WindowRoot, InteractionManager, FocusManager, OverlayManager, pipeline orchestration, animation, compositor, scene caching, controllers, action dispatch, theme types, the WidgetTestHarness. Depends on `oriterm_core` only. **Must be testable in a `#[test]` without a GPU, display server, or terminal** — see the litmus test in `.claude/rules/crate-boundaries.md`.

## UI Framework — Zero Exceptions Rule

**Every single UI control** — buttons, toggles, sliders, dropdowns, text inputs, window chrome buttons, tab bar tabs, close buttons, menu items, scroll thumbs, dialog headers — goes through the unified controller + animator + propagation pipeline. **No special cases, no manual `hovered: bool` fields, no one-off `handle_mouse()` implementations.** One system, one path, no exceptions.

- **WindowRoot** is the per-window composition unit. It owns the widget tree, InteractionManager, FocusManager, OverlayManager, compositor, and pipeline. Both `WidgetTestHarness` and production windows wrap WindowRoot. No framework state may be owned outside WindowRoot.
- **InteractionManager** is the single source of truth for all interaction state (hot, active, focused, disabled). Widgets must not maintain shadow copies of this state.
- **VisualStateAnimator** drives all state-dependent visual transitions (hover colors, focus rings, pressed states). No widget implements its own color animation state machine.
- **EventControllers** (`HoverController`, `ClickController`, `DragController`, `KeyController`, `FocusController`) handle all input. No widget implements `handle_mouse()` / `handle_key()` directly — every input routes through a controller.
- **The propagation pipeline** routes events through the widget tree. No container manually calls `child.handle_mouse()`. The pipeline determines which widget gets each event based on hit testing, z-order, and focus.

**If you find a widget doing its own hover/press/focus tracking outside this system, that is a bug. Fix it — don't add a workaround.**

## Widget Test Harness

`WidgetTestHarness` (`oriterm_ui/src/testing/`) enables headless widget testing without GPU, display server, or platform dependencies. It wraps `WindowRoot` and provides input simulation, state inspection, and paint capture.

**Running harness tests**: `cargo test -p oriterm_ui` runs all widget and harness tests.

**Writing new harness tests** (in any `tests.rs` file within `oriterm_ui`):
```rust
let mut h = WidgetTestHarness::new(ButtonWidget::new("OK"));
h.mouse_move_to(center);      // Input simulation
assert!(h.is_hot(button_id)); // State inspection
h.click(center);              // Click helper (move + down + up)
let scene = h.render();       // Paint capture (returns Scene, no GPU)
```

Key APIs: `mouse_move()`, `mouse_down()`, `mouse_up()`, `click()`, `key_press()`, `tab()`, `shift_tab()`, `scroll()`, `drag()`, `type_text()`, `advance_time()`, `resize()`, `render()`, `is_hot()`, `is_active()`, `is_focused()`, `interaction_state()`, `get_widget()`, `all_widget_ids()`, `widgets_with_sense()`, `push_popup()`, `has_overlays()`, `dismiss_overlays()`.

**Rule**: every widget with input senses (hover / click / drag / keyboard / focus) MUST have at least one harness test covering each sense it owns. A widget that owns a sense and has no harness test for that sense is untested — fix it, don't ship it.

## Action & Keymap System

Actions are typed enums declared by widgets. Keybindings are data (not code) that map keystrokes to actions. Dispatch routes through the context-scoped focus path.

**Declaring an action** (in `oriterm_ui/src/action/keymap_action/mod.rs`):
```rust
actions!(widget, [Activate, Dismiss, NavigateDown, NavigateUp, Confirm]);
```

**Adding a keybinding** (in `oriterm_ui/src/action/keymap/defaults.rs`):
```rust
KeyBinding::new(Keystroke::new(Key::Enter), None, Box::new(widget::Activate))
```

**Context scoping**: widgets return `key_context() -> Option<&'static str>` (e.g. `"Button"`, `"Dropdown"`). Bindings match only when the focused widget's context stack includes the binding's context.

**Widget integration**: implement `handle_keymap_action(&mut self, action: &dyn KeymapAction) -> Option<WidgetAction>` to receive dispatched actions.

## Interaction Utilities

Pure interaction logic — resize geometry, cursor hiding, mark mode motion — lives in `oriterm_ui/src/interaction/`. These are pure functions that can be tested headlessly. **Drag state machines stay in `oriterm`** (per Section 08.3 of the framework plan) because they couple to winit's drag contract.

## Forbidden

- No GPU types (`wgpu::Device`, `wgpu::Surface`, shader pipelines, `wgpu::Texture`) — those live in `oriterm`
- No window lifecycle management (event handling, per-window state storage, `TermWindow`, `HashMap<WindowId, WindowContext>`) — those live in `oriterm`. NOTE: `oriterm_ui` provides `window::create_window()` (returns `Arc<Window>`) and `WindowConfig` for config-driven window creation, but must not manage window lifecycle.
- No terminal types (Grid, Cell, PTY, VTE, Selection beyond basic geometry) — those live in `oriterm_core`
- No mux types (`PaneId`, `MuxBackend`, domain management) — those live in `oriterm_mux`
- No IPC types (`oriterm_ipc` transport)
- No font rasterization (swash, skrifa, glyph atlas) — lives in `oriterm`
- No configuration (`Config` struct, TOML parsing, file watching) — lives in `oriterm`
- No manual `hovered: bool` / `pressed: bool` shadow state on widgets — use `InteractionManager`
- No direct `handle_mouse()` implementations on widgets — use `EventControllers` via the propagation pipeline
- No `println!` debugging — use `log` macros
- No `unwrap()` in library code
