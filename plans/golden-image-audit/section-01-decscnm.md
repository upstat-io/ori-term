---
section: "01"
title: "DECSCNM Reverse Video Rendering"
status: not-started
reviewed: true
goal: "vttest light-background screens render with white background and dark text"
inspired_by:
  - "WezTerm reverse_video_mode (term/src/terminalstate/mod.rs:274, wezterm-gui/src/termwindow/render/screen_line.rs:172-189)"
depends_on: []
third_party_review:
  status: none
  updated: null
sections:
  - id: "01.1"
    title: "VTE Parser: Recognize Mode 5"
    status: not-started
  - id: "01.2"
    title: "TermMode Flag + Handler"
    status: not-started
  - id: "01.3"
    title: "GPU Renderer: Apply Reverse Video"
    status: not-started
  - id: "01.4"
    title: "Tests"
    status: not-started
  - id: "01.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "01.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 01: DECSCNM Reverse Video Rendering

**Status:** Not Started
**Goal:** When DECSCNM (mode 5) is active, the entire screen renders with swapped default fg/bg — white background, dark text. 12 vttest golden images show correct light-background rendering.

**Context:** DECSCNM (`CSI ? 5 h`) is completely unimplemented. The gap exists at ALL 5 stages of the pipeline: parser doesn't recognize mode 5, no TermMode flag exists, no handler case, no renderer logic. vttest menu 2 screens 02_03, 02_04, and 02_14 all say "light background" but render with a dark background — frozen as "correct" golden images.

**Reference implementations:**
- **WezTerm** `term/src/terminalstate/mod.rs:274`: Stores `reverse_video_mode: bool` in terminal state. `wezterm-gui/src/termwindow/render/screen_line.rs:172-189`: When `params.dims.reverse_video` is true, fills line bg with fg color.

**Depends on:** None.

---

## 01.1 VTE Parser: Recognize Mode 5

**File(s):** `crates/vte/src/ansi/types.rs`

The vendored VTE crate's `PrivateMode::new()` (lines 176-203) maps CSI mode numbers to `NamedPrivateMode` variants. Mode 5 is missing — it falls through to `PrivateMode::Unknown(5)`.

- [ ] Add `ReverseVideo = 5` variant to the `NamedPrivateMode` enum in `crates/vte/src/ansi/types.rs`
- [ ] Add `5 => Self::Named(NamedPrivateMode::ReverseVideo)` case in `PrivateMode::new()`
- [ ] Verify with `cargo build -p vte`

---

## 01.2 TermMode Flag + Handler

**File(s):** `oriterm_core/src/term/mode/mod.rs`, `oriterm_core/src/term/handler/modes.rs`

Add the mode flag and wire the handler.

- [ ] Add `const REVERSE_VIDEO = 1 << {next_available_bit}` to `TermMode` bitflags in `oriterm_core/src/term/mode/mod.rs`
- [ ] Add case arm in `apply_decset()` at `handler/modes.rs`: `NamedPrivateMode::ReverseVideo => self.mode.insert(TermMode::REVERSE_VIDEO)`
- [ ] Add case arm in `apply_decrst()`: `NamedPrivateMode::ReverseVideo => self.mode.remove(TermMode::REVERSE_VIDEO)`
- [ ] Unit test: set mode 5 via handler, verify `TermMode::REVERSE_VIDEO` is set; reset, verify cleared
- [ ] `./build-all.sh` and `./clippy-all.sh` green

---

## 01.3 GPU Renderer: Apply Reverse Video

**File(s):** `oriterm/src/gpu/extract/from_snapshot/mod.rs`, `oriterm/src/gpu/prepare/mod.rs`, `oriterm/src/gpu/frame_input/mod.rs`

The renderer must swap default fg/bg when REVERSE_VIDEO is active. Two changes needed:

**A. Palette swap in extraction:**

`snapshot_palette()` at `extract/from_snapshot/mod.rs` builds `FramePalette` from the palette array. When `RenderableContent.mode` contains `REVERSE_VIDEO`, swap `background` and `foreground`.

- [ ] Add `reverse_video: bool` field to `FrameInput` (after `window_focused`). Default `false`. Doc: "Screen-wide reverse video (DECSCNM). When true, default fg/bg are swapped."
- [ ] In `extract_frame_from_snapshot()`: check `content.mode.contains(TermMode::REVERSE_VIDEO)` and set `frame.reverse_video` accordingly
- [ ] In `snapshot_palette()` or the caller: when reverse_video, swap `palette.background` and `palette.foreground`
- [ ] Update `FrameInput::test_grid()` and all construction sites to set `reverse_video: false`

**B. Clear color and cell color resolution:**

- [ ] In `prepare_frame_shaped_into()` / `fill_frame_shaped()`: the clear color `input.palette.background` is already used — after palette swap, this will automatically be the fg color (now "background"). No additional change needed if palette swap is correct.
- [ ] In `resolve_cell_colors()` at `prepare/mod.rs`: cells with default colors should use the (already-swapped) palette colors. Cells with explicit SGR colors keep their explicit colors. Per-cell SGR 7 (INVERSE flag) should still swap on top of the DECSCNM swap — so `DECSCNM + SGR 7 = double swap = normal appearance` for that cell. Verify this logic.
- [ ] `./build-all.sh` and `./clippy-all.sh` green

---

## 01.4 Tests

**File(s):** `oriterm_core/src/term/mode/tests.rs`, `oriterm/src/gpu/visual_regression/`, `oriterm/src/gpu/prepare/tests.rs`

- [ ] Unit test in oriterm_core: parse `\x1b[?5h`, verify REVERSE_VIDEO set; parse `\x1b[?5l`, verify cleared
- [ ] Prepare unit test: `reverse_video_swaps_clear_color` — construct FrameInput with `reverse_video: true`, verify clear color uses foreground
- [ ] Prepare unit test: `reverse_video_plus_sgr7_is_normal` — cell with SGR 7 inverse under DECSCNM should look like normal (double swap)
- [ ] Re-render vttest golden images for 02_03, 02_04, 02_14 at all 3 sizes (9 images) with `ORITERM_UPDATE_GOLDEN=1`
- [ ] **Visually inspect** each re-rendered image by reading the PNG — verify white/light background with dark text
- [ ] `./test-all.sh` green

---

## 01.R Third Party Review Findings

- None.

---

## 01.N Completion Checklist

- [ ] `CSI ? 5 h` enables DECSCNM, `CSI ? 5 l` disables it
- [ ] vttest 02_03 (light bg, 132-col) shows white background at all 3 sizes
- [ ] vttest 02_04 (light bg, 80-col) shows white background at all 3 sizes
- [ ] vttest 02_14 (light bg, SGR rendition) shows white background at all 3 sizes
- [ ] SGR 7 inverse still works correctly on individual cells
- [ ] DECSCNM + SGR 7 double-swap produces normal appearance
- [ ] All 9 re-rendered golden images visually verified
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./test-all.sh` green
- [ ] `/tpr-review` passed

**Exit Criteria:** vttest light-background screens render with a white/light background. Golden images show visually correct reverse video rendering. DECSCNM mode 5 is fully implemented from parser through renderer.
