---
section: "03"
title: "oriterm/app — Event Loop/Input/Redraw"
status: complete
goal: "Eliminate panics, deduplicate logic, reduce per-frame allocations, and split oversize files in oriterm/app"
depends_on: []
sections:
  - id: "03.1"
    title: "LEAKs — Replace .expect() with Graceful Handling"
    status: complete
  - id: "03.2"
    title: "DRIFTs — Deduplicate Diverged Logic"
    status: complete
  - id: "03.3"
    title: "GAPs — Fix Platform Conditionals and Missing Abstractions"
    status: complete
  - id: "03.4"
    title: "WASTEs — Reduce Per-Frame Allocations"
    status: complete
  - id: "03.5"
    title: "EXPOSUREs — Decouple from Concrete Event Loop Types"
    status: complete
  - id: "03.6"
    title: "BLOATs — Split Oversize Files"
    status: complete
  - id: "03.7"
    title: "Completion Checklist"
    status: complete
---

# Section 03: oriterm/app — Event Loop/Input/Redraw

**Status:** Complete
**Goal:** Zero `.expect()` panics in input handlers. SelectAll logic unified. Platform merge helpers extracted from 3 files. Per-frame Vec allocations eliminated via scratch buffers. `event_loop.rs` under 500 lines.

**Context:** The app layer handles winit events, dispatches input, manages redraw, and bridges the GUI to the mux. Rapid feature development (mouse input, mark mode, tab dragging, multi-pane) introduced duplicate logic across platform-specific files, panicking expects in mouse handlers, and per-frame allocations that accumulate under heavy use.

---

## 03.1 LEAKs — Replace .expect() with Graceful Handling

**File(s):** `oriterm/src/app/mouse_input.rs`, `oriterm/src/app/mark_mode/mod.rs`, `oriterm/src/app/keyboard_input/mod.rs`

- [x] **Finding 15**: `mouse_input.rs:235,288` — `.expect()` in `handle_mouse_press` and `handle_mouse_drag` replaced with `let Some(mux) = self.mux.as_mut() else { return; };`. Also fixed `.expect()` in `keyboard_input/mod.rs:try_dispatch_mark_mode`.

- [x] **Finding 16**: `mark_mode/mod.rs` — Restructured `word_left`/`word_right` to call `extract_word_context()` directly in each match arm, eliminating the pre-computed Option and its `.expect("computed above")`.

---

## 03.2 DRIFTs — Deduplicate Diverged Logic

**File(s):** `oriterm/src/app/keyboard_input/action_dispatch.rs`, `oriterm/src/app/keyboard_input/overlay_dispatch.rs`, `oriterm/src/app/tab_drag/merge.rs`, `oriterm/src/app/tab_drag/merge_linux.rs`, `oriterm/src/app/tab_drag/merge_macos.rs`, `oriterm/src/app/mouse_input.rs`

- [x] **Finding 7**: Extracted shared `select_all_in_pane()` method in `pane_accessors.rs` that tries shell input selection (OSC 133 zones) first, falls back to entire buffer. Both `action_dispatch.rs` and `overlay_dispatch.rs` now call this unified helper.

- [x] **Finding 8**: Extracted `compute_drop_index`, `execute_tab_merge`, and `compute_drop_index_for_target` into `tab_drag/merge_core.rs`. All three platform merge files (`merge.rs`, `merge_linux.rs`, `merge_macos.rs`) now call these shared helpers.

- [x] **Finding 9**: Extracted `overlay_scale_if_active()` and `dispatch_overlay_mouse()` helpers in `mouse_input.rs`, reducing three `try_overlay_*` methods from ~50 lines each to ~15 lines each.

---

## 03.3 GAPs — Fix Platform Conditionals and Missing Abstractions

**File(s):** `oriterm/src/app/tab_bar_input.rs`, `oriterm/src/app/chrome/resize.rs`, `oriterm/src/app/event_loop.rs`, `oriterm/src/app/constructors.rs`, `oriterm/src/app/snapshot_grid/mod.rs`

- [x] **Finding 10**: `tab_bar_input.rs` — Removed `#[cfg]` from function parameters. All platforms now accept `event_loop: &ActiveEventLoop`; platforms that don't use it apply `let _ = event_loop;`.

- [x] **Finding 11**: Reviewed `chrome/resize.rs` and `event_loop.rs` inline `#[cfg]` blocks. These are small, contextually appropriate platform-specific blocks (macOS fullscreen processing, Windows DPI detection, Windows modal loop, macOS/Linux cursor-left). Extracting to separate files would be overengineering — no change needed.

- [x] **Finding 12**: Extracted `build_common()` method in `constructors.rs` shared by `new()` and `new_daemon()`. Eliminates 35-line duplicated struct literal.

- [x] **Finding 13**: `snapshot_grid/mod.rs` — Changed magic constants to use `CellFlags` directly: `const WIDE_CHAR_SPACER_BIT: WireCellFlags = CellFlags::WIDE_CHAR_SPACER.bits();` and `const WRAP_BIT: WireCellFlags = CellFlags::WRAP.bits();`. Compile-time evaluated, zero drift risk.

- [x] **Finding 14**: Made `delimiter_class` public in `oriterm_core::selection` and imported it in `snapshot_grid/mod.rs`. Removed the duplicate implementation.

---

## 03.4 WASTEs — Reduce Per-Frame Allocations

**File(s):** `oriterm/src/app/event_loop.rs`, `oriterm/src/app/event_loop_helpers/mod.rs`, `oriterm/src/app/redraw/multi_pane.rs`, `oriterm/src/app/chrome/mod.rs`

- [x] **Finding 3**: Added `scratch_dirty_windows: Vec<WindowId>` field to App. `render_dirty_windows()` (now in `render_dispatch.rs`) clears and reuses it each frame instead of allocating.

- [x] **Finding 4**: `modal_loop_render` updated to use the same `scratch_dirty_windows` buffer instead of allocating a local `Vec<WindowId>`.

- [x] **Finding 5**: Added `scratch_pane_sels: HashMap<PaneId, Selection>` and `scratch_pane_mcs: HashMap<PaneId, MarkCursor>` fields to App. `multi_pane.rs` clears and reuses them instead of per-frame local HashMap allocations.

- [x] **Finding 6**: Reviewed `update_tab_bar_hover` layout clone. The `.clone()` is necessary to break the borrow on `self` (immutable borrow via `focused_ctx()` must end before mutable access later). The layout is a small struct (a few floats + Vec<f32> of tab positions, ~80-160 bytes). No change needed — the clone is the correct Rust pattern here.

---

## 03.5 EXPOSUREs — Decouple from Concrete Event Loop Types

**File(s):** `oriterm/src/app/mod.rs`, `oriterm/src/app/constructors.rs`

- [x] **Finding 2**: Replaced `EventLoopProxy<TermEvent>` field with `EventSender` newtype wrapping `Arc<dyn Fn(TermEvent) + Send + Sync>`. The concrete `EventLoopProxy` is consumed in `build_common()` to create the `EventSender`. All 5 call sites updated from `.send_event()` to `.send()`: `clipboard_ops/mod.rs`, `tab_management/move_ops.rs`, `keyboard_input/overlay_dispatch.rs`, `keyboard_input/action_dispatch.rs` (x2).

---

## 03.6 BLOATs — Split Oversize Files

**File(s):** `oriterm/src/app/event_loop.rs`

- [x] **Finding 1**: Extracted `render_dirty_windows()` (70 lines) into `app/render_dispatch.rs`. `event_loop.rs` reduced from 524 to 453 lines.

---

## 03.7 Completion Checklist

- [x] Zero `.expect()` in `mouse_input.rs`, `mark_mode/mod.rs`, and `keyboard_input/mod.rs`
- [x] `SelectAll` logic unified in shared `select_all_in_pane()` helper
- [x] Platform merge helpers extracted to `tab_drag/merge_core.rs`
- [x] `try_overlay_mouse*` boilerplate extracted to helper
- [x] No `#[cfg]` on individual function parameters
- [x] `new()` and `new_daemon()` share common init via `build_common()`
- [x] `WIDE_CHAR_SPACER_BIT`/`WRAP_BIT` use `CellFlags` directly (compile-time evaluated)
- [x] `delimiter_class` imported from core (duplicate removed)
- [x] Per-frame `Vec<WindowId>` allocations use scratch buffers
- [x] `update_tab_bar_hover` clone reviewed — necessary for borrow breaking, no change needed
- [x] `EventLoopProxy` not embedded directly in App (replaced with `EventSender`)
- [x] `event_loop.rs` under 500 lines (453)
- [x] `./test-all.sh` passes
- [x] `./clippy-all.sh` clean
- [x] `./build-all.sh` succeeds

**Exit Criteria:** Zero panics in input handlers. No duplicated logic across platform files. Per-frame allocations use scratch buffers. `event_loop.rs` under 500 lines. `./test-all.sh && ./clippy-all.sh && ./build-all.sh` all green. ✓
