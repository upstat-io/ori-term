# Section 29 Verification Results: Mux Crate + Layout Engine

**Date:** 2026-03-29
**Branch:** dev
**Verdict:** PASS (with architectural deviation notes)

## Context Loaded

- CLAUDE.md (project root) -- read in full
- .claude/rules/code-hygiene.md -- read in full
- .claude/rules/test-organization.md -- read in full (also in CLAUDE.md header)
- .claude/rules/impl-hygiene.md -- read in full
- .claude/rules/crate-boundaries.md -- read in full (loaded via system reminder)
- plans/roadmap/section-29-mux-layout-engine.md -- read in full

## Architectural Deviation: Location of Session Model

The plan specifies all components (SplitTree, FloatingLayer, layout computation, nav) should live
in `oriterm_mux/src/layout/` and `oriterm_mux/src/nav.rs`. The actual implementation correctly
follows the CLAUDE.md crate boundary rules instead:

- **`oriterm_mux/src/id/`** -- PaneId, DomainId, ClientId (pane-server IDs)
- **`oriterm/src/session/id/`** -- TabId, WindowId (GUI-local IDs)
- **`oriterm/src/session/split_tree/`** -- Immutable SplitTree
- **`oriterm/src/session/floating/`** -- FloatingLayer
- **`oriterm/src/session/compute/`** -- Layout computation
- **`oriterm/src/session/nav/`** -- Spatial navigation
- **`oriterm/src/session/rect/`** -- Rect primitive
- **`oriterm/src/session/tab/`** -- Tab (owns SplitTree + FloatingLayer)
- **`oriterm/src/session/window/`** -- Window (ordered tab collection)
- **`oriterm/src/session/registry/`** -- SessionRegistry

This is the correct architecture per CLAUDE.md: "oriterm_mux is a flat pane server -- no tabs,
windows, sessions, or layouts." The plan's file locations were aspirational and were correctly
overridden during implementation. The mux crate's `lib.rs` re-exports only `PaneId`, `DomainId`,
`ClientId` -- no layout or nav modules.

Similarly, `TabId`, `WindowId`, and `SessionId` are NOT in `oriterm_mux` -- they are correctly
GUI-local types in `oriterm/src/session/id/`. The plan's item 29.1 claimed these would be in
`oriterm_mux/src/id.rs` alongside `PaneId`, but the implementation properly separates them.

## 29.1 Crate Bootstrap + Newtype IDs

### Mux IDs (oriterm_mux/src/id/)

| Item | Status | Evidence |
|------|--------|----------|
| PaneId(u64) newtype | VERIFIED | `id/mod.rs:14` -- `pub struct PaneId(u64)` |
| DomainId(u64) newtype | VERIFIED | `id/mod.rs:20` -- `pub struct DomainId(u64)` |
| ClientId(u64) newtype | VERIFIED | `id/mod.rs:27` -- `pub struct ClientId(u64)` |
| Debug, Clone, Copy, PartialEq, Eq, Hash derives | VERIFIED | All three types derive all six traits |
| serde derives | VERIFIED | `serde::Serialize, serde::Deserialize` on all three |
| Display impl | VERIFIED | `Pane(42)`, `Domain(5)`, `Client(3)` format |
| IdAllocator monotonic, starts at 1 | VERIFIED | `counter: 1` in `new()`, `alloc()` increments |
| Sealed MuxId trait | VERIFIED | `sealed::Sealed` pattern prevents external impls |
| from_raw/raw round-trip | VERIFIED | Tested in `raw_round_trip` and `mux_id_trait_round_trip` |

### GUI Session IDs (oriterm/src/session/id/)

| Item | Status | Evidence |
|------|--------|----------|
| TabId(u64) newtype | VERIFIED | `id/mod.rs:16` -- `pub struct TabId(u64)` |
| WindowId(u64) newtype | VERIFIED | `id/mod.rs:22` -- `pub struct WindowId(u64)` |
| Derives (all required) | VERIFIED | Copy, Hash, Eq, Debug, Serialize, Deserialize |
| Display impl | VERIFIED | `Tab(3)`, `Window(5)` format |
| IdAllocator for session IDs | VERIFIED | Separate `SessionId` sealed trait, starts at 1 |

### Plan vs Reality: SessionId

The plan calls for a `SessionId(u64)` newtype. In the implementation, `SessionId` is a sealed
trait for type-safe allocation (like `MuxId`), not a newtype ID. There is no `SessionId(u64)`
identity type. This is a deliberate design choice -- sessions are not a first-class concept in
the current architecture.

### Tests (mux IDs): 13 tests, ALL PASS

```
id::tests::id_types_are_copy_hash_eq       -- compile-time trait bound verification
id::tests::allocator_starts_at_one          -- counter initialized to 1
id::tests::allocator_produces_monotonically_increasing_values
id::tests::allocator_values_are_unique      -- 1000 allocations, HashSet dedup check
id::tests::allocator_returns_correct_type   -- type-parameterized allocator
id::tests::display_pane_id                  -- "Pane(42)"
id::tests::display_domain_id                -- "Domain(5)"
id::tests::display_client_id                -- "Client(3)"
id::tests::raw_round_trip                   -- from_raw/raw identity
id::tests::mux_id_trait_round_trip          -- generic MuxId trait
id::tests::different_id_types_are_not_interchangeable  -- type safety
id::tests::ids_work_as_hash_keys            -- HashSet operations
id::tests::allocator_default_same_as_new    -- Default impl consistency
```

### Tests (session IDs): 11 tests, ALL PASS

Covers: round-trip, display, equality, allocator monotonicity, allocator default, generic
SessionId trait usage, hash consistency.

## 29.2 Immutable SplitTree

**Location:** `oriterm/src/session/split_tree/mod.rs` (180 lines) + `mutations.rs` (394 lines)

| Item | Status | Evidence |
|------|--------|----------|
| SplitTree enum (Leaf/Split) | VERIFIED | `mod.rs:50-64` -- Leaf(PaneId), Split { direction, ratio, first: Arc, second: Arc } |
| SplitDirection (Horizontal/Vertical) | VERIFIED | `mod.rs:28-34` |
| split_at returns new tree | VERIFIED | `mutations.rs:21-62` -- `#[must_use]`, recursive replacement |
| remove returns Option<SplitTree> | VERIFIED | `mutations.rs:70-109` -- None for last pane, collapse parent |
| set_ratio clamps 0.1..=0.9 | VERIFIED | `mutations.rs:122-160` -- uses `clamp_ratio()` |
| set_divider_ratio | VERIFIED | `mutations.rs:169-217` -- pane-pair targeted |
| resize_toward (deepest-first) | VERIFIED | `mutations.rs:231-244` + inner helper `299-373` |
| try_resize_toward (Option variant) | VERIFIED | `mutations.rs:249-259` |
| equalize sets all to 0.5 | VERIFIED | `mutations.rs:263-278` |
| swap two pane positions | VERIFIED | `mutations.rs:288-293` + `swap_inner 376-393` |
| contains, pane_count, panes, depth | VERIFIED | `mod.rs:77-114` |
| parent_split, sibling | VERIFIED | `mod.rs:121-166` |
| first_pane (non-allocating) | VERIFIED | `mod.rs:93-98` |
| Ratio clamping MIN_RATIO=0.1 MAX_RATIO=0.9 | VERIFIED | `mod.rs:17-25` |
| Arc structural sharing | VERIFIED | `mutations.rs` uses `Arc::clone(second)` for unchanged subtrees |
| Immutability (all methods return new) | VERIFIED | All mutation methods are `&self` + `#[must_use]` |

### Tests: 60 tests, ALL PASS

Comprehensive coverage including:
- Single pane: count, contains, depth, panes list, no parent/sibling (6 tests)
- Split at leaf: produces correct node, preserves order, stores direction/ratio, nonexistent pane noop (4 tests)
- Nested splits: 3-pane, 4-pane grid (2 tests)
- Remove: last pane None, collapse to sibling, middle pane preserves remaining, nonexistent noop (4 tests)
- Equalize: all ratios to 0.5, single pane noop (2 tests)
- Ratio clamping: below min, above max, set_ratio clamps (3 tests)
- Swap: basic exchange, nested tree, same pane noop, nonexistent noop (4 tests)
- Depth-first order (1 test)
- Structural sharing: Arc::ptr_eq verification (1 test)
- Sibling: leaf in split, None when sibling is split, nonexistent (3 tests)
- set_ratio: updates matching direction, ignores wrong direction (2 tests)
- SplitDirection Display (1 test)
- Deep nesting (6+ levels): count, depth, contains all, remove middle, swap leaf/deep, equalize (6 tests)
- Duplicate pane IDs: edge cases (2 tests)
- set_divider_ratio: simple, nested inner, nested outer, clamps, nonexistent, on leaf (6 tests)
- Split ratio boundary values: 0.0 clamps to 0.1, 1.0 clamps to 0.9 (2 tests)
- Exhaustive leaf removal: 4-pane and 7-pane deep chain (2 tests)
- resize_toward: all directions, nested finds deepest, outer when inner wrong side, clamps, leaf noop, mixed directions (8 tests)

## 29.3 FloatingLayer

**Location:** `oriterm/src/session/floating/mod.rs` (321 lines)

| Item | Status | Evidence |
|------|--------|----------|
| FloatingPane struct (pane_id, rect, z_order) | VERIFIED | `mod.rs:31-39` |
| FloatingLayer (Vec<FloatingPane>, z-ordered) | VERIFIED | `mod.rs:67-71` |
| add (immutable, correct z-order insertion) | VERIFIED | `mod.rs:120-125` -- `partition_point` for sorted insert |
| remove (immutable) | VERIFIED | `mod.rs:129-137` |
| move_pane (immutable) | VERIFIED | `mod.rs:145-161` |
| resize_pane (immutable) | VERIFIED | `mod.rs:169-189` |
| move_pane_mut (hot-path in-place) | VERIFIED | `mod.rs:195-200` |
| resize_pane_mut (hot-path in-place) | VERIFIED | `mod.rs:206-211` |
| set_pane_rect_mut (combined move+resize) | VERIFIED | `mod.rs:217-221` |
| raise (bring to front) | VERIFIED | `mod.rs:225-245` |
| lower (send to back) | VERIFIED | `mod.rs:256-277` |
| hit_test (reverse z-order) | VERIFIED | `mod.rs:109-115` |
| pane_rect, contains, panes, is_empty | VERIFIED | `mod.rs:80-100` |
| centered() constructor (60% size) | VERIFIED | `mod.rs:45-61` |
| snap_to_edge (10px threshold) | VERIFIED | `mod.rs:288-317` -- free function, all four edges |
| MIN_FLOATING_PANE_CELLS (20, 5) | VERIFIED | `mod.rs:22` |
| SNAP_THRESHOLD_PX = 10.0 | VERIFIED | `mod.rs:28` |

### Tests: 28 tests, ALL PASS

Coverage includes: empty layer, add/remove/contains, hit_test (topmost, per-pane, None), raise/lower,
move/resize, pane_rect, z-order sorting invariant, z-order stability across mutations, centered pane
(60% size, centered position, offset bounds), snap-to-edge (left, right, top, bottom, corner, no-snap,
offset bounds), z-order after remove-middle, z-order with overlapping panes.

## 29.4 Layout Computation

**Location:** `oriterm/src/session/compute/mod.rs` (357 lines)

| Item | Status | Evidence |
|------|--------|----------|
| LayoutDescriptor (available, cell_width/height, divider_px, min_pane_cells) | VERIFIED | `mod.rs:13-25` |
| PaneLayout (pane_id, pixel_rect, cols, rows, is_focused, is_floating) | VERIFIED | `mod.rs:28-42` |
| DividerLayout (rect, direction, pane_before, pane_after) | VERIFIED | `mod.rs:45-55` |
| compute_all (combined pane + divider) | VERIFIED | `mod.rs:67-82` |
| compute_layout (pane-only) | VERIFIED | `mod.rs:92-99` |
| compute_dividers (divider-only) | VERIFIED | `mod.rs:108-115` |
| Recursive tree traversal | VERIFIED | `compute_tree()` at `mod.rs:119-176` |
| split_rect (ratio-based subdivision) | VERIFIED | `mod.rs:218-263` |
| clamp_split (minimum pane enforcement) | VERIFIED | `mod.rs:266-326` |
| snap_to_grid (cell-aligned rects) | VERIFIED | `mod.rs:329-338` |
| Floating pane append with min size clamping | VERIFIED | `append_floating()` at `mod.rs:184-214` |
| Divider rect computation | VERIFIED | `mod.rs:151-166` -- correct position between children |
| Cols/rows from pixel dimensions | VERIFIED | `(width / cell_width).floor() as u16` at `mod.rs:129` |

### Tests: 34 tests, ALL PASS

Coverage includes: single pane fills rect, horizontal 50/50, vertical 70/30, nested L-shape, cell grid
snapping alignment, dividers for single/nested splits, minimum pane size enforcement, floating panes
appended with correct cols/rows, focused pane marking, determinism, sequential split workflow,
resize via set_ratio + recompute, remove pane + recompute, hit-test consistency after resize, exact
pixel value tests (3), no-overlap after various operations (5), resize_toward layout propagation,
set_divider_ratio shift, fractional cell dimensions, zero-size rect no-panic, floating min-size clamp.

## 29.5 Spatial Navigation

**Location:** `oriterm/src/session/nav/mod.rs` (236 lines)

| Item | Status | Evidence |
|------|--------|----------|
| Direction enum (Up/Down/Left/Right) | VERIFIED | `mod.rs:11-23` |
| navigate (centroid-based, primary + 0.5*perp scoring) | VERIFIED | `mod.rs:55-121` |
| navigate_wrap (wraps to farthest opposite) | VERIFIED | `mod.rs:129-185` |
| cycle (layout order, wraps around) | VERIFIED | `mod.rs:192-208` |
| nearest_pane (floating preferred, contains_point check) | VERIFIED | `mod.rs:214-233` |
| Direction::opposite helper | VERIFIED | `mod.rs:27-34` |
| Direction Display impl | VERIFIED | `mod.rs:37-46` |
| Floating panes participate in navigation | VERIFIED | Tests confirm tiled-to-floating, floating-to-floating nav |

### Tests: 60 tests, ALL PASS

Comprehensive coverage:
- 2x2 grid: all four directional navigations (4), rightmost/topmost returns None (2), diagonal picks nearest (1)
- Cycle: forward in order, wraps first, backward, wraps last, single pane returns self (5)
- nearest_pane: tiled, prefers floating, None outside (3)
- Floating navigation: tiled-to-floating, floating-to-tiled, floating-to-floating (3)
- Uneven splits: 60/40, uneven 2x2 (3)
- Empty/single edge cases: empty layouts, single pane (6)
- Asymmetric T-shape and L-shape layouts (5)
- 3-pane nested: all directions, cycle visits all (2)
- Progressive pane removal: navigate and cycle after removals (2)
- Border/tie-breaking: exact border, just inside edge, equidistant determinism (3)
- Cycle with mixed tiled+floating (1)
- Multiple overlapping floats z-order (2)
- Partial/no vertical overlap (2)
- 5-pane asymmetric (Ghostty-style): right, down, up, left, edges, cycle forward/backward (7)
- Degenerate geometry: zero width/height panes no panic (3)
- Floating-only layouts (1)
- navigate_wrap: right/left/up wrap, no wrap when target exists, single pane, two-pane bidirectional (6)

## 29.6 Section Completion

| Item | Status | Evidence |
|------|--------|----------|
| All 29.1-29.5 items complete | VERIFIED | See individual sections above |
| oriterm_mux compiles | VERIFIED | `cargo test -p oriterm_mux -- id::` completed |
| session module compiles | VERIFIED | 241 session tests all pass |
| Newtype IDs with Display, Hash, Eq | VERIFIED | Mux: PaneId/DomainId/ClientId. Session: TabId/WindowId |
| SplitTree immutable, structural sharing | VERIFIED | Arc-based COW, #[must_use], Arc::ptr_eq test |
| FloatingLayer immutable + hot-path mut variants | VERIFIED | Both immutable and `_mut` APIs |
| compute_layout: pixel rects, cell snapping, dividers, min enforcement | VERIFIED | 34 tests |
| Spatial navigation: directional + cycling + wrap | VERIFIED | 60 tests |
| Zero dependencies on oriterm_core in session module | VERIFIED | Session imports only oriterm_mux::PaneId |
| No unsafe code | VERIFIED | Grep for `unsafe` in session/ returns no matches; `#![deny(unsafe_code)]` in mux |

## Test Summary

| Module | Tests | Status |
|--------|-------|--------|
| oriterm_mux::id | 13 | ALL PASS |
| session::id | 11 | ALL PASS |
| session::split_tree | 60 | ALL PASS |
| session::floating | 28 | ALL PASS |
| session::compute | 34 | ALL PASS |
| session::nav | 60 | ALL PASS |
| session::rect | 5 | ALL PASS |
| session::tab | 11 | ALL PASS |
| session::window | 16 | ALL PASS |
| session::registry | 16 | ALL PASS |
| **TOTAL** | **254** | **ALL PASS** |

The plan claims 86 tests in section 29.6. The actual count is 254 -- significantly exceeding
the original estimate due to extensive edge case and integration tests.

## Code Hygiene

| Rule | Status |
|------|--------|
| File size < 500 lines | PASS -- max is mutations.rs at 394 lines |
| //! module docs on every file | PASS -- all files have module-level doc comments |
| /// on all pub items | PASS -- all public types, methods, and fields documented |
| No unsafe code | PASS -- `#![deny(unsafe_code)]` in mux; no unsafe in session |
| Sibling tests.rs pattern | PASS -- all test modules use `#[cfg(test)] mod tests;` with sibling file |
| Import organization (3 groups) | PASS -- std, external, crate imports properly grouped |
| No dead/commented code | PASS -- dead_code items have `#[allow(dead_code, reason = "...")]` with justification |
| No unwrap() in library code | PASS -- no unwrap in production paths |
| No decorative banners | PASS -- no `// ===`, `// ---`, etc. |

## Observations

1. **Plan vs Implementation Divergence on Crate Location**: The section plan was written before the
   crate boundary rules were finalized. The plan places SplitTree, FloatingLayer, compute, and nav
   in `oriterm_mux`. The implementation correctly places them in `oriterm/src/session/` per the
   crate boundary rules (mux is pane-only; session model is GUI-owned). This is a plan inaccuracy,
   not an implementation defect.

2. **Beyond Plan Scope**: The implementation includes several features not in the original plan:
   - `Tab` struct with undo/redo stacks (VecDeque<SplitTree>, capped at 32 entries, stale-entry skipping)
   - `Window` struct with tab reordering, insert-at, replace_tabs (daemon sync)
   - `SessionRegistry` with pane/tab/window lookup, `is_last_pane`
   - `navigate_wrap` (directional with wrap-around)
   - `try_resize_toward` (Option variant for change detection)
   - `set_divider_ratio` (pane-pair targeted ratio update)
   - `resize_toward` (deepest-first border adjustment)
   - Hot-path `_mut` variants (move_pane_mut, resize_pane_mut, set_pane_rect_mut)
   - `first_pane` (non-allocating leftmost pane query)

3. **Structural Sharing Verified**: The `split_at_shares_unchanged_subtrees` test uses `Arc::ptr_eq`
   to confirm that unchanged subtrees share the same allocation after mutation -- the core COW
   guarantee is tested, not just assumed.

4. **Degenerate Geometry Coverage**: Tests cover zero-width and zero-height panes without panicking.
   These represent transient states during resize that must be handled gracefully.

5. **No `SessionId` Newtype**: The plan specifies `SessionId(u64)` as a newtype in `oriterm_mux`.
   The implementation uses `SessionId` as a sealed trait for type-safe allocation in
   `oriterm/src/session/id/`. There is no session persistence ID. This is a design decision,
   not a gap -- sessions are not yet a serializable concept.
