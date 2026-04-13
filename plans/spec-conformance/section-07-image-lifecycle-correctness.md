---
section: "07"
title: "Image Lifecycle Correctness"
status: not-started
reviewed: false
goal: "Add the missing resize handler to `oriterm_core/src/image/cache/mod.rs` and verify image placements survive every non-reflow grid transformation correctly: scrollback eviction, grid resize (column width and row count changes without reflow), alt-screen toggle, ED/EL erase. Reflow invalidation is documented and scoped out — StableRowIndex is eviction-stable but not reflow-stable, and remapping requires a future cross-cutting change."
success_criteria:
  - "`ImageCache::on_resize(new_cols, new_rows)` exists and removes image placements whose column extent is entirely outside the new grid bounds (currently MISSING — only `prune_scrollback` and `remove_placements_in_region` exist)"
  - "Image placement regression matrix exists: every image protocol (sixel, kitty, iTerm2) x every sizing mode (CellCount, FixedPixels) x every non-reflow grid transformation (scrollback eviction, grid resize, alt-screen enter, alt-screen exit, ED, EL) — 36 scenarios verified by `oriterm_core/src/image/cache/tests.rs` and `oriterm_core/tests/image_lifecycle_matrix.rs`"
  - "`Term::resize` calls `image_cache.on_resize(...)` (and `alt_image_cache.on_resize(...)` if the alt image cache exists — matching the alt grid resize condition, NOT gated on alt screen being active)"
  - "FixedPixels placement handling is scoped to grid-dimension-only resizes (window resize, DECCOLM toggle) where cell metrics are unchanged. Font-size/DPI-driven resizes that change cell metrics are a cross-crate plumbing gap: `Term::set_cell_dimensions` has no production caller (only tests). This gap is documented and tracked; section 07 ensures `on_resize` works correctly when `cols`/`rows` are already up-to-date"
  - "Reflow invalidation is explicitly scoped out with a concrete blocked-by link to a future section, and the non-reflow resize path is fully tested"
  - "Placement lifecycle methods are extracted into `cache/lifecycle.rs` to keep `cache/mod.rs` under 500 lines"
  - "New cache tests live in `cache/tests.rs` (NOT `image/tests.rs` which is already 854 lines)"
  - "At least one negative rendering pin: `RenderableContent::images` does NOT contain a removed placement after resize"
  - "Existing image cache tests in `oriterm_core/src/image/tests.rs` still pass without modification"
  - "Existing teseq tests covering image scenarios still pass"
  - "`./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release"
  - "Section's mission criterion connection: contributes to **Image lifecycle correct under resize/reflow/scrollback/alt-screen** mission criterion in 00-overview.md"
inspired_by:
  - "ori_term existing `oriterm_core/src/image/cache/mod.rs:325-358` — `prune_scrollback` and `remove_placements_in_region` are the existing handlers; `on_resize` follows the same pattern of `remove_placements_where` + targeted `prune_if_orphaned`"
  - "Ghostty image cache resize — placements with stable row indices survive but column-out-of-bounds placements are removed"
  - "WezTerm — attaches images to cells and reflows through screen rewrap; ori_term uses cache-coordinate approach instead"
depends_on: ["04"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "07.1"
    title: "Research reference impl resize behavior and extract lifecycle submodule"
    status: not-started
  - id: "07.2"
    title: "Write failing regression matrix tests (TDD: tests FIRST)"
    status: not-started
  - id: "07.3"
    title: "Implement ImageCache::on_resize"
    status: not-started
  - id: "07.4"
    title: "Wire Term::resize to call image_cache.on_resize"
    status: not-started
  - id: "07.5"
    title: "Document reflow invalidation scope and add reflow-skip tests"
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
**Goal:** Add the missing image cache resize handler. Pass 1 confirmed `oriterm_core/src/image/cache/mod.rs` has `prune_scrollback` and `remove_placements_in_region` but NO resize handler. When the grid resizes, image placements with `cell_row: StableRowIndex` survive (because row indices are stable across eviction) but their `cell_col` may become out-of-bounds — and there's currently no code that handles this. Additionally, `FixedPixels` placements may have stale `cols`/`rows` after a cell-metric-driven resize (font size change). The `keller` notcurses-demo scene resizes mid-test and breaks any image lifecycle bug, so this fix is a prerequisite for section 21+24 (notcurses-demo harness + full-pass).

**Critical limitation — reflow invalidation:** `Grid::resize(..., reflow: true)` completely rewrites row topology via `reflow_cols` → `apply_reflow_result`. During reflow, `scrollback.clear()` is called (line 268 of `grid/resize/mod.rs`) and scrollback is rebuilt from scratch with merged/split rows, but `total_evicted` is NOT adjusted. This means `StableRowIndex` values computed before reflow point to completely wrong content rows after reflow. No reference impl (Ghostty, WezTerm, Kitty) solves this by remapping cache-coordinate placements through reflow — WezTerm avoids the problem entirely by attaching images to cells (so they move with text during rewrap). Remapping StableRowIndex through reflow requires Grid::resize to emit a row-remap table, which is a cross-cutting change. This section explicitly scopes reflow out and handles only the non-reflow resize path (column width changes with `reflow: false`, row count changes, and DECCOLM 80↔132 toggles). Reflow-aware image placement remapping is tracked as a blocked dependency for a future section.

**Success Criteria:**
- [ ] `ImageCache::on_resize(new_cols, new_rows)` exists and removes out-of-bounds column placements
- [ ] Regression matrix covers every image protocol x every sizing mode x every non-reflow grid transformation
- [ ] `Term::resize` invokes `image_cache.on_resize` (and alt cache if `alt_image_cache` exists)
- [ ] FixedPixels cell-metric gap documented (no production caller for `set_cell_dimensions`)
- [ ] Reflow invalidation documented; non-reflow path fully tested
- [ ] `cache/lifecycle.rs` extracted; `cache/mod.rs` stays under 500 lines
- [ ] New cache tests in `cache/tests.rs` per test-organization.md
- [ ] At least one negative rendering pin via `RenderableContent::images`
- [ ] Existing image tests + teseq tests still pass
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green
- [ ] Connects to mission criterion: **Image lifecycle correct under resize/reflow/scrollback/alt-screen**

**Context:** The image cache is the SSOT for image placements (cache-coordinate model: `cell_col: usize` + `cell_row: StableRowIndex`). It handles scrollback eviction (`prune_scrollback` at `cache/mod.rs:325-328`) and ED/EL erase (`remove_placements_in_region` at `cache/mod.rs:336-358`), but it has no handler for grid resize. When the user resizes the window (or DECCOLM toggles between 80↔132 columns), the grid dimensions change and image placements that referenced columns beyond the new width are now invalid.

**Kitty placeholder mode note:** Kitty's unicode placeholder protocol (`U=1`) writes `U+10EEEE` characters into grid cells (see `image/mod.rs:29` constant `KITTY_PLACEHOLDER`). For placeholder-mode placements, the SSOT for location is the grid cells themselves, not the cache's `cell_col`/`cell_row`. Reflow handles these automatically because the placeholder characters move with the text. Section 13 (Kitty Graphics Protocol) owns placeholder-mode correctness; this section handles cache-coordinate-based protocols (sixel, iTerm2, and non-placeholder Kitty).

**Reference implementations:**
- **ori_term existing** `cache/mod.rs:268-286` — `remove_placements_where` + targeted `prune_if_orphaned` is the canonical pattern. `on_resize` follows the same shape.
- **ori_term existing** `cache/mod.rs:395-410` — `update_cell_coverage(cell_w, cell_h)` recomputes `cols`/`rows` for FixedPixels placements when cell dimensions change.
- **Ghostty** — derives clamped rects at use time rather than maintaining a runtime policy enum. Placements with stable row indices survive but out-of-bounds columns are removed.
- **WezTerm** — attaches images to cells and reflows through screen rewrap. Different architecture (cell-based) vs ori_term (cache-coordinate-based).

**Depends on:** Section 04 (the spec_chain harness exists; section 07 uses it for the regression matrix tests in 07.2).

**Cross-section links:**
- Section 13 (Kitty Graphics Protocol) — placeholder-mode Kitty has different SSOT semantics; this section handles non-placeholder Kitty only.
- Section 12 (Sixel) and Section 14 (iTerm2) — both use cache-coordinate placements; this section's resize handler covers them.
- Section 26 (Historical Vector Stacks) — depends on section 07 for `ImageCache::on_resize`.
- Future section (not yet written) — reflow-aware image placement remapping requires `Grid::resize` to emit a row-remap table.

---

## 07.1 Research reference impl resize behavior and extract lifecycle submodule

**File(s):** `oriterm_core/src/image/cache/mod.rs` (extract from), `oriterm_core/src/image/cache/lifecycle.rs` (new)

This subsection has two parts: (a) research the correct resize behavior from reference implementations to confirm the approach (remove out-of-bounds, no policy enum, no runtime configuration), and (b) proactively split `cache/mod.rs` (currently 437 lines) before adding `on_resize` would push it past the 500-line limit.

**Part A: Research**

- [ ] Read reference impls to confirm resize removal behavior:
  - `~/projects/reference_repos/console_repos/ghostty/` — search for image cache resize handling in `src/terminal/kitty/` or equivalent
  - `~/projects/reference_repos/console_repos/wezterm/` — `term/src/terminalstate/image.rs` or equivalent for how images are handled on resize
  - `~/projects/reference_repos/console_repos/kitty/` — `kitty/graphics.py` for kitty's own image resize behavior
- [ ] Confirm the approach: **remove placements entirely outside the new grid bounds** (column extent fully >= new_cols). No clamping, no runtime policy enum, no design doc. Reference impls do not use a runtime resize policy — this is a fixed behavior, not a configurable choice.

**Rationale for no ResizePolicy enum:** No reference implementation (Ghostty, WezTerm, Kitty) uses a runtime resize policy enum for image placement bounds. Ghostty derives clamped rects at use time. WezTerm attaches images to cells. Creating a 3-variant `ResizePolicy` enum would be speculative generalization (WASTE per impl-hygiene.md "No Cargo-Culted Design Patterns"). The correct behavior is determined by research, not by parameterization.

**Part B: Extract lifecycle submodule**

`cache/mod.rs` is 437 lines. Adding `on_resize` (estimated ~30 lines) plus the `#[cfg(test)] mod tests;` declaration would approach 470+ lines. To prevent crossing the 500-line hard limit and maintain single-responsibility:

- [ ] Create `oriterm_core/src/image/cache/lifecycle.rs` containing these methods extracted from `cache/mod.rs`:
  - `prune_scrollback` (currently lines 325-328)
  - `remove_placements_in_region` (currently lines 336-358)
  - `update_cell_coverage` (currently lines 395-410)
  - The new `on_resize` method (implemented in 07.3)
- [ ] Update `cache/mod.rs`:
  - Add `mod lifecycle;` declaration
  - Remove the extracted method bodies
  - Add `#[cfg(test)] mod tests;` at the bottom
- [ ] Create empty `oriterm_core/src/image/cache/tests.rs` with the test-organization.md preamble (imports from `super::`)
- [ ] Verify `cache/mod.rs` is well under 500 lines after extraction
- [ ] `./build-all.sh` green — extraction is a refactor, no behavior change
- [ ] `./test-all.sh` green — existing tests in `image/tests.rs` still pass (they test through `ImageCache` public API, which is unchanged)

**Validation**: Reference research documented in checklist comments. Lifecycle submodule extracted. `cache/mod.rs` < 450 lines. All existing tests pass.

---

## 07.2 Write failing regression matrix tests (TDD: tests FIRST)

**File(s):** `oriterm_core/src/image/cache/tests.rs` (new), `oriterm_core/tests/image_lifecycle_matrix.rs` (new)

Per CLAUDE.md TDD discipline and tests.md matrix testing rule, failing tests are written FIRST. The implementation in 07.3/07.4 makes them pass.

**Test location rationale:** Existing image tests live in `oriterm_core/src/image/tests.rs` (854 lines, 48 tests). Per test-organization.md, cache-specific tests belong in `cache/tests.rs` (one `tests.rs` per source file). The existing `image/tests.rs` tests cover the `image/mod.rs` module (ImageCache basics, placement, eviction — they call through the public API). New resize-specific tests targeting `cache/lifecycle.rs` methods go in `cache/tests.rs`. The integration-level matrix goes in `oriterm_core/tests/image_lifecycle_matrix.rs`.

### Unit tests in `cache/tests.rs`

- [ ] `on_resize_removes_placement_fully_outside_new_cols()` — place an image at col=90 spanning 10 cols (90..99), resize to 80 cols, assert placement removed
- [ ] `on_resize_preserves_placement_within_new_cols()` — place an image at col=5 spanning 10 cols (5..14), resize to 80 cols, assert placement survives
- [ ] `on_resize_preserves_partially_overlapping_placement()` — place an image at col=75 spanning 10 cols (75..84), resize to 80 cols, assert placement survives (partially visible is kept; the renderer clips)
- [ ] `on_resize_prunes_orphaned_image_after_all_placements_removed()` — store image, place it at col=90, resize to 80, assert image data also removed (prune_if_orphaned)
- [ ] `on_resize_preserves_deferred_kitty_image_without_placements()` — store image with no placements (Kitty `a=t, U=1`), resize, assert image data NOT removed (prune_if_orphaned only checks affected IDs)
- [ ] `on_resize_fixed_pixels_placement_within_bounds_survives()` — create FixedPixels placement at col=0 spanning 5 cells (already computed by test setup calling `set_cell_dimensions`). Call `on_resize(10, 24)` — placement at cols 0..4 fits in 10 cols, survives.
- [ ] `on_resize_fixed_pixels_placement_out_of_bounds_removed()` — create FixedPixels placement at col=8 spanning 5 cells. Call `on_resize(8, 24)` — placement at cell_col=8 >= new_cols=8, removed.

### Negative rendering pin in `cache/tests.rs`

- [ ] `removed_placement_not_in_renderable_content()` — construct a `Term`, place a sixel image at col=90, resize to 80 cols, call `renderable_content_into()`, assert `RenderableContent::images` does NOT contain the removed placement's image_id. This is the semantic pin: the renderer physically cannot see a removed placement.

### Integration matrix in `tests/image_lifecycle_matrix.rs`

- [ ] Build a table-driven test matrix:
  ```rust
  struct LifecycleScenario {
      name: &'static str,
      protocol: ImageProtocol,     // Sixel, Kitty, ITerm2
      sizing: PlacementSizingKind, // CellCount, FixedPixels
      mutation: GridMutation,      // ScrollbackEvict, Resize, AltEnter, AltExit, EraseDisplay, EraseLine
      // NOTE: Reflow is explicitly excluded — see 07.5
      expected: PlacementState,    // Survives, Removed
  }
  ```
- [ ] Enumerate the matrix: 3 protocols x 2 sizing modes x 6 mutations = 36 scenarios (Reflow excluded — 7th mutation omitted with documented rationale)
- [ ] Each scenario: instantiate harness, place an image via the protocol, apply the mutation, assert the expected state
- [ ] Include self-verifying count assertion per tests.md:
  ```rust
  assert_eq!(count, PROTOCOLS.len() * SIZING_MODES.len() * MUTATIONS.len());
  ```
- [ ] **Validation**: all matrix tests initially FAIL on `on_resize` scenarios (method doesn't exist yet). Other scenarios (scrollback evict, alt screen, ED, EL) should already pass — they exercise existing handlers. If any existing-handler scenario fails, that is a bug to file via `/add-bug`.

---

## 07.3 Implement ImageCache::on_resize

**File(s):** `oriterm_core/src/image/cache/lifecycle.rs`

The implementation follows the existing `remove_placements_where` + `prune_if_orphaned` pattern used by `prune_scrollback` (line 325-328) and `remove_placements_in_region` (line 336-358).

- [ ] Add `pub(crate) fn on_resize(&mut self, new_cols: usize, _new_rows: usize)` to `ImageCache` in `lifecycle.rs`:
  - Walk every placement. Compute column extent: `start_col..start_col + cols - 1`
  - Remove placements whose column extent is fully >= `new_cols` (i.e., `cell_col >= new_cols`)
  - Partially overlapping placements (start < new_cols, end >= new_cols) are preserved — the renderer clips them. This matches Ghostty behavior.
  - Use `remove_placements_where(|p| p.cell_col >= new_cols)` for the removal predicate
  - After processing, call `prune_if_orphaned(&affected_ids)` — targeted pruning only, NOT a full `prune_orphan_images` sweep. This preserves Kitty deferred images with zero placements (consistent with `prune_scrollback` and `remove_placements_in_region`).
  - `_new_rows` is accepted for forward compatibility but unused — row bounds are handled by StableRowIndex eviction in `prune_scrollback`, not by resize.
- [ ] Mark `dirty = true` if any placements were removed.
- [ ] **Validation**: unit tests from 07.2 now pass. Run `./test-all.sh` — all green.

---

## 07.4 Wire Term::resize to call image_cache.on_resize

**File(s):** `oriterm_core/src/term/mod.rs` (around line 446, the `resize` method)

- [ ] In `Term::resize`, after the grid is resized and scrollback pruning is done, add:
  ```rust
  self.image_cache.on_resize(new_cols, new_lines);
  ```
  This goes after line 465 (after `prune_scrollback` call for primary grid).

- [ ] For the alt image cache — use the same condition as the alt grid resize: `if let Some(alt) = &mut self.alt_grid`. The alt grid is resized whenever it EXISTS (line 470: `if let Some(alt) = &mut self.alt_grid`), NOT only when alt screen is active. The alt cache must follow the same condition:
  ```rust
  if let Some(cache) = &mut self.alt_image_cache {
      cache.on_resize(new_cols, new_lines);
  }
  ```
  This goes inside the existing `if let Some(alt) = &mut self.alt_grid` block, after the alt scrollback pruning (after line 479).

- [ ] **FixedPixels note:** `Term::set_cell_dimensions` (at `image_config.rs:17`) calls `update_cell_coverage` but has NO production caller — only test code calls it. This means font-size/DPI-driven resizes do NOT currently update FixedPixels coverage in production. This is a cross-crate plumbing gap (app → mux → Term) tracked for a future section. Section 07's `on_resize` assumes `cols`/`rows` are already correct for grid-dimension-only resizes (window resize, DECCOLM toggle) where cell metrics don't change. Add a doc comment on `on_resize` documenting this assumption.

- [ ] Sibling test in `oriterm_core/src/term/tests.rs` (or the appropriate test file for term/mod.rs):
  - `term_resize_invokes_image_cache_on_resize()` — install a sixel placement at col=90, resize the grid to 80 cols, assert the placement is removed from the cache
  - `term_resize_updates_alt_cache_when_alt_exists()` — ensure_alt_grid, place an image in the alt cache, resize, assert alt cache placement is removed

- [ ] **Validation**: tests pass. Existing teseq tests pass. `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green.

---

## 07.5 Document reflow invalidation scope and add reflow-skip tests

**File(s):** `oriterm_core/src/image/cache/lifecycle.rs` (doc comment), `oriterm_core/src/image/cache/tests.rs` (reflow-skip tests)

This subsection explicitly documents the reflow limitation and adds tests that prove the non-reflow path works correctly while reflow is scoped out.

**The reflow problem (documented for future section):**

`Grid::resize(new_lines, new_cols, reflow: true)` calls `reflow_cols` which:
1. Collects ALL rows (scrollback + visible) into a flat vec (`collect_all_rows`, line 229)
2. Rewrites them cell-by-cell into new-width rows (`reflow_cells`, line 303)
3. Clears scrollback and rebuilds it from the rewritten rows (`apply_reflow_result`, line 268)

During this process, `total_evicted` is NOT adjusted (unlike `Grid::reset` which does `total_evicted += scrollback.len()` at line 206 of `grid/mod.rs`). This means:
- Old `StableRowIndex` values (computed as `total_evicted + absolute_row_index`) now map to wrong rows, because absolute row indices changed (rows were merged/split/reordered by reflow)
- There is no remap table emitted by `reflow_cols` — the mapping from old-row to new-row is not tracked

Solving this requires one of:
- (a) `Grid::resize` emitting a row-remap callback/table that `ImageCache` can use to translate old StableRowIndex → new StableRowIndex
- (b) Switching to WezTerm's cell-attachment model (images move with cells during reflow)
- (c) Clearing all image placements on reflow (lossy but simple)

All three options are cross-cutting changes. This section takes approach: **document the limitation, test the non-reflow path, and scope reflow to a future section**.

- [ ] Add a doc comment on `ImageCache::on_resize` explicitly stating:
  ```
  /// NOTE: This method handles column-bounds removal only. It does NOT
  /// handle StableRowIndex remapping after reflow. When `Grid::resize` is
  /// called with `reflow: true`, row topology changes and existing
  /// StableRowIndex values may point to wrong content rows. Reflow-aware
  /// placement remapping requires Grid::resize to emit a row-remap table
  /// (not yet implemented). Until then, image placements may be
  /// positionally incorrect after a reflow event.
  ```

- [ ] Add a `<!-- blocked-by: future-section-reflow-image-remap -->` marker in this section file for tracking.

- [ ] Add reflow-skip tests in `cache/tests.rs`:
  - `on_resize_correct_without_reflow()` — construct a Term, place images, call `term.resize(new_lines, new_cols, false)` (reflow disabled), verify placements are correctly handled
  - `on_resize_with_reflow_images_may_drift()` — construct a Term, place images at known stable row indices, call `term.resize(new_lines, new_cols, true)` (reflow enabled), verify no panic/crash (defensive), document that placement positions may be stale. This is a "known limitation" test, not a correctness test — it proves the code doesn't crash or corrupt state even when placements are positionally stale.

- [ ] **Validation**: tests pass. The reflow limitation is documented, not hidden.

<!-- blocked-by: future-section-reflow-image-remap -->

---

## 07.R Third Party Review Findings

- None.

---

## 07.N Completion Checklist

**TDD ordering enforced:** 07.2 (failing tests) BEFORE 07.3/07.4 (implementation). 07.1 (research + refactor) is prerequisite for both.

- [ ] 07.1: Reference impl research completed — resize behavior confirmed (remove out-of-bounds, no policy enum)
- [ ] 07.1: `cache/lifecycle.rs` extracted from `cache/mod.rs` — `prune_scrollback`, `remove_placements_in_region`, `update_cell_coverage` moved
- [ ] 07.1: `cache/mod.rs` < 450 lines after extraction
- [ ] 07.1: `cache/tests.rs` created with `#[cfg(test)] mod tests;` in `cache/mod.rs`
- [ ] 07.1: Existing `image/tests.rs` tests still pass (no modification needed)
- [ ] 07.2: Failing test matrix written — cache unit tests (7+ tests in `cache/tests.rs`) + integration matrix (36 scenarios in `tests/image_lifecycle_matrix.rs`)
- [ ] 07.2: On_resize-specific tests initially FAIL (method doesn't exist yet)
- [ ] 07.2: Existing-handler scenarios (scrollback, alt screen, ED, EL) pass — if any fail, file via `/add-bug`
- [ ] 07.2: Negative rendering pin: `removed_placement_not_in_renderable_content()` written
- [ ] 07.3: `ImageCache::on_resize(new_cols, new_rows)` implemented in `lifecycle.rs`
- [ ] 07.3: Uses `remove_placements_where` + targeted `prune_if_orphaned` (NOT full orphan sweep)
- [ ] 07.3: All cache unit tests from 07.2 now pass
- [ ] 07.4: `Term::resize` calls `image_cache.on_resize` after grid resize + scrollback prune
- [ ] 07.4: Alt cache condition: `if let Some(cache) = &mut self.alt_image_cache` inside `if let Some(alt) = &mut self.alt_grid` — matches alt grid existence condition, NOT alt screen active condition
- [ ] 07.4: FixedPixels cell-metric gap documented in `on_resize` doc comment (`set_cell_dimensions` has no production caller)
- [ ] 07.4: Term-level tests pass
- [ ] 07.5: Reflow invalidation documented in `on_resize` doc comment
- [ ] 07.5: Reflow-skip tests pass (non-reflow correct, reflow doesn't crash)
- [ ] **Matrix dimensions**: 3 protocols x 2 sizing modes x 6 non-reflow mutations = 36 scenarios + self-verifying count assertion
- [ ] **Semantic pin**: `on_resize_removes_placement_fully_outside_new_cols` — ONLY passes with the new on_resize behavior, not accidentally passing due to existing code
- [ ] **Negative pin**: `removed_placement_not_in_renderable_content` — rejected from render output, not just from internal cache state
- [ ] All 36 matrix scenarios pass
- [ ] Existing image cache tests (`image/tests.rs`) pass without modification
- [ ] Existing teseq tests pass
- [ ] Alloc regression unchanged
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release
- [ ] Section frontmatter `status` -> `complete`
- [ ] `00-overview.md` Quick Reference + mission criteria updated
- [ ] `index.md` section 07 status updated
- [ ] Cross-links verified: sections 12, 13, 14, 26 reference section 07
- [ ] `/tpr-review` passed
- [ ] `/impl-hygiene-review last commit` passed (after `/tpr-review` is clean)

**Exit Criteria:** Image cache has on_resize handler; regression matrix covers every protocol x every sizing mode x every non-reflow grid mutation; reflow limitation documented with blocked-by marker; no regressions in existing image tests; ready for sections 12-14 (image stack) and section 21 (notcurses-demo harness) to use.
