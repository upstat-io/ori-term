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
- Terminal IO thread (per-pane thread owning `Term` exclusively — VTE parsing, reflow, snapshot production)
- Snapshot double buffer (lock-free snapshot transfer from IO thread to main thread)
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

### `crates/oriterm_test_support` (workspace test helpers)

Shared test utilities used by crate-level `cargo test -p <crate>` runs across the workspace. Lives at `crates/oriterm_test_support/` as a real workspace member (`Cargo.toml` `members` list) rather than a dev-dependency of a single crate so that every crate can re-use the same helpers.

**Owns:**
- Headless fixture builders (grid, cell, palette, selection, snapshot mocks)
- PTY mock for `oriterm_mux` tests (no real child process required)
- Reference golden-image loader for GPU visual-regression tests
- Shared teseq / tack / vttest fixture helpers

**Depends on:** whatever crate the fixture targets (dev-dep only — used under `[dev-dependencies]` in consumer crates, never pulled in at runtime). Must never be in a consumer crate's `[dependencies]`.

**Must NOT contain:**
- Production logic — if the helper is useful at runtime, it belongs in the target crate, not here
- Platform-specific FFI — use the target crate's existing abstraction

### `crates/vte` (vendored VTE parser)

Vendored fork of the upstream `vte` crate, patched for oriterm-specific performance and protocol handling. Treat as an external dependency — **do not add oriterm-specific types here**. If a change is genuinely needed, open an issue upstream first and vendor the patch with a clear reason in the crate's README.

### `crates/portable-pty` (vendored PTY abstraction)

Vendored fork of `portable-pty`. Same discipline as `crates/vte` — treat as external, upstream fixes first, minimal local patches. Consumed by `oriterm_mux`.

### `crates/wgpu-hal` (vendored wgpu hardware abstraction)

Vendored slice of `wgpu-hal` for a specific wgpu patch this project needs. Same discipline as the other vendored crates. Consumed by `oriterm` (via `wgpu`).

## Allowed Dependency Direction

```
oriterm_ipc              (standalone — no oriterm_* deps)
oriterm_core             (standalone — no oriterm_* deps)
oriterm_ui               → oriterm_core
oriterm_mux              → oriterm_core, oriterm_ipc
oriterm                  → oriterm_core, oriterm_ui, oriterm_mux

crates/oriterm_test_support → used as [dev-dependencies] only
crates/vte                   → consumed by oriterm_core (vendored)
crates/portable-pty          → consumed by oriterm_mux (vendored)
crates/wgpu-hal              → consumed by oriterm (vendored)
```

## Litmus Test

> **Can this code be tested in a `#[test]` without a GPU, display server, or terminal?**
> If yes → it belongs in `oriterm_ui`. If no → it belongs in `oriterm`.

## Code Review Checklist

When reviewing PRs that add code to `oriterm/src/app/`:
- Does this struct/function need GPU, platform, or terminal state? If not, it belongs in `oriterm_ui`.
- Does this duplicate a type already in `oriterm_ui`? If so, use the existing one.
- Could this be tested headlessly? If yes, move it to `oriterm_ui`.
