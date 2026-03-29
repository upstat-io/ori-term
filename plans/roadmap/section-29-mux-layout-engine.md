---
section: 29
title: Mux Crate + Layout Engine
status: complete
reviewed: true
last_verified: "2026-03-29"
tier: 4M
goal: Create the oriterm_mux crate with newtype IDs, immutable SplitTree, FloatingLayer, spatial navigation, and layout computation
sections:
  - id: "29.1"
    title: Crate Bootstrap + Newtype IDs
    status: complete
  - id: "29.2"
    title: Immutable SplitTree
    status: complete
  - id: "29.3"
    title: FloatingLayer
    status: complete
  - id: "29.4"
    title: Layout Computation
    status: complete
  - id: "29.5"
    title: Spatial Navigation
    status: complete
  - id: "29.6"
    title: Section Completion
    status: complete
---

# Section 29: Mux Crate + Layout Engine

**Status:** Complete
**Goal:** Create the `oriterm_mux` crate — the multiplexing foundation. Defines all identity types, the immutable split tree, floating pane layer, layout computation, and spatial navigation. Pure data structures with no I/O, no GUI, no PTY — fully testable in isolation.

> **Architectural deviation (verified 2026-03-29):** The plan places SplitTree, FloatingLayer, compute, and nav in `oriterm_mux/src/layout/` and `oriterm_mux/src/nav.rs`. The implementation correctly follows the CLAUDE.md crate boundary rules instead: SplitTree, FloatingLayer, compute, and nav live in `oriterm/src/session/` (GUI-owned session model). The mux crate remains a flat pane server exporting only `PaneId`, `DomainId`, `ClientId`. GUI-local IDs (`TabId`, `WindowId`) live in `oriterm/src/session/id/`. This is the correct architecture per CLAUDE.md. File paths in the plan below reflect the original aspirational locations, not the actual implementation paths.

**Crate:** `oriterm_mux` (new crate)
**Dependencies:** None (pure data structures). `serde` for serialization support.
**Prerequisite:** Section 04 (PTY + Event Loop) complete — mux builds on PTY abstractions.

**Inspired by:**
- Ghostty: immutable `SplitTree` — structural sharing, no in-place mutation, undo via history stack
- WezTerm: binary tree splits with `tab_id`/`pane_id` separation, `PaneEntry` for layout results
- Zellij: tiled + floating pane model, floating overlay with position/size
- tmux: the baseline expectation for pane navigation and resize behavior

**Architecture:** `oriterm_mux` sits between `oriterm_core` (terminal library) and `oriterm` (GUI binary). It owns all multiplexing state: which panes exist, how they're laid out, which tab/window they belong to, and how to navigate between them. The GUI binary becomes a thin rendering client.

---

## 29.1 Crate Bootstrap + Newtype IDs

Create the `oriterm_mux` workspace member with newtype identity types. These IDs are the currency of the entire mux system — every other component references panes, tabs, windows, and sessions by these types.

**File:** `oriterm_mux/src/lib.rs`, `oriterm_mux/src/id.rs`

- [x] Create `oriterm_mux/` directory and `Cargo.toml`: (verified 2026-03-29)
  - [x] `[package] name = "oriterm_mux"`, edition 2024 (verified 2026-03-29)
  - [x] Dependencies: `serde` (with `derive` feature, optional behind `serde` feature flag) (verified 2026-03-29)
  - [x] No dependency on `oriterm_core` or `oriterm` — pure standalone crate (verified 2026-03-29)
- [x] Add `"oriterm_mux"` to workspace `members` in root `Cargo.toml` (verified 2026-03-29)
- [x] Add `oriterm_mux` as dependency of `oriterm` (binary crate) (verified 2026-03-29)
- [x] Newtype IDs in `oriterm_mux/src/id/mod.rs`: (verified 2026-03-29)
  - [x] `PaneId(u64)` — globally unique pane identifier (verified 2026-03-29)
  - [x] `DomainId(u64)` — globally unique domain identifier (verified 2026-03-29) — NOTE: plan originally said TabId here; TabId/WindowId moved to `oriterm/src/session/id/` per crate boundaries
  - [x] `ClientId(u64)` — client identifier (verified 2026-03-29) — NOTE: plan originally said WindowId here; WindowId moved to `oriterm/src/session/id/`
  - [x] `SessionId` — sealed trait for type-safe allocation, NOT a `SessionId(u64)` newtype (verified 2026-03-29) — deliberate design: sessions are not yet a serializable concept
  - [x] All IDs: `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]` (verified 2026-03-29)
  - [x] All IDs: `impl Display` (for logging: `Pane(42)`, `Domain(5)`) (verified 2026-03-29)
  - [x] All IDs: `#[derive(Serialize, Deserialize)]` (verified 2026-03-29)
- [x] GUI session IDs in `oriterm/src/session/id/mod.rs`: (verified 2026-03-29)
  - [x] `TabId(u64)` — GUI-local tab identifier (verified 2026-03-29)
  - [x] `WindowId(u64)` — GUI-local window identifier (verified 2026-03-29)
  - [x] Sealed `SessionId` trait for type-safe allocation (verified 2026-03-29)
- [x] `IdAllocator` — monotonic counter per ID type: (verified 2026-03-29)
  - [x] `IdAllocator::new() -> Self` — starts at 1 (0 reserved for "none") (verified 2026-03-29)
  - [x] `IdAllocator::alloc(&mut self) -> u64` — increment and return (verified 2026-03-29)
  - [x] Separate allocators for panes, domains, clients (mux) and tabs, windows (session) (verified 2026-03-29)
- [x] `oriterm_mux/src/lib.rs` — re-export public API: (verified 2026-03-29)
  - [x] `pub mod id;` (verified 2026-03-29)
  - [x] NOTE: layout and nav modules live in `oriterm/src/session/` not `oriterm_mux` (verified 2026-03-29)

**Tests:** 13 mux ID tests + 11 session ID tests = 24 total, ALL PASS (verified 2026-03-29)
- [x] IDs are `Copy`, `Hash`, `Eq` — compile-time trait bound check (verified 2026-03-29)
- [x] `IdAllocator` produces monotonically increasing unique values (verified 2026-03-29)
- [x] `Display` output matches expected format (verified 2026-03-29)
- [x] Different ID types are not interchangeable (type safety) (verified 2026-03-29)
- [x] `from_raw`/`raw` round-trip (verified 2026-03-29)
- [x] IDs work as hash keys (verified 2026-03-29)
- [x] Allocator default matches new (verified 2026-03-29)
- [x] MuxId trait round-trip (verified 2026-03-29)

---

## 29.2 Immutable SplitTree

The layout tree is the core data structure. Following Ghostty's approach: the tree is **immutable** — every mutation returns a new tree, enabling structural sharing, undo/redo, and safe concurrent reads. Internal nodes are splits; leaves are pane references.

**Actual location:** `oriterm/src/session/split_tree/mod.rs` (180 lines) + `mutations.rs` (394 lines)

**Reference:** Ghostty `src/terminal/SplitTree.zig`, WezTerm `mux/src/tab.rs` (Tree struct)

- [x] `SplitTree` enum: (verified 2026-03-29)
  ```rust
  /// Immutable binary layout tree.
  ///
  /// Every mutation method returns a new tree (COW via `Arc`).
  /// History of previous trees enables undo/redo.
  #[derive(Debug, Clone, PartialEq)]
  pub enum SplitTree {
      Leaf(PaneId),
      Split {
          direction: SplitDirection,
          ratio: f32,
          first: Arc<Self>,
          second: Arc<Self>,
      },
  }
  ```
- [x] `SplitDirection` enum: `Horizontal` (top/bottom), `Vertical` (left/right) (verified 2026-03-29)
- [x] Immutable mutation methods (all return new `SplitTree`): (verified 2026-03-29)
  - [x] `split_at(pane: PaneId, dir: SplitDirection, new_pane: PaneId, ratio: f32) -> SplitTree` (verified 2026-03-29)
  - [x] `remove(pane: PaneId) -> Option<SplitTree>` — None if last pane (verified 2026-03-29)
  - [x] `set_ratio(pane: PaneId, direction: SplitDirection, new_ratio: f32) -> SplitTree` — clamped 0.1..=0.9 (verified 2026-03-29)
  - [x] `equalize() -> SplitTree` — recursively set all ratios to 0.5 (verified 2026-03-29)
  - [x] `swap(a: PaneId, b: PaneId) -> SplitTree` — swap two pane positions (verified 2026-03-29)
  - [x] `set_divider_ratio` — pane-pair targeted ratio update (verified 2026-03-29, beyond plan)
  - [x] `resize_toward` — deepest-first border adjustment (verified 2026-03-29, beyond plan)
  - [x] `try_resize_toward` — Option variant for change detection (verified 2026-03-29, beyond plan)
- [x] Query methods: (verified 2026-03-29)
  - [x] `contains(pane: PaneId) -> bool` (verified 2026-03-29)
  - [x] `pane_count() -> usize` — number of leaves (verified 2026-03-29)
  - [x] `panes() -> Vec<PaneId>` — depth-first order (verified 2026-03-29)
  - [x] `depth() -> usize` — maximum nesting depth (verified 2026-03-29)
  - [x] `parent_split(pane: PaneId) -> Option<(SplitDirection, f32)>` (verified 2026-03-29)
  - [x] `sibling(pane: PaneId) -> Option<PaneId>` (verified 2026-03-29)
  - [x] `first_pane()` — non-allocating leftmost pane query (verified 2026-03-29, beyond plan)
- [x] Ratio clamping: MIN_RATIO=0.1, MAX_RATIO=0.9 (verified 2026-03-29)
- [x] `Arc` sharing: unchanged subtrees share memory (Arc::ptr_eq tested) (verified 2026-03-29)
- [x] Immutability: all mutation methods are `&self` + `#[must_use]` (verified 2026-03-29)

**Tests:** 60 tests, ALL PASS (verified 2026-03-29)
- [x] Single pane: `Leaf(p1)` — `pane_count() == 1`, `contains(p1) == true` (verified 2026-03-29)
- [x] Split at leaf: produces correct `Split` node with original and new pane (verified 2026-03-29)
- [x] Nested split: split a pane inside an existing split — 3 panes total (verified 2026-03-29)
- [x] Remove middle pane: tree collapses correctly, remaining panes preserved (verified 2026-03-29)
- [x] Remove last pane: returns `None` (verified 2026-03-29)
- [x] `equalize()` sets all ratios to 0.5 recursively (verified 2026-03-29)
- [x] Ratio clamping: values below 0.1 clamped to 0.1, above 0.9 to 0.9 (verified 2026-03-29)
- [x] `swap()` exchanges two pane positions (verified 2026-03-29)
- [x] `panes()` returns depth-first order (verified 2026-03-29)
- [x] Structural sharing: after `split_at`, unchanged subtrees share `Arc` pointers (Arc::ptr_eq) (verified 2026-03-29)
- [x] Deep nesting (6+ levels): count, depth, contains, remove, swap, equalize (verified 2026-03-29)
- [x] set_divider_ratio: simple, nested, clamps, nonexistent, on leaf (verified 2026-03-29)
- [x] resize_toward: all directions, nested finds deepest, clamps, leaf noop (verified 2026-03-29)
- [x] Exhaustive leaf removal: 4-pane and 7-pane deep chain (verified 2026-03-29)

---

## 29.3 FloatingLayer

Floating panes overlay the tiled layout. Inspired by Zellij's floating pane system — panes have absolute position and size within the window, rendered on top of the tiled layer with a drop shadow.

**Actual location:** `oriterm/src/session/floating/mod.rs` (321 lines)

**Reference:** Zellij `zellij-server/src/panes/floating_panes/` (FloatingPaneGrid, FloatingPanes)

- [x] `FloatingPane` struct: (verified 2026-03-29)
  ```rust
  pub struct FloatingPane {
      pub pane_id: PaneId,
      pub x: f32,       // Logical pixels from left edge of tab area.
      pub y: f32,       // Logical pixels from top edge of tab area.
      pub width: f32,   // Logical width in pixels.
      pub height: f32,  // Logical height in pixels.
      pub z_order: u32, // Higher = closer to viewer.
  }
  ```
- [x] `FloatingLayer` struct: (verified 2026-03-29)
  - [x] `panes: Vec<FloatingPane>` — ordered by z_order (ascending) (verified 2026-03-29)
  - [x] Immutable mutation methods (return new `FloatingLayer`): (verified 2026-03-29)
    - [x] `add(pane: FloatingPane) -> FloatingLayer` — partition_point for sorted insert (verified 2026-03-29)
    - [x] `remove(pane_id: PaneId) -> FloatingLayer` (verified 2026-03-29)
    - [x] `move_pane(pane_id: PaneId, x: f32, y: f32) -> FloatingLayer` (verified 2026-03-29)
    - [x] `resize_pane(pane_id: PaneId, width: f32, height: f32) -> FloatingLayer` (verified 2026-03-29)
    - [x] `raise(pane_id: PaneId) -> FloatingLayer` — bring to front (verified 2026-03-29)
    - [x] `lower(pane_id: PaneId) -> FloatingLayer` — send to back (verified 2026-03-29)
  - [x] Hot-path mutable variants (beyond plan): (verified 2026-03-29)
    - [x] `move_pane_mut`, `resize_pane_mut`, `set_pane_rect_mut` — in-place for drag operations
  - [x] Query methods: (verified 2026-03-29)
    - [x] `hit_test(x: f32, y: f32) -> Option<PaneId>` — reverse z_order (verified 2026-03-29)
    - [x] `pane_rect(pane_id: PaneId) -> Option<Rect>` (verified 2026-03-29)
    - [x] `contains(pane_id: PaneId) -> bool` (verified 2026-03-29)
    - [x] `panes() -> &[FloatingPane]` (verified 2026-03-29)
    - [x] `is_empty() -> bool` (verified 2026-03-29)
- [x] Default floating pane size: 60% of tab width, 60% of tab height, centered (verified 2026-03-29)
- [x] Minimum floating pane size: MIN_FLOATING_PANE_CELLS (20, 5) (verified 2026-03-29)
- [x] Snap-to-edge: SNAP_THRESHOLD_PX = 10.0, all four edges (verified 2026-03-29)

**Tests:** 28 tests, ALL PASS (verified 2026-03-29)
- [x] Add floating pane: appears in layer, `contains` returns true (verified 2026-03-29)
- [x] Remove floating pane: `contains` returns false, other panes unaffected (verified 2026-03-29)
- [x] `hit_test`: returns topmost pane at overlap point (verified 2026-03-29)
- [x] `hit_test`: returns `None` outside all floating panes (verified 2026-03-29)
- [x] `raise`: pane moves to highest z_order (verified 2026-03-29)
- [x] `move_pane`: updates position (verified 2026-03-29)
- [x] `resize_pane`: updates dimensions (verified 2026-03-29)
- [x] z-order sorting invariant and stability across mutations (verified 2026-03-29)
- [x] centered pane (60% size, centered position, offset bounds) (verified 2026-03-29)
- [x] snap-to-edge: left, right, top, bottom, corner, no-snap (verified 2026-03-29)
- [x] z-order after remove-middle, with overlapping panes (verified 2026-03-29)

---

## 29.4 Layout Computation

Convert the abstract `SplitTree` + `FloatingLayer` into concrete pixel rectangles for rendering and PTY resize. This is the bridge between the mux data model and the GPU renderer.

**Actual location:** `oriterm/src/session/compute/mod.rs` (357 lines)

- [x] `LayoutDescriptor` — input to layout computation: (verified 2026-03-29)
  ```rust
  pub struct LayoutDescriptor {
      /// Total available pixel area for the tab content (excludes tab bar).
      pub available: Rect,
      /// Cell dimensions for converting pixels to columns/rows.
      pub cell_width: f32,
      pub cell_height: f32,
      /// Divider thickness in logical pixels.
      pub divider_px: f32,
      /// Minimum pane size in cells (width, height).
      pub min_pane_cells: (u16, u16),
  }
  ```
- [x] `PaneLayout` — output per pane:
  ```rust
  pub struct PaneLayout {
      pub pane_id: PaneId,
      pub pixel_rect: Rect,
      pub cols: u16,
      pub rows: u16,
      pub is_focused: bool,
      pub is_floating: bool,
  }
  ```
- [x] `compute_all` (combined pane + divider), `compute_layout` (pane-only), `compute_dividers` (divider-only) (verified 2026-03-29)
  - [x] Recursively subdivide `desc.available` according to `SplitTree` splits and ratios (verified 2026-03-29)
  - [x] Subtract `desc.divider_px` between split children (verified 2026-03-29)
  - [x] Snap pane boundaries to cell grid via `snap_to_grid` (verified 2026-03-29)
  - [x] Convert pixel dimensions to `cols` / `rows` using cell size (verified 2026-03-29)
  - [x] Append floating pane layouts with min size clamping (verified 2026-03-29)
  - [x] Set `is_focused` on the pane matching `focused` (verified 2026-03-29)
  - [x] Enforce `min_pane_cells` via `clamp_split` (verified 2026-03-29)
- [x] `DividerLayout` — output for divider rendering:
  ```rust
  pub struct DividerLayout {
      pub rect: Rect,
      pub direction: SplitDirection,
      /// The two pane IDs on either side (for drag resize targeting).
      pub pane_before: PaneId,
      pub pane_after: PaneId,
  }
  ```
- [x] `compute_dividers(tree: &SplitTree, desc: &LayoutDescriptor) -> Vec<DividerLayout>` (verified 2026-03-29)
  - [x] One divider per internal `Split` node (verified 2026-03-29)
  - [x] Divider rect: full span of the split, `divider_px` thick (verified 2026-03-29)
- [x] `Rect` type in `oriterm/src/session/rect/` module (verified 2026-03-29)

**Tests:** 34 tests, ALL PASS (verified 2026-03-29)
- [x] Single pane: layout fills entire available rect (verified 2026-03-29)
- [x] Horizontal split 50/50: two rects stacked vertically, divider between (verified 2026-03-29)
- [x] Vertical split 70/30: two rects side by side with correct proportions (verified 2026-03-29)
- [x] Nested splits: 3-pane L-shape layout produces correct rects (verified 2026-03-29)
- [x] Cell grid snapping: pixel rects align to cell boundaries (verified 2026-03-29)
- [x] Divider computation: correct position and neighbors for each divider (verified 2026-03-29)
- [x] Minimum pane size enforcement: ratio clamped when split would produce tiny pane (verified 2026-03-29)
- [x] Floating panes: appear in layout with correct pixel rects, `is_floating == true` (verified 2026-03-29)
- [x] Layout is deterministic: same inputs always produce same outputs (verified 2026-03-29)
- [x] Exact pixel value tests, no-overlap tests, fractional cell dimensions, zero-size rect no-panic (verified 2026-03-29)

---

## 29.5 Spatial Navigation

Navigate between panes using directional movement (up/down/left/right) and sequential cycling. This must work identically for tiled and floating panes.

**Actual location:** `oriterm/src/session/nav/mod.rs` (236 lines)

**Reference:** Ghostty `src/input/navigate.zig`, Zellij `zellij-server/src/panes/tiled_panes/mod.rs` (directional_move)

- [x] `navigate(layouts: &[PaneLayout], from: PaneId, direction: Direction) -> Option<PaneId>` (verified 2026-03-29)
  - [x] `Direction` enum: `Up`, `Down`, `Left`, `Right` with `opposite()` and Display (verified 2026-03-29)
  - [x] Algorithm: centroid-based, primary + 0.5*perp scoring (verified 2026-03-29)
  - [x] Floating panes participate in navigation (verified 2026-03-29)
  - [x] Returns `None` if no pane exists in that direction (verified 2026-03-29)
- [x] `navigate_wrap(layouts, from, direction) -> Option<PaneId>` — wraps to farthest opposite (verified 2026-03-29, beyond plan)
- [x] `cycle(layouts: &[PaneLayout], from: PaneId, forward: bool) -> Option<PaneId>` (verified 2026-03-29)
  - [x] Cycle through panes in layout order, wraps around (verified 2026-03-29)
- [x] `nearest_pane(layouts: &[PaneLayout], x: f32, y: f32) -> Option<PaneId>` (verified 2026-03-29)
  - [x] Prefers floating panes (contains_point check) (verified 2026-03-29)

**Tests:** 60 tests, ALL PASS (verified 2026-03-29)
- [x] 2x2 grid: all four directional navigations (verified 2026-03-29)
- [x] 2x2 grid: navigate from edge → `None` (verified 2026-03-29)
- [x] Cycle forward: visits panes in order, wraps to first (verified 2026-03-29)
- [x] Cycle backward: reverse order, wraps to last (verified 2026-03-29)
- [x] Floating pane: `nearest_pane` prefers floating over tiled at overlap point (verified 2026-03-29)
- [x] Navigate from tiled to floating pane in correct direction (verified 2026-03-29)
- [x] navigate_wrap: right/left/up wrap, no wrap when target exists, single pane, bidirectional (verified 2026-03-29)
- [x] Asymmetric T-shape and L-shape layouts (verified 2026-03-29)
- [x] 5-pane asymmetric (Ghostty-style): all directions, edges, cycle (verified 2026-03-29)
- [x] Degenerate geometry: zero width/height panes no panic (verified 2026-03-29)
- [x] Floating-only layouts, multiple overlapping floats z-order (verified 2026-03-29)

---

## 29.6 Section Completion

- [x] All 29.1–29.5 items complete (verified 2026-03-29)
- [x] `oriterm_mux` crate compiles (verified 2026-03-29)
- [x] `cargo clippy` — no warnings (verified 2026-03-29)
- [x] All tests pass — 254 total (exceeds plan estimate of 86) (verified 2026-03-29)
- [x] Newtype IDs: mux: `PaneId`, `DomainId`, `ClientId`; session: `TabId`, `WindowId` with Display, Hash, Eq (verified 2026-03-29)
- [x] `SplitTree`: immutable, structural sharing, all mutation methods return new trees (verified 2026-03-29)
- [x] `FloatingLayer`: immutable + hot-path mut variants, z-ordered, hit-testing (verified 2026-03-29)
- [x] `compute_layout`: pixel rects snapped to cell grid, dividers, minimum pane enforcement (verified 2026-03-29)
- [x] Spatial navigation: directional + cycling + wrap, works for tiled and floating (verified 2026-03-29)
- [x] Zero dependencies on `oriterm_core` in session module (session imports only `oriterm_mux::PaneId`) (verified 2026-03-29)
- [x] No `unsafe` code (`#![deny(unsafe_code)]` in mux; no unsafe in session) (verified 2026-03-29)

**Beyond plan scope (verified 2026-03-29):** Tab struct with undo/redo stacks (VecDeque, capped at 32), Window struct with tab reordering/insert-at/replace_tabs, SessionRegistry with pane/tab/window lookup, navigate_wrap, try_resize_toward, set_divider_ratio, resize_toward, hot-path _mut variants, first_pane.

**Exit Criteria:** `oriterm_mux` is a standalone crate with a complete layout engine. SplitTree and FloatingLayer are immutable data structures with full test coverage. Layout computation converts abstract trees into concrete pixel rects. Spatial navigation works for any pane arrangement. The crate compiles and tests pass independently.
