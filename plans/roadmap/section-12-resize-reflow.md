---
section: 12
title: Resize & Reflow
status: complete
reviewed: true
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

**Status:** In Progress
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

- [x] On `WindowEvent::Resized(new_physical_size)`:
  - [x] Calculate available area: subtract tab bar height, padding (top, bottom, left, right)
  - [x] New cols = `available_width / cell_width` (integer division)
  - [x] New rows = `available_height / cell_height` (integer division)
  - [x] Zero dimension guard: `if new_cols == 0 || new_rows == 0 { return; }` (`.max(1)`)
  - [x] Only resize if dimensions actually changed (compare against stored grid dims)
- [x] Store current grid dimensions for size comparison
- [x] Call `grid.resize(new_cols, new_rows, reflow=true)` for primary grid
- [x] Call `grid.resize(new_cols, new_rows, reflow=false)` for alternate grid (no reflow in alt screen)
- [x] Notify PTY of new dimensions (see 12.2)
- [x] Reconfigure GPU surface if pixel dimensions changed
- [x] Mark all rows dirty for full redraw after resize
- [x] **Resize increments** (cell-boundary snapping):
  - [x] When `config.window.resize_increments` is true, call `window.set_resize_increments(Some(PhysicalSize::new(cell_width, cell_height)))`
  - [x] Update increments on font change or DPI change (cell dimensions change)
  - [x] Snaps window resize to exact cell boundaries — no partial-cell padding at edges
  - [x] **Ref:** Alacritty `display/mod.rs` `set_resize_increments`, winit `Window::set_resize_increments(Option<Size>)`

**Ref:** Alacritty `event.rs` zero-dimension guard, resize pipeline

---

## 12.2 PTY Resize Notification

Tell the child process about the new terminal size so it can redraw.

**File:** `oriterm/src/tab/mod.rs` (Tab::resize), `oriterm/src/pty/event_loop/mod.rs` (resize_pty)

- [x] After grid resize, notify PTY of new dimensions:
  - [x] Windows (ConPTY): `portable-pty` `MasterPty::resize(PtySize)` or `resize_pty()` on handle
  - [x] Unix: `ioctl(fd, TIOCSWINSZ, &winsize)` (handled internally by portable-pty)
- [x] `PtySize { rows: u16, cols: u16, pixel_width: u16, pixel_height: u16 }`
  - [x] Include both character dimensions (cols, rows) and pixel dimensions
- [x] Never send 0x0 resize — crashes ConPTY on Windows
  - [x] Guard: `if rows == 0 || cols == 0 { return; }` (Term::resize + App `.max(1)`)
- [x] Store PTY master handle in Tab for resize access
- [x] Resize both primary and alternate grids, but only send one PTY notification (the shell sees unified dimensions)

**Ref:** Alacritty `tty/mod.rs` OnResize, WezTerm PtySize

---

## 12.3 Grid Row Resize

Handle vertical dimension changes: adding/removing rows with scrollback interaction.

**File:** `oriterm_core/src/grid/resize/mod.rs`

**Reference:** `_old/src/grid/reflow.rs` (resize_rows)

- [x] `Grid::resize_rows(&mut self, new_lines: usize)`
- [x] **Row decrease (shrinking)**:
  - [x] Prefer trimming trailing blank rows first (don't push empty rows to scrollback)
  - [x] `count_trailing_blank_rows(max: usize) -> usize` — count blank rows from bottom, below cursor
  - [x] After trimming blanks: push remaining excess top rows to scrollback
  - [x] Adjust cursor row: `cursor.row = cursor.row.saturating_sub(rows_pushed_to_scrollback)`
  - [x] Ensure at least `new_lines` rows in viewport (pad with empty if needed)
- [x] **Row increase (growing)**:
  - [x] If cursor at bottom of screen: pull lines from scrollback history (restore hidden content)
    - [x] `from_scrollback = delta.min(scrollback.len())`
    - [x] Pop from scrollback back, prepend to viewport
    - [x] Adjust cursor row: `cursor.row += from_scrollback`
  - [x] If cursor in middle: append empty rows at bottom (don't disturb scrollback)
- [x] Resize dirty tracker to match new line count
- [x] **Tests**:
  - [x] Shrink: trailing blank rows trimmed first
  - [x] Shrink: non-blank rows pushed to scrollback, cursor adjusted
  - [x] Grow: empty rows added when cursor in middle
  - [x] Grow: scrollback pulled when cursor at bottom
  - [x] Zero-size guard: resize(0, 0) is no-op

---

## 12.4 Text Reflow

When columns change, reflow wrapped lines to fit the new width. Uses Ghostty-style cell-by-cell rewriting.

**File:** `oriterm_core/src/grid/resize/mod.rs`

**Reference:** `_old/src/grid/reflow.rs` (reflow_cols), Alacritty `grid/resize.rs`, Ghostty `src/terminal/PageList.zig`

- [x] `Grid::resize(&mut self, new_cols: usize, new_lines: usize, reflow: bool)`
  - [x] Guards: early return if 0x0 or dimensions unchanged
  - [x] With reflow + column change:
    - [x] Growing cols: reflow first (unwrap), then adjust rows
    - [x] Shrinking cols: adjust rows first, then reflow (wrap)
    - [x] Order matters: growing unwraps before row adjustment to avoid losing content; shrinking wraps after row adjustment to handle overflow correctly
  - [x] Without reflow: resize rows, then resize each row's cell count
  - [x] Reset scroll region after resize: `scroll_top = 0, scroll_bottom = new_lines - 1`
  - [x] Clamp cursor: `cursor.row.min(lines - 1)`, `cursor.col.min(cols - 1)`
  - [x] Clamp display_offset to scrollback length
  - [x] Resize dirty tracker
- [x] `Grid::reflow_cols(&mut self, new_cols: usize)` — unified cell-by-cell rewriting
  - [x] Collect all rows: scrollback + visible, in order
  - [x] Track cursor position in the unified list (absolute index + column)
  - [x] Create output rows at new column width
  - [x] For each source row:
    - [x] Determine if wrapped: `WRAPLINE` flag set at old column boundary
    - [x] Content length: wrapped rows use full width, non-wrapped rows trim trailing blanks
    - [x] For each source cell:
      - [x] Skip `WIDE_CHAR_SPACER` cells (regenerated at new positions)
      - [x] Skip `LEADING_WIDE_CHAR_SPACER` cells (regenerated at new boundaries)
      - [x] If cell doesn't fit in current output row: wrap to next row
        - [x] Wide char at boundary: insert `LEADING_WIDE_CHAR_SPACER` padding
        - [x] Set `WRAPLINE` flag on boundary cell
      - [x] Write cell to output (strip old `WRAPLINE` flag)
      - [x] Write wide char spacer in next column if wide
    - [x] Non-wrapped source row: finalize output row
    - [x] Wrapped source row: continue filling output row (unwrapping)
  - [x] Track cursor through reflow: map old (abs_row, col) to new (abs_row, col)
  - [x] Split result into scrollback + visible
  - [x] Update cursor position from tracked coordinates
- [x] **Wide char boundary handling**:
  - [x] If wide char would be split at new column boundary:
    - [x] Insert `LEADING_WIDE_CHAR_SPACER` at end of current row
    - [x] Move wide char to start of next row
  - [x] If grid too narrow for wide chars (`new_cols < 2`): treat as narrow
- [x] **Cursor reflow**:
  - [x] Track which content position the cursor was on before reflow
  - [x] After reflow: place cursor at same content position in new layout
  - [x] If cursor was on a spacer cell: adjust to the preceding wide char
  - [x] If cursor past content: clamp to end of row
- [x] Ensure all output rows have correct column count
- [x] Ensure at least one row exists after reflow
- [x] **Tests** (`oriterm_core/src/grid/resize/tests.rs`):
  - [x] Column increase: wrapped lines unwrap (WRAPLINE cleared, content merged)
  - [x] Column decrease: long lines re-wrap (WRAPLINE set, content split)
  - [x] Wide char at shrink boundary: `LEADING_WIDE_CHAR_SPACER` inserted, wide char moved to next row
  - [x] Cursor preservation: cursor on 'X' before reflow is on 'X' after reflow
  - [x] Scrollback reflow: content pushed to scrollback on shrink, pulled on grow
  - [x] Empty grid: reflow produces at least one row
  - [x] No-op: same column count does not modify grid

---

## 12.5 Alternate Screen Resize

Resize the alternate screen buffer without reflow (full-screen apps manage their own layout).

- [x] When alternate screen is active: `grid.resize(new_cols, new_lines, reflow=false)`
  - [x] Rows truncated or extended with blank cells
  - [x] No WRAPLINE manipulation, no content merging/splitting
  - [x] Cursor clamped to new bounds
- [x] Apps like vim, htop, tmux send their own redraw commands after receiving SIGWINCH/ConPTY resize
- [x] Both primary and alternate grids resized on every window resize
  - [x] Primary: with reflow
  - [x] Alternate: without reflow

---

## 12.6 Section Completion

- [x] All 12.1-12.5 items complete
- [x] `cargo test -p oriterm_core` — reflow tests pass
- [x] `cargo clippy -p oriterm_core --target x86_64-pc-windows-gnu` — no warnings
- [x] Resizing the window resizes the grid with correct new dimensions
- [x] PTY receives new dimensions on resize (no 0x0)
- [x] Shell prompt redraws correctly after resize
- [x] Text reflows when columns change (long lines wrap/unwrap)
- [x] Wide characters handled at reflow boundaries (LEADING_WIDE_CHAR_SPACER)
- [x] Cursor position preserved through resize and reflow
- [x] No crash on zero-dimension resize
- [x] No crash on rapid resize sequences
- [x] Alternate screen resizes correctly (no reflow, cursor clamped)
- [x] vim/htop/tmux redraw correctly after resize

**Exit Criteria:** Resizing the window produces correct terminal behavior -- text reflows, the shell adapts, full-screen apps redraw properly, and cursor position is preserved through resize operations.
