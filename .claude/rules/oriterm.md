---
paths:
  - "oriterm/src/**"
  - "oriterm/tests/**"
---

# oriterm — Application Shell

The canonical home for the application shell: winit event loop, window lifecycle (`TermWindow`, `HashMap<WindowId, WindowContext>`), GPU initialization and rendering (wgpu, shader pipelines, atlas, compositor), session model (tabs, split trees, floating panes, navigation), font pipeline (rasterization, atlas, shaping cache), configuration loading and hot reload, clipboard integration, mux integration (pane CRUD, event pump), and platform chrome (title bar, resize handles). Consumes `oriterm_core`, `oriterm_ui`, and `oriterm_mux`. See `.claude/rules/crate-boundaries.md` for the allowed dependency direction.

This is the "anything that needs a GPU, a display server, a terminal, or platform APIs" crate. Everything here is platform-bound or tied to a real window.

## Performance Invariants

These invariants are enforced by regression tests in `oriterm_core/tests/alloc_regression.rs` and `oriterm/src/app/event_loop_helpers/tests.rs`. **Do not introduce code that violates them.**

### Zero idle CPU beyond cursor blink

When idle, the event loop sleeps via `ControlFlow::Wait`. The **only** wakeup source is the cursor blink timer (~1.89 Hz). No polling, no spurious `WaitUntil` lingering from prior activity. Verified by `compute_control_flow()` pure function tests in `event_loop_helpers/tests.rs`. Any change that touches event-loop scheduling MUST re-run these tests.

### Zero allocations in the hot render path

- The IO thread calls `renderable_content_into()` into a reusable buffer
- `SnapshotDoubleBuffer::flip_swap()` exchanges the back buffer with the front buffer via `std::mem::swap()`
- The main thread calls `swap_front()` + `swap_renderable_content()` — all pointer swaps, zero allocation
- All `Vec` buffers are reused via `.clear()` + capacity retention
- `HashSet` scratch buffers live on `RenderableContent`
- **No `Vec::new()` or `Box::new()` per cell or per frame.** Ever.

### Stable RSS under sustained output

- Scrollback is bounded by `max_scrollback` with row recycling via `Row::reset()`
- Image caches evict via frame-based aging
- GPU textures drop via `wgpu::Texture::Drop`
- No unbounded growth vector exists for normal terminal operation
- Verified by `rss_stability_under_sustained_output` in `oriterm_core/tests/rss_regression.rs`

### Buffer shrink discipline

Grow-only `Vec` buffers (instance writers, shaping scratch, notification buffer, `RenderableContent` fields) apply `maybe_shrink()` post-render:
```
if capacity > 4 * len && capacity > 4096 → shrink_to(len * 2)
```
**No shrinking during `draw_frame()`** (pure computation, no side effects). Shrink happens after the frame completes.

## GPU Render Path Testing

The production render path uses **content caching** (`render_cached`): content is rendered to an offscreen cache texture, then copied to the surface via `copy_texture_to_texture`. Test-only `render_frame()` skips this entirely — it renders directly to an offscreen target. **Bugs in the cached path are invisible to `render_frame()`.**

`render_frame_cached()` in `oriterm/src/gpu/window_renderer/render.rs` is the test-only method that exercises the production cached render path. It accepts a target size that may differ from the prepared viewport — exactly the mismatch that occurs when the surface is reconfigured during interactive resize.

**Writing cached render path tests** (in `oriterm/src/gpu/visual_regression/resize_stress.rs`):
```rust
// Prepare at one size, render to a smaller target (simulates resize race).
renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, true);
renderer.render_frame_cached(&gpu, &pipelines, target_w, target_h, true);
```

**Key rule**: when testing GPU rendering under resize or any condition where viewport and surface dimensions may diverge, always use `render_frame_cached()`. Use `gpu.create_copy_dst_target()` when manually creating destination targets (adds `COPY_DST` usage to simulate a surface texture).

**Test-only APIs**:
- `WindowRenderer::render_frame_cached()` — cached render to controllable target size
- `GpuState::create_copy_dst_target()` — render target with `COPY_DST` for copy destinations
- `RenderTarget::texture()` — backing texture access for copy operations

## Session Model

GUI-owned session state lives in `oriterm/src/session/`. This is **where tabs, windows, split trees, floating layers, and directional navigation live** — NOT in `oriterm_mux`. The mux is flat / pane-only; the session layer builds the tree on top.

- `oriterm/src/session/split_tree/` — SplitTree pane tiling
- `oriterm/src/session/floating/` — FloatingLayer pane overlay
- `oriterm/src/session/compute/` — pixel-space layout computation (`compute_window_layout()`: 3-element flex — tab bar 36px + grid fill + status bar 22px, inset by 2px window border; grid rect from `WindowLayout::grid_rect`)
- `oriterm/src/session/nav/` — directional pane navigation

Exports: `TabId`, `WindowId`, `SessionRegistry`, `Tab`, `Window`, `SplitTree`, `FloatingPane`, `Rect`, layout compute, nav.

## Borrow Checker Patterns

When mutating `tab.selection` while reading `tab.grid()`, extract data from grid FIRST into local variables, then mutate selection. You can't hold `&mut tab.selection` and call `tab.grid()` simultaneously. Use `tab.selection.as_ref().map(|s| s.field)` to read before mutating.

## Cross-Platform Discipline

- Every `#[cfg(target_os = ...)]` must have counterparts for Linux / macOS / Windows
- Windows cross-compile from WSL: `cargo build --target x86_64-pc-windows-gnu --release`
- ConPTY on Windows, POSIX PTY on Linux/macOS
- Win32 input mode for Ctrl+C delivery through ConPTY (see BUG-11-1)
- Clipboard integration: `arboard` on Unix, `clipboard-win` wrappers on Windows

## Testing

- **Architecture tests** (`cargo test -p oriterm --test architecture`) — enforce crate-boundary rules
- **GPU visual regression** (`cargo test -p oriterm --test main_window` and siblings under `oriterm/src/gpu/visual_regression/`) — cached render path golden tests, resize stress, tack + vttest conformance mirrors
- **Allocation regression** (`oriterm_core/tests/alloc_regression.rs`) — re-run after any hot-path change
- **Event loop control flow** (`oriterm/src/app/event_loop_helpers/tests.rs`) — re-run after any event-loop change

## Key Paths

- `oriterm/src/app/` — `App` struct, winit event loop, GPU init, input dispatch — thin shell delegating to `WindowRoot`
- `oriterm/src/session/` — GUI session model
- `oriterm/src/gpu/` — GPU rendering (wgpu, `draw_frame`, atlas, compositor, extract, image_render, instance_writer, bind_groups, builtin_glyphs, frame_input, icon_rasterizer)
- `oriterm/src/font/` — font pipeline (swash rasterizer, skrifa-backed hinting, UI font registry, shaping cache)
- `oriterm/src/config/` — config loading, TOML parsing, hot reload
- `oriterm/src/clipboard/` — clipboard integration

## Forbidden

- No terminal emulation logic (Grid, VTE handler, reflow, selection) — those live in `oriterm_core`
- No widget definitions — use `oriterm_ui::widgets`
- No pure interaction logic — use `oriterm_ui::interaction`
- No framework state management — use `WindowRoot`
- No pipeline orchestration — use `WindowRoot` methods
- No duplicate type definitions of anything in `oriterm_ui`
- No pane lifecycle / PTY I/O — those live in `oriterm_mux`
- No `println!` debugging — use `log` macros
- No `unwrap()` in non-test code
- No allocations in `draw_frame()` — period.
