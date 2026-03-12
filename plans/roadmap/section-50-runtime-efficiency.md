---
section: 50
title: Runtime Efficiency — CPU & Memory Tuning
status: in-progress
reviewed: true
tier: 2
goal: Achieve near-zero idle CPU (<0.05%) and stable memory with no growth during steady-state terminal operation.
sections:
  - id: "50.1"
    title: Idle CPU Elimination
    status: in-progress
  - id: "50.2"
    title: Memory Stability
    status: not-started
  - id: "50.3"
    title: Event Loop Discipline
    status: complete
  - id: "50.4"
    title: Allocation Audit
    status: in-progress
  - id: "50.5"
    title: Profiling Infrastructure
    status: not-started
  - id: "50.6"
    title: Regression Prevention
    status: not-started
---

# Section 50: Runtime Efficiency — CPU & Memory Tuning

**Observed problems** (2026-03-12, Windows via WSL2):
- **Idle CPU: ~3%** — terminal sitting at a shell prompt with no programs running burns measurable CPU.
- **htop CPU: 0.1–0.3%** — acceptable for active output but should be lower.
- **Memory growth** — memory rises continuously during htop, never reclaims after exit. Suggests allocations that grow but never shrink, or caches without eviction pressure.

**Target**: Idle CPU < 0.05%, active CPU proportional to output volume, zero memory growth during steady-state operation (flat RSS after initial warmup).

**Implementation order**: 50.1 first (highest impact, mostly one-liners), then 50.3 (event loop audit — verifies 50.1 fixes and identifies any remaining wakeup sources), then 50.4 (allocation fixes), then 50.2 (memory), then 50.5 (profiling infra), then 50.6 (CI). The `ControlFlow::Wait` fix in 50.1 alone may resolve the 3% idle CPU.

**Crate ordering**: Within each subsection, changes must follow the dependency order: `oriterm_core` (library) first, then `oriterm_mux` (mux layer), then `oriterm` (binary). This matches the workspace dependency graph and ensures each layer compiles before its consumers.

**Sync points — types/traits modified**:
- `MuxBackend` trait (`oriterm_mux/src/backend/mod.rs`): new `has_pending_wakeup() -> bool` method. Must be implemented on `EmbeddedMux` (reads `wakeup_pending` AtomicBool) and `MuxClient` (reads transport's `wakeup_pending`). Provide a default impl returning `true` (conservative: always poll) so existing code compiles before both impls are done.
- `RenderableCell.zerowidth` field type change (`oriterm_core/src/term/renderable/mod.rs`): `Vec<char>` to `Option<Vec<char>>`. **Deprioritized.** `Option<Vec<char>>` is the same size as `Vec<char>` (24 bytes) due to niche optimization (null pointer niche), so no memory savings. The only benefit is semantic clarity (`None` = no combining marks). `.zerowidth` is referenced ~70 times across 17 source files in 3 crates (`oriterm_core`, `oriterm_mux`, `oriterm`). Key update sites: `oriterm_core/src/term/snapshot.rs`, `oriterm/src/font/shaper/mod.rs`, `oriterm/src/url_detect/mod.rs`, `oriterm/src/app/redraw/preedit.rs`, `oriterm/src/gpu/extract/from_snapshot/mod.rs`, `oriterm_mux/src/server/snapshot.rs`, `oriterm_core/src/selection/{text,html}`, plus ~40 test assertions. **Do not implement** — the risk vastly outweighs the benefit. Focus on `ControlFlow::Wait` and `extract_images()` HashSet reuse instead.
- `ImageCache::placements_in_viewport` signature change (`oriterm_core/src/image/cache/mod.rs`): if changed from `-> Vec<&ImagePlacement>` to `fn fill_viewport_placements(&self, ..., out: &mut Vec<&ImagePlacement>)`, update the call site in `Term::extract_images` (`oriterm_core/src/term/snapshot.rs`) and ~15 test call sites in `oriterm_core/src/image/tests.rs` and `oriterm_core/src/image/{kitty,iterm2}/tests.rs`.
- `PerfStats` struct (`oriterm/src/app/perf_stats.rs`): new fields for frame timing. No external consumers.

---

## 50.1 Idle CPU Elimination

Root causes identified in the event loop (`oriterm/src/app/event_loop.rs`, `fn about_to_wait`):

- [x] **Explicitly set `ControlFlow::Wait` in idle path** — the `else` branch at line 450 (no animations, no blinking, no dirty) falls through without calling `event_loop.set_control_flow(ControlFlow::Wait)`. Winit 0.30's `set_control_flow` is persistent across iterations: a prior `WaitUntil` from cursor blink or animation remains in effect, causing spurious wakeups even after the activity stops. Add `event_loop.set_control_flow(ControlFlow::Wait);` in the else branch. **This is likely the primary root cause of the 3% idle CPU.** Verify: the old prototype (`_old/src/app/event_loop.rs:174`) had this call and it was lost in the rebuild.
- [x] **Guard `pump_mux_events()` with wakeup flag** — `EmbeddedMux` already has a `wakeup_pending: Arc<AtomicBool>` (cleared in `poll_events()`), but `App::pump_mux_events()` unconditionally calls `mux.poll_events()` on every `about_to_wait` tick. Guard behind a wakeup flag check: skip `poll_events()` when no wakeup has arrived since the last poll. The `try_recv()` inside `InProcessMux::poll_events()` is cheap but acquires the channel lock. **Requires**: add `has_pending_wakeup() -> bool` to `MuxBackend` trait — the wakeup flag is a private field on `EmbeddedMux`. `MuxClient` transport has its own `wakeup_pending` via `clear_wakeup_pending()` — both backends must implement the new method. `drain_notifications()` does NOT need gating — it is O(1) when empty (just `Vec::drain` on an empty vec).
- [x] **Guard compositor animation tick** — the animation block (lines 378–403) unconditionally calls `focused_ctx_mut()` and runs `layer_animator.tick()`. The `is_any_animating()` check at line 399 only decides whether to mark dirty, but `tick()` itself always runs. Guard with `is_any_animating()` BEFORE calling `tick()`. Also: the block only ticks the focused window's animator — **unfocused windows with active animations will stall** (e.g., a fade started just before focus switch). Fix by iterating `self.windows.values_mut()` instead of `focused_ctx_mut()`. **Borrow note**: `values_mut()` borrows `self.windows` broadly, so verify no `&self` method calls exist inside the loop body.
- [x] **Guard dialog animation tick** — `tick_dialog_animations()` (`event_loop_helpers.rs:194`) iterates all dialog windows on every tick. Each dialog already guards `tick()` behind `is_any_animating()` (verified in source), so the per-dialog overhead is minimal. Add `if self.dialogs.is_empty() { return; }` at the top to skip the function call entirely when no dialogs exist (the common case).
- [x] **Guard torn-off drag update** — `update_torn_off_drag()` already early-returns when `self.torn_off_pending` is `None`. No additional work needed.
- [x] **Guard `flush_pending_focus_out()`** — already guarded via `let Some(pending) = self.pending_focus_out.take() else { return; }` in `event_loop_helpers.rs`. No work needed.
- [x] **Guard `process_fullscreen_events()` (macOS)** — already guarded. `take_fullscreen_events()` returns `None` when no events are pending.
- [x] **Guard `check_torn_off_merge()` (Windows)** — already guarded. Early-returns when `self.torn_off_pending` is `None`.
- [ ] **Cursor blink: verify 500ms sleep** — when cursor blink is the only activity, the event loop should wake exactly twice per second (blink on/off). Verify with `PerfStats` tick logging that no spurious wakeups occur between blink intervals.
- [ ] **Measure idle CPU** — after all guards are in place, measure idle CPU on Windows, Linux, and macOS. Target: < 0.05% (effectively zero wakeups between cursor blink intervals). Use `PerfStats` ticks/s metric: idle should show ~2 ticks/s (blink on + blink off) or 0 ticks/s (cursor not blinking).

## 50.2 Memory Stability

- [ ] **Profile RSS over time** — run htop (or `yes | head -10000`) for 5 minutes, then exit. Measure RSS at 0s, 60s, 120s, 300s, and after exit. RSS should plateau and not grow monotonically.
- [ ] **Audit `Vec` high-water-mark buffers** — instance buffers, shaping scratch buffers, notification buffers all grow via `.push()` and `.clear()` but never shrink. Shrink strategy: after each frame, if `capacity > 4 * len` and `capacity > 4096`, call `shrink_to(len * 2)`. This bounds waste to 2x while avoiding constant reallocation. **Note**: `maybe_shrink()` calls must happen AFTER rendering, not during `draw_frame()` — rendering discipline requires `draw_frame()` to be pure computation with no side effects on App state. Call `maybe_shrink()` in `about_to_wait` after `render_dirty_windows()`. Apply to:
  - `InstanceWriter.buf` (`oriterm/src/gpu/instance_writer/mod.rs`) — add `maybe_shrink()` called after each frame's upload.
  - `ShapingScratch.{runs, glyphs, col_starts, col_map}` (`oriterm/src/gpu/window_renderer/helpers.rs`) — add `maybe_shrink()` called after `shape_frame()`.
  - `notification_buf` (`oriterm/src/app/mod.rs`) — shrink in `with_drained_notifications()` after drain.
  - `RenderableContent.{cells, damage, images, image_data}` (`oriterm_core/src/term/renderable/mod.rs`) — add `maybe_shrink()` called after extraction.
  - `empty_keys: HashSet<RasterKey>` (`oriterm/src/gpu/window_renderer/mod.rs`) — grows monotonically as missing glyphs are discovered. Cap at 10,000 entries and clear when exceeded.
- [ ] **Pane cache eviction verification** — `PaneRenderCache` (`oriterm/src/gpu/pane_cache/mod.rs`) stores per-pane `PreparedFrame` instances. `remove(pane_id)` is called from `handle_pane_closed()` and `retain_only()` for batch cleanup. Verify closed panes are evicted promptly and their `InstanceWriter` buffer memory freed (no stale entries after tab close).
- [ ] **Scrollback memory cap verification** — `ScrollbackBuffer` (`oriterm_core/src/grid/ring/mod.rs`) has a max size. Verify that at capacity, old rows are truly freed (not just logically overwritten while retaining allocations in `Row` inner vecs). Test: push rows until full, verify total memory does not exceed `max_size * sizeof(Row)` plus a bounded overhead.
- [x] **Row allocation reuse** — already implemented. In `scroll/mod.rs`, `self.scrollback.push(evicted)` returns a recycled row when scrollback is full. The recycled row is `reset()` and reused as the new blank row. No additional work needed.
- [ ] **Image cache memory pressure** — two caches exist: the CPU-side `ImageCache` (`oriterm_core/src/image/cache/mod.rs`, 320 MiB default) stores decoded `Arc<Vec<u8>>` data, and the GPU-side `ImageTextureCache` (`oriterm/src/gpu/image_render/mod.rs`, 512 MiB default) stores `wgpu::Texture` objects. Verify that CPU cache eviction frees the `Arc<Vec<u8>>` data (no dangling `Arc` clones in `RenderableImageData` or `EmbeddedMux.renderable_cache` holding memory). Verify both caches evict in sync — a CPU eviction should trigger a GPU texture drop.
- [ ] **GPU texture leak audit** — verify GPU textures are released when images are evicted from `ImageTextureCache` (`oriterm/src/gpu/image_render/mod.rs`). Test: load an image, evict it, verify the `wgpu::Texture` is dropped (not just removed from the HashMap).
- [ ] **VTE parser state** — verify the VTE parser does not grow unbounded during malformed escape sequences or sustained binary output. Check `vte::ansi::Processor` internal buffers.
- [ ] **ImageCache animation map cleanup** — `ImageCache` has three per-image HashMaps (`animations`, `animation_frames`, `frame_starts`) cleaned up on `remove_image()` and `clear()`. Verify entries are removed when animations complete and when images are evicted by LRU. The `animation_frames` map stores `Vec<Arc<Vec<u8>>>` (one `Arc` per decoded frame), so leaked entries retain significant memory.
- [ ] **`EmbeddedMux` cache cleanup** — `renderable_cache` and `snapshot_cache` (`HashMap<PaneId, ...>`) are cleaned up in `cleanup_closed_pane()`. Verify no code path exists where a pane closes without calling `cleanup_closed_pane()`. Each entry holds a full `RenderableContent` (cells Vec + damage Vec) and `PaneSnapshot`, so leaked entries grow proportional to terminal size.
- [ ] **Measure and cap peak RSS** — establish acceptable RSS targets: < 30 MB for a single empty tab, < 50 MB for a tab running htop, < 100 MB for 10 tabs.

## 50.3 Event Loop Discipline

- [x] **Wakeup source inventory** — enumerate all paths that wake the event loop. Known sources: `TermEvent` variants (`MuxWakeup`, `ConfigReload`, `CreateWindow`, `MoveTabToNewWindow`, `OpenSettings`, `OpenConfirmation`), winit window events (mouse, keyboard, resize, focus, theme), and `ControlFlow::WaitUntil` timers (cursor blink, animation). Verify no other sources exist and that each source properly marks dirty or sets `WaitUntil`.
- [x] **Coalesce PTY wakeups** — already implemented. `EmbeddedMux::new()` wraps the wakeup callback with an `AtomicBool` guard (`wakeup_pending`): only the first PTY reader wakeup per poll cycle triggers the `EventLoopProxy`. `MuxClient` transport uses the same pattern via `clear_wakeup_pending()`. Verify correctness under sustained flood output (10,000+ lines/s).
- [x] **Frame budget enforcement** — verified. `FRAME_BUDGET` (16ms) is enforced: `render_dirty_windows()` sets `self.last_render` after rendering. The `about_to_wait` check gates rendering at max 60 FPS. No spin-render possible.
- [x] **`mark_all_windows_dirty()` audit** — `handle_mux_notification` in `mux_pump/mod.rs` calls `mark_all_windows_dirty()` for `PaneOutput`, `PaneBell`, and `PaneMetadataChanged`. This marks ALL windows dirty even if the pane lives in one window. For multi-window setups, this causes unnecessary redraws. Fix: mark only the window containing the affected pane. **Prerequisite**: add a pane-to-window reverse index (`HashMap<PaneId, SessionWindowId>` maintained on pane add/remove, or `fn window_for_pane(PaneId) -> Option<SessionWindowId>` on `SessionRegistry`). Low priority for single-window setups.
- [x] **No redundant `request_redraw()`** — verified. The codebase uses `ctx.dirty` flag + `FRAME_BUDGET` check, not `winit::Window::request_redraw()`. The `RedrawRequested` handler only fires from the Win32 modal loop timer path.
- [x] **Perf counter overhead** — verified cheap. `record_tick()` is a plain `u32` increment. `maybe_log()` calls `elapsed()` (~20ns syscall) and compares against `LOG_INTERVAL` (5s). Formatting only happens on the 5-second boundary.

## 50.4 Allocation Audit

Hot-path allocation analysis. These paths must be zero-alloc after warmup:

- [ ] **Render path** — `WindowRenderer::prepare_pane_into()` calls `fill_frame_shaped()` (`oriterm/src/gpu/prepare/mod.rs`) which fills the passive `PreparedFrame` struct via `InstanceWriter::push_*()`. Verify zero allocations after first frame (all `InstanceWriter` buffers and `Vec` fields at high-water mark via `.clear()` + capacity reuse). Check two specific allocation risks:
  - `shape_frame()` in `helpers.rs` calls `fonts.create_shaping_faces()` per frame — verify this returns borrowed references or is cached, not allocating `Vec` or `Box` per call. If it allocates, cache faces on `ShapingScratch` or `FontCollection`.
  - `ShapingScratch.unicode_buffer: Option<rustybuzz::UnicodeBuffer>` is taken/returned each call — verify rustybuzz does not reallocate the buffer's internal storage when reused.
- [ ] **VTE handler path** — `PtyEventLoop::parse_chunk()` calls `vte::ansi::Processor::advance()` which dispatches to `Handler::input()` (`oriterm_core/src/term/handler/mod.rs`) for each character. Only OSC string payloads and title changes may allocate. Verify no per-character allocations exist.
- [ ] **Event dispatch path** — `window_event()` dispatches to key/mouse handlers. Verify zero allocations per input event.
- [x] **Snapshot extraction** — `renderable_content_into()` reuses top-level `Vec` buffers via `.clear()`, but has specific allocation sites:
  - [x] `RenderableCell.zerowidth: Vec<char>` — `Vec::new()` (line 107 in `snapshot.rs`) does NOT heap-allocate (zero-capacity sentinel), costing only 24-byte zeroing. `e.zerowidth.clone()` (line 105) heap-allocates for cells with combining marks, but these are rare. The `cells` Vec itself is reused via `.clear()` so `cells.push()` is zero-alloc after warmup. **No action needed** on `zerowidth` — the cost is acceptable.
  - [x] `extract_images()` creates `HashSet::new()` per call (line 219 in `snapshot.rs`). Fix: add a `seen_image_ids: &mut HashSet<ImageId>` parameter. `extract_images()` is a private associated function (no `&self`), so thread the scratch buffer from `renderable_content_into()` through the caller chain: `EmbeddedMux::refresh_pane_snapshot()` -> `build_snapshot_into()` (`oriterm_mux/src/server/snapshot.rs`) -> `term.renderable_content_into()`. Store the HashSet on `EmbeddedMux` alongside `renderable_cache`.
  - [x] `placements_in_viewport()` (`oriterm_core/src/image/cache/mod.rs:244`) returns `Vec<&ImagePlacement>`, allocating on every call. Fix: change to `fn fill_viewport_placements(&self, ..., out: &mut Vec<&ImagePlacement>)` and pass a reusable buffer from caller. Update ~15 test call sites in `oriterm_core/src/image/tests.rs` and `oriterm_core/src/image/{kitty,iterm2}/tests.rs`. Implement both `oriterm_core` changes before updating `build_snapshot_into` in `oriterm_mux`.
  - [x] `RenderableImageData.data: Arc<Vec<u8>>` — each frame clones one `Arc` per visible image (atomic increment, cheap). If a snapshot is held in `EmbeddedMux.renderable_cache` while `ImageCache` evicts the image, the `Arc` keeps data alive for one extra frame. This is a bounded lag, not a leak. Verify the renderable cache is swapped promptly.
  - [x] Color resolution and damage collection are allocation-free (verified).
- [ ] **Add `#[global_allocator]` counting in debug builds** — instrument with a counting allocator to detect regressions. Log allocation count per frame. Target: 0 allocations per idle frame, < 10 per active frame (excluding VTE string payloads).

## 50.5 Profiling Infrastructure

**Scope boundary with Section 23**: Section 23 (Performance & Damage Tracking) focuses on throughput under load — parsing speed, damage-driven rendering skip, benchmarks. This section (50) focuses on efficiency at rest and memory discipline. The profiling infrastructure here serves both sections.

- [ ] **Add `--profile` CLI flag** — enable frame timing output and allocation counting. Use a Cargo feature (`profile`) so the `GlobalAlloc` wrapper and frame timers are zero-cost in normal builds. The `GlobalAlloc` wrapper must live in the binary crate (`oriterm/src/main.rs` or a dedicated `oriterm/src/alloc.rs`) since `#[global_allocator]` can only be set once per binary. The feature flag goes in `oriterm/Cargo.toml`. Library crates (`oriterm_core`, `oriterm_mux`) must NOT depend on this feature.
- [ ] **Frame timing stats** — extend `PerfStats` (`oriterm/src/app/perf_stats.rs`, currently 83 lines) with frame time min/max/avg fields. It already tracks renders/s, wakeups/s, cursor/s, ticks/s. Add `frame_time_min`, `frame_time_max`, `frame_time_sum` fields, measured with `Instant::elapsed()` around `render_dirty_windows()`. Log in `maybe_log()`. File stays well under the 500-line limit.
- [ ] **Allocation counter** — track allocations per frame using a custom `GlobalAlloc` wrapper behind `#[cfg(feature = "profile")]`. Log allocation spikes. `GlobalAlloc` wrapping adds ~5ns per alloc/dealloc (atomic increment).
- [ ] **Idle detection logging** — log when the event loop enters true idle (no wakeups for > 1s) and when it exits idle. Add `last_activity: Instant` field to `App`, log state transitions in `about_to_wait`. Useful for verifying the `ControlFlow::Wait` fix.
- [ ] **Memory watermark logging** — periodically log RSS and key buffer capacities (scrollback, instance buffers, glyph atlas) alongside `PerfStats::maybe_log()`. Platform-specific RSS reading: `/proc/self/statm` (Linux), `mach_task_basic_info` (macOS), `GetProcessMemoryInfo` (Windows). Put the RSS query function in `oriterm/src/platform/` with per-platform implementations per impl-hygiene rules — no inline `#[cfg]` blocks in `perf_stats.rs`.

## 50.6 Regression Prevention

- [ ] **Test: `ControlFlow::Wait` in idle** — extract the control-flow decision from `about_to_wait` into a pure function: `fn compute_control_flow(any_dirty: bool, budget_elapsed: bool, has_animations: bool, blinking_active: bool, ...) -> ControlFlow`. Unit-test this function in a sibling `tests.rs` without needing a winit `EventLoop` (follows impl-hygiene rule: no concrete external-resource types in logic layers). Test cases: (1) all false returns `Wait`, (2) dirty+budget returns `WaitUntil(remaining)`, (3) animations returns `WaitUntil(16ms)`, (4) blinking returns `WaitUntil(next_toggle)`.
- [ ] **Test: snapshot extraction zero-alloc steady state** — verify `renderable_content_into()` performs zero heap allocations after the first call. Use a counting allocator in a `#[test]` with `Term<VoidListener>`: call `renderable_content_into()` twice on the same `RenderableContent`, assert zero allocations on the second call. Place in `oriterm_core/src/term/tests.rs`. The `extract_images()` `HashSet` (if not yet moved to a reusable buffer) will be the primary remaining allocation source.
- [ ] **CI benchmark: allocation per frame** — unit test in `oriterm_core` (no GPU needed): use `Term<VoidListener>` + counting allocator, render 100 frames via `renderable_content_into()`, assert zero allocations after warmup. Place in `oriterm_core/src/term/tests.rs`.
- [ ] **CI benchmark: RSS stability** — unit test in `oriterm_core` (no GPU needed): feed 100,000 lines of output to `Term<VoidListener>`, measure RSS before and after. Fail if delta > 5 MB. Measure via `std::alloc::System` wrapper or platform RSS APIs. Place in `oriterm_core/src/term/tests.rs`.
- [ ] **CI benchmark: idle CPU** — not directly feasible on headless CI (requires GPU context + display server). Instead, rely on the `compute_control_flow` pure function test (above) and the `PerfStats` ticks/s metric for manual verification. Alternatively, use `xvfb-run` + `wgpu::Backends::GL` on Linux CI if CI complexity is acceptable.
- [ ] **Document performance invariants** — add to CLAUDE.md: "Zero idle CPU beyond cursor blink. Zero allocations in hot paths. Stable RSS under sustained output."
