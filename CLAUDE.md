# ori_term

GPU-accelerated terminal emulator in Rust (same category as Alacritty, WezTerm, Ghostty). Opens a native frameless window, renders a terminal grid via wgpu, runs shell processes through ConPTY/PTY.

**Cross-platform: macOS, Windows, and Linux.** All code must compile and run correctly on all three platforms. Never write platform-specific code without corresponding implementations for the other two. Every `#[cfg(target_os = "...")]` block must have counterparts for all supported targets — no platform left behind. If a feature cannot be implemented on a platform, it must degrade gracefully with a compile-time `cfg` gate, not a runtime panic. CI builds and tests on all three. Local dev cross-compiles from WSL targeting `x86_64-pc-windows-gnu`.

**Broken Window Policy**: Fix EVERY issue you encounter — no exceptions. Never say "this is pre-existing", "this is unrelated", or "outside the scope". If you see it, you own it. Leaving broken code because "it was already broken" is explicitly forbidden.

**Do it properly, not just simply. Correct architecture over quick hacks; no shortcuts or "good enough" solutions.**

**NO WORKAROUNDS. NO HACKS. NO SHORTCUTS.**
- **Proper fixes only** — If a fix feels hacky, it IS hacky. Find the right solution.
- **When unsure, STOP and ASK** — Do not guess. Do not assume. Pause and ask the user for guidance.
- **Fact-check everything** — Verify behavior against reference implementations. Test your assumptions. Read the code you're modifying.
- **Consult reference repos** — Check `~/projects/reference_repos/console_repos/` for established patterns and idioms.
- **No "temporary" fixes** — There is no such thing. Today's temporary fix is tomorrow's permanent tech debt.
- **If you can't do it right, say so** — Communicate blockers rather than shipping bad code.

---

## Coding Standards

**Extracted from**: Alacritty, WezTerm, Ghostty, Ptyxis, Ratatui, Crossterm, Bubbletea, Lipgloss, Termenv — the patterns every serious terminal project agrees on.

**Error Handling**: No `unwrap()` in library code — return `Result` or provide a default. No `panic!` on user-recoverable errors. Use `std::io::Result<T>` for I/O operations. Custom `Error` enum with `From` impls for domain-specific errors. Error chains via `.context()` or `source()`.

**Unsafe**: `unsafe_code = "deny"` in Cargo.toml. Zero unsafe in library code (Ratatui forbids it entirely). Only justified platform FFI in clearly marked modules.

**Linting**: Clippy warnings are errors (`all = deny`). Pedantic + nursery enabled as warnings. No `#[allow(clippy)]` without written justification. `enum_glob_use = deny`, `if_not_else = deny`.

**Formatting**: `imports_granularity = "Module"`. Group imports: std, external, crate. Comments wrapped at 100 chars. Format code in doc comments.

**Module Organization**: Separate terminal logic from GUI (Alacritty pattern: pure terminal lib vs. rendering binary). One primary type per module file. Re-export key types at parent `mod.rs`. Two-file pattern: `style.rs` + `style/` directory for sub-modules. Platform-specific code behind `#[cfg()]` in dedicated files. **Source files (excluding `tests.rs`) must not exceed 500 lines** — when writing new code, proactively split into submodules before hitting the limit rather than writing a large file and splitting later.

**Public API**: Keep surface small — expose primitives, not internals. `#[must_use]` on builder methods. `impl Into<T>` and `impl AsRef<str>` for ergonomic APIs. Document every public item with `///`. First line: summary. Second: blank. Then details.

**Functions**: < 50 lines (target < 30). No 20+ arm match blocks — extract helpers at 3+ similar arms. No boolean flag parameters (split function or use enum). > 3 params → config/options struct.

**Memory**: Newtypes for IDs (`TabId(u64)`, not bare `u64`). `Arc` only when shared ownership is required. No `Arc` cloning in hot paths. Intern/cache repeated strings. `#[cold]` on error-path factory functions.

**Performance**: O(n^2) → O(n) or O(n log n). Hash lookups not linear scans. No allocation in hot loops. Iterators over indexing. Buffer output, flush atomically — never write char-by-char. Damage tracking to minimize GPU work.

**Testing**: Buffer/TestBackend approach for rendering tests (from Ratatui). Test Unicode width with CJK, emoji, combining marks, ZWJ sequences. Test every env var combination for color detection. Platform matrix in CI. Visual regression tests where applicable. Verify behavior not implementation.

**Style**: No dead/commented code, no banners. `//!`/`///` doc comments. Full sentences with periods in comments. No `println!` debugging — use `log` macros.

---

## Terminal Emulator Rules

Non-negotiable. Every one comes from a real bug observed across the reference repos.

**Color Detection Priority** (every project agrees on this order):
```
NO_COLOR set (any value)          → disabled (highest priority)
CLICOLOR_FORCE != "0"             → force color even if not TTY
CLICOLOR == "0"                   → disabled
COLORTERM=truecolor|24bit         → TrueColor
COLORTERM/TERM contains 256color  → ANSI256
TERM set + not "dumb"             → ANSI (16 color)
TERM=dumb or not a TTY            → None
```
Colors downgrade gracefully: TrueColor → nearest ANSI256 → nearest ANSI → stripped.

**Width = Unicode, not `len()`**: Never use `str.len()` or `chars().count()` for display width. Use `unicode-width` crate. CJK = width 2. Combining marks = width 0. Strip ANSI before measuring. Wrap and truncate by display width, not bytes. Ellipsis is `…` (U+2026, width 1), not `...`.

**Buffer Output**: Never write char-by-char. Buffer the full frame, flush once. Synchronized output (Mode 2026). Double-buffer and diff (only write changed cells). This prevents flicker.

**RAII Cleanup**: Raw mode via Drop guards. Panic hook restores terminal state before printing. SIGINT/SIGTERM restore. Alternate screen: enter it → must leave it. No leaked terminal state on any exit path.

**Resize**: SIGWINCH on Unix. Re-query size after signal. Never cache stale terminal size. Fallback: 80x24. All layout relative to current terminal width — never hardcode.

**Piped Output**: `!stdout().is_terminal()` → no colors (unless CLICOLOR_FORCE), no cursor manipulation, no raw mode, plain text only. Check the actual output fd, not stdin.

**Dumb Terminals**: `TERM=dumb` or no TERM → no escape sequences, no cursor movement, no colors. Degrade gracefully, never crash.

---

## Commands

**Primary**: `./fmt-all.sh`, `./clippy-all.sh`, `./build-all.sh`, `./test-all.sh`
**Build**: `cargo build --target x86_64-pc-windows-gnu` (debug), `cargo build --target x86_64-pc-windows-gnu --release` (release)
**After EVERY change, run `./build-all.sh`, `./clippy-all.sh`, and `./test-all.sh`. No exceptions. Do not skip any of these.**

---

## Key Paths

**oriterm (GUI binary — thin shell):** `oriterm/src/app/` — App struct, winit event loop, GPU init, input dispatch — thin shell delegating to WindowRoot | `oriterm/src/session/` — GUI session model (tabs, windows, layouts) | `oriterm/src/session/split_tree/` — SplitTree pane tiling | `oriterm/src/session/floating/` — FloatingLayer pane overlay | `oriterm/src/session/compute/` — Layout computation (pixel-space) | `oriterm/src/session/nav/` — Directional pane navigation

**oriterm_ui (UI framework):** `oriterm_ui/src/widgets/` — Widget trait + all widget implementations | `oriterm_ui/src/window_root/` — WindowRoot (per-window UI composition unit) | `oriterm_ui/src/interaction/` — Pure interaction utilities (resize geometry, cursor hiding, mark mode motion) | `oriterm_ui/src/pipeline/` — Pipeline orchestration (layout → prepaint → paint → dispatch) | `oriterm_ui/src/testing/` — WidgetTestHarness (headless testing)

**oriterm_mux (pane server):** `oriterm_mux/src/in_process/` — InProcessMux (pane CRUD, event pump) | `oriterm_mux/src/registry/` — PaneRegistry (flat pane storage) | `oriterm_mux/src/pane/` — Pane (IO thread handle, lock-free atomics) | `oriterm_mux/src/pane/io_thread/` — PaneIoThread (owns Term exclusively, VTE parsing, snapshot production, command processing) | `oriterm_mux/src/pane/io_thread/snapshot/` — SnapshotDoubleBuffer (lock-free snapshot transfer IO→main) | `oriterm_mux/src/backend/` — MuxBackend trait (embedded + daemon) | `oriterm_mux/src/server/` — Daemon server (IPC protocol) | `oriterm_mux/src/protocol/` — Wire protocol (PDU codec)

**oriterm_core (terminal emulation):** `oriterm_core/src/grid/` — Grid (rows, cursor, scrollback, reflow) | `oriterm_core/src/term_handler.rs` — VTE Handler impl | `oriterm_core/src/cell.rs` — Rich Cell + CellFlags | `oriterm_core/src/palette.rs` — Color palette | `oriterm_core/src/selection.rs` — Selection model | `oriterm_core/src/search.rs` — Search (plain + regex)

**oriterm_gpu (rendering):** `oriterm_gpu/src/renderer.rs` — GPU rendering (wgpu, draw_frame) | `oriterm_gpu/src/atlas.rs` — Glyph atlas | `oriterm_gpu/src/pipeline.rs` — WGSL shader pipelines

## Crate Boundaries

**`oriterm_core`** — Terminal emulation library (grid, VTE, selection, search). Standalone, no workspace deps.
**`oriterm_ui`** — UI framework (widgets, WindowRoot, interaction, pipeline, animation, testing). Depends on `oriterm_core` only.
**`oriterm_mux`** — Pane server (PTY I/O, pane lifecycle, mux backend). Each pane has a dedicated Terminal IO thread that owns `Term` exclusively — VTE parsing, reflow, and snapshot production happen on the IO thread. The main thread reads lock-free snapshots via `SnapshotDoubleBuffer`. Depends on `oriterm_core` + `oriterm_ipc`.
**`oriterm_ipc`** — Platform IPC transport (Unix sockets, Windows named pipes). Standalone, no workspace deps.
**`oriterm`** — Application shell (winit event loop, GPU, font pipeline, session model). Consumes all other crates.

**Allowed dependency direction:**
```
oriterm_ipc  (standalone)
oriterm_core (standalone)
oriterm_ui   → oriterm_core
oriterm_mux  → oriterm_core, oriterm_ipc
oriterm      → oriterm_core, oriterm_ui, oriterm_mux
```

**Litmus test:** Can this code be tested in a `#[test]` without a GPU, display server, or terminal? If yes → `oriterm_ui`. If no → `oriterm`. See `.claude/rules/crate-boundaries.md` for full ownership rules.

## Reference Repos (`~/projects/reference_repos/console_repos/`)

- **tmux** — C, the canonical terminal multiplexer. Grid/screen/tty separation, `input.c` (83k-line VT parser), `grid.c` (cell storage + extended cells for wide/RGB), `screen-write.c` (damage-tracked screen updates), `window-copy.c` (selection/search/vi-mode). Gold standard for PTY management, reflow, and session persistence
- **alacritty** — 4-crate workspace, OpenGL, `vte` parser, strict clippy (`deny(clippy::all)`), `rustfmt.toml` with module imports
- **wezterm** — 69-crate monorepo, `anyhow`+`thiserror` errors, Lua config, `portable-pty`, multiplexer architecture
- **ghostty** — Zig, Metal+OpenGL+WebGL, SIMD, comptime C ABI, AGENTS.md, Valgrind integration
- **ratatui** — 9-crate workspace, `unsafe_code = "forbid"`, Buffer-based widget tests, TestBackend, pedantic clippy
- **crossterm** — Single crate, Command trait pattern (`queue!`/`execute!` macros), `io::Result<T>` everywhere
- **bubbletea** — Go Elm Architecture (Model/Update/View), frame-based rendering (60/120 FPS), goroutine channels
- **lipgloss** — CSS-like fluent styling, AdaptiveColor/CompleteColor, lazy `sync.Once` renderer
- **ptyxis** — C/GTK4, GNOME's default terminal (Fedora/RHEL/Ubuntu). libvte consumer with GPU-accelerated rendering, `ptyxis-agent` out-of-process PTY helper for Flatpak sandboxing, `.palette` file format for color schemes with light/dark auto-adaptation, profile system (per-profile container/palette/shell), tab monitor for process tracking (`sudo`/SSH indicators), container-first architecture (Podman/Toolbox/Distrobox discovery), encrypted scrollback, terminal inspector for OSC/mouse debugging
- **termenv** — Color profile detection (NO_COLOR/CLICOLOR), `Environ` interface for testing, profile-aware downgrade

## Performance Invariants

These invariants are enforced by regression tests in `oriterm_core/tests/alloc_regression.rs` and `oriterm/src/app/event_loop_helpers/tests.rs`. Do not introduce code that violates them.

- **Zero idle CPU beyond cursor blink.** When idle, the event loop sleeps via `ControlFlow::Wait`. The only wakeup source is the cursor blink timer (~1.89 Hz). No polling, no spurious `WaitUntil` lingering from prior activity. Verified by `compute_control_flow()` pure function tests.
- **Zero allocations in hot render path.** The IO thread calls `renderable_content_into()` into a reusable buffer, then `SnapshotDoubleBuffer::flip_swap()` exchanges it with the front buffer via `std::mem::swap()`. The main thread calls `swap_front()` + `swap_renderable_content()` — all pointer swaps, zero allocation. All `Vec` buffers are reused via `.clear()` + capacity retention. `HashSet` scratch buffers live on `RenderableContent`. No `Vec::new()` or `Box::new()` per cell or per frame.
- **Stable RSS under sustained output.** Scrollback is bounded by `max_scrollback` with row recycling via `Row::reset()`. Image caches evict via frame-based aging. GPU textures drop via `wgpu::Texture::Drop`. No unbounded growth vector exists for normal terminal operation.
- **Buffer shrink discipline.** Grow-only `Vec` buffers (instance writers, shaping scratch, notification buffer, `RenderableContent` fields) apply `maybe_shrink()` post-render: `if capacity > 4 * len && capacity > 4096 → shrink_to(len * 2)`. No shrinking during `draw_frame()` (pure computation, no side effects).

## Plans

Implementation plans live in `plans/`. Each plan is a directory with an `index.md`, `00-overview.md`, and numbered section files (`section-01-*.md`, `section-02-*.md`, etc.).

When the user says **"continue plan X"** or **"resume plan X"** or **"pick up plan X"**:
1. Look in `plans/` for a directory matching the name (fuzzy match — "threading" matches `threaded-pty`, "font" matches `font-rendering`, etc.).
2. Read `00-overview.md` for the full context and mandate.
3. Read each `section-*.md` to find the first section with `status: not-started` or `status: in-progress`.
4. Resume work from that section.
5. **After completing each section**, update the plan files: set YAML status to `complete`, check checkboxes, update `index.md`, and record any deviations.

Plans are the source of truth for multi-session work. Keep them in sync with reality.

**Review Gate:** Every roadmap section has `reviewed: true/false` in its frontmatter. Sections with `reviewed: false` have NOT been vetted by `/review-plan` and must not be implemented without review. `/continue-roadmap` enforces this gate automatically — it will stop and warn before working on an unreviewed section.

---

## UI Framework — Zero Exceptions Rule

Every single UI control — buttons, toggles, sliders, dropdowns, text inputs, window chrome buttons, tab bar tabs, close buttons, menu items, scroll thumbs, dialog headers — goes through the unified controller + animator + propagation pipeline. No special cases, no manual `hovered: bool` fields, no one-off `handle_mouse()` implementations. One system, one path, no exceptions.

- **WindowRoot** is the per-window composition unit — owns widget tree, InteractionManager, FocusManager, OverlayManager, compositor, and pipeline. Both WidgetTestHarness and production windows wrap WindowRoot. No framework state should be owned outside WindowRoot.
- **InteractionManager** is the single source of truth for all interaction state (hot, active, focused, disabled).
- **VisualStateAnimator** drives all state-dependent visual transitions (hover colors, focus rings, pressed states).
- **EventControllers** (HoverController, ClickController, DragController, etc.) handle all input — no widget implements raw event methods directly.
- **The propagation pipeline** routes events through the widget tree — no container manually calls `child.handle_mouse()`.

If you find a widget doing its own hover/press/focus tracking outside this system, that is a bug. Fix it.

---

## Widget Test Harness

`WidgetTestHarness` (`oriterm_ui/src/testing/`) enables headless widget testing without GPU, display server, or platform dependencies. It wraps `WindowRoot` and provides input simulation, state inspection, and paint capture.

**Running harness tests**: `cargo test -p oriterm_ui` runs all widget and harness tests. Architecture tests: `cargo test -p oriterm --test architecture`.

**Writing new harness tests** (in any `tests.rs` file within `oriterm_ui`):
```rust
let mut h = WidgetTestHarness::new(ButtonWidget::new("OK"));
h.mouse_move_to(center);      // Input simulation
assert!(h.is_hot(button_id)); // State inspection
h.click(center);              // Click helper (move + down + up)
let scene = h.render();       // Paint capture (returns Scene)
```

Key APIs: `mouse_move()`, `mouse_down()`, `mouse_up()`, `click()`, `key_press()`, `tab()`, `shift_tab()`, `scroll()`, `drag()`, `type_text()`, `advance_time()`, `render()`, `is_hot()`, `is_active()`, `is_focused()`, `interaction_state()`, `get_widget()`, `all_widget_ids()`, `widgets_with_sense()`, `push_popup()`, `has_overlays()`, `dismiss_overlays()`.

---

## Action & Keymap System

Actions are typed enums declared by widgets. Keybindings are data (not code) that map keystrokes to actions. Dispatch routes through context-scoped focus path.

**Declaring an action** (in `oriterm_ui/src/action/keymap_action/mod.rs`):
```rust
actions!(widget, [Activate, Dismiss, NavigateDown, NavigateUp, Confirm]);
```

**Adding a keybinding** (in `oriterm_ui/src/action/keymap/defaults.rs`):
```rust
KeyBinding::new(Keystroke::new(Key::Enter), None, Box::new(widget::Activate))
```

**Context scoping**: Widgets return `key_context() -> Option<&'static str>` (e.g., `"Button"`, `"Dropdown"`). Bindings match only when the focused widget's context stack includes the binding's context.

**Widget integration**: Implement `handle_keymap_action(&mut self, action: &dyn KeymapAction) -> Option<WidgetAction>` to receive dispatched actions.

---

## Current State

See [plans/roadmap/](plans/roadmap/) — the roadmap is the current state. 28 sections, 8 tiers. Use `/continue-roadmap` to resume work. Old prototype in `_old/` for reference.
