---
section: "06"
title: "Rendering & Performance Bugs"
status: in-progress
reviewed: true
goal: "Track and fix rendering performance bugs — frame time, input latency, GPU bottlenecks"
depends_on: []
third_party_review:
  status: resolved
  updated: 2026-03-31
sections:
  - id: "06.1"
    title: "Active Bugs"
    status: in-progress
  - id: "06.R"
    title: "Third Party Review Findings"
    status: complete
---

# Section 06: Rendering & Performance Bugs

**Status:** In Progress
**Goal:** Track and fix all rendering performance issues — frame time, input latency, GPU pipeline bottlenecks.

**Note:** This section is never marked complete. New bugs are appended as discovered.

---

## 06.1 Active Bugs

- [x] **BUG-06.1**: Noticeable input lag during key repeat — worse at smaller window widths
  - **Severity**: critical
  - **Found**: 2026-03-29 — manual, user report
  - **Resolved**: 2026-03-30 — User confirmed fixed. Likely resolved by frame budget and render pipeline improvements in recent commits.

- [x] **BUG-06.2**: Random extra text appears after resize following sustained key repeat
  - **Severity**: medium
  - **File(s)**: `oriterm_mux/src/pane/io_thread/handler.rs` (resize via IO thread)
  - **Root cause**: Race between queued key repeat events and synchronous main-thread reflow + PTY resize (SIGWINCH). Shell processes both simultaneously, producing interleaved output.
  - **Fix**: Threaded IO plan (plans/threaded-io). Resize now flows through `PaneIoCommand::Resize` to the IO thread, which serializes bytes and resize in its priority loop. Grid reflow and PTY resize happen atomically on the IO thread — no concurrent main-thread access. Coalescing ensures only the final size is applied during rapid resize.
  - **Resolved**: 2026-04-01 — threaded IO architecture eliminates the race condition.

- [x] **BUG-06.3**: Window surface not redrawn after dragging partially off-screen and back
  - **Severity**: high
  - **Found**: 2026-03-30 — manual, user report
  - **Root cause**: During a Win32 modal move loop, windows are never marked dirty (no terminal content changes). The 60 FPS timer generates `RedrawRequested` via `InvalidateRect`, but `modal_loop_render()` skips because no window is dirty. After the loop ends (`WM_EXITSIZEMOVE`), the timer is killed and no subsequent event marks the window dirty. The stale surface persists until cursor blink or mouse interaction.
  - **Fixed**: 2026-03-30 — Added `MODAL_LOOP_ENDED` atomic flag set in `WM_EXITSIZEMOVE`. `about_to_wait()` checks and clears it, marking all terminal windows dirty. Also hide terminal windows before close to prevent stale surface flash during teardown.

- [x] **BUG-06.4**: Settings dialog shows baby blue flash on open/close
  - **Severity**: medium
  - **File(s)**: `oriterm/src/app/dialog_management.rs` (dialog lifecycle), `oriterm/src/gpu/state/mod.rs` (poll_device)
  - **Root cause**: `render_dialog()` called `render_to_surface()` which submits GPU commands and presents, but did not call `device.poll()` to flush the work. The window became visible on the next tick via `show_primed_dialogs()` before the GPU had finished rendering the first frame, briefly showing uninitialized VRAM (baby blue). Terminal windows avoided this because `clear_surface()` already called `device.poll(wait_indefinitely())`.
  - **Found**: 2026-03-30 — user report. Only affects settings dialog, not terminal windows.
  - **Fixed**: 2026-03-30 — Added `GpuState::poll_device()` method and called it in `finalize_dialog()` after `render_dialog()`, matching the terminal window pattern. GPU work is now flushed synchronously before the Primed → Visible transition.

- [x] **BUG-06.5**: DX12 backend: terminal grid blank, only tab bar chrome renders
  - **Severity**: medium
  - **File(s)**: `oriterm/src/gpu/instance_writer/mod.rs` (`CLIP_UNCLIPPED`), `oriterm_ui/src/draw/scene/content_mask.rs` (`ContentMask::unclipped()`)
  - **Root cause**: `CLIP_UNCLIPPED` used `f32::NEG_INFINITY` / `f32::INFINITY` as clip rect values. In the shader, `clip_max = clip.xy + clip.zw` computed `-INF + INF = NaN`. DX12/HLSL treats NaN comparisons (`frag_pos > NaN`) as `true`, causing the clip test to discard EVERY fragment. Tab bar chrome was unaffected because UI framework widgets use finite clip rects from the layout system, not `CLIP_UNCLIPPED`. Same issue in `ContentMask::unclipped()` which used infinity for the default scene clip mask.
  - **Repro**: Set `gpu_backend = "dx12"` in `[rendering]`. NVIDIA RTX 3080, Windows, `Bgra8UnormSrgb` format.
  - **Found**: 2026-03-31 — manual, user testing.
  - **Fixed**: 2026-03-31 — Replaced infinity with large finite values (`-100_000.0, -100_000.0, 200_000.0, 200_000.0`) in both `CLIP_UNCLIPPED` and `ContentMask::unclipped()`. No NaN, all comparisons well-defined.

- [x] **BUG-06.6**: Font config changes not applied in realtime (weight, hinting, subpixel AA)
  - **Severity**: high
  - **File(s)**: `oriterm/src/gpu/window_renderer/font_config.rs` (`clear_and_recache`), `oriterm/src/app/config_reload/mod.rs` (`apply_font_changes`)
  - **Root cause**: Two stale caches: (1) `clear_and_recache()` cleared GPU atlases but not the `ShapedFrame` cache — the prepare fast path served old glyph IDs from the previous font. (2) `apply_font_changes()` rebuilt the `FontCollection` but didn't reset `last_rendered_pane` or `frame`, so the redraw path saw `content_changed=false` and skipped re-extraction.
  - **Found**: 2026-04-01 — manual, user report during settings UI testing.
  - **Fixed**: 2026-04-01 — (1) `clear_and_recache()` now resets `ShapingScratch.frame` to empty. (2) `apply_font_changes()` calls `ctx.invalidate_font_caches()` which clears `last_rendered_pane` and `frame`. Commits bfe736f and 793f52d.
  - **Needs**: Regression test that font config change invalidates shaping + frame caches.

- [ ] **BUG-06.7**: Vulkan backend: baby blue flash when opening settings dialog
  - **Severity**: low
  - **File(s)**: `oriterm/src/app/dialog_management.rs` (dialog lifecycle)
  - **Root cause**: Same symptom as BUG-06.4 (uninitialized VRAM visible before first frame). BUG-06.4 was fixed with `poll_device()` after `render_dialog()`, which works on DX12 but apparently Vulkan's poll timing differs — the GPU work may not be fully flushed before the window becomes visible.
  - **Repro**: Set `gpu_backend = "vulkan"`, open settings dialog. Brief baby blue flash before content renders.
  - **Found**: 2026-03-31 — manual, user report. DX12 (default) is not affected.

- [ ] **BUG-06.9**: Focus border around panes has asymmetric padding — left padded, right clips out of bounds
  - **Severity**: medium
  - **File(s)**: `oriterm/src/session/compute/mod.rs` (`snap_to_grid`, pane `pixel_rect` computation), `oriterm/src/app/redraw/multi_pane/mod.rs` (focus border call site)
  - **Root cause**: The pane `pixel_rect` (used by `append_focus_border`) has inconsistent insets — the left edge has proper padding (from window border or divider offset) but the right edge either extends to the window edge without border inset or the `snap_to_grid` trimming creates an asymmetric result. The focus border renders exactly at the `pixel_rect` bounds, so if the rect itself is wrong, the border clips on the right side.
  - **Repro**: Split a pane (Ctrl+Shift+D or equivalent). Observe the accent focus border on the active pane — left side has visible padding from the window edge, right side clips or has no padding.
  - **Found**: 2026-03-31 — manual, user report.
  - **Note**: Roadmap section 33 (split-nav-floating) touches this area.

- [ ] **BUG-06.10**: `rss_plateaus_under_sustained_output` is flaky on Windows — reports 3.0 MB growth instead of <2 MB ceiling
  - **Severity**: medium
  - **File(s)**: `oriterm_core/tests/rss_regression.rs:134`
  - **Repro**: `cargo test -p oriterm_core --test rss_regression rss_plateaus_under_sustained_output` — fails when run as part of the full workspace test serial pass (`cargo test --workspace -- --test-threads=1`); passes in isolation. Observed value `RSS grew 3.0 MB after 100k lines (warmup: 7.6 MB, after: 10.6 MB)` vs the 2 MB ceiling. The 1 MB excess is small enough to be Windows heap/scheduler noise from prior tests in the same process group inflating the warmup baseline lower than steady state.
  - **Found**: 2026-04-08 — surfaced during `/fix-bug` BUG-07-009 verification
  - **Source**: `/fix-bug` test-all sweep
  - **Fix**: Either (1) re-baseline the warmup measurement to be more representative (run a longer warmup loop, or take min-of-N samples), or (2) raise the ceiling to ~5 MB on Windows where heap fragmentation produces a larger noise floor than on Linux. The semantic invariant — RSS plateaus under sustained output — is correct; only the threshold is too tight.

- [ ] **BUG-06.8**: Floating pane (Ctrl+Shift+P) is completely transparent — no background fill
  - **Severity**: high
  - **File(s)**: `oriterm/src/gpu/window_renderer/multi_pane.rs` (`append_floating_decoration`), `oriterm/src/app/redraw/multi_pane/mod.rs`
  - **Root cause**: `append_floating_decoration()` only renders a drop shadow and accent border around floating panes. There is no opaque background fill — the floating pane's cell backgrounds are composited directly over the main window content beneath, making it appear fully transparent. The floating pane should use the same background settings as the main window (opacity, blur, bg color).
  - **Repro**: Press Ctrl+Shift+P to toggle a floating pane. The pane content is transparent — you can see the main terminal grid behind it.
  - **Found**: 2026-03-31 — manual, user report.

- [ ] `[BUG-06-013][critical]` **GPU atlas unit tests hang indefinitely on `GpuState::new_headless()` — `cargo test -p oriterm` never completes**
  Repro: `timeout 120 cargo test -p oriterm --lib gpu::atlas::tests::insert_and_lookup_round_trip`. The test prints `test ... has been running for over 60 seconds` and never completes. All 49 tests under `oriterm::gpu::atlas::tests::*` (i.e. every `#[test]` fn in `oriterm/src/gpu/atlas/tests.rs`) are affected; the sibling `gpu::atlas::rect_packer::tests::*` (pure rect-packing, no GPU) complete in 0.00s. Symptom is reproducible from a clean build tree (`cargo clean -p oriterm` followed by `cargo test`). Earlier in the same session the same tests passed cleanly through lefthook's pre-commit hook at commit `310429a2` (round-9 TPR fixes) — something transitioned the environment or left stale GPU state that now blocks `pollster::block_on(instance.enumerate_adapters(...))` in `oriterm/src/gpu/state/helpers.rs:77`.
  Subsystem: `oriterm/src/gpu/state/helpers.rs` (`pick_adapter`, `request_device`), `oriterm/src/gpu/state/headless.rs` (`GpuState::new_headless`), `oriterm/src/gpu/atlas/tests.rs` (49 tests that all call `GpuState::new_headless()` as preamble)
  Impact: Critical — the pre-commit hook runs `cargo test` and blocks indefinitely, making every commit through lefthook impossible. Forces either `--no-verify` (banned by CLAUDE.md) or manual recovery. Also blocks `/tpr-review` iteration loops that rely on clean commits between iterations.
  Diagnostics run:
    - `timeout 300 cargo test -p oriterm --lib gpu::atlas::tests::insert_and_lookup_round_trip` — still hanging at `test has been running for over 60 seconds` after 5 minutes.
    - `RUST_LOG=info cargo test ... -- --nocapture` — NO log output before the hang (the `log::info!("GPU (headless): adapter=...")` line in `headless.rs:69` never fires), so the hang is BEFORE device init completes — likely in `enumerate_adapters` or `request_adapter`.
    - Process state: 31 threads, main thread in `futex_wait_queue`, CPU time <= 120 jiffies over 10 min — classic indefinite block.
    - Interactive `cargo test -p oriterm_core` passes in <70s and 1699/1699 tests green — hang is ISOLATED to `oriterm` crate's GPU-initializing tests.
    - `/dev/dxg` exists and is readable (WSL GPU device).
  Suspect causes:
    1. WSL `/dev/dxg` connection got into a bad state after a prior `kill -9` on a wgpu-using test process and now blocks adapter enumeration indefinitely (no timeout inside wgpu).
    2. Atlas tests are over-coupled: they use a real GPU adapter to test pure rect-packing / LRU logic that does not need GPU state. 49 `#[test]` fns each initialize a full wgpu `Instance` + `Adapter` + `Device` — if any one blocks, the whole suite blocks; and the tests cannot skip gracefully when adapter init HANGS (only when it ERRORS).
    3. `pollster::block_on(instance.enumerate_adapters(...))` has no timeout wrapper — any backend driver hang (vulkan via dzn/dxvk on WSL) blocks the test thread forever.
  Proposed fix directions (all needed; not alternatives):
    - **Design fix**: split `oriterm::gpu::atlas::tests` so the pure packing-logic tests (`insert_at_max_dimension_succeeds`, `insert_oversized_glyph_returns_none`, `glyphs_do_not_overlap`, `lru_eviction_*`, `insert_duplicate_returns_cached`, `insert_triggers_new_page_allocation`, etc.) use a fake `Device`/`Queue` or stub-buffer backend and DO NOT call `GpuState::new_headless()`. Only the handful of tests that truly exercise the GPU texture path (texture upload, format verification) should touch a real adapter. Expected: 40+ tests go from multi-minute blocking → sub-ms. This directly addresses the user's observation that "the GPU Atlas tests should be nearly instant".
    - **Infra fix**: wrap `GpuState::new_headless()` body in a bounded wait — `std::thread::spawn` + `join_timeout(Duration::from_secs(5))` around the adapter-enumeration + device-request — so a stuck backend driver produces `Err(GpuInitError)` instead of hanging. Callers already handle `Err` by skipping (`let Ok(gpu) = ... else { eprintln!("skipped: no GPU adapter available"); return; };` pattern), so the fix is localized.
    - **Graceful-skip reinforcement**: match the `Graceful Skip Protocol` from `.claude/rules/tests.md` — currently the adapter-absent path skips cleanly, but the adapter-HANG path does not. A timeout turns a hang into a skip, which is the correct degradation.
  Subsystem: `oriterm/src/gpu/atlas/` (test design), `oriterm/src/gpu/state/headless.rs` (adapter-init timeout), `oriterm/src/gpu/state/helpers.rs` (pollster::block_on wrap)
  Found: 2026-04-14 | Source: manual
  Note: Regression appeared mid-session on 2026-04-14 between 14:29 (tests passed via lefthook at commit 310429a2) and ~15:15 (tests hang). Related roadmap area: atlas test infrastructure may intersect with `oriterm/src/gpu/visual_regression/` dialog_helpers and pipeline_tests (also use `new_headless`). A fix here should double-check those paths aren't susceptible to the same hang.
  Root cause (confirmed 2026-04-14): `WAYLAND_DISPLAY=wayland-0` + `DISPLAY=:0` are set in the shell (WSLg defaults). wgpu's Vulkan backend enumeration discovers Wayland/X11 surface extensions, tries to connect to the display-server sockets as part of adapter init, and blocks indefinitely when the sockets are unresponsive (WSLg state drift mid-session). Reproduced via `timeout 30 env -i PATH="$PATH" HOME="$HOME" bash -c 'cargo test -p oriterm --lib gpu::atlas::tests::atlas_creation_succeeds'` → PASSES in 0.07s. Reproduced again via `unset WAYLAND_DISPLAY DISPLAY; cargo test ...` → PASSES in 0.06s. With the vars set in the normal shell → HANGS indefinitely. The fix on the wgpu side is pick_adapter/request_device timeout (proposed above). The test-side workaround until that lands: the test harness should unset `WAYLAND_DISPLAY` and `DISPLAY` at the top of `GpuState::new_headless()` when no window is being created, or the tests should set `WGPU_BACKEND=vulkan` and configure wgpu to skip surface-backend probing. The `Graceful Skip Protocol` from `.claude/rules/tests.md` already handles the adapter-absent case, but an adapter-HANG needs either a timeout OR pre-init env sanitization.

- [ ] `[BUG-06-012][high]` **notcurses-demo reports ~5 FPS drops that did not occur before Section 07 work**
  Repro: Run `notcurses-demo` in oriterm. Observe FPS counter — reports ~5 frame drops during playback that were not present before the Section 07 image lifecycle commits (2026-04-12 to 2026-04-14).
  Subsystem: `oriterm_core/src/image/cache/`, `oriterm_mux/src/backend/embedded/mod.rs`
  Found: 2026-04-14 | Source: manual
  Note: Overall quality and performance improved, but the FPS drops are a regression. Suspect candidates: snapshot_dirty guard change (13 methods now skip insert for nonexistent panes — could cause missed refreshes), apply_frame extraction (branch ordering change), or animation intersects_viewport predicate refactor. Active work in spec-conformance section 07 (now complete) and section 21 (notcurses-demo harness) touches this area.

- [ ] `[BUG-06-011][medium]` **Settings dialog golden image mismatch — `settings_appearance_clean_96dpi`**
  Repro: `cargo test -p oriterm --lib gpu::visual_regression::settings_dialog::settings_appearance_clean_96dpi`
  Subsystem: `oriterm/src/gpu/visual_regression/settings_dialog/`
  Found: 2026-04-12 | Source: continue-roadmap
  Note: Produces actual + diff PNGs at `oriterm/tests/references/settings_appearance_clean_96dpi_{actual,diff}.png`. May be a pixel-level drift from font/layout changes rather than a functional regression.

---

## 06.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- [x] `[TPR-06-001][high]` `oriterm/src/app/event_loop.rs:442`, `oriterm/src/gpu/state/helpers.rs:111` — the frame-budget gate was removed under the assumption that `PresentMode::Mailbox` always paces rendering, but the renderer explicitly falls back to `Immediate` when Mailbox is unavailable.
  Resolved: Added `GpuState::needs_frame_budget()` that returns true for `PresentMode::Immediate`. The rendering gate in `about_to_wait()` now applies the budget check only when the surface requires it (Immediate mode), while Mailbox/Fifo paths render immediately. Fixed 2026-03-30.

- [x] `[TPR-06-002][medium]` `oriterm/src/app/perf_stats.rs:305` — the new phase-breakdown instrumentation logs at `info` level even when profiling is disabled.
  Resolved: Phase breakdown logging now routes through the same `log_fn`/`self.profiling` gate as the rest of the perf output. Fixed 2026-03-30.

- [x] `[TPR-06-003][high]` `oriterm/src/app/init/mod.rs:36`, `oriterm/src/app/window_management.rs:133`, `oriterm/src/gpu/state/mod.rs:80`, `oriterm/src/gpu/state/mod.rs:104`, `oriterm_ui/src/window/mod.rs:241` — `use_compositor_surface` is decided from the requested config backend before GPU initialization, but `GpuState::new()` can still fall back from the DX12 `DirectComposition` path to plain DX12 or Vulkan when that init attempt fails. Because the window is already created with `WS_EX_NOREDIRECTIONBITMAP`, the fallback backend inherits a compositor-surface window it cannot present to correctly, which reintroduces the invisible-window failure on the exact fallback path this patch is trying to harden.
  Resolved: Fixed on 2026-03-31. Added `uses_dcomp` field to `GpuState` (set during `try_init`). `init/mod.rs` now calls `clear_compositor_surface_flag()` after GPU init if DComp wasn't actually used. `window_management.rs` uses `gpu.uses_dcomp()` instead of config-based backend check for new windows. `clear_compositor_surface_flag()` added to `oriterm_ui/src/window/mod.rs` (Win32 FFI to remove `WS_EX_NOREDIRECTIONBITMAP`).

- [x] `[TPR-06-004][high]` `oriterm_ui/src/window/mod.rs:339` — `clear_compositor_surface_flag()` clears `WS_EX_NOREDIRECTIONBITMAP` with `SetWindowLongPtrW`, but never follows up with `SetWindowPos(..., SWP_FRAMECHANGED)`. Microsoft’s `SetWindowLongPtrW` docs state that cached window data does not take effect until `SetWindowPos` is called, so the fallback path in `oriterm/src/app/init/mod.rs:70` can still leave the startup window in the compositor-surface state it was created with.
  Resolved: Fixed on 2026-03-31. Added `SetWindowPos(SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED)` after `SetWindowLongPtrW` in `clear_compositor_surface_flag()`.

- [x] `[TPR-06-005][high]` `oriterm/src/app/init/mod.rs:214`, `oriterm/src/app/window_management.rs:95` — the steady-state render path now correctly forces opacity to `1.0` on surfaces without alpha support (`handle_redraw` and `handle_redraw_multi_pane` both do this), but the pre-show `gpu.clear_surface()` path still uses the configured window opacity unconditionally. On the same Vulkan/opaque fallback path this patch is hardening, the first presented frame can therefore still be rendered with the invalid sub-1.0 opacity before the later redraw clamps it.
  Resolved: Fixed on 2026-03-31. Both `init/mod.rs` and `window_management.rs` now check `gpu.supports_transparency()` and clamp opacity to 1.0 when the surface lacks alpha support.

---
