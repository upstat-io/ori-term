---
section: "01"
title: "DECSCNM Reverse Video Rendering"
status: in-progress
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
    status: complete
  - id: "01.2"
    title: "TermMode Flag + Handler"
    status: complete
  - id: "01.3"
    title: "GPU Renderer: Apply Reverse Video"
    status: complete
  - id: "01.4"
    title: "Tests"
    status: complete
  - id: "01.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "01.N"
    title: "Completion Checklist"
    status: in-progress
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

- [x] Add `ReverseVideo = 5` variant to the `NamedPrivateMode` enum in `crates/vte/src/ansi/types.rs`
- [x] Add `5 => Self::Named(NamedPrivateMode::ReverseVideo)` case in `PrivateMode::new()`
- [x] Verify with `cargo build -p vte`

---

## 01.2 TermMode Flag + Handler

**File(s):** `oriterm_core/src/term/mode/mod.rs`, `oriterm_core/src/term/handler/modes.rs`

Add the mode flag and wire the handler.

- [x] Add `const REVERSE_VIDEO = 1 << {next_available_bit}` to `TermMode` bitflags in `oriterm_core/src/term/mode/mod.rs`
- [x] Add case arm in `apply_decset()` at `handler/modes.rs`: `NamedPrivateMode::ReverseVideo => self.mode.insert(TermMode::REVERSE_VIDEO)`
- [x] Add case arm in `apply_decrst()`: `NamedPrivateMode::ReverseVideo => self.mode.remove(TermMode::REVERSE_VIDEO)`
- [x] Unit test: set mode 5 via handler, verify `TermMode::REVERSE_VIDEO` is set; reset, verify cleared
- [x] `./build-all.sh` and `./clippy-all.sh` green

---

## 01.3 GPU Renderer: Apply Reverse Video

**File(s):** `oriterm/src/gpu/extract/from_snapshot/mod.rs`, `oriterm/src/gpu/prepare/mod.rs`, `oriterm/src/gpu/frame_input/mod.rs`

The renderer must swap default fg/bg when REVERSE_VIDEO is active. Two changes needed:

**A. Palette swap in extraction:**

`snapshot_palette()` at `extract/from_snapshot/mod.rs` builds `FramePalette` from the palette array. When `RenderableContent.mode` contains `REVERSE_VIDEO`, swap `background` and `foreground`.

- [x] Add `reverse_video: bool` field to `FrameInput` (after `window_focused`). Default `false`. Doc: "Screen-wide reverse video (DECSCNM). When true, default fg/bg are swapped."
- [x] In `extract_frame_from_snapshot()`: check `content.mode.contains(TermMode::REVERSE_VIDEO)` and set `frame.reverse_video` accordingly
- [x] In `renderable_content_into()`: when REVERSE_VIDEO, clone palette with `swap_fg_bg()` and use for cell resolution + palette snapshot (architecturally cleaner than GPU-side swap — resolves at the canonical color resolution point)
- [x] Update `FrameInput::test_grid()` and all construction sites to set `reverse_video: false`

**B. Clear color and cell color resolution:**

- [x] Clear color uses `input.palette.background` which is already swapped via palette snapshot — verified correct.
- [x] Cell colors resolved at `renderable_content_into()` with swapped palette. SGR 7 XOR: `apply_inverse()` on already-swapped palette colors produces double swap = normal. Verified by `decscnm_plus_sgr7_is_normal` test.
- [x] `./build-all.sh` and `./clippy-all.sh` green

---

## 01.4 Tests

**File(s):** `oriterm_core/src/term/mode/tests.rs`, `oriterm/src/gpu/visual_regression/`, `oriterm/src/gpu/prepare/tests.rs`

- [x] Unit test in oriterm_core: parse `\x1b[?5h`, verify REVERSE_VIDEO set; parse `\x1b[?5l`, verify cleared
- [x] Prepare unit test: `reverse_video_clear_color_uses_swapped_bg` — construct FrameInput with `reverse_video: true`, verify clear color uses foreground
- [x] Renderable unit test: `decscnm_plus_sgr7_is_normal` — cell with SGR 7 inverse under DECSCNM should look like normal (double swap)
- [x] Renderable unit test: `decscnm_swaps_default_cell_colors` — default cells have swapped fg/bg
- [x] Renderable unit test: `decscnm_palette_snapshot_has_swapped_entries` — palette snapshot entries are swapped
- [x] Re-render vttest golden images for 02_03, 02_04, 02_14 at all 3 sizes (9+ images, full menu2) with `ORITERM_UPDATE_GOLDEN=1`
- [x] **Visually inspected** each re-rendered image — white/light background with dark text confirmed
- [x] `./test-all.sh` green

---

## 01.R Third Party Review Findings

- None.

---

## 01.N Completion Checklist

- [x] `CSI ? 5 h` enables DECSCNM, `CSI ? 5 l` disables it
- [x] vttest 02_03 (light bg, 132-col) shows white background at all 3 sizes
- [x] vttest 02_04 (light bg, 80-col) shows white background at all 3 sizes
- [x] vttest 02_14 (light bg, SGR rendition) shows white background at all 3 sizes
- [x] SGR 7 inverse still works correctly on individual cells
- [x] DECSCNM + SGR 7 double-swap produces normal appearance
- [x] All 9 re-rendered golden images visually verified
- [x] `./build-all.sh` green
- [x] `./clippy-all.sh` green
- [x] `./test-all.sh` green
- [ ] `/tpr-review` passed

**Exit Criteria:** vttest light-background screens render with a white/light background. Golden images show visually correct reverse video rendering. DECSCNM mode 5 is fully implemented from parser through renderer.
