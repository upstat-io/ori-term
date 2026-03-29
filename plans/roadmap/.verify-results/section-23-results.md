# Section 23: Performance & Damage Tracking - Verification Results

**Verified by:** Claude Opus 4.6 (1M context)
**Date:** 2026-03-29
**Branch:** dev
**Status:** in-progress (substantial work complete, benchmarks and polish remaining)

## Context Loaded

- `CLAUDE.md` (full project instructions, performance invariants, coding standards)
- `.claude/rules/code-hygiene.md` (file size limits, import organization, naming)
- `.claude/rules/impl-hygiene.md` (module boundaries, data flow, rendering discipline)
- `.claude/rules/test-organization.md` (sibling tests.rs pattern)
- `plans/roadmap/section-23-performance.md` (full plan, 627 lines)

---

## 23.1 Damage Tracking

### Per-Row Dirty Flag: VERIFIED

**Source:** `oriterm_core/src/grid/dirty/mod.rs` (246 lines, under 500-line limit)

Implementation is solid. `DirtyTracker` uses `Vec<LineDamageBounds>` with per-line `dirty` bool and inclusive `(left, right)` column bounds. Key APIs:
- `mark(line)` -- full-line dirty (sets `left=0, right=cols-1`)
- `mark_cols(line, left, right)` -- column-level via `LineDamageBounds::expand()`
- `mark_range(Range)` -- marks contiguous range; delegates to `mark_all()` when range covers all lines
- `mark_all()` -- sets `all_dirty` shortcut flag (O(1) instead of O(n))
- `drain()` -- yields `DirtyLine { line, left, right }` and resets via `DirtyIter`. `Drop` impl clears un-iterated entries

**Tests:** `oriterm_core/src/grid/dirty/tests.rs` -- 26 tests, all passing. Coverage:
- Clean state on creation (`new_tracker_is_clean`)
- Single-line mark (`mark_single_line`)
- Full-line bounds via `mark()` (`mark_reports_full_line_bounds`)
- `mark_all` flag behavior (`mark_all_makes_everything_dirty`)
- Drain yields correct lines and resets (`drain_returns_dirty_lines`, `drain_resets_to_clean`, `drain_mark_all_yields_every_line`)
- Resize marks all dirty (`resize_marks_all_dirty`)
- Partial iteration + drop clears remaining (`drain_drop_clears_remaining`)
- Range marking: partial, full, superset, empty (`mark_range_*` 6 tests)
- Out-of-bounds safety (`mark_out_of_bounds_is_safe`, `mark_cols_out_of_bounds_is_safe`)
- Column-level: single char, expanding range, erase range, full-line, col_bounds API, all_dirty yields full bounds (8 tests)

**Editing integration tests:** `oriterm_core/src/grid/editing/tests.rs` -- 23 relevant tests passing. Covers `put_char`, `insert_blank`, `delete_chars`, `erase_chars`, `erase_line` (below/above/all), `erase_display` (below/above/all), line wrap dirty marking.

### Column-Level Damage Bounds: VERIFIED

`LineDamageBounds` struct has `left/right` fields. `mark_cols()` calls `expand(left, right)`. `col_bounds()` returns `Option<(usize, usize)>`. `collect_damage()` in snapshot layer propagates column bounds into `DamageLine { line, left: Column(left), right: Column(right) }`. All grid operations (`put_char`, `erase_line`, `erase_chars`, `insert_blank`, `delete_chars`) use `mark_cols()` for column-level precision. Full-line operations (`scroll_up`, `scroll_down`, cursor move) use `mark()`.

Tests: `mark_cols_single_char`, `mark_cols_expands_range`, `mark_cols_erase_range`, `mark_full_line_reports_full_width`, `mark_cols_then_mark_full_expands_to_full`, `col_bounds_*` tests -- all verified in dirty/tests.rs.

### Instance Buffer Caching (PaneRenderCache): VERIFIED

**Source:** `oriterm/src/gpu/pane_cache/mod.rs` (132 lines)

`PaneRenderCache` stores `HashMap<PaneId, CachedPaneFrame>` where each entry has a `PreparedFrame` and the `PaneLayout` at cache time. `get_or_prepare()` checks `dirty` flag and layout match; cache hit returns existing frame, miss calls `prepare_fn`. `invalidate_all()` clears all entries. `remove()` removes single pane. `retain_only()` batch-prunes stale entries.

**Hygiene note:** `retain_only()` and `invalidate()` both have `#[allow(dead_code)]` with reason strings. `invalidate()` reason says "used for targeted invalidation" but it IS used in tests -- acceptable. `retain_only()` says "batch prune API -- call site wired when needed" -- genuinely dead code, acceptable with the reason.

**Tests:** `oriterm/src/gpu/pane_cache/tests.rs` -- 16 tests, all passing. Coverage: clean pane cache hit, dirty re-prepare, layout change invalidation, invalidate_all, remove, extend_from merge, position change, selective dirty, is_cached/get_cached API, single-pane invalidate, retain_only.

### Row-Level Dirty Skip (Incremental Prepare): VERIFIED

**Source:** `oriterm/src/gpu/prepare/dirty_skip/mod.rs` (465 lines, under 500-line limit)

`fill_frame_incremental()` implements the incremental path. For each row transition, checks `scratch_dirty[row]`. Clean rows copy cached instances from `saved_tier` using `RowInstanceRanges`. Dirty rows are processed normally (color resolution, shaping, decoration). `build_dirty_set()` combines content damage (`input.content.damage`), cursor visibility, and selection damage. `mark_selection_damage()` computes symmetric difference of old/new selection ranges.

`prepare_frame_shaped_into()` (in `mod.rs:208-242`) decides between incremental and full rebuild: `!input.content.all_dirty && out.saved_tier.has_cached_rows()`. The `needs_full_repaint()` function exists on `FrameInput` but remains `#[allow(dead_code)]` -- the incremental path reads `content.all_dirty` directly instead.

Key data structures: `RowInstanceRanges` (byte ranges per pipeline buffer), `SavedTerminalTier` (previous frame's buffer data), `BufferLengths` (snapshot for range computation).

**Tests:** `oriterm/src/gpu/prepare/dirty_skip/tests.rs` -- 12 tests, all passing:
- `all_dirty_marks_every_row`, `damage_marks_specific_rows`
- `cursor_row_always_dirty`, `invisible_cursor_not_dirty`
- `buffer_lengths_range_since`, `empty_row_range_is_default`
- Selection damage: new, clear, extend, shrink, same (no-op), integrated with build_dirty_set, clamped to viewport (7 tests)

**Integration tests:** `oriterm/src/gpu/prepare/tests.rs` -- `incremental_all_dirty_matches_full_rebuild` and `incremental_no_dirty_rows_matches_cached` verify byte-equality between incremental and full rebuild paths.

### Selection Damage Tracking: VERIFIED

`mark_selection_damage()` in `dirty_skip/mod.rs:155-210` handles all four cases: None->None, Some->None (clear), None->Some (new), Some->Some (symmetric diff + boundary lines). `build_dirty_set()` calls it with `prev_selection` from `PreparedFrame`. `PreparedFrame::prev_selection_range` is updated after both full and incremental paths (line 237-241).

Tests cover all scenarios: new selection, clear, extend, shrink, same (no-op), integration with content damage, viewport clamping.

### Insert Mode Damage: VERIFIED

`insert_blank()` marks `[col, cols-1]` via `mark_cols()` (editing/mod.rs:223). `put_char()` marks `[col, col+width-1]`. Combined via `expand()`, total damage is `[col, cols-1]`.

Tests: `insert_blank_then_put_char_damages_cursor_to_right_edge` and `insert_blank_at_col_zero_damages_full_line` in `editing/tests.rs` -- both passing with correct damage bounds verified.

### Full Redraw Triggers: VERIFIED

- Resize: `Grid::resize()` -> `dirty.mark_all()` (verified in code)
- Scroll display: `Grid::scroll_display()` -> `dirty.mark_all()` (verified in code)
- Alt screen swap: `toggle_alt_common()` -> `grid_mut().dirty_mut().mark_all()` (verified in alt_screen.rs:83)
- Selection dirty: `Term::selection_dirty` bool set by all mutating operations (separate from grid dirty)
- Font change: `sync_grid_layout()` -> `Grid::resize()` -> `mark_all()` + `pane_cache.invalidate_all()`
- Color/palette change: `apply_color_changes()` -> `mark_all()` + `pane_cache.invalidate_all()`
- Tab switch: natural cache miss (different PaneId)

### Skip Present When Clean: PARTIALLY VERIFIED

`about_to_wait()` checks `any_dirty && budget_elapsed`. `MuxWakeup` -> `mark_pane_window_dirty(pane_id)` targets specific windows. `ControlFlow::Wait` with cursor blink only wakeup verified by `compute_control_flow()` tests (8 tests passing in `event_loop_helpers/tests.rs`).

**Remaining unchecked item:** "when ctx.dirty is set but NO grid rows are actually dirty AND cursor hasn't blinked AND no overlay changed, skip the prepare+render pass entirely" -- acknowledged as low-priority in the plan.

### Snapshot Extraction Optimization: PARTIAL

Profiling done (52us for 120x50, 167us for 240x80 -- under 0.5ms threshold). Remaining items deferred:
- `content.dirty_lines` optimization -- deferred (extraction is fast enough)
- `zerowidth.clone()` SmallVec optimization -- deferred (profiling suggests negligible)
- `collect_damage()` second pass -- deferred (O(lines) vs O(lines*cols), negligible)

---

## 23.2 Parsing Performance: VERIFIED COMPLETE

### Batch Processing: VERIFIED
PTY reader uses 1MB buffer (`READ_BUFFER_SIZE = 0x10_0000`) with 64KB max locked parse (`MAX_LOCKED_PARSE = 0x1_0000`). Verified in `oriterm_mux/src/pty/event_loop/mod.rs`. `PtyEventLoop::parse_chunk()` calls `processor.advance(term, chunk)` on full chunks.

### Fast ASCII Path: VERIFIED
`Grid::put_char_ascii(ch)` exists in `editing/mod.rs`. `Term::input()` in `handler/mod.rs` detects ASCII printable (0x20-0x7E) + no INSERT mode + `charset.is_ascii()` and dispatches to fast path. `CharsetState::is_ascii()` predicate confirmed.

**Tests:** 6 handler tests passing (`ascii_fast_path_writes_cells_correctly`, `ascii_fast_path_preserves_sgr_attributes`, `ascii_fast_path_falls_through_for_insert_mode`, `ascii_fast_path_falls_through_for_non_ascii_charset`, `ascii_fast_path_handles_wrap_at_line_end`, `ascii_fast_path_overwriting_wide_char_falls_to_slow_path`). 4 charset `is_ascii()` tests confirmed.

### Reduce Allocations: VERIFIED
`alloc_regression.rs` integration tests all passing:
- `snapshot_extraction_zero_alloc_steady_state` -- < 50 allocs threshold
- `hundred_frames_zero_alloc_after_warmup` -- 100 frames measured
- `rss_stability_under_sustained_output` -- 100K lines, < 50MB total allocs
- `vte_1mb_ascii_zero_alloc_after_warmup` -- 1MB ASCII parse, < 50 allocs

### Throttle Rendering: VERIFIED
Three mechanisms confirmed: FRAME_BUDGET (16ms), MuxWakeup coalescing (flag-only, render in about_to_wait), synchronized output (Mode 2026 suppression).

**PTY event loop tests:** 12 tests passing including:
- `no_data_loss_under_renderer_contention` (5000 numbered lines with 16ms contention)
- `sync_mode_delivers_content_atomically` (BSU + 10 lines + ESU)
- `reader_throughput_no_contention`, `sustained_flood_no_oom`, etc.

---

## 23.3 Memory Optimization: VERIFIED COMPLETE

### Ring Buffer: VERIFIED
`ScrollbackBuffer` in `ring/mod.rs` (159 lines). O(1) push via `mem::replace` at `start` index, O(1) index via `physical_index()`, incremental growth (no pre-allocation), `push()` returns evicted row for recycling, `pop_newest()` for resize restoration, `drain_oldest_first()` for reflow.

**Tests:** 38 tests in `ring/tests.rs`, all passing. Includes: basic push/retrieve, ring wrap/eviction, clear, iteration, zero-max-scrollback, boundary conditions, push return values, wide char preservation, grid integration (scroll_up, display_offset, sub-region), drain operations, pop/push cycles, resize interactions.

### Row Memory: VERIFIED
Row occupancy tracking (`occ` field) confirmed. `CellExtra` is `Option<Arc<CellExtra>>` (8 bytes when empty). Profiling tests exist (`profile_blank_row_memory`, `profile_resize_allocation_count`) as `#[ignore]` tests -- correctly excluded from CI.

### Alt Screen On-Demand Allocation: VERIFIED
`alt_grid: Option<Grid>` and `alt_image_cache: Option<ImageCache>` in `Term` struct. `ensure_alt_grid()` allocates on first use (alt_screen.rs:62-69). All sync points updated (swap_alt, swap_alt_no_cursor, swap_alt_clear, toggle_alt_common, grid()/grid_mut(), resize, handler/esc.rs).

**Tests:** 5 tests in `term/tests.rs`, all passing: `alt_grid_not_allocated_initially`, `alt_grid_allocated_on_first_entry`, `alt_grid_survives_exit`, `resize_before_alt_screen_no_crash`, `alt_screen_reentry_correct`.

### Atlas Texture Over-Allocation Fix: VERIFIED
`GlyphAtlas::new()` creates texture with `depth_or_array_layers: 1` (line 162). `grow_texture()` method (line 460) creates new texture array, copies existing layers via `CommandEncoder::copy_texture_to_texture()`, increments `generation`. Callers check `generation()` for stale bind groups.

**Tests:** 43 tests in `atlas/tests.rs`, all passing. Includes: creation with 1 page, insert/lookup roundtrip, page growth on overflow (`insert_triggers_new_page_allocation`), growth preserves coordinates (`atlas_growth_preserves_existing_glyph_coordinates`), generation tracking (3 tests: starts at zero, increments on growth, stable on same-page inserts), LRU eviction, clear resets to 1 page, overlapping checks, boundary dimensions.

### Font Data Deduplication: VERIFIED
`FontByteCache` in `font/collection/loading.rs` -- `HashMap<PathBuf, Arc<Vec<u8>>>`. Used by `FontSet::load_cached()` and `from_discovery()`. Cache is short-lived (constructed in `App::new()`, dropped after loading).

**Tests:** 3 tests in `font/collection/tests.rs`, all passing:
- `font_byte_cache_deduplicates_across_calls` -- Arc::ptr_eq verification
- `font_byte_cache_produces_identical_results` -- identical metrics
- `font_byte_cache_dropped_after_loading` -- cache freed after loading

### Content Cache Texture: VERIFIED
Analyzed and deemed acceptable: GPU-only memory (no RSS impact), 20x idle CPU reduction justifies the cost. No action needed.

---

## 23.4 Rendering Performance: PARTIALLY COMPLETE

### Instance Buffer Partial Updates: NOT DONE
Profiling instrumentation added but remaining items unchecked:
- Row-to-instance-range mapping prerequisite done (via `RowInstanceRanges` in dirty_skip)
- Partial `write_buffer()` with offset: NOT implemented
- Cache-hit GPU upload skip: NOT implemented

### Glyph Atlas Growth: VERIFIED (except stress-test)
Multi-page atlas with growth and LRU eviction working. Debug logging at 80% utilization. Missing: stress-test with heavy Unicode workload.

### Frame Pacing: VERIFIED
`ControlFlow::Wait`, FRAME_BUDGET, ctx.dirty per window, cursor blink timer, animation timer, MuxWakeup per-pane targeting -- all confirmed working. 20% -> 1% idle CPU via content cache texture.

### Draw Call Reduction: MOSTLY DONE
Per-pipeline instance buffers, compositor with render target pooling, zero-instance draw skip, draw call count logging. Missing: image texture atlasing (deferred to Section 39).

### Resize Rendering: NOT DONE
Windows-only baby blue background bug during dialog resize -- tracked but not fixed.

### Debug Overlay: NOT DONE
FPS counter, dirty-row percentage, instance count, atlas utilization -- not implemented.

### Skip Off-Screen Content: VERIFIED
`row_off_screen` flag computed per row transition in both `fill_frame_shaped()` and `fill_frame_incremental()`. Cells in off-screen rows are skipped.

### Rendering Performance Tests: PARTIAL
- `incremental_all_dirty_matches_full_rebuild`: PASSING
- `incremental_no_dirty_rows_matches_cached`: PASSING
- `atlas_growth_preserves_existing_glyph_coordinates`: PASSING
- Idle terminal zero redraws test: deferred to manual testing

---

## 23.5 Benchmarks: PARTIALLY COMPLETE

### Throughput Benchmark: VERIFIED
`oriterm_core/benches/vte_throughput.rs` exists with 3 benchmark groups (ASCII-only, mixed, heavy escape) across 2 terminal sizes. Baseline results documented. Benchmarks compile successfully.

### Grid Benchmarks: VERIFIED
`oriterm_core/benches/grid.rs` exists with 19 benchmarks including `renderable_content_into` and `dirty_drain`.

### Rendering Benchmark: NOT DONE (BLOCKED)
`oriterm/benches/rendering.rs` does not exist. Blocked: `oriterm` is a binary crate with no `lib.rs` -- benchmark binaries cannot import `pub(crate)` types. Acknowledged as blocked in the plan.

### Memory Benchmark: NOT DONE
RSS measurement tests (10K/100K scrollback, per-tab overhead, leak detection) not implemented.

### Latency Benchmark: NOT DONE
Internal latency instrumentation (keypress-to-present) not implemented.

### Regression Testing: PARTIAL
Criterion benchmarks exist and compile. Missing: CI pipeline integration, baseline storage, regression detection.

---

## 23.6 Section Completion: NOT STARTED

Most completion criteria are unmet:
- [x] Scrollback ring buffer
- [x] Line-level dirty tracking
- [x] Per-pane PreparedFrame caching
- [x] Frame budget throttling
- [x] PTY read-ahead with bounded parse
- [x] Synchronized output (Mode 2026)
- [x] Grid benchmarks (19 criterion)
- [x] VTE throughput benchmarks
- [ ] All 23.1-23.5 unchecked items
- [ ] `cat 100MB_file.txt` <32ms frames
- [ ] `fill_frame_shaped()` <2ms for 120x50
- [ ] RSS bounded by scrollback
- [ ] Row-level dirty skip wired (DONE -- but listed as incomplete in plan)
- [ ] Idle CPU <0.5%
- [ ] Keypress-to-present p95 <5ms
- [ ] Rendering benchmarks
- [ ] All build/test/clippy passes

---

## Hygiene Issues Found

### 1. `oriterm/src/gpu/atlas/mod.rs` exceeds 500-line limit (579 lines)
The code-hygiene rules state "Source files (excluding tests.rs) must not exceed 500 lines." The atlas module is 579 lines. The `grow_texture()`, `materialize()`, and `evict_lru_page()` methods could be extracted to a submodule.

### 2. `needs_full_repaint()` still `#[allow(dead_code)]`
The plan mentions wiring `FrameInput::needs_full_repaint()` as the gate for full vs incremental prepare. The function exists but is unused in production code -- the decision is made via `!input.content.all_dirty && out.saved_tier.has_cached_rows()` directly. The allow annotation references "damage tracking optimization for later sections" which is now partially done.

### 3. `PaneRenderCache::retain_only()` is dead code
Has `#[allow(dead_code)]` with reason "batch prune API -- call site wired when needed". This is genuinely unwired dead code. Acceptable with the reason annotation but worth noting.

---

## Summary

| Subsection | Status | Tests Pass | Coverage Quality |
|---|---|---|---|
| 23.1 Damage Tracking | MOSTLY COMPLETE | Yes (68 tests) | Excellent -- all dirty paths tested, column bounds tested, selection damage tested, incremental path tested |
| 23.2 Parsing Performance | COMPLETE | Yes (22 tests) | Excellent -- fast ASCII path, alloc regression, sync mode, throughput |
| 23.3 Memory Optimization | COMPLETE | Yes (53 tests) | Excellent -- ring buffer, alt screen lazy alloc, atlas grow-on-demand, font dedup |
| 23.4 Rendering Performance | IN PROGRESS | Yes (for done items) | Good for done items; debug overlay, partial upload, resize bug remain |
| 23.5 Benchmarks | PARTIAL | Yes (compile check) | Throughput/grid benchmarks solid; rendering/memory/latency benchmarks missing |
| 23.6 Section Completion | NOT STARTED | N/A | Multiple exit criteria unmet |

**Overall:** The foundational performance infrastructure is solid -- dirty tracking, instance caching, ring buffer, atlas growth, font dedup, and parsing optimizations are all well-implemented and well-tested. The remaining work is mostly in the optimization tail: partial GPU buffer updates, rendering benchmarks (blocked on crate structure), memory/latency measurement, and the debug overlay. The section is approximately 70% complete.
