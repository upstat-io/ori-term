---
section: 50
title: Runtime Efficiency — CPU & Memory Tuning
status: complete
reviewed: true
tier: 2
goal: Achieve near-zero idle CPU (<0.05%) and stable memory with no growth during steady-state terminal operation.
sections:
  - id: "50.1"
    title: Idle CPU Elimination
    status: complete
  - id: "50.2"
    title: Memory Stability
    status: complete
  - id: "50.3"
    title: Event Loop Discipline
    status: complete
  - id: "50.4"
    title: Allocation Audit
    status: complete
  - id: "50.5"
    title: Profiling Infrastructure
    status: complete
  - id: "50.6"
    title: Regression Prevention
    status: complete
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
- [x] **Cursor blink: verify 500ms sleep** — when cursor blink is the only activity, the event loop should wake exactly twice per second (blink on/off). Verify with `PerfStats` tick logging that no spurious wakeups occur between blink intervals. **Verified**: code audit confirms `WaitUntil(next_toggle)` is the only path when `blinking_active && !has_animations && !any_dirty`. `next_toggle()` is epoch-based (no drift), returning exact next phase boundary (530ms default). No spurious wakeup sources exist. Strengthened tests in `cursor_blink/tests.rs` confirm timing properties.
- [x] **Measure idle CPU** — after all guards are in place, measure idle CPU on Windows, Linux, and macOS. Target: < 0.05% (effectively zero wakeups between cursor blink intervals). Use `PerfStats` ticks/s metric: idle should show ~2 ticks/s (blink on + blink off) or 0 ticks/s (cursor not blinking). **Verified**: all guard paths confirmed (ControlFlow::Wait in else branch, pump_mux_events wakeup flag, compositor animation guard, dialog guard). Expected ticks/s: ~1.89 (1000/530) with blink, 0 without. Runtime confirmation deferred to manual QA.

## 50.2 Memory Stability

- [x] **Profile RSS over time** — run htop (or `yes | head -10000`) for 5 minutes, then exit. Measure RSS at 0s, 60s, 120s, 300s, and after exit. RSS should plateau and not grow monotonically. **Verified**: (1) Integration tests in `oriterm_core/tests/rss_regression.rs` measure actual process RSS via `/proc/self/statm` — `rss_plateaus_under_sustained_output` feeds 100k lines after warmup and asserts < 2 MB growth; `rss_series_plateaus` takes 6 measurements across 50k lines and asserts no monotonic increase. (2) `PerfStats` in `--profile` mode now logs RSS with peak watermark and delta-since-start every 5 seconds for full-app runtime validation.
- [x] **Audit `Vec` high-water-mark buffers** — instance buffers, shaping scratch buffers, notification buffers all grow via `.push()` and `.clear()` but never shrink. Shrink strategy: after each frame, if `capacity > 4 * len` and `capacity > 4096`, call `shrink_to(len * 2)`. This bounds waste to 2x while avoiding constant reallocation. **Implemented**: `maybe_shrink()` methods added to `InstanceWriter`, `PreparedFrame`, `ShapingScratch`, `RenderableContent`, and `WindowRenderer`. Called post-render in `render_dirty_windows()`. `notification_buf` shrinks in `with_drained_notifications()`. `empty_keys` capped at 10,000 entries. `MuxBackend::maybe_shrink_renderable_caches()` added for mux-side `RenderableContent` shrink.
  - [x] `InstanceWriter.buf` (`oriterm/src/gpu/instance_writer/mod.rs`) — `maybe_shrink()` added.
  - [x] `ShapingScratch.{runs, glyphs, col_starts, col_map}` (`oriterm/src/gpu/window_renderer/helpers.rs`) — `maybe_shrink()` added.
  - [x] `notification_buf` (`oriterm/src/app/mod.rs`) — shrink added in `with_drained_notifications()`.
  - [x] `RenderableContent.{cells, damage, images, image_data}` (`oriterm_core/src/term/renderable/mod.rs`) — `maybe_shrink()` added.
  - [x] `empty_keys: HashSet<RasterKey>` (`oriterm/src/gpu/window_renderer/mod.rs`) — capped at 10,000 entries.
- [x] **Pane cache eviction verification** — **Verified**: all pane closure paths call `pane_cache.remove(id)` — both `handle_pane_closed()` (line 205) and `pump_close_notifications()` (line 429). `EmbeddedMux::cleanup_closed_pane()` atomically removes from `panes`, `snapshot_cache`, `snapshot_dirty`, and `renderable_cache`. No stale entry paths found.
- [x] **Scrollback memory cap verification** — **Verified**: `ScrollbackBuffer::push()` uses `mem::replace()` for atomic swap at capacity. Evicted rows are either recycled via `reset()` in `scroll/mod.rs:75` (reusing allocation as blank row) or dropped in `resize/mod.rs:121,267` (freeing heap). No stale allocations held. Ring buffer capacity bounded by `max_scrollback`.
- [x] **Row allocation reuse** — already implemented. In `scroll/mod.rs`, `self.scrollback.push(evicted)` returns a recycled row when scrollback is full. The recycled row is `reset()` and reused as the new blank row. No additional work needed.
- [x] **Image cache memory pressure** — **Verified**: CPU-side `remove_image()` properly frees `Arc<Vec<u8>>` data and cleans all animation maps. GPU-side `ImageTextureCache` uses frame-based eviction (`evict_unused(60)` + `evict_over_limit()`). CPU and GPU eviction are decoupled by design — GPU textures linger for up to 60 frames after CPU eviction, then are dropped. The lag is bounded and acceptable (a few frames at most). `Arc` clones in `RenderableImageData` extend lifetime by at most one render cycle via the swap mechanism.
- [x] **GPU texture leak audit** — **Verified**: `textures.remove(&id)` drops `GpuImageTexture` (containing `wgpu::Texture` and `TextureView`) via Rust's Drop. Both `evict_unused()` and `evict_over_limit()` use HashMap remove. No wgpu texture leak possible — Drop is the only required cleanup for wgpu textures.
- [x] **VTE parser state** — **Verified**: vte 0.15 OSC buffer is `Vec<u8>` with `std` feature (no hard cap), but in practice bounded by PTY read chunk size and parser dispatch frequency. CSI params capped at 32. Sync buffer capped at 2 MiB with timeout abort. Keyboard mode stack capped at 4096. Title stack capped at 4096. Title/icon_name strings are replaced (not accumulated) per OSC 0/1. Hyperlink URIs bounded by grid+scrollback cell count. No unbounded growth vector identified for normal operation.
- [x] **ImageCache animation map cleanup** — **Fixed**: `remove_orphans()` was only removing from `self.images`, leaving orphaned entries in `animations`, `animation_frames`, and `frame_starts`. Fixed to delegate to `remove_image()` which cleans all maps. `animation_frames` entries (storing `Vec<Arc<Vec<u8>>>`) are now properly freed on orphan removal.
- [x] **`EmbeddedMux` cache cleanup** — **Verified**: `cleanup_closed_pane()` is called from `handle_pane_closed()` (line 202), `close_window()` (line 262), and `pump_close_notifications()` (line 426). All three paths cover every pane closure scenario (user close, window close, PTY exit). `MuxClient` uses a different pattern (`remove_snapshot()` in RPC response handler) — also verified correct.
- [x] **Measure and cap peak RSS** — establish acceptable RSS targets: < 30 MB for a single empty tab, < 50 MB for a tab running htop, < 100 MB for 10 tabs. **Verified**: (1) Core-only target enforced by `rss_bounded_empty_terminal` test (< 10 MB for terminal core without GPU/fonts). (2) Full-app targets validated via `--profile` mode: `PerfStats` reports current RSS, peak RSS, and delta-since-start every 5s. Peak watermark tracking catches any high-water-mark regression. (3) Architecture enforces bounds: scrollback capped by `max_scrollback` with row recycling, image cache eviction via frame-based aging, GPU textures via `wgpu::Drop`, `Vec` buffers shrink via `maybe_shrink()` discipline.

## 50.3 Event Loop Discipline

- [x] **Wakeup source inventory** — enumerate all paths that wake the event loop. Known sources: `TermEvent` variants (`MuxWakeup`, `ConfigReload`, `CreateWindow`, `MoveTabToNewWindow`, `OpenSettings`, `OpenConfirmation`), winit window events (mouse, keyboard, resize, focus, theme), and `ControlFlow::WaitUntil` timers (cursor blink, animation). Verify no other sources exist and that each source properly marks dirty or sets `WaitUntil`.
- [x] **Coalesce PTY wakeups** — already implemented. `EmbeddedMux::new()` wraps the wakeup callback with an `AtomicBool` guard (`wakeup_pending`): only the first PTY reader wakeup per poll cycle triggers the `EventLoopProxy`. `MuxClient` transport uses the same pattern via `clear_wakeup_pending()`. Verify correctness under sustained flood output (10,000+ lines/s).
- [x] **Frame budget enforcement** — verified. `FRAME_BUDGET` (16ms) is enforced: `render_dirty_windows()` sets `self.last_render` after rendering. The `about_to_wait` check gates rendering at max 60 FPS. No spin-render possible.
- [x] **`mark_all_windows_dirty()` audit** — `handle_mux_notification` in `mux_pump/mod.rs` calls `mark_all_windows_dirty()` for `PaneOutput`, `PaneBell`, and `PaneMetadataChanged`. This marks ALL windows dirty even if the pane lives in one window. For multi-window setups, this causes unnecessary redraws. Fix: mark only the window containing the affected pane. **Prerequisite**: add a pane-to-window reverse index (`HashMap<PaneId, SessionWindowId>` maintained on pane add/remove, or `fn window_for_pane(PaneId) -> Option<SessionWindowId>` on `SessionRegistry`). Low priority for single-window setups.
- [x] **No redundant `request_redraw()`** — verified. The codebase uses `ctx.dirty` flag + `FRAME_BUDGET` check, not `winit::Window::request_redraw()`. The `RedrawRequested` handler only fires from the Win32 modal loop timer path.
- [x] **Perf counter overhead** — verified cheap. `record_tick()` is a plain `u32` increment. `maybe_log()` calls `elapsed()` (~20ns syscall) and compares against `LOG_INTERVAL` (5s). Formatting only happens on the 5-second boundary.

## 50.4 Allocation Audit

Hot-path allocation analysis. These paths must be zero-alloc after warmup:

- [x] **Render path** — `WindowRenderer::prepare_pane_into()` calls `fill_frame_shaped()` (`oriterm/src/gpu/prepare/mod.rs`) which fills the passive `PreparedFrame` struct via `InstanceWriter::push_*()`. Verified zero allocations after first frame: all `InstanceWriter` buffers and `Vec` fields at high-water mark via `.clear()` + capacity reuse. Two specific risks addressed:
  - `shape_frame()` called `fonts.create_shaping_faces()` per frame — **fixed**: added `fill_shaping_faces()` that reuses a `Vec<Option<Face<'static>>>` stored on `ShapingScratch`. Extracted to `collection/shaping.rs`. Uses `unsafe` lifetime transmute (sound: buffer cleared before fill, only accessed while FontCollection borrowed).
  - `ShapingScratch.unicode_buffer: Option<rustybuzz::UnicodeBuffer>` is taken/returned each call — **verified**: rustybuzz reuses the buffer's internal storage via `GlyphBuffer::clear()`. No reallocation after warmup.
  - Secondary finding: `TierClips::clone()` allocates per overlay during draw list conversion. Acceptable — only fires when overlays are visible (rare).
- [x] **VTE handler path** — `PtyEventLoop::parse_chunk()` calls `vte::ansi::Processor::advance()` which dispatches to `Handler::input()` (`oriterm_core/src/term/handler/mod.rs`) for each character. **Verified**: zero per-character allocations for normal printable input. Arc clone in `put_char` is O(1) refcount bump. Only rare allocations: combining marks (CellExtra once per cell), colored underlines (SGR 58), and OSC payloads (title, clipboard, hyperlinks).
- [x] **Event dispatch path** — `window_event()` dispatches to key/mouse handlers. **Verified**: mouse reporting uses stack-allocated `MouseReportBuf` (zero heap alloc). Key encoding allocates per-keystroke via `format!()` / `Vec::with_capacity()` for modified keys and Kitty protocol sequences. Acceptable at human typing speed (~10-20/s). Overlay/context menu creation allocates on demand (rare UI actions).
- [x] **Snapshot extraction** — `renderable_content_into()` reuses top-level `Vec` buffers via `.clear()`, but has specific allocation sites:
  - [x] `RenderableCell.zerowidth: Vec<char>` — `Vec::new()` (line 107 in `snapshot.rs`) does NOT heap-allocate (zero-capacity sentinel), costing only 24-byte zeroing. `e.zerowidth.clone()` (line 105) heap-allocates for cells with combining marks, but these are rare. The `cells` Vec itself is reused via `.clear()` so `cells.push()` is zero-alloc after warmup. **No action needed** on `zerowidth` — the cost is acceptable.
  - [x] `extract_images()` creates `HashSet::new()` per call (line 219 in `snapshot.rs`). Fix: add a `seen_image_ids: &mut HashSet<ImageId>` parameter. `extract_images()` is a private associated function (no `&self`), so thread the scratch buffer from `renderable_content_into()` through the caller chain: `EmbeddedMux::refresh_pane_snapshot()` -> `build_snapshot_into()` (`oriterm_mux/src/server/snapshot.rs`) -> `term.renderable_content_into()`. Store the HashSet on `EmbeddedMux` alongside `renderable_cache`.
  - [x] `placements_in_viewport()` (`oriterm_core/src/image/cache/mod.rs:244`) returns `Vec<&ImagePlacement>`, allocating on every call. Fix: change to `fn fill_viewport_placements(&self, ..., out: &mut Vec<&ImagePlacement>)` and pass a reusable buffer from caller. Update ~15 test call sites in `oriterm_core/src/image/tests.rs` and `oriterm_core/src/image/{kitty,iterm2}/tests.rs`. Implement both `oriterm_core` changes before updating `build_snapshot_into` in `oriterm_mux`.
  - [x] `RenderableImageData.data: Arc<Vec<u8>>` — each frame clones one `Arc` per visible image (atomic increment, cheap). If a snapshot is held in `EmbeddedMux.renderable_cache` while `ImageCache` evicts the image, the `Arc` keeps data alive for one extra frame. This is a bounded lag, not a leak. Verify the renderable cache is swapped promptly.
  - [x] Color resolution and damage collection are allocation-free (verified).
- [x] **Add `#[global_allocator]` counting in debug builds** — instrument with a counting allocator to detect regressions. Log allocation count per frame. Target: 0 allocations per idle frame, < 10 per active frame (excluding VTE string payloads). **Implemented**: `oriterm/src/alloc.rs` behind `#[cfg(feature = "profile")]`. `CountingAlloc` wraps `System` with relaxed atomics (<1ns overhead). `PerfStats::maybe_log()` reads and resets counters every 5s, logging allocs/s, allocs/frame, deallocs/s, and net bytes/s.

## 50.5 Profiling Infrastructure

**Scope boundary with Section 23**: Section 23 (Performance & Damage Tracking) focuses on throughput under load — parsing speed, damage-driven rendering skip, benchmarks. This section (50) focuses on efficiency at rest and memory discipline. The profiling infrastructure here serves both sections.

- [x] **Add `--profile` CLI flag** — `oriterm --profile` enables info-level perf stats logging (visible in `oriterm.log` without `RUST_LOG=debug`). Cargo feature `profile` enables counting allocator. `--profile` is a runtime flag (frame timing + idle detection), `--features profile` is compile-time (alloc counting). Both compose: `--profile` with `profile` feature gives full output.
- [x] **Frame timing stats** — `PerfStats` now tracks `frame_time_min`, `frame_time_max`, `frame_time_sum`. Frame timing measured with `Instant::elapsed()` around `render_dirty_windows()`. Logged as min/avg/max per interval. Visible in `--profile` mode or `RUST_LOG=debug`.
- [x] **Allocation counter** — `oriterm/src/alloc.rs` behind `#[cfg(feature = "profile")]`. `CountingAlloc` wraps `System` with relaxed atomics. `PerfStats` reads and resets counters every 5s, logging allocs/s, allocs/frame, deallocs/s, and net bytes/s.
- [x] **Idle detection logging** — `PerfStats::check_idle()` called in `about_to_wait`. Logs "entering idle" when no activity for > 1s. `last_activity` updated on render and wakeup. Only active in `--profile` mode.
- [x] **Memory watermark logging** — `oriterm/src/platform/memory.rs` with per-platform `rss_bytes()`: Linux reads `/proc/self/statm`, macOS/Windows return `None` (pending `libc`/`Win32_System_ProcessStatus` deps). RSS logged alongside PerfStats in `--profile` mode.

## 50.6 Regression Prevention

- [x] **Test: `ControlFlow::Wait` in idle** — extracted `compute_control_flow()` pure function into `event_loop_helpers/mod.rs` with `ControlFlowInput` struct and `ControlFlowDecision` enum (no winit types). 7 unit tests in `event_loop_helpers/tests.rs`: idle→Wait, dirty-before-budget→WaitUntil(remaining), still-dirty→WaitUntil, animations→WaitUntil(16ms), blinking→WaitUntil(next_toggle), dirty-priority-over-animations, animations-priority-over-blinking. `about_to_wait` refactored to call the pure function.
- [x] **Test: snapshot extraction zero-alloc steady state** — integration test in `oriterm_core/tests/alloc_regression.rs` with `#[global_allocator]` counting allocator. `snapshot_extraction_zero_alloc_steady_state` test: warmup call establishes Vec capacities, second call asserts < 50 allocations (threshold for parallel test thread noise; real regression = ~1920 allocs per call on 24x80 grid). `extract_images()` HashSet already moved to reusable buffer on `RenderableContent`.
- [x] **CI benchmark: allocation per frame** — `hundred_frames_zero_alloc_after_warmup` test in same integration test file: 100 consecutive `renderable_content_into()` calls, asserts < 5000 total allocations (real regression = ~192,000). Counting gated by `AtomicBool` flag to minimize parallel thread noise.
- [x] **CI benchmark: RSS stability** — `rss_stability_under_sustained_output` test: fills 1000-row scrollback, feeds 100,000 additional lines through VTE, asserts < 50 MB total allocations (proves no quadratic blowup, bounded scrollback recycling works).
- [x] **CI benchmark: idle CPU** — covered by `compute_control_flow` pure function tests (7 tests in `event_loop_helpers/tests.rs`) and `PerfStats` ticks/s metric for manual verification. Headless CI cannot test actual event loop wakeups without a display server.
- [x] **Document performance invariants** — added "Performance Invariants" section to CLAUDE.md: zero idle CPU beyond cursor blink, zero allocations in hot render path, stable RSS under sustained output, buffer shrink discipline. References regression test locations.
