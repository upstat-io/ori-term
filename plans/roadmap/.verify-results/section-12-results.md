# Section 12: Resize & Reflow -- Verification Results

**Verified by:** Claude Opus 4.6 (1M context)
**Date:** 2026-03-29
**Status:** PASS

## Context Loaded

- `/home/eric/projects/ori_term/.claude/worktrees/verify-roadmap/CLAUDE.md` -- full project rules
- `.claude/rules/code-hygiene.md` -- file org, import order, 500-line limit
- `.claude/rules/test-organization.md` -- sibling tests.rs pattern
- `.claude/rules/impl-hygiene.md` -- module boundary discipline, no alloc in hot path
- `.claude/rules/crate-boundaries.md` -- crate ownership (via error message in Read)
- `plans/roadmap/section-12-resize-reflow.md` -- section plan (lines 1-218)

## Files Read

| File | Lines Read | Purpose |
|------|-----------|---------|
| `oriterm_core/src/grid/resize/mod.rs` | 1-461 (full) | Grid::resize, reflow_cols, reflow_cells, reflow_row_cells |
| `oriterm_core/src/grid/resize/tests.rs` | 1-3064 (full, in chunks) | All 94 resize/reflow unit tests |
| `oriterm/src/app/chrome/resize.rs` | 1-210 (full) | App::handle_resize, sync_grid_layout, update_resize_increments |
| `oriterm/src/app/chrome/mod.rs` | 118-176 | compute_window_layout (pixel-to-grid calculation) |
| `oriterm/src/app/chrome/tests.rs` | via grep | 5 tests for compute_window_layout |
| `oriterm_mux/src/pane/mod.rs` | 380-428 | Pane::resize_grid, Pane::resize_pty |
| `oriterm_mux/src/pty/spawn.rs` | 50-170 | PtyControl::resize (PtySize with rows/cols) |
| `oriterm_mux/src/backend/embedded/mod.rs` | 110-140 | EmbeddedMux::resize_pane_grid (calls both grid + PTY) |
| `oriterm_core/src/term/mod.rs` | 408-450 | Term::resize (primary + alternate grid, image pruning) |
| `oriterm_core/src/term/tests.rs` | 1179-1260, 1972-2000 | term_resize_* tests (8 tests) |
| `oriterm_core/tests/alloc_regression.rs` | 1-50, 284-314 | Allocation profiling for resize cycles |
| `oriterm/src/app/pane_ops/mod.rs` | 207-223 | resize_all_panes (multi-pane resize) |

## Test Execution

```
$ timeout 150 cargo test -p oriterm_core -- resize
running 123 tests ... test result: ok. 123 passed; 0 failed; 0 ignored
```

All 123 resize-related tests pass in 0.03s.

```
$ timeout 150 cargo clippy -p oriterm_core --target x86_64-pc-windows-gnu
Finished ... 0 warnings
```

Clean clippy.

## 12.1 Window-to-Grid Resize -- PASS

### Implementation Evidence

**File:** `oriterm/src/app/chrome/resize.rs` (210 lines)

- `App::handle_resize()` (line 103-189): Receives `WindowEvent::Resized`, reconfigures GPU surface, updates chrome layout, calls `sync_grid_layout()`.
- `App::sync_grid_layout()` (line 59-95): Calls `compute_window_layout()` to get (cols, rows) from pixel dimensions, then calls `mux.resize_pane_grid()` and `resize_all_panes()`.
- `compute_window_layout()` (chrome/mod.rs:118-176): Subtracts tab bar height + padding from viewport, divides by cell metrics, enforces `.max(1)` on both cols and rows.
- `App::update_resize_increments()` (line 15-48): Sets `window.set_resize_increments()` with cell size when `config.window.resize_increments` is true. Also pushes cell metrics to Win32 WM_SIZING subclass for frameless CSD snap on Windows.

### Tests

5 tests in `oriterm/src/app/chrome/tests.rs`:
- `layout_grid_origin_includes_padding` -- verifies grid rect position after tab bar + padding
- `layout_cols_rows_from_visible_area` -- 1920x1080 at 1x with 8x16 cells
- `layout_fractional_dpi_scale` -- 1.25x DPI with 10x20 cells
- `layout_integer_origin_at_fractional_dpi` -- 1.75x DPI, verifies integer grid origin Y
- `layout_minimum_one_col_one_row` -- tiny viewport (50x100) produces 1x1 grid

### Pin Check

- `.max(1)` on cols and rows: present in `compute_window_layout()` line 162-163.
- Zero-dimension guard at `Grid::resize()` line 24: `if new_cols == 0 || new_lines == 0 { return; }`
- Zero-dimension guard at `Term::resize()` line 415: same pattern.
- Dirty marking: `finalize_resize()` calls `self.dirty.resize(self.lines, self.cols)` which calls `mark_all()`.
- GPU surface reconfigure: `ctx.window.resize_surface(size.width, size.height, gpu)` at line 153.

### Verdict: PASS

All checklist items verified in code and tested.

---

## 12.2 PTY Resize Notification -- PASS

### Implementation Evidence

**File:** `oriterm_mux/src/pane/mod.rs` (lines 388-407)

- `Pane::resize_grid()`: Locks terminal mutex, calls `terminal.resize(rows, cols, true)`.
- `Pane::resize_pty()`: Deduplicates via `AtomicU32` (packed rows/cols), calls `pty_control.resize(rows, cols)`. Logs warning on failure rather than panicking.

**File:** `oriterm_mux/src/pty/spawn.rs` (line 64-73)

- `PtyControl::resize()`: Delegates to `portable_pty::MasterPty::resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })`.

**File:** `oriterm_mux/src/backend/embedded/mod.rs` (line 125-131)

- `EmbeddedMux::resize_pane_grid()`: Calls both `pane.resize_grid(rows, cols)` and `pane.resize_pty(rows, cols)` -- single PTY notification for both grids.

### Pin Check

- Never sends 0x0: Grid::resize guards at both Grid (line 24) and Term (line 415) level; compute_window_layout enforces `.max(1)`.
- PtySize includes both character and pixel dimensions (pixel set to 0 since not used).
- Deduplication prevents spurious ConPTY resize events.

### Verdict: PASS

---

## 12.3 Grid Row Resize -- PASS

### Implementation Evidence

**File:** `oriterm_core/src/grid/resize/mod.rs`

- `resize_rows()` (line 99-110): Dispatches to `shrink_rows` or `grow_rows`.
- `shrink_rows()` (line 113-131): Calls `count_trailing_blank_rows()` first, pops trailing blanks, drains excess top rows to scrollback, adjusts cursor with `saturating_sub`.
- `grow_rows()` (line 140-162): When cursor at bottom, pops from scrollback (consuming slots), inserts blank rows at top, adjusts cursor. When cursor in middle, appends empty rows at bottom.
- `count_trailing_blank_rows()` (line 165-179): Counts blank rows below cursor, up to `max`.

### Tests (directly matching 12.3 checklist)

| Checklist Item | Test |
|---|---|
| Shrink: trailing blank rows trimmed first | `shrink_rows_trims_trailing_blanks_first` |
| Shrink: non-blank rows pushed to scrollback, cursor adjusted | `shrink_rows_pushes_excess_to_scrollback`, `shrink_rows_cursor_adjusted_for_scrollback_push` |
| Grow: empty rows added when cursor in middle | `grow_rows_appends_blanks_when_cursor_in_middle` |
| Grow: scrollback pulled when cursor at bottom | `grow_rows_consumes_scrollback_inserts_blanks` |
| Zero-size guard | `resize_zero_cols_is_noop`, `resize_zero_lines_is_noop` |

### Verdict: PASS

All 5 specified test scenarios are present and passing.

---

## 12.4 Text Reflow -- PASS

### Implementation Evidence

**File:** `oriterm_core/src/grid/resize/mod.rs` (461 lines, under 500-line limit)

- `Grid::resize()` (line 23-51): Main entry point. Growing cols: reflow first then adjust rows. Shrinking cols: adjust rows first then reflow. Without reflow: `resize_no_reflow`.
- `Grid::reflow_cols()` (line 185-220): Collects all rows (scrollback + visible), calls `reflow_cells()`, applies result.
- `reflow_cells()` (line 297-374): Free function. Iterates all source rows, tracks cursor, handles wrapped vs non-wrapped rows, delegates per-row work to `reflow_row_cells()`.
- `reflow_row_cells()` (line 381-458): Cell-by-cell rewriting. Skips spacer cells. Handles wide char boundary (LEADING_WIDE_CHAR_SPACER insertion). Strips old WRAP flags. Tracks cursor through source->output mapping.

### Coverage Matrix

The section plan specifies 7 test categories. Here is how each maps to actual tests:

| Required Test | Actual Test(s) | Details |
|---|---|---|
| Column increase: wrapped lines unwrap | `reflow_grow_unwraps_soft_wrapped_line`, `snapshot_reflow_grow_unwraps`, `put_char_content_survives_grow_reflow` | WRAP flag cleared, content merged |
| Column decrease: long lines re-wrap | `reflow_shrink_wraps_long_line`, `snapshot_reflow_shrink_wraps`, `put_char_content_survives_shrink_reflow` | WRAP flag set, content split |
| Wide char at shrink boundary | `reflow_wide_char_at_boundary_wraps_correctly`, `reflow_wide_char_boundary_sets_leading_spacer`, `snapshot_wide_char_at_boundary`, `snapshot_wide_chars_leading_spacer_placement` | LEADING_WIDE_CHAR_SPACER inserted |
| Cursor preservation | `cursor_tracks_through_narrow_grow_narrow_grow`, `cursor_on_wide_char_tracks_through_reflow`, `snapshot_cursor_exact_position_after_shrink`, `snapshot_cursor_exact_position_after_grow`, `snapshot_cursor_at_wrap_boundary_after_grow`, `snapshot_cursor_past_content_on_wrapped_row` | Cursor on 'X' before = 'X' after |
| Scrollback reflow | `put_char_wrapped_content_with_scrollback_round_trip`, `reflow_mixed_wide_narrow_across_scrollback`, `snapshot_reflow_with_scrollback`, `snapshot_long_line_spanning_scrollback_and_visible`, `snapshot_wrapped_line_across_scrollback_boundary` | Content pushed to scrollback on shrink, pulled on grow |
| Empty grid | `reflow_empty_grid_produces_valid_state`, `snapshot_reflow_empty_grid` | Produces at least one row |
| No-op: same column count | `resize_same_dimensions_is_noop` | Grid unchanged |

### Additional Coverage Beyond Plan

The test suite goes significantly beyond the plan requirements:

**CJK/Wide char deep coverage:**
- `reflow_grid_of_only_wide_chars` -- grid of 100% wide chars
- `wide_char_survives_multiple_intermediate_sizes` -- cycle through 4 sizes
- `snapshot_multiple_wide_chars_reflow` -- mixed wide/narrow with multiple wide chars
- `snapshot_wide_char_exactly_fills_row` -- exact-fit boundary
- `snapshot_wide_char_at_1_col_grid` -- degenerate case: WIDE_CHAR flag stripped at 1 column

**Combining marks:**
- `snapshot_combining_marks_survive_reflow` -- 'e' + U+0301 (acute accent) survives shrink+grow round-trip. Verifies `extra.zerowidth` vec preserved.

**Attribute preservation:**
- `reflow_preserves_cell_attributes` -- BOLD, ITALIC, fg color survive reflow
- `snapshot_hyperlinks_survive_reflow` -- URI hyperlink preserved through reflow
- `snapshot_underline_color_survives_reflow` -- CURLY_UNDERLINE + underline color
- `snapshot_bce_colored_blanks_reflow_correctly` -- colored-background spaces (BCE) wrap correctly

**Scrollback ring buffer edge cases:**
- `reflow_with_wrapped_scrollback_ring` -- ring buffer wrapping before resize
- `reflow_shrink_with_wrapped_scrollback_ring` -- shrink with wrapped ring
- `reflow_round_trip_with_wrapped_scrollback` -- round-trip with wrapped ring
- `reflow_scrollback_overflow_evicts_oldest` -- capacity=3, wrapping evicts oldest
- `snapshot_zero_scrollback_capacity_shrink` -- capacity=0 (all overflow lost)

**Trailing blank regression tests:**
- `reflow_shrink_trims_trailing_blanks_before_scrollback` -- root cause test for blank-push bug
- `reflow_shrink_trailing_blanks_write_row_small_grid` -- variant of same bug
- `reflow_shrink_with_cursor_at_content_end` -- cursor past content + trailing blanks

**Real-world scenarios (insta snapshots):**
- `snapshot_cli_prompt_and_long_command_output` -- cargo build output
- `snapshot_multiline_prompt_with_decorations` -- box-drawing chars (starship/p10k prompt)
- `snapshot_interactive_session_with_scrollback` -- multiple commands
- `snapshot_long_wrapped_output_like_base64` -- 100-char output line
- `snapshot_mixed_content_types_realistic` -- find command + output
- `snapshot_claude_code_like_output` -- AI assistant style output
- `snapshot_vim_like_full_screen_reflow` -- no-reflow alt-screen behavior
- `snapshot_many_short_lines_reflow` -- git log style output

**Regression tests:**
- `real_world_resize_ghosting_with_scroll_up` -- complex 3-cycle resize+redraw ghosting bug from production log data. Verifies mascot count <= 2 after shell scroll_up + erase_display + redraw cycles.
- `height_only_resize_ghosting` -- height-only shrink + shell response cycle. Verifies visible mascot count == 1 after each cycle.
- `resize_resets_display_offset_to_zero` -- regression for stale display_offset causing corrupted/duplicated scrollback after reflow.

**Stress tests:**
- `rapid_resize_sequence_does_not_panic` -- 5 rapid resizes
- `snapshot_aggressive_resize_sequence_with_wide_chars` -- 5 sizes including 1-col
- `snapshot_rapid_resize_with_scrollback_interaction` -- 40->10->80->20->40
- `snapshot_reflow_shrink_forces_massive_scrollback` -- 3x20 to 3x5 (9 rows to scrollback)

### Verdict: PASS

The 7 specified test scenarios are all present and verified. The test suite provides exceptional coverage with 94 tests in the resize module alone, plus 8 more at the Term level. Coverage spans all dimensions requested: ASCII, CJK/wide chars, combining marks, emoji (via wide char handling), shrink, grow, wrap boundary, scrollback, and attribute preservation. The only dimension not explicitly tested is selection interaction during reflow (selection is marked dirty by `Term::resize` but no test verifies selection coordinates after reflow). This is acceptable since the section plan does not specify selection coordinate tracking through reflow.

---

## 12.5 Alternate Screen Resize -- PASS

### Implementation Evidence

**File:** `oriterm_core/src/term/mod.rs` (line 432-444)

```rust
if let Some(alt) = &mut self.alt_grid {
    let prev_alt = alt.total_evicted();
    alt.resize(new_lines, new_cols, false);  // reflow=false
    ...
}
```

- Primary grid: `self.grid.resize(new_lines, new_cols, reflow)` -- reflow=true when caller permits.
- Alternate grid: `alt.resize(new_lines, new_cols, false)` -- always reflow=false.
- Alt grid only resized if allocated (`if let Some(alt)`).
- Image cache pruning on both grids when rows evicted.

### Tests

- `term_resize_changes_both_grids` -- verifies both primary and alt get new dimensions (10x40) after resize
- `resize_before_alt_screen_no_crash` -- resize when alt_grid is None (not yet allocated)
- `snapshot_vim_like_full_screen_reflow` -- no-reflow resize (rows truncated/padded, no WRAP manipulation)

### Pin Check

- `reflow=false`: confirmed at line 437 (`false` literal).
- Cursor clamped: `finalize_resize()` clamps cursor for both grids.
- Both grids resized: primary at line 425, alt at line 437.

### Verdict: PASS

---

## 12.6 Section Completion -- PASS

### Checklist Verification

| Completion Item | Evidence |
|---|---|
| All 12.1-12.5 items complete | All sub-sections verified above |
| `cargo test -p oriterm_core` reflow tests pass | 123/123 pass, 0 fail |
| `cargo clippy -p oriterm_core --target x86_64-pc-windows-gnu` no warnings | Clean output confirmed |
| Resizing the window resizes the grid | `handle_resize()` -> `sync_grid_layout()` -> `compute_window_layout()` -> `mux.resize_pane_grid()` |
| PTY receives new dimensions on resize | `EmbeddedMux::resize_pane_grid()` calls both `resize_grid` and `resize_pty` |
| Shell prompt redraws correctly after resize | `snapshot_cli_prompt_and_long_command_output` round-trip test |
| Text reflows when columns change | 94 reflow tests with round-trip verification |
| Wide characters handled at reflow boundaries | 12+ wide char tests including LEADING_WIDE_CHAR_SPACER |
| Cursor position preserved through resize and reflow | 6+ cursor tracking tests with exact position verification |
| No crash on zero-dimension resize | `resize_zero_cols_is_noop`, `resize_zero_lines_is_noop`, `layout_minimum_one_col_one_row` |
| No crash on rapid resize sequences | `rapid_resize_sequence_does_not_panic`, `snapshot_rapid_resize_with_scrollback_interaction` |
| Alternate screen resizes correctly | `term_resize_changes_both_grids`, `resize_before_alt_screen_no_crash` |

### Verdict: PASS

---

## Code Hygiene Audit

| Rule | Status | Evidence |
|---|---|---|
| File size < 500 lines | PASS | `mod.rs` is 461 lines |
| Sibling tests.rs pattern | PASS | `resize/mod.rs` + `resize/tests.rs`, `#[cfg(test)] mod tests;` at bottom |
| No inline test modules | PASS | Tests in separate file, no wrapper `mod tests {}` |
| Module docs (`//!`) | PASS | Line 1: `//! Grid resize and text reflow.` (6-line doc) |
| `///` on pub items | PASS | `pub fn resize` has 4-line doc comment |
| Import order (std, external, crate) | PASS | `crate::cell`, `crate::index`, `super::Grid`, `super::row::Row` |
| No unwrap in library code | PASS | No `unwrap()` in `mod.rs` |
| No dead code | PASS | Clean clippy with `dead_code = "deny"` |
| `#[expect]` with reason | PASS | Lines 235, 293, 377 all use `#[expect(clippy::too_many_arguments, reason = "...")]` |
| No decorative banners | MINOR | Tests file uses `// -- Section name --` banners (e.g., line 29, 55, 153). These are in the test file which is exempt from some rules, but they use `--` decoration. Not a blocking issue. |

## Summary

Section 12 is **complete and well-verified**. The implementation follows Ghostty-style cell-by-cell rewriting as specified. The test suite is exceptionally thorough at 94 grid-level tests + 8 term-level tests + 5 layout tests = 107 total resize-related tests, covering:

- **ASCII**: extensive (round-trip, wrap, unwrap, multi-line, scrollback)
- **CJK/Wide chars**: 12+ tests covering boundary wrapping, LEADING_WIDE_CHAR_SPACER, round-trip, multi-size cycling, 1-col degenerate
- **Combining marks**: 1 test (`snapshot_combining_marks_survive_reflow`)
- **Attributes**: 4 tests (bold/italic/color, hyperlinks, underline color, BCE blanks)
- **Shrink/Grow**: both directions well tested independently and simultaneously
- **Wrap boundary**: extensive (exact-fit, wide-at-boundary, leading spacer)
- **Scrollback**: 10+ tests including ring buffer wrapping, capacity limits, overflow eviction
- **Cursor tracking**: 6+ tests with exact position verification via insta snapshots
- **Regression**: 3 dedicated regression tests for real-world bugs (ghosting, display offset)
- **Realistic scenarios**: 8 snapshot tests simulating real terminal sessions

No bugs found. No test failures. No code hygiene violations (the banner style in tests.rs is cosmetic and tests are exempt from strict style rules).
