---
section: "01"
title: "Redraw Pipeline Consolidation"
status: complete
reviewed: true
goal: "Eliminate ~150 lines of algorithmic duplication between single-pane and multi-pane render paths"
depends_on: []
third_party_review:
  status: resolved
  updated: 2026-04-03
sections:
  - id: "01.1"
    title: "Extract Chrome Rendering Helper"
    status: complete
  - id: "01.2"
    title: "Extract Blink Opacity Helper"
    status: complete
  - id: "01.3"
    title: "Extract Opacity Resolution Helper"
    status: complete
  - id: "01.4"
    title: "Extract Frame-After-Swap Reset"
    status: complete
  - id: "01.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "01.N"
    title: "Completion Checklist"
    status: complete
---

# Section 01: Redraw Pipeline Consolidation

**Status:** Not Started
**Goal:** Extract shared algorithms from single-pane and multi-pane render paths into helper functions, eliminating ~150 lines of duplicated code while preserving both render strategies.

**Context:** The single-pane path (`redraw/mod.rs:handle_redraw`) and multi-pane path (`redraw/multi_pane/mod.rs:handle_redraw_multi_pane`) share identical chrome rendering, opacity resolution, blink threshold, and frame-reset code. Any behavioral change (new chrome element, new opacity rule) requires edits in two places — classic SSOT violation.

---

## 01.1 Extract Chrome Rendering Helper

**File(s):** `oriterm/src/app/redraw/chrome.rs` (new), `oriterm/src/app/redraw/mod.rs`, `oriterm/src/app/redraw/multi_pane/mod.rs`

The chrome rendering pipeline (phase gating → tab bar → overlays → search bar → status bar → window border → render_to_surface) was duplicated between single-pane and multi-pane paths. Extracted to `chrome.rs` with `ChromeParams` struct for the varying parts (pane count, content_dirty, selection_changed, blink_changed). Search state and grid dimensions read from `ctx.frame` internally.

- [x] Create a helper in `redraw/chrome.rs` that encapsulates the shared chrome rendering pipeline. Accept parameters for the variable parts: pane count (for status bar), whether content changed (for `needs_full_render`), and any other single-pane vs multi-pane differences.
- [x] Refactor `handle_redraw()` in `redraw/mod.rs` to call the new helper after its pane-specific work.
- [x] Refactor `handle_redraw_multi_pane()` in `multi_pane/mod.rs` to call the same helper.
- [x] Verify that `multi_pane/mod.rs` drops below 500 lines after extraction (509 → 400 lines).

---

## 01.2 Extract Blink Opacity Helper

**File(s):** `oriterm/src/app/redraw/mod.rs`, `oriterm/src/app/redraw/multi_pane/mod.rs`

The blink opacity threshold pattern appears 4 times:
- `redraw/mod.rs:244-255` (cursor blink, single-pane)
- `redraw/mod.rs:258-267` (text blink, single-pane)
- `multi_pane/mod.rs:283-292` (text blink, multi-pane)
- `multi_pane/mod.rs:297-312` (cursor blink, multi-pane)

All share: `raw intensity → if fade { raw } else if raw > 0.5 { 1.0 } else { 0.0 }`.

- [x] Add named constants in `oriterm/src/app/redraw/draw_helpers.rs`: `BLINK_SNAP_THRESHOLD` (0.5) and `BLINK_OPACITY_EPSILON` (0.001).
- [x] Create a helper function `blink_opacity(raw, use_fade)` in `draw_helpers.rs`.
- [x] Replace all 4 inline computations with calls to `blink_opacity()`. Grep confirms zero remaining inline copies.
- [x] Named the `0.001` blink delta threshold as `BLINK_OPACITY_EPSILON`.

---

## 01.3 Extract Opacity Resolution Helper

**File(s):** `oriterm/src/app/redraw/mod.rs`, `oriterm/src/app/redraw/multi_pane/mod.rs`

Window opacity resolution (surface alpha check → focused/unfocused opacity) appears 3 times:
- `redraw/mod.rs:163-169`
- `multi_pane/mod.rs:80-88`
- `multi_pane/mod.rs:198-204`

- [x] Created `resolve_palette_opacity()` in `draw_helpers.rs`.
- [x] Replaced all 3 inline computations. Only the canonical helper calls `effective_opacity()`/`effective_unfocused_opacity()` now.

---

## 01.4 Extract Frame-After-Swap Reset

**File(s):** `oriterm/src/app/redraw/mod.rs`, `oriterm/src/app/redraw/multi_pane/mod.rs`

Frame field resets after `swap_renderable_content` are duplicated:
- `redraw/mod.rs:124-137` (7 field resets)
- `multi_pane/mod.rs:145-160` (8 field resets — includes `window_focused`)

- [x] Added `clear_transient_fields()` method on `FrameInput` in `gpu/frame_input/mod.rs`.
- [x] Both swap paths call `clear_transient_fields()`. Multi-pane also sets `window_focused = true` after the shared reset.

---

## 01.R Third Party Review Findings

- [x] `[TPR-01-001][medium]` `oriterm/src/app/redraw/multi_pane/mod.rs` — split-pane text blink stale for clean unfocused panes.
  Resolved: Fixed on 2026-04-03. Computed `text_blink_opacity` once before the pane loop, detect changes via `BLINK_OPACITY_EPSILON` against `prev_text_blink_opacity`, and include `blink_opacity_changed` in the per-pane dirty check. Also split `frame_input/mod.rs` (531→391 lines) by extracting `FrameSearch` to `search.rs`.

---

## 01.N Completion Checklist

- [x] Chrome rendering exists in exactly one location (`chrome::render_chrome`), called from both paths
- [x] Blink opacity computation exists in exactly one function (`blink_opacity()`), called from all 4 sites
- [x] Opacity resolution exists in exactly one function (`resolve_palette_opacity()`), called from all 3 sites
- [x] Frame-after-swap reset exists in exactly one function (`FrameInput::clear_transient_fields()`), called from both paths
- [x] `multi_pane/mod.rs` is under 500 lines (378)
- [x] `./test-all.sh` green
- [x] `./build-all.sh` green
- [x] `./clippy-all.sh` green
- [x] `/tpr-review` passed — 1 finding accepted and fixed (stale blink for cached panes + file size)

**Exit Criteria:** Each of the 4 extracted algorithms has exactly one canonical definition. Grep for the old inline patterns confirms zero remaining copies.
