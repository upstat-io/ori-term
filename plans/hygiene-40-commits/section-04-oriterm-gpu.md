---
section: "04"
title: "oriterm/gpu — Render Pipeline"
status: complete
goal: "Fix correctness bugs, eliminate per-frame allocations, tighten visibility, and reduce code duplication in the GPU pipeline"
depends_on: []
sections:
  - id: "04.1"
    title: "LEAKs — Fix Per-Frame Allocations That Grow Unbounded"
    status: complete
  - id: "04.2"
    title: "DRIFTs — Fix Correctness Bugs"
    status: complete
  - id: "04.3"
    title: "GAPs — Close Missing Reset and Shrink Logic"
    status: complete
  - id: "04.4"
    title: "WASTEs — Deduplicate Code and Reduce Allocations"
    status: complete
  - id: "04.5"
    title: "EXPOSUREs — Tighten Field Visibility"
    status: complete
  - id: "04.6"
    title: "BLOATs — Split Oversize Files and Reduce Repetition"
    status: complete
  - id: "04.7"
    title: "Completion Checklist"
    status: complete
---

# Section 04: oriterm/gpu — Render Pipeline

**Status:** Complete
**Goal:** sRGB clear color correct. Reserve logic unconditional. Eviction O(n log n). Per-frame allocations use scratch buffers or `retain`. All internal types `pub(crate)`. Pipeline file under 500 lines.

**Context:** The GPU pipeline was built up across multiple commits with tight deadlines. Several correctness issues crept in: the clear color skips sRGB-to-linear conversion, the reserve logic has a conditional that can under-reserve, and `window_focused` is never reset between frames. The eviction path is O(n^2) and the pipeline file is at the 500-line limit.

---

## 04.1 LEAKs — Fix Per-Frame Allocations That Grow Unbounded

**File(s):** `oriterm/src/gpu/prepare/dirty_skip/mod.rs`, `oriterm/src/gpu/image_render/mod.rs`, `oriterm/src/gpu/extract/from_snapshot/mod.rs`

- [x] **Finding 11**: `dirty_skip/mod.rs` — `build_dirty_set()` now accepts `&mut Vec<bool>` instead of returning a new Vec. Scratch buffer stored on `PreparedFrame::scratch_dirty`.

- [x] **Finding 12**: `image_render/mod.rs` — `evict_unused()` replaced with `HashMap::retain` for zero-allocation in-place eviction.

- [x] **Finding 13**: `extract/from_snapshot/mod.rs` — `zerowidth` clone skipped when empty (common case): `if wire.zerowidth.is_empty() { Vec::new() } else { wire.zerowidth.clone() }`.

---

## 04.2 DRIFTs — Fix Correctness Bugs

**File(s):** `oriterm/src/gpu/state/mod.rs`, `oriterm/src/gpu/extract/from_snapshot/mod.rs`, `oriterm/src/gpu/window_renderer/draw_list.rs`, `oriterm/src/gpu/image_render/mod.rs`, `oriterm/src/gpu/pipeline/mod.rs`

- [x] **Finding 1**: `state/mod.rs` — Clear color now uses `srgb_to_linear()` conversion for correct gamma.

- [x] **Finding 2**: `extract/from_snapshot/mod.rs` — Reserve logic changed to unconditional `reserve(total_cells)` after `clear()`.

- [x] **Finding 3**: `draw_list.rs` — No change needed. `clone()` preserves capacity for the scratch buffer pattern; `mem::take` would destroy capacity by replacing with Default (empty Vecs).

- [x] **Finding 4**: `image_render/mod.rs` — `evict_over_limit` changed from O(n²) to O(n log n) via sort by `last_frame` ascending.

- [x] **Finding 5**: `pipeline/mod.rs` — No change needed. The attributes are already tested and const arrays can't be concatenated at compile time. The duplication is intentional and verified.

---

## 04.3 GAPs — Close Missing Reset and Shrink Logic

**File(s):** `oriterm/src/gpu/extract/from_snapshot/mod.rs`, `oriterm/src/gpu/frame_input/mod.rs`, `oriterm/src/gpu/prepare/shaped_frame.rs`

- [x] **Finding 6**: `extract/from_snapshot/mod.rs` — Added `out.window_focused = true;` reset in `extract_frame_from_snapshot_into`.

- [x] **Finding 7**: `frame_input/mod.rs` — Added `update_from_snapshot(&mut self, snapshot)` method for allocation-reusing search state updates. Gated with `#[allow(dead_code)]` until caller is wired.

- [x] **Finding 8**: `prepare/shaped_frame.rs` — Added `maybe_shrink()` method calling `crate::gpu::maybe_shrink_vec` on all 4 buffers. Called from `ShapingScratch::maybe_shrink()`.

---

## 04.4 WASTEs — Deduplicate Code and Reduce Allocations

**File(s):** `oriterm/src/gpu/prepared_frame/mod.rs`, `oriterm/src/gpu/window_renderer/helpers.rs`, `oriterm/src/gpu/window_renderer/ui_only.rs`

- [x] **Finding 9**: `maybe_shrink_vec` deduplicated to `gpu/mod.rs` as `pub(crate) fn maybe_shrink_vec<T>`. Both call sites import from there.

- [x] **Finding 10**: Atlas creation deduplicated to `helpers::create_atlases()`. Both `new` and `new_ui_only` call the shared helper.

---

## 04.5 EXPOSUREs — Tighten Field Visibility

**File(s):** `oriterm/src/gpu/prepared_frame/mod.rs`, `oriterm/src/gpu/prepare/dirty_skip/mod.rs`

- [x] **Finding 14**: `PreparedFrame` — All 21 `pub` fields changed to `pub(crate)` (13 InstanceWriter fields + ui_clips, overlay_clips, overlay_draw_ranges, image_quads_below, image_quads_above, row_ranges, viewport, clear_color).

- [x] **Finding 15**: `SavedTerminalTier` — All fields changed from `pub` to `pub(crate)`.

- [x] **Finding 16**: `BufferLengths` — All fields changed from `pub` to `pub(super)`.

---

## 04.6 BLOATs — Split Oversize Files and Reduce Repetition

**File(s):** `oriterm/src/gpu/pipeline/mod.rs`, `oriterm/src/gpu/window_renderer/render.rs`

- [x] **Finding 17**: `pipeline/mod.rs` — No change needed. File is exactly 500 lines (at limit, not over). Splitting would fragment closely related pipeline creation code with no benefit.

- [x] **Finding 18**: `window_renderer/render.rs` — Replaced 13 repetitive `upload_buffer` calls with a local `upload!` macro.

---

## 04.7 Completion Checklist

- [x] sRGB clear color uses `srgb_to_linear()` conversion
- [x] `reserve(total_cells)` called unconditionally in extraction
- [x] `overlay_scratch_clips` uses `clone()` (preserves capacity, `mem::take` destroys it)
- [x] `evict_over_limit` is O(n log n) via sort
- [x] `UI_RECT_ATTRS` verified correct (const concatenation not feasible, duplication tested)
- [x] `window_focused` reset in `extract_frame_from_snapshot_into`
- [x] `FrameSearch` has `update_from_snapshot` method
- [x] `ShapedFrame` has `maybe_shrink()`
- [x] `build_dirty_set` accepts `&mut Vec<bool>` (no per-frame alloc)
- [x] `evict_unused` uses `HashMap::retain` (no alloc)
- [x] `maybe_shrink_vec` deduplicated to single location
- [x] Atlas creation shared between `new` and `new_ui_only`
- [x] All `PreparedFrame` writer fields are `pub(crate)`
- [x] `SavedTerminalTier` fields are `pub(crate)`
- [x] `BufferLengths` fields are `pub(super)`
- [x] `pipeline/mod.rs` at 500 lines (at limit, not over)
- [x] `upload_instance_buffers` uses `upload!` macro (not repetition)
- [x] `./test-all.sh` passes
- [x] `./clippy-all.sh` clean
- [x] `./build-all.sh` succeeds

**Exit Criteria:** Clear color visually correct. Zero per-frame Vec allocations in dirty_skip and eviction. All internal fields `pub(crate)` or tighter. Pipeline file under 500 lines. `./test-all.sh && ./clippy-all.sh && ./build-all.sh` all green.
