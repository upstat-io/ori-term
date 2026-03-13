---
section: "01"
title: "oriterm_core — Term/Grid/Image Boundaries"
status: complete
goal: "Eliminate panics, reduce visibility, close gaps, and split oversize files in oriterm_core"
depends_on: []
sections:
  - id: "01.1"
    title: "LEAKs — Replace .expect() with Graceful Fallbacks"
    status: complete
  - id: "01.2"
    title: "DRIFTs — Sync Drifted Logic and Fix Incorrect Conversions"
    status: complete
  - id: "01.3"
    title: "GAPs — Implement Stubbed Functionality"
    status: complete
  - id: "01.4"
    title: "WASTEs — Reduce Unnecessary Allocations"
    status: complete
  - id: "01.5"
    title: "EXPOSUREs — Tighten Visibility"
    status: complete
  - id: "01.6"
    title: "BLOATs — Split Oversize Files"
    status: complete
  - id: "01.7"
    title: "Completion Checklist"
    status: complete
---

# Section 01: oriterm_core — Term/Grid/Image Boundaries

**Status:** Not Started
**Goal:** All `.expect()` calls on alt-screen accessors replaced with safe fallbacks. Drifted DECSET/mode sync covered by exhaustive test. Scrollback clearing implemented. Oversize files split below 500 lines. Public API surface minimized to `pub(crate)` where external consumption is not needed.

**Context:** The last 40 commits touched term/handler, grid/editing, snapshot, and image/cache extensively. The rapid iteration introduced 4 panicking `.expect()` calls on alt-screen grid access, leaked `pub` fields on internal scratch buffers, a silent no-op for scrollback erase, and 3 files at or above the 500-line limit.

---

## 01.1 LEAKs — Replace .expect() with Graceful Fallbacks

**File(s):** `oriterm_core/src/term/mod.rs`

Four `.expect()` calls panic when `ALT_SCREEN` mode is set but `alt_grid` is `None`. This can happen during race conditions at mode transitions or if a malformed escape sequence sets the mode flag without allocating the grid.

- [x] **Finding 9**: `term/mod.rs:255-256` — `grid()` method uses `.expect("alt_grid must be Some when ALT_SCREEN is set")`. Replace with `.unwrap_or(&self.grid)` so the primary grid is used as fallback.

- [x] **Finding 10**: `term/mod.rs:292-294` — `image_cache()` and the two `_mut` variants use the same `.expect()` pattern. Replace all four call sites:
  - `grid()` — return `&self.grid` as fallback
  - `grid_mut()` — return `&mut self.grid` as fallback
  - `image_cache()` — return primary image cache as fallback
  - `image_cache_mut()` — return primary image cache as fallback

- [x] Add a `debug_assert!` alongside the fallback so the inconsistency is caught in test builds but never panics in release.

---

## 01.2 DRIFTs — Sync Drifted Logic and Fix Incorrect Conversions

**File(s):** `oriterm_core/src/term/handler/helpers.rs`, `oriterm_core/src/term/modes.rs`, `oriterm_core/src/term/handler/image/kitty.rs`, `oriterm_core/src/term/snapshot.rs`

- [x] **Finding 16**: `handler/helpers.rs:53-79` vs `modes.rs:17-97` — `named_private_mode_flag` and `apply_decset` must enumerate the same set of modes. Add an exhaustive test that iterates all `PrivateMode` variants and asserts both functions handle them (or explicitly skip with a comment). Place test in `oriterm_core/src/term/handler/tests.rs` or a new `oriterm_core/src/term/mode_sync_tests.rs`.

- [x] **Finding 17**: `snapshot.rs:68-165` — `renderable_content_into()` reads the dirty flags but does not drain them. Document the coupling: add a comment explaining that the caller is responsible for draining dirty flags after consuming the renderable content, and cite the call site that does so.

- [x] **Finding 18**: `handler/image/kitty.rs:168-170` — Kitty delete `d=y` uses raw protocol value as `StableRowIndex`. Verify this is correct by checking the Kitty graphics protocol spec. If the protocol sends viewport-relative rows, add the conversion: `scrollback_len - display_offset + protocol_row`.

- [x] **Finding 19**: `handler/image/kitty.rs:346` — Path traversal check uses `contains("..")` which can be bypassed (e.g., `foo/..bar`). Replace with proper path normalization:
  ```rust
  use std::path::Path;
  let canonical = path.canonicalize()?;
  if !canonical.starts_with(&allowed_base) {
      return Err(...);
  }
  ```
  If `canonicalize` is too expensive (requires filesystem access), use iterative component checking that rejects any `Component::ParentDir`.

---

## 01.3 GAPs — Implement Stubbed Functionality

**File(s):** `oriterm_core/src/grid/editing/mod.rs`, `oriterm_core/src/term/handler/helpers.rs`, `oriterm_core/src/term/handler/dcs.rs`, `oriterm_core/src/term/handler/osc.rs`

- [x] **Finding 11**: `grid/editing/mod.rs:330-332` — `DisplayEraseMode::Scrollback` is a silent no-op. Implement scrollback clearing: clear all rows in the scrollback buffer, reset `display_offset` to 0, and fire a damage notification for the full viewport.

- [x] **Finding 12**: `handler/helpers.rs:193` — Duplicate stub for scrollback clearing in `clear_images_after_ed`. Once Finding 11 is implemented, wire this to actually clear image placements that were in the scrollback region.

- [x] **Finding 21** (NOTE, but actionable): `handler/dcs.rs:127-131` — `dcs_text_area_size_pixels` returns hardcoded 0x0. Wire to actual terminal dimensions. The handler has access to the terminal size through `self.cols()` and `self.rows()` — multiply by cell dimensions (which may need to be plumbed from the renderer or stored on the terminal).

- [x] **Finding 25**: `handler/osc.rs:98-115` — `osc_dynamic_color_sequence` takes `prefix: String` by value. Change parameter to `&str` to avoid allocation on every OSC color query.

---

## 01.4 WASTEs — Reduce Unnecessary Allocations

**File(s):** `oriterm_core/src/term/snapshot.rs`, `oriterm_core/src/image/cache/mod.rs`, `oriterm_core/src/image/cache/animation.rs`, `oriterm_core/src/grid/resize/mod.rs`, `oriterm_core/src/grid/editing/mod.rs`

- [x] **Finding 4 [DEFERRED]**: `snapshot.rs:111` — `Vec::new()` per cell with no combining marks. Deferred to `plans/roadmap/section-23-performance.md` (change zerowidth to `Option<Vec<char>>`).

- [x] **Finding 5 [DEFERRED]**: `snapshot.rs:109` — `zerowidth.clone()` deep-clones Vec per cell. Deferred to roadmap Section 23.

- [x] **Finding 6 [DEFERRED]**: `snapshot.rs:217` — `Vec::new()` per frame in `extract_images`. Deferred to roadmap Section 23.

- [x] **Finding 7 [DEFERRED]**: `image/cache/mod.rs:138-143` — `placed_id_set()` O(placements) on every `store()`. Deferred to roadmap Section 23.

- [x] **Finding 8 [DEFERRED]**: `image/cache/animation.rs:94-107` — O(animations * placements) visibility check per frame. Deferred to roadmap Section 23.

- [x] **Finding 23 [DEFERRED]**: `grid/editing/mod.rs:92` — `tmpl_extra.clone()` Arc bump in `put_char` hot path. Deferred to roadmap Section 23.

- [x] **Finding 22**: `grid/resize/mod.rs:151` — `Vec` allocation in `grow_rows`. Use `splice` with an iterator that yields default rows directly, avoiding the intermediate Vec.

---

## 01.5 EXPOSUREs — Tighten Visibility

**File(s):** `oriterm_core/src/term/renderable/mod.rs`, `oriterm_core/src/image/mod.rs`, `oriterm_core/src/image/cache/mod.rs`

- [x] **Finding 13**: `renderable/mod.rs:157-158` — `#[doc(hidden)] pub seen_image_ids` scratch buffer leaks into public API. Change to `pub(crate)`. Remove the `#[doc(hidden)]` attribute (it's no longer needed when visibility is correct).

- [x] **Finding 14**: `image/mod.rs:74` — `ImageData.data` is `pub` with `Arc<Vec<u8>>`. Change to `pub(crate)` and add a `pub fn data(&self) -> &[u8]` accessor if downstream crates need read access. Check if any crate outside `oriterm_core` accesses `.data` directly.

- [x] **Finding 15**: `image/cache/mod.rs` — Multiple methods are `pub` but only used within `oriterm_core`. Audit each `pub` method; change to `pub(crate)` for any that have no external callers. Run `cargo build` after to verify no downstream breakage.

---

## 01.6 BLOATs — Split Oversize Files

**File(s):** `oriterm_core/src/term/handler/mod.rs`, `oriterm_core/src/term/mod.rs`, `oriterm_core/src/grid/editing/mod.rs`

- [x] **Finding 1**: `handler/mod.rs` — 520 lines. Removed redundant doc comments on pure-delegation trait impl methods (trait defines the contract). Reduced to 349 lines. Note: Rust requires a single `impl Trait` block, so file-splitting isn't feasible — compacted instead.

- [x] **Finding 2**: `term/mod.rs` — 504 lines. Moved `cwd_short_path()` and shell-state helpers to `term/shell_state.rs`, image config methods to `term/image_config.rs`. Reduced to 454 lines.

- [x] **Finding 3**: `grid/editing/mod.rs` — 502 lines. Extracted `fix_wide_boundaries()` and `clear_wide_char_at()` into `grid/editing/wide_char.rs`. Reduced to 455 lines.

- [x] **Finding 24** (NOTE, but actionable): `term/mod.rs` — Impl block ordering already follows code-hygiene rules (constructors → accessors → predicates → operations). No reordering needed.

---

## 01.7 Completion Checklist

- [x] All 4 `.expect()` calls in `term/mod.rs` replaced with fallback + `debug_assert!`
- [x] DECSET/mode sync test added and passing
- [x] `DisplayEraseMode::Scrollback` implemented (not a no-op)
- [x] Kitty path traversal uses proper normalization (not `contains("..")`)
- [x] `osc_dynamic_color_sequence` accepts `&str` (not `String`)
- [x] `seen_image_ids` is `pub(crate)` (not `pub`)
- [x] `ImageData.data` is `pub(crate)` with accessor
- [x] `handler/mod.rs` under 500 lines (349 lines)
- [x] `term/mod.rs` under 500 lines (454 lines)
- [x] `grid/editing/mod.rs` under 500 lines (455 lines)
- [x] `./test-all.sh` passes
- [x] `./clippy-all.sh` clean
- [x] `./build-all.sh` succeeds

**Exit Criteria:** Zero `.expect()` on alt-screen accessors, all files under 500 lines, `pub` surface reduced, scrollback clearing functional. `./test-all.sh && ./clippy-all.sh && ./build-all.sh` all green.
