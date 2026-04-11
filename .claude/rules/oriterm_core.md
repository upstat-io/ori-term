---
paths:
  - "oriterm_core/src/**"
  - "oriterm_core/tests/**"
---

# oriterm_core — Terminal Emulation Library

The canonical home for the terminal emulation library: grid, VTE handler, cell representation, color palette, selection, search, terminfo conformance. Standalone crate — no workspace dependencies (see `.claude/rules/crate-boundaries.md` for ownership and allowed dependency direction).

## Non-Negotiable Rules

These rules come from real bugs observed across the reference terminal emulators (tmux, alacritty, wezterm, ghostty, ratatui, ptyxis, termenv). Every one is load-bearing.

### Color Detection Priority

Every color capability probe MUST follow this exact precedence — every reference terminal agrees on this order:

```
NO_COLOR set (any value)          → disabled (highest priority, overrides everything)
CLICOLOR_FORCE != "0"             → force color even if not TTY
CLICOLOR == "0"                   → disabled
COLORTERM=truecolor|24bit         → TrueColor
COLORTERM/TERM contains 256color  → ANSI256
TERM set + not "dumb"             → ANSI (16 color)
TERM=dumb or not a TTY            → None
```

Colors downgrade gracefully: TrueColor → nearest ANSI256 → nearest ANSI → stripped. Never crash on a missing capability; always degrade.

### Width = Unicode, not `len()`

Never use `str.len()` or `chars().count()` for display width calculations. Always use the `unicode-width` crate.

- CJK characters = width 2
- Combining marks = width 0
- ZWJ sequences = combined width of the base glyph cluster
- Strip ANSI before measuring
- Wrap and truncate by display width, not bytes
- The ellipsis character is `…` (U+2026, width 1), not three ASCII dots `...`

A width bug at the cell-level is a reflow bug at the grid-level — test both.

### Buffer Output

Never write to the screen character-by-character. Buffer the full frame, flush once.

- Honor Mode 2026 (synchronized output) — queue all output between `BSU`/`ESU` bracketing
- Double-buffer and diff: only write changed cells to the damage-tracked region
- Flush atomically — partial flushes cause visible tearing

This discipline is what prevents flicker and double-rendered cursors.

### RAII Cleanup

Every piece of terminal state that CAN be modified MUST be restored:

- Raw mode entry → restore via `Drop` guard
- Alternate screen: enter it → must leave it
- Mouse tracking: enable → disable on exit
- Panic hook: restore terminal state BEFORE printing the panic message
- SIGINT / SIGTERM: restore on signal
- Any `unsafe` FFI boundary (termios, conhost) must pair set/restore in the same `Drop` impl

If there is ANY exit path (normal, panic, signal, process-death) where the terminal isn't restored, that is a bug.

### Resize

- Linux/macOS: listen on SIGWINCH, re-query via `TIOCGWINSZ`
- Windows: `GetConsoleScreenBufferInfo` on ConPTY resize events
- Never cache stale terminal size across resize events
- Fallback on query failure: 80×24
- All layout is relative to current cell dimensions — never hardcode absolute pixel sizes in `oriterm_core`

### Piped Output

When `!stdout().is_terminal()` (output is piped or redirected):
- No colors (unless `CLICOLOR_FORCE` is set)
- No cursor manipulation
- No raw mode
- Plain text only

Check the actual output fd, not stdin.

### Dumb Terminals

`TERM=dumb` or unset → no escape sequences, no cursor movement, no colors. Degrade gracefully; never crash on a missing capability.

## Architectural Invariants

- **Grid uses absolute row indexing**: `scrollback[0..N]` + visible `rows[0..lines]`. Use `grid.absolute_row(abs_row)` to map an absolute index into a `Row` (scrollback then visible). Use `grid.visible_row(line)` to map a viewport line accounting for `display_offset`.
- **Viewport-to-absolute mapping**: `scrollback.len() - display_offset + viewport_line`
- **Scrollback is bounded**: `max_scrollback` enforced via row recycling through `Row::reset()`. No unbounded growth vectors allowed.
- **Row recycling**: when the scrollback buffer evicts a row, it is recycled via `Row::reset()`, not freshly allocated. Preserving this discipline is why the hot render path allocates zero per cell.

## Testing

- **Teseq scenarios** (`cargo test -p oriterm_core --test teseq`) — 176 tests across 10 protocol families. Requires the `reseq` binary (Linux: `sudo apt install teseq`; tests skip gracefully on macOS/Windows).
- **Tack PTY + direct-VTE xcheck** (`cargo test -p oriterm_core --test tack`) — 27 PTY scenarios + 51 direct-VTE cap cross-check. Cap xcheck runs on all platforms unconditionally; PTY scenarios skip on Windows.
- **Vttest menu structural markers** (`cargo test -p oriterm_core --test vttest`) — visual conformance for DA / DSR responses.
- **Allocation regression** (`oriterm_core/tests/alloc_regression.rs`) — enforces zero allocations in the hot render path.
- **RSS regression** (`oriterm_core/tests/rss_regression.rs`) — enforces stable RSS under sustained output.
- **Update insta snapshots** via `INSTA_UPDATE=1 cargo test -p oriterm_core --test teseq`.

When adding a new escape sequence handler or terminfo capability, the matching conformance test MUST land in the same commit as the handler. No handler merges without a test.

## Performance Invariants (hot render path)

- **Zero allocations in `renderable_content_into()`** — the reusable buffer is owned by `RenderableContent`, not allocated per call. Adding a `Vec::new()` or `Box::new()` per cell IS the regression.
- **Zero allocations in snapshot double-buffer flip** — `SnapshotDoubleBuffer::flip_swap()` uses `std::mem::swap()`, not clone.
- **Buffer shrink discipline**: grow-only `Vec` buffers (`RenderableContent` fields) apply `maybe_shrink()` post-render: `if capacity > 4 * len && capacity > 4096 → shrink_to(len * 2)`. No shrinking during `draw_frame()` (pure computation, no side effects).

## Forbidden

- No GUI types (no `Widget`, no `wgpu::*`, no `winit::*`) — those live in `oriterm_ui` / `oriterm`
- No PTY / process management — lives in `oriterm_mux`
- No font rasterization — lives in `oriterm`
- No session model (tabs / windows / layouts) — lives in `oriterm`
- No IPC transport — lives in `oriterm_ipc`
- No `println!` debugging — use `log` macros
- No `unwrap()` in library code — return `Result` or provide a default
- No `unsafe` in library code — `unsafe_code = "deny"` at the workspace level enforces this
