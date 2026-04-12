---
section: "07"
title: "Image Lifecycle Correctness"
status: not-started
reviewed: false
goal: "Add the missing resize/reflow handler to `oriterm_core/src/image/cache/mod.rs` and verify image placements survive every grid transformation correctly: scrollback eviction, grid resize, column reflow, alt-screen toggle, ED/EL erase."
success_criteria:
  - "`ImageCache::on_resize(new_cols, new_rows)` exists and handles image placements that become out-of-bounds after a grid resize (currently MISSING — Pass 1 confirmed only `prune_scrollback` and `remove_placements_in_region` exist)"
  - "Image placement regression matrix exists: every image protocol (sixel, kitty, iTerm2) × every grid transformation (scrollback eviction, grid resize, column reflow, alt-screen enter, alt-screen exit, ED, EL) — verified by `oriterm_core/src/image/cache/tests.rs`"
  - "Term::resize calls `image_cache.on_resize(...)` (and `alt_image_cache.on_resize(...)` if active) so the resize event reaches the cache"
  - "Existing image cache tests in `oriterm_core/src/image/cache/tests.rs` and `oriterm_core/src/image/tests.rs` still pass without modification"
  - "Existing teseq tests covering image scenarios still pass"
  - "`./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release"
  - "Section's mission criterion connection: contributes to **Image lifecycle correct under resize/reflow/scrollback/alt-screen** mission criterion"
inspired_by:
  - "ori_term existing `oriterm_core/src/image/cache/mod.rs` — `prune_scrollback` and `remove_placements_in_region` are the existing handlers; `on_resize` follows the same pattern"
  - "Ghostty image cache resize behavior — placements with stable row indices survive but column-out-of-bounds placements are removed or clamped"
depends_on: ["04"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "07.1"
    title: "Define resize policy: clamp vs remove vs both"
    status: not-started
  - id: "07.2"
    title: "Implement ImageCache::on_resize"
    status: not-started
  - id: "07.3"
    title: "Wire Term::resize to call image_cache.on_resize"
    status: not-started
  - id: "07.4"
    title: "Build the image lifecycle regression matrix"
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
**Goal:** Add the missing image cache resize handler. Pass 1 confirmed `oriterm_core/src/image/cache/mod.rs` has `prune_scrollback` and `remove_placements_in_region` but NO resize handler. When the grid resizes, image placements with `cell_row: StableRowIndex` survive (because row indices are stable) but their `cell_col` may become out-of-bounds — and there's currently no code that handles this. The `keller` notcurses-demo scene resizes mid-test and breaks any image lifecycle bug, so this fix is a prerequisite for section 21+24 (notcurses-demo harness + full-pass).

**Success Criteria:**
- [ ] `ImageCache::on_resize(new_cols, new_rows)` exists and handles out-of-bounds column placements
- [ ] Regression matrix covers every image protocol × every grid transformation
- [ ] `Term::resize` invokes `image_cache.on_resize` (and alt cache if active)
- [ ] Existing image tests still pass
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green
- [ ] Connects to mission criterion: **Image lifecycle correct under resize/reflow/scrollback/alt-screen**

**Context:** The image cache is the SSOT for image placements. It handles scrollback eviction (placements scroll off the top) and ED/EL erase (placements in the erased region are removed), but it has no handler for grid resize. When the user resizes the window (or DECCOLM toggles between 80↔132 columns), the grid reflows: rows may be shifted, columns may shrink, and image placements that referenced columns beyond the new width are now invalid. Pass 1 found this gap; Codex Q2 confirmed it's a Section 02-foundation prerequisite (now section 07 after the foundation split).

**Reference implementations:**
- **ori_term existing** `oriterm_core/src/image/cache/mod.rs:330-358` — `remove_placements_in_region(top, bottom, left, right)` is the existing pattern. `on_resize` follows the same shape but with full-cache iteration.
- **ori_term existing** `oriterm_core/src/term/handler/helpers.rs` — `prune_scrollback(threshold)` call site, the precedent for "image cache observes a grid event."
- **Ghostty** image cache — placements with stable row indices survive grid resize but out-of-bounds columns are clamped or removed depending on the protocol.

**Depends on:** Section 04 (the spec_chain harness exists; section 07 uses it for the regression matrix tests in 07.4).

---

## 07.1 Define resize policy: clamp vs remove vs both

**File(s):** `oriterm_core/src/image/cache/resize_policy.rs` (new), `plans/spec-conformance/specs/image-resize-policy.md` (new design doc)

When a grid resize makes an image placement out-of-bounds, there are three options: (a) remove the placement entirely, (b) clamp the placement's bounding box to the new grid, or (c) hybrid (remove if entirely outside, clamp if partially overlapping). The decision affects what `keller`/`trans`/`luigi` notcurses scenes look like after a resize.

- [ ] Research the policy across reference impls:
  - Read `~/projects/reference_repos/console_repos/wezterm/term/src/terminalstate/sixel.rs` and `term/src/terminalstate/kitty.rs` resize code (if any)
  - Read `~/projects/reference_repos/console_repos/kitty/kitty/graphics.py` resize behavior
  - Read `~/projects/reference_repos/console_repos/notcurses/src/lib/sprite.c` for how notcurses tracks placements across resize
- [ ] Based on the research, write a design decision doc at `plans/spec-conformance/specs/image-resize-policy.md` explaining the chosen policy, the alternatives considered, and the rationale.
- [ ] Recommended policy (subject to Codex / TPR review): hybrid — remove placements entirely outside the new grid; clamp placements that partially overlap.
- [ ] Define the policy as an enum:
  ```rust
  pub enum ResizePolicy {
      RemoveOutOfBounds,   // any placement extending beyond the new grid is removed
      ClampToBounds,       // placements are clipped to the new grid extent
      Hybrid,              // remove fully-outside, clamp partially-overlapping (recommended)
  }
  ```
- [ ] **Validation**: design doc committed; policy enum exists.

---

## 07.2 Implement ImageCache::on_resize

**File(s):** `oriterm_core/src/image/cache/mod.rs`, sibling tests

- [ ] Add `pub fn on_resize(&mut self, new_cols: usize, new_rows: usize, policy: ResizePolicy)` to `ImageCache`.
- [ ] Walk every placement in the cache. For each:
  - Compute the placement's column extent (e.g., `start_col..start_col + cols_wide`)
  - If the column extent is fully outside `0..new_cols`, remove the placement (per `RemoveOutOfBounds` and `Hybrid`)
  - If the column extent partially overlaps `0..new_cols` AND the policy is `ClampToBounds` or `Hybrid`, clamp the extent
- [ ] After processing all placements, call `prune_orphan_images()` (the existing helper that removes images with zero remaining placements).
- [ ] Sibling tests in `oriterm_core/src/image/cache/tests.rs`:
  - `on_resize_removes_fully_out_of_bounds_placement()`
  - `on_resize_clamps_partially_overlapping_placement_under_hybrid_policy()`
  - `on_resize_preserves_in_bounds_placement()`
  - `on_resize_orphan_image_pruned_after_all_placements_removed()`
- [ ] **Validation**: tests pass; no regressions in existing image cache tests.

---

## 07.3 Wire Term::resize to call image_cache.on_resize

**File(s):** `oriterm_core/src/term/mod.rs` (around the `resize` method), sibling tests

- [ ] Find `Term::resize(&mut self, lines: usize, cols: usize)` (or whatever the canonical resize entry point is).
- [ ] After the grid is resized, call `self.image_cache.on_resize(cols, lines, ResizePolicy::Hybrid)`.
- [ ] If the alt screen is active, also call `self.alt_image_cache.as_mut().map(|c| c.on_resize(cols, lines, ResizePolicy::Hybrid))`.
- [ ] Sibling test:
  - `term_resize_invokes_image_cache_on_resize()` — install a sixel placement, resize the grid smaller, assert the placement is removed/clamped per policy
- [ ] **Validation**: test passes; existing teseq tests still pass.

---

## 07.4 Build the image lifecycle regression matrix

**File(s):** `oriterm_core/tests/image_lifecycle_matrix.rs` (new), or extend `oriterm_core/src/image/cache/tests.rs`

The regression matrix is the test set that proves image placements survive every grid transformation correctly. Per CLAUDE.md TDD discipline, this matrix is written FIRST (failing) and then the implementation in 07.2/07.3 makes it pass.

- [ ] Build the test matrix as a table-driven test:
  ```rust
  #[derive(Debug)]
  struct LifecycleScenario {
      name: &'static str,
      protocol: ImageProtocol, // Sixel, Kitty, ITerm2
      mutation: GridMutation,  // ScrollbackEvict, Resize { new_cols, new_rows }, Reflow, AltEnter, AltExit, EraseDisplay, EraseLine
      expected_placement_state: PlacementState, // Survives, Removed, Clamped { new_extent }
  }
  ```
- [ ] Enumerate the matrix: 3 protocols × 7 mutations = 21 scenarios. Each scenario instantiates the harness, places an image via the protocol, applies the mutation, asserts the expected state.
- [ ] Use `SpecHarness` from section 04 for instantiation and observation.
- [ ] **Validation**: all 21 matrix entries pass.

---

## 07.R Third Party Review Findings

- None.

---

## 07.N Completion Checklist

- [ ] Failing test matrix written FIRST: 21 lifecycle matrix tests, all initially failing
- [ ] **Matrix dimensions**: image protocol (sixel/kitty/iterm2) × grid mutation (scrollback evict, resize, reflow, alt enter, alt exit, ED, EL) = 21 cells
- [ ] **Semantic pin**: matrix tests are the permanent regression guard; if a future change breaks image lifecycle on any cell, the matrix flags it
- [ ] `ImageCache::on_resize` exists with documented `ResizePolicy`
- [ ] Resize policy decision doc committed under `plans/spec-conformance/specs/image-resize-policy.md`
- [ ] `Term::resize` invokes `on_resize` for both primary and alt cache
- [ ] All 21 matrix scenarios pass
- [ ] Existing image cache tests pass without modification
- [ ] Existing teseq tests pass
- [ ] Alloc regression unchanged
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release
- [ ] Plan annotation cleanup
- [ ] Section frontmatter `status` → `complete`
- [ ] `00-overview.md` Quick Reference + mission criteria updated
- [ ] `index.md` section 07 status updated
- [ ] `/tpr-review` passed
- [ ] `/impl-hygiene-review last commit` passed (after `/tpr-review` is clean)

**Exit Criteria:** Image cache has on_resize handler; regression matrix covers every protocol × every grid mutation; no regressions in existing image tests; ready for sections 12-14 (image stack) and section 21 (notcurses-demo harness) to use.
