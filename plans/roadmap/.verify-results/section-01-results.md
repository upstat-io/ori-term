# Section 01: Cell + Grid -- Verification Results

**Date:** 2026-03-29
**Verifier:** Claude Opus 4.6 (automated)
**Branch:** dev (commit a31012a)

**Context loaded:**
- CLAUDE.md (read in full -- coding standards, performance invariants, crate boundaries, testing requirements)
- `.claude/rules/code-hygiene.md` (read -- file organization, import order, naming, 500-line limit, style)
- `.claude/rules/crate-boundaries.md` (read -- oriterm_core ownership, allowed deps)
- `.claude/rules/impl-hygiene.md` (read -- module boundaries, data flow, error handling)
- `.claude/rules/test-organization.md` (read -- sibling tests.rs pattern, no inline tests, super:: imports)
- Reference: section-01-cell-grid.md frontmatter + all 12 subsections

---

## Test Run Summary

```
cargo test -p oriterm_core: 1429 passed, 0 failed, 2 ignored (profiling), finished in 1.07s
alloc_regression tests: 4 passed, 2 ignored
rss_regression tests: 3 passed
```

All Section 01 tests pass. No hangs, no timeouts.

---

## 1.1 Workspace Setup

**Tests found:** N/A (structural verification)
**Tests run:** `cargo test -p oriterm_core` succeeds.
**Audit:** READ `/home/eric/projects/ori_term/oriterm_core/Cargo.toml` -- name=oriterm_core, edition=workspace, dependencies include bitflags=2, vte=0.15.0 with ansi feature, unicode-width=0.2, log=0.4. Also has base64, parking_lot, regex, image(optional). Extra deps beyond plan spec are fine (later sections added them).
**Audit:** READ `/home/eric/projects/ori_term/oriterm_core/src/lib.rs` -- `#![deny(unsafe_code)]` present at line 8. Module declarations, re-exports for Cell, CellFlags, CellExtra, Hyperlink, Grid, Row, Cursor, CursorShape, index types all present.
**Coverage:** Structural. Workspace layout confirmed by successful build.
**Semantic pin:** `cargo build` and `cargo test` both succeed.
**Hygiene:** `#![deny(unsafe_code)]` present. Lint config via `[lints] workspace = true`.
**Status: VERIFIED**

---

## 1.2 Index Newtypes

**Tests found:** `/home/eric/projects/ori_term/oriterm_core/src/index/tests.rs` (122 lines, 16 tests)
**Tests run:** All pass.
**Audit:** READ `index/mod.rs` (129 lines) -- `Line(i32)`, `Column(usize)`, `Point<L>`, `Side`, `Direction`, `Boundary` all defined. `index_ops!` macro generates From, Add, Sub, AddAssign, SubAssign, Display for both Line and Column. Point has manual Ord (line-first, column tiebreak). All derive attributes match plan spec.
**Audit:** READ `index/tests.rs` -- All 16 tests present matching plan spec:
  - `line_arithmetic`, `line_assign_arithmetic`, `line_conversions`, `line_display`
  - `column_arithmetic`, `column_assign_arithmetic`, `column_conversions`, `column_display`
  - `point_ordering`, `point_ordering_with_negative_lines`
  - `side_equality`, `direction_equality`
  - `point_default_is_origin`, `line_ordering`, `column_ordering`, `point_same_line_column_breaks_tie`

**Coverage assessment:**
| Input/State | Tested |
|---|---|
| Line arithmetic (add/sub/assign) | Yes |
| Line negative values | Yes |
| Column arithmetic | Yes |
| Point ordering (line priority) | Yes |
| Point negative lines | Yes |
| Side/Direction equality | Yes |
| Default values | Yes |
| Overflow/underflow (Column sub below 0) | No (would panic -- Rust unsigned) |

**Semantic pin:** `point_ordering_with_negative_lines` uniquely pins that Line(-1) < Line(0). `point_same_line_column_breaks_tie` pins column tiebreak behavior.
**Hygiene:** File at bottom has `#[cfg(test)] mod tests;`. No inline tests. Imports follow `super::` pattern. Module doc comment present (`//!`). All pub items documented. File under 500 lines (129).
**Status: VERIFIED**

---

## 1.3 Cell Types

**Tests found:** `/home/eric/projects/ori_term/oriterm_core/src/cell/tests.rs` (262 lines, 26 tests)
**Tests run:** All pass.
**Audit:** READ `cell/mod.rs` (250 lines) -- CellFlags bitflags with all 16 flags listed in plan (BOLD through LEADING_WIDE_CHAR_SPACER, plus ALL_UNDERLINES). CellExtra struct with underline_color, hyperlink, zerowidth fields. Hyperlink struct with id/uri. Cell struct with ch, fg, bg, flags, extra fields. Compile-time size assert: `const _: () = assert!(size_of::<Cell>() <= 24);` at line 133.

Methods present: `Cell::default()` (space, Named colors, empty flags, no extra), `Cell::reset()`, `Cell::is_empty()`, `Cell::width()` (WIDE_CHAR -> 2, SPACER -> 0, else unicode-width), `Cell::set_underline_color()`, `Cell::set_hyperlink()`, `Cell::hyperlink()`, `Cell::push_zerowidth()`. `From<Color> for Cell` (BCE). `From<vte::ansi::Hyperlink> for Hyperlink`. `Display for Hyperlink`.

**Audit:** READ `cell/tests.rs` -- All 26 tests present, matching plan spec plus extras:
  - Size: `size_assertion`
  - Default: `default_cell_is_space_with_default_colors`
  - Reset: `reset_clears_to_template`, `reset_copies_template_extra`, `reset_clears_extra_when_template_has_none`
  - is_empty: 5 variants (default, char, bg, flags, extra)
  - Width: `wide_char_width`, `spacer_width`, `normal_char_width`, `width_cjk_ideographic_space`, `width_emoji`
  - Extra: `extra_is_none_for_normal_cells`, `extra_created_for_underline_color`, `extra_created_for_hyperlink`
  - Zerowidth: `push_zerowidth_creates_extra`, `push_zerowidth_multiple_marks`
  - Arc: `clone_shares_arc_refcount`, `push_zerowidth_cow_on_shared_arc`
  - BCE: `from_color_creates_bce_cell`
  - Flags: `cellflags_set_clear_query`, `cellflags_combine`
  - Display: `hyperlink_display`

**Coverage assessment:**
| Input/State | Tested |
|---|---|
| Default cell properties | Yes |
| Reset (with/without extra, template propagation) | Yes |
| is_empty (all 5 negation paths) | Yes |
| Width (normal, wide, spacer, CJK, emoji) | Yes |
| CellExtra lazy allocation | Yes |
| Zerowidth combining marks (single, multiple) | Yes |
| Arc sharing on clone | Yes |
| COW on shared Arc mutation | Yes |
| BCE cell from Color | Yes |
| Compile-time size <= 24 bytes | Yes |
| LEADING_WIDE_CHAR_SPACER width | Not directly (tested via WIDE_CHAR_SPACER which shares the branch) |
| Hyperlink Display | Yes |
| set_underline_color / set_hyperlink | Not directly tested (used via term_handler) |

**Semantic pin:** `size_assertion` uniquely fails if Cell grows beyond 24 bytes. `clone_shares_arc_refcount` uniquely fails if Arc sharing breaks. `push_zerowidth_cow_on_shared_arc` pins COW semantics.
**Hygiene:** Module doc present. All pub items documented. File under 500 lines (250). `#[cfg(test)] mod tests;` at bottom. No unwrap in production code.
**Status: VERIFIED**

---

## 1.4 Row

**Tests found:** `/home/eric/projects/ori_term/oriterm_core/src/grid/row/tests.rs` (274 lines, 21 tests)
**Tests run:** All pass.
**Audit:** READ `row/mod.rs` (194 lines) -- Row struct with `inner: Vec<Cell>` and `occ: usize`. Custom `PartialEq` (compares inner only, ignores occ). Methods: `new()`, `reset()` (with BCE guard), `cols()`, `occ()`, `clear_range()`, `truncate()`, `is_blank()`, `content_len()`, `resize()`, `as_mut_slice()`, `append()` (test-only), `clamp_occ()`, `set_occ()`. Index<Column>, IndexMut<Column> with occ tracking.

**Audit:** READ `row/tests.rs` -- All 21 tests cover:
  - Construction: `new_row_has_correct_length_and_defaults`
  - Occ tracking: `writing_cell_updates_occ`, `index_mut_updates_occ`, `append_empty_cell_does_not_bump_occ`
  - Reset: `reset_clears_and_resets_occ`, `reset_bce_across_consecutive_resets`, `reset_resizes_row_larger`, `reset_shrinks_row`
  - Clear range: basic, full row, with BCE, inverted range, start beyond row, BCE updates occ, BCE survives reset
  - Truncate: from column, at col 0, beyond row, BCE updates occ
  - Equality: `row_equality`
  - Index: `index_returns_correct_cell`

**Coverage assessment:**
| Input/State | Tested |
|---|---|
| New row (length, occ, defaults) | Yes |
| Occ tracking (write, index_mut, empty append) | Yes |
| Reset (basic, resize up/down, BCE) | Yes |
| Clear range (basic, full, BCE, boundary safety) | Yes |
| Truncate (basic, col 0, boundary, BCE) | Yes |
| is_blank | Not directly tested (used via other tests) |
| content_len | Not directly tested |
| resize (standalone, not via reset) | Not directly tested |

**Semantic pin:** `reset_bce_across_consecutive_resets` uniquely pins the BCE guard in reset (bg mismatch forces full-row update). `append_empty_cell_does_not_bump_occ` pins empty-cell optimization.
**Hygiene:** Module doc present. `#[cfg(test)] mod tests;` at bottom. File under 500 lines (194). No unwrap in production code.
**Status: VERIFIED**

---

## 1.5 Grid Foundation

**Tests found:** `/home/eric/projects/ori_term/oriterm_core/src/grid/tests.rs` (81 lines, 9 tests)
**Tests run:** All pass.
**Audit:** READ `grid/mod.rs` (265 lines) -- Grid struct with rows, cols, lines, cursor, saved_cursor, tab_stops, scroll_region, scrollback, display_offset, total_evicted, resize_pushed, dirty. Module declarations for all submodules. Methods: `new()`, `with_scrollback()`, `lines()`, `cols()`, `cursor()`, `cursor_mut()`, `tab_stops()`, `total_lines()`, `display_offset()`, `total_evicted()`, `scrollback()`, `absolute_row()`, `scroll_region()`, `dirty()`, `dirty_mut()`, `scroll_display()`, `reset()`, `init_tab_stops()`, `reset_tab_stops()`, `move_cursor_line()`, `move_cursor_col()`. Index<Line>, IndexMut<Line> with debug_assert.

**Audit:** READ `grid/tests.rs` -- 9 tests:
  - `new_grid_has_correct_dimensions`, `tab_stops_every_8_columns`, `index_by_line_returns_correct_row`, `cursor_starts_at_origin`, `grid_1x1_minimum_dimensions`, `scroll_region_defaults_to_full_grid`, `saved_cursor_starts_as_none`, `tab_stops_for_narrow_grid`, `all_rows_initialized_empty`

**Coverage assessment:**
| Input/State | Tested |
|---|---|
| Dimensions correct | Yes |
| Tab stops (standard, narrow grid) | Yes |
| Index by Line | Yes |
| Cursor starts at origin | Yes |
| Minimum 1x1 grid | Yes |
| Scroll region defaults | Yes |
| Saved cursor None initially | Yes |
| All rows empty | Yes |
| Grid::reset() | Not directly (tested via term tests) |
| Grid::scroll_display() | Tested in ring/tests.rs |

**Semantic pin:** `grid_1x1_minimum_dimensions` pins minimum grid behavior. `tab_stops_for_narrow_grid` pins edge case for <8 cols.
**Hygiene:** Module doc present. All pub items documented. `pub mod snapshot;` is outside the `#[cfg(test)]` block (correct, it's used by test code). File under 500 lines (265).
**Status: VERIFIED**

---

## 1.6 Cursor

**Tests found:** `/home/eric/projects/ori_term/oriterm_core/src/grid/cursor/tests.rs` (61 lines, 6 tests)
**Tests run:** All pass.
**Audit:** READ `cursor/mod.rs` (100 lines) -- CursorShape enum (Block, Underline, Bar, HollowBlock, Hidden) with Default=Block. From<vte::ansi::CursorShape> conversion. Cursor struct with line, col, template. Methods: `new()`, `line()`, `col()`, `template()`, `template_mut()`, `set_line()`, `set_col()`. Default impl delegates to new().

**Audit:** READ `cursor/tests.rs` -- 6 tests:
  - `default_cursor_at_origin`, `set_line_and_col`, `default_shape_is_block`, `template_defaults_to_empty_cell`, `cursor_clone_preserves_all_fields`, `cursor_shape_all_variants_distinct`

**Coverage assessment:**
| Input/State | Tested |
|---|---|
| Default position (0, 0) | Yes |
| Set line/col | Yes |
| Default shape is Block | Yes |
| Template starts empty | Yes |
| Clone preserves fields | Yes |
| All 4 non-Hidden shapes distinct | Yes |
| Hidden variant distinct from others | No (Known Gap -- acknowledged in section) |

**Semantic pin:** `default_shape_is_block` pins the Default impl. `cursor_clone_preserves_all_fields` pins clone correctness.
**Hygiene:** Module doc present. File under 500 lines (100). `#[cfg(test)] mod tests;` at bottom.
**Status: VERIFIED** (with noted gap: CursorShape::Hidden not tested for distinctness)

---

## 1.7 Grid Editing

**Tests found:** `/home/eric/projects/ori_term/oriterm_core/src/grid/editing/tests.rs` (1094 lines, 66 tests)
**Tests run:** All pass.
**Audit:** READ `editing/mod.rs` (504 lines) -- DisplayEraseMode (Below, Above, All, Scrollback), LineEraseMode (Right, Left, All). Methods: `put_char()`, `put_char_ascii()`, `push_zerowidth()`, `insert_blank()`, `delete_chars()`, `erase_display()`, `erase_line()`, `erase_chars()`. Internal: `put_char_slow()`, `erase_line_with_template()`.
**Audit:** READ `editing/wide_char.rs` (69 lines) -- `fix_wide_boundaries()`, `clear_wide_char_at()`.

**Audit:** READ `editing/tests.rs` -- 66 tests covering:
  - Core put_char (3 tests), template inheritance (2), wrap behavior (3), wide char boundary (2)
  - insert_blank (6 tests including BCE, boundary, wide char interactions)
  - delete_chars (6 tests including BCE, boundary, wide char interactions)
  - erase_display (7 tests: Below, Above, All, BCE, boundary cases)
  - erase_line (4 tests: Right, Left, All, BCE)
  - erase_chars (4 tests: basic, past end, BCE, default bg occ)
  - Wide char boundary edge cases from tmux audit (10 tests)
  - Dirty tracking integration (12 tests)
  - Snapshot tests (4 tests using insta)
  - INSERT mode damage (2 tests)

**Coverage assessment:**
| Input/State | Tested |
|---|---|
| put_char: ASCII, wide, wrap, template | Yes |
| put_char_ascii fast path | No direct unit test (Known Gap) |
| push_zerowidth | No direct unit test (Known Gap) |
| insert_blank: basic, at end, overflow, BCE, past end, wide char | Yes |
| delete_chars: basic, at end, overflow, BCE, past end, wide char | Yes |
| erase_display: all 4 modes, BCE, boundaries | Yes |
| erase_line: all 3 modes, BCE | Yes |
| erase_chars: basic, past end, BCE, wide char boundaries | Yes |
| Wide char pair cleanup (overwrite base/spacer) | Yes |
| LEADING_WIDE_CHAR_SPACER on wrap | Yes (snapshot test) |
| Dirty tracking for all operations | Yes |
| Zero-count insert_blank/delete_chars/erase_chars | No (TPR-01-001 -- confirmed bug) |

**Semantic pin:** `put_char_wide_writes_pair` uniquely pins WIDE_CHAR + WIDE_CHAR_SPACER pair. `overwrite_spacer_clears_wide_char` pins wide char cleanup. `wrap_flag_set_on_wrapped_line` pins WRAP flag semantics. Snapshot tests (`snapshot_wide_char_put_and_wrap`) pin LEADING_WIDE_CHAR_SPACER behavior.
**Hygiene:** `editing/mod.rs` is 504 lines -- **4 lines over the 500-line hard limit** (TPR-01-003 confirmed). `wide_char.rs` extraction helped but didn't fully resolve. No unwrap in production code. Module doc present. `#[cfg(test)] mod tests;` at bottom.
**Status: VERIFIED** (with noted issues: TPR-01-001 zero-count damage, TPR-01-003 504-line file)

---

## 1.8 Grid Navigation

**Tests found:** `/home/eric/projects/ori_term/oriterm_core/src/grid/navigation/tests.rs` (677 lines, 57 tests)
**Tests run:** All pass.
**Audit:** READ `navigation/mod.rs` (207 lines) -- TabClearMode (Current, All). Methods: `move_up()`, `move_down()`, `move_forward()`, `move_backward()`, `move_to()`, `move_to_column()`, `move_to_line()`, `carriage_return()`, `backspace()`, `linefeed()`, `reverse_index()`, `next_line()`, `tab()`, `tab_backward()`, `set_tab_stop()`, `clear_tab_stop()`, `save_cursor()`, `restore_cursor()`.

All movement methods properly clamp to bounds. Scroll region awareness in move_up/down (inside region clamps to region bounds, outside clamps to 0/lines-1). Linefeed at bottom of scroll region triggers scroll_up. Reverse index at top triggers scroll_down. Backspace handles wrap-pending (col >= cols). Tab/tab_backward search through tab_stops vector. Save/restore use `saved_cursor: Option<Cursor>`.

**Audit:** READ `navigation/tests.rs` -- Sampled tests cover:
  - Basic movement (up/down/forward/backward with expected deltas)
  - Clamping (all directions, move_to, move_to_column, move_to_line)
  - Scroll region clamping (move_up/down inside/outside region)
  - Linefeed (bottom scroll, middle move, column preservation)
  - Reverse index (top scroll_down, middle move)
  - Next line (CR+LF combined)
  - Backspace (mid-line, col 0, wrap-pending, consecutive)
  - Tab (next stop, last stop, from col 0, wrap-pending, after clear)
  - Tab backward (previous stop, col 0, wrap-pending)
  - Tab stop management (set/clear current/all)
  - Save/restore (round-trip, no save resets to origin, multiple saves overwrite, template preservation)
  - Scroll region content preservation
  - Dirty tracking for all movement operations

**Coverage assessment:**
| Input/State | Tested |
|---|---|
| All CUU/CUD/CUF/CUB movement | Yes |
| All clamping boundaries | Yes |
| Scroll region awareness | Yes |
| LF at bottom (scroll), middle (move), outside (no-op) | Yes |
| RI at top (scroll), middle (move), outside (no-op) | Yes |
| NEL (CR+LF) | Yes |
| BS (normal, col 0, wrap-pending) | Yes |
| HT/CBT (tab stops, boundaries) | Yes |
| Tab stop set/clear (current, all) | Yes |
| DECSC/DECRC (save/restore) | Yes |
| Dirty tracking for all | Yes |

**Semantic pin:** `linefeed_at_bottom_triggers_scroll` pins scroll-on-LF. `reverse_index_at_top_triggers_scroll_down` pins RI. `backspace_from_wrap_pending_snaps_to_last_column` (exists in file) pins wrap-pending handling.
**Hygiene:** File under 500 lines (207). Module doc present. No unwrap.
**Status: VERIFIED**

---

## 1.9 Grid Scrolling

**Tests found:** `/home/eric/projects/ori_term/oriterm_core/src/grid/scroll/tests.rs` (1071 lines, 66 tests)
**Tests run:** All pass.
**Audit:** READ `scroll/mod.rs` (173 lines) -- Methods: `set_scroll_region()` (1-based params, clamped, min 2 lines), `scroll_up()` (with scrollback push for full-screen, display_offset stabilization, row recycling), `scroll_down()`, `insert_lines()` (within scroll region), `delete_lines()` (within scroll region). Internal: `scroll_range_up()` (rotate_left + reset), `scroll_range_down()` (rotate_right + reset).

Key implementation detail: scroll_up pushes evicted rows to scrollback only when scroll region is full screen. Sub-region scrolls lose top rows. Row recycling via `scrollback.push()` returning evicted row. Display offset stabilization on push. Reflow overflow cleanup (`resize_pushed`). O(1) rotation via `rotate_left`/`rotate_right`.

**Coverage assessment:** (from test file names and plan spec)
| Input/State | Tested |
|---|---|
| set_scroll_region: full screen, sub-region, default bottom, invalid, oversized | Yes |
| scroll_up: 1 line, N lines, sub-region, count overflow, BCE | Yes |
| scroll_down: 1 line, sub-region, count overflow, BCE | Yes |
| insert_lines: mid-region, outside (no-op), count cap, BCE | Yes |
| delete_lines: mid-region, outside (no-op), count cap, BCE | Yes |
| Display offset stabilization | Yes |
| Row recycling | Yes |
| Dirty tracking for all | Yes |
| Sub-region does not push to scrollback | Yes |

**Semantic pin:** `scroll_up_pushes_to_scrollback` (in ring/tests.rs) pins scrollback integration. `set_scroll_region` tests pin 1-based parameter conversion. Display offset stabilization tests pin the user scrollback view stability.
**Hygiene:** File under 500 lines (173). Module doc present. No unwrap.
**Status: VERIFIED**

---

## 1.10 Scrollback Ring Buffer

**Tests found:** `/home/eric/projects/ori_term/oriterm_core/src/grid/ring/tests.rs` (765 lines, 38 tests)
**Tests run:** All pass.
**Audit:** READ `ring/mod.rs` (159 lines) -- ScrollbackBuffer struct with inner (Vec<Row>), max_scrollback, len, start. Methods: `new()`, `push()` (returns evicted row), `len()`, `is_empty()`, `max_scrollback()`, `get()` (0=newest), `iter()` (newest to oldest), `pop_newest()`, `drain_oldest_first()`, `clear()`. Internal: `physical_index()`.

Push behavior: max_scrollback==0 returns immediately. Growth phase: extends vector or reuses placeholder slots from pop_newest. Full: swaps at start, advances start. Pop_newest: decrements len, replaces with placeholder. Drain: builds Vec oldest-first, clears buffer.

**Coverage assessment:** (from plan spec)
| Input/State | Tested |
|---|---|
| Core ring: empty, push/retrieve order, wrap/eviction | Yes |
| Clear: empties, usable after | Yes |
| Iterator: newest to oldest, after wrap, after pop/push | Yes |
| Edge: zero max, max=1, exact capacity boundary | Yes |
| Push return value: None during growth, evicted when full | Yes |
| pop_newest: empty, until empty, after wrap, growth phase | Yes |
| drain_oldest_first: empty, growth, wrapped, exactly full | Yes |
| Wide char preservation in scrollback | Yes |
| Grid integration: scroll_up pushes, sub-region doesn't | Yes |
| Display offset: scroll through history, clamping, total_lines | Yes |
| Resize interaction: no placeholder leak, no duplication | Yes |

**Semantic pin:** `ring_wraps_evicts_oldest` pins ring buffer eviction. `zero_max_scrollback_returns_pushed_row` pins zero-capacity behavior. `wide_char_flags_preserved_in_scrollback` pins wide char fidelity through scrollback.
**Hygiene:** File under 500 lines (159). Module doc present. No unwrap. `push()`, `pop_newest()`, `drain_oldest_first()` are `pub(super)` -- correct visibility.
**Status: VERIFIED**

---

## 1.11 Dirty Tracking

**Tests found:** `/home/eric/projects/ori_term/oriterm_core/src/grid/dirty/tests.rs` (313 lines, 26 tests)
**Tests run:** All pass.
**Audit:** READ `dirty/mod.rs` (246 lines) -- LineDamageBounds (dirty, left, right), DirtyLine (line, left, right), DirtyTracker (lines, cols, all_dirty). Methods: `new()`, `mark()`, `mark_cols()`, `mark_range()`, `mark_all()`, `is_all_dirty()`, `is_dirty()`, `is_any_dirty()`, `col_bounds()`, `drain()`, `resize()`. DirtyIter with Drop impl that clears remaining entries.

**Coverage assessment:**
| Input/State | Tested |
|---|---|
| New tracker is clean | Yes |
| Mark single line (full width bounds) | Yes |
| Mark all | Yes |
| Drain: returns dirty, resets to clean, mark_all yields all | Yes |
| Drain drop clears remaining | Yes |
| Mark range: target only, empty, full (sets all_dirty), partial | Yes |
| Mark out of bounds is safe | Yes |
| Resize marks all dirty | Yes |
| Column-level: single char, expand range, erase range | Yes |
| Full line then mark_cols union | Yes |
| col_bounds: clean=None, marked=range, all_dirty=full | Yes |
| mark_cols out of bounds safe | Yes |
| all_dirty yields full line bounds for unmarked lines | Yes |

**Semantic pin:** `drain_drop_clears_remaining` pins the Drop impl (partial iteration safety). `mark_cols_expands_range` pins union semantics. `all_dirty_yields_full_line_bounds_for_unmarked_lines` pins the all_dirty fast-path behavior.
**Hygiene:** File under 500 lines (246). Module doc present. No unwrap.
**Status: VERIFIED**

---

## 1.R Third Party Review Findings

**TPR-01-001 (zero-count damage):** CONFIRMED OPEN.
  - Evidence: Read `editing/mod.rs`. For `insert_blank(0)`: count clamps to 0, but `clear_wide_char_at()` runs, swap loop runs (self-swaps), and `dirty.mark_cols(line, col, cols-1)` still executes. For `delete_chars(0)`: same pattern. For `erase_chars(0)`: `end = col`, empty loop body, but `dirty.mark_cols(line, col, col.saturating_sub(1))` still fires.
  - No tests for zero-count cases exist. This is a real false-positive damage bug.

**TPR-01-002 (rustdoc warning):** CONFIRMED OPEN.
  - Evidence: Running `cargo doc -p oriterm_core --no-deps` produces: `warning: unresolved link to 'Term::renderable_content_into'`. The warning originates from `oriterm_core/src/term/renderable/mod.rs:188`, not from Section 01 code directly. However, the section completion checklist item `cargo doc -p oriterm_core --no-deps -- generates clean docs` is marked `[ ]` (unchecked), which is accurate.

**TPR-01-003 (500-line limit):** CONFIRMED OPEN.
  - Evidence: `wc -l editing/mod.rs` = 504 lines. The `.claude/rules/code-hygiene.md` hard limit is 500. The `wide_char.rs` extraction moved 69 lines out but was insufficient. 4 lines over.

**Status: NOT STARTED** (as marked in plan -- these findings are unresolved)

---

## 1.12 Section Completion

**Checklist verification:**
- [x] All 1.1-1.11 items complete -- **Confirmed**: all items checked in plan, all tests pass.
- [x] `cargo test -p oriterm_core` all pass -- **Confirmed**: 1429 pass, 0 fail.
- [x] `cargo clippy` no warnings -- Not re-verified here (plan says checked).
- [ ] `cargo doc` clean docs -- **Confirmed OPEN**: rustdoc warning present (TPR-01-002).
- [x] Grid operations functional -- **Confirmed**: create, write, wide chars, cursor, scroll, erase, tab stops, scrollback, dirty.
- [x] No VTE, events, palette, selection, rendering in scope -- **Confirmed**: lib.rs shows these exist in later modules, not in grid/.

**Test Coverage Summary verification:**
Plan claims ~244 tests across 4720 lines. Actual count: **331 tests** across **4720 lines**. The plan's test count is outdated (understated by ~87). Line count matches exactly.

**Known Test Gaps (from plan, verified):**
1. `put_char_ascii()` -- No dedicated unit test. Confirmed: no test calls this method directly. Tested indirectly via TermHandler.
2. `push_zerowidth()` (Grid method) -- No dedicated unit test. Confirmed: Cell::push_zerowidth is tested, but Grid::push_zerowidth is not directly tested.
3. `CursorShape::Hidden` -- No dedicated assertion for distinctness from other variants. Confirmed: the `cursor_shape_all_variants_distinct` test explicitly excludes Hidden.

**Status: IN-PROGRESS** (as marked -- TPR findings unresolved, doc warning open)

---

## Gap Analysis

### Section Goal Fulfillment
> "Build the core data structures -- Cell, Row, Grid -- in oriterm_core with full test coverage"

**Assessment:** The goal is substantially fulfilled. Cell, Row, and Grid are fully implemented with comprehensive test suites covering normal operation, edge cases, BCE, wide chars, dirty tracking, scrollback, and navigation. The 331 tests across 4720 lines represent thorough coverage.

### Identified Gaps

1. **TPR-01-001: Zero-count edit damage (medium).** `insert_blank(0)`, `delete_chars(0)`, and `erase_chars(0)` produce false-positive dirty marks despite performing no visible mutation. No tests cover this. Fix: add early returns for count==0 and corresponding tests.

2. **TPR-01-002: Rustdoc warning (medium).** Unresolved link to `Term::renderable_content_into` blocks the doc-clean checklist item. The warning is in `term/renderable/mod.rs`, not Section 01 code, but the section's completion criteria include crate-wide doc cleanliness.

3. **TPR-01-003: editing/mod.rs at 504 lines (low).** 4 lines over the 500-line hard limit. A further extraction (e.g., moving erase operations to `editing/erase.rs`) would resolve this.

4. **Test count discrepancy.** The section's table claims ~244 tests but actual count is 331. The table should be updated to reflect reality.

5. **Missing direct tests for `put_char_ascii` and `push_zerowidth`.** These are acknowledged gaps. They have indirect coverage through integration tests, but direct unit tests would improve semantic pinning.

6. **Row::is_blank() and Row::content_len() lack direct unit tests.** These are used by other code but have no dedicated test verifying their behavior in isolation.

7. **Grid::reset() lacks a direct unit test.** This method clears all state (rows, cursor, tab stops, scroll region, scrollback) but is only tested indirectly via term tests.

### What Is NOT Missing

- Wide char handling is exceptionally thorough (10 dedicated boundary tests plus integration).
- BCE (Background Color Erase) is tested across all editing, scrolling, and erase operations.
- Dirty tracking integration is verified for every mutation operation.
- Scrollback ring buffer has excellent edge case coverage (zero capacity, max=1, exact boundary, pop/push cycles, drain).
- No TODOs, FIXMEs, HACKs, or #[ignore] in any Section 01 code.
- No `unwrap()` in any production code (only in test files).
- All test files follow the sibling `tests.rs` pattern with `super::` imports.
- All source files have `//!` module docs and `///` on public items.
- No dead code, no commented-out code, no println debugging.

### Conclusion

Section 01 is in solid shape. The three TPR findings are real but bounded: two are medium-severity code issues (zero-count damage, 504-line file) and one is a doc-link problem outside Section 01's owned code. The test suite is comprehensive with 331 tests pinning all major behaviors. The section can be closed once the three TPR items are resolved.
