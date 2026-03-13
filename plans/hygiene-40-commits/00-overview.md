---
plan: "hygiene-40-commits"
title: "Implementation Hygiene Fixes from 40-Commit Review"
status: in-progress
references:
  - "plans/roadmap/section-23-performance.md"
---

# Implementation Hygiene Fixes from 40-Commit Review

## Mission

Fix ~120 implementation hygiene findings discovered by reviewing all files touched in the last 40 commits across all 5 crates plus VTE. This is not architecture work or feature work — it is tightening joints: eliminating panics on recoverable errors, reducing unnecessary visibility, fixing allocation waste, correcting drifted logic, closing gaps in stubbed functionality, and splitting oversize files. Every finding traces to a concrete code location.

## Scope

Six crate/module boundaries, each with a dedicated section:

1. **oriterm_core** — Term, Grid, Image subsystems. **COMPLETE.** All `.expect()` calls removed, oversize files split, `pub` surface tightened, scrollback clearing implemented, Kitty path traversal fixed. Allocation waste items deferred to roadmap Section 23.
2. **oriterm_mux** — Server, Protocol, PTY subsystems. Stale ID allocation, missing IPC forwarding, O(n*m) subscription sync, buffer shrink gaps. (Original Finding 1 about duplicated `set_common_env` was verified as already correct.)
3. **oriterm/app** — Event loop, Input, Redraw. Panicking `.expect()` in mouse handlers, per-frame Vec allocations, duplicated platform logic, oversize event_loop.rs.
4. **oriterm/gpu** — Render pipeline. Wrong sRGB conversion, O(n^2) eviction, per-frame allocations, excessive `pub` on internal types, oversize pipeline.rs.
5. **oriterm_ui** — Widget tree. Dead enum variants, `Instant::now()` in widgets, per-frame allocations in slider/dropdown/menu, `pub` fields that should be `pub(super)`.
6. **Misc + VTE** — Config, Keybindings, WindowManager, VTE crate. VTE ansi.rs at 2686 lines, inline tests violating sibling pattern, `pub` types in binary crate, type-unsafe event variants.

A final cleanup section (Section 7) runs the full verification suite.

## Design Principles

**No panics on recoverable errors.** Every `.expect()` and `.unwrap()` on user-reachable paths must be replaced with graceful fallbacks. The terminal must never crash because a PTY closed, an alt screen wasn't initialized, or a mouse event arrived before a mux was ready.

**Minimal visibility.** `pub` is the default only for items consumed by downstream crates. Within a binary crate, everything is `pub(crate)` or tighter. Leaked `pub` fields invite accidental coupling.

**Files under 500 lines.** The project rule is enforced proactively. Files at or above the limit get split before they grow further.

## Section Dependency Graph

```
Section 01 (core)  ──┐
Section 02 (mux)   ──┤
Section 03 (app)   ──┤── all independent ──→ Section 07 (cleanup)
Section 04 (gpu)   ──┤
Section 05 (ui)    ──┤
Section 06 (misc)  ──┘
```

Sections 01-06 are independent and can be worked in any order. Section 07 (cleanup/verification) depends on all of them.

**Cross-section interactions:**
- **Section 01 + Section 04**: `zerowidth.clone()` appears in both core snapshot and GPU extraction. If core changes the `zerowidth` type from `Vec<char>` to `Option<Vec<char>>`, GPU extraction must update simultaneously.
- **Section 01 + Section 03**: `delimiter_class` is duplicated between core and app/snapshot_grid. Whichever section makes it `pub`, the other must consume it.
- **Section 03 + Section 06**: `MoveTabToNewWindow(usize)` in `event.rs` must change to `TabId` — this is in Section 06 but app event handlers (Section 03) must update call sites.

## Implementation Sequence

```
Phase 0 - Safety (LEAKs)
  +-- Section 01: Replace .expect() in grid()/image_cache() (4 call sites) [DONE]
  +-- Section 03: Replace .expect() in mouse handlers + mark_mode (4 call sites)

Phase 1 - Correctness (DRIFTs + GAPs)
  +-- Section 01: Implement scrollback clearing, fix Kitty path traversal [DONE]
  +-- Section 02: Fix DomainId off-by-one, missing IPC forwarding
  +-- Section 03: Deduplicate SelectAll, extract platform merge helpers
  +-- Section 04: Fix sRGB clear color, reserve logic, stale window_focused
  +-- Section 05: Remove dead EventResponse::RequestRedraw, WidgetResponse::redraw()
  +-- Section 06: Fix key_to_binding_key allocation, TabId type safety

Phase 2 - Efficiency (WASTEs)
  +-- All sections: Fix per-frame allocations, O(n^2) algorithms, unnecessary clones
  Gate: ./test-all.sh passes, no new clippy warnings

Phase 3 - Tightening (EXPOSUREs + BLOATs)
  +-- All sections: Reduce pub to pub(crate), split oversize files
  Gate: ./clippy-all.sh clean, all files under 500 lines

Phase 4 - Verification
  +-- Section 07: test-all, clippy-all, build-all green
```

## Relationship to Roadmap Section 23

Several findings overlap with `plans/roadmap/section-23-performance.md` (in-progress). These are marked `[PLANNED]` in each section and should NOT be implemented here — they will be addressed as part of the performance roadmap. Specifically:

- **oriterm_core snapshot allocations** (findings 4, 5, 6): `Vec::new()` per cell, `zerowidth.clone()`, `Vec::new()` per frame in `extract_images`
- **oriterm_core image cache** (findings 7, 8): `placed_id_set()` O(placements), O(animations * placements) visibility check
- **oriterm_core grid hot path** (finding 23): `tmpl_extra.clone()` Arc bump in `put_char`
- **oriterm_mux snapshot** (finding 2): `icon_name`/`cwd` per snapshot cycle
- **oriterm_mux push path** (finding 8): `snapshot.clone()` deep copy
- **oriterm_mux frame_io/codec** (findings 12, 13): buffer shrink discipline

## Estimated Effort

| Section | Actionable Findings | Est. Complexity | Depends On |
|---------|-------------------|----------------|------------|
| 01 oriterm_core | ~18 (6 deferred) | Medium | — |
| 02 oriterm_mux | ~15 (4 deferred) | Medium | — |
| 03 oriterm/app | ~15 | Medium | — |
| 04 oriterm/gpu | ~18 | Medium-High | — |
| 05 oriterm_ui | ~17 | Medium | — |
| 06 Misc + VTE | ~20 | High (VTE split) | — |
| 07 Cleanup | 1 (verification) | Low | 01-06 |
| **Total** | **~104 actionable** | | |

## Quick Reference

| ID | Title | File | Status |
|----|-------|------|--------|
| 01 | oriterm_core — Term/Grid/Image Boundaries | `section-01-oriterm-core.md` | Complete |
| 02 | oriterm_mux — Server/Protocol/PTY Boundaries | `section-02-oriterm-mux.md` | Not Started |
| 03 | oriterm/app — Event Loop/Input/Redraw | `section-03-oriterm-app.md` | Not Started |
| 04 | oriterm/gpu — Render Pipeline | `section-04-oriterm-gpu.md` | Not Started |
| 05 | oriterm_ui — Widget Tree | `section-05-oriterm-ui.md` | Not Started |
| 06 | oriterm misc — Config/Keybindings/WindowManager/VTE | `section-06-misc-vte.md` | Not Started |
| 07 | Cleanup | `section-07-cleanup.md` | Not Started |
