---
paths:
  - "**/src/**"
  - "**/Cargo.toml"
---

# Crate Boundary Rules

## Ownership

### `oriterm_core` (terminal emulation library)

**Owns:**
- Grid data structure (rows, columns, cursor, scrollback, reflow)
- Cell representation (`Cell`, `CellFlags`, hyperlinks)
- VTE handler (`term_handler.rs` — escape sequence processing)
- Color palette (`Palette`, ANSI/256/TrueColor mapping)
- Selection model (rectangular, linear, semantic)
- Search (plain text + regex)
- Terminal index types (`Line`, `Column`, `Cursor`)

**Must NOT contain:**
- UI framework types (widgets, layout, interaction, hit testing)
- GPU types (wgpu, shaders, atlas)
- PTY/process management (belongs in `oriterm_mux`)
- Window or platform types (winit, platform-specific code)
- Mux types (`PaneId`, `DomainId`, `ClientId`)

### `oriterm_ui` (UI framework)

**Owns:**
- Widget trait and all widget implementations
- WindowRoot (per-window composition unit)
- InteractionManager, FocusManager, OverlayManager
- Layout engine, hit testing, event propagation
- Controllers (hover, click, drag, focus, key activation)
- Animation engine (VisualStateAnimator, RenderScheduler, CursorBlink)
- Compositor (LayerTree, LayerAnimator)
- Scene caching, invalidation tracking
- Pure interaction utilities (resize geometry, cursor hiding, mark mode motion — NOT drag state machines, which stay in `oriterm` per Section 08.3)
- Action types and dispatch infrastructure
- Theme types (UiTheme, color tokens)
- Test harness (WidgetTestHarness wrapping WindowRoot)
- Pipeline orchestration (layout → prepaint → paint → dispatch)

**Depends on:** `oriterm_core` (for `Color` type reuse and terminal-related geometry). Also depends on `winit` (for `WindowConfig` and `create_window()` — window creation config, NOT lifecycle management). No other `oriterm_*` workspace crate dependencies.

**Must NOT contain:**
- GPU types (`wgpu::Device`, `wgpu::Surface`, shader pipelines)
- Window lifecycle management (event handling, per-window state storage, `TermWindow`). Note: `oriterm_ui` provides `window::create_window()` (returns `Arc<Window>`) and `WindowConfig` for config-driven window creation, but must not manage window lifecycle (event dispatch, `HashMap<WindowId, WindowContext>` storage).
- Terminal types (Grid, Cell, PTY, VTE, Selection beyond basic geometry)
- Mux types (`PaneId`, `MuxBackend`, domain management)
- IPC types (`oriterm_ipc` transport)
- Font rasterization (swash, skrifa, glyph atlas)
- Configuration (`Config` struct, TOML parsing, file watching)

### `oriterm_mux` (pane server)

**Owns:**
- Pane lifecycle (create, resize, close)
- PTY I/O (read/write, event pump)
- PaneRegistry (flat pane storage)
- MuxBackend trait (embedded + daemon)
- Daemon server (IPC protocol via `oriterm_ipc`)
- Wire protocol (PDU codec)
- ID types: `PaneId`, `DomainId`, `ClientId`

**Must NOT contain:**
- UI framework types (widgets, layout, interaction)
- GPU types (wgpu, shaders, rendering)
- Session model (tabs, windows, layouts — that is `oriterm`'s concern)
- Window or platform types (winit)

### `oriterm_ipc` (IPC abstraction)

**Owns:**
- Platform-specific IPC transport (Unix domain sockets, Windows named pipes)
- Connection lifecycle (listen, accept, connect)
- Mio integration for async I/O

**Must NOT contain:**
- Protocol semantics (PDU types, serialization — that is `oriterm_mux/protocol`)
- Any dependency on `oriterm_core`, `oriterm_ui`, or `oriterm`

### `oriterm` (application shell)

**Owns:**
- winit event loop and window lifecycle
- GPU initialization and rendering (wgpu, shader pipelines)
- Window ↔ WindowRoot mapping (`HashMap<WindowId, WindowContext>`)
- Terminal-specific interactions (selection, mouse reporting, PTY encoding)
- Session model (tabs, split trees, floating panes, navigation)
- Configuration loading and hot-reload
- Clipboard integration
- Mux integration (pane CRUD, event pump)
- Platform chrome (title bar, resize handles)
- Font pipeline (rasterization, atlas, shaping cache)

**Must NOT contain:**
- Widget definitions (use `oriterm_ui::widgets`)
- Pure interaction logic (use `oriterm_ui::interaction`)
- Framework state management (use `WindowRoot`)
- Pipeline orchestration (use `WindowRoot` methods)
- Duplicate type definitions of anything in `oriterm_ui`

### `crates/vte` (vendored VTE parser)

Vendored fork of the `vte` crate. Treat as external dependency. Do not add oriterm-specific types here.

## Allowed Dependency Direction

```
oriterm_ipc  (standalone — no oriterm_* deps)
oriterm_core (standalone — no oriterm_* deps)
oriterm_ui   → oriterm_core
oriterm_mux  → oriterm_core, oriterm_ipc
oriterm      → oriterm_core, oriterm_ui, oriterm_mux
```

## Litmus Test

> **Can this code be tested in a `#[test]` without a GPU, display server, or terminal?**
> If yes → it belongs in `oriterm_ui`. If no → it belongs in `oriterm`.

## Code Review Checklist

When reviewing PRs that add code to `oriterm/src/app/`:
- Does this struct/function need GPU, platform, or terminal state? If not, it belongs in `oriterm_ui`.
- Does this duplicate a type already in `oriterm_ui`? If so, use the existing one.
- Could this be tested headlessly? If yes, move it to `oriterm_ui`.
