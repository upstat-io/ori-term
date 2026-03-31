---
section: "06"
title: "Rendering & Performance Bugs"
status: in-progress
reviewed: true
goal: "Track and fix rendering performance bugs — frame time, input latency, GPU bottlenecks"
depends_on: []
third_party_review:
  status: resolved
  updated: 2026-03-30
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

- [ ] **BUG-06.2**: Random extra text appears after resize following sustained key repeat
  - **Severity**: medium
  - **File(s)**: `oriterm/src/app/event_loop.rs` (resize handling), `oriterm_mux/` (PTY resize notification)
  - **Root cause**: TBD. Likely a race between queued key repeat events and the PTY resize (SIGWINCH) notification — the shell processes both simultaneously, producing interleaved output. WezTerm exhibits the same behavior; Alacritty does not.
  - **Repro**: Hold a key to fill the screen with text, release, then resize the window. Extra/garbled characters appear in the terminal.
  - **Found**: 2026-03-30 — manual, user report. Pre-existing — not caused by frame budget changes.

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

---

## 06.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- [x] `[TPR-06-001][high]` `oriterm/src/app/event_loop.rs:442`, `oriterm/src/gpu/state/helpers.rs:111` — the frame-budget gate was removed under the assumption that `PresentMode::Mailbox` always paces rendering, but the renderer explicitly falls back to `Immediate` when Mailbox is unavailable.
  Resolved: Added `GpuState::needs_frame_budget()` that returns true for `PresentMode::Immediate`. The rendering gate in `about_to_wait()` now applies the budget check only when the surface requires it (Immediate mode), while Mailbox/Fifo paths render immediately. Fixed 2026-03-30.

- [x] `[TPR-06-002][medium]` `oriterm/src/app/perf_stats.rs:305` — the new phase-breakdown instrumentation logs at `info` level even when profiling is disabled.
  Resolved: Phase breakdown logging now routes through the same `log_fn`/`self.profiling` gate as the rest of the perf output. Fixed 2026-03-30.

---
