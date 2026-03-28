---
section: 1
title: Cell + Grid
status: in-progress
reviewed: true
third_party_review:
  status: findings
  updated: 2026-03-26
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
    status: complete
  - id: "1.5"
    title: Grid Foundation
    status: complete
  - id: "1.6"
    title: Cursor
    status: complete
  - id: "1.7"
    title: Grid Editing
    status: complete
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
    title: "Third Party Review Findings"
    status: not-started
  - id: "1.12"
    title: Section Completion
    status: in-progress
---

# Section 01: Cell + Grid

**Status:** In Progress
**Goal:** Build the foundational data structures for terminal emulation in `oriterm_core`. Every terminal operation ultimately reads or writes cells in a grid. This layer must be rock-solid before anything else is built on top.

**Crate:** `oriterm_core`
**Dependencies:** `bitflags`, `vte` (Color types only), `unicode-width`, `log`

---

## 1.1 Workspace Setup

Convert the single-crate project into a Cargo workspace with `oriterm_core` as the first library crate.

- [x] Create `oriterm_core/` directory with `Cargo.toml` and `src/lib.rs`
  - [x] `Cargo.toml`: name = `oriterm_core`, edition = 2024, same lint config as root
  - [x] Dependencies: `bitflags = "2"`, `vte = { version = "0.15.0", features = ["ansi"] }`, `unicode-width = "0.2"`, `log = "0.4"`
  - [x] `src/lib.rs`: module declarations, `//!` doc comment, `#![deny(unsafe_code)]`
- [x] Convert root `Cargo.toml` to workspace
  - [x] Add `[workspace]` section with `members = ["oriterm_core", "oriterm"]`
  - [x] Move binary crate to `oriterm/` directory
  - [x] `oriterm/Cargo.toml`: depends on `oriterm_core = { path = "../oriterm_core" }`
  - [x] Binary at `oriterm/src/main.rs` (move current `src/main.rs`)
- [x] Verify: `cargo build --target x86_64-pc-windows-gnu` succeeds for workspace
- [x] Verify: `cargo test -p oriterm_core` runs (even if no tests yet)

---

## 1.2 Index Newtypes

Type-safe indices prevent mixing up row/column/line values. These are used everywhere.

**File:** `oriterm_core/src/index/mod.rs` (directory module with `tests.rs` sibling)

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
- [x] **Tests** (`oriterm_core/src/index/tests.rs`):
  - [x] `line_arithmetic` — add, subtract, negative lines
  - [x] `line_assign_arithmetic` — `+=`, `-=`
  - [x] `line_conversions` — `From<i32>`, `From<Line> for i32`
  - [x] `line_display` — `Display` trait output
  - [x] `column_arithmetic` — add, subtract
  - [x] `column_assign_arithmetic` — `+=`, `-=`
  - [x] `column_conversions` — `From<usize>`, `From<Column> for usize`
  - [x] `column_display` — `Display` trait output
  - [x] `point_ordering` — line takes priority over column
  - [x] `point_ordering_with_negative_lines` — history (`Line(-1)`) orders before visible (`Line(0)`)
  - [x] `side_equality` / `direction_equality` — enum variant equality
  - [x] `point_default_is_origin` — `Default` impl yields `(Line(0), Column(0))`
  - [x] `line_ordering` / `column_ordering` — `Ord` impl correctness
  - [x] `point_same_line_column_breaks_tie` — column tiebreaker on same line

---

## 1.3 Cell Types

A Cell represents one character position in the terminal grid. Must be compact (target: 24 bytes) because there are `rows x cols x scrollback` of them.

**File:** `oriterm_core/src/cell/mod.rs` (directory module with `tests.rs` sibling)

**Reference:** `_old/src/cell.rs` — carry forward the proven 24-byte layout.

- [x] `CellFlags` — `bitflags! { struct CellFlags: u16 { ... } }`
  - [x] `BOLD`, `DIM`, `ITALIC`, `UNDERLINE`, `BLINK`, `INVERSE`, `HIDDEN`, `STRIKETHROUGH`
  - [x] `WIDE_CHAR` — This cell is a wide character (width 2)
  - [x] `WIDE_CHAR_SPACER` — This cell is the trailing spacer of a wide character
  - [x] `WRAP` — Line wrapped at this cell (soft wrap)
  - [x] `CURLY_UNDERLINE`, `DOTTED_UNDERLINE`, `DASHED_UNDERLINE`, `DOUBLE_UNDERLINE`
  - [x] `LEADING_WIDE_CHAR_SPACER` — Padding cell at `cols-1` when a wide char wraps to next line; skipped by text extraction, selection, search, and reflow
  - [x] `ALL_UNDERLINES` — Union constant for mutual exclusion of underline variants
  - [x] Tests: set/clear/query individual flags, combine flags with `|`
- [x] `CellExtra` — Heap-allocated optional data (only for cells that need it)
  - [x] Fields:
    - `underline_color: Option<vte::ansi::Color>` — colored underline (SGR 58)
    - `hyperlink: Option<Hyperlink>` — OSC 8 hyperlink
    - `zerowidth: Vec<char>` — combining marks / zero-width characters appended to this cell
  - [x] Wrapped in `Option<Arc<CellExtra>>` in Cell — None for normal cells (zero overhead), Arc enables O(1) clone for cursor template propagation
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
    - `extra: Option<Arc<CellExtra>>` (8 bytes pointer, None = 0)
  - [x] `Cell::default()` — space character, default fg/bg, no flags, no extra
  - [x] `Cell::reset(&mut self, template: &Cell)` — reset to template (for erase operations)
  - [x] `Cell::is_empty(&self) -> bool` — space char, default colors, no flags
  - [x] `Cell::width(&self) -> usize` — returns 2 if `WIDE_CHAR`, 0 if `WIDE_CHAR_SPACER`/`LEADING_WIDE_CHAR_SPACER`, else `UnicodeWidthChar::width(self.ch).unwrap_or(1)`
  - [x] Derive: `Debug`, `Clone`, `PartialEq`, `Eq`
- [x] Verify `std::mem::size_of::<Cell>()` <= 24 bytes
  - [x] Add compile-time assert: `const _: () = assert!(std::mem::size_of::<Cell>() <= 24);`
- [x] Re-export `Cell`, `CellFlags`, `CellExtra`, `Hyperlink` from `lib.rs`
- [x] **Tests** (`oriterm_core/src/cell/tests.rs`):
  - [x] `size_assertion` — `size_of::<Cell>() <= 24`
  - [x] `default_cell_is_space_with_default_colors` — space, Named(Foreground/Background), no flags, no extra
  - [x] `reset_clears_to_template` / `reset_copies_template_extra` / `reset_clears_extra_when_template_has_none` — full reset coverage
  - [x] `is_empty_for_default` / `is_empty_false_after_setting_char` / `is_empty_false_for_non_default_bg` / `is_empty_false_for_flags` / `is_empty_false_for_extra` — exhaustive `is_empty` edge cases
  - [x] `wide_char_width` / `spacer_width` / `normal_char_width` — width dispatch per flag
  - [x] `width_cjk_ideographic_space` — U+3000 ideographic space (width 2)
  - [x] `width_emoji` — emoji crab U+1F980 (width 2 via WIDE_CHAR flag)
  - [x] `extra_is_none_for_normal_cells` / `extra_created_for_underline_color` / `extra_created_for_hyperlink` — CellExtra presence
  - [x] `push_zerowidth_creates_extra` / `push_zerowidth_multiple_marks` — combining mark accumulation
  - [x] `clone_shares_arc_refcount` — Arc sharing on clone (no deep copy)
  - [x] `push_zerowidth_cow_on_shared_arc` — COW semantics: mutating cloned cell doesn't affect original
  - [x] `from_color_creates_bce_cell` — `Cell::from(Color)` for BCE (background color erase)
  - [x] `cellflags_set_clear_query` / `cellflags_combine` — bitflags operations
  - [x] `hyperlink_display` — `Display` trait on `Hyperlink`

---

## 1.4 Row

A Row is a contiguous array of Cells representing one terminal line.

**File:** `oriterm_core/src/grid/row/mod.rs` (directory module with `tests.rs` sibling)

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
  - [x] `Row::truncate(&mut self, col: Column, template: &Cell)` — clear from col to end with BCE template, update occ
  - [x] `Row::append(&mut self, col: Column, cell: &Cell)` — write cell at col, update occ (test-only helper)
  - [x] `Row::is_blank(&self) -> bool` — whether row contains only empty cells
  - [x] `Row::content_len(&self) -> usize` — index of last non-empty cell + 1
  - [x] `Row::resize(&mut self, new_cols: usize)` — resize row, padding or truncating
- [x] **Tests** (`oriterm_core/src/grid/row/tests.rs`):
  - [x] `new_row_has_correct_length_and_defaults` — cols, occ=0, cells empty
  - [x] `writing_cell_updates_occ` — append at col 5 sets occ=6
  - [x] `reset_clears_and_resets_occ` — reset clears cells and occ
  - [x] `index_returns_correct_cell` / `index_mut_updates_occ` — Index/IndexMut correctness
  - [x] `clear_range_resets_columns` / `clear_range_full_row` / `clear_range_with_bce` — range clear coverage
  - [x] `truncate_clears_from_column_to_end` / `truncate_at_col_zero_clears_entire_row` — truncate edge cases
  - [x] `reset_bce_across_consecutive_resets` — BCE bg mismatch forces repaint on second reset
  - [x] `reset_resizes_row_larger` / `reset_shrinks_row` — resize during reset
  - [x] `append_empty_cell_does_not_bump_occ` — empty cell append is no-op on occ
  - [x] `row_equality` — `PartialEq` for identical/different rows
  - [x] `clear_range_bce_updates_occ` / `clear_range_bce_survives_reset` — BCE + occ interaction
  - [x] `truncate_bce_updates_occ` — BCE truncate covers all dirty cells
  - [x] `clear_range_inverted_is_noop` / `clear_range_start_beyond_row_is_noop` / `truncate_beyond_row_is_noop` — boundary safety (no panic)

---

## 1.5 Grid Foundation

The Grid is the 2D cell storage. At this stage: a simple Vec of Rows with dimensions. No scrollback yet (added in 1.10).

**File:** `oriterm_core/src/grid/mod.rs`

- [x] Module declarations: `mod row; mod cursor; mod scroll; mod editing; mod navigation; mod ring; mod dirty; mod resize; mod stable_index; mod snapshot;`
- [x] Re-export key types
- [x] `Grid` struct (initial, no scrollback)
  - [x] Fields:
    - `rows: Vec<Row>` — visible rows (indexed 0 = top, N-1 = bottom)
    - `cols: usize` — number of columns
    - `lines: usize` — number of visible lines
    - `cursor: Cursor` — current cursor position + template
    - `saved_cursor: Option<Cursor>` — DECSC/DECRC saved cursor
    - `tab_stops: Vec<bool>` — tab stop at each column (default every 8)
    - `total_evicted: usize` — rows evicted from scrollback (for `StableRowIndex` stability)
    - `resize_pushed: usize` — reflow overflow counter consumed by `scroll_up`/`erase_display(All)`
  - [x] `Grid::new(lines: usize, cols: usize) -> Self`
    - [x] Allocate `lines` rows of `cols` cells each
    - [x] Initialize tab stops every 8 columns
    - [x] Cursor at (0, 0) with default template
  - [x] `Grid::with_scrollback(lines, cols, max_scrollback) -> Self` — explicit scrollback capacity constructor (used in ring/resize tests)
  - [x] `Grid::lines(&self) -> usize`
  - [x] `Grid::cols(&self) -> usize`
  - [x] `Grid::cursor(&self) -> &Cursor`
  - [x] `Grid::cursor_mut(&mut self) -> &mut Cursor`
  - [x] `impl Index<Line> for Grid` — returns `&Row` (Line(0) = first visible row)
  - [x] `impl IndexMut<Line> for Grid` — returns `&mut Row`
- [x] **Tests** (`oriterm_core/src/grid/tests.rs`):
  - [x] `new_grid_has_correct_dimensions` — lines, cols match constructor args
  - [x] `tab_stops_every_8_columns` — stops at 0, 8, 16, 24, 72; not at 1, 79
  - [x] `index_by_line_returns_correct_row` — `grid[Line(0)]` and `grid[Line(23)]` valid
  - [x] `cursor_starts_at_origin` — line=0, col=Column(0)
  - [x] `grid_1x1_minimum_dimensions` — 1x1 grid works, single cell is empty
  - [x] `scroll_region_defaults_to_full_grid` — `0..lines`
  - [x] `saved_cursor_starts_as_none` — no saved cursor initially
  - [x] `tab_stops_for_narrow_grid` — grid narrower than 8 cols: only col 0 is stop
  - [x] `all_rows_initialized_empty` — all cells in all rows are `is_empty()`

---

## 1.6 Cursor

The cursor tracks the current write position and the "template cell" used for newly written characters.

**File:** `oriterm_core/src/grid/cursor/mod.rs` (directory module with `tests.rs` sibling)

- [x] `Cursor` struct
  - [x] Fields:
    - `line: usize` — line index into visible rows (0-based)
    - `col: Column` — column index (0-based)
    - `template: Cell` — cell template: fg, bg, flags applied to new characters
  - [x] Note: `CursorShape` is stored on `Term`, not `Cursor` (DECSCUSR is global, not per-screen)
  - [x] `Cursor::new() -> Self` — line=0, col=0, default template
  - [x] `Cursor::line(&self) -> usize`
  - [x] `Cursor::col(&self) -> Column`
  - [x] `Cursor::set_line(&mut self, line: usize)`
  - [x] `Cursor::set_col(&mut self, col: Column)`
- [x] `CursorShape` enum — `Block`, `Underline`, `Bar`, `HollowBlock`, `Hidden`
  - [x] `Default` impl returns `Block`
- [x] **Tests** (`oriterm_core/src/grid/cursor/tests.rs`):
  - [x] `default_cursor_at_origin` — line=0, col=Column(0)
  - [x] `set_line_and_col` — setting line/col updates point
  - [x] `default_shape_is_block` — `CursorShape::default() == Block`
  - [x] `template_defaults_to_empty_cell` — cursor template starts as `is_empty()`
  - [x] `cursor_clone_preserves_all_fields` — clone retains line, col, template
  - [x] `cursor_shape_all_variants_distinct` — all 4 non-Hidden variants are distinct via `PartialEq`

---

## 1.7 Grid Editing

Character insertion, deletion, and erase operations. These are the primitives the VTE handler will call.

**File:** `oriterm_core/src/grid/editing/mod.rs` (directory module with `tests.rs` sibling + `wide_char.rs` submodule)

Methods on `Grid`:

- [x] `put_char(&mut self, ch: char)`
  - [x] Write `ch` into cell at cursor position, using cursor template for colors/flags
  - [x] Handle wide chars: write cell with `WIDE_CHAR` flag, write spacer in next column with `WIDE_CHAR_SPACER`
  - [x] If cursor is at last column, set `WRAP` flag but don't advance (next char triggers scroll + wrap)
  - [x] Otherwise, advance cursor column by character width
  - [x] If overwriting a wide char spacer, clear the preceding wide char cell
  - [x] If overwriting a wide char, clear its spacer
  - [x] Mark row dirty
- [x] `put_char_ascii(&mut self, ch: char) -> bool` — fast path for ASCII 0x20-0x7E, skips width lookup and wide char handling, returns false if wrap-pending or wide char cleanup needed
- [x] `push_zerowidth(&mut self, ch: char)` — append combining mark to previous cell, handles wrap-pending and wide-char spacers
- [x] `insert_blank(&mut self, count: usize)`
  - [x] Insert `count` blank cells at cursor, shifting existing cells right
  - [x] Cells that shift past the right edge are lost
  - [x] Mark row dirty
- [x] `delete_chars(&mut self, count: usize)`
  - [x] Delete `count` cells at cursor, shifting remaining cells left
  - [x] New cells at right edge are blank (cursor template)
  - [x] Mark row dirty
- [x] `erase_display(&mut self, mode: DisplayEraseMode)`
  - [x] `DisplayEraseMode::Below` — erase from cursor to end of display
  - [x] `DisplayEraseMode::Above` — erase from start of display to cursor
  - [x] `DisplayEraseMode::All` — erase entire display
  - [x] `DisplayEraseMode::Scrollback` — erase scrollback buffer only
  - [x] Mark affected rows dirty
- [x] `erase_line(&mut self, mode: LineEraseMode)`
  - [x] `Right` — erase from cursor to end of line
  - [x] `Left` — erase from start of line to cursor
  - [x] `All` — erase entire line
  - [x] Mark row dirty
- [x] `erase_chars(&mut self, count: usize)`
  - [x] Erase `count` cells starting at cursor (replace with template, don't shift)
  - [x] Mark row dirty
- [x] `DisplayEraseMode` enum — `Below`, `Above`, `All`, `Scrollback`
- [x] `LineEraseMode` enum — `Right`, `Left`, `All`
- [x] **Tests** (`oriterm_core/src/grid/editing/tests.rs`, ~1094 lines):
  - [x] Core `put_char`: `put_char_writes_and_advances`, `put_char_wide_writes_pair`, `put_char_sequence_fills_correctly`
  - [x] Template inheritance: `put_char_inherits_template_attributes`, `put_char_wide_spacer_inherits_template_bg`
  - [x] Wrap behavior: `wide_char_at_last_column_wraps`, `put_char_fills_row_and_wraps_to_next_line`, `wrap_flag_set_on_wrapped_line`
  - [x] Wide char boundary: `overwrite_spacer_clears_wide_char`, `overwrite_wide_char_clears_spacer`, `wide_char_on_single_column_grid_does_not_hang`
  - [x] `insert_blank`: basic shift, at end, count exceeds remaining, with BCE, cursor past end is no-op
  - [x] `delete_chars`: basic shift, at end, count exceeds remaining, with BCE, cursor past end is no-op
  - [x] `erase_display`: Below, Above, All, with BCE background, boundary cases (at last/first line)
  - [x] `erase_line`: Right, Left, All, with BCE
  - [x] `erase_chars`: basic erase, past end, with BCE, default bg does not inflate occ
  - [x] Dirty tracking integration: `put_char_marks_cursor_line_dirty`, `put_char_wraparound_marks_new_line_dirty`, `insert_blank_marks_cursor_line_dirty`, `delete_chars_marks_cursor_line_dirty`, `erase_chars_marks_cursor_line_dirty`, `erase_display_*_marks_dirty`, `erase_line_*_marks_dirty`
  - [x] **Note:** `Grid::put_char_ascii` and `Grid::push_zerowidth` lack dedicated unit tests; they are tested indirectly through `term_handler` integration tests. Direct tests would improve coverage.

---

## 1.8 Grid Navigation

Cursor movement operations. The VTE handler calls these for CUU/CUD/CUF/CUB/CUP/CR/LF/etc.

**File:** `oriterm_core/src/grid/navigation/mod.rs` (directory module with `tests.rs` sibling)

Methods on `Grid`:

- [x] `move_up(&mut self, count: usize)` — CUU: move cursor up, clamped to top of screen (or scroll region)
- [x] `move_down(&mut self, count: usize)` — CUD: move cursor down, clamped to bottom of screen (or scroll region)
- [x] `move_forward(&mut self, count: usize)` — CUF: move cursor right, clamped to last column
- [x] `move_backward(&mut self, count: usize)` — CUB: move cursor left, clamped to column 0
- [x] `move_to(&mut self, line: usize, col: Column)` — CUP: absolute position, clamped to grid bounds
- [x] `move_to_column(&mut self, col: Column)` — CHA: absolute column, clamped
- [x] `move_to_line(&mut self, line: usize)` — VPA: absolute line, clamped
- [x] `carriage_return(&mut self)` — CR: cursor to column 0
- [x] `backspace(&mut self)` — BS: move cursor left by one column; handles wrap-pending state
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
- [x] **Tests** (`oriterm_core/src/grid/navigation/tests.rs`, ~678 lines):
  - [x] Basic movement: `move_up`, `move_down`, `move_forward`, `move_backward` with expected deltas
  - [x] Clamping: `move_up/down/forward/backward` with count exceeding bounds, `move_to` out of bounds, `move_to_column`, `move_to_line`
  - [x] Scroll region clamping: `move_up_clamped_to_scroll_region_top`, `move_down_clamped_to_scroll_region_bottom`, `move_up/down_outside_scroll_region_clamps_to_zero/last`
  - [x] Linefeed: at bottom triggers scroll, in middle moves down, preserves column, at last line outside scroll region is no-op
  - [x] Reverse index: at top triggers scroll_down, in middle moves up, preserves column, at line 0 outside scroll region is no-op
  - [x] Next line: `next_line_combines_cr_and_lf`, `next_line_at_bottom_of_scroll_region_scrolls`
  - [x] Backspace: from mid-line, at col 0 (no-op), from wrap-pending snaps to last column, consecutive
  - [x] Carriage return: sets col 0, from wrap-pending
  - [x] Tab: advances to next stop, at last stop goes to end, from col 0, from wrap-pending, after clearing all stops
  - [x] Tab backward: to previous stop, at col 0 stays, from wrap-pending
  - [x] Tab stop management: `set_and_clear_tab_stop` (current + all modes)
  - [x] Save/restore: round-trip, `restore_cursor_without_save_resets_to_origin`, `multiple_saves_overwrite_not_stack`, `save_cursor_preserves_template`
  - [x] Scroll region content: `scroll_region_up/down_preserves_content_outside`, `scroll_region_fill_uses_bce_background`
  - [x] Dirty tracking: `move_up/down/to_marks_old_and_new_lines_dirty`, `carriage_return_marks_current_line_dirty`, `linefeed/reverse_index_non_scroll_marks_dirty`, `restore_cursor_marks_dirty`, `tab_marks_dirty`, `save_cursor_does_not_dirty`

---

## 1.9 Grid Scrolling

Scroll operations within scroll regions. A scroll region is a range of lines (set by DECSTBM).

**File:** `oriterm_core/src/grid/scroll/mod.rs` (directory module with `tests.rs` sibling)

- [x] Add to `Grid`:
  - [x] Field: `scroll_region: Range<usize>` — top..bottom (default: 0..lines)
  - [x] `set_scroll_region(&mut self, top: usize, bottom: Option<usize>)` — DECSTBM
    - [x] Parameters are 1-based (matching VTE/ECMA-48); converted internally to 0-based
    - [x] Validate: region must span at least 2 lines, clamped to grid bounds
    - [x] Store as `top..bottom` (0-based half-open range)
- [x] `scroll_up(&mut self, count: usize)`
  - [x] Move rows in scroll region up by `count`
  - [x] Top rows go to scrollback (if scroll region is full screen) or are lost (if sub-region)
  - [x] New blank rows appear at bottom of region
  - [x] Mark affected rows dirty
- [x] `scroll_down(&mut self, count: usize)`
  - [x] Move rows in scroll region down by `count`
  - [x] Bottom rows are lost
  - [x] New blank rows appear at top of region
  - [x] Mark affected rows dirty
- [x] `insert_lines(&mut self, count: usize)` — IL: insert blank lines at cursor, pushing down
  - [x] Only operates within scroll region
  - [x] Cursor must be within scroll region
- [x] `delete_lines(&mut self, count: usize)` — DL: delete lines at cursor, pulling up
  - [x] Only operates within scroll region
  - [x] New blank lines at bottom of region
- [x] **Tests** (`oriterm_core/src/grid/scroll/tests.rs`, ~1071 lines):
  - [x] `set_scroll_region`: full screen, sub-region, default bottom, invalid top>=bottom, top=0 treated as 1, oversized bottom clamped, does not move cursor
  - [x] `scroll_up`: 1 line full screen, 3 lines full screen, sub-region preserves outside, count exceeds region (clamped), BCE fill
  - [x] `scroll_down`: 1 line full screen, sub-region preserves outside, count exceeds region, BCE fill
  - [x] `insert_lines`: mid-region, outside region is no-op, count capped, BCE fill
  - [x] `delete_lines`: mid-region, outside region is no-op, count capped, BCE fill
  - [x] Display offset stabilization: `scroll_up_stabilizes_display_offset`, `scroll_up_display_offset_clamped_to_max_scrollback`
  - [x] Dirty tracking: `scroll_up/down_marks_affected_region_dirty`, `insert/delete_lines_marks_dirty`, `set_scroll_region_does_not_dirty`
  - [x] Row recycling: scroll_up evicted rows recycled via blank row reuse
  - [x] Scrollback interaction: sub-region scroll does not push to scrollback (verified in ring/tests.rs)

---

## 1.10 Scrollback Ring Buffer

Efficient storage for scrollback history. Rows that scroll off the top go into a ring buffer. Users can scroll up to view history.

**File:** `oriterm_core/src/grid/ring/mod.rs` (directory module with `tests.rs` sibling)

- [x] `ScrollbackBuffer` struct
  - [x] Fields:
    - `inner: Vec<Row>` — ring buffer storage, grows on demand up to `max_scrollback`
    - `max_scrollback: usize` — maximum history lines (configurable, default 10000)
    - `len: usize` — current number of rows in buffer
    - `start: usize` — ring buffer start index
  - [x] `ScrollbackBuffer::new(max_scrollback: usize) -> Self`
  - [x] `push(&mut self, row: Row) -> Option<Row>` — add row to scrollback; returns evicted oldest row for allocation recycling if full
  - [x] `len(&self) -> usize` — number of rows stored
  - [x] `get(&self, index: usize) -> Option<&Row>` — index 0 = most recent, len-1 = oldest
  - [x] `iter(&self) -> impl Iterator<Item = &Row>` — iterate newest to oldest
  - [x] `pop_newest(&mut self) -> Option<Row>` — remove and return newest row (inverse of push, used by resize to pull rows back to viewport)
  - [x] `drain_oldest_first(&mut self) -> Vec<Row>` — drain all rows oldest-first for reflow (avoids clone+reverse)
  - [x] `clear(&mut self)` — clear all scrollback
- [x] Integrate with `Grid`:
  - [x] Add field: `scrollback: ScrollbackBuffer`
  - [x] Add field: `display_offset: usize` — how many lines scrolled back (0 = live)
  - [x] `Grid::scroll_up` pushes evicted rows to scrollback (when scroll region is full screen)
  - [x] `Grid::total_lines(&self) -> usize` — `self.lines + self.scrollback.len()`
  - [x] `Grid::display_offset(&self) -> usize`
  - [x] `Grid::scroll_display(&mut self, delta: isize)` — adjust display_offset, clamped
- [x] **Tests** (`oriterm_core/src/grid/ring/tests.rs`, ~766 lines):
  - [x] Core ring: `new_buffer_is_empty`, `push_and_retrieve_order` (newest=idx 0), `ring_wraps_evicts_oldest`, `ring_wraps_many_extra`
  - [x] Clear: `clear_empties_buffer`, usable after clear
  - [x] Iterator: `iter_newest_to_oldest`, `iter_after_wrap`, `iter_after_pop_push_matches_get`
  - [x] Edge cases: `zero_max_scrollback_returns_pushed_row`, `max_scrollback_returns_configured_limit`, `max_scrollback_one_only_retains_latest`, `max_scrollback_one_pop_push_cycle`
  - [x] Boundary: `exact_capacity_boundary_first_eviction` (start becomes non-zero on max+1)
  - [x] Push return value: `push_returns_none_during_growth`, `push_returns_evicted_row_when_full`
  - [x] `pop_newest`: empty returns None, until empty, then push full buffer, twice then push twice, after wrap, growth phase no placeholder leak, repeated cycles
  - [x] `drain_oldest_first`: empty, growth phase, wrapped ring, exactly full, wrapped many extra, usable after drain
  - [x] Wide char preservation: `wide_char_flags_preserved_in_scrollback`, `wide_char_survives_scrollback_via_grid_scroll_up`
  - [x] Grid integration: `scroll_up_pushes_to_scrollback`, `scroll_up_multiple_pushes_in_order`, `scroll_up_sub_region_does_not_push_to_scrollback`
  - [x] Display offset: `display_offset_scrolls_through_history`, `display_offset_clamped_to_scrollback_len`, `total_lines_reflects_scrollback`
  - [x] Resize interaction: `scroll_up_after_grow_rows_preserves_scrollback` (no placeholder/null rows), `scroll_up_while_scrolled_back_no_duplication`

---

## 1.11 Dirty Tracking

Track which rows have changed since last read. Enables damage-based rendering.

**File:** `oriterm_core/src/grid/dirty/mod.rs` (directory module with `tests.rs` sibling)

- [x] `DirtyTracker` struct
  - [x] Fields:
    - `lines: Vec<LineDamageBounds>` — per-line damage bounds with column range tracking
    - `cols: usize` — number of columns (for full-line marks)
    - `all_dirty: bool` — shortcut: everything changed (resize, scroll, alt screen swap)
  - [x] `LineDamageBounds` — per-line `{dirty: bool, left: usize, right: usize}` for column-level damage
  - [x] `DirtyLine` — yielded by drain: `{line: usize, left: usize, right: usize}`
  - [x] `DirtyTracker::new(num_lines: usize, cols: usize) -> Self` — all clean
  - [x] `mark(&mut self, line: usize)` — mark single line fully dirty (all columns)
  - [x] `mark_cols(&mut self, line: usize, left: usize, right: usize)` — mark specific column range dirty
  - [x] `mark_range(&mut self, range: Range<usize>)` — mark contiguous range of lines dirty
  - [x] `mark_all(&mut self)` — mark everything dirty
  - [x] `is_dirty(&self, line: usize) -> bool`
  - [x] `is_any_dirty(&self) -> bool`
  - [x] `is_all_dirty(&self) -> bool`
  - [x] `col_bounds(&self, line: usize) -> Option<(usize, usize)>` — get column damage range
  - [x] `drain(&mut self) -> DirtyIter` — returns iterator of `DirtyLine` entries with column bounds, resets all to clean
  - [x] `resize(&mut self, num_lines: usize, cols: usize)` — resize tracker, mark all dirty
- [x] Integrate with `Grid`:
  - [x] Add field: `dirty: DirtyTracker`
  - [x] All editing/scroll/navigation methods that change cells call `self.dirty.mark(line)` or `self.dirty.mark_cols(line, left, right)` for column-level precision
  - [x] `scroll_up`/`scroll_down` call `self.dirty.mark_range(range)` on the affected scroll region
- [x] **Tests** (`oriterm_core/src/grid/dirty/tests.rs`, ~314 lines):
  - [x] Core: `new_tracker_is_clean`, `mark_single_line`, `mark_all_makes_everything_dirty`
  - [x] Drain: `drain_returns_dirty_lines` (idempotent duplicate marks), `drain_resets_to_clean`, `drain_mark_all_yields_every_line`, `drain_drop_clears_remaining` (partial iteration + Drop)
  - [x] `mark_reports_full_line_bounds` — full-line mark yields `left=0, right=cols-1`
  - [x] `mark_range`: only target lines, empty range is no-op, drain yields only range, full range sets `all_dirty`, superset sets `all_dirty`, partial does not set `all_dirty`
  - [x] `mark_out_of_bounds_is_safe` — no panic on index > num_lines
  - [x] `resize_marks_all_dirty` — resize changes dimensions and marks all dirty
  - [x] Column-level damage: `mark_cols_single_char`, `mark_cols_expands_range` (two writes union bounds), `mark_cols_erase_range`, `mark_full_line_reports_full_width`, `mark_cols_then_mark_full_expands_to_full`
  - [x] `col_bounds`: returns None for clean line, returns marked range, with `all_dirty` returns full
  - [x] `mark_cols_out_of_bounds_is_safe` — no panic
  - [x] `all_dirty_yields_full_line_bounds_for_unmarked_lines` — individually marked lines keep their bounds under `all_dirty`

---

## 01.R Third Party Review Findings

- [ ] `[TPR-01-001][medium]` `oriterm_core/src/grid/editing/mod.rs:224` — Zero-count edit operations still report grid damage instead of acting as no-ops.
  Evidence: `insert_blank`, `delete_chars`, and `erase_chars` clamp `count` but still fall through to `dirty.mark_cols(...)` at lines 274, 334, and 496. Fresh reproduction in a throwaway offline crate printed dirty ranges for `insert_blank(0)`, `delete_chars(0)`, and `erase_chars(0)` as `[2,4]`, `[0,4]`, and `[0,0]` respectively.
  Impact: Callers that forward a zero parameter get false-positive damage, unnecessary redraw work, and incorrect "grid changed" signaling despite no visible mutation.
  Required plan update: Add explicit zero-count early returns plus direct unit coverage for all three public edit paths before re-closing Section 01.

- [ ] `[TPR-01-002][medium]` `plans/roadmap/section-01-cell-grid.md:527` — Section 01 still claims a clean rustdoc pass, but the current crate emits a broken intra-doc-link warning.
  Evidence: `cargo doc -p oriterm_core --no-deps` on 2026-03-26 warned about unresolved link `Term::renderable_content_into` at `oriterm_core/src/term/renderable/mod.rs:188`.
  Impact: The section completion checklist is currently false in repository state, so `status: complete` / `third_party_review: complete` was not defensible.
  Required plan update: Fix the intra-doc link or narrow the checklist wording to Section 01-owned modules, then rerun rustdoc before restoring complete status.

- [ ] `[TPR-01-003][low]` `oriterm_core/src/grid/editing/mod.rs` — A Section 01 source file still violates the hard 500-line source-file limit.
  Evidence: `wc -l` reports 504 lines for `grid/editing/mod.rs`, while `.claude/rules/code-hygiene.md` defines 500 lines as a hard maximum for non-test source files.
  Impact: Section 01 is out of compliance with the repository hygiene gate even though the functional test suite passes.
  Required plan update: Split `grid/editing/mod.rs` further (for example, shift/erase helpers) and rerun Section 01 verification before re-closing the section.

---

## 1.12 Section Completion

- [x] All 1.1-1.11 items complete
- [x] `cargo test -p oriterm_core` — all tests pass
- [x] `cargo clippy -p oriterm_core --target x86_64-pc-windows-gnu` — no warnings
- [ ] `cargo doc -p oriterm_core --no-deps` — generates clean docs
- [x] Grid can: create, write chars (including wide), move cursor, scroll, erase, tab stops, scrollback, dirty tracking
- [x] No VTE, no events, no palette, no selection, no rendering — just data structures + operations

### Test Coverage Summary

| Test file | Approx. tests | Lines |
|-----------|--------------|-------|
| `index/tests.rs` | 14 | 122 |
| `cell/tests.rs` | 22 | 262 |
| `grid/tests.rs` | 8 | 81 |
| `grid/cursor/tests.rs` | 6 | 61 |
| `grid/row/tests.rs` | 18 | 274 |
| `grid/editing/tests.rs` | ~40 | 1094 |
| `grid/navigation/tests.rs` | ~45 | 677 |
| `grid/scroll/tests.rs` | ~35 | 1071 |
| `grid/ring/tests.rs` | ~35 | 765 |
| `grid/dirty/tests.rs` | 21 | 313 |
| **Total** | **~244** | **4720** |

### Source File Sizes (hygiene check)

All source files are within the 500-line limit except:
- `grid/editing/mod.rs` — **504 lines** (4 over limit). The `wide_char.rs` extraction already mitigated this. A further extraction of erase operations would resolve the overshoot.

### Known Test Gaps

- `Grid::put_char_ascii()` has no dedicated unit test (tested indirectly via `TermHandler::input` fast path).
- `Grid::push_zerowidth()` has no dedicated unit test (tested indirectly via term handler and renderable tests; the `Cell::push_zerowidth()` primitive is well-tested).
- `CursorShape::Hidden` variant has no dedicated assertion (implicit via exhaustiveness, but a `Hidden != Block` assertion would be trivial to add).
- No performance/allocation regression test at this layer (added in later sections via `alloc_regression.rs`).

### Later Extensions to Section 01's Foundation

The following modules were built as part of later sections but live within `grid/` and extend this section's data structures:
- `grid/resize/` — `Grid::resize()` with text reflow (cell-by-cell Ghostty-style). Built in Section 12.
- `grid/stable_index.rs` — `StableRowIndex` (monotonic row identity surviving eviction). Built during rendering work.
- `grid/snapshot.rs` — `GridSnapshot` test helper for human-readable grid diagrams. Built organically during testing.
- `grid/editing/wide_char.rs` — Wide char boundary fixup helpers extracted from `editing/mod.rs` for 500-line limit.

- [ ] `/tpr-review` passed — independent Codex review found no critical or major issues (or all findings triaged)

**Exit Criteria:** `oriterm_core` compiles, all grid operations are tested, `cargo test -p oriterm_core` passes with zero failures.
