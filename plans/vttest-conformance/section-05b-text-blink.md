---
section: "05B"
title: "Text Blink Rendering (SGR 5/6)"
status: not-started
reviewed: true
goal: "Cells with CellFlags::BLINK visually blink at configurable rate, verified by GPU visual regression tests"
inspired_by:
  - "WezTerm text_blink_rate (config/src/config.rs:667-685)"
  - "Existing cursor blink pipeline (oriterm_ui/src/animation/cursor_blink/mod.rs)"
depends_on: ["05"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "05B.1"
    title: "Add text_blink_opacity to FrameInput"
    status: not-started
  - id: "05B.2"
    title: "Modulate fg_dim for BLINK Cells"
    status: not-started
  - id: "05B.3"
    title: "App Timer + Config"
    status: not-started
  - id: "05B.4"
    title: "Tests"
    status: not-started
  - id: "05B.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "05B.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 05B: Text Blink Rendering (SGR 5/6)

**Status:** Not Started
**Goal:** Terminal cells with the BLINK attribute (SGR 5 slow, SGR 6 rapid) visually blink. The existing `fg_dim` alpha pipeline carries the blink opacity to the GPU — no shader changes needed.

**Context:** The VTE handler already parses SGR 5/6 and stores `CellFlags::BLINK` per cell (`oriterm_core/src/term/handler/sgr.rs:46`). The GPU prepare pipeline already has an `fg_dim: f32` opacity parameter that flows to every `push_glyph()` call. But no code reads the BLINK flag during rendering — blinking text renders as static text. vttest menu 2 screens 13-14 test SGR graphic rendition including blink. This section makes blink visible.

**Design:** Reuse the existing `CursorBlink` type as a `TextBlink` timer on App. Each frame, compute `text_blink_opacity: f32` from the timer and pass it through `FrameInput`. In the prepare pipeline, cells with `CellFlags::BLINK` have their glyph alpha multiplied by this opacity. The blink rate is configurable (default 500ms, matching WezTerm). Both SGR 5 and SGR 6 map to the same `CellFlags::BLINK` flag — they blink at the same rate. If separate rates are needed later, a `RAPID_BLINK` flag can be added to `CellFlags`.

**Reference implementations:**
- **WezTerm** `config/src/config.rs:667-685`: `text_blink_rate = 500ms`, `text_blink_rate_rapid = 250ms`, separate easing functions.
- **Existing cursor blink** `oriterm_ui/src/animation/cursor_blink/mod.rs`: `CursorBlink` with `intensity() -> f32` and `next_change() -> Instant`.

**Depends on:** Section 05 (cursor fade blink — provides `CursorBlink` type with `intensity()` API).

---

## 05B.1 Add `text_blink_opacity` to FrameInput

**File(s):** `oriterm/src/gpu/frame_input/mod.rs`

Add a field to `FrameInput` that carries the current text blink opacity for the frame. This is the bridge between the app-layer timer and the GPU prepare pipeline.

- [ ] Add `text_blink_opacity: f32` field to `FrameInput` at `frame_input/mod.rs` (after `fg_dim`). Default: `1.0` (no blink effect). Doc comment: "Opacity multiplier for cells with `CellFlags::BLINK` (0.0 = hidden, 1.0 = visible)."
- [ ] Update `FrameInput::test_grid()` to set `text_blink_opacity: 1.0`
- [ ] Update all existing `FrameInput` construction sites that use struct literal syntax (search for `fg_dim:` to find them — the new field must be added alongside)

---

## 05B.2 Modulate `fg_dim` for BLINK Cells

**File(s):** `oriterm/src/gpu/prepare/mod.rs`, `oriterm/src/gpu/prepare/dirty_skip/mod.rs`, `oriterm/src/gpu/prepare/unshaped.rs`, `oriterm/src/gpu/prepare/emit.rs`

In the per-cell loop of each prepare path, check `CellFlags::BLINK` and multiply the glyph alpha by `text_blink_opacity`.

The key insight: `fg_dim` is already extracted as a local `let fg_dim = input.fg_dim;` at the top of each fill function. For blink cells, we compute a per-cell effective dim: `let effective_dim = if cell.flags.contains(CellFlags::BLINK) { fg_dim * text_blink_opacity } else { fg_dim };`. This effective dim flows to all glyph emission calls for that cell.

- [ ] In `fill_frame_shaped()` at `prepare/mod.rs`: extract `let text_blink_opacity = input.text_blink_opacity;` alongside `fg_dim`. In the per-cell loop, compute `let cell_dim = if cell.flags.contains(CellFlags::BLINK) { fg_dim * text_blink_opacity } else { fg_dim };`. Pass `cell_dim` to the `GlyphEmitter` for that cell's glyphs (the emitter already accepts `fg_dim` — thread the per-cell value through).
  - **Important:** `GlyphEmitter` is constructed once and reused for all cells. The simplest approach: set `emitter.fg_dim = cell_dim;` before emitting each cell's glyphs, then restore to `fg_dim` after. Or pass `cell_dim` as a parameter to `emitter.emit()`.
- [ ] In `fill_frame_incremental()` at `prepare/dirty_skip/mod.rs`: same pattern — per-cell dim based on BLINK flag.
- [ ] In `fill_frame()` at `prepare/unshaped.rs`: same pattern.
- [ ] For builtin geometric glyphs emitted directly via `push_glyph()` at `prepare/mod.rs:421`: use `cell_dim` instead of `fg_dim`.
- [ ] Verify decorations at `prepare/decorations.rs` — underlines/strikethrough on BLINK cells should also fade. Currently decorations use a fixed `1.0` alpha. If the decoration is on a BLINK cell, the decoration should fade too. This may require passing `text_blink_opacity` to the decoration emitter.
- [ ] `./build-all.sh` and `./clippy-all.sh` green after changes

---

## 05B.3 App Timer + Config

**File(s):** `oriterm/src/app/mod.rs`, `oriterm/src/app/event_loop.rs`, `oriterm/src/app/redraw/mod.rs`, `oriterm/src/app/redraw/multi_pane/mod.rs`, `oriterm/src/config/mod.rs`, `oriterm/src/app/config_reload/mod.rs`, `oriterm/src/app/settings_overlay/`

Add a `text_blink: CursorBlink` timer to App, drive it from the event loop, and pass its opacity to the redraw paths.

- [ ] Add `text_blink: CursorBlink` field to `App` at `app/mod.rs` (below `cursor_blink`)
- [ ] Initialize in constructor at `constructors.rs` with configurable interval (default: 500ms)
- [ ] Add config fields to `TerminalConfig`:
  - `text_blink_rate_ms: u64` (default: 500) — blink rate for SGR 5/6
  - `text_blink_fade: bool` (default: true) — smooth fade vs hard toggle (reuses cursor fade pattern)
- [ ] Add settings dialog toggle for text blink fade (in terminal section, below cursor blink fade)
- [ ] In `event_loop.rs` `about_to_wait()`: call `text_blink.update()` and mark dirty if changed. Always run text blink (unlike cursor blink which is gated by `blinking_active` — text blink is always active because any cell in any pane could have BLINK)
- [ ] Update `ControlFlowInput`: add `text_blink_active: bool` and `next_text_blink_change: Instant`. Scheduling logic: use `min(next_blink_change, next_text_blink_change)` when both are active.
  - `text_blink_active` is true when any visible cell has the BLINK flag. To avoid scanning all cells each frame, use a conservative approach: set `text_blink_active = true` always (the timer runs cheaply even with no BLINK cells; the only cost is a plateau wakeup every 500ms which is negligible).
- [ ] In `redraw/mod.rs`: compute `text_blink_opacity` from `self.text_blink.intensity()` (apply fade toggle). Set `frame.text_blink_opacity = text_blink_opacity;` before calling `prepare()`.
- [ ] In `redraw/multi_pane/mod.rs`: same — compute opacity, set on each pane's `FrameInput`.
- [ ] Config reload handler at `config_reload/mod.rs`: update `text_blink.set_interval()` when `text_blink_rate_ms` changes.
- [ ] Text blink timer does NOT reset on keypress (unlike cursor blink). It runs continuously.
- [ ] `./build-all.sh`, `./clippy-all.sh`, `./test-all.sh` green

---

## 05B.4 Tests

**File(s):** `oriterm/src/gpu/visual_regression/`, `oriterm/src/gpu/prepare/tests.rs`, `oriterm/src/config/tests.rs`

- [ ] GPU visual regression test: `text_blink_visible` — render a cell with `CellFlags::BLINK` and `text_blink_opacity = 1.0`, verify glyph is visible (pixel matches non-blink cell)
- [ ] GPU visual regression test: `text_blink_hidden` — render with `text_blink_opacity = 0.0`, verify BLINK cell's glyph is invisible (pixel matches background) while non-BLINK cells remain visible
- [ ] GPU visual regression test: `text_blink_half` — render with `text_blink_opacity = 0.5`, verify BLINK cell's glyph brightness is intermediate
- [ ] Prepare unit test: `blink_cell_gets_dimmed_fg` — construct a `FrameInput` with one BLINK cell and `text_blink_opacity = 0.5`, run through `prepare_frame`, verify the glyph instance has alpha = `fg_dim * 0.5`
- [ ] Prepare unit test: `non_blink_cell_ignores_text_blink_opacity` — same input but cell without BLINK flag, verify alpha = `fg_dim`
- [ ] Config test: `text_blink_defaults` — verify `text_blink_rate_ms = 500` and `text_blink_fade = true`
- [ ] Config test: `text_blink_from_toml` — verify deserialization
- [ ] Existing `compute_control_flow` tests still pass
- [ ] `./test-all.sh` green

---

## 05B.R Third Party Review Findings

- None.

---

## 05B.N Completion Checklist

- [ ] Cells with `CellFlags::BLINK` visually blink (not static)
- [ ] Blink rate defaults to 500ms (configurable via `text_blink_rate_ms`)
- [ ] Smooth fade configurable via `text_blink_fade` (settings dialog toggle)
- [ ] Non-BLINK cells unaffected by text blink opacity
- [ ] vttest menu 2 screens 13-14 show blinking text
- [ ] GPU visual regression tests pass (3 opacity levels)
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./test-all.sh` green
- [ ] `/tpr-review` passed

**Exit Criteria:** Blinking text (SGR 5/6) visually animates in the terminal. GPU tests verify BLINK-flagged cells respond to text_blink_opacity while non-BLINK cells are unaffected. Configuration allows adjusting rate and enabling/disabling fade.
