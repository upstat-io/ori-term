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

- [ ] **BUG-06.8**: Floating pane (Ctrl+Shift+P) is completely transparent — no background fill
  - **Severity**: high
  - **File(s)**: `oriterm/src/gpu/window_renderer/multi_pane.rs` (`append_floating_decoration`), `oriterm/src/app/redraw/multi_pane/mod.rs`
  - **Root cause**: `append_floating_decoration()` only renders a drop shadow and accent border around floating panes. There is no opaque background fill — the floating pane's cell backgrounds are composited directly over the main window content beneath, making it appear fully transparent. The floating pane should use the same background settings as the main window (opacity, blur, bg color).
  - **Repro**: Press Ctrl+Shift+P to toggle a floating pane. The pane content is transparent — you can see the main terminal grid behind it.
  - **Found**: 2026-03-31 — manual, user report.

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
