---
section: 55
title: "Minimap Scrollbar"
status: not-started
reviewed: false
last_verified: "2026-04-05"
tier: 8
goal: "VS Code-style full-content minimap on the right edge of the terminal grid — a zoomed-out overview of all scrollback + visible content as colored pixel blocks, with viewport highlight and click/drag navigation"
success_criteria:
  - "Minimap renders full scrollback as colored pixel blocks on the right edge of the terminal grid"
  - "Viewport highlight accurately shows current visible region within the full content"
  - "Click on minimap scrolls to that position; drag viewport highlight scrolls smoothly"
  - "Minimap updates incrementally — only dirty rows uploaded via queue.write_texture()"
  - "Zero minimap work when terminal content is unchanged (event-driven, not per-frame)"
  - "Alt-screen hides minimap entirely"
  - "scrollbar_mode config: none | scrollbar | minimap | both"
  - "Works in embedded and daemon modes with no performance degradation"
  - "Sampling: when scrollback exceeds minimap pixel height, every Nth line sampled"
  - "Resize triggers full minimap rebuild (reflow invalidates all row positions)"
  - "Initial enable on terminal with existing scrollback seeds the full minimap immediately"
  - "All tests pass: ./test-all.sh green"
inspired_by:
  - "VS Code minimap (minimap.ts, minimapPreBaked.ts) — prebaked character pixels, line-level dirty tracking, MinimapSamplingState"
  - "Game engine minimaps — incremental tile updates, never full regeneration (Cyberpunk 2077 rewrite lessons)"
  - "Alacritty damage tracking (term/mod.rs) — row-level dirty flags"
  - "ori_term ScrollbarCaptureController (oriterm_ui/src/controllers/scrollbar_capture/) — capture-phase mouse handling"
depends_on: ["54", "05", "07", "10", "23"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "55.1"
    title: "MinimapRowProducer — IO Thread Row Sampling"
    status: not-started
  - id: "55.2"
    title: "MinimapModel — Sampling State & History Damage"
    status: not-started
  - id: "55.3"
    title: "GPU Minimap Renderer"
    status: not-started
  - id: "55.4"
    title: "Input & Interaction"
    status: not-started
  - id: "55.5"
    title: "Configuration & Integration"
    status: not-started
  - id: "55.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "55.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 55: Minimap Scrollbar

**Status:** Not Started
**Goal:** Add a VS Code-style minimap to ori_term — a persistent, GPU-rendered overview of the full terminal content (scrollback + visible area) displayed as a narrow panel on the right edge. Each terminal cell is rendered as a single blended pixel. A semi-transparent viewport highlight shows the current visible region. Users click or drag to navigate scrollback. The minimap updates incrementally — only dirty rows are uploaded to the GPU texture. When content is unchanged, the minimap does zero work.

**Success Criteria:**

- [ ] Minimap shows full scrollback + visible area as colored pixel blocks on the right edge — satisfies mission criterion for content overview
- [ ] Viewport highlight rectangle accurately represents visible region's position and size within full content
- [ ] Click anywhere on minimap scrolls terminal to that position; drag highlight scrolls smoothly
- [ ] Incremental updates: only dirty rows uploaded per frame via `queue.write_texture()` — satisfies mission criterion for performance
- [ ] Zero minimap GPU/CPU work when content unchanged — event-driven, not per-frame
- [ ] Alt-screen (vim, less, etc.) hides minimap entirely — no scrollback in alt mode
- [ ] Config: `scrollbar_mode = none | scrollbar | minimap | both` — coexists with Section 48 scrollbar
- [ ] Daemon mode: minimap row data travels in `PaneSnapshot`, rendering identical to embedded mode
- [ ] Sampling: when scrollback exceeds minimap pixel height, every Nth line sampled (VS Code pattern)
- [ ] Resize triggers full minimap rebuild (reflow invalidates all row positions)
- [ ] Initial enable on terminal with existing scrollback seeds the full minimap immediately (not blank until new output)
- [ ] `./build-all.sh` green, `./clippy-all.sh` green, `./test-all.sh` green

**Context:** Terminal emulators (Alacritty, Ghostty, WezTerm) offer at most a thin scrollbar for scrollback navigation. VS Code proved that a zoomed-out content preview is far more useful — it provides spatial orientation, visual structure recognition, and direct navigation. The minimap is especially valuable for terminals with large scrollback (build logs, server output). This section builds the minimap as the first consumer of the shared GPU texture infrastructure from Section 54.

**Performance invariants (from game engine research + Codex consensus):**
1. **Minimap history damage is separate from viewport damage** — the grid's `DirtyTracker` only covers visible rows. The minimap needs full-scrollback dirty tracking.
2. **Skipped snapshots never lose minimap deltas** — when `SnapshotDoubleBuffer` detects a skipped frame, minimap escalates to `all_dirty` (same as viewport does today).
3. **Sampling-ratio changes force full rebuild** — when `N` in "every Nth line" changes due to scrollback growth or window resize, the entire minimap texture is invalidated.
4. **GPU residency is globally budgeted** — via Section 54's `PersistentTextureCache` memory limit.

**Reference implementations:**
- **VS Code** `minimap.ts`: `MinimapSamplingState` maps sampled minimap indices to source line numbers. `RenderedLinesCollection` tracks per-line validity with `dy` property. Incremental `putImageData` for changed lines only.
- **VS Code** `minimapPreBaked.ts`: Prebaked pixel data for ASCII chars at 2x1 and 4x2 resolution. We use a simpler approach: single blended pixel per cell.
- **ori_term** `oriterm_ui/src/controllers/scrollbar_capture/`: `ScrollbarCaptureController` demonstrates capture-phase mouse handling for overlay elements not in the hit-test tree. Same pattern needed for minimap.
- **ori_term** `oriterm_core/src/grid/dirty/mod.rs`: `DirtyTracker` with per-row flags and column bounds. Minimap needs an analogous tracker for full scrollback.

**Depends on:** Section 54 (PersistentTextureCache, FrameCapture). Section 05 (GPU pipeline). Section 07 (UI framework, compositor). Section 10 (Mouse input). Section 23 (Damage tracking patterns).

---

## 55.1 MinimapRowProducer — IO Thread Row Sampling

**File(s):**
- `oriterm_core/src/term/minimap/mod.rs` (new module) — pure sampling primitives only (`sample_row`, `MinimapRowData`). Register via `pub mod minimap;` in `oriterm_core/src/term/mod.rs` (currently 487 lines — only 13 lines of headroom; the `mod minimap;` declaration adds 1 line, which fits, but if any other changes are needed in this file, proactively split first).
- `oriterm_core/src/term/minimap/tests.rs` — tests for pure sampling
- `oriterm_mux/src/pane/io_thread/minimap/mod.rs` (new module) — snapshot production orchestration (`produce_minimap_snapshot`, `MinimapSnapshot`, dirty tracking, config gating). Register via `mod minimap;` in `oriterm_mux/src/pane/io_thread/mod.rs` (currently 457 lines — 43 lines of headroom; adding `mod minimap;` + the minimap double buffer field + init code will consume ~10-15 lines, which fits but is tight).
- `oriterm_mux/src/pane/io_thread/minimap/tests.rs` — tests for snapshot production
- `oriterm_mux/src/protocol/snapshot.rs` (add minimap data to wire protocol)

**Crate boundary rationale:** `oriterm_core` owns terminal emulation — pure row sampling (`sample_row`) is a pure function from grid data to pixel data, so it belongs there. But snapshot production orchestration (`produce_minimap_snapshot`) involves config gating, dirty tracking across snapshots, sampling ratio management, and prev_snapshot buffer reuse — this is IO thread production policy, which belongs in `oriterm_mux` (the IO thread is the snapshot producer). See `crate-boundaries.md`: `oriterm_core` "Must NOT contain" mux types; `oriterm_mux` owns "snapshot production."

The IO thread already iterates visible cells to produce `RenderableContent`. The `MinimapRowProducer` extends this to sample full-scrollback rows into compact RGBA pixel data. For each dirty row, it samples N columns (e.g., 128), blending fg/bg colors into a single pixel per sample. The output is a compact byte array suitable for direct `queue.write_texture()` upload.

**Key design (from VS Code prebaked pattern + Codex consensus):**
- Empty cells → background color pixel
- Non-empty cells → blend fg and bg with a luminance bump: `rgb = lerp(bg, fg, 0.4)` (tunable)
- Wide chars: sample at logical column position (skip spacer cells)
- Colors resolved by `sample_row` from raw grid `Row` + `Palette` (NOT from `RenderableCell`, which only covers visible rows — the minimap needs full scrollback). `sample_row` resolves bold-as-bright, dim, and inverse inline using the palette and cell attributes.

- [ ] Write failing tests first: `minimap_sample_row_basic`, `minimap_sample_row_wide_char`, `minimap_sample_empty_row`, `minimap_sample_blends_fg_bg`, `minimap_dirty_rows_only`

- [ ] Define `MinimapRowData` — compact per-row RGBA output:
  ```rust
  /// Minimap pixel data for a single terminal row.
  /// Each element is an RGBA pixel (4 bytes) for one sampled column.
  /// Length = minimap_width (e.g., 128 samples).
  /// NOT heap-allocated per row — sample_row writes directly into a
  /// caller-provided slice (part of MinimapSnapshot's reused buffers).
  pub struct MinimapRowData<'a> {
      pub pixels: &'a mut [u8],  // RGBA, length = width * 4
  }
  ```
  The `sample_row` function writes into a mutable slice provided by the caller (a sub-slice of `MinimapSnapshot::full_data` or a dirty band buffer). No per-row allocation.

- [ ] Define `MinimapSnapshot` — the minimap data transferred in snapshots:
  ```rust
  /// Minimap state transferred from IO thread to render thread.
  pub struct MinimapSnapshot {
      /// Width in pixels (number of sampled columns).
      pub width: u32,
      /// Total height in pixels (number of rows represented).
      pub height: u32,
      /// Current sampling ratio (1 = every row, N = every Nth row).
      /// The IO thread is the single authority for this value.
      /// The render-thread MinimapModel reads it from here — never computes independently.
      pub sampling_ratio: u32,
      /// Dirty row ranges (offsets into band_data).
      pub dirty_bands: Vec<MinimapDirtyBand>,
      /// Shared pixel data buffer for all dirty bands (reused via .clear() + retain).
      pub band_data: Vec<u8>,
      /// True when entire minimap must be rebuilt (resize, sampling ratio change, skipped frames).
      pub all_dirty: bool,
      /// Full RGBA data when all_dirty (width * height * 4 bytes).
      pub full_data: Vec<u8>,
      /// Total scrollback rows (for viewport highlight calculation).
      pub total_rows: usize,
      /// Current display_offset (for viewport highlight position).
      pub display_offset: usize,
      /// Visible row count (for viewport highlight size).
      pub visible_rows: usize,
  }

  pub struct MinimapDirtyBand {
      pub y_offset: u32,      // Row offset in minimap texture
      pub data_offset: usize, // Byte offset into MinimapSnapshot::band_data
      pub data_len: usize,    // Byte length in band_data (width * height * 4)
      pub height: u32,        // Number of rows in this band
  }
  ```
  Note: `MinimapDirtyBand` uses offset+length into `MinimapSnapshot`'s shared `band_data: Vec<u8>` buffer rather than per-band `Vec<u8>` allocations. The shared buffer is reused via `.clear()` + capacity retention across frames.

- [ ] Implement `sample_row(row: &Row, palette: &Palette, bold_is_bright: bool, minimap_width: u32, out: &mut [u8])` — writes RGBA pixels into caller-provided slice (no allocation):
  - Compute column stride: `row.len() / minimap_width` (may be fractional — use f32 stepping)
  - For each sample position, read cell at that column
  - Resolve cell fg/bg colors using `palette` (handle indexed, RGB, and default colors) with `bold_is_bright` for bold ANSI color promotion
  - If cell is empty (no content): output `[bg.r, bg.g, bg.b, 255]`
  - If cell has content: output blended `lerp(bg, fg, 0.4)` as `[r, g, b, 255]`
  - Skip `WIDE_CHAR_SPACER` cells (sample the base cell instead)
  - Handle combining marks: use base cell's fg color

- [ ] Implement `produce_minimap_snapshot(term: &Term<T>, config: &MinimapConfig, state: &mut MinimapProducerState) -> MinimapSnapshot` (in `oriterm_mux`). `MinimapProducerState` holds the previous snapshot's reusable buffers, the dirty tracker `BitVec`, and the previous sampling ratio. It is a field on the IO thread (allocated once, reused across frames).
  - If alt-screen active: return empty/disabled snapshot
  - Compute minimap dimensions: `width = min(128, grid.columns())`, `height = min(total_rows, max_minimap_height)`
  - Compute sampling ratio: if `total_rows > max_minimap_height`, `ratio = total_rows / max_minimap_height` (this is the canonical computation — the render thread reads it from the snapshot, never computes independently)
  - Store `sampling_ratio` in the snapshot
  - If `prev_snapshot` is `None` (first invocation — minimap just enabled, or terminal just created with existing scrollback): set `all_dirty = true` to seed the full minimap texture from existing content
  - Track sampling ratio — if it changed since prev_snapshot, set `all_dirty = true`
  - Iterate dirty scrollback + visible rows using a **minimap-specific dirty tracker** (NOT the grid's `DirtyTracker`, which only covers visible rows). This tracker lives in `oriterm_mux/src/pane/io_thread/minimap/mod.rs` as a field on the minimap producer state — a `BitVec` or `Vec<bool>` sized to `total_rows / sampling_ratio`. Reset after each snapshot production. Marked dirty by comparing the grid's `total_evicted()` counter to detect scrollback changes, and by checking the grid's visible-row dirty flags for rows in the visible region.
  - For each dirty row: call `sample_row()` from `oriterm_core` and add to `dirty_bands`
  - If `all_dirty`: sample all rows into `full_data`
  - Reuse `Vec` allocations from `prev_snapshot` via buffer swap pattern (same as `RenderableContent`)

- [ ] Transfer minimap data via a **separate** double buffer — NOT a field on `RenderableContent`:
  - `RenderableContent` is a hot-path type swapped via `std::mem::swap()` every frame. Adding a `MinimapSnapshot` (potentially 5+ MB of `full_data` for large scrollbacks: 128 * 10000 * 4 bytes) would bloat every swap, violating the "zero allocations in hot render path" performance invariant.
  - **Current obstacle:** `SnapshotDoubleBuffer` (`oriterm_mux/src/pane/io_thread/snapshot/mod.rs`, 97 lines) is hardcoded to `RenderableContent` — it stores `front: RenderableContent` and its methods take `&mut RenderableContent`. It cannot be used for `MinimapSnapshot` without modification.
  - **Concrete fix (option A, preferred):** Make `SnapshotDoubleBuffer` generic: `SnapshotDoubleBuffer<T: Default>`. The `flip_swap` and `swap_front` methods become `fn flip_swap(&self, buf: &mut T)` etc. The `all_dirty` flag on skipped frames must be handled via a trait method or callback (since `all_dirty` is specific to `RenderableContent`). Alternatively, add a trait `SnapshotData: Default` with `fn mark_all_dirty(&mut self)` and implement it for both `RenderableContent` and `MinimapSnapshot`.
  - **Concrete fix (option B, simpler):** Create `MinimapDoubleBuffer` in `oriterm_mux/src/pane/io_thread/minimap/mod.rs` that mirrors the same `Arc<Mutex<Slot>>` + seqno pattern but for `MinimapSnapshot`. Acceptable if the shared skeleton is <30 lines.
  - Add the chosen double buffer as a field on `PaneIoThread` (`oriterm_mux/src/pane/io_thread/mod.rs`, currently 457 lines — only 43 lines of headroom before the 500-line limit; adding the field + initialization in `new_with_handle()` will consume ~10 lines, which fits, but be aware of the tight margin).
  - `MinimapSnapshot` implements `Default` and `clear()` with capacity retention, following the same buffer reuse pattern as `RenderableContent`.
  - Expose the minimap double buffer on `PaneIoHandle` analogously to the existing `double_buffer()` accessor.
  - When minimap is disabled or alt-screen active: the minimap double buffer is not flipped (zero cost).

- [ ] Add minimap data to `PaneSnapshot` wire protocol in `oriterm_mux/src/protocol/snapshot.rs`:
  - Add `WireMinimapSnapshot` (serde-serializable mirror of `MinimapSnapshot`) with `sampling_ratio`, `dirty_bands`, `full_data`, viewport metadata
  - Serialize as part of the pane snapshot PDU
  - Only included when minimap enabled (config flag)
  - Dirty bands are compact: 128 * 4 = 512 bytes per dirty row. Typical frame: 1-50 dirty rows = 0.5-25 KB

- [ ] Wire `produce_minimap_snapshot()` into the IO thread's `produce_snapshot()` method (`oriterm_mux/src/pane/io_thread/mod.rs`, line ~303). After `renderable_content_into()` and `fill_search_snapshot()`, call `produce_minimap_snapshot()` and flip the minimap double buffer. Guard with a config flag so disabled minimap does zero work.

- [ ] All Vec buffers in `MinimapSnapshot` use `.clear()` + capacity retention (zero allocation after warmup)

- [ ] Source files under 500 lines each. Tests in sibling `tests.rs`.

- [ ] **Test matrix:**
  - Sample ASCII row (all cells filled) → non-bg-color pixels
  - Sample empty row → all bg-color pixels
  - Sample row with wide chars (CJK) → skip spacers, sample base cells
  - Sample with `minimap_width < grid.columns()` → correct stride
  - Sample with `minimap_width > grid.columns()` → clamp to grid width
  - All-dirty on sampling ratio change
  - All-dirty on skipped snapshot (sequence number gap)
  - Alt-screen returns `None`
  - First invocation (prev_snapshot=None) produces `all_dirty=true` with full_data populated (initial seeding)
  - **Semantic pin:** non-empty cell produces different pixel than empty cell at same position

- [ ] Verify all tests pass: `timeout 150 cargo test -p oriterm_core -- minimap` and `timeout 150 cargo test -p oriterm_mux -- minimap`

- [ ] **TPR checkpoint** — `/tpr-review` covering 55.1 implementation

---

## 55.2 MinimapModel — Sampling State & History Damage

**File(s):** `oriterm/src/gpu/minimap/model/mod.rs` (new directory module), `oriterm/src/gpu/minimap/model/tests.rs`

The minimap model owns the mapping between terminal rows and minimap pixels, tracks which minimap regions need GPU texture updates, and manages visibility/animation state. This is the render-thread counterpart to the IO thread's `MinimapRowProducer`.

**Key design (from VS Code `MinimapSamplingState` + Codex consensus):**
- Sampling maps minimap pixel Y → source row index
- When scrollback < minimap height: 1:1 mapping (every row = one pixel)
- When scrollback > minimap height: every Nth row sampled
- Viewport highlight: position and size derived from `display_offset` / `total_rows`

- [ ] Write failing tests first: `model_viewport_highlight_position`, `model_sampling_ratio_change`, `model_update_from_snapshot`, `model_row_pixel_roundtrip`

- [ ] Define `MinimapModel`:
  ```rust
  pub(crate) struct MinimapModel {
      /// Current sampling ratio (1 = every row, 2 = every other, etc.)
      sampling_ratio: u32,
      /// Total source rows represented in the minimap.
      total_rows: usize,
      /// Minimap texture dimensions.
      width: u32,
      height: u32,
      /// Viewport highlight (normalized 0.0..1.0 within minimap height).
      viewport_top: f32,
      viewport_height: f32,
      /// Visibility state.
      visible: bool,
      /// Whether the minimap needs a full texture rebuild.
      needs_full_rebuild: bool,
  }
  ```

- [ ] Implement `update_from_snapshot(snapshot: &MinimapSnapshot)`:
  - Update `total_rows`, `width`, `height` from snapshot
  - Read `sampling_ratio` from `snapshot.sampling_ratio` (the IO thread is the single authority — the model never computes this independently)
  - Detect sampling ratio change (compare with stored value) → set `needs_full_rebuild`
  - Recompute viewport highlight position: `viewport_top = (total_rows - display_offset - visible_rows) as f32 / total_rows as f32` (where `total_rows = scrollback_len + visible_rows`, and `display_offset=0` means bottom/latest content, larger values mean further up in history — the highlight moves toward the top of the minimap as `display_offset` increases)
  - Recompute viewport highlight size: `viewport_height = visible_rows as f32 / total_rows as f32`

- [ ] Implement `minimap_row_to_source_row(minimap_y: u32) -> usize`:
  - Maps a click/drag position on the minimap to a source scrollback row
  - Accounts for sampling ratio
  - Clamps to valid range

- [ ] Implement `source_row_to_minimap_y(source_row: usize) -> u32`:
  - Inverse mapping for viewport highlight rendering

- [ ] Implement `viewport_highlight_rect(minimap_rect: Rect) -> Rect`:
  - Returns the pixel rectangle for the viewport highlight within the minimap's screen-space bounds
  - Used for both rendering and hit testing

- [ ] Source file under 500 lines. Tests in sibling `tests.rs`.

- [ ] **Test matrix:**
  - 1:1 mapping (500 rows, 500px minimap) → every row represented
  - 2:1 sampling (1000 rows, 500px minimap) → every other row
  - Viewport at bottom (display_offset=0) → highlight at bottom
  - Viewport at top (display_offset=max) → highlight at top
  - Viewport in middle → highlight proportional
  - Sampling ratio change detected → needs_full_rebuild
  - Row↔pixel round-trip: `minimap_row_to_source_row(source_row_to_minimap_y(r))` ≈ `r`
  - **Semantic pin:** viewport highlight moves upward when display_offset increases (scrolling up through history)

- [ ] Verify all tests pass: `timeout 150 cargo test -p oriterm -- minimap_model`

---

## 55.3 GPU Minimap Renderer

**File(s):** `oriterm/src/gpu/minimap/renderer/mod.rs`, `oriterm/src/gpu/minimap/renderer/tests.rs`

Reads `MinimapSnapshot` from the dedicated minimap double buffer (generic `SnapshotDoubleBuffer<MinimapSnapshot>` or `MinimapDoubleBuffer` — whichever was chosen in 55.1), swapped independently from `RenderableContent`. Uploads dirty bands to the persistent texture via `PersistentTextureCache` (Section 54), and composites the minimap as a textured quad in the overlay pass. The main thread calls `swap_front()` on the minimap double buffer in the same place it swaps the `RenderableContent` double buffer, but they are independent swaps.

- [ ] Write failing tests first: `renderer_uploads_dirty_bands`, `renderer_full_rebuild_uploads_all`, `renderer_skips_when_clean`, `renderer_composites_overlay`, `renderer_viewport_highlight`

- [ ] Define `MinimapRenderer`:
  ```rust
  pub(crate) struct MinimapRenderer {
      model: MinimapModel,
      /// Bind group for the minimap texture (cached, recreated on texture change).
      bind_group: Option<wgpu::BindGroup>,
      /// Screen-space rect where minimap is rendered (right edge of terminal area).
      minimap_rect: Rect,
      /// Whether minimap should render this frame.
      active: bool,
  }
  ```

- [ ] Add `swap_minimap_snapshot()` accessor on `Pane` (`oriterm_mux/src/pane/mod.rs`) mirroring `swap_io_snapshot()` — delegates to the minimap double buffer's `swap_front()`. The main thread calls this alongside `swap_io_snapshot()` to get the latest minimap data.

- [ ] Implement `update(snapshot: &MinimapSnapshot, texture_cache: &mut PersistentTextureCache, device, queue, pane_id)`:
  - If `snapshot.all_dirty`: upload `full_data` via `texture_cache.write_region()` covering entire texture
  - Otherwise: for each `dirty_band`, upload via `texture_cache.write_region()` at band's Y offset
  - If no dirty bands and not all_dirty: skip (zero work)
  - Update `model.update_from_snapshot(snapshot)`
  - Recreate `bind_group` if texture was resized

- [ ] Implement `compute_minimap_rect(grid_rect: Rect, minimap_width: f32, scale_factor: f32) -> Rect`:
  - Minimap positioned at right edge of grid rect
  - Width: `minimap_width * scale_factor` (default 80px logical, configurable)
  - Height: full grid height
  - Inset from right edge by 2px for visual padding

- [ ] Bridge minimap texture into the compositor:
  - Allocate a stable `LayerId` for the minimap layer (one per pane) during `MinimapRenderer::new()` or on first activation
  - **Key integration step:** The minimap texture view (from `PersistentTextureCache`) must be wrapped in a compositor group-2 bind group. Use `CompositionPass::create_texture_bind_group(device, &persistent_texture_view)` to create the bind group. This bind group must be cached on `MinimapRenderer` and recreated when the texture is resized (same invalidation as `bind_group` field already tracks).
  - Register a custom `TextureAssignment` in the compositor's `layer_textures` map (this may require adding a method like `register_external_texture(layer_id, view, bind_group)` to `GpuCompositor` since the current `ensure_layer_target` path goes through `RenderTargetPool`). Alternatively, bypass the compositor entirely and render the minimap quad directly in the overlay render pass using the `CompositionPass::draw_layers()` method with a manually constructed `CompositeLayerDesc`.
  - When minimap is disabled or alt-screen: remove the layer (or skip composition). No residual compositor state.
  - The layer uses `Opacity` compositing (semi-transparent overlay on top of terminal grid)

- [ ] Implement `render(encoder, compositor, ...)`:
  - If not active or texture not ready: skip
  - Render minimap texture as textured quad via existing `CompositionPass`:
    ```rust
    compositor.compose(queue, pass, &[
        LayerCompositeInfo {
            layer_id: minimap_layer_id,
            bounds: minimap_rect.into(),
            transform: IDENTITY,
            opacity: minimap_opacity,
        }
    ]);
    ```
  - Render viewport highlight: semi-transparent rectangle (e.g., `rgba(255, 255, 255, 0.15)`) at `model.viewport_highlight_rect(minimap_rect)` — pushed as an overlay quad

- [ ] Integrate into `WindowRenderer::render_cached()` in `render.rs` (currently 441 lines — adding minimap calls here risks exceeding 500 lines):
  - After copying content cache to surface (existing step)
  - Before cursor/overlay pass: update minimap texture + render minimap overlay
  - Only executes when minimap is enabled in config and not alt-screen
  - **If render.rs would exceed 500 lines:** extract the minimap render integration into a separate method in `oriterm/src/gpu/window_renderer/minimap_pass.rs` (new submodule), called from `render_cached()`. The method receives the encoder, compositor, minimap renderer, and snapshot references.

- [ ] Register minimap module tree: add `pub(crate) mod minimap;` to `oriterm/src/gpu/mod.rs`. The `minimap/mod.rs` file declares `mod model;`, `mod renderer;`, `mod input;` submodules and re-exports the public-crate types.

- [ ] Respects `ScaleFactor` for crisp rendering at all DPI levels.

- [ ] Source files under 500 lines each. Tests in sibling `tests.rs` per test-organization.md.

- [ ] **TPR checkpoint** — `/tpr-review` covering 55.1–55.3 implementation

- [ ] **Test matrix:**
  - Dirty bands: upload 5 dirty bands, verify only those regions written to texture
  - Full rebuild: verify entire texture uploaded
  - No changes: verify zero GPU work (no write_texture calls)
  - Viewport highlight position at various display_offsets
  - DPI scaling: verify minimap width scales with scale_factor
  - Compositor layer lifecycle: layer registered when minimap activates, removed when deactivated
  - **Semantic pin:** minimap renders nothing when alt-screen active (no texture upload, no overlay quad)

- [ ] Verify all tests pass: `timeout 150 cargo test -p oriterm -- minimap_renderer`

---

## 55.4 Input & Interaction

**File(s):** `oriterm/src/gpu/minimap/input/mod.rs`, `oriterm/src/gpu/minimap/input/tests.rs`

Mouse interaction with the minimap: click-to-scroll, drag viewport highlight, hover effects. Minimap clicks are intercepted before PTY mouse reporting — uses capture-phase handling similar to `ScrollbarCaptureController`.

- [ ] Write failing tests first: `input_click_scrolls_to_position`, `input_drag_updates_offset`, `input_hover_expands`, `input_not_forwarded_to_pty`

- [ ] Define `MinimapInputState` (the minimap is a GPU-only overlay outside the widget hit-test tree, so InteractionManager cannot track it — same pattern as `ScrollbarCaptureController`):
  ```rust
  pub(crate) struct MinimapInputState {
      /// Whether mouse is currently over the minimap area.
      /// Tracked manually because the minimap is a GPU overlay outside
      /// the widget tree (same pattern as ScrollbarCaptureController).
      hovered: bool,
      /// Active drag state.
      dragging: Option<MinimapDragState>,
  }

  struct MinimapDragState {
      /// Initial display_offset when drag started.
      start_offset: usize,
      /// Initial mouse Y position when drag started.
      start_y: f32,
  }
  ```

- [ ] Implement hit testing: `is_in_minimap(mouse_pos: Point, minimap_rect: Rect) -> bool`
  - Checks if mouse position falls within the minimap screen-space rect
  - Extended by hit slop (4px) on the left edge for easier acquisition

- [ ] Implement click-to-scroll:
  - On mouse down within minimap rect (outside viewport highlight): compute target row via `model.minimap_row_to_source_row()`, set `display_offset` to center viewport at that row
  - Scroll position update sent via existing event channel (not direct grid mutation)

- [ ] Implement drag viewport highlight:
  - On mouse down within viewport highlight rect: enter drag state, capture mouse
  - On mouse move during drag: compute delta in minimap pixels, convert to row delta via sampling ratio, update `display_offset` proportionally
  - On mouse up: exit drag state, release capture
  - Use `drag_delta_to_offset` pattern from existing `scrollbar/mod.rs:324`

- [ ] Implement hover effects:
  - Mouse enter minimap rect: set `hovered = true`, expand minimap width (default 80px → 96px on hover, matching scrollbar hover expansion pattern)
  - Mouse leave: set `hovered = false`, start fade timer if configured

- [ ] Capture-phase mouse handling:
  - Minimap clicks MUST NOT be forwarded to PTY mouse reporting
  - Follow `ScrollbarCaptureController` pattern from `oriterm_ui/src/controllers/scrollbar_capture/`
  - The minimap overlay is not in the widget hit-test tree (exists only in the GPU paint layer), so capture-phase interception is required

- [ ] Mouse wheel passthrough: wheel events over the minimap area scroll normally (not consumed by minimap)

- [ ] Source file under 500 lines. Tests in sibling `tests.rs`.

- [ ] **Test matrix:**
  - Click at top of minimap → display_offset = max (top of scrollback)
  - Click at bottom → display_offset = 0 (latest content)
  - Click in middle → proportional offset
  - Drag highlight downward → display_offset decreases (scroll toward bottom)
  - Drag highlight upward → display_offset increases (scroll toward top)
  - Mouse in minimap area while PTY mouse mode active → click NOT sent to PTY
  - Mouse wheel over minimap → normal scroll behavior (passthrough)
  - **Semantic pin:** minimap click produces a different display_offset than the current one (actually scrolls)

- [ ] Verify all tests pass: `timeout 150 cargo test -p oriterm -- minimap_input`

---

## 55.5 Configuration & Integration

**File(s):** `oriterm/src/config/rendering.rs` (add `ScrollbarMode` enum + `minimap_width` + `minimap_overlay` fields — currently 49 lines, ample room), `oriterm/src/config/mod.rs` (add fields to `Config` struct — currently 423 lines, tight but fits if adding only 2-3 fields), `oriterm/src/gpu/minimap/mod.rs` (minimap module root — re-exports + feature gating)

User-configurable minimap behavior and integration with the rest of the terminal.

- [ ] Write failing tests first: `config_scrollbar_mode_default`, `config_scrollbar_mode_minimap_hides_scrollbar`, `config_alt_screen_hides_minimap`, `config_hot_reload_mode_change`, `config_minimap_overlay_vs_inset`

- [ ] Add `scrollbar_mode` config option:
  ```toml
  [appearance]
  # Scrollback navigation control: none, scrollbar, minimap, both
  scrollbar_mode = "minimap"

  # Minimap width in logical pixels (default: 80)
  minimap_width = 80
  ```
  - `none` — no scrollbar or minimap
  - `scrollbar` — traditional overlay scrollbar (Section 48)
  - `minimap` — minimap only (this section)
  - `both` — minimap + scrollbar (scrollbar overlaid on minimap's left edge)

- [ ] Config hot-reload: minimap visibility updates immediately on config change
  - Switching from `none` → `minimap`: triggers full minimap rebuild
  - Switching from `minimap` → `none`: releases GPU texture, stops minimap work

- [ ] Alt-screen integration:
  - When terminal enters alt screen (`swap_alt`): hide minimap, stop updates
  - When terminal exits alt screen: show minimap, trigger full rebuild (scrollback may have changed)

- [ ] Resize integration:
  - Column resize (reflow): full minimap rebuild (all rows invalidated)
  - Row resize (more/fewer visible rows): update viewport highlight size, no full rebuild
  - Window resize: recompute `minimap_rect` position

- [ ] Layout integration:
  - Default overlay mode (`minimap_overlay = true`): minimap does NOT consume terminal columns — it overlays on the right edge of the grid, composited on top of content
  - Config `minimap_overlay = true | false` (default `true`): when `true`, minimap overlays on top of grid content (text renders underneath); when `false`, grid column count reduced by minimap width so text never renders under the minimap. Both modes must work correctly.
  - **Inset mode (`minimap_overlay = false`) implementation:** the column reduction must happen in `compute_window_layout()` (`oriterm/src/session/compute/`), which computes the grid rect. Subtract `minimap_width` from the grid rect's right edge before computing column count. This triggers a PTY resize (fewer columns) — the IO thread reflows. The minimap then renders in the reclaimed space (not overlaid). This is a significant layout change and must be tested with reflow.

- [ ] Update Section 48 (Native Scrollbar) interaction (note: Section 48 is Tier 5 and may be implemented first — if so, Section 48 defines `ScrollbarMode` with `none | scrollbar`, and this section extends it to add `minimap | both`):
  - When `scrollbar_mode = both`: scrollbar renders on the minimap's left edge
  - When `scrollbar_mode = scrollbar`: Section 48 behavior unchanged
  - The `scrollbar_mode` config replaces Section 48's separate `[appearance] scrollbar` config
  - **Concrete steps:** (1) Define `ScrollbarMode` enum in config module with all four variants from day one (even if minimap/both are no-ops until Section 55 lands). (2) Section 48 implements behavior for `none` and `scrollbar` variants. (3) Section 55 implements behavior for `minimap` and `both` variants. (4) Both sections gate on the same enum — no separate config keys.

- [ ] **Test matrix:**
  - `none` mode: no minimap rendered, no GPU texture allocated
  - `minimap` mode: minimap visible, scrollbar hidden
  - `scrollbar` mode: scrollbar visible, minimap hidden
  - `both` mode: both visible, scrollbar on minimap left edge
  - Hot-reload: switch modes at runtime, verify correct behavior
  - Alt-screen: enter vim, verify minimap hidden; exit, verify minimap restored
  - Resize: change window width, verify minimap rect updates
  - **Semantic pin:** switching from `minimap` to `none` releases the pane's GPU texture (memory reclaimed)

- [ ] Verify all tests pass: `timeout 150 cargo test -p oriterm -- scrollbar_mode`

- [ ] **TPR checkpoint** — `/tpr-review` covering 55.4–55.5 implementation

---

## 55.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- None.

---

## 55.N Completion Checklist

- [ ] Pure `sample_row()` in `oriterm_core`; snapshot production orchestration in `oriterm_mux`
- [ ] MinimapSnapshot travels via dedicated double buffer (generic `SnapshotDoubleBuffer<MinimapSnapshot>` or purpose-built `MinimapDoubleBuffer` — NOT inside RenderableContent) and PaneSnapshot (daemon)
- [ ] MinimapModel maps scrollback rows ↔ minimap pixels with sampling
- [ ] History damage tracking via minimap-specific `BitVec` dirty tracker on IO thread (separate from grid's viewport `DirtyTracker`)
- [ ] Skipped snapshots escalate to all_dirty (never lose deltas)
- [ ] Sampling-ratio computed by IO thread (single authority), carried in MinimapSnapshot, read by MinimapModel
- [ ] Sampling-ratio changes force full minimap rebuild
- [ ] Initial enable (first invocation, prev_snapshot=None) seeds full minimap from existing scrollback
- [ ] GPU minimap texture updated incrementally via PersistentTextureCache
- [ ] Minimap composited as overlay via existing CompositionPass
- [ ] Viewport highlight accurately shows current visible region
- [ ] Click-to-scroll and drag-to-scroll work correctly
- [ ] Minimap clicks NOT forwarded to PTY mouse reporting (capture-phase)
- [ ] `scrollbar_mode` config: none / scrollbar / minimap / both
- [ ] Alt-screen hides minimap; exit restores it
- [ ] Resize triggers full rebuild
- [ ] Zero minimap work when content unchanged
- [ ] Works identically in embedded and daemon modes
- [ ] `timeout 150 cargo test -p oriterm -- minimap` passes
- [ ] `timeout 150 cargo test -p oriterm_core -- minimap` passes
- [ ] `timeout 150 cargo test -p oriterm_mux -- minimap` passes
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./test-all.sh` green
- [ ] Source files under 500 lines each
- [ ] Tests in sibling `tests.rs` files per test-organization.md
- [ ] Plan annotation cleanup: all temporary scaffolding removed from `.rs` files
- [ ] All intermediate TPR checkpoint findings resolved
- [ ] **Plan sync** — update plan metadata:
  - [ ] This section's frontmatter `status` → `complete`, subsection statuses updated
  - [ ] `00-overview.md` Quick Reference table updated
  - [ ] `index.md` section status updated
  - [ ] Cross-links to Section 48 updated if scrollbar_mode config replaces Section 48's config
- [ ] `/tpr-review` passed (final, full-section)
- [ ] `/impl-hygiene-review last commit` passed — MUST run AFTER `/tpr-review` is clean

**Exit Criteria:** A minimap panel on the right edge of the terminal shows a zoomed-out overview of the full scrollback as colored pixel blocks. The viewport highlight accurately tracks the visible region. Click and drag navigate scrollback. The minimap updates incrementally (only dirty rows), does zero work when idle, hides on alt-screen, works in daemon mode, and is configurable via `scrollbar_mode`. `./test-all.sh` green with 0 regressions.
