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

- [ ] **BUG-06.1**: Noticeable input lag during key repeat — worse at smaller window widths
  - **Severity**: critical
  - **File(s)**: `oriterm/src/app/event_loop.rs` (render loop), `oriterm/src/app/redraw/mod.rs` (frame building), GPU renderer (draw_frame)
  - **Root cause**: TBD — requires profiling. Likely candidates:
    1. **Full frame render per keystroke**: Each key repeat triggers PTY write → PTY read → VTE parse → grid mutation → mark dirty → full `render_dirty_windows()`. If frame time exceeds the key repeat interval (~30ms at 33Hz repeat rate), input events queue up and the terminal feels laggy.
    2. **Smaller window = more wrapping**: Narrower windows cause more line wrapping, producing more visible rows/cells per frame. If the render path scales with visible cell count (likely — cell loop in GPU renderer), narrower windows are slower per frame.
    3. **No input coalescing**: Multiple pending key events may each trigger separate PTY writes and redraws instead of batching. The `FRAME_BUDGET` throttle exists but if `pump_mux_events()` + render exceeds budget, events still pile up.
    4. **Synchronous PTY round-trip**: Key press → PTY write → wait for PTY read → VTE parse → render. If this is synchronous rather than pipelined, each keystroke pays full round-trip latency.
  - **Repro**: Hold any key (e.g., 'a') in the terminal. Observe characters appearing with visible delay. Resize window narrower — lag increases.
  - **Found**: 2026-03-29 — manual, user report
  - **Fix**: Requires profiling to identify the dominant bottleneck. Potential fixes include: input coalescing (batch multiple key events before render), damage-tracked partial redraws (only re-render changed rows), decoupling input dispatch from render (process all pending input before any render), and ensuring the PTY read → render pipeline is non-blocking.

- [ ] **BUG-06.2**: Random extra text appears after resize following sustained key repeat
  - **Severity**: medium
  - **File(s)**: `oriterm/src/app/event_loop.rs` (resize handling), `oriterm_mux/` (PTY resize notification)
  - **Root cause**: TBD. Likely a race between queued key repeat events and the PTY resize (SIGWINCH) notification — the shell processes both simultaneously, producing interleaved output. WezTerm exhibits the same behavior; Alacritty does not.
  - **Repro**: Hold a key to fill the screen with text, release, then resize the window. Extra/garbled characters appear in the terminal.
  - **Found**: 2026-03-30 — manual, user report. Pre-existing — not caused by frame budget changes.

---

## 06.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- [x] `[TPR-06-001][high]` `oriterm/src/app/event_loop.rs:442`, `oriterm/src/gpu/state/helpers.rs:111` — the frame-budget gate was removed under the assumption that `PresentMode::Mailbox` always paces rendering, but the renderer explicitly falls back to `Immediate` when Mailbox is unavailable.
  Resolved: Added `GpuState::needs_frame_budget()` that returns true for `PresentMode::Immediate`. The rendering gate in `about_to_wait()` now applies the budget check only when the surface requires it (Immediate mode), while Mailbox/Fifo paths render immediately. Fixed 2026-03-30.

- [x] `[TPR-06-002][medium]` `oriterm/src/app/perf_stats.rs:305` — the new phase-breakdown instrumentation logs at `info` level even when profiling is disabled.
  Resolved: Phase breakdown logging now routes through the same `log_fn`/`self.profiling` gate as the rest of the perf output. Fixed 2026-03-30.

---
