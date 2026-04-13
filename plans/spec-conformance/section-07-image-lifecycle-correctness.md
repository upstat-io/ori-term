---
section: "07"
title: "Image Lifecycle Correctness"
status: not-started
reviewed: false
goal: "Add the missing resize handler, reflow-aware placement remapping, and cell-metric plumbing so image placements survive every grid transformation correctly: scrollback eviction, grid resize, column reflow, alt-screen toggle, ED/EL erase, and font-size/DPI changes."
success_criteria:
  - "`ImageCache::on_resize(new_cols, new_rows)` exists and removes image placements whose column extent is entirely outside the new grid bounds"
  - "`ReflowMapping` struct emitted by `Grid::resize` when reflow occurs, mapping old absolute row indices to new row ranges"
  - "`ImageCache::remap_placements(mapping)` exists and translates placement `StableRowIndex` values through a reflow mapping so images follow their content rows"
  - "Image placement regression matrix: 3 protocols x 2 sizing modes x 7 mutations (incl. reflow) = 42 scenarios, all pass"
  - "`Term::resize` invokes `on_resize` AND `remap_placements` (when reflow occurs) on both primary and alt image caches"
  - "Cell-metric plumbing wired: `ImageConfig` carries `cell_width`/`cell_height`, app sends updated metrics through mux to `Term::set_cell_dimensions` on font-size/DPI changes"
  - "Placement lifecycle methods extracted into `cache/lifecycle.rs` to keep `cache/mod.rs` under 500 lines"
  - "New cache tests in `cache/tests.rs` per test-organization.md"
  - "At least one negative rendering pin: `RenderableContent::images` does NOT contain a removed placement after resize"
  - "Existing image cache tests + teseq tests still pass"
  - "`./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release"
  - "Connects to mission criterion: **Image lifecycle correct under resize/reflow/scrollback/alt-screen**"
inspired_by:
  - "ori_term existing `oriterm_core/src/image/cache/mod.rs:325-358` — `prune_scrollback` and `remove_placements_in_region` are the existing handlers; `on_resize` follows the same `remove_placements_where` + `prune_if_orphaned` pattern"
  - "ori_term existing `oriterm_core/src/grid/resize/mod.rs:303-379` — `reflow_cells` already tracks `src_idx` per row; emitting a row-remap table is ~30 lines of additional tracking"
  - "Ghostty — derives clamped rects at use time; out-of-bounds columns removed on resize"
  - "WezTerm — attaches images to cells (cell-based model); reflows through screen rewrap. ori_term uses cache-coordinate model instead, requiring explicit remapping"
depends_on: ["04"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "07.1"
    title: "Research reference impls and extract lifecycle submodule"
    status: not-started
  - id: "07.2"
    title: "Write failing regression matrix tests (TDD: tests FIRST)"
    status: not-started
  - id: "07.3"
    title: "Add ReflowMapping to Grid::resize"
    status: not-started
  - id: "07.4"
    title: "Implement ImageCache::on_resize and remap_placements"
    status: not-started
  - id: "07.5"
    title: "Wire Term::resize with full reflow support"
    status: not-started
  - id: "07.6"
    title: "Wire cell-metric plumbing (app → mux → Term)"
    status: not-started
  - id: "07.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "07.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 07: Image Lifecycle Correctness

**Status:** Not Started
**Goal:** Make image placements survive every grid transformation correctly. This requires three pieces of work: (1) a resize handler that removes column-out-of-bounds placements, (2) a reflow-aware remapping system that keeps placements attached to their content rows when `Grid::resize` rewrites row topology, and (3) cell-metric plumbing so `FixedPixels` placement coverage is updated when font size or DPI changes at runtime.

**The three problems solved by this section:**

1. **Column-out-of-bounds (resize without reflow):** When the grid shrinks horizontally (window resize, DECCOLM 80↔132 toggle), placements whose `cell_col` is beyond the new column count become invalid. Currently NO code handles this — `ImageCache` has `prune_scrollback` and `remove_placements_in_region` but no resize handler.

2. **Row topology rewrite (resize with reflow):** `Grid::resize(..., reflow: true)` calls `reflow_cols` which collects all rows, rewrites them cell-by-cell at the new width, and clears/rebuilds scrollback (`scrollback.clear()` at `grid/resize/mod.rs:268`). During this process `total_evicted` is NOT adjusted, so `StableRowIndex` values computed before reflow silently point to wrong content rows. The `reflow_cells` loop (`grid/resize/mod.rs:303-379`) already tracks `src_idx` per source row — we add row-range tracking to emit a `ReflowMapping` that maps old absolute rows → new absolute row ranges. `ImageCache::remap_placements` then translates each placement's `StableRowIndex` through this mapping.

3. **Stale cell coverage (font/DPI changes):** `Term::set_cell_dimensions` (at `image_config.rs:17`) calls `update_cell_coverage` to recompute `cols`/`rows` for `FixedPixels` placements, but has NO production caller — only test code calls it. `ImageConfig` doesn't carry cell dimensions, `PaneIoCommand::SetImageConfig` doesn't transport them, and `sync_grid_layout`/`handle_dpi_change` in the app layer never send them. We wire this end-to-end.

**Success Criteria:**
- [ ] `ImageCache::on_resize(new_cols, new_rows)` exists and removes out-of-bounds column placements
- [ ] `ReflowMapping` emitted by `Grid::resize` when reflow occurs
- [ ] `ImageCache::remap_placements(mapping)` translates placement row indices through reflow
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
- **ori_term existing** `grid/resize/mod.rs:303-379` — `reflow_cells` loop tracks `src_idx` per source row; adding row-range tracking is O(1) per-row overhead.
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

- [ ] Read reference impls to confirm resize/reflow behavior:
  - `~/projects/reference_repos/console_repos/ghostty/` — image cache resize handling in `src/terminal/kitty/` or equivalent
  - `~/projects/reference_repos/console_repos/wezterm/` — `term/src/terminalstate/image.rs` for cell-attachment model
  - `~/projects/reference_repos/console_repos/kitty/` — `kitty/graphics.py` for resize behavior
- [ ] Confirm the column-resize approach: **remove placements entirely outside the new grid bounds** (cell_col >= new_cols). No clamping, no runtime policy enum.
- [ ] Research reflow handling: confirm no reference impl remaps cache-coordinate placements through reflow (WezTerm avoids via cell attachment; Ghostty uses tracked pins). Our approach (return-value `ReflowMapping` from `Grid::resize`) is novel for the cache-coordinate model.

**Part B: Extract lifecycle submodule**

`cache/mod.rs` is 436 lines. Adding `on_resize` (~30 lines) + `remap_placements` (~40 lines) + `#[cfg(test)] mod tests;` would exceed 500 lines. Proactive split:

- [ ] Create `oriterm_core/src/image/cache/lifecycle.rs` with these methods extracted from `cache/mod.rs`:
  - `prune_scrollback` (currently lines 325-328)
  - `remove_placements_in_region` (currently lines 336-358)
  - `update_cell_coverage` (currently lines 395-410)
  - The new `on_resize` and `remap_placements` methods (implemented in 07.4)
- [ ] Update `cache/mod.rs`: add `mod lifecycle;`, remove extracted method bodies, add `#[cfg(test)] mod tests;`
- [ ] Create empty `oriterm_core/src/image/cache/tests.rs` with test-organization.md preamble
- [ ] Verify `cache/mod.rs` well under 500 lines after extraction
- [ ] `./build-all.sh` green — extraction is a refactor, no behavior change
- [ ] `./test-all.sh` green — existing tests in `image/tests.rs` pass unchanged

**Validation**: Research documented. Lifecycle submodule extracted. `cache/mod.rs` < 400 lines. All existing tests pass.

---

## 07.2 Write failing regression matrix tests (TDD: tests FIRST)

**File(s):** `oriterm_core/src/image/cache/tests.rs` (new), `oriterm_core/tests/image_lifecycle_matrix.rs` (new)

Per CLAUDE.md TDD discipline, failing tests are written FIRST.

### Unit tests in `cache/tests.rs`

**Column-bounds tests:**
- [ ] `on_resize_removes_placement_fully_outside_new_cols()` — place at col=90 spanning 10, resize to 80, assert removed
- [ ] `on_resize_preserves_placement_within_new_cols()` — place at col=5 spanning 10, resize to 80, assert survives
- [ ] `on_resize_preserves_partially_overlapping_placement()` — place at col=75 spanning 10, resize to 80, assert survives (renderer clips)
- [ ] `on_resize_prunes_orphaned_image_after_all_placements_removed()` — store+place at col=90, resize to 80, assert image data removed
- [ ] `on_resize_preserves_deferred_kitty_image_without_placements()` — store without placing, resize, assert image data NOT removed

**FixedPixels tests:**
- [ ] `on_resize_fixed_pixels_within_bounds_survives()` — FixedPixels placement at col=0, fits in new cols, survives
- [ ] `on_resize_fixed_pixels_out_of_bounds_removed()` — FixedPixels placement at col=8, new_cols=8, removed

**Reflow remapping tests:**
- [ ] `remap_placements_updates_stable_row_index_after_reflow()` — place image, build a `ReflowMapping` that moves row 5 → row 10, call `remap_placements`, assert placement's `cell_row` updated
- [ ] `remap_placements_removes_placement_when_source_row_evicted_by_reflow()` — place image at row that reflow merged/eliminated, assert placement removed
- [ ] `remap_placements_handles_row_split()` — single source row split into 2 new rows by reflow, placement maps to first new row
- [ ] `remap_placements_preserves_kitty_deferred_images()` — images with no placements survive remap unchanged

**Negative rendering pin:**
- [ ] `removed_placement_not_in_renderable_content()` — construct Term, place sixel at col=90, resize to 80, assert `RenderableContent::images` does NOT contain the removed placement

### Integration matrix in `tests/image_lifecycle_matrix.rs`

- [ ] Build table-driven test matrix:
  ```rust
  struct LifecycleScenario {
      name: &'static str,
      protocol: ImageProtocol,     // Sixel, Kitty, ITerm2
      sizing: PlacementSizingKind, // CellCount, FixedPixels
      mutation: GridMutation,      // ScrollbackEvict, Resize, Reflow, AltEnter, AltExit, EraseDisplay, EraseLine
      expected: PlacementState,    // Survives, Removed, Remapped
  }
  ```
- [ ] Enumerate: 3 protocols x 2 sizing modes x 7 mutations = 42 scenarios
- [ ] Self-verifying count assertion: `assert_eq!(count, 42);`
- [ ] **Validation**: on_resize and remap tests initially FAIL (methods don't exist). Existing-handler scenarios (scrollback, alt screen, ED, EL) should pass. If any fail, file via `/add-bug`.

---

## 07.3 Add ReflowMapping to Grid::resize

**File(s):** `oriterm_core/src/grid/resize/mod.rs`, `oriterm_core/src/grid/mod.rs`

This subsection adds the row-remap infrastructure that makes reflow-aware image placement remapping possible. The `reflow_cells` loop already tracks `src_idx` per source row — we add per-row boundary recording with O(1) overhead.

- [ ] Define `ReflowMapping` struct in `grid/resize/mod.rs`:
  ```rust
  /// Maps old absolute row indices to new row ranges after reflow.
  /// Built during `reflow_cells` with O(1) per-row overhead.
  #[derive(Debug, Clone)]
  pub struct ReflowMapping {
      /// For each source row index: (new_start_abs, new_end_abs) — half-open range.
      /// A source row that was merged into its predecessor has an empty range.
      pub rows: Vec<(usize, usize)>,
      /// Old total_evicted (before reflow) — needed to convert StableRowIndex → old abs row.
      pub old_total_evicted: u64,
      /// New total_evicted (after reflow) — needed to convert new abs row → StableRowIndex.
      pub new_total_evicted: u64,
  }
  ```
- [ ] Modify `reflow_cells()` (at `resize/mod.rs:303`) to build the `rows` vec:
  - At the top of the loop (`for (src_idx, src_row) in all_rows.iter().enumerate()`), record `let out_start = result.len();`
  - After the source row is fully processed (either appended to result or merged), record `let out_end = result.len();`
  - Push `(out_start, out_end)` to the mapping vec
  - Return the mapping alongside existing return values
- [ ] Modify `reflow_cols()` to thread the mapping through `apply_reflow_result()`:
  - `apply_reflow_result` adjusts `total_evicted` if scrollback rows were dropped during trim — capture both old and new `total_evicted` values for the mapping
- [ ] Modify `Grid::resize()` return type: `pub fn resize(...) -> Option<ReflowMapping>`:
  - Return `Some(mapping)` when `reflow: true` and column count changed (reflow actually ran)
  - Return `None` when `reflow: false` or column count unchanged (no reflow)
- [ ] Export `ReflowMapping` from `grid/mod.rs` (add to `pub use resize::ReflowMapping;`)
- [ ] Sibling tests in `grid/resize/tests.rs`:
  - `reflow_mapping_tracks_row_split()` — 80-col grid with a 120-char wrapped line, resize to 60 cols, verify the source row maps to 2 new rows
  - `reflow_mapping_tracks_row_merge()` — 40-col grid with two short lines that fit on one 80-col line, resize to 80, verify both source rows map to same new row
  - `reflow_mapping_none_when_no_reflow()` — resize with `reflow: false`, verify `None` returned
  - `reflow_mapping_none_when_cols_unchanged()` — resize with same cols but different rows, verify `None` returned
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green

**Validation**: `ReflowMapping` is produced correctly for row splits, merges, and no-change cases. Grid tests pass. No regressions.

---

## 07.4 Implement ImageCache::on_resize and remap_placements

**File(s):** `oriterm_core/src/image/cache/lifecycle.rs`

### on_resize (column-bounds removal)

- [ ] Add `pub(crate) fn on_resize(&mut self, new_cols: usize, _new_rows: usize)`:
  - `remove_placements_where(|p| p.cell_col >= new_cols)` — removes placements whose starting column is entirely outside the new grid
  - Partially overlapping placements (start < new_cols, end >= new_cols) survive — the renderer clips
  - `prune_if_orphaned(&affected_ids)` — targeted pruning only, preserving Kitty deferred images
  - `_new_rows` accepted for forward-compatibility but unused (row bounds handled by StableRowIndex eviction)
  - Mark `dirty = true` if any placements removed
- [ ] Unit tests from 07.2 now pass for column-bounds scenarios

### remap_placements (reflow-aware row remapping)

- [ ] Add `pub(crate) fn remap_placements(&mut self, mapping: &ReflowMapping)`:
  - For each placement, convert `cell_row: StableRowIndex` to old absolute row: `old_abs = cell_row.0 - mapping.old_total_evicted`
  - Look up in `mapping.rows[old_abs]` → `(new_start, new_end)`
  - If range is empty (source row was merged away): remove the placement
  - If range is non-empty: update `cell_row = StableRowIndex(mapping.new_total_evicted + new_start as u64)`
  - If `old_abs` is out of range (row was evicted before reflow): leave placement unchanged (already handled by `prune_scrollback`)
  - After processing all placements, `prune_if_orphaned` for any removed placements
  - Mark `dirty = true` if any placements changed
- [ ] Unit tests from 07.2 now pass for reflow remapping scenarios

**Validation**: All cache unit tests pass. `./test-all.sh` green.

---

## 07.5 Wire Term::resize with full reflow support

**File(s):** `oriterm_core/src/term/mod.rs` (around line 446)

- [ ] Modify `Term::resize` to capture the `Option<ReflowMapping>` from `Grid::resize`:
  ```rust
  let mapping = self.grid.resize(new_lines, new_cols, reflow);
  ```
- [ ] After grid resize + scrollback prune, call `on_resize` on primary cache:
  ```rust
  self.image_cache.on_resize(new_cols, new_lines);
  ```
- [ ] If reflow produced a mapping, call `remap_placements`:
  ```rust
  if let Some(ref mapping) = mapping {
      self.image_cache.remap_placements(mapping);
  }
  ```
- [ ] For the alt image cache — use the same condition as alt grid resize (`if let Some(alt) = &mut self.alt_grid`), matching alt grid EXISTENCE, NOT alt screen active:
  ```rust
  if let Some(cache) = &mut self.alt_image_cache {
      cache.on_resize(new_cols, new_lines);
      // Alt grid never reflows (reflow: false), so no remap needed
  }
  ```
- [ ] Sibling tests in `oriterm_core/src/term/tests.rs`:
  - `term_resize_removes_out_of_bounds_image_placement()` — sixel at col=90, resize to 80, assert removed
  - `term_resize_updates_alt_cache_when_alt_exists()` — ensure_alt_grid, place in alt, resize, assert removed
  - `term_resize_remaps_image_placement_through_reflow()` — place image, resize with reflow=true, assert placement's `cell_row` updated to follow content
  - `term_resize_without_reflow_skips_remap()` — place image, resize with reflow=false, assert `cell_row` unchanged
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green

**Validation**: Term-level tests pass. Existing teseq tests pass. Integration matrix from 07.2 passes for resize + reflow scenarios.

---

## 07.6 Wire cell-metric plumbing (app → mux → Term)

**File(s):** `oriterm_mux/src/backend/mod.rs`, `oriterm_mux/src/pane/io_thread/handler.rs`, `oriterm/src/app/chrome/resize.rs`, `oriterm/src/app/mod.rs`

This subsection wires the end-to-end path so `Term::set_cell_dimensions` (at `image_config.rs:17`) gets called in production when font size or DPI changes. Currently it has zero production callers.

**Approach**: Extend `ImageConfig` with `cell_width`/`cell_height` fields. This is the cleanest path — `SetImageConfig` is already the command for image-related config, cell dimensions are logically grouped with image rendering, and the embedded + daemon mode paths are already built.

- [ ] Add `cell_width: u16` and `cell_height: u16` to `ImageConfig` in `oriterm_mux/src/backend/mod.rs`
- [ ] Update `MuxPdu::SetImageConfig` in `oriterm_mux/src/protocol/messages.rs` to include the two new fields
- [ ] Update `PaneIoCommand::SetImageConfig` handler in `oriterm_mux/src/pane/io_thread/handler.rs`:
  ```rust
  self.terminal.set_cell_dimensions(config.cell_width, config.cell_height);
  ```
- [ ] In `oriterm/src/app/chrome/resize.rs` `sync_grid_layout()`: after resizing panes, also send updated cell metrics:
  ```rust
  let cell = renderer.cell_metrics();
  let mut config = self.config.terminal.image_config();
  config.cell_width = cell.width.round() as u16;
  config.cell_height = cell.height.round() as u16;
  mux.set_image_config(pane_id, config);
  ```
- [ ] In `oriterm/src/app/mod.rs` `handle_dpi_change()`: after font re-rasterization, send updated cell metrics through the same path
- [ ] Update all existing `ImageConfig` construction sites (6 call sites in app layer) to include cell dimensions — use 0 as initial value before font rasterization completes, or pass actual metrics if the renderer is already initialized
- [ ] Sibling test: `term_set_cell_dimensions_updates_fixed_pixels_coverage()` — already exists in `term/tests.rs`, verify it still passes
- [ ] Integration test: place a FixedPixels sixel image, change cell dimensions via the mux command path, verify `cols`/`rows` updated
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green (including Windows cross-compile — `ImageConfig` is in the wire protocol)
- [ ] Close BUG-08-9 in `plans/bug-tracker/section-08-core-terminal.md`

**Validation**: `set_cell_dimensions` now has a production caller. FixedPixels placements get correct coverage after font/DPI changes. BUG-08-9 resolved.

---

## 07.R Third Party Review Findings

- [x] `[TPR-07-001-codex][high]` `plans/spec-conformance/section-07-image-lifecycle-correctness.md:211` — Fill the GAP in FixedPixels cell-metric plumbing before relying on resize ordering.
  Resolved: Rescoped section to include full cell-metric plumbing in subsection 07.6 (app → mux → Term). BUG-08-9 filed and will be closed by 07.6. No more scoping out.
- [x] `[TPR-07-002-codex][medium]` `plans/spec-conformance/section-07-image-lifecycle-correctness.md:6` — Resolve the DRIFT between the frontmatter contract and the blocked reflow scope.
  Resolved: Rescoped section to include full reflow remapping in subsections 07.3 + 07.4 + 07.5. No more reflow scoping out — frontmatter, goal, success criteria, and body all agree.

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
- [ ] 07.3: `ReflowMapping` struct defined in `grid/resize/mod.rs`
- [ ] 07.3: `reflow_cells()` builds row-range mapping with O(1) per-row overhead
- [ ] 07.3: `Grid::resize()` returns `Option<ReflowMapping>`
- [ ] 07.3: Grid reflow tests pass (row split, row merge, no-reflow cases)
- [ ] 07.4: `ImageCache::on_resize(new_cols, new_rows)` implemented in `lifecycle.rs`
- [ ] 07.4: Uses `remove_placements_where` + targeted `prune_if_orphaned` (NOT full orphan sweep)
- [ ] 07.4: `ImageCache::remap_placements(mapping)` implemented — translates StableRowIndex through ReflowMapping
- [ ] 07.4: All cache unit tests pass
- [ ] 07.5: `Term::resize` captures `Option<ReflowMapping>` from `Grid::resize`
- [ ] 07.5: Primary cache gets `on_resize` + `remap_placements` (when mapping present)
- [ ] 07.5: Alt cache gets `on_resize` only (alt grid never reflows)
- [ ] 07.5: Alt cache condition: `if alt_image_cache exists` (matches alt grid existence, NOT active)
- [ ] 07.5: Term-level tests pass including reflow remapping
- [ ] 07.6: `ImageConfig` extended with `cell_width`/`cell_height`
- [ ] 07.6: IO thread handler calls `set_cell_dimensions`
- [ ] 07.6: `sync_grid_layout()` sends cell metrics after resize
- [ ] 07.6: `handle_dpi_change()` sends cell metrics after font re-rasterization
- [ ] 07.6: All 6 existing `ImageConfig` construction sites updated
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
