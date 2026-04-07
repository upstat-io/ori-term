---
section: "07"
title: "GPU Golden Images for Tack Visual Subset"
status: not-started
reviewed: false
goal: "Add GPU golden image tests for a curated subset of tack scenarios where the visual rendering matters: color screen (named colors must render with the right RGB), graphic rendition screen (bold/dim/italic/underline must render with the right pixel patterns), and character set screen (DEC line-drawing chars must render at the right glyphs). Reuse Section 04's ScenarioRunner pattern but plug in the GPU pipeline instead of `grid_text`. Each scenario produces a PNG golden under `oriterm/tests/references/tack_*.png` and asserts pixel-equality against it via `compare_with_reference`."
success_criteria:
  - "`oriterm/src/gpu/visual_regression/tack/` directory exists with `mod.rs` orchestrating GPU tack tests"
  - "`oriterm/src/gpu/visual_regression/tack/mod.rs` is below 500 lines (BLOAT gate per code-hygiene rules)"
  - "`run_tack_scenario_golden(spec, gpu, pipelines, renderer, snapshot_name)` helper takes a `&ScenarioSpec`, drives the same `ScenarioRunner` pipeline as Section 04, then renders the captured PtySession through the GPU and asserts via `compare_with_reference`"
  - "Four tack scenarios produce golden images: color, graphic_rendition, character_sets, modes — gated behind the `gpu-tests` feature like vttest goldens are"
  - "Each golden PNG is committed under `oriterm/tests/references/tack_color_80x24.png`, `tack_graphic_rendition_80x24.png`, `tack_character_sets_80x24.png`, `tack_modes_80x24.png` (and at the larger sizes 97x33, 120x40 for color). Six PNG goldens total."
  - "Tests skip cleanly when GPU adapter is unavailable (`headless_env()` returns `None`), when `tack` is unavailable, OR when `tic` is unavailable"
  - "All pixel comparisons use the existing `compare_with_reference` (with the project's existing `PIXEL_TOLERANCE`)"
  - "`timeout 150 cargo test -p oriterm --features gpu-tests -- tack_golden` passes"
  - "Satisfies mission criterion: 'GPU golden images exist for curated visual tack test subset (color, SGR, character sets)'"
inspired_by:
  - "ori_term vttest GPU goldens (oriterm/src/gpu/visual_regression/vttest/mod.rs:206-294 — frame_input + assert_golden pattern after Section 01 dedup)"
  - "ori_term Section 01 dedup (plans/tack-conformance/section-01-shared-pty-session.md — assert_golden becomes a free function taking &PtySession)"
  - "ori_term Section 04 scenario framework (plans/tack-conformance/section-04-scenario-framework.md — ScenarioRunner pattern plugged into GPU here)"
  - "Alacritty visual regression test patterns (alacritty/extra/alacritty.info compiled, then alacritty's screenshots-for-comparison flow)"
depends_on: ["01", "02", "04", "05"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "07.1"
    title: "Bridge ScenarioRunner into the GPU pipeline"
    status: not-started
  - id: "07.2"
    title: "tack_color golden (size matrix)"
    status: not-started
  - id: "07.3"
    title: "tack_graphic_rendition golden"
    status: not-started
  - id: "07.4"
    title: "tack_character_sets golden"
    status: not-started
  - id: "07.5"
    title: "Determinism + tolerance verification"
    status: not-started
  - id: "07.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "07.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 07: GPU Golden Images for Tack Visual Subset

**Status:** Not Started
**Goal:** Add GPU golden image regression tests for tack scenarios where the rendered pixels matter — color (named-color rows must produce the right RGB), graphic rendition (SGR styles must produce the right pixel patterns: bold strokes, italic slant, underline pixels at the right baseline), and character sets (DEC line-drawing chars must produce the right glyphs at the right cell offsets). The tests reuse the Section 04 `ScenarioRunner` pipeline (spawn tack with pinned terminfo, navigate, capture) but plug in the GPU rendering pipeline at the end instead of just calling `grid_text`. Pixel comparison uses the existing `compare_with_reference` from `oriterm/src/gpu/visual_regression/`.

**Success Criteria:**

- [ ] `oriterm/src/gpu/visual_regression/tack/` exists
- [ ] `oriterm/src/gpu/visual_regression/tack/mod.rs` is <500 lines
- [ ] `run_tack_scenario_golden(...)` helper exists and is the single canonical entry point for GPU tack tests
- [ ] Four scenarios snapshotted: tack_color (3 sizes), tack_graphic_rendition (80x24), tack_character_sets (80x24), tack_modes (80x24)
- [ ] Six PNG goldens total committed under `oriterm/tests/references/tack_*.png`:
  - `tack_color_80x24.png`, `tack_color_97x33.png`, `tack_color_120x40.png`
  - `tack_graphic_rendition_80x24.png`
  - `tack_character_sets_80x24.png`
  - `tack_modes_80x24.png`
- [ ] Tests gated behind `gpu-tests` feature (matches vttest convention)
- [ ] Tests skip cleanly when GPU adapter, tack, or tic is unavailable
- [ ] `timeout 150 cargo test -p oriterm --features gpu-tests -- tack_golden` green
- [ ] Satisfies mission criterion #10

**Context:** Sections 05-06 cover tack's text grid (insta snapshots of `grid_text`). Text snapshots catch most regressions — wrong characters, missing labels, wrong screen wording. They DO NOT catch rendering bugs: a color regression where `red` becomes `dim red`, an italic regression where the slant has the wrong angle, an underline regression where the underline pixels are at the wrong baseline. Those bugs only show up when you compare PIXELS. The GPU goldens close that gap.

We don't need to GPU-test every tack scenario — only the visual ones. Section 05/06 already cover ~25 scenarios via text snapshots; Section 07 adds 6 GPU goldens on top (4 scenarios, one of them across the 80x24/97x33/120x40 size matrix), focused on color, SGR styling, DEC graphic chars, and the modes screen's SGR-styled cap labels. This is the same balance Alacritty strikes (extensive text-based ref tests, focused visual tests for the visual subset).

**Reference implementations:**
- **ori_term vttest GPU** `oriterm/src/gpu/visual_regression/vttest/mod.rs:206-294`: the existing `frame_input` + `assert_golden` pattern. After Section 01's dedup, `assert_golden` is a free function taking `&PtySession`. Section 07 calls into the same `assert_golden` but for tack scenarios.
- **Section 01** `plans/tack-conformance/section-01-shared-pty-session.md`: defines `assert_golden(session: &PtySession, name, gpu, pipelines, renderer)` as the canonical GPU bridge. Section 07 consumes that exact API.
- **Section 04** `plans/tack-conformance/section-04-scenario-framework.md`: `ScenarioRunner::run_at(spec, cols, rows)` returns `ScenarioOutcome` for text scenarios. Section 07 needs a parallel runner that returns the LIVE `PtySession` (not just text) so the GPU can render it.
- **`oriterm/src/gpu/visual_regression/headless_env`**: produces `(GpuState, GpuPipelines, WindowRenderer)` for headless GPU tests. Used unchanged here.
- **`oriterm/src/gpu/visual_regression/compare_with_reference`**: pixel comparison helper with PIXEL_TOLERANCE. Used unchanged here.

**Depends on:** Section 01 (shared PtySession + the new free-function `assert_golden`), Section 02 (TerminfoEnv), Section 04 (ScenarioRunner framework), Section 05 (text scenarios — Section 07 mirrors a subset of those).

---

## 07.1 Bridge ScenarioRunner into the GPU pipeline

**File(s):** `oriterm/src/gpu/visual_regression/tack/mod.rs` (NEW)

Section 04 already defined `ScenarioRunner::run_with_session_at(spec, cols, rows) -> LiveSession` and the `LiveSession` wrapper (which holds the live `PtySession`, the parsed `ScreenFacts`, AND the `TerminfoEnv` so it outlives the session). This subsection plugs that API into the GPU pipeline.

The framework already lives in `crates/oriterm_test_support/src/tack_framework/` (Section 04), so cross-crate visibility is solved — `oriterm/src/gpu/visual_regression/tack/` can `use oriterm_test_support::tack_framework::*` directly.

- [ ] Add `oriterm_test_support` to `oriterm/Cargo.toml` `[dev-dependencies]` if it's not already there from Section 01. It is — Section 01.4 added it during the GPU vttest migration. Confirm with `grep oriterm_test_support oriterm/Cargo.toml`.

- [ ] Create `oriterm/src/gpu/visual_regression/tack/mod.rs`:
  ```rust
  //! GPU golden image tests for tack scenarios.
  //!
  //! Spawns tack against ori_term's pinned terminfo (via Section 02
  //! `TerminfoEnv`), navigates to the target screen using Section 04's
  //! `ScenarioRunner::run_with_session_at`, then renders the live
  //! PtySession's `Term` through the GPU pipeline and compares the
  //! resulting framebuffer against a committed PNG golden under
  //! `oriterm/tests/references/tack_*.png`.
  //!
  //! Gated behind the `gpu-tests` Cargo feature (matches the existing
  //! vttest goldens convention).
  //!
  //! Skip cases (returns Ok with eprintln message):
  //!   - GPU adapter unavailable (no compatible wgpu backend)
  //!   - tack not installed
  //!   - tic not installed (TerminfoEnv::compile would panic)

  use oriterm_test_support::tack_framework::{LiveSession, ScenarioRunner, ScenarioSpec};
  use oriterm_test_support::{tack_available, tic_available};

  use super::{compare_with_reference, headless_env};
  use crate::font::CellMetrics;
  use crate::gpu::frame_input::{FrameInput, FramePalette, ViewportSize};
  use crate::gpu::pipelines::GpuPipelines;
  use crate::gpu::state::GpuState;
  use crate::gpu::window_renderer::WindowRenderer;
  ```

- [ ] Define the GPU bridge helper in `oriterm/src/gpu/visual_regression/tack/mod.rs`:
  ```rust
  /// Run a tack scenario through the GPU pipeline and assert the
  /// rendered framebuffer matches a committed PNG golden.
  ///
  /// The flow:
  ///   1. ScenarioRunner spawns tack and navigates to spec.ready_anchor
  ///   2. We pull the live PtySession from the LiveSession wrapper
  ///   3. Build a FrameInput from session.term().renderable_content()
  ///   4. render_to_pixels(...) produces a framebuffer
  ///   5. compare_with_reference(name, &pixels, w, h) does the diff
  ///   6. Drop the LiveSession wrapper — Drop reaps tack and the temp
  ///      terminfo dir
  pub(super) fn run_tack_scenario_golden(
      spec: &ScenarioSpec,
      cols: u16,
      rows: u16,
      gpu: &GpuState,
      pipelines: &GpuPipelines,
      renderer: &mut WindowRenderer,
      golden_name: &str,
  ) {
      let live = ScenarioRunner::run_with_session_at(spec, cols, rows);
      let cell = renderer.cell_metrics();
      let input = build_frame_input(&live.session, cell);
      let w = input.viewport.width;
      let h = input.viewport.height;
      let pixels = super::render_to_pixels(gpu, pipelines, renderer, &input);
      if let Err(msg) = compare_with_reference(golden_name, &pixels, w, h) {
          panic!("tack visual regression ({golden_name}): {msg}");
      }
      // live drops here — temp terminfo cleaned up + tack child reaped
  }

  /// Build a FrameInput from a live PtySession running tack.
  ///
  /// Mirrors the post-Section-01 vttest `frame_input` helper exactly
  /// (same fg/bg, same FramePalette, same viewport sizing). Kept as a
  /// separate function so future extensions (theme variations, blink
  /// frames) can wrap it.
  fn build_frame_input(
      session: &oriterm_test_support::PtySession,
      cell: CellMetrics,
  ) -> FrameInput {
      use oriterm_core::{Rgb, TermMode};

      let cols = session.cols() as usize;
      let rows = session.rows() as usize;
      let w = (cell.width * cols as f32).ceil() as u32;
      let h = (cell.height * rows as f32).ceil() as u32;

      let content = session.term().renderable_content();

      // Same fg/bg constants as the existing vttest goldens
      // (oriterm/src/gpu/visual_regression/vttest/mod.rs:214-247).
      // Keeping them identical ensures tack and vttest goldens have
      // matching baseline colors — diffs reflect tack vs. vttest
      // differences, not palette differences.
      let fg = Rgb { r: 211, g: 215, b: 207 };
      let palette_bg = Rgb { r: 1, g: 1, b: 1 };
      let reverse_video = content.mode.contains(TermMode::REVERSE_VIDEO);
      let (frame_fg, frame_bg) = if reverse_video {
          (palette_bg, fg)
      } else {
          (fg, palette_bg)
      };
      let palette = FramePalette {
          background: frame_bg,
          foreground: frame_fg,
          cursor_color: Rgb { r: 255, g: 255, b: 255 },
          opacity: 1.0,
          selection_fg: None,
          selection_bg: None,
      };
      FrameInput {
          content,
          viewport: ViewportSize::new(w, h),
          cell_size: cell,
          content_cols: cols,
          content_rows: rows,
          palette,
          selection: None,
          search: None,
          hovered_cell: None,
          hovered_url_segments: Vec::new(),
          mark_cursor: None,
          window_focused: true,
          reverse_video,
          fg_dim: 1.0,
          text_blink_opacity: 1.0,
          subpixel_positioning: true,
          prompt_marker_rows: Vec::new(),
      }
  }

  /// Top-level skip gate: returns true iff every dependency
  /// (tack, tic, GPU adapter) is available.
  fn tack_gpu_available() -> bool {
      tack_available() && tic_available()
  }
  ```

  Note: `headless_env()` is called per-test (not in this helper) because the GPU adapter setup is expensive and Rust test functions don't share state.

- [ ] Wire `mod tack;` into `oriterm/src/gpu/visual_regression/mod.rs` as a sibling of the existing `mod vttest;` declaration. **No additional `#[cfg]` gating needed**: the `visual_regression` module itself is declared in `oriterm/src/gpu/mod.rs:79-80` under `#[cfg(all(test, feature = "gpu-tests"))]`, so everything under it — including the new `tack/` submodule — is already only compiled under `cargo test --features gpu-tests`. This is exactly how `vttest/` is wired (plain `mod vttest;` in `visual_regression/mod.rs:34`, no per-submodule cfg). The dev-dependency on `oriterm_test_support` also works because the entire tree is `cfg(test)`.

---

## 07.2 tack_color golden (size matrix)

**File(s):** `oriterm/src/gpu/visual_regression/tack/mod.rs` (test functions)

The color screen is the highest-value GPU test: it directly validates `setaf`/`setab` rendering, named colors, and the 256-color block. Run at three sizes to match the Section 05 size matrix.

- [ ] Add to `tack/mod.rs`:
  ```rust
  // The const ScenarioSpec for the color screen lives in
  // oriterm_test_support::tack_framework::scenarios::color (defined
  // in Section 05). Both this GPU test and the text test in
  // oriterm_core/tests/tack/test_menu/color.rs reference the SAME
  // const — single source of truth for "how do you reach the color
  // screen and what does the parser extract".
  use oriterm_test_support::tack_framework::scenarios::color::TACK_COLOR;

  fn run_tack_color_golden(cols: u16, rows: u16, golden_name: &str) {
      if !tack_gpu_available() { return; }
      let Some((gpu, pipelines, mut renderer)) = headless_env() else {
          eprintln!("skipped: no GPU adapter available");
          return;
      };
      run_tack_scenario_golden(
          &TACK_COLOR, cols, rows,
          &gpu, &pipelines, &mut renderer, golden_name,
      );
  }

  #[test]
  fn tack_golden_color_80x24() {
      run_tack_color_golden(80, 24, "tack_color_80x24");
  }

  #[test]
  fn tack_golden_color_97x33() {
      run_tack_color_golden(97, 33, "tack_color_97x33");
  }

  #[test]
  fn tack_golden_color_120x40() {
      run_tack_color_golden(120, 40, "tack_color_120x40");
  }
  ```

  **Catalog placement (from Section 05):** all const ScenarioSpec values live in `oriterm_test_support::tack_framework::scenarios::*` (one submodule per screen family). Both text tests and GPU tests reference the same const — no duplication.

- [ ] Generate the golden. The project's canonical golden-update env var is `ORITERM_UPDATE_GOLDEN=1` (see `oriterm/src/gpu/visual_regression/mod.rs:11` — `ORITERM_UPDATE_GOLDEN=1: overwrites references with current output`):
  ```
  cd /home/eric/projects/ori_term
  ORITERM_UPDATE_GOLDEN=1 timeout 150 cargo test -p oriterm --features gpu-tests -- tack_golden_color_80x24
  ```


- [ ] Repeat for 97x33 and 120x40 sizes.

- [ ] Verify the goldens are committed under `oriterm/tests/references/`:
  - `tack_color_80x24.png`
  - `tack_color_97x33.png`
  - `tack_color_120x40.png`

- [ ] Re-run all 3 tack_color_* tests (without the generation env var). All must PASS — pixel-equality holds.

---

## 07.3 tack_graphic_rendition golden

**File(s):** `oriterm/src/gpu/visual_regression/tack/mod.rs` (test functions)

The graphic rendition screen is the second-highest GPU test value: bold/dim/italic/underline/reverse/blink labels are drawn IN their respective styles. Visual regressions in glyph shaping, italic skew, or underline pixel placement only show up here.

- [ ] Add to `tack/mod.rs`:
  ```rust
  use oriterm_test_support::tack_framework::scenarios::graphic_rendition::TACK_GRAPHIC_RENDITION_SGR;

  #[test]
  fn tack_golden_graphic_rendition_80x24() {
      if !tack_gpu_available() { return; }
      let Some((gpu, pipelines, mut renderer)) = headless_env() else {
          eprintln!("skipped: no GPU adapter available");
          return;
      };
      run_tack_scenario_golden(
          &TACK_GRAPHIC_RENDITION_SGR, 80, 24,
          &gpu, &pipelines, &mut renderer,
          "tack_graphic_rendition_80x24",
      );
  }
  ```

- [ ] Generate the golden, commit under `oriterm/tests/references/tack_graphic_rendition_80x24.png`. Run the test — must PASS.

---

## 07.4 tack_character_sets golden

**File(s):** `oriterm/src/gpu/visual_regression/tack/mod.rs` (test functions)

DEC line-drawing characters are a GPU-test specialty: text snapshots can verify the chars are present but can't verify they render at the right pixel positions (line-drawing chars must connect cell-to-cell or borders look broken).

- [ ] Add to `tack/mod.rs`:
  ```rust
  use oriterm_test_support::tack_framework::scenarios::character_sets::TACK_TOOLS_G0_DEC_GRAPHICS;

  #[test]
  fn tack_golden_character_sets_80x24() {
      if !tack_gpu_available() { return; }
      let Some((gpu, pipelines, mut renderer)) = headless_env() else {
          eprintln!("skipped: no GPU adapter available");
          return;
      };
      run_tack_scenario_golden(
          &TACK_TOOLS_G0_DEC_GRAPHICS, 80, 24,
          &gpu, &pipelines, &mut renderer,
          "tack_character_sets_80x24",
      );
  }
  ```

- [ ] Generate the golden, commit, verify pass.

- [ ] **TPR checkpoint** — `/tpr-review` covering 07.1–07.4 (the GPU bridge and all five new goldens). Catches: TerminfoEnv lifetime bugs (the LiveSession `_terminfo` field), wrong palette in `build_frame_input`, missing `gpu-tests` feature gate, golden generation that wasn't actually committed, scenario const path drift between text and GPU consumers.

---

## 07.4b tack_modes golden (SGR styling on cap labels)

**File(s):** `oriterm/src/gpu/visual_regression/tack/mod.rs` (test function)

The modes screen in tack draws each supported capability LABEL in a styled cell (bold/reverse/underlined depending on state), then places the cap name next to it. The text snapshot in Section 05 catches missing labels; only a pixel comparison catches cases where the bold label renders without its bold-weight glyphs, or where the reverse-video swatch has the wrong background alignment. The modes screen is visually dense and is a strong candidate for GPU coverage.

- [ ] Add to `tack/mod.rs`:
  ```rust
  use oriterm_test_support::tack_framework::scenarios::modes::TACK_MODES_AM;

  #[test]
  fn tack_golden_modes_80x24() {
      if !tack_gpu_available() { return; }
      let Some((gpu, pipelines, mut renderer)) = headless_env() else {
          eprintln!("skipped: no GPU adapter available");
          return;
      };
      // Reuse TACK_MODES_AM — it navigates to the same modes screen
      // the Section 05 text scenarios snapshot. The golden name is
      // distinct from the text snapshot's (`tack_modes_am` insta vs.
      // `tack_modes_80x24` PNG), so there's no collision.
      run_tack_scenario_golden(
          &TACK_MODES_AM, 80, 24,
          &gpu, &pipelines, &mut renderer,
          "tack_modes_80x24",
      );
  }
  ```

- [ ] Generate the golden, commit `oriterm/tests/references/tack_modes_80x24.png`, verify pass.

---

## 07.5 Determinism + tolerance verification

**File(s):** None (verification only)

GPU tests are subject to subtle non-determinism: different GPU adapters produce slightly different anti-aliased pixels, font hinting can vary frame-to-frame, the existing vttest goldens cope via `PIXEL_TOLERANCE`. Verify the new tack goldens behave the same way.

- [ ] Run all 6 tack_golden_* tests 10 times in a row. All must pass:
  ```
  for i in $(seq 1 10); do
      timeout 150 cargo test -p oriterm --features gpu-tests -- tack_golden || break
  done
  ```

- [ ] Run with `--test-threads=1` (serial) and `--test-threads=4` (parallel). Both must pass.

- [ ] Check the project's `PIXEL_TOLERANCE` constant in `oriterm/src/gpu/visual_regression/mod.rs`. The tack goldens should pass at the SAME tolerance as the vttest goldens — no special tolerance for tack. If they require a higher tolerance to be deterministic, that's a real bug in the rendering pipeline (file via `/add-bug` and treat as blocker).

- [ ] Check golden file sizes: each tack PNG should be small (a few KB) — much smaller than the vttest goldens which capture more complex screens. If a tack golden is mysteriously large (>100KB), inspect it visually for stray content (e.g., partial alt-screen exit sequences leaking into the captured frame).

- [ ] Cross-platform note: GPU tests require a working GPU adapter. On WSL2 Linux with the user's setup (per memory: "GPU works in WSL"), the tests run. On macOS the tests run via Metal. On Windows the tests run via DirectX (or skip cleanly if the adapter is unavailable in CI).

---

## 07.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- None.

---

## 07.N Completion Checklist

- [ ] `oriterm/src/gpu/visual_regression/tack/mod.rs` exists, <500 lines
- [ ] `LiveSession` wrapper holds the `TerminfoEnv` to keep it alive for tack's lifetime
- [ ] `run_tack_scenario_golden(...)` is the single canonical entry point
- [ ] `build_frame_input(...)` mirrors the post-Section-01 vttest helper exactly (same palette, same constants)
- [ ] `mod tack;` added to `oriterm/src/gpu/visual_regression/mod.rs` as a plain sibling of `mod vttest;` — NO per-submodule `#[cfg]`. The parent `visual_regression` module itself is gated at `oriterm/src/gpu/mod.rs:79` under `#[cfg(all(test, feature = "gpu-tests"))]`, so the new submodule inherits that gate automatically. This matches `mod vttest;` on line 34 exactly.
- [ ] 6 PNG goldens committed under `oriterm/tests/references/`:
  - tack_color_80x24.png
  - tack_color_97x33.png
  - tack_color_120x40.png
  - tack_graphic_rendition_80x24.png
  - tack_character_sets_80x24.png
  - tack_modes_80x24.png
- [ ] All 6 tack_golden_* tests pass: `timeout 150 cargo test -p oriterm --features gpu-tests -- tack_golden`
- [ ] 10 reruns clean (determinism)
- [ ] `--test-threads=1` and `--test-threads=4` both pass
- [ ] PIXEL_TOLERANCE unchanged (tack goldens use the same tolerance as vttest goldens)
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `timeout 150 ./test-all.sh` green
- [ ] Plan annotation cleanup
- [ ] All TPR checkpoint findings resolved (see `07.R`)
- [ ] **Plan sync**:
  - [ ] Section frontmatter `status` → `complete`
  - [ ] `00-overview.md` Quick Reference table updated
  - [ ] `00-overview.md` Mission Success Criteria #10 ticked
  - [ ] `index.md` Section 07 updated
  - [ ] Section 04, 05, 06 still passing — no framework changes needed since they already reference `oriterm_test_support::tack_framework::*` from the start
- [ ] `/tpr-review` final pass clean
- [ ] `/impl-hygiene-review last commit` final pass clean (after TPR)

**Exit Criteria:** `timeout 150 cargo test -p oriterm --features gpu-tests -- tack_golden` runs all 6 tack GPU goldens (3 color sizes + 1 graphic rendition + 1 character sets + 1 modes) deterministically. Pixel comparison passes at the existing PIXEL_TOLERANCE. Goldens are committed under `oriterm/tests/references/tack_*.png`. The text scenarios from Sections 04/05/06 still pass — they reference the same const ScenarioSpec values via `oriterm_test_support::tack_framework::scenarios::*`. Section 07 closes the visual regression gap for tack scenarios.
