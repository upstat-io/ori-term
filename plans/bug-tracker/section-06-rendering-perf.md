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
    status: in-progress
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

- [ ] **BUG-06.4**: Settings dialog shows baby blue flash on open/close
  - **Severity**: medium
  - **File(s)**: `oriterm/src/app/dialog_management.rs` (dialog lifecycle), `oriterm/src/app/dialog_rendering.rs` (first frame)
  - **Root cause**: TBD. The dialog uses the Primed lifecycle (render first frame → show on next tick with DWM transition suppression), but a brief baby blue flash is still visible. May be a timing issue between `render_to_surface()` and DWM composition, or the GPU not flushing the first frame before showing (unlike `clear_surface()` which calls `device.poll()`).
  - **Found**: 2026-03-30 — user report. Only affects settings dialog, not terminal windows.
  - **Note**: Terminal windows use `clear_surface()` + `device.poll()` before showing. Dialogs use `render_dialog()` (no GPU flush) + Primed lifecycle. Adding `device.poll()` after the first dialog render might fix it.

---

## 06.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- [x] `[TPR-06-001][high]` `oriterm/src/app/event_loop.rs:442`, `oriterm/src/gpu/state/helpers.rs:111` — the frame-budget gate was removed under the assumption that `PresentMode::Mailbox` always paces rendering, but the renderer explicitly falls back to `Immediate` when Mailbox is unavailable.
  Resolved: Added `GpuState::needs_frame_budget()` that returns true for `PresentMode::Immediate`. The rendering gate in `about_to_wait()` now applies the budget check only when the surface requires it (Immediate mode), while Mailbox/Fifo paths render immediately. Fixed 2026-03-30.

- [x] `[TPR-06-002][medium]` `oriterm/src/app/perf_stats.rs:305` — the new phase-breakdown instrumentation logs at `info` level even when profiling is disabled.
  Resolved: Phase breakdown logging now routes through the same `log_fn`/`self.profiling` gate as the rest of the perf output. Fixed 2026-03-30.

---
