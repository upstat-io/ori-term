---
section: 12
title: Resize & Reflow
status: complete
reviewed: true
last_verified: "2026-03-29"
tier: 3
goal: Dynamic grid resize with PTY notification and text reflow on column change
sections:
  - id: "12.1"
    title: Window-to-Grid Resize
    status: complete
  - id: "12.2"
    title: PTY Resize Notification
    status: complete
  - id: "12.3"
    title: Grid Row Resize
    status: complete
  - id: "12.4"
    title: Text Reflow
    status: complete
  - id: "12.5"
    title: Alternate Screen Resize
    status: complete
  - id: "12.6"
    title: Section Completion
    status: complete
---

# Section 12: Resize & Reflow

**Status:** Complete
**Goal:** When the window resizes, the terminal grid resizes to match and the PTY is notified of the new dimensions. Text reflows intelligently on column changes, preserving wrapped line continuity and cursor position.

**Crate:** `oriterm_core` (Grid::resize, reflow), `oriterm` (window resize handler, PTY notification)
**Reference:** `_old/src/grid/reflow.rs`, `_old/plans/terminal-core/section-04-resize.md`, Alacritty `grid/resize.rs`, Ghostty `src/terminal/PageList.zig` (resize within page structure), Ghostty `src/terminal/Screen.zig` (cell-by-cell reflow)

**Prerequisite:** Section 01 (Grid with rows, scrollback, dirty tracking), Section 03 (PTY handle for resize notification)

**Inspired by:**
- Alacritty's grid reflow (`grid/resize.rs`) with wide-char handling
- Ghostty's unified cell-by-cell rewriting approach (used in old prototype)

---

## 12.1 Window-to-Grid Resize

Calculate new grid dimensions from window pixel size and cell metrics. Dispatch resize to grid and PTY.

**File:** `oriterm/src/app/chrome.rs` (`App::handle_resize`)

- [x] On `WindowEvent::Resized(new_physical_size)`: (verified 2026-03-29)
  - [x] Calculate available area: subtract tab bar height, padding (top, bottom, left, right) (verified 2026-03-29)
  - [x] New cols = `available_width / cell_width` (integer division) (verified 2026-03-29)
  - [x] New rows = `available_height / cell_height` (integer division) (verified 2026-03-29)
  - [x] Zero dimension guard: `if new_cols == 0 || new_rows == 0 { return; }` (`.max(1)`) (verified 2026-03-29)
  - [x] Only resize if dimensions actually changed (compare against stored grid dims) (verified 2026-03-29)
- [x] Store current grid dimensions for size comparison (verified 2026-03-29)
- [x] Call `grid.resize(new_cols, new_rows, reflow=true)` for primary grid (verified 2026-03-29)
- [x] Call `grid.resize(new_cols, new_rows, reflow=false)` for alternate grid (no reflow in alt screen) (verified 2026-03-29)
- [x] Notify PTY of new dimensions (see 12.2) (verified 2026-03-29)
- [x] Reconfigure GPU surface if pixel dimensions changed (verified 2026-03-29)
- [x] Mark all rows dirty for full redraw after resize (verified 2026-03-29)
- [x] **Resize increments** (cell-boundary snapping): (verified 2026-03-29)
  - [x] When `config.window.resize_increments` is true, call `window.set_resize_increments(Some(PhysicalSize::new(cell_width, cell_height)))` (verified 2026-03-29)
  - [x] Update increments on font change or DPI change (cell dimensions change) (verified 2026-03-29)
  - [x] Snaps window resize to exact cell boundaries — no partial-cell padding at edges (verified 2026-03-29)
  - [x] **Ref:** Alacritty `display/mod.rs` `set_resize_increments`, winit `Window::set_resize_increments(Option<Size>)`

**Ref:** Alacritty `event.rs` zero-dimension guard, resize pipeline

---

## 12.2 PTY Resize Notification

Tell the child process about the new terminal size so it can redraw.

**File:** `oriterm/src/tab/mod.rs` (Tab::resize), `oriterm/src/pty/event_loop/mod.rs` (resize_pty)

- [x] After grid resize, notify PTY of new dimensions: (verified 2026-03-29)
  - [x] Windows (ConPTY): `portable-pty` `MasterPty::resize(PtySize)` or `resize_pty()` on handle (verified 2026-03-29)
  - [x] Unix: `ioctl(fd, TIOCSWINSZ, &winsize)` (handled internally by portable-pty) (verified 2026-03-29)
- [x] `PtySize { rows: u16, cols: u16, pixel_width: u16, pixel_height: u16 }` (verified 2026-03-29)
  - [x] Include both character dimensions (cols, rows) and pixel dimensions (verified 2026-03-29)
- [x] Never send 0x0 resize — crashes ConPTY on Windows (verified 2026-03-29)
  - [x] Guard: `if rows == 0 || cols == 0 { return; }` (Term::resize + App `.max(1)`) (verified 2026-03-29)
- [x] Store PTY master handle in Tab for resize access (verified 2026-03-29)
- [x] Resize both primary and alternate grids, but only send one PTY notification (the shell sees unified dimensions) (verified 2026-03-29)

**Ref:** Alacritty `tty/mod.rs` OnResize, WezTerm PtySize

---

## 12.3 Grid Row Resize

Handle vertical dimension changes: adding/removing rows with scrollback interaction.

**File:** `oriterm_core/src/grid/resize/mod.rs`

**Reference:** `_old/src/grid/reflow.rs` (resize_rows)

- [x] `Grid::resize_rows(&mut self, new_lines: usize)` (verified 2026-03-29)
- [x] **Row decrease (shrinking)**: (verified 2026-03-29)
  - [x] Prefer trimming trailing blank rows first (don't push empty rows to scrollback) (verified 2026-03-29)
  - [x] `count_trailing_blank_rows(max: usize) -> usize` — count blank rows from bottom, below cursor (verified 2026-03-29)
  - [x] After trimming blanks: push remaining excess top rows to scrollback (verified 2026-03-29)
  - [x] Adjust cursor row: `cursor.row = cursor.row.saturating_sub(rows_pushed_to_scrollback)` (verified 2026-03-29)
  - [x] Ensure at least `new_lines` rows in viewport (pad with empty if needed) (verified 2026-03-29)
- [x] **Row increase (growing)**: (verified 2026-03-29)
  - [x] If cursor at bottom of screen: pull lines from scrollback history (restore hidden content) (verified 2026-03-29)
    - [x] `from_scrollback = delta.min(scrollback.len())` (verified 2026-03-29)
    - [x] Pop from scrollback back, prepend to viewport (verified 2026-03-29)
    - [x] Adjust cursor row: `cursor.row += from_scrollback` (verified 2026-03-29)
  - [x] If cursor in middle: append empty rows at bottom (don't disturb scrollback) (verified 2026-03-29)
- [x] Resize dirty tracker to match new line count (verified 2026-03-29)
- [x] **Tests**: (verified 2026-03-29)
  - [x] Shrink: trailing blank rows trimmed first (verified 2026-03-29)
  - [x] Shrink: non-blank rows pushed to scrollback, cursor adjusted (verified 2026-03-29)
  - [x] Grow: empty rows added when cursor in middle (verified 2026-03-29)
  - [x] Grow: scrollback pulled when cursor at bottom (verified 2026-03-29)
  - [x] Zero-size guard: resize(0, 0) is no-op (verified 2026-03-29)

---

## 12.4 Text Reflow

When columns change, reflow wrapped lines to fit the new width. Uses Ghostty-style cell-by-cell rewriting.

**File:** `oriterm_core/src/grid/resize/mod.rs`

**Reference:** `_old/src/grid/reflow.rs` (reflow_cols), Alacritty `grid/resize.rs`, Ghostty `src/terminal/PageList.zig`

- [x] `Grid::resize(&mut self, new_cols: usize, new_lines: usize, reflow: bool)` (verified 2026-03-29)
  - [x] Guards: early return if 0x0 or dimensions unchanged (verified 2026-03-29)
  - [x] With reflow + column change: (verified 2026-03-29)
    - [x] Growing cols: reflow first (unwrap), then adjust rows (verified 2026-03-29)
    - [x] Shrinking cols: adjust rows first, then reflow (wrap) (verified 2026-03-29)
    - [x] Order matters: growing unwraps before row adjustment to avoid losing content; shrinking wraps after row adjustment to handle overflow correctly (verified 2026-03-29)
  - [x] Without reflow: resize rows, then resize each row's cell count (verified 2026-03-29)
  - [x] Reset scroll region after resize: `scroll_top = 0, scroll_bottom = new_lines - 1` (verified 2026-03-29)
  - [x] Clamp cursor: `cursor.row.min(lines - 1)`, `cursor.col.min(cols - 1)` (verified 2026-03-29)
  - [x] Clamp display_offset to scrollback length (verified 2026-03-29)
  - [x] Resize dirty tracker (verified 2026-03-29)
- [x] `Grid::reflow_cols(&mut self, new_cols: usize)` — unified cell-by-cell rewriting (verified 2026-03-29)
  - [x] Collect all rows: scrollback + visible, in order (verified 2026-03-29)
  - [x] Track cursor position in the unified list (absolute index + column) (verified 2026-03-29)
  - [x] Create output rows at new column width (verified 2026-03-29)
  - [x] For each source row: (verified 2026-03-29)
    - [x] Determine if wrapped: `WRAPLINE` flag set at old column boundary (verified 2026-03-29)
    - [x] Content length: wrapped rows use full width, non-wrapped rows trim trailing blanks (verified 2026-03-29)
    - [x] For each source cell: (verified 2026-03-29)
      - [x] Skip `WIDE_CHAR_SPACER` cells (regenerated at new positions) (verified 2026-03-29)
      - [x] Skip `LEADING_WIDE_CHAR_SPACER` cells (regenerated at new boundaries) (verified 2026-03-29)
      - [x] If cell doesn't fit in current output row: wrap to next row (verified 2026-03-29)
        - [x] Wide char at boundary: insert `LEADING_WIDE_CHAR_SPACER` padding (verified 2026-03-29)
        - [x] Set `WRAPLINE` flag on boundary cell (verified 2026-03-29)
      - [x] Write cell to output (strip old `WRAPLINE` flag) (verified 2026-03-29)
      - [x] Write wide char spacer in next column if wide (verified 2026-03-29)
    - [x] Non-wrapped source row: finalize output row (verified 2026-03-29)
    - [x] Wrapped source row: continue filling output row (unwrapping) (verified 2026-03-29)
  - [x] Track cursor through reflow: map old (abs_row, col) to new (abs_row, col) (verified 2026-03-29)
  - [x] Split result into scrollback + visible (verified 2026-03-29)
  - [x] Update cursor position from tracked coordinates (verified 2026-03-29)
- [x] **Wide char boundary handling**: (verified 2026-03-29)
  - [x] If wide char would be split at new column boundary: (verified 2026-03-29)
    - [x] Insert `LEADING_WIDE_CHAR_SPACER` at end of current row (verified 2026-03-29)
    - [x] Move wide char to start of next row (verified 2026-03-29)
  - [x] If grid too narrow for wide chars (`new_cols < 2`): treat as narrow (verified 2026-03-29)
- [x] **Cursor reflow**: (verified 2026-03-29)
  - [x] Track which content position the cursor was on before reflow (verified 2026-03-29)
  - [x] After reflow: place cursor at same content position in new layout (verified 2026-03-29)
  - [x] If cursor was on a spacer cell: adjust to the preceding wide char (verified 2026-03-29)
  - [x] If cursor past content: clamp to end of row (verified 2026-03-29)
- [x] Ensure all output rows have correct column count (verified 2026-03-29)
- [x] Ensure at least one row exists after reflow (verified 2026-03-29)
- [x] **Tests** (`oriterm_core/src/grid/resize/tests.rs`): (verified 2026-03-29)
  - [x] Column increase: wrapped lines unwrap (WRAPLINE cleared, content merged) (verified 2026-03-29)
  - [x] Column decrease: long lines re-wrap (WRAPLINE set, content split) (verified 2026-03-29)
  - [x] Wide char at shrink boundary: `LEADING_WIDE_CHAR_SPACER` inserted, wide char moved to next row (verified 2026-03-29)
  - [x] Cursor preservation: cursor on 'X' before reflow is on 'X' after reflow (verified 2026-03-29)
  - [x] Scrollback reflow: content pushed to scrollback on shrink, pulled on grow (verified 2026-03-29)
  - [x] Empty grid: reflow produces at least one row (verified 2026-03-29)
  - [x] No-op: same column count does not modify grid (verified 2026-03-29)

---

## 12.5 Alternate Screen Resize

Resize the alternate screen buffer without reflow (full-screen apps manage their own layout).

- [x] When alternate screen is active: `grid.resize(new_cols, new_lines, reflow=false)` (verified 2026-03-29)
  - [x] Rows truncated or extended with blank cells (verified 2026-03-29)
  - [x] No WRAPLINE manipulation, no content merging/splitting (verified 2026-03-29)
  - [x] Cursor clamped to new bounds (verified 2026-03-29)
- [x] Apps like vim, htop, tmux send their own redraw commands after receiving SIGWINCH/ConPTY resize (verified 2026-03-29)
- [x] Both primary and alternate grids resized on every window resize (verified 2026-03-29)
  - [x] Primary: with reflow (verified 2026-03-29)
  - [x] Alternate: without reflow (verified 2026-03-29)

---

## 12.6 Section Completion

- [x] All 12.1-12.5 items complete (verified 2026-03-29)
- [x] `cargo test -p oriterm_core` — reflow tests pass (verified 2026-03-29 — 123/123 pass)
- [x] `cargo clippy -p oriterm_core --target x86_64-pc-windows-gnu` — no warnings (verified 2026-03-29)
- [x] Resizing the window resizes the grid with correct new dimensions (verified 2026-03-29)
- [x] PTY receives new dimensions on resize (no 0x0) (verified 2026-03-29)
- [x] Shell prompt redraws correctly after resize (verified 2026-03-29)
- [x] Text reflows when columns change (long lines wrap/unwrap) (verified 2026-03-29 — 94 reflow tests)
- [x] Wide characters handled at reflow boundaries (LEADING_WIDE_CHAR_SPACER) (verified 2026-03-29 — 12+ wide char tests)
- [x] Cursor position preserved through resize and reflow (verified 2026-03-29 — 6+ cursor tracking tests)
- [x] No crash on zero-dimension resize (verified 2026-03-29)
- [x] No crash on rapid resize sequences (verified 2026-03-29)
- [x] Alternate screen resizes correctly (no reflow, cursor clamped) (verified 2026-03-29)
- [x] vim/htop/tmux redraw correctly after resize (verified 2026-03-29)

**Exit Criteria:** Resizing the window produces correct terminal behavior -- text reflows, the shell adapts, full-screen apps redraw properly, and cursor position is preserved through resize operations.
