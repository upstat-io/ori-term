---
section: 1
title: Cell + Grid
status: in-progress
reviewed: true
last_verified: "2026-03-29"
tier: 0
goal: Build the core data structures — Cell, Row, Grid — in oriterm_core with full test coverage
sections:
  - id: "1.1"
    title: Workspace Setup
    status: complete
  - id: "1.2"
    title: Index Newtypes
    status: complete
  - id: "1.3"
    title: Cell Types
    status: complete
  - id: "1.4"
    title: Row
    status: in-progress
  - id: "1.5"
    title: Grid Foundation
    status: in-progress
  - id: "1.6"
    title: Cursor
    status: in-progress
  - id: "1.7"
    title: Grid Editing
    status: in-progress
  - id: "1.8"
    title: Grid Navigation
    status: complete
  - id: "1.9"
    title: Grid Scrolling
    status: complete
  - id: "1.10"
    title: Scrollback Ring Buffer
    status: complete
  - id: "1.11"
    title: Dirty Tracking
    status: complete
  - id: "1.R"
    title: TPR Findings
    status: not-started
  - id: "1.12"
    title: Section Completion
    status: in-progress
third_party_review:
  status: findings
  updated: "2026-03-29"
---

# Section 01: Cell + Grid

**Status:** 📋 Planned
**Goal:** Build the foundational data structures for terminal emulation in `oriterm_core`. Every terminal operation ultimately reads or writes cells in a grid. This layer must be rock-solid before anything else is built on top.

**Crate:** `oriterm_core`
**Dependencies:** `bitflags`, `vte` (Color types only), `unicode-width`, `log`

---

## 1.1 Workspace Setup

Convert the single-crate project into a Cargo workspace with `oriterm_core` as the first library crate.

- [x] Create `oriterm_core/` directory with `Cargo.toml` and `src/lib.rs` (verified 2026-03-29)
  - [x] `Cargo.toml`: name = `oriterm_core`, edition = 2024, same lint config as root
  - [x] Dependencies: `bitflags = "2"`, `vte = { version = "0.15.0", features = ["ansi"] }`, `unicode-width = "0.2"`, `log = "0.4"`
  - [x] `src/lib.rs`: module declarations, `//!` doc comment, `#![deny(unsafe_code)]`
- [x] Convert root `Cargo.toml` to workspace (verified 2026-03-29)
  - [x] Add `[workspace]` section with `members = ["oriterm_core", "oriterm"]`
  - [x] Move binary crate to `oriterm/` directory
  - [x] `oriterm/Cargo.toml`: depends on `oriterm_core = { path = "../oriterm_core" }`
  - [x] Binary at `oriterm/src/main.rs` (move current `src/main.rs`)
- [x] Verify: `cargo build --target x86_64-pc-windows-gnu` succeeds for workspace (verified 2026-03-29)
- [x] Verify: `cargo test -p oriterm_core` runs (even if no tests yet) (verified 2026-03-29)

---

## 1.2 Index Newtypes

Type-safe indices prevent mixing up row/column/line values. These are used everywhere.

**File:** `oriterm_core/src/index.rs`

- [x] `Line(i32)` — Signed line index (negative = scrollback history)
  - [x] `impl From<i32> for Line`, `impl From<Line> for i32`
  - [x] `impl Add`, `Sub`, `AddAssign`, `SubAssign` for `Line`
  - [x] `impl Display` for `Line` — shows inner value
- [x] `Column(usize)` — Unsigned column index (0-based)
  - [x] `impl From<usize> for Column`, `impl From<Column> for usize`
  - [x] `impl Add`, `Sub`, `AddAssign`, `SubAssign` for `Column`
  - [x] `impl Display` for `Column`
- [x] `Point<L = Line>` — Generic grid coordinate
  - [x] Fields: `line: L`, `column: Column`
  - [x] `impl Point<Line>`: `fn new(line: Line, column: Column) -> Self`
  - [x] Derive: `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, `PartialOrd`, `Ord`
- [x] `Side` enum — `Left`, `Right` (which half of a cell the cursor is on, for selection)
  - [x] Derive: `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`
- [x] `Direction` enum — `Left`, `Right` (for search, movement)
  - [x] Derive: `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`
- [x] `Boundary` enum — `Grid`, `Cursor`, `Wrap` (semantic boundaries for selection)
  - [x] Derive: `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`
- [x] Re-export all types from `lib.rs`
- [x] **Tests** (`oriterm_core/src/index.rs` `#[cfg(test)]`) — 16 tests, all pass (verified 2026-03-29):
  - [x] Line arithmetic: `Line(5) + Line(3) == Line(8)`, negative lines
  - [x] Column arithmetic: `Column(5) - Column(3) == Column(2)`
  - [x] Point ordering: `Point { line: Line(0), column: Column(5) } < Point { line: Line(1), column: Column(0) }`

---

## 1.3 Cell Types

A Cell represents one character position in the terminal grid. Must be compact (target: 24 bytes) because there are `rows × cols × scrollback` of them.

**File:** `oriterm_core/src/cell.rs`

**Reference:** `_old/src/cell.rs` — carry forward the proven 24-byte layout.

- [x] `CellFlags` — `bitflags! { struct CellFlags: u16 { ... } }`
  - [x] `BOLD`, `DIM`, `ITALIC`, `UNDERLINE`, `BLINK`, `INVERSE`, `HIDDEN`, `STRIKETHROUGH`
  - [x] `WIDE_CHAR` — This cell is a wide character (width 2)
  - [x] `WIDE_CHAR_SPACER` — This cell is the trailing spacer of a wide character
  - [x] `WRAP` — Line wrapped at this cell (soft wrap)
  - [x] `CURLY_UNDERLINE`, `DOTTED_UNDERLINE`, `DASHED_UNDERLINE`, `DOUBLE_UNDERLINE`
  - [x] Tests: set/clear/query individual flags, combine flags with `|`
- [x] `CellExtra` — Heap-allocated optional data (only for cells that need it)
  - [x] Fields:
    - `underline_color: Option<vte::ansi::Color>` — colored underline (SGR 58)
    - `hyperlink: Option<Hyperlink>` — OSC 8 hyperlink
    - `zerowidth: Vec<char>` — combining marks / zero-width characters appended to this cell
  - [x] Wrapped in `Option<Box<CellExtra>>` in Cell — None for normal cells (zero overhead)
  - [x] `CellExtra::new() -> Self` — all fields None/empty
- [x] `Hyperlink` — URL hyperlink data
  - [x] Fields: `id: Option<String>`, `uri: String`
  - [x] Derive: `Debug`, `Clone`, `PartialEq`, `Eq`
- [x] `Cell` — The core cell struct
  - [x] Fields:
    - `ch: char` (4 bytes) — the character
    - `fg: vte::ansi::Color` (enum, for deferred palette resolution)
    - `bg: vte::ansi::Color`
    - `flags: CellFlags` (2 bytes)
    - `extra: Option<Box<CellExtra>>` (8 bytes pointer, None = 0)
  - [x] `Cell::default()` — space character, default fg/bg, no flags, no extra
  - [x] `Cell::reset(&mut self, template: &Cell)` — reset to template (for erase operations)
  - [x] `Cell::is_empty(&self) -> bool` — space char, default colors, no flags
  - [x] `Cell::width(&self) -> usize` — returns `unicode_width::UnicodeWidthChar::width(self.ch).unwrap_or(1)`, respecting `WIDE_CHAR` flag
  - [x] Derive: `Debug`, `Clone`, `PartialEq`
- [x] Verify `std::mem::size_of::<Cell>()` ≤ 24 bytes
  - [x] Add compile-time assert: `const _: () = assert!(std::mem::size_of::<Cell>() <= 24);`
- [x] Re-export `Cell`, `CellFlags`, `CellExtra`, `Hyperlink` from `lib.rs`
- [x] **Tests** (`oriterm_core/src/cell.rs` `#[cfg(test)]`) — 26 tests, all pass (verified 2026-03-29):
  - [x] Default cell is space with default colors
  - [x] Reset clears to template
  - [x] `is_empty` returns true for default, false after setting char
  - [x] Wide char cell has `WIDE_CHAR` flag, width returns 2
  - [x] CellExtra is None for normal cells, Some for underline color/hyperlink/zerowidth
  - [x] Appending a combining mark to a cell creates CellExtra with zerowidth vec
  - [x] Size assertion: `size_of::<Cell>() <= 24`

---

## 1.4 Row

A Row is a contiguous array of Cells representing one terminal line.

**File:** `oriterm_core/src/grid/row.rs`

- [x] `Row` struct
  - [x] Fields:
    - `inner: Vec<Cell>` — the cells
    - `occ: usize` — occupancy: index of last non-empty cell + 1 (optimization for sparse rows)
  - [x] `Row::new(cols: usize) -> Self` — creates row of `cols` default cells, `occ = 0`
  - [x] `Row::reset(&mut self, cols: usize, template: &Cell)` — reset all cells to template, resize if needed, `occ = 0`
  - [x] `Row::cols(&self) -> usize` — returns `inner.len()`
  - [x] `Row::occ(&self) -> usize` — returns occupancy
  - [x] `impl Index<Column> for Row` — returns `&Cell` at column
  - [x] `impl IndexMut<Column> for Row` — returns `&mut Cell` at column, updates `occ` if needed
  - [x] `Row::clear_range(&mut self, range: Range<Column>, template: &Cell)` — clear cells in range
  - [x] `Row::truncate(&mut self, col: Column)` — clear from col to end, update occ
  - [x] `Row::append(&mut self, col: Column, cell: &Cell)` — write cell at col, update occ
- [x] **Tests** (`oriterm_core/src/grid/row.rs` `#[cfg(test)]`) — 21 tests, all pass (verified 2026-03-29):
  - [x] New row has correct length, all default cells, occ = 0
  - [x] Writing a cell at column 5 sets occ = 6
  - [x] Reset clears all cells and resets occ
  - [x] Index/IndexMut return correct cells
  - [x] clear_range resets specified columns
  - [x] truncate clears from column to end
  - [ ] `Row::is_blank()` — no direct unit test (WEAK TESTS — only tested indirectly)
  - [ ] `Row::content_len()` — no direct unit test (WEAK TESTS — only tested indirectly)

---

## 1.5 Grid Foundation

The Grid is the 2D cell storage. At this stage: a simple Vec of Rows with dimensions. No scrollback yet (added in 1.10).

**File:** `oriterm_core/src/grid/mod.rs`

- [x] Module declarations: `mod row; mod cursor; mod scroll; mod editing; mod navigation; mod ring; mod dirty;`
- [x] Re-export key types
- [x] `Grid` struct (initial, no scrollback)
  - [x] Fields:
    - `rows: Vec<Row>` — visible rows (indexed 0 = top, N-1 = bottom)
    - `cols: usize` — number of columns
    - `lines: usize` — number of visible lines
    - `cursor: Cursor` — current cursor position + template
    - `saved_cursor: Option<Cursor>` — DECSC/DECRC saved cursor
    - `tab_stops: Vec<bool>` — tab stop at each column (default every 8)
  - [x] `Grid::new(lines: usize, cols: usize) -> Self`
    - [x] Allocate `lines` rows of `cols` cells each
    - [x] Initialize tab stops every 8 columns
    - [x] Cursor at (0, 0) with default template
  - [x] `Grid::lines(&self) -> usize`
  - [x] `Grid::cols(&self) -> usize`
  - [x] `Grid::cursor(&self) -> &Cursor`
  - [x] `Grid::cursor_mut(&mut self) -> &mut Cursor`
  - [x] `impl Index<Line> for Grid` — returns `&Row` (Line(0) = first visible row)
  - [x] `impl IndexMut<Line> for Grid` — returns `&mut Row`
- [x] **Tests** (`oriterm_core/src/grid/mod.rs` `#[cfg(test)]`) — 9 tests, all pass (verified 2026-03-29):
  - [x] New grid has correct dimensions
  - [x] Tab stops initialized at every 8 columns
  - [x] Index by Line returns correct row
  - [x] Cursor starts at (0, 0)
  - [ ] `Grid::reset()` — no direct unit test (WEAK TESTS — only tested indirectly via term tests)

---

## 1.6 Cursor

The cursor tracks the current write position and the "template cell" used for newly written characters.

**File:** `oriterm_core/src/grid/cursor.rs`

- [x] `Cursor` struct
  - [x] Fields:
    - `point: Point<usize>` — line (usize index into visible rows), column
    - `template: Cell` — cell template: fg, bg, flags applied to new characters
    - `shape: CursorShape` — block, underline, bar (for rendering)
  - [x] `Cursor::new() -> Self` — point at (0, 0), default template, block shape
  - [x] `Cursor::line(&self) -> usize`
  - [x] `Cursor::col(&self) -> Column`
  - [x] `Cursor::set_line(&mut self, line: usize)`
  - [x] `Cursor::set_col(&mut self, col: Column)`
- [x] `CursorShape` enum — `Block`, `Underline`, `Bar`, `HollowBlock`
  - [x] `Default` impl returns `Block`
- [x] **Tests** — 6 tests, all pass (verified 2026-03-29):
  - [x] Default cursor at (0, 0) with block shape
  - [x] Setting line/col updates point
  - [ ] `CursorShape::Hidden` not tested for distinctness from other variants (WEAK TESTS — `cursor_shape_all_variants_distinct` explicitly excludes Hidden)

---

## 1.7 Grid Editing

Character insertion, deletion, and erase operations. These are the primitives the VTE handler will call.

**File:** `oriterm_core/src/grid/editing.rs`

Methods on `Grid`:

- [x] `put_char(&mut self, ch: char)`
  - [x] Write `ch` into cell at cursor position, using cursor template for colors/flags
  - [x] Handle wide chars: write cell with `WIDE_CHAR` flag, write spacer in next column with `WIDE_CHAR_SPACER`
  - [x] If cursor is at last column, set `WRAP` flag but don't advance (next char triggers scroll + wrap)
  - [x] Otherwise, advance cursor column by character width
  - [x] If overwriting a wide char spacer, clear the preceding wide char cell
  - [x] If overwriting a wide char, clear its spacer
  - [x] Mark row dirty  <!-- blocked-by:1.11 -->
- [x] `insert_blank(&mut self, count: usize)`
  - [x] Insert `count` blank cells at cursor, shifting existing cells right
  - [x] Cells that shift past the right edge are lost
  - [x] Mark row dirty  <!-- blocked-by:1.11 -->
- [x] `delete_chars(&mut self, count: usize)`
  - [x] Delete `count` cells at cursor, shifting remaining cells left
  - [x] New cells at right edge are blank (cursor template)
  - [x] Mark row dirty  <!-- blocked-by:1.11 -->
- [x] `erase_display(&mut self, mode: EraseMode)`
  - [x] `EraseMode::Below` — erase from cursor to end of display
  - [x] `EraseMode::Above` — erase from start of display to cursor
  - [x] `EraseMode::All` — erase entire display
  - [x] `EraseMode::Scrollback` — erase scrollback buffer only
  - [x] Mark affected rows dirty  <!-- blocked-by:1.11 -->
- [x] `erase_line(&mut self, mode: EraseMode)`
  - [x] `Below` — erase from cursor to end of line
  - [x] `Above` — erase from start of line to cursor
  - [x] `All` — erase entire line
  - [x] Mark row dirty  <!-- blocked-by:1.11 -->
- [x] `erase_chars(&mut self, count: usize)`
  - [x] Erase `count` cells starting at cursor (replace with template, don't shift)
  - [x] Mark row dirty  <!-- blocked-by:1.11 -->
- [x] `EraseMode` enum — `Below`, `Above`, `All`, `Scrollback`
- [x] **Tests** (`oriterm_core/src/grid/editing.rs` `#[cfg(test)]`) — 66 tests, all pass (verified 2026-03-29):
  - [x] `put_char('A')` at (0,0) writes 'A', cursor advances to col 1
  - [x] `put_char('好')` (wide) writes 好 + spacer, cursor advances by 2
  - [x] Wide char at last column: wraps correctly
  - [x] Overwriting spacer clears preceding wide char
  - [x] Overwriting wide char clears its spacer
  - [x] `insert_blank(3)` shifts cells right by 3
  - [x] `delete_chars(2)` shifts cells left by 2, blanks at right
  - [x] `erase_display(Below)` clears from cursor to end
  - [x] `erase_display(Above)` clears from start to cursor
  - [x] `erase_display(All)` clears everything
  - [x] `erase_line(Below)` clears from cursor to end of line
  - [x] `erase_line(All)` clears entire line
  - [x] `erase_chars(5)` erases 5 cells without shifting
  - [ ] `put_char_ascii()` fast path — no dedicated unit test (WEAK TESTS — only tested indirectly via TermHandler)
  - [ ] `push_zerowidth()` (Grid method) — no dedicated unit test (WEAK TESTS — Cell::push_zerowidth tested, but Grid method not directly)
  - [ ] Zero-count `insert_blank(0)`, `delete_chars(0)`, `erase_chars(0)` — no tests, produces false-positive dirty marks (TPR-01-001)

---

## 1.8 Grid Navigation

Cursor movement operations. The VTE handler calls these for CUU/CUD/CUF/CUB/CUP/CR/LF/etc.

**File:** `oriterm_core/src/grid/navigation.rs`

Methods on `Grid`:

- [x] `move_up(&mut self, count: usize)` — CUU: move cursor up, clamped to top of screen (or scroll region)
- [x] `move_down(&mut self, count: usize)` — CUD: move cursor down, clamped to bottom of screen (or scroll region)
- [x] `move_forward(&mut self, count: usize)` — CUF: move cursor right, clamped to last column
- [x] `move_backward(&mut self, count: usize)` — CUB: move cursor left, clamped to column 0
- [x] `move_to(&mut self, line: usize, col: Column)` — CUP: absolute position, clamped to grid bounds
- [x] `move_to_column(&mut self, col: Column)` — CHA: absolute column, clamped
- [x] `move_to_line(&mut self, line: usize)` — VPA: absolute line, clamped
- [x] `carriage_return(&mut self)` — CR: cursor to column 0
- [x] `linefeed(&mut self)` — LF: move down one line; if at bottom of scroll region, scroll up
- [x] `reverse_index(&mut self)` — RI: move up one line; if at top of scroll region, scroll down
- [x] `next_line(&mut self)` — NEL: carriage return + linefeed
- [x] `tab(&mut self)` — HT: advance to next tab stop (or end of line)
  - [x] Respects `self.tab_stops` vector
- [x] `tab_backward(&mut self)` — CBT: move to previous tab stop (or start of line)
- [x] `set_tab_stop(&mut self)` — HTS: set tab stop at current column
- [x] `clear_tab_stop(&mut self, mode: TabClearMode)` — TBC: clear current or all tab stops
- [x] `TabClearMode` enum — `Current`, `All`
- [x] `save_cursor(&mut self)` — DECSC: save cursor position + template to `saved_cursor`
- [x] `restore_cursor(&mut self)` — DECRC: restore from `saved_cursor` (or reset if none)
- [x] **Tests** (`oriterm_core/src/grid/navigation.rs` `#[cfg(test)]`) — 57 tests, all pass (verified 2026-03-29):
  - [x] `move_up(3)` from line 5 → line 2
  - [x] `move_up(100)` from line 5 → line 0 (clamped)
  - [x] `move_down(3)` from line 0 → line 3
  - [x] `move_down(100)` clamps to bottom
  - [x] `move_forward(5)` from col 0 → col 5
  - [x] `move_forward(100)` clamps to last column
  - [x] `move_backward(3)` from col 5 → col 2
  - [x] `move_to(5, 10)` sets cursor to (5, 10)
  - [x] `carriage_return` sets col to 0
  - [x] `linefeed` at bottom of screen triggers scroll
  - [x] `linefeed` in middle of screen moves cursor down
  - [x] `reverse_index` at top triggers scroll_down
  - [x] `tab` advances to next tab stop
  - [x] `tab` at last tab stop goes to end of line
  - [x] `tab_backward` moves to previous tab stop
  - [x] `set_tab_stop` / `clear_tab_stop` work correctly
  - [x] `save_cursor` / `restore_cursor` round-trip

---

## 1.9 Grid Scrolling

Scroll operations within scroll regions. A scroll region is a range of lines (set by DECSTBM).

**File:** `oriterm_core/src/grid/scroll.rs`

- [x] Add to `Grid`:
  - [x] Field: `scroll_region: Range<usize>` — top..bottom (default: 0..lines)
  - [x] `set_scroll_region(&mut self, top: usize, bottom: Option<usize>)` — DECSTBM
    - [x] Validate: top < bottom, both within grid bounds
    - [x] Store as `top..bottom` (0-based half-open range)
- [x] `scroll_up(&mut self, count: usize)`
  - [x] Move rows in scroll region up by `count`
  - [x] Top rows go to scrollback (if scroll region is full screen) or are lost (if sub-region)
  - [x] New blank rows appear at bottom of region
  - [x] Mark affected rows dirty  <!-- blocked-by:1.11 -->
- [x] `scroll_down(&mut self, count: usize)`
  - [x] Move rows in scroll region down by `count`
  - [x] Bottom rows are lost
  - [x] New blank rows appear at top of region
  - [x] Mark affected rows dirty  <!-- blocked-by:1.11 -->
- [x] `insert_lines(&mut self, count: usize)` — IL: insert blank lines at cursor, pushing down
  - [x] Only operates within scroll region
  - [x] Cursor must be within scroll region
- [x] `delete_lines(&mut self, count: usize)` — DL: delete lines at cursor, pulling up
  - [x] Only operates within scroll region
  - [x] New blank lines at bottom of region
- [x] **Tests** (`oriterm_core/src/grid/scroll.rs` `#[cfg(test)]`) — 66 tests, all pass (verified 2026-03-29):
  - [x] `scroll_up(1)` with full-screen region: top row evicted, blank at bottom
  - [x] `scroll_up(3)` with sub-region: only region rows move
  - [x] `scroll_down(1)`: bottom row lost, blank at top of region
  - [x] `insert_lines(2)` at cursor line: 2 blank lines inserted, bottom rows lost
  - [x] `delete_lines(2)` at cursor line: 2 lines removed, blanks at bottom
  - [x] Scroll region boundaries respected (rows outside region untouched)
  - [x] `set_scroll_region` with invalid values is clamped

---

## 1.10 Scrollback Ring Buffer

Efficient storage for scrollback history. Rows that scroll off the top go into a ring buffer. Users can scroll up to view history.

**File:** `oriterm_core/src/grid/ring.rs`

- [x] `ScrollbackBuffer` struct
  - [x] Fields:
    - `buf: Vec<Row>` — pre-allocated ring buffer
    - `max_scrollback: usize` — maximum history lines (configurable, default 10000)
    - `len: usize` — current number of rows in buffer
    - `start: usize` — ring buffer start index
  - [x] `ScrollbackBuffer::new(max_scrollback: usize) -> Self`
  - [x] `push(&mut self, row: Row)` — add row to scrollback (evicts oldest if full)
  - [x] `len(&self) -> usize` — number of rows stored
  - [x] `get(&self, index: usize) -> Option<&Row>` — index 0 = most recent, len-1 = oldest
  - [x] `iter(&self) -> impl Iterator<Item = &Row>` — iterate newest to oldest
  - [x] `clear(&mut self)` — clear all scrollback
- [x] Integrate with `Grid`:
  - [x] Add field: `scrollback: ScrollbackBuffer`
  - [x] Add field: `display_offset: usize` — how many lines scrolled back (0 = live)
  - [x] `Grid::scroll_up` pushes evicted rows to scrollback (when scroll region is full screen)
  - [x] `Grid::total_lines(&self) -> usize` — `self.lines + self.scrollback.len()`
  - [x] `Grid::display_offset(&self) -> usize`
  - [x] `Grid::scroll_display(&mut self, delta: isize)` — adjust display_offset, clamped
- [x] **Tests** (`oriterm_core/src/grid/ring.rs` `#[cfg(test)]`) — 38 tests, all pass (verified 2026-03-29):
  - [x] Push rows into scrollback, verify retrieval order (newest first)
  - [x] Ring buffer wraps: push max+10 rows, only max retained
  - [x] Clear empties the buffer
  - [x] Integration: scroll_up pushes to scrollback
  - [x] display_offset scrolls through history
  - [x] display_offset clamped to scrollback length

---

## 1.11 Dirty Tracking

Track which rows have changed since last read. Enables damage-based rendering.

**File:** `oriterm_core/src/grid/dirty.rs`

- [x] `DirtyTracker` struct
  - [x] Fields:
    - `dirty: Vec<bool>` — one bool per visible row
    - `all_dirty: bool` — shortcut: everything changed (resize, scroll, alt screen swap)
  - [x] `DirtyTracker::new(lines: usize) -> Self` — all clean
  - [x] `mark(&mut self, line: usize)` — mark single line dirty
  - [x] `mark_all(&mut self)` — mark everything dirty
  - [x] `is_dirty(&self, line: usize) -> bool`
  - [x] `is_any_dirty(&self) -> bool`
  - [x] `drain(&mut self) -> DirtyIterator` — returns iterator of dirty line indices, resets all to clean
  - [x] `resize(&mut self, lines: usize)` — resize tracker, mark all dirty
- [x] Integrate with `Grid`:  <!-- unblocks:1.7 --><!-- unblocks:1.8 --><!-- unblocks:1.9 -->
  - [x] Add field: `dirty: DirtyTracker`
  - [x] All editing/scroll/navigation methods that change cells call `self.dirty.mark(line)`
  - [x] `scroll_up`/`scroll_down` call `self.dirty.mark_all()` (conservative, can optimize later)
- [x] **Tests** (`oriterm_core/src/grid/dirty.rs` `#[cfg(test)]`) — 26 tests, all pass (verified 2026-03-29):
  - [x] New tracker: nothing dirty
  - [x] Mark line 5: only line 5 is dirty
  - [x] Mark all: everything dirty
  - [x] Drain: returns dirty lines, resets to clean
  - [x] After drain, nothing is dirty
  - [x] Resize marks all dirty

---

## 1.R Third Party Review Findings

TPR findings triaged from independent review. All confirmed open as of 2026-03-29 verification.

- [ ] **TPR-01-001 (zero-count damage):** `insert_blank(0)`, `delete_chars(0)`, and `erase_chars(0)` produce false-positive dirty marks despite performing no visible mutation. Fix: add early returns for count==0 and corresponding tests.
  Accepted: Validated against codebase on 2026-03-31. All three functions confirmed to lack explicit early returns for count==0.
- [ ] **TPR-01-002 (rustdoc warning):** `cargo doc -p oriterm_core --no-deps` produces `warning: unresolved link to 'Term::renderable_content_into'` from `term/renderable/mod.rs:188`. Not Section 01 code directly, but blocks the crate-wide doc-clean checklist item.
  Accepted: Validated on 2026-03-31. Warning confirmed present.
- [ ] **TPR-01-003 (500-line limit):** `editing/mod.rs` is 504 lines — 4 lines over the 500-line hard limit. A further extraction (e.g., moving erase operations to `editing/erase.rs`) would resolve this.
  Accepted: Validated on 2026-03-31. File is exactly 504 lines.

---

## 1.12 Section Completion

- [x] All 1.1–1.11 items complete (verified 2026-03-29)
- [x] `cargo test -p oriterm_core` — all tests pass (verified 2026-03-29 — 1429 passed, 0 failed, 2 ignored)
- [x] `cargo clippy -p oriterm_core --target x86_64-pc-windows-gnu` — no warnings
- [ ] `cargo doc -p oriterm_core --no-deps` — generates clean docs (INCOMPLETE — TPR-01-002 rustdoc warning present)
- [x] Grid can: create, write chars (including wide), move cursor, scroll, erase, tab stops, scrollback, dirty tracking (verified 2026-03-29)
- [x] No VTE, no events, no palette, no selection, no rendering — just data structures + operations (verified 2026-03-29)

**Test Coverage Summary:** 331 tests across 4720 lines of test code (verified 2026-03-29). No TODOs, FIXMEs, HACKs, or `#[ignore]` in any Section 01 code. No `unwrap()` in production code.

**Exit Criteria:** `oriterm_core` compiles, all grid operations are tested, `cargo test -p oriterm_core` passes with zero failures.
