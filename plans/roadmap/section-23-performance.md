---
section: 23
title: Performance & Damage Tracking
status: in-progress
reviewed: true
tier: 5
goal: Optimize rendering, parsing, and memory for heavy terminal workloads
sections:
  - id: "23.1"
    title: Damage Tracking
    status: in-progress
  - id: "23.2"
    title: Parsing Performance
    status: not-started
  - id: "23.3"
    title: Memory Optimization
    status: not-started
  - id: "23.4"
    title: Rendering Performance
    status: not-started
  - id: "23.5"
    title: Benchmarks
    status: not-started
  - id: "23.6"
    title: Section Completion
    status: not-started
---

# Section 23: Performance & Damage Tracking

**Status:** Not Started
**Goal:** Terminal handles heavy workloads (large file cats, rapid output, complex TUIs) without lag, dropped frames, or excessive memory usage. Every optimization must be measurable — no speculative "optimization" without profiling.

**Crate:** `oriterm_core` (grid damage tracking, ring buffer), `oriterm_mux` (PTY event loop, VTE processing), `oriterm` (rendering)
**Dependencies:** `criterion` (benchmarks), `wgpu` (GPU), `vte` (parser)

**Reference:**
- Alacritty's "fastest terminal emulator" design: batched rendering, ring buffer, SIMD parsing
- Ghostty's page-based memory management and damage tracking
- VTE library's parser performance (state machine, table-driven dispatch)

**Current state:** wgpu GPU-accelerated rendering with instanced draw calls (80-byte stride), glyph texture atlas (2048x2048, up to 4 pages, R8Unorm + Rgba8Unorm + Rgba8UnormSrgb) with lazy rasterization and LRU page eviction, multi-pass pipeline (bg rects, mono glyphs, subpixel glyphs, color glyphs, images, cursors, plus chrome and overlay tiers). `ScrollbackBuffer` ring buffer with O(1) push/eviction (already implemented in `oriterm_core/src/grid/ring/mod.rs`). Line-level `DirtyTracker` already exists (`oriterm_core/src/grid/dirty/mod.rs`) with `mark`, `mark_all`, `mark_range`, and `drain`. PTY reader uses a 1 MB read buffer with 64 KB max-locked-parse batching (`oriterm_mux/src/pty/event_loop/mod.rs`). `DamageLine` type exists in renderable layer but currently always reports `left=0, right=cols-1` (no column-level bounds). **Per-pane `PreparedFrame` caching** already exists (`oriterm/src/gpu/pane_cache/mod.rs`): `PaneRenderCache` stores one `PreparedFrame` per pane and skips re-preparing clean panes (checked via `dirty` flag + layout comparison). **Frame-level throttling** already exists: `FRAME_BUDGET = 16ms` in `oriterm/src/app/mod.rs`, enforced in `about_to_wait()` — rendering only proceeds when `any_dirty && budget_elapsed`. Per-window `ctx.dirty` flag gates rendering; `MuxWakeup` sets all windows dirty without calling `request_redraw()`. **Synchronized output** (Mode 2026): PTY event loop checks `sync_bytes_count()` and suppresses `Wakeup` events while sync mode is active. Remaining work: row-level dirty-skip within the prepare phase (currently rebuilds all cells even for clean panes marked dirty), column-level damage bounds, and idle-terminal render skipping.

---

## 23.1 Damage Tracking

Only redraw cells that changed since last frame. Currently the prepare phase (`prepare_pane_into()` / `prepare_frame()`) iterates every visible cell every frame (~120 cols x 30 rows = 3,600 cells), building instance data even when nothing changed. For idle terminals this is wasted GPU work.

**Files:** `oriterm_core/src/grid/dirty/mod.rs` (already exists), `oriterm_core/src/grid/row/mod.rs`, `oriterm/src/gpu/window_renderer/` (render pipeline), `oriterm/src/gpu/prepare/` (instance buffer generation), `oriterm/src/gpu/pane_cache/mod.rs` (per-pane frame caching — already exists), `oriterm/src/gpu/frame_input/mod.rs` (`needs_full_repaint()` — already exists, currently `#[allow(dead_code)]` — wire this as the gate for full vs. incremental prepare)

**Reference:** `_old/src/grid/dirty.rs`, `_old/src/grid/row.rs`, `_old/src/gpu/renderer.rs`, Ghostty `src/terminal/page.zig` (Row.dirty) + `src/terminal/render.zig` (RenderState), Alacritty `alacritty_terminal/src/term/mod.rs` (dirty state)

### Per-Row Dirty Flag

**Status: Already implemented.** `DirtyTracker` exists at `oriterm_core/src/grid/dirty/mod.rs` with `mark(line)`, `mark_range(Range)`, `mark_all()`, `drain()` → `DirtyIter`, `is_any_dirty()`, `is_dirty(line)`, `resize()`. Grid operations (`put_char`, `erase_line`, `erase_display`, `scroll_up`, `scroll_down`, `scroll_display`, `move_cursor_line`, `move_cursor_col`, `resize`) already mark dirty appropriately.

- [x] Add dirty tracking to the grid
  - [x] Mark dirty on: `put_char`, cell write
  - [x] Mark dirty on: `erase_line`, `erase_display`, `erase_chars`
  - [x] Mark all rows in scroll region dirty on: `scroll_up`, `scroll_down`
  - [x] Mark old cursor row + new cursor row dirty on cursor movement (`move_cursor_line`, `move_cursor_col`)
  - [x] Mark all visible rows dirty on: scroll position change (`scroll_display`), resize, font change, color scheme change
- [x] Correctness invariant: every code path that mutates a cell must mark the containing row dirty. Missing a path causes stale rendering — this is the main risk.

### Instance Buffer Caching

**Partially implemented.** `PaneRenderCache` (`oriterm/src/gpu/pane_cache/mod.rs`) already caches per-pane `PreparedFrame`s. On each frame, `get_or_prepare()` checks `dirty` flag + layout match; clean panes reuse their cached frame without re-preparing. Cache is invalidated on: pane close (`remove`), font/atlas change (`invalidate_all`), palette change (`invalidate`).

**What's done:**
- [x] Per-pane `PreparedFrame` caching with dirty + layout invalidation
- [x] `PaneRenderCache::get_or_prepare()` — cache hit returns existing frame; miss calls `prepare_fn`
- [x] `invalidate_all()` for font/atlas changes; `remove()` for pane close
- [x] `retain_only()` for batch cleanup of stale entries

**Remaining — row-level dirty optimization within a pane's prepare pass:**
- [x] **File size warning:** `prepare/mod.rs` is 372 lines. All row-skip logic MUST go in a new `prepare/dirty_skip.rs` submodule, not inline in `fill_frame_shaped()`. Extract the per-row dirty check + cached instance merge into that submodule.
- [x] When a pane is dirty, `fill_frame_shaped()` still iterates ALL `input.content.cells` — even rows that haven't changed
- [x] Add row-level dirty skip in `fill_frame_shaped()` (in `prepare/dirty_skip.rs`). Start with the simple approach: check `is_dirty(row)` for each cell's row inside the cell loop and `continue` for clean rows. This still iterates all cells but skips the expensive per-cell instance generation (shaping, atlas lookup, decoration emit). Profile to verify this is sufficient before implementing the more complex filtered approach
- [x] For incremental updates, merge clean rows' cached instances with dirty rows' fresh instances:
  - [x] Add `row_ranges: Vec<Range<usize>>` to `PreparedFrame` mapping each visible row to its instance buffer index range
  - [x] On partial update: copy clean rows' instance ranges from the cached frame, regenerate only dirty rows' instances
  - [x] Wide chars produce fewer instances per row (spacer cells are skipped), so row-to-range mapping must be built during prepare, not pre-computed
- [x] Clear dirty flags after render via `Term::damage()` or `Term::reset_damage()` — must happen AFTER the prepare phase consumes the snapshot, not during snapshot extraction (which is a pure read)
- [x] On first frame or after a full-redraw trigger, rebuild the entire buffer

### Full Redraw Triggers

These events mark everything dirty (conservative — can optimize individual cases later):

**Already handled by `DirtyTracker.mark_all()` or `mark_range(0..lines)`:**
- [x] Window resize (`Grid::resize` calls `dirty.mark_all()`)
- [x] Scroll position change (`Grid::scroll_display` calls `dirty.mark_all()`)
- [x] Alt screen swap (`toggle_alt_common` calls `grid_mut().dirty_mut().mark_all()`)

**Already handled by `selection_dirty` flag on `Term`:**
- [x] Selection change — `Term::selection_dirty` is set by `put_char`, `erase_*`, `scroll_*`, `insert_blank`, `delete_chars`, `linefeed`, `insert_lines`, `delete_lines`, alt screen swap. Cleared by `clear_selection_dirty()`. Separate from grid dirty tracking — used to invalidate selection overlay rendering.

**Not yet handled (need to trigger `PaneRenderCache` invalidation or `mark_all()`):**
- [ ] Font size change (Ctrl+Plus/Minus) — currently handled by `invalidate_all()` on the pane cache, but verify grid dirty is also set
- [ ] Color scheme / palette change — verify both grid dirty and pane cache invalidation
- [ ] Tab switch (different tab has entirely different grid) — different pane ID, so pane cache naturally misses

### Skip Present When Clean

**Partially implemented.** `App::about_to_wait()` already checks `any_dirty && budget_elapsed` before calling `render_dirty_windows()`. Per-window `ctx.dirty` is set by `MuxWakeup`, cursor blink, animations. `FRAME_BUDGET = 16ms` prevents over-rendering.

- [x] Per-window `ctx.dirty` flag gates rendering (set by PTY output, input, blink, animations)
- [x] `FRAME_BUDGET` (16ms) time-based throttle prevents >60fps rendering
- [x] `MuxWakeup` marks all windows dirty without calling `request_redraw()` — just sets flags
- [ ] Refine: when `ctx.dirty` is set but NO grid rows are actually dirty AND cursor hasn't blinked AND no overlay changed, skip the prepare+render pass entirely (currently `ctx.dirty` is too coarse — any `MuxWakeup` marks the window dirty even if the PTY output was for a background tab)
- [ ] Track per-pane dirty flags so `MuxWakeup` only dirties the windows containing affected panes
- [ ] Idle terminal with no PTY output should produce zero GPU submissions (near-zero CPU)

- [x] **Tests** (`oriterm_core/src/grid/dirty/tests.rs`, `oriterm_core/src/grid/editing/tests.rs`):
  - [x] New grid: nothing dirty after initial build
  - [x] `put_char` marks the cursor row dirty
  - [x] `erase_line` marks the target row dirty
  - [x] `scroll_up` marks all rows in scroll region dirty
  - [x] Cursor move marks both old and new rows dirty
  - [x] `drain()` returns dirty row indices and resets all to clean
  - [x] After drain, `is_any_dirty()` returns false
  - [x] Resize marks all rows dirty

### Column-Level Damage Bounds

Refine damage granularity from full-line to column ranges. `DamageLine` already has `left` and `right` fields (in `oriterm_core/src/term/renderable/mod.rs`), but `TermDamage::next()` always fills them as `left=Column(0), right=self.right` (full-line). Alacritty tracks per-line `LineDamageBounds` with `left`/`right` that expand via `min(left)` / `max(right)` as cells are modified.

- [ ] **Structural change to `DirtyTracker`**: replace `dirty: Vec<bool>` with `dirty: Vec<LineDamageBounds>` where `LineDamageBounds { dirty: bool, left: usize, right: usize }`. This consolidates dirty state and column bounds into a single struct per line (no parallel Vec). If DirtyTracker exceeds 300 lines after this change, extract `LineDamageBounds` into `grid/dirty/column_damage.rs`
  - [ ] **Sync points that call `mark(line)` must be updated to call `mark_col(line, col)` or `mark_col_range(line, left, right)`:**
    - [ ] `Grid::put_char()` — mark column at cursor position
    - [ ] `Grid::erase_line()` — mark affected column range
    - [ ] `Grid::erase_chars()` — mark cursor to cursor+count
    - [ ] `Grid::insert_blank()` — mark cursor to end of line (content shifts right)
    - [ ] `Grid::delete_chars()` — mark cursor to end of line (content shifts left)
    - [ ] `Grid::clear_range_on_row()` — mark the cleared column range
    - [ ] `Grid::scroll_up/scroll_down` — keep as full-line dirty (all columns affected)
    - [ ] `Grid::move_cursor_col/move_cursor_line` — keep as full-line for cursor row
- [ ] Initial undamaged state: `left=cols, right=0` (inverted → `is_damaged = left <= right`)
- [ ] Each cell mutation calls `expand(col, col)`: `left = min(left, col)`, `right = max(right, col)`
- [ ] Erase/delete operations: expand from cursor to affected range end
- [ ] `collect_damage()` in `oriterm_core/src/term/renderable/mod.rs` must propagate column bounds into `DamageLine` (currently hardcodes `left=Column(0), right=self.right`)
- [ ] `TermDamage::next()` must read column bounds from the tracker instead of using full-line defaults
- [ ] Renderer uses column bounds to skip unchanged cells within a row
- [ ] **Tests** (in `oriterm_core/src/grid/dirty/tests.rs` — extend existing sibling test file):
  - [ ] Write single char → damage bounds cover only that column
  - [ ] Write two chars at different columns → bounds expand to cover both
  - [ ] Erase chars → bounds cover erase range
  - [ ] Full-line operations still report `left=0, right=cols-1`

### Selection Damage Tracking

When selection changes, only damage the affected lines rather than forcing a full redraw. Alacritty tracks `old_selection` and diffs against the new selection to determine which lines need redrawing.

**Existing mechanism:** `Term::selection_dirty` (bool) is set by any grid mutation that could invalidate a selection (put_char, erase, scroll, insert, delete, linefeed, alt screen swap). Checked via `is_selection_dirty()`, cleared via `clear_selection_dirty()`. This flag tells the renderer "selection might be stale" but does NOT indicate which lines are affected.

- [ ] Store previous selection range (start line, end line) after each frame
- [ ] On selection change, compute the symmetric difference of old and new selection line ranges
- [ ] Damage only lines in the symmetric difference (lines that changed selection state)
- [ ] Selection clear damages only the previously-selected lines (not the whole grid)
- [ ] Selection drag damages only the incrementally changed lines (not the entire selection)
- [ ] **Integration with `selection_dirty`:** when `is_selection_dirty()` returns true AND the selection has been mutated by grid operations (not user drag), fall back to full-selection-range damage since the selection endpoints may have shifted due to scrolling
- [ ] **Tests** (in `oriterm_core/src/term/tests.rs` — extend existing sibling test file, or a new `oriterm_core/src/term/selection_damage/tests.rs` if the module is extracted):
  - [ ] New selection damages only the selected lines
  - [ ] Extending selection damages only the newly-covered lines
  - [ ] Clearing selection damages only the previously-selected lines
  - [ ] Grid mutation (put_char) while selection active sets `selection_dirty` and damages selection lines

### Snapshot Extraction Optimization

`Term::renderable_content_into()` (`oriterm_core/src/term/snapshot.rs`) currently iterates ALL visible cells every frame, pushing `RenderableCell`s into a flat `Vec<RenderableCell>`. This is the main bottleneck for damage-aware rendering — even if `fill_frame_shaped()` could skip clean rows, the snapshot extraction has already done O(rows * cols) work.

**Rendering discipline warning:** `renderable_content_into()` takes `&self` (immutable). It can READ `dirty.is_dirty(line)` to skip clean rows, but must NOT call `drain()` or mutate the tracker. The dirty state is consumed by `Term::damage()` or `Term::reset_damage()` after the render pipeline is done with the snapshot. This two-phase design (read-then-clear) is intentional and must be preserved.

- [ ] **Profile first:** measure `renderable_content_into()` wall time for a 120x50 grid and a 240x80 grid. If extraction is <0.5ms, defer this optimization and focus on the prepare phase (23.1 Instance Buffer Caching) instead
- [ ] If extraction is a bottleneck (>0.5ms), add a `content.dirty_lines: Vec<usize>` field to `RenderableContent` populated from `DirtyTracker::is_dirty()` (read-only). Keep extracting all cells (the prepare phase needs all cells for bg rendering), but provide the dirty line list so `fill_frame_shaped()` can skip clean rows. This avoids splitting extraction into two modes while still enabling the downstream optimization
- [ ] `zerowidth.clone()` in extraction: allocates per-cell only for cells with combining marks (<1% of cells). The common case (`Vec::new()`) is zero-cost. Profile to confirm this is negligible before optimizing. If it matters, change `RenderableCell::zerowidth` to `SmallVec<[char; 2]>` (covers 99%+ of combining mark cases without heap allocation)
- [ ] `collect_damage()` second pass: O(lines) overhead vs. O(lines*cols) cell extraction. The second pass is negligible by comparison. Do not merge into the cell loop unless profiling shows otherwise (merging couples damage collection with cell extraction, reducing independent optimization)

### Insert Mode Damage Interaction

When INSERT mode (IRM) is active, cell insertions shift existing content right, which can affect the entire line from cursor to right margin. Alacritty forces full damage when INSERT mode is active during `damage()`.

- [ ] When INSERT mode is active, force full-line damage for the cursor row on any cell write
- [ ] On `unset_mode(Insert)`, mark all lines dirty (exit from insert mode may have left stale damage)
- [ ] **Tests** (in `oriterm_core/src/grid/dirty/tests.rs` or `oriterm_core/src/term/handler/tests.rs` depending on where the mode logic is implemented):
  - [ ] Write char in INSERT mode → full line damaged
  - [ ] Exit INSERT mode → all dirty

### Dependencies Between Subsections

The optimizations in this section have a dependency order. Do not start later items before their prerequisites are verified.

**Crate ordering rule:** All `oriterm_core` changes must be implemented and tested before dependent `oriterm` changes. `oriterm_mux` changes (23.2) are independent of `oriterm` changes (23.4) and can proceed in parallel. This follows the impl-hygiene rule: library crate before binary crate.

1. **Per-Row Dirty Flag** (done, `oriterm_core`) → **Instance Buffer Caching** (row-level dirty skip in `fill_frame_shaped`, `oriterm`)
2. **Instance Buffer Caching** (`oriterm`) → **Instance Buffer Partial Updates** (23.4, `oriterm` — needs row-to-instance mapping)
3. **Per-Row Dirty Flag** (done, `oriterm_core`) → **Column-Level Damage Bounds** (structural change to `DirtyTracker`, `oriterm_core`)
4. **Column-Level Damage Bounds** (`oriterm_core`) → **Snapshot Extraction Optimization** (skip clean rows in `renderable_content_into`, `oriterm_core`)
5. **Selection Damage Tracking** and **Insert Mode Damage** are independent of column-level bounds (both `oriterm_core`)
6. **Skip Present When Clean** requires Instance Buffer Caching to be effective (otherwise every frame still does full prepare, `oriterm`)
7. **Alt Screen On-Demand Allocation** (23.3, `oriterm_core`) is independent — can be implemented any time

---

## 23.2 Parsing Performance

Optimize VTE sequence parsing throughput for high-volume output.

**Files:** `oriterm_mux/src/pty/event_loop/mod.rs` (PTY read loop, VTE processing), `oriterm_core/src/term/handler/mod.rs` (VTE handler impl)

**Reference:** `_old/src/tab/mod.rs`, `_old/src/term_handler/mod.rs`, Alacritty `alacritty_terminal/src/event_loop.rs` (PTY read loop), Ghostty `src/terminal/stream.zig` (SIMD-optimized stream processing) + `src/simd/vt.zig`

### Batch Processing

**Status: Already implemented.** `PtyEventLoop::parse_chunk()` calls `self.processor.advance(term, chunk)` on the full chunk (up to 64 KB). The PTY reader reads into a 1 MB buffer and parses in bounded chunks — no byte-by-byte processing.

- [x] Entire PTY read buffer processed in one `processor.advance()` call (not byte-by-byte)
- [x] PTY reader uses shared `Arc<FairMutex<Term<T>>>` — no channel-based `Vec<u8>` transfer needed

### Increase PTY Read Buffer

**Status: Already implemented.** `READ_BUFFER_SIZE = 0x10_0000` (1 MB) in `oriterm_mux/src/pty/event_loop/mod.rs`, with `MAX_LOCKED_PARSE = 0x1_0000` (64 KB) per lock acquisition. Matches Alacritty's approach. The reader drains PTY data into the 1 MB buffer, then parses in bounded 64 KB chunks under the terminal lock.

- [x] Current: 1 MB read buffer with 64 KB max locked parse (matching Alacritty)
- [x] Read-ahead pattern: reader drains PTY into buffer even when terminal lock is held
- [x] Prevents ConPTY back-pressure on Windows from cascading into hangs

### Fast ASCII Path

`vte::ansi::Processor::advance()` already batches consecutive printable characters into a single `input(&str)` call. The per-character overhead is in `Term::input()` which calls `grid.put_char(c)` per character — each call does `UnicodeWidthChar::width()` (always 1 for ASCII), dirty marking, and cell construction.

- [ ] **Profile first:** benchmark `Term::input()` with a 64 KB ASCII-only string. If throughput is >200 MB/s, skip this optimization. If <200 MB/s, proceed
- [ ] Add `Grid::put_ascii_run(s: &str)` that batch-writes ASCII cells: skip `UnicodeWidthChar::width()` (hardcode width=1), write cells in a tight loop without per-character dirty marking, mark the affected row(s) dirty once at the end
- [ ] In `Term::input()`, detect ASCII-only input (all bytes in 0x20..=0x7E) and call `put_ascii_run()` instead of per-character `put_char()`
- [ ] **FairMutex constraint:** the fast path runs under the terminal lock within the `MAX_LOCKED_PARSE = 64 KB` window. The batch write must not exceed this per-chunk budget

### Reduce Allocations in Hot Path

- [ ] Verify `input()` handler writes directly to grid cells (no intermediate `String` allocation) by reading `term/handler/mod.rs::input()`
- [ ] Audit hot-path allocations with DHAT or a tracking allocator during `cat 100MB_file.txt`:
  - [ ] **Known:** `log::trace!` in `try_parse()` calls `String::from_utf8_lossy()` which allocates for non-UTF8 data. Zero-cost at default log level (lazy evaluation). If trace logging is enabled during benchmarks, gate with `if log::log_enabled!(log::Level::Trace)`
  - [ ] Verify `log::debug!()`/`log::trace!()` macros are compiled out at release log level (standard `log` crate does this, but verify no custom wrapper macros bypass lazy evaluation)
  - [ ] Check scroll/reflow for temporary `Vec` allocations — `scroll_up()`, `scroll_down()`, `resize()` should reuse existing row allocations, not create new ones
  - [ ] `format!()` calls in error paths are acceptable (errors are rare and not in the hot path)

### Throttle Rendering During Heavy Output

**Largely implemented.** Three throttling mechanisms are already in place:

1. **`FRAME_BUDGET = 16ms`** (`oriterm/src/app/mod.rs:83`): `about_to_wait()` checks `now.duration_since(self.last_render) >= FRAME_BUDGET` before rendering. Limits to ~60fps.
2. **`MuxWakeup` coalescing** (`oriterm/src/app/event_loop.rs`): `MuxWakeup` just sets `ctx.dirty = true` for all windows. Actual rendering happens in `about_to_wait()` after all pending events are drained. Multiple `MuxWakeup` events between frames produce one render.
3. **Synchronized output suppression** (`oriterm_mux/src/pty/event_loop/mod.rs`): PTY event loop checks `sync_bytes_count()` — when Mode 2026 is active, `Wakeup` events are suppressed until the sync buffer is flushed, preventing partial-frame rendering.

- [x] Do not request a redraw for every PTY output chunk — `MuxWakeup` sets flag only
- [x] Coalesce: `about_to_wait()` processes `pump_mux_events()` then renders once
- [x] Time-based throttle: `FRAME_BUDGET = 16ms` enforced in `about_to_wait()`
- [x] Synchronized output (Mode 2026): PTY reader suppresses `Wakeup` while sync buffer active
- [ ] Test final-frame edge case: run `seq 1 100000`, verify the terminal shows the last line ("100000") after output completes. If the final `MuxWakeup` arrives just after a render and no subsequent wakeup triggers a redraw, add a "trailing render" — when `pump_mux_events()` returns with data processed, always set `ctx.dirty`
- [ ] Verify `thread::yield_now()` between parse cycles allows the UI thread to snapshot terminal state during sustained PTY floods. Run `yes | head -1000000` and confirm the terminal remains interactive (responds to Ctrl+C within 100ms)

- [ ] **Tests** (allocation-free verification in `oriterm_core/benches/grid.rs` or a new `oriterm_mux/benches/parsing.rs`; integration tests in `oriterm_mux/src/pty/event_loop/tests.rs`):
  - [ ] Processing a 1MB ASCII buffer does not allocate (use a custom allocator or `#[global_allocator]` tracking in a benchmark)
  - [ ] All PTY data is processed even when rendering is throttled (no data loss)
  - [ ] Synchronized output: no partial-frame renders while Mode 2026 is active

---

## 23.3 Memory Optimization

Control memory usage, especially for scrollback. The ring buffer (`ScrollbackBuffer`) is already in place. Remaining targets: row-level memory optimization, lazy alt screen allocation, and grid resize memory reuse.

**Files:** `oriterm_core/src/grid/ring/mod.rs` (already exists), `oriterm_core/src/grid/row/mod.rs`, `oriterm_core/src/grid/mod.rs`, `oriterm_core/src/term/mod.rs` (Term struct — `alt_grid` field), `oriterm_core/src/term/alt_screen.rs` (alt screen swap logic), `oriterm_core/src/term/handler/esc.rs` (RIS reset accesses `alt_grid`)

**Reference:** `_old/src/grid/ring.rs`, `_old/src/grid/row.rs`, Alacritty `alacritty_terminal/src/grid/storage.rs` (ring buffer), Ghostty `src/terminal/PageList.zig` (page linked list + memory pools) + `src/terminal/page.zig` (contiguous page layout)

### Ring Buffer for Scrollback

**Status: Already implemented.** `ScrollbackBuffer` in `oriterm_core/src/grid/ring/mod.rs`:
```rust
struct ScrollbackBuffer {
    inner: Vec<Row>,
    max_scrollback: usize,
    len: usize,
    start: usize, // index of the oldest row when full
}
```
- [x] O(1) push: when full, `mem::replace` at `start` index, advance `start` wrapping
- [x] O(1) index via `physical_index()`: logical 0 = newest, `len - 1` = oldest
- [x] Incremental growth: Vec grows up to `max_scrollback`, then wraps (does NOT pre-allocate)
- [x] `clear()` clears inner Vec and resets `len`/`start`
- [x] `push()` returns evicted row (if full) for allocation recycling
- [x] `pop_newest()` for resize viewport restoration
- [x] `drain_oldest_first()` for reflow operations

### Row Memory Optimization

- [x] Row occupancy tracking (`occ` field) already implemented in `oriterm_core/src/grid/row/mod.rs` — `IndexMut` bumps occ, `reset()` uses occ-bounded iteration, `clear_range()`/`truncate()` maintain occ correctly
- [x] `CellExtra` uses `Option<Arc<CellExtra>>` — zero cost (8 bytes, null pointer) when not needed. Uses `Arc` (not `Box`) for O(1) clone of cursor template attributes via refcount bump.
  - [x] Less than 1% of cells typically have `CellExtra` (only cells with colored underlines, hyperlinks, or combining marks)
- [ ] Profile memory savings from compact blank rows: measure RSS with 10K lines of scrollback where >50% are blank. If blank rows consume >5 MB, implement `RowStorage` enum (`Blank { cols: usize }` vs. `Full(Vec<Cell>)`) that lazily expands on first cell write. If savings are <5 MB, skip this optimization (the branching cost on every `Index`/`IndexMut` is not worth marginal savings)
- [ ] Profile `SmallVec<[Cell; 80]>` vs. `Vec<Cell>` for Row storage: benchmark `Row::new(80)` allocation count with a custom allocator. `SmallVec<[Cell; 80]>` is `80 * 24 = 1920 bytes` inline, which exceeds typical stack budgets and is likely worse than a heap-allocated `Vec`. Only implement if profiling shows measurable allocation pressure during rapid resize cycles

### Alt Screen On-Demand Allocation

**Complexity warning:** `alt_grid` is accessed directly in 8+ locations across 4 files. The `Option` wrapper changes every access site. The critical invariant is: `mode.contains(ALT_SCREEN)` implies `alt_grid.is_some()`. This must be enforced by allocating in `swap_alt*()` BEFORE toggling the mode flag.

- [ ] Allocate the alternate screen buffer lazily — only when an application first switches to it (`DECSET 1049`)
  - [ ] Currently both primary and alt screen are allocated in `Term::new()` (`alt_grid: Grid::with_scrollback(lines, cols, 0)`)
  - [ ] Most terminals never enter alt screen (only editors, pagers, etc.)
  - [ ] Saves several MB per terminal that never uses alt screen
- [ ] Change `alt_grid: Grid` → `alt_grid: Option<Grid>` in `Term` struct
  - [ ] **Sync points that access `self.alt_grid` directly — ALL must handle `Option`:**
    - [ ] `Term::new()` — change from `Grid::with_scrollback(lines, cols, 0)` to `None`
    - [ ] `alt_screen.rs::swap_alt()` — allocate on first entry: `self.alt_grid.get_or_insert_with(|| Grid::with_scrollback(lines, cols, 0))`
    - [ ] `alt_screen.rs::swap_alt_no_cursor()` — same lazy allocation
    - [ ] `alt_screen.rs::swap_alt_clear()` — calls `self.alt_grid.reset()` before swap, must handle `None`
    - [ ] `alt_screen.rs::toggle_alt_common()` — swaps `image_cache`/`alt_image_cache` via `std::mem::swap`; if alt_grid is `None`, alt_image_cache should also be lazy
    - [ ] `Term::grid()` / `Term::grid_mut()` — when `ALT_SCREEN` mode is set, returns alt grid (must unwrap safely)
    - [ ] `Term::resize()` — resizes both grids (`self.alt_grid.resize()`); skip alt if `None`
    - [ ] `handler/esc.rs` — `self.alt_grid.reset()` (RIS full reset); must handle `None`
    - [ ] `snapshot.rs::renderable_content_into()` — only accesses active grid via `self.grid()`, should be safe
  - [ ] Also make `alt_image_cache: Option<ImageCache>` to match (swap with `image_cache` in `toggle_alt_common`)
- [ ] Deallocate alt screen when returning to primary screen (optional, configurable)
  - [ ] Or keep alive for fast re-entry (trade memory for speed)
- [ ] Reference: Ghostty 1.3.0 — "Alt screen allocated on-demand, saving several megabytes per terminal"
- [ ] **Tests** (in `oriterm_core/src/term/tests.rs` — extend existing sibling test file):
  - [ ] Fresh terminal: alt screen not allocated (measure with memory tracking)
  - [ ] Enter alt screen: alt screen allocated with correct dimensions
  - [ ] Exit alt screen: alt screen optionally deallocated
  - [ ] Resize before entering alt screen: alt screen still `None`, no crash
  - [ ] Enter alt → exit → re-enter: correct behavior regardless of deallocation policy

### Scrollback Memory Estimates

Reference calculations for validation during memory benchmarks (23.5):
- 24 bytes/cell x 120 cols x 10,000 rows = ~28.8 MB per tab (default scrollback)
- 24 bytes/cell x 120 cols x 100,000 rows = ~288 MB per tab (large scrollback)
- `CellExtra` on <1% of cells adds negligible overhead (8 bytes per cell that has one)

- [ ] Verify actual RSS matches these estimates in the memory benchmark (23.5). If RSS exceeds estimate by >20%, investigate hidden overhead (row metadata, Vec capacity slack, allocator fragmentation)
- [ ] Compressed scrollback for large histories (>100K lines) is deferred. The default 10K-line limit at ~28.8 MB is acceptable. Revisit if users request >100K lines as a configuration option

### Grid Resize Memory Reuse

- [ ] Add a `row_pool: Vec<Row>` field to `Grid` (capped at `lines` capacity). Populate it from `shrink_rows()` freed rows and `ScrollbackBuffer::push()` evicted rows. In `grow_rows()`, pop from the pool (calling `row.resize(cols)` + `row.reset()`) instead of allocating `Row::default()`
- [x] Column resize already resizes rows in place via `Row::resize()` (no new allocations)
- [ ] Profile rapid resize cycles: drag a window edge continuously for 5 seconds, measure allocation count with DHAT. If allocation count is >1000 and causes jank (>16ms frame time), the row pool is justified. If not, defer

- [x] **Tests** (`oriterm_core/src/grid/ring/tests.rs` — already exist):
  - [x] Push rows into ring, verify retrieval order (newest first via index 0)
  - [x] Ring wraps: push `capacity + 10` rows, only `capacity` retained, oldest evicted
  - [x] Index wraps correctly: `get(0)` is newest, `get(len-1)` is oldest
  - [x] Clear resets length to 0
  - [x] Verified: ring buffer does NOT pre-allocate — `inner` grows incrementally up to `max_scrollback` (this is correct; pre-allocation would waste memory)
  - [x] Integration: `grid.scroll_up()` pushes evicted row to ring buffer
  - [x] Memory: ring buffer does not grow beyond `capacity`

---

## 23.4 Rendering Performance

Optimize the GPU rendering pipeline for minimal CPU and GPU overhead per frame.

**Files:** `oriterm/src/gpu/window_renderer/` (render pipeline, prepare, draw calls), `oriterm/src/gpu/instance_writer/mod.rs`, `oriterm/src/gpu/atlas/mod.rs`, `oriterm/src/gpu/state/mod.rs`, `oriterm/src/gpu/prepare/` (instance buffer generation), `oriterm/src/gpu/pane_cache/mod.rs` (per-pane frame caching), `oriterm/src/gpu/compositor/mod.rs` (multi-layer composition), `oriterm/src/app/mod.rs` (`FRAME_BUDGET`, `mark_all_windows_dirty`)

**Reference:** `_old/src/gpu/renderer.rs`, `_old/src/gpu/instance_writer.rs`, `_old/src/gpu/atlas.rs`, Ghostty `src/renderer/Thread.zig` (120 FPS timer, coalescing), Alacritty `alacritty/src/renderer/` (renderer modules)

### Instance Buffer Partial Updates

**Current mechanism:** `WindowRenderer::upload_instance_buffers()` (`render.rs`) calls `upload_buffer()` for each pipeline (bg, fg, subpixel, color). `upload_buffer()` does `queue.write_buffer()` with the full buffer contents every frame. When the pane cache hits (clean pane), the prepare phase is skipped entirely — but the upload still sends the full cached buffer to the GPU.

**Complexity warning:** This is the highest-risk item in the section. `upload_buffer()` in `helpers.rs` already uses grow-only power-of-2 buffer allocation (recreates only when the existing buffer is too small), so buffers persist across frames when instance counts are stable. However, it always writes the full buffer contents at offset 0 via `queue.write_buffer()`. Partial updates require: (1) row-to-byte-offset mapping that accounts for variable-width characters, (2) selective `write_buffer()` calls with non-zero offsets for dirty regions only. Profile the full-buffer upload cost FIRST (120x50 grid = ~14K instances = ~1.1 MB at 80 bytes/instance). If upload is <0.5ms, skip this optimization entirely.

- [ ] **Profile first:** measure `upload_instance_buffers()` wall time for typical and stress-case terminal sizes before implementing partial updates
- [ ] With damage tracking (23.1), only rebuild instances for dirty rows within `fill_frame_shaped()`
- [ ] **Prerequisite**: row-to-instance-range mapping in `PreparedFrame` (see Instance Buffer Caching above)
- [ ] Use `wgpu::Queue::write_buffer()` with offset for partial buffer updates:
  - [ ] Calculate byte offset for the dirty row's instance range
  - [ ] Upload only the changed region, not the entire buffer
  - [ ] GPU buffers already persist across frames (grow-only power-of-2 allocation in `upload_buffer()`). The change is to call `write_buffer()` with a non-zero offset for dirty regions instead of always writing from offset 0
- [ ] Alternative: persistent mapped buffer with `wgpu::BufferUsages::MAP_WRITE | COPY_SRC`
  - [ ] Map once, write dirty regions, unmap before draw
  - [ ] May have better performance for frequent small updates
  - [ ] **Warning:** mapped buffer API varies across backends. Verify `MAP_WRITE | COPY_SRC` is supported on all three platforms (Windows/macOS/Linux) with the wgpu backends in use (Vulkan, Metal, DX12)
- [ ] When pane cache hits and the pane is clean, skip the GPU upload entirely (not just the prepare phase)
- [ ] Measure: compare full-buffer upload vs. partial update latency

### Glyph Atlas Growth

**Status: Already implemented.** `GlyphAtlas` in `oriterm/src/gpu/atlas/mod.rs` uses a pre-allocated `Texture2DArray` (2048x2048, up to 4 pages) with guillotine bin packing and LRU page eviction. Three atlas instances at runtime:
- **Monochrome** (`R8Unorm`): standard glyph alpha masks.
- **Subpixel** (`Rgba8Unorm`): LCD subpixel coverage masks (RGB/BGR).
- **Color** (`Rgba8UnormSrgb`): color emoji and bitmap glyphs.

- [x] Multi-page atlas with automatic page addition up to 4 pages
- [x] LRU page eviction when all pages are full
- [x] Color emoji support via separate `Rgba8UnormSrgb` atlas
- [ ] Add `log::debug!` in `GlyphAtlas::allocate()` when page utilization exceeds 80% (total allocated pixels / total page pixels)
- [ ] Stress-test with heavy Unicode workload (CJK + emoji + combining marks filling 240x80 grid). If 4 pages overflow, make max pages configurable via `GlyphAtlas::new(max_pages: u32)` with a default of 4

### Frame Pacing

**Partially implemented.** The event loop already uses `ControlFlow::Wait` (not `Poll`), waking only for events. `FRAME_BUDGET` throttles to ~60fps. `ctx.dirty` is set by specific events, not every iteration.

- [x] wgpu presentation handles VSync automatically (already in place)
- [x] `ControlFlow::Wait` — event loop sleeps when no events pending
- [x] `ctx.dirty` flag per window — only set by: `MuxWakeup` (PTY output), cursor blink timer, animation tick, input events
- [x] `FRAME_BUDGET` check before rendering — skip if <16ms since last render
- [x] Cursor blink timer: `cursor_blink.update()` only sets dirty when blink state changes
- [x] Animation timer: `layer_animator.tick()` only sets dirty when animations are active
- [ ] Refine `MuxWakeup` to only dirty windows containing active panes (currently marks ALL windows dirty — wasteful for multi-window setups where output is in one window)
- [ ] Verify `selection_dirty` propagates to `ctx.dirty`: trace the path from `Term::selection_dirty = true` to `ctx.dirty = true` in the event loop. If there is no path, add one (e.g., `MuxNotification::SelectionChanged`)
- [ ] Verify overlay updates (search bar, settings dialog) set `ctx.dirty`: trace each overlay's state mutation to confirm it reaches `ctx.dirty`. If any overlay mutates state without setting dirty, add the dirty call
- [ ] Profile idle terminal CPU usage: run a terminal with no PTY output for 30 seconds, measure CPU with `perf stat` or Activity Monitor. Target: <0.5% CPU (only cursor blink timer wakes the event loop)

### Draw Call Reduction

**Largely optimized.** The compositor (`oriterm/src/gpu/compositor/mod.rs`) manages multi-layer rendering with render target pooling. Each tier is already batched into per-pipeline instance buffers.

- [x] Each tier uses per-pipeline instance buffers (one draw call per pipeline per tier)
- [x] Compositor with render target pooling for multi-layer composition
- [ ] Audit current draw call count per frame (add a `draw_call_count` counter to `WindowRenderer::render()`, log at debug level). Current expected count: ~15+ per frame across three tiers (terminal: 7+, chrome: 4, overlay: 4 per overlay, plus per-image draws)
- [ ] Verify `record_draw()` / `record_draw_clipped()` skip the draw when instance count is zero. If not, add an `if instance_count == 0 { return; }` guard
- [ ] Image texture atlasing: when 3+ images are visible simultaneously, merge their textures into a shared atlas to reduce per-image bind group switches and draw calls. Deferred until image protocol support (Section 39) is implemented

### Resize Rendering Performance

- [ ] **BUG (Windows-only):** Dialog resize shows uninitialized surface (baby blue background) — GPU redraw during `WM_SIZING` modal loop is too slow. The render timer in `WM_ENTERSIZEMOVE` fires `WM_TIMER` which invalidates, but the redraw can't keep up with the resize rate. Investigate: frame budget during modal resize, whether the `WM_TIMER` approach is optimal, or if `WM_PAINT` handling during modal loop needs improvement. Affects both settings dialogs and main windows. Discovered during chrome plan verification (2026-03-10).
  - [ ] Fix must be behind `#[cfg(target_os = "windows")]` — macOS and Linux do not have this modal resize loop issue
  - [ ] Verify macOS and Linux resize behavior is smooth (no equivalent bug)

### Debug Overlay

- [ ] Optional FPS counter and dirty-row percentage in debug overlay
- [ ] Toggled via config flag or keyboard shortcut
- [ ] Shows: current FPS, dirty rows this frame, total instance count, atlas utilization
- [ ] Implement as a new overlay type in the compositor layer (similar to existing overlays)
- [ ] **File:** new `oriterm/src/gpu/debug_overlay/mod.rs` (keep under 500 lines)

### Skip Off-Screen Content

- [ ] In `fill_frame_shaped()`, skip generating instances for cells whose computed pixel position (after origin offset) falls entirely outside the render target bounds. This matters when: (a) a pane in a split layout extends beyond the window edge during resize, or (b) partially visible rows at the top/bottom edge of a pane
- [ ] Add a bounds check before `emit_cell()`: if `cell_y + cell_height < 0.0 || cell_y > viewport_height`, skip the cell

### Rendering Performance Tests

- [ ] **Tests** (unit tests in `oriterm/src/gpu/prepare/tests.rs` and `oriterm/src/gpu/atlas/tests.rs`):
  - [ ] Partial buffer update produces the same visual result as full rebuild: prepare same `FrameInput` with full vs. partial path, assert `PreparedFrame` equality
  - [ ] Atlas growth preserves glyph coordinates: fill atlas to 80%+ capacity, add new glyphs, verify existing glyph UV coordinates are unchanged
  - [ ] Idle terminal produces zero redraws: run terminal with no PTY output for 5 seconds, assert `render()` call count is 0 (excluding initial frame and cursor blink)

---

## 23.5 Benchmarks

Establish performance baselines and regression testing. Every optimization in this section must be validated by benchmarks.

**Files:** `oriterm_core/benches/grid.rs` (already exists — 15 benchmarks), `oriterm/benches/rendering.rs` (new — prepare-phase benchmarks only, using mock `AtlasLookup`)

**Reference:** Ghostty `src/main_bench.zig` (benchmark harness), `criterion` crate, Alacritty `alacritty_terminal/src/grid/storage.rs` (ring buffer patterns to benchmark against)

### Throughput Benchmark

**Partially covered.** `oriterm_core/benches/grid.rs` already benchmarks `put_char` throughput (ASCII + CJK, single line + full screen + output burst) and scroll/erase operations across three terminal sizes. Missing: end-to-end VTE parser throughput (bytes in → grid cells out).

- [x] Grid-level `put_char` throughput: `bench_put_char_ascii`, `bench_put_char_cjk`, `bench_put_char_full_screen` (already in `grid.rs`)
- [x] Realistic output burst: `bench_realistic_output_burst` — 100 lines of ASCII output with linefeed+scroll (already in `grid.rs`)
- [ ] Add `oriterm_core/benches/vte_throughput.rs` with criterion benchmarks:
  - [ ] `bench_vte_ascii_only`: create `Term<MockListener>` + `vte::Processor`, feed 1 MB of printable ASCII (0x20-0x7E), measure bytes/sec
  - [ ] `bench_vte_mixed`: feed 1 MB of terminal output with interleaved SGR color sequences (`\x1b[38;5;Nm`) and cursor movement (`\x1b[A/B/C/D`) — simulates realistic compiler output
  - [ ] `bench_vte_heavy_escape`: feed 1 MB of dense escape sequences (every 10 chars has a color change) — worst case for parser
- [ ] Target: >100 MB/s for ASCII-only, >50 MB/s for mixed (Alacritty achieves ~200 MB/s ASCII)
- [ ] Document baseline results as comments in the benchmark file for regression comparison

### Rendering Benchmark

**Note on testability:** The prepare phase (`fill_frame_shaped`, `prepare_frame_shaped`) is pure computation (no wgpu types) and can be benchmarked with criterion using the existing mock `AtlasLookup` from tests. GPU submit and present benchmarks require a live `wgpu::Device` — these must be `#[ignore]` tests or manual benchmarks, not CI-blocking criterion benchmarks. The `oriterm/benches/rendering.rs` file should focus on the prepare phase.

- [ ] Add `oriterm/benches/rendering.rs` with criterion benchmarks using mock `AtlasLookup` from `oriterm/src/gpu/prepare/tests.rs`:
  - [ ] `bench_prepare_plain`: 120x50 grid of plain ASCII text → `fill_frame_shaped()` → `PreparedFrame`
  - [ ] `bench_prepare_colored`: 120x50 grid where every cell has a unique fg/bg color (worst case for instance generation)
  - [ ] `bench_prepare_240x80`: 240x80 grid (large terminal) with mixed content
- [ ] Target: prepare phase completes in <2ms for 120x50, <8ms for 240x80 (must fit within 16ms frame budget with headroom for GPU upload and present)
- [ ] Manual test (not criterion): run `yes | head -100000` and verify 60fps is maintained. Measure frame drops by logging frame time >16ms in debug mode
- [ ] Manual test: 256-color gradient filling 120x50 grid — verify no visible jank during continuous scrolling

### Memory Benchmark

Measure RSS using `/proc/self/status` (Linux) or `mach_task_info` (macOS) or `GetProcessMemoryInfo` (Windows).

- [ ] 10K-line scrollback: fill scrollback with `seq 1 10000`, measure RSS. Expected: ~28.8 MB for grid data. Actual RSS will be higher (binary, GPU buffers, etc.) — document the overhead
- [ ] 100K-line scrollback: fill with `seq 1 100000`, measure RSS. Expected: ~288 MB for grid data. Document whether this is acceptable for the default config
- [ ] Baseline per-tab overhead: open a fresh tab with no output, measure RSS delta vs. no tabs. This isolates the per-tab overhead (Term struct, Grid allocation, PTY, etc.)
- [ ] Leak detection: run `while true; do echo test; sleep 0.01; done` for 10 minutes with 10K scrollback limit. Sample RSS every 30 seconds. After scrollback fills (~first 60 seconds), RSS must not grow by more than 1 MB over the remaining 9 minutes

### Latency Benchmark

- [ ] Add internal latency instrumentation (behind a `--latency-log` CLI flag or compile-time feature):
  - [ ] Record `Instant::now()` at `KeyboardInput` event receipt in `handle_keyboard_input()`
  - [ ] Record `Instant::now()` at `frame.present()` call in `WindowRenderer::render()`
  - [ ] Log the delta for each keypress to a CSV file (`timestamp, event_to_present_ms`)
- [ ] Target: p50 <3ms, p95 <5ms, p99 <8ms from `KeyboardInput` to `frame.present()`
- [ ] External validation: use `typometer` (https://github.com/blakesmith/typometer) or `Termpal` for end-to-end latency measurement including display pipeline lag

### Regression Testing

**Partially implemented.** `oriterm_core/benches/grid.rs` already contains criterion benchmarks for grid hot-path operations across three terminal sizes (80x24, 120x50, 240x80).

- [x] Criterion-based microbenchmarks (already in `oriterm_core/benches/grid.rs`):
  - [x] `grid.put_char()` — ASCII line, CJK line, full screen fill (3 benchmarks x 3 sizes)
  - [x] `grid.linefeed()` / `scroll_up()` — normal, BCE, reverse index, sub-region (4 benchmarks x 3 sizes)
  - [x] `grid.erase_display(All)` — full screen clear
  - [x] `grid.erase_line(Right)` — partial line erase
  - [x] `grid.insert_blank()` / `grid.delete_chars()` — editing operations
  - [x] `grid.insert_lines()` / `grid.delete_lines()` — line insert/delete
  - [x] `Row::reset()` — dirty/clean, default/BCE template
  - [x] Realistic: `output_burst` (100 lines of compiler output) and `tui_redraw` (10-line partial update)
- [ ] Missing benchmarks to add (see Throughput and Rendering Benchmark subsections above for details):
  - [ ] `vte_throughput.rs`: ASCII-only, mixed, and heavy-escape VTE parsing (oriterm_core/benches/)
  - [ ] `rendering.rs`: prepare-phase benchmarks with mock atlas (oriterm/benches/)
  - [ ] `bench_renderable_content_into`: snapshot extraction for 120x50 and 240x80 grids (oriterm_core/benches/)
  - [ ] `bench_dirty_drain`: `DirtyTracker::drain()` for 50 and 80 lines (oriterm_core/benches/grid.rs)
- [ ] Add `cargo bench` to CI pipeline. Store criterion baseline JSON in `benches/baseline/` directory. Fail CI if any benchmark regresses by >10% vs. stored baseline (use `criterion --load-baseline` and `--save-baseline`)
- [ ] Verify all benchmarks compile and complete within 60 seconds total (`cargo bench --no-run` for compile check)

---

## 23.6 Section Completion

**Infrastructure already in place:**
- [x] Scrollback uses ring buffer — no O(n) operations (already implemented)
- [x] Line-level dirty tracking in grid (DirtyTracker with mark/drain)
- [x] Per-pane PreparedFrame caching (PaneRenderCache)
- [x] Frame budget throttling (FRAME_BUDGET = 16ms, ctx.dirty flag)
- [x] PTY read-ahead with bounded parse (1 MB buffer, 64 KB max locked parse)
- [x] Synchronized output (Mode 2026) suppresses partial-frame renders
- [x] Grid benchmarks (15 criterion benchmarks across 3 terminal sizes)

**Remaining verification and optimization:**
- [ ] All 23.1-23.5 unchecked items complete
- [ ] `cat 100MB_file.txt` completes with no frame >32ms (2x budget) — verify via debug frame-time logging
- [ ] `fill_frame_shaped()` with 120x50 colored grid completes in <2ms (criterion benchmark)
- [ ] RSS bounded by scrollback limit: after filling 10K-line scrollback, RSS does not grow further over 10 minutes of continued output
- [ ] Column-level damage bounds populated in `DamageLine` (not hardcoded to full-line)
- [ ] Row-level dirty skip wired in `fill_frame_shaped()` via `prepare/dirty_skip.rs`
- [ ] Idle terminal CPU <0.5% measured over 30 seconds with no PTY output (only cursor blink wakes)
- [ ] Keypress-to-present latency: p95 <5ms (measured via internal instrumentation)
- [ ] `yes | head -100000` renders final line with no visible frame drops
- [ ] `oriterm_core/benches/vte_throughput.rs` added and baselined (>100 MB/s ASCII)
- [ ] `oriterm/benches/rendering.rs` added and baselined (<2ms for 120x50 prepare)
- [ ] `./build-all.sh` — all targets compile
- [ ] `./test-all.sh` — all tests pass
- [ ] `./clippy-all.sh` — no warnings
- [ ] `cargo bench` — all benchmarks compile and run without error

**Exit Criteria:** Terminal handles heavy workloads (large file output, rapid scrolling, complex TUIs) smoothly at 60fps with bounded memory usage. Performance is measured, baselined, and regression-tested.
