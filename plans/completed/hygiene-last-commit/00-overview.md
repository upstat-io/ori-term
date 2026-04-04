---
plan: "hygiene-last-commit"
title: "Hygiene: Last Commit (Blink Timer + Event Loop)"
status: complete
references:
  - "plans/completed/vttest-conformance/section-05-fade-blink.md"
---

# Hygiene: Last Commit (Blink Timer + Event Loop)

## Mission

Fix 23 implementation hygiene findings discovered in a review scoped to the last commit (`c34198ba` — blink timer path changes). The review expanded to `oriterm/src/app/` (event loop, redraw pipeline, blink timers) and traced data flow into `oriterm/src/gpu/` and `oriterm_ui/src/animation/cursor_blink/`. The dominant issue is massive algorithmic duplication between the single-pane and multi-pane render paths (~150+ shared skeleton lines).

## Architecture

```
about_to_wait()
  ├── drive_blink_timers()  → marks dirty + request_redraw
  ├── tick animations
  ├── render_dirty_windows()
  │     ├── [windows loop] → handle_redraw()
  │     │                      ├── EXTRACT (snapshot → frame)      ← DUPLICATED (single/multi)
  │     │                      ├── ANNOTATE (selection, search)    ← DUPLICATED
  │     │                      ├── CHROME (tab bar, overlays, etc) ← DUPLICATED
  │     │                      └── SUBMIT (render_to_surface)
  │     └── [dialogs loop] → render_dialog()                      ← DUPLICATED SKELETON
  ├── compute_control_flow()  → ControlFlow decision
  └── schedule_blink_wakeup() → thread::spawn per tick            ← WASTE
```

Key data flow: `CursorBlink::intensity()` → blink opacity threshold → `FrameInput.text_blink_opacity` / `cursor_opacity` → GPU prepare → instance buffers.

## Design Principles

1. **Extract shared algorithms, not shared code**: The single-pane and multi-pane paths differ in loop structure (one pane vs. N panes). The shared chrome rendering, opacity resolution, and blink threshold logic are extractable algorithms that should become shared helper functions called by both paths. Do NOT merge the paths into one — they have legitimately different pane iteration strategies.

2. **Name your magic**: Bare `0.5`, `0.001`, `16` scattered across blink code are the kind of inline policy that drifts. Named constants make the intent greppable and changeable from one location.

## Section Dependency Graph

```
01 (Redraw Pipeline)  ──→  05 (Cleanup)
02 (Blink Timers)     ──→  05 (Cleanup)
03 (Render Dispatch)  ──→  05 (Cleanup)
04 (Scattered + Bloat) ──→ 05 (Cleanup)
```

Sections 01–04 are independent and can be worked in any order. Section 05 requires all.

## Finding Summary

| Category | Count | Severity |
|----------|-------|----------|
| LEAK:algorithmic-duplication | 8 | Critical |
| LEAK:scattered-knowledge | 2 | Critical |
| GAP | 3 | Major |
| WASTE | 4 | Minor |
| BLOAT | 4 | Minor |
| EXPOSURE | 1 | Informational |
| DRIFT | 1 | Minor |
| **Total** | **23** | |

## Quick Reference

| ID | Title | File | Status |
|----|-------|------|--------|
| 01 | Redraw Pipeline Consolidation | `section-01-redraw-pipeline.md` | Complete |
| 02 | Blink Timer Fixes | `section-02-blink-timers.md` | Complete |
| 03 | Render Dispatch Consolidation | `section-03-render-dispatch.md` | Complete |
| 04 | Scattered Knowledge and Bloat | `section-04-scattered-and-bloat.md` | Complete |
| 05 | Cleanup | `section-05-cleanup.md` | Complete |
