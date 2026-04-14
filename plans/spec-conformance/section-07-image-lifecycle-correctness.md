---
section: "07"
title: "Image Lifecycle Correctness"
status: in-progress
reviewed: true
goal: "Add the missing resize handler, reflow-aware placement remapping, and cell-metric plumbing so image placements survive every grid transformation correctly: scrollback eviction, grid resize, column reflow, alt-screen toggle, ED/EL erase, and font-size/DPI changes."
success_criteria:
  - "`ImageCache::on_resize(new_cols, new_rows)` exists and removes image placements whose column extent is entirely outside the new grid bounds"
  - "`ReflowMapping` struct emitted by `Grid::resize` when reflow occurs, with `first_output_row: Vec<usize>` mapping each source row to its output row index (accounting for wrapped-row accumulation into pending out_row)"
  - "`ImageCache::remap_placements(mapping)` exists and translates placement `StableRowIndex` values through the mapping's `first_output_row` table, using `old_total_evicted` as the conversion base and `checked_sub` for underflow safety"
  - "Image placement regression matrix: 3 protocols x 2 sizing modes x 7 mutations (incl. reflow) = 42 scenarios, all pass"
  - "`Term::resize` invokes `remap_placements` (primary cache only, when reflow occurs) then `prune_scrollback` then `on_resize` on primary cache; alt cache gets `on_resize` only (alt grid never reflows)"
  - "Cell-metric plumbing wired: new `PaneIoCommand::SetCellDimensions` variant (NOT extending ImageConfig — static config vs runtime state separation), app sends updated metrics to ALL panes through mux to `Term::set_cell_dimensions` on font-size/DPI changes"
  - "Placement lifecycle methods extracted into `cache/lifecycle.rs` to keep `cache/mod.rs` under 500 lines"
  - "New cache tests in `cache/tests.rs` per test-organization.md"
  - "At least one negative rendering pin: `RenderableContent::images` does NOT contain a removed placement after resize"
  - "Existing image cache tests + teseq tests still pass"
  - "`./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release"
  - "Connects to mission criterion: **Image lifecycle correct under resize/reflow/scrollback/alt-screen**"
inspired_by:
  - "ori_term existing `oriterm_core/src/image/cache/mod.rs:325-358` — `prune_scrollback` and `remove_placements_in_region` are the existing handlers; `on_resize` follows the same `remove_placements_where` + `prune_if_orphaned` pattern"
  - "ori_term existing `oriterm_core/src/grid/resize/mod.rs:303-379` — `reflow_cells` already tracks `src_idx` per row; emitting `first_output_row` per source row is ~30 lines of additional per-row boundary recording"
  - "Ghostty — derives clamped rects at use time; out-of-bounds columns removed on resize"
  - "WezTerm — attaches images to cells (cell-based model); reflows through screen rewrap. ori_term uses cache-coordinate model instead, requiring explicit remapping"
depends_on: ["04"]
third_party_review:
  status: resolved
  updated: 2026-04-13
sections:
  - id: "07.1"
    title: "Research reference impls and extract lifecycle submodule"
    status: complete
  - id: "07.2"
    title: "Write failing regression matrix tests (TDD: tests FIRST)"
    status: complete
  - id: "07.3"
    title: "Add ReflowMapping to Grid::resize"
    status: complete
  - id: "07.4"
    title: "Implement ImageCache::on_resize and remap_placements"
    status: complete
  - id: "07.5"
    title: "Wire Term::resize with full reflow support"
    status: complete
  - id: "07.6"
    title: "Wire cell-metric plumbing (app → mux → Term)"
    status: complete
  - id: "07.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "07.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 07: Image Lifecycle Correctness

**Status:** In Progress
**Goal:** Make image placements survive every grid transformation correctly. This requires three pieces of work: (1) a resize handler that removes column-out-of-bounds placements, (2) a reflow-aware remapping system that keeps placements attached to their content rows when `Grid::resize` rewrites row topology, and (3) cell-metric plumbing so `FixedPixels` placement coverage is updated when font size or DPI changes at runtime.

**The three problems solved by this section:**

1. **Column-out-of-bounds (resize without reflow):** When the grid shrinks horizontally (window resize, DECCOLM 80↔132 toggle), placements whose `cell_col` is beyond the new column count become invalid. Currently NO code handles this — `ImageCache` has `prune_scrollback` and `remove_placements_in_region` but no resize handler.

2. **Row topology rewrite (resize with reflow):** `Grid::resize(..., reflow: true)` calls `reflow_cols` which collects all rows, rewrites them cell-by-cell at the new width, and clears/rebuilds scrollback (`scrollback.clear()` at `grid/resize/mod.rs:268`). During this process `total_evicted` is NOT adjusted, so `StableRowIndex` values computed before reflow silently point to wrong content rows. The `reflow_cells` loop (`grid/resize/mod.rs:303-379`) already tracks `src_idx` per source row — we add per-row output-index recording to emit a `ReflowMapping` with `first_output_row` that maps each source row to the output row where its first cell landed. `ImageCache::remap_placements` then translates each placement's `StableRowIndex` through this mapping.

3. **Stale cell coverage (font/DPI changes):** `Term::set_cell_dimensions` (at `image_config.rs:17`) calls `update_cell_coverage` to recompute `cols`/`rows` for `FixedPixels` placements, but has NO production caller — only test code calls it. `ImageConfig` doesn't carry cell dimensions, `PaneIoCommand::SetImageConfig` doesn't transport them, and `sync_grid_layout`/`handle_dpi_change` in the app layer never send them. We wire this end-to-end.

**Success Criteria:**
- [ ] `ImageCache::on_resize(new_cols, new_rows)` exists and removes out-of-bounds column placements
- [ ] `ReflowMapping` emitted by `Grid::resize` when reflow occurs (`first_output_row` per source row)
- [ ] `ImageCache::remap_placements(mapping)` translates placement StableRowIndex via `first_output_row` + `old_total_evicted`
- [ ] Cell-metric plumbing wired end-to-end (app → mux → `Term::set_cell_dimensions`)
- [ ] Regression matrix: 3 protocols x 2 sizing modes x 7 mutations = 42 scenarios pass
- [ ] `cache/lifecycle.rs` extracted; `cache/mod.rs` under 500 lines
- [ ] New cache tests in `cache/tests.rs` per test-organization.md
- [ ] Negative rendering pin via `RenderableContent::images`
- [ ] Existing image tests + teseq tests still pass
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green
- [ ] Connects to mission criterion: **Image lifecycle correct under resize/reflow/scrollback/alt-screen**

**Context:** The image cache uses a cache-coordinate model: `cell_col: usize` + `cell_row: StableRowIndex`. `StableRowIndex` is `total_evicted + absolute_row_index` (see `grid/stable_index.rs`), making it eviction-stable but NOT reflow-stable. When reflow rewrites row topology, absolute indices shift but `total_evicted` doesn't adjust, so `StableRowIndex` values become stale.

**Kitty placeholder mode note:** Kitty's unicode placeholder protocol (`U=1`) writes `U+10EEEE` characters into grid cells (see `image/mod.rs:29`). For placeholder-mode placements, the SSOT for location is the grid cells themselves — reflow handles these automatically because the placeholder characters move with the text. Section 13 owns placeholder-mode correctness; this section handles cache-coordinate-based protocols (sixel, iTerm2, non-placeholder Kitty).

**Reference implementations:**
- **ori_term existing** `cache/mod.rs:268-286` — `remove_placements_where` + targeted `prune_if_orphaned` is the canonical placement-removal pattern.
- **ori_term existing** `grid/resize/mod.rs:303-379` — `reflow_cells` loop tracks `src_idx` per source row; recording `first_output_row` per source row is O(1) per-row overhead.
- **Ghostty** — derives clamped rects at use time; out-of-bounds columns removed.
- **WezTerm** — cell-attachment model; reflows through screen rewrap. Different architecture.

**Depends on:** Section 04 (SpecHarness exists for regression matrix tests).

**Cross-section links:**
- Section 12 (Sixel), Section 14 (iTerm2) — cache-coordinate placements; this section's handlers cover them.
- Section 13 (Kitty Graphics) — placeholder-mode has different SSOT semantics; this section handles non-placeholder Kitty only.
- Section 26 (Historical Vector Stacks) — depends on section 07 for image lifecycle correctness.

---

## 07.1 Research reference impls and extract lifecycle submodule

**File(s):** `oriterm_core/src/image/cache/mod.rs` (extract from), `oriterm_core/src/image/cache/lifecycle.rs` (new)

**Part A: Research**

- [x] Read reference impls to confirm resize/reflow behavior:
  - `~/projects/reference_repos/console_repos/ghostty/` — image cache resize handling in `src/terminal/kitty/` or equivalent
  - `~/projects/reference_repos/console_repos/wezterm/` — `term/src/terminalstate/image.rs` for cell-attachment model
  - `~/projects/reference_repos/console_repos/kitty/` — `kitty/graphics.py` for resize behavior
- [x] Confirm the column-resize approach: **remove placements entirely outside the new grid bounds** (cell_col >= new_cols). No clamping, no runtime policy enum.
- [x] Research reflow handling: confirm no reference impl remaps cache-coordinate placements through reflow (WezTerm avoids via cell attachment; Ghostty uses tracked pins). Our approach (return-value `ReflowMapping` from `Grid::resize`) is novel for the cache-coordinate model.

**Part B: Extract lifecycle submodule**

`cache/mod.rs` is 436 lines. Adding `on_resize` (~30 lines) + `remap_placements` (~40 lines) + `#[cfg(test)] mod tests;` would exceed 500 lines. Proactive split:

- [x] Create `oriterm_core/src/image/cache/lifecycle.rs` with these methods extracted from `cache/mod.rs`:
  - `prune_scrollback` (currently lines 325-328)
  - `remove_placements_in_region` (currently lines 336-358)
  - `update_cell_coverage` (currently lines 395-410)
  - The new `on_resize` and `remap_placements` methods (implemented in 07.4)
- [x] Update `cache/mod.rs`: add `mod lifecycle;`, remove extracted method bodies, add `#[cfg(test)] mod tests;`
- [x] Create empty `oriterm_core/src/image/cache/tests.rs` with test-organization.md preamble
- [x] Verify `cache/mod.rs` well under 500 lines after extraction
- [x] `./build-all.sh` green — extraction is a refactor, no behavior change
- [x] `./test-all.sh` green — existing tests in `image/tests.rs` pass unchanged

**Validation**: Research documented. Lifecycle submodule extracted. `cache/mod.rs` < 400 lines. All existing tests pass.

---

## 07.2 Write failing regression matrix tests (TDD: tests FIRST)

**File(s):** `oriterm_core/src/image/cache/tests.rs` (new), `oriterm_core/tests/image_lifecycle_matrix.rs` (new)

Per CLAUDE.md TDD discipline, failing tests are written FIRST.

### Unit tests in `cache/tests.rs`

**Column-bounds tests:**
- [x] `on_resize_removes_placement_fully_outside_new_cols()` — place at col=90 spanning 10, resize to 80, assert removed
- [x] `on_resize_preserves_placement_within_new_cols()` — place at col=5 spanning 10, resize to 80, assert survives
- [x] `on_resize_preserves_partially_overlapping_placement()` — place at col=75 spanning 10, resize to 80, assert survives (renderer clips)
- [x] `on_resize_prunes_orphaned_image_after_all_placements_removed()` — store+place at col=90, resize to 80, assert image data removed
- [x] `on_resize_preserves_deferred_kitty_image_without_placements()` — store without placing, resize, assert image data NOT removed

**FixedPixels tests:**
- [x] `on_resize_fixed_pixels_within_bounds_survives()` — FixedPixels placement at col=0, fits in new cols, survives
- [x] `on_resize_fixed_pixels_out_of_bounds_removed()` — FixedPixels placement at col=8, new_cols=8, removed

**Reflow remapping tests:**
- [x] `remap_placements_updates_stable_row_index_after_reflow()` — place image, build a `ReflowMapping` that moves row 5 → output row 10, call `remap_placements`, assert placement's `cell_row` updated to `StableRowIndex(old_total_evicted + 10)`
- [x] `remap_placements_skips_already_evicted_placement()` — place image at StableRowIndex below old_total_evicted (already evicted), assert no panic (checked_sub prevents underflow), placement unchanged
- [x] `remap_placements_handles_unwrap()` — soft-wrapped continuation line unwrapped into single row by width increase, both source rows map to same output row, placement on continuation row remaps correctly
- [x] `remap_placements_handles_row_split()` — single source row split into 2 new rows by width decrease, placement maps to first new row
- [x] `remap_placements_preserves_kitty_deferred_images()` — images with no placements survive remap unchanged

**Negative rendering pin:**
- [x] `removed_placement_not_in_renderable_content()` — construct Term, place sixel at col=90, resize to 80, assert `RenderableContent::images` does NOT contain the removed placement

### Integration matrix in `cache/matrix_tests.rs`

Note: lives at `oriterm_core/src/image/cache/matrix_tests.rs` (not `tests/`) because `ImageCache::{place,store,next_image_id,placements_in_viewport}` are `pub(crate)` — production placements go through VTE handlers, not the public API.

- [x] Build table-driven test matrix:
  ```rust
  struct LifecycleScenario {
      name: &'static str,
      protocol: ImageProtocol,     // Sixel, Kitty, ITerm2
      sizing: PlacementSizingKind, // CellCount, FixedPixels
      mutation: GridMutation,      // ScrollbackEvict, Resize, Reflow, AltEnter, AltExit, EraseDisplay, EraseLine
      expected: PlacementState,    // Survives, Removed, Remapped
  }
  ```
- [x] Enumerate: 3 protocols x 2 sizing modes x 7 mutations = 42 scenarios
- [x] Self-verifying count assertion: `assert_eq!(count, 42);`
- [x] **Validation**: All 42 scenarios pass with the on_resize + remap_placements implementation from 07.4. BUG-08-10 filed during implementation (image_cache()/image_cache_mut() inversion — does not affect matrix correctness since Term::resize operates on fields directly).

---

## 07.3 Add ReflowMapping to Grid::resize

**File(s):** `oriterm_core/src/grid/resize/mod.rs`, `oriterm_core/src/grid/mod.rs`

This subsection adds the row-remap infrastructure that makes reflow-aware image placement remapping possible. The `reflow_cells` loop already tracks `src_idx` per source row — we add per-row boundary recording with O(1) overhead.

- [x] Define `ReflowMapping` struct in `grid/resize/mod.rs`:
  ```rust
  /// Maps old absolute row indices to result row indices after reflow.
  /// Built during `reflow_cells` with O(1) per-row overhead.
  ///
  /// IMPORTANT: wrapped source rows contribute content to the pending
  /// `out_row` without finalizing it (line 362-367: finalization only
  /// happens for non-wrapped rows). So a wrapped row's mapping points
  /// to the result row where its content WILL land once the next
  /// non-wrapped row finalizes the pending out_row.
  #[derive(Debug, Clone)]
  pub struct ReflowMapping {
      /// For each source row index: the result row index where its
      /// first cell was written. For wrapped rows, this is the index
      /// of the pending out_row (which may not be finalized yet —
      /// use `pending_output_row` to track the current out_row's
      /// eventual position). Never empty — every source row maps
      /// to exactly one output row.
      pub first_output_row: Vec<usize>,
      /// Old total_evicted (before reflow) — needed to convert
      /// StableRowIndex → old absolute row index.
      pub old_total_evicted: u64,
  }
  ```
- [x] Modify `reflow_cells()` (at `resize/mod.rs:303`) to build the mapping:
  - Track `pending_output_row = result.len()` — the index the current `out_row` will have when finalized
  - At the START of each source row's processing: record `first_output_row.push(pending_output_row + out_col_contribution)` where `out_col_contribution` accounts for whether we're mid-row (content will land on `pending_output_row`) or if `reflow_row_cells` pushed result rows mid-processing
  - **Key insight**: when `reflow_row_cells` fills `out_row` and pushes it to result mid-row, `pending_output_row` must be updated to `result.len()`. Track this by comparing `result.len()` before and after `reflow_row_cells`
  - For each source row: `first_output_row.push(result.len() - if_just_pushed_else_pending)`
  - The simplest correct approach: record `result.len()` before calling `reflow_row_cells`, then record `result.len()` after. If result grew, the source row's content started on a prior output row. Map to the row where the first cell landed.
  - Return the mapping alongside existing return values
- [x] Capture `old_total_evicted = self.total_evicted` BEFORE `scrollback.clear()` in `apply_reflow_result` — this is the conversion base for StableRowIndex → old absolute row
- [x] Handle scrollback overflow in `apply_reflow_result`: when `self.scrollback.push(row)` evicts an old row (ring buffer at capacity), increment `self.total_evicted`. Currently this is NOT done (confirmed: line 271 pushes without tracking eviction). This is a pre-existing bug affecting ALL StableRowIndex users, not just images.
- [x] Modify `Grid::resize()` return type: `pub fn resize(...) -> Option<ReflowMapping>`:
  - Return `Some(mapping)` when `reflow: true` and column count changed
  - Return `None` when `reflow: false` or column count unchanged
- [x] Export `ReflowMapping` from `grid/mod.rs`
- [x] Sibling tests in `grid/resize/tests.rs`:
  - `reflow_mapping_tracks_row_split()` — 80-col grid with a 120-char wrapped (WRAP flag set) line, resize to 60 cols, verify split
  - `reflow_mapping_tracks_unwrap()` — 40-col grid with a soft-wrapped continuation line (WRAP flag on first row), resize to 80, verify both source rows map to the SAME output row (unwrap, not merge of independent lines)
  - `reflow_mapping_none_when_no_reflow()` — resize with `reflow: false`, verify `None` returned
  - `reflow_mapping_none_when_cols_unchanged()` — resize with same cols but different rows, verify `None` returned
- [x] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green

**Validation**: `ReflowMapping` is produced correctly for row splits, merges, and no-change cases. Grid tests pass. No regressions.

---

## 07.4 Implement ImageCache::on_resize and remap_placements

**File(s):** `oriterm_core/src/image/cache/lifecycle.rs`

### on_resize (column-bounds removal)

- [x] Add `pub(crate) fn on_resize(&mut self, new_cols: usize, _new_rows: usize)`:
  - `remove_placements_where(|p| p.cell_col >= new_cols)` — removes placements whose starting column is entirely outside the new grid
  - Partially overlapping placements (start < new_cols, end >= new_cols) survive — the renderer clips
  - `prune_if_orphaned(&affected_ids)` — targeted pruning only, preserving Kitty deferred images
  - `_new_rows` accepted for forward-compatibility but unused (row bounds handled by StableRowIndex eviction)
  - Mark `dirty = true` if any placements removed
- [x] Unit tests from 07.2 now pass for column-bounds scenarios

### remap_placements (reflow-aware row remapping)

- [x] Add `pub(crate) fn remap_placements(&mut self, mapping: &ReflowMapping)`:
  - For each placement, convert `cell_row: StableRowIndex` to old absolute row using `checked_sub`: `let Some(old_abs) = cell_row.0.checked_sub(mapping.old_total_evicted) else { continue; }` — if underflow, the placement was already evicted before reflow; skip it (prune_scrollback will clean it up)
  - If `old_abs as usize >= mapping.first_output_row.len()`: skip (out of range — row added after mapping was built)
  - Look up `new_output_row = mapping.first_output_row[old_abs as usize]`
  - Update `cell_row = StableRowIndex(mapping.old_total_evicted + new_output_row as u64)` — use `old_total_evicted` as the base, NOT `new_total_evicted`, because `first_output_row` indices are relative to the pre-eviction result array
  - **Never remove a placement because its mapping range is "empty"** — in the reflow algorithm, wrapped rows contribute to a pending `out_row` without finalizing it, so an empty range means "absorbed into pending row", NOT "deleted". Every source row maps to exactly one output row via `first_output_row`.
  - After processing, `prune_if_orphaned` for any removed placements (only from underflow/out-of-range skips)
  - Mark `dirty = true` if any placements changed
- [x] Unit tests from 07.2 now pass for reflow remapping scenarios

**Validation**: All cache unit tests pass. `./test-all.sh` green.

---

## 07.5 Wire Term::resize with full reflow support

**File(s):** `oriterm_core/src/term/mod.rs` (around line 446)

- [x] Modify `Term::resize` to capture the `Option<ReflowMapping>` from `Grid::resize`:
  ```rust
  let mapping = self.grid.resize(new_lines, new_cols, reflow);
  ```
- [x] **Operation ordering is critical**: `remap_placements` MUST run BEFORE `prune_scrollback`, because remap translates old StableRowIndex values to new ones — if prune runs first, it compares unmapped (old) cell_row values against the post-reflow eviction boundary and incorrectly deletes placements whose content survived reflow.
  ```rust
  // 1. Remap FIRST (translate old StableRowIndex → new)
  if let Some(ref mapping) = mapping {
      self.image_cache.remap_placements(mapping);
  }
  // 2. THEN prune scrollback (now using correctly remapped row indices)
  if new_primary > prev_primary {
      self.image_cache.prune_scrollback(StableRowIndex(new_primary as u64));
  }
  // 3. THEN remove column-out-of-bounds
  self.image_cache.on_resize(new_cols, new_lines);
  ```
- [x] For the alt image cache — use the same condition as alt grid resize (`if let Some(alt) = &mut self.alt_grid`), matching alt grid EXISTENCE, NOT alt screen active:
  ```rust
  if let Some(cache) = &mut self.alt_image_cache {
      cache.on_resize(new_cols, new_lines);
      // Alt grid never reflows (reflow: false), so no remap needed
  }
  ```
- [x] Sibling tests in `oriterm_core/src/term/tests.rs`:
  - `term_resize_removes_out_of_bounds_image_placement()` — sixel at col=90, resize to 80, assert removed
  - `term_resize_updates_alt_cache_when_alt_exists()` — ensure_alt_grid, place in alt, resize, assert removed
  - `term_resize_remaps_image_placement_through_reflow()` — place image, resize with reflow=true, assert placement's `cell_row` updated to follow content
  - `term_resize_without_reflow_skips_remap()` — place image, resize with reflow=false, assert `cell_row` unchanged
- [x] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green

**Validation**: Term-level tests pass. Existing teseq tests pass. Integration matrix from 07.2 passes for resize + reflow scenarios.

---

## 07.6 Wire cell-metric plumbing (app → mux → Term)

**File(s):** `oriterm_mux/src/backend/mod.rs`, `oriterm_mux/src/pane/io_thread/handler.rs`, `oriterm/src/app/chrome/resize.rs`, `oriterm/src/app/mod.rs`

This subsection wires the end-to-end path so `Term::set_cell_dimensions` (at `image_config.rs:17`) gets called in production when font size or DPI changes. Currently it has zero production callers.

**Approach**: Add a NEW `PaneIoCommand::SetCellDimensions { width: u16, height: u16 }` variant — do NOT extend `ImageConfig`. `ImageConfig` represents user TOML configuration (enabled, memory_limit, max_single, animation_enabled). Cell dimensions are runtime state derived from font rasterization — mixing them into `ImageConfig` would mean a config reload (`apply_image_changes` in `config_reload/mod.rs:338`) overwrites runtime cell metrics with stale/zero values from the TOML config struct, violating SSOT by conflating static config with hardware/font state.

- [x] Add `SetCellDimensions { width: u16, height: u16 }` variant to `PaneIoCommand` in `oriterm_mux/src/pane/io_thread/commands/mod.rs`
- [x] Update `fmt::Debug` for the new variant
- [x] Add `MuxPdu::SetCellDimensions { pane_id: PaneId, width: u16, height: u16 }` in `oriterm_mux/src/protocol/messages.rs`
- [x] Register the new PDU in the wire protocol sync points:
  - `oriterm_mux/src/protocol/pdu_traits.rs` — add dispatch arm
  - `oriterm_mux/src/protocol/msg_type.rs` — add numeric `MsgType` variant + `from_u16` arm
  - `oriterm_mux/src/protocol/tests.rs` — add inventory and roundtrip test for `SetCellDimensions`
  - `oriterm_mux/src/server/dispatch/mod.rs` — daemon-side PDU → `PaneIoCommand` dispatch
- [x] Add handler in `oriterm_mux/src/pane/io_thread/handler.rs`:
  ```rust
  PaneIoCommand::SetCellDimensions { width, height } => {
      self.terminal.set_cell_dimensions(width, height);
  }
  ```
- [x] Add `set_cell_dimensions(pane_id, width, height)` method to `MuxBackend` trait and implement for embedded + daemon backends
- [x] In `oriterm/src/app/chrome/resize.rs` `sync_grid_layout()`: send cell metrics to EVERY pane in the window REGARDLESS of whether grid dimensions changed. Font-size changes via config reload can change cell metrics without changing cols/rows (the grid_changed branch at `resize.rs:109-123` only runs when dimensions change). Cell-metric propagation must run unconditionally after cell metrics are computed:
  ```rust
  let cell = renderer.cell_metrics();
  let w = cell.width.round() as u16;
  let h = cell.height.round() as u16;
  for pane_id in all_pane_ids_in_window {
      mux.set_cell_dimensions(pane_id, w, h);
  }
  ```
- [x] In `oriterm/src/app/mod.rs` `handle_dpi_change()`: after font re-rasterization, send cell metrics to all panes in the affected window
- [x] **No zero-metric fallback**: `ImageConfig` construction sites are NOT modified — they continue sending config-only data. Cell dimensions are sent SEPARATELY, only when the renderer has real metrics available.
- [x] **Pane creation paths**: every pane setup path must send cell dimensions immediately after pane creation (not waiting for a later resize/DPI event). The 6 pane creation sites are:
  1. `init/mod.rs:518` — initial pane setup
  2. `init/mod.rs:570` — initial pane setup (alt path)
  3. `tab_management/mod.rs:54` — new tab
  4. `pane_ops/mod.rs:104` — split pane
  5. `pane_ops/floating.rs:66` — floating pane
  6. `window_management/create.rs:70` — new window pane (currently only sends `set_pane_theme`)
  Add `SetCellDimensions` calls at each site via shared helper: `fn send_cell_metrics_to_pane(mux, pane_id, cell_w, cell_h)`. Without this, newly created panes start with Term's default 8x16 metrics.
  **Init-time caveat**: the two `init/mod.rs` paths (`create_initial_tab`, `create_handoff_tab`) run before `WindowContext` is inserted, so `renderer` isn't accessible at the call site. Solution: pass `(cell_w, cell_h)` into the tab-creation helpers from `try_init` (which has just finished font rasterization and knows the metrics), or have those functions return the `PaneId` so `try_init` sends metrics immediately after the window context is inserted. Either approach is acceptable — the constraint is that the pane receives real cell metrics before any image protocol data arrives.
- [x] Sibling test: Term-level `FixedPixels` coverage recomputation covered by `update_cell_coverage_recalculates_fixed_pixel_placements` + `fixed_pixel_placement_viewport_correct_after_resize` in `oriterm_core/src/image/tests.rs` and `on_resize_fixed_pixels_*` in `oriterm_core/src/image/cache/tests.rs`.
- [x] Integration test: `test_set_cell_dimensions_command_marks_dirty` in `oriterm_mux/src/pane/io_thread/tests.rs` verifies the command flags grid-dirty so snapshots pick up the cell-metric change; `set_cell_dimensions_missing_pane_is_noop` in `oriterm_mux/src/backend/embedded/tests.rs` guards the multi-pane broadcast against stale pane ids.
- [x] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green (including Windows cross-compile — wire protocol change)
- [x] Close BUG-08-9 in `plans/bug-tracker/section-08-core-terminal.md`

**Validation**: `set_cell_dimensions` now has a production caller. FixedPixels placements get correct coverage after font/DPI changes. BUG-08-9 resolved.

---

## 07.R Third Party Review Findings

**Round 1 (pre-rescoping):**
- [x] `[TPR-07-001-codex][high]` — Fill the GAP in FixedPixels cell-metric plumbing.
  Resolved: Rescoped section to include full cell-metric plumbing in 07.6. BUG-08-9 filed.
- [x] `[TPR-07-002-codex][medium]` — Resolve frontmatter/body DRIFT on reflow scope.
  Resolved: Rescoped to include full reflow remapping in 07.3/07.4/07.5.

**Round 2 (post-rescoping):**
- [x] `[TPR-07-001-codex][high]` — Redesign ReflowMapping around real wrapped-row merges.
  Resolved: Rewritten 07.3 to use `first_output_row` per source row, accounting for wrapped rows' pending `out_row` accumulation. Removed empty-range merge semantics.
- [x] `[TPR-07-002-codex][high]` — Fan cell-metric updates out to every resized pane.
  Resolved: 07.6 now sends metrics to ALL panes in the window, not just active pane. Multi-pane integration test added.
- [x] `[TPR-07-003-codex][medium]` — Delete the 0x0 cell-metric fallback.
  Resolved: Separated `SetCellDimensions` from `ImageConfig`. No zero-metric fallback. Renderer-free config paths don't touch cell metrics.
- [x] `[TPR-07-001-gemini][high]` — Fix silent total_evicted drops in apply_reflow_result.
  Resolved: Added checklist item in 07.3 to increment `total_evicted` when `scrollback.push()` evicts (pre-existing bug fix).
- [x] `[TPR-07-002-gemini][high]` — Correct StableRowIndex calculation base in remap_placements.
  Resolved: 07.4 uses `old_total_evicted` as base, not `new_total_evicted`.
- [x] `[TPR-07-003-gemini][high]` — Run remap_placements before prune_scrollback on resize.
  Resolved: 07.5 ordering is now: remap → prune → on_resize.
- [x] `[TPR-07-004-gemini][medium]` — Do not remove placements when source row range is empty.
  Resolved: 07.4 uses `first_output_row` — every source row maps to exactly one output row. No empty-range removal.
- [x] `[TPR-07-005-gemini][medium]` — Prevent underflow when calculating old_abs for remapping.
  Resolved: 07.4 uses `checked_sub` with `continue` on underflow.
- [x] `[TPR-07-006-gemini][medium]` — Separate cell metrics from static ImageConfig.
  Resolved: 07.6 uses new `PaneIoCommand::SetCellDimensions` variant. `ImageConfig` unchanged.

**Round 3:**
- [x] `[TPR-07-001-codex][medium]` — Rewrite summary contract to match first_output_row and primary-only remap.
  Resolved: Fixed stale "row ranges" and "both caches" language in frontmatter, success criteria, and inspired_by.
- [x] `[TPR-07-002-codex][high]` — Add pane-creation and adoption paths to cell-metric plumbing.
  Resolved: Added 6 pane creation sites (including window_management/create.rs) to 07.6 with shared helper. Init-time caveat documented. Regression tests added.

**Round 4:**
- [x] `[TPR-07-001-codex][high]` — Add new-window pane creation path to cell-metric fanout.
  Resolved: Added window_management/create.rs as 6th pane path.
- [x] `[TPR-07-002-codex][high]` — Decouple cell-metric propagation from grid-size changes.
  Resolved: sync_grid_layout sends metrics unconditionally. Font-only metric test added.
- [x] `[TPR-07-003-codex][medium]` — Purge stale row-range and ImageConfig-extension language.
  Resolved: Fixed in section body and index.md.
- [x] `[TPR-07-001-gemini][medium]` — Fix stale row-range language in section summary.
  Resolved: Fixed.
- [x] `[TPR-07-002-gemini][low]` — Fix stale row-range language in index.md.
  Resolved: Fixed.

**Round 5:**
- [x] `[TPR-07-001-codex][high]` — Account for mux wire protocol sync points.
  Resolved: Added pdu_traits.rs, msg_type.rs, protocol/tests.rs to 07.6 file list.
- [x] `[TPR-07-002-codex][medium]` — Specify how init-only pane paths obtain cell metrics.
  Resolved: Added init-time caveat documenting two approaches for passing metrics to create_initial_tab/create_handoff_tab.
- [x] `[TPR-07-003-codex][low]` — Purge remaining stale round-4 wording.
  Resolved: Fixed row-range tracking → first_output_row in reference section.
- [x] `[TPR-07-001-gemini][high]` — Add round 4 findings to 07.R.
  Resolved: All round 4+5 findings recorded.
- [x] `[TPR-07-002-gemini][medium]` — Purge remaining stale row-range terminology.
  Resolved: Fixed.

**Round 6 (post-implementation):**
- [x] `[TPR-07-001-codex][high]` `oriterm_core/src/term/mod.rs:474` — Route each resize path through the cache that owns that screen.
  Resolved: Fixed on 2026-04-13 in commit {next}. Removed the image-cache field swap from `toggle_alt_common`. `image_cache` and `alt_image_cache` fields now carry their semantic contents at all times. `image_cache()` / `image_cache_mut()` route by `ALT_SCREEN` mode (mirroring `grid()` / `grid_mut()`). `Term::resize` now correctly pairs `self.grid` with `self.image_cache` (primary) and `self.alt_grid` with `self.alt_image_cache` (alt). Added regression `term_resize_routes_each_grid_through_its_own_image_cache` that reads fields directly to catch any future routing inversion. BUG-08-10 closed as part of this fix. Matrix test `image_lifecycle_matrix` updated to swap back to primary mode before observation (the old bug meant alt-mode `image_cache()` leaked primary content, which the old matrix inadvertently relied on).
- [x] `[TPR-07-001-gemini][high]` `oriterm_core/src/term/mod.rs:456` — Fix primary grid reflow mapping applied to alt image cache in alt screen mode.
  Resolved: Same fix as [TPR-07-001-codex] (agreement — both reviewers flagged the same root cause).
- [x] `[TPR-07-002-gemini][medium]` `oriterm/src/app/chrome/resize.rs:135` — Prevent unconditional cell metrics broadcast from dirtying all pane snapshots.
  Resolved: Fixed on 2026-04-13. Added `WindowContext::last_broadcast_cell_dims: Option<(u16, u16)>` and short-circuit in `App::broadcast_cell_metrics_to_window`: if the new `(cell_w, cell_h)` equals the last broadcast dims, return early without dirtying any pane or issuing `set_cell_dimensions` IPC. On first broadcast (field is `None`) and on any change, the broadcast runs and the field is updated. `sync_grid_layout` and `handle_dpi_change` callers are unchanged — the short-circuit lives at the canonical broadcast site (SSOT).

**Round 7 (post-round-6 TPR):**
- [x] `[TPR-07-001-codex][medium]` `oriterm/src/app/cell_metrics.rs:67` — Invalidate cached broadcast state when a window gains panes.
  Resolved: Fixed on 2026-04-13. Added `App::seed_pane_with_window_cell_metrics` helper that sends the destination window's current cell metrics to a specific pane. Wired into `move_tab_to_window` (oriterm/src/app/tab_management/move_ops.rs) and `tear_off_tab` (oriterm/src/app/tab_drag/tear_off.rs) — after panes move across a window boundary, each moved pane is seeded with the destination window's metrics. Per-pane seeding is surgical (no broadcast, no other pane re-dirtied), which matters when source and destination windows share metrics (the short-circuit would skip a full rebroadcast in that case).
- [x] `[TPR-07-002-codex][low]` `oriterm/src/app/cell_metrics.rs:51` — Add regression test for broadcast short-circuit.
  Resolved: Same fix as [TPR-07-001-gemini] round 7 (agreement — both reviewers flagged the same test gap).
- [x] `[TPR-07-001-gemini][medium]` `oriterm/src/app/cell_metrics.rs:57` — Missing regression test for cell metric broadcast short-circuit.
  Resolved: Fixed on 2026-04-13. Extracted `cell_metric_broadcast_needed(last, new) -> bool` as a pure decision helper and added `oriterm/src/app/cell_metrics/tests.rs` with 6 unit tests: `first_broadcast_always_fires`, `identical_dims_short_circuit`, `width_change_fires`, `height_change_fires`, `both_axis_change_fires`, `negative_pin_degenerate_dims_short_circuit`. The short-circuit rule is now pinned by a test — a future refactor that reverts the guard will fail `identical_dims_short_circuit`.

**Round 8 (post-round-7 TPR):**
- [x] `[TPR-07-001-codex][low]` `oriterm/src/app/cell_metrics/tests.rs:3` — Add regression coverage for cross-window pane seeding.
  Resolved: Filed as `[BUG-09-3][low]` — full integration test for `seed_pane_with_window_cell_metrics` from move/tear-off call sites requires an App fixture that does not yet exist in test infrastructure. Bug entry captures the gap and proposed fix (minimal App+EmbeddedMux+WindowContext fixture OR architecture test grepping for call-site patterns). `TPR-07-003-gemini` below was a separate concern (stateful cache-assignment not pinned) and was resolved inline via `try_claim_broadcast`, not via BUG-09-3.
- [x] `[TPR-07-001-gemini][high]` `oriterm/src/app/chrome/resize.rs:126` — Queue SetCellDimensions before Resize to prevent incorrect image eviction.
  Resolved: Rejected after verification on 2026-04-13. Claim is factually incorrect. `ImageCache::on_resize` (`oriterm_core/src/image/cache/lifecycle.rs:86`) implements removal as `remove_placements_where(|p| p.cell_col >= new_cols)` — it consults ONLY the placement's STARTING column index (`p.cell_col`) against the new grid column count (`new_cols`). It does NOT read `p.cols` (placement span) nor any cell pixel metric. Therefore the order of `SetCellDimensions` vs `Resize` in the IO-thread command queue cannot affect which placements get evicted on resize. `SetCellDimensions` updates per-placement `cols`/`rows` via `update_cell_coverage` for FixedPixels placements, which affects RENDERING (how many cells the renderer paints) but not `on_resize` eviction. The reviewer conflated "cell coverage" (placement span) with "eviction criterion" (starting column index).
- [x] `[TPR-07-002-gemini][medium]` `oriterm/src/app/tab_management/move_ops.rs:39` — move_tab_to_window resizes wrong panes if destination is unfocused.
  Resolved: Verified true — `resize_all_panes()` routes through `compute_pane_layouts()` which uses `self.active_window` + `self.focused_ctx()` (`oriterm/src/app/redraw/multi_pane/pane_layouts.rs:15,26`), tied to the focused window. Cross-window tab moves to a BACKGROUND window currently resize the wrong panes. Filed as BUG-09-2 (medium). The fix is architectural (refactor `resize_all_panes` / `compute_pane_layouts` to accept a target window parameter, or add `resize_panes_in_tab(tab_id, winit_id)` helper) — scoped beyond Section 07 and tracked for dedicated `/fix-bug` work. Section 07's own fix (`seed_pane_with_window_cell_metrics`) DOES correctly target the destination window, so cell-metric seeding for cross-window moves works; only grid-dimension resize is affected by BUG-09-2.
- [x] `[TPR-07-003-gemini][medium]` `oriterm/src/app/cell_metrics/tests.rs:10` — Stateful cache update for short-circuit rule remains unpinned.
  Resolved: Fixed on 2026-04-13. Added `try_claim_broadcast(cached: &mut Option<(u16, u16)>, new: (u16, u16)) -> bool` helper that fuses the decision AND the state update into a single testable function. `broadcast_cell_metrics_to_window` now delegates to it. Added 4 new tests in `oriterm/src/app/cell_metrics/tests.rs`: `try_claim_broadcast_updates_cache_on_first_call` (pins assignment on first call), `try_claim_broadcast_short_circuits_and_leaves_cache` (pins no-op on match), `try_claim_broadcast_updates_cache_on_change` (pins assignment on change), `try_claim_broadcast_full_sequence` (pins the None → match → change → match state machine). A refactor that removes the cache assignment from `try_claim_broadcast` now fails at least 3 of these tests.
- [x] `[TPR-07-004-gemini][informational]` `oriterm/src/app/tab_drag/tear_off.rs:96` — Redundant manual seeding of cell metrics during tear-off.
  Resolved: Rejected after verification on 2026-04-13. Claim is incorrect. `handle_redraw` does NOT call `sync_grid_layout`. Grepping `sync_grid_layout` across `oriterm/src/app/` finds exactly three callers: `chrome/resize.rs:252` (window resize events), `config_reload/mod.rs:218` (config reload), and `config_reload/window_config.rs:100` (window config change). None of them fire from `handle_redraw` directly. Without the manual seed in `tear_off_tab`, the torn window's panes would wait for the NEXT window-resize event to receive `SetCellDimensions`, during which time `FixedPixels` image placements would use stale metrics. The seed is required, not redundant.

**Round 9 (post-round-8 TPR — docs/metadata consistency):**
- [x] `[TPR-07-001-codex][low]` `plans/bug-tracker/00-overview.md:42` — Correct the section 09 open-bug count.
  Resolved: Fixed on 2026-04-13. Overview table's "Open" column for Section 09 updated from `1` to `3` to match the three `[ ]` entries (BUG-09-1, BUG-09-2, BUG-09-3) in `plans/bug-tracker/section-09-session.md`.
- [x] `[TPR-07-002-codex][low]` `plans/spec-conformance/section-07-image-lifecycle-correctness.md:433` — Align BUG-09-3 metadata with the filed bug entry.
  Resolved: Fixed on 2026-04-13. Rewrote the round-8 `[TPR-07-001-codex]` resolution to reference BUG-09-3's actual severity (`[low]`) and to clarify that `TPR-07-003-gemini` was resolved inline via `try_claim_broadcast` (not merged into BUG-09-3). The two findings are now correctly separated in the audit trail.

Round 9 clean-pass gate: Gemini returned `"no_findings": true` (fully clean). Codex surfaced only the two low-severity plan/metadata inconsistencies above, fixed in this round. Section 07's TPR gate is now satisfied for close-out — no actionable findings remain.

---

## 07.N Completion Checklist

**TDD ordering enforced:** 07.2 (failing tests) BEFORE 07.3-07.6 (implementation). 07.1 (research + refactor) is prerequisite for all.

- [ ] 07.1: Reference impl research completed — resize + reflow behavior confirmed
- [ ] 07.1: `cache/lifecycle.rs` extracted from `cache/mod.rs` — `prune_scrollback`, `remove_placements_in_region`, `update_cell_coverage` moved
- [ ] 07.1: `cache/mod.rs` < 400 lines after extraction
- [ ] 07.1: `cache/tests.rs` created with `#[cfg(test)] mod tests;` in `cache/mod.rs`
- [ ] 07.1: Existing `image/tests.rs` tests still pass
- [ ] 07.2: Failing test matrix written — cache unit tests (11+ in `cache/tests.rs`) + integration matrix (42 scenarios)
- [ ] 07.2: on_resize + remap tests initially FAIL
- [ ] 07.2: Existing-handler scenarios pass — if any fail, file via `/add-bug`
- [ ] 07.2: Negative rendering pin written
- [ ] 07.3: `ReflowMapping` struct defined with `first_output_row: Vec<usize>` and `old_total_evicted: u64`
- [ ] 07.3: `reflow_cells()` builds mapping accounting for wrapped rows' pending out_row
- [ ] 07.3: `apply_reflow_result` increments `total_evicted` on scrollback overflow (pre-existing bug fix)
- [ ] 07.3: `Grid::resize()` returns `Option<ReflowMapping>`
- [ ] 07.3: Grid reflow tests pass (row split, unwrap, no-reflow cases)
- [ ] 07.4: `ImageCache::on_resize(new_cols, new_rows)` implemented in `lifecycle.rs`
- [ ] 07.4: Uses `remove_placements_where` + targeted `prune_if_orphaned` (NOT full orphan sweep)
- [ ] 07.4: `ImageCache::remap_placements(mapping)` uses `checked_sub` for underflow prevention
- [ ] 07.4: Never removes placements for "empty range" — wrapped rows map via `first_output_row`
- [ ] 07.4: Uses `old_total_evicted` as StableRowIndex base (not new_total_evicted)
- [ ] 07.4: All cache unit tests pass
- [ ] 07.5: `Term::resize` captures `Option<ReflowMapping>` from `Grid::resize`
- [ ] 07.5: Operation ordering: remap FIRST → prune scrollback → on_resize (column bounds)
- [ ] 07.5: Alt cache gets `on_resize` only (alt grid never reflows)
- [ ] 07.5: Alt cache condition: `if alt_image_cache exists` (matches alt grid existence, NOT active)
- [ ] 07.5: Term-level tests pass including reflow remapping
- [ ] 07.6: NEW `PaneIoCommand::SetCellDimensions` variant (NOT extending ImageConfig)
- [ ] 07.6: IO thread handler calls `set_cell_dimensions`
- [ ] 07.6: `sync_grid_layout()` sends cell metrics to ALL panes in window (not just active)
- [ ] 07.6: `handle_dpi_change()` sends cell metrics to ALL panes in affected window
- [ ] 07.6: Existing `ImageConfig` construction sites NOT modified (separation of concerns)
- [ ] 07.6: All 6 pane creation paths send `SetCellDimensions` via shared helper after pane setup (incl. window_management/create.rs)
- [ ] 07.6: Multi-pane integration test: newly created split pane gets correct cell metrics without resize
- [ ] 07.6: Multi-pane integration test: both split panes get updated metrics after font change
- [ ] 07.6: Regression test: font-size change without grid-size change still sends SetCellDimensions
- [ ] 07.6: Regression test: new-window pane gets correct cell metrics on creation
- [ ] 07.6: Wire protocol sync points: pdu_traits.rs, msg_type.rs, protocol/tests.rs updated for SetCellDimensions
- [ ] 07.6: Windows cross-compile green (wire protocol change)
- [ ] 07.6: BUG-08-9 closed
- [ ] **Matrix**: 3 protocols x 2 sizing modes x 7 mutations = 42 scenarios + self-verifying count
- [ ] **Semantic pin**: `on_resize_removes_placement_fully_outside_new_cols` — ONLY passes with new behavior
- [ ] **Negative pin**: `removed_placement_not_in_renderable_content` — rejected from render output
- [ ] All 42 matrix scenarios pass
- [ ] Existing image cache tests pass without modification
- [ ] Existing teseq tests pass
- [ ] Alloc regression unchanged
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release
- [ ] Plan annotation cleanup
- [ ] Section frontmatter `status` → `complete`
- [ ] `00-overview.md` Quick Reference + mission criteria updated
- [ ] `index.md` section 07 status updated
- [ ] Cross-links verified: sections 12, 13, 14, 26 reference section 07
- [ ] `/tpr-review` passed
- [ ] `/impl-hygiene-review last commit` passed (after `/tpr-review` is clean)

**Exit Criteria:** Image placements survive every grid transformation: resize, reflow, scrollback eviction, alt-screen toggle, ED/EL erase, and font/DPI changes. No limitations scoped out. Regression matrix proves it with 42 scenarios. Ready for sections 12-14 (image protocols) and section 21 (notcurses-demo harness).
