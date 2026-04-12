---
section: "07"
title: "GPU Golden Images for Tack Visual Subset"
status: complete
reviewed: true
goal: "Add GPU golden image tests for a curated subset of tack scenarios where the visual rendering matters: color screen (named colors must render with the right RGB), graphic rendition screen (bold/dim/italic/underline must render with the right pixel patterns), and character set screen (DEC line-drawing chars must render at the right glyphs). Reuse Section 04's ScenarioRunner pattern but plug in the GPU pipeline instead of `grid_text`. Each scenario produces a PNG golden under `oriterm/tests/references/tack_*.png` and asserts pixel-equality against it via `compare_with_reference`."
success_criteria:
  - "`oriterm/src/gpu/visual_regression/frame_input_helper.rs` exists and is the single canonical `PtySession -> FrameInput` builder consumed by BOTH vttest (`vttest/render.rs::frame_input`) AND tack (`tack/mod.rs`). The duplicated body in `vttest/render.rs` is deleted — the vttest side now calls the shared helper. No `frame_input` copies remain."
  - "`oriterm/src/gpu/visual_regression/tack/` directory exists with `mod.rs` orchestrating GPU tack tests"
  - "`oriterm/src/gpu/visual_regression/tack/mod.rs` is below 500 lines (BLOAT gate per code-hygiene rules)"
  - "`run_tack_scenario_golden(spec, cols, rows)` helper takes a `&ScenarioSpec` + grid size, drives the same `ScenarioRunner` pipeline as Section 04, acquires `headless_env()` internally, renders the captured PtySession through the GPU, asserts via `compare_with_reference`, AND calls `live.finish()` before returning so the M5 cleanup contract is honored. No `gpu`/`pipelines`/`renderer` parameters — the bridge owns acquisition so per-test wrappers stay one-liners."
  - "The bridge owns the SINGLE consolidated skip gate (tack + tic + `headless_env`). Per-test functions do NOT scatter `headless_env()` probes or call any `tack_gpu_available()` helper — they delegate to `run_tack_scenario_golden`. This fixes the side-logic LEAK that would scatter the GPU-adapter skip check across 6 test functions."
  - "The golden file name comes from `live.golden_name()` (the `LiveSession` method defined in Section 04 that delegates to the same `'<screen_id>_<cols>x<rows>'` format literal as `ScenarioOutcome::golden_name()`) — NOT a hand-passed string parameter and NOT a rebuilt `format!` at the call site. Single source of truth for naming lives in Section 04's `LiveSession::golden_name()` / `ScenarioOutcome::golden_name()` pair"
  - "Four tack scenarios produce golden images: color, graphic_rendition, character_sets, modes — gated behind the `gpu-tests` feature like vttest goldens are"
  - "Each golden PNG is committed under `oriterm/tests/references/tack_color_80x24.png`, `tack_graphic_rendition_sgr_80x24.png`, `tack_character_sets_80x24.png`, `tack_modes_80x24.png` (and at the larger sizes 97x33, 120x40 for color). Six PNG goldens total."
  - "Tests skip cleanly when GPU adapter is unavailable (`headless_env()` returns `None`), when `tack` is unavailable, OR when `tic` is unavailable"
  - "All pixel comparisons use the existing `compare_with_reference` (with the project's existing `PIXEL_TOLERANCE`)"
  - "`timeout 150 cargo test -p oriterm --features gpu-tests -- tack_golden` passes (with `--test-threads=1` AND `--test-threads=4`)"
  - "`compare_with_reference` panics hard (not `debug_assert!`) when BOTH `CI` and `ORITERM_UPDATE_GOLDEN` are set. A regression test (`compare_with_reference_ci_guard_fires`) in `oriterm/src/gpu/visual_regression/meta_tests.rs` (NOT `tests.rs` — that file does not exist; the existing meta-tests are in `meta_tests.rs`) forces both env vars and asserts the panic fires via `#[should_panic]`. The test is marked `#[ignore]` because `text_blink_tests.rs:100` ALSO reads `ORITERM_UPDATE_GOLDEN` — running the regression test in default `cargo test` mode would leak the env var into `text_blink_tests` and silently regenerate its goldens. Run explicitly via `--ignored`."
  - "Section 07 does NOT add or modify any `.github/workflows/*.yml` file. As of 2026-04-08 there is no `gpu-tests` job in the workflow tree (verified by `rg gpu-tests .github/workflows`). Section 07's CI guard is in-source ONLY. The cross-plan handoff to any future plan that adds GPU CI is encoded as `<!-- blocks: any-future-gpu-ci-plan -->` inside subsection 07.5b."
  - "All 6 PNG goldens were generated on WSL2/Linux using the embedded-font rasterization path — NEVER from macOS Metal or Windows DirectX (cross-adapter AA divergence exceeds `MAX_MISMATCH_PERCENT`)"
  - "Satisfies mission criterion #10: 'GPU golden images exist for curated visual tack test subset: color (3 sizes), graphic rendition, character sets, modes' (direct 1:1 trace — color→TACK_COLOR x3, graphic_rendition→TACK_GRAPHIC_RENDITION_SGR, character_sets→TACK_TOOLS_G0_DEC_GRAPHICS, modes→TACK_MODES_AM)"
inspired_by:
  - "ori_term vttest GPU goldens (oriterm/src/gpu/visual_regression/vttest/mod.rs:206-294 — frame_input + assert_golden pattern after Section 01 dedup)"
  - "ori_term Section 01 dedup (plans/tack-conformance/section-01-shared-pty-session.md — assert_golden becomes a free function taking &PtySession)"
  - "ori_term Section 04 scenario framework (plans/tack-conformance/section-04-scenario-framework.md — ScenarioRunner pattern plugged into GPU here)"
  - "Alacritty visual regression test patterns (alacritty/extra/alacritty.info compiled, then alacritty's screenshots-for-comparison flow)"
depends_on: ["01", "02", "04", "05", "06"]
depends_on_contract:
  - section: "05"
    contract: "scenario const paths (color, graphic_rendition, modes) — Section 07 imports `scenarios::color::TACK_COLOR`, `scenarios::graphic_rendition::TACK_GRAPHIC_RENDITION_SGR`, `scenarios::modes::TACK_MODES_AM`. Section 05 landed these at the pinned paths under `crates/oriterm_test_support/src/tack_framework/scenarios/`. Section 07 has ZERO cap-coverage contribution — the goldens are pixel regression tests, not cap exercisers. The modes GPU golden uses the stable-screen `TACK_MODES_AM` (always-visible `(os)` cap), not a phase-captured per-cap golden, per the 05.5b architectural verdict."
  - section: "06"
    contract: "scenario const path (character_sets) — Section 07 imports `scenarios::character_sets::TACK_TOOLS_G0_DEC_GRAPHICS`. Section 06 landed the const at `crates/oriterm_test_support/src/tack_framework/scenarios/character_sets/mod.rs:180` with the single-word `character_sets` module name (verified 2026-04-08)."
third_party_review:
  status: resolved
  updated: 2026-04-09
sections:
  - id: "07.0"
    title: "Extract shared `frame_input_helper` (dedup vttest)"
    status: complete
  - id: "07.1"
    title: "Bridge ScenarioRunner into the GPU pipeline"
    status: complete
  - id: "07.2"
    title: "tack_color golden (size matrix)"
    status: complete
  - id: "07.3"
    title: "tack_graphic_rendition golden"
    status: complete
  - id: "07.4"
    title: "tack_character_sets golden"
    status: complete
  - id: "07.4b"
    title: "tack_modes golden (SGR styling on cap labels)"
    status: complete
  - id: "07.5a"
    title: "Determinism, tolerance, cross-adapter verification"
    status: complete
  - id: "07.5b"
    title: "CI guard implementation (in-source panic + regression test)"
    status: complete
  - id: "07.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "07.N"
    title: "Completion Checklist"
    status: complete
---


# Section 07: GPU Golden Images for Tack Visual Subset

**Status:** In Progress — `reviewed: true` (multi-agent review pass complete).

**Implementation contract (load-bearing, do NOT regress):**

1. **Never destructure `live` and never move `live.session` out.** `LiveSession` holds `_terminfo: TerminfoEnv` as `pub(super)` — the `TerminfoEnv` temp dir exists so tack's lazy terminfo reads resolve against ori_term's pinned caps. If you move `live.session` into a standalone binding, the `_terminfo` drops early and tack's next terminfo query hits an empty tree. Borrow through `&live.session` only. `live.finish()` is the single consuming move allowed.
2. **`live.finish()` is called AFTER `compare_with_reference` and BEFORE `run_tack_scenario_golden` returns.** This is the M5 cleanup contract from Section 04. Do NOT add a `FinishOnDrop` RAII guard to "make it panic-safe" — `PtySession::Drop` (see `crates/oriterm_test_support/src/session/mod.rs:413`) already kills + reaps the child on the panic path, so the only thing lost on panic is the exit-status assertion (not cleanup). A `Drop`-based guard would introduce double-panic abort risk (Rust's `panic-during-drop` semantics aborts the process). Accept the trade-off: the current `compare → finish → log` order is correct.
3. **Golden name comes from `live.golden_name()` — never rebuild the format literal.** `LiveSession::golden_name()` delegates to the `"<screen_id>_<cols>x<rows>"` SSOT helper in `tack_framework::runner::scenario_name`. Rebuilding `format!("{}_{}x{}", ...)` at the call site is `LEAK:scattered-knowledge`.
4. **`frame_input` has ONE canonical home (Section 07.0).** The existing `vttest/render.rs::frame_input` is algorithmically identical to the tack `build_frame_input` — Section 07.0 extracts the shared helper into `oriterm/src/gpu/visual_regression/frame_input_helper.rs` and migrates vttest to consume it. Having two copies is `LEAK:algorithmic-duplication` (per impl-hygiene.md, 2 cross-file instances already trigger extraction).
5. **`mod tack;` is a plain sibling of `mod vttest;` under `visual_regression/mod.rs`.** No per-submodule `#[cfg]` — the `visual_regression` module itself is gated at `oriterm/src/gpu/mod.rs:79` under `#[cfg(all(test, feature = "gpu-tests"))]` and the child inherits.

**Scenario list (fixed):** color x3 sizes + graphic_rendition + character_sets + modes = 6 PNG goldens under `oriterm/tests/references/tack_*.png`. The modes golden uses `TACK_MODES_AM` (always-visible `(os)` cap on the stable modes screen), NOT a phase-captured per-cap golden — the open architectural decision in Section 05.5b defaulted to stable-screen only, and Section 07 inherits that verdict. **Verified 2026-04-08:** `TACK_MODES_AM` in `crates/oriterm_test_support/src/tack_framework/scenarios/modes/mod.rs:29` is a plain `ScenarioSpec` (not a `PhaseSpec`), with `ready_anchor: "Done"` and `parser: parse_modes_screen` — it captures the final post-run screen where all caps have been reported. There is no `run_phase_with_session_at` GPU bridge, and none is needed. If Section 07 ever accidentally references a `PhaseSpec` const (e.g., `TACK_MODES_PHASE_AM`), the implementation has drifted from the 05.5b verdict and should be reverted.

**Mission trace (direct 1:1 with 00-overview Criterion #10):**
- `color (3 sizes)` → `TACK_COLOR` at 80x24, 97x33, 120x40 (subsection 07.2)
- `graphic rendition` → `TACK_GRAPHIC_RENDITION_SGR` at 80x24 (subsection 07.3)
- `character sets` → `TACK_TOOLS_G0_DEC_GRAPHICS` at 80x24 (subsection 07.4)
- `modes` → `TACK_MODES_AM` at 80x24 (subsection 07.4b)

Every clause in mission criterion #10 has a delivering subsection, and every delivering subsection maps back to one mission clause. If future criterion edits add a fifth visual scenario (e.g., "cursor movement"), Section 07 must grow to match or the mission must be edited — there is no middle ground.

**Goal:** Add GPU golden image regression tests for tack scenarios where the rendered pixels matter — color (named-color rows must produce the right RGB), graphic rendition (SGR styles must produce the right pixel patterns: bold strokes, italic slant, underline pixels at the right baseline), and character sets (DEC line-drawing chars must produce the right glyphs at the right cell offsets). The tests reuse the Section 04 `ScenarioRunner` pipeline (spawn tack with pinned terminfo, navigate, capture) but plug in the GPU rendering pipeline at the end instead of just calling `grid_text`. Pixel comparison uses the existing `compare_with_reference` from `oriterm/src/gpu/visual_regression/`.

**Success Criteria:**

- [x] `oriterm/src/gpu/visual_regression/frame_input_helper.rs` exists — single canonical `PtySession -> FrameInput` builder consumed by BOTH vttest and tack (07.0). The vttest `frame_input` copy in `vttest/render.rs` is DELETED and `vttest/render.rs::frame_input_with_blink` wraps the shared helper.
- [x] `oriterm/src/gpu/visual_regression/tack/` exists
- [x] `oriterm/src/gpu/visual_regression/tack/mod.rs` is <500 lines
- [x] `run_tack_scenario_golden(...)` helper exists and is the single canonical entry point for GPU tack tests
- [x] The single consolidated skip gate (tack + tic + `headless_env()` probe) lives INSIDE `run_tack_scenario_golden`. There is NO `tack_gpu_available()` helper function — the skip-check logic is centralized in the bridge body, not extracted into a separate helper that per-test wrappers might be tempted to call. Per-test skip checks do NOT scatter `headless_env()` calls across 6 tests; they delegate to `run_tack_scenario_golden`.
- [x] Four scenarios snapshotted: tack_color (3 sizes), tack_graphic_rendition (80x24), tack_character_sets (80x24), tack_modes (80x24)
- [x] Six PNG goldens total committed under `oriterm/tests/references/tack_*.png`:
  - `tack_color_80x24.png`, `tack_color_97x33.png`, `tack_color_120x40.png`
  - `tack_graphic_rendition_sgr_80x24.png`
  - `tack_character_sets_80x24.png`
  - `tack_modes_80x24.png`
- [x] Tests gated behind `gpu-tests` feature (matches vttest convention)
- [x] Tests skip cleanly when GPU adapter, tack, or tic is unavailable
- [x] `timeout 150 cargo test -p oriterm --features gpu-tests -- tack_golden` green
- [x] Satisfies mission criterion #10

**Context:** Sections 05-06 cover tack's text grid (insta snapshots of `grid_text`). Text snapshots catch most regressions — wrong characters, missing labels, wrong screen wording. They DO NOT catch rendering bugs: a color regression where `red` becomes `dim red`, an italic regression where the slant has the wrong angle, an underline regression where the underline pixels are at the wrong baseline. Those bugs only show up when you compare PIXELS. The GPU goldens close that gap.

We don't need to GPU-test every tack scenario — only the visual ones. Section 05/06 already cover ~25 scenarios via text snapshots; Section 07 adds 6 GPU goldens on top (4 scenarios, one of them across the 80x24/97x33/120x40 size matrix), focused on color, SGR styling, DEC graphic chars, and the modes screen's SGR-styled cap labels. This is the same balance Alacritty strikes (extensive text-based ref tests, focused visual tests for the visual subset).

**Reference implementations:**
- **ori_term vttest GPU** `oriterm/src/gpu/visual_regression/vttest/mod.rs:206-294`: the existing `frame_input` + `assert_golden` pattern. After Section 01's dedup, `assert_golden` is a free function taking `&PtySession`. Section 07 calls into the same `assert_golden` but for tack scenarios.
- **Section 01** `plans/tack-conformance/section-01-shared-pty-session.md`: defines `assert_golden(session: &PtySession, name, gpu, pipelines, renderer)` as the canonical GPU bridge. Section 07 consumes that exact API.
- **Section 04** `plans/tack-conformance/section-04-scenario-framework.md`: `ScenarioRunner::run_at(spec, cols, rows)` returns `ScenarioOutcome` for text scenarios. Section 07 needs a parallel runner that returns the LIVE `PtySession` (not just text) so the GPU can render it.
- **`oriterm/src/gpu/visual_regression/headless_env`**: produces `(GpuState, GpuPipelines, WindowRenderer)` for headless GPU tests. Used unchanged here.
- **`oriterm/src/gpu/visual_regression/compare_with_reference`**: pixel comparison helper with PIXEL_TOLERANCE. Used unchanged here.

**Depends on:** Section 01 (shared PtySession + the new free-function `assert_golden`), Section 02 (TerminfoEnv), Section 04 (ScenarioRunner framework), Section 05 (text scenarios — Section 07 mirrors a subset of those), Section 06 (character_sets scenario consts).

---

## 07.0 Extract shared `frame_input_helper` (dedup vttest)

**File(s):** `oriterm/src/gpu/visual_regression/frame_input_helper.rs` (NEW); `oriterm/src/gpu/visual_regression/vttest/render.rs` (MODIFIED); `oriterm/src/gpu/visual_regression/mod.rs` (MODIFIED — add `mod frame_input_helper;`).

**Why this subsection exists:** The existing `vttest/render.rs::frame_input(session: &PtySession, cell: CellMetrics) -> FrameInput` (lines 21-83) constructs a `FrameInput` from a live `PtySession` with a specific set of palette/display defaults (fg `(211, 215, 207)`, palette_bg `(1, 1, 1)`, `subpixel_positioning: true`, `fg_dim: 1.0`, etc.). Section 07's `tack/mod.rs` needs the IDENTICAL builder — the two helpers would differ only in which test module imports them. Per `impl-hygiene.md`, **2 cross-file copies of a >5-line algorithm is an immediate extraction trigger**, and the cross-file version is worse because drift is invisible across test suites. Extract once, consume twice.

- [x] Create `oriterm/src/gpu/visual_regression/frame_input_helper.rs` with the canonical builder lifted verbatim from `vttest/render.rs::frame_input`:
  ```rust
  //! Shared PtySession → FrameInput builder for vttest and tack GPU
  //! golden tests.
  //!
  //! Both `vttest/render.rs` and `tack/mod.rs` need to construct a
  //! `FrameInput` from a live `PtySession` running a terminal program
  //! (vttest or tack) under ori_term's pinned terminfo. The construction
  //! is ALGORITHMICALLY IDENTICAL — same palette constants, same
  //! reverse-video handling, same `subpixel_positioning: true`,
  //! same unused selection/search/hover/mark fields. This module is
  //! the single canonical home for that construction; duplicating it
  //! is `LEAK:algorithmic-duplication` and will be caught by
  //! `/impl-hygiene-review`.

  use oriterm_core::{Rgb, TermMode};
  use oriterm_test_support::PtySession;

  use crate::font::CellMetrics;
  use crate::gpu::frame_input::{FrameInput, FramePalette, ViewportSize};

  /// Build a `FrameInput` from a live `PtySession` with the standard
  /// golden-test palette (matching the vttest/tack conventions).
  ///
  /// Visibility is `pub(in crate::gpu::visual_regression)` — both
  /// `vttest/render.rs` and `tack/mod.rs` (siblings under
  /// `visual_regression/`) consume it. The fully-qualified `pub(in ...)`
  /// path documents the intended sharing boundary; bare `pub(super)`
  /// would only restrict to `frame_input_helper`'s parent (which IS
  /// `visual_regression`, so functionally equivalent), but the explicit
  /// path makes the cross-sibling consumption rule visible to readers
  /// and to `/impl-hygiene-review`.
  pub(in crate::gpu::visual_regression) fn frame_input(session: &PtySession, cell: CellMetrics) -> FrameInput {
      let cols = session.cols() as usize;
      let rows = session.rows() as usize;
      let w = (cell.width * cols as f32).ceil() as u32;
      let h = (cell.height * rows as f32).ceil() as u32;

      let content = session.term().renderable_content();

      let fg = Rgb { r: 211, g: 215, b: 207 };
      // Palette bg must differ from the cell bg so the prepare phase
      // emits bg quads (cells have bg=(0,0,0); palette_bg=(1,1,1) is a
      // near-black that forces the bg-quad path).
      let palette_bg = Rgb { r: 1, g: 1, b: 1 };

      let reverse_video = content.mode.contains(TermMode::REVERSE_VIDEO);
      // DECSCNM: cell colors are already resolved against the swapped
      // palette in renderable_content_into(); frame palette fg/bg must
      // also swap so the screen-clear color matches.
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
  ```

- [x] Add `mod frame_input_helper;` to `oriterm/src/gpu/visual_regression/mod.rs` (a new sibling of `mod vttest;` / `mod tack;`).

- [x] Migrate `oriterm/src/gpu/visual_regression/vttest/render.rs` to consume the shared helper:
  - Delete the body of `frame_input(session: &PtySession, cell: CellMetrics) -> FrameInput` (lines 21-83 in the current file).
  - Replace the local `frame_input` definition entirely. The canonical helper has `pub(in crate::gpu::visual_regression)` visibility, so `vttest/render.rs` (a sibling under `visual_regression/`) imports it directly: `use crate::gpu::visual_regression::frame_input_helper::frame_input;`.
  - `assert_golden` (currently calls local `frame_input` on line 106) updates its call to the imported `frame_input` — no signature change needed.
  - `frame_input_with_blink` stays in `vttest/render.rs` as a thin wrapper that forwards to the shared helper:
    ```rust
    use crate::gpu::visual_regression::frame_input_helper::frame_input;

    pub(super) fn frame_input_with_blink(
        session: &PtySession,
        cell: CellMetrics,
        text_blink_opacity: f32,
    ) -> FrameInput {
        let mut input = frame_input(session, cell);
        input.text_blink_opacity = text_blink_opacity;
        input
    }
    ```
  - `vttest/mod.rs` line 20 (`use self::render::{assert_golden, cell_brightness, frame_input_with_blink};`) is unchanged — `frame_input_with_blink` is still re-exported from `render.rs`.

- [x] **No `frame_input_helper/tests.rs` is created.** The helper is a pure builder function with no branching beyond the documented `reverse_video` swap. The semantic pin "vttest goldens stay byte-identical pre/post extraction" already exercises the helper end-to-end via the existing vttest golden test corpus (verified by the next checkbox below). Per `test-organization.md`, sibling `tests.rs` is created when there ARE tests; it is not mandatory for every source file. Since the file is flat (`frame_input_helper.rs`, NOT `frame_input_helper/mod.rs`) and has no helper-specific unit tests, no `tests.rs` is needed.

- [x] **Semantic pin (the failing-test-first gate for the extraction):** the extraction is correct ONLY IF the existing vttest golden corpus produces byte-identical output pre and post-extraction. The vttest goldens ARE the test that proves the extraction is a pure lift — there is no separate unit test for `frame_input_helper::frame_input` because writing one would just be mocking the same builder logic. The pin: `timeout 150 cargo test -p oriterm --features gpu-tests -- vttest_golden` must be green BOTH before and after the 07.0 commit.
  - Before: capture the baseline. Run the command on the pre-07.0 working tree, confirm green.
  - After: run the same command on the 07.0 commit, confirm STILL green AND that `git status oriterm/tests/references/` shows no modifications (the goldens were not regenerated — the extraction produces pixel-identical output).
  - If any vttest golden's status flips to "modified" (i.e., the extraction inadvertently changed pixel output), the extraction has a bug — most likely a struct field swap or a defaults mismatch. Revert and re-extract.

- [x] `./clippy-all.sh` green — no dead-code warnings on the old removed helper.

- [x] Run debug AND release: `timeout 150 cargo test -p oriterm --features gpu-tests --release -- vttest_golden` must also pass. The lifted helper must produce identical output across both profiles.

- [x] **Green-vttest gate (LOAD-BEARING for 07.1 start).** Do not start 07.1 until ALL of the following are green on the extraction commit: (a) `timeout 150 cargo test -p oriterm --features gpu-tests -- vttest_golden` (debug), (b) the same command in `--release` mode, (c) `./build-all.sh`, (d) `./clippy-all.sh`. The tack bridge in 07.1 will re-import the shared helper — if the extraction broke vttest, the tack bridge cannot safely build on top of it. Treat this like a migration gate: commit 07.0 as its own atomic change, verify green, THEN start 07.1.

**Invariant pin:** `oriterm/src/gpu/visual_regression/` contains EXACTLY ONE `frame_input` function. Adding a second copy is a `LEAK:algorithmic-duplication` finding that will be caught by `/impl-hygiene-review last commit` at the end of Section 07.

---

## 07.1 Bridge ScenarioRunner into the GPU pipeline

**File(s):** `oriterm/src/gpu/visual_regression/tack/mod.rs` (NEW)

**Ordering prerequisite:** 07.0 must be LANDED and vttest-green before this subsection starts. The tack bridge imports `super::frame_input_helper::frame_input` — if 07.0 hasn't extracted the helper yet, that import does not exist. Do not interleave 07.0 and 07.1.

Section 04 already defined `ScenarioRunner::run_with_session_at(spec, cols, rows) -> LiveSession` and the `LiveSession` wrapper (which holds the live `PtySession`, the parsed `ScreenFacts`, AND the `TerminfoEnv` so it outlives the session). This subsection plugs that API into the GPU pipeline.

The framework already lives in `crates/oriterm_test_support/src/tack_framework/` (Section 04), so cross-crate visibility is solved — `oriterm/src/gpu/visual_regression/tack/` can `use oriterm_test_support::tack_framework::*` directly.

- [x] Add `oriterm_test_support` to `oriterm/Cargo.toml` `[dev-dependencies]` if it's not already there from Section 01. It is — Section 01.4 added it during the GPU vttest migration. Confirm with `grep oriterm_test_support oriterm/Cargo.toml`.

- [x] Create `oriterm/src/gpu/visual_regression/tack/mod.rs`:
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
  //! Skip cases (prints `skipped: <reason>` via eprintln and returns
  //! without running the test):
  //!   - GPU adapter unavailable (no compatible wgpu backend)
  //!   - tack not installed
  //!   - tic not installed (TerminfoEnv::compile would panic)
  //!
  //! All three skip conditions are consolidated into
  //! `run_tack_scenario_golden` via the closure form below —
  //! per-test functions do NOT scatter `headless_env()` probes.

  use oriterm_test_support::tack_framework::{ScenarioRunner, ScenarioSpec};
  use oriterm_test_support::{tack_available, tic_available};

  use super::frame_input_helper::frame_input;
  use super::{compare_with_reference, headless_env, render_to_pixels};
  ```

- [x] Define the GPU bridge helper with a CONSOLIDATED skip gate. `run_tack_scenario_golden` owns `headless_env()` acquisition so per-test functions never probe the GPU adapter themselves — they just call the bridge with the scenario spec and grid size:
  ```rust
  /// Run a tack scenario through the GPU pipeline and assert the
  /// rendered framebuffer matches a committed PNG golden.
  ///
  /// This function owns the ENTIRE skip gate:
  ///   - `tack_available()` — else `eprintln` + return
  ///   - `tic_available()` — else `eprintln` + return
  ///   - `headless_env()` — else `eprintln` + return
  /// Per-test wrappers MUST NOT call `headless_env()` directly.
  /// Centralizing the gate here eliminates the 6-site scatter that
  /// /tp-help flagged as `LEAK:scattered-knowledge` on the skip logic.
  ///
  /// Flow on the hot path (all gates satisfied):
  ///   1. ScenarioRunner spawns tack, navigates to spec.ready_anchor
  ///   2. Borrow &live.session to build a FrameInput via the shared
  ///      `frame_input_helper::frame_input` (Section 07.0) — the
  ///      `RenderableContent` field is owned, not borrowed, so the
  ///      borrow of `live.session` ends after this call.
  ///   3. render_to_pixels produces a framebuffer.
  ///   4. `live.golden_name()` produces the PNG filename (SSOT helper
  ///      from Section 04 — never rebuild `format!` at this site).
  ///   5. compare_with_reference diffs pixels vs. the committed PNG.
  ///   6. `live.finish()` consumes `live`, quits tack, asserts
  ///      `exit.success()`. This runs AFTER the compare so the visual
  ///      diff is logged before the exit-status assertion.
  ///   7. If the visual diff failed, panic with the golden name for
  ///      debugability.
  ///
  /// **Do NOT wrap `live` in a `FinishOnDrop` RAII guard.** `PtySession`
  /// already has a `Drop` impl (`crates/oriterm_test_support/src/session/mod.rs:413`)
  /// that kills + reaps the child, so the panic path still cleans up FDs;
  /// the only thing lost on panic is the exit-status assertion.
  /// A `Drop`-based `finish` would trigger Rust's panic-during-drop
  /// abort semantics on the (rare) double-failure path.
  pub(super) fn run_tack_scenario_golden(
      spec: &ScenarioSpec,
      cols: u16,
      rows: u16,
  ) {
      if !tack_available() {
          eprintln!("skipped: tack not installed");
          return;
      }
      if !tic_available() {
          eprintln!("skipped: tic not installed");
          return;
      }
      let Some((gpu, pipelines, mut renderer)) = headless_env() else {
          eprintln!("skipped: no GPU adapter available");
          return;
      };

      let live = ScenarioRunner::run_with_session_at(spec, cols, rows);
      let cell = renderer.cell_metrics();

      // Borrow &live.session only — never move live.session out. The
      // _terminfo sibling field in LiveSession is pub(super) and must
      // stay alive until live.finish() runs. `renderable_content()`
      // returns an owned RenderableContent, so the borrow of
      // live.session ends at the end of this statement.
      let input = frame_input(&live.session, cell);
      let w = input.viewport.width;
      let h = input.viewport.height;
      let pixels = render_to_pixels(&gpu, &pipelines, &mut renderer, &input);

      let golden_name = live.golden_name();
      let visual_result = compare_with_reference(&golden_name, &pixels, w, h);

      // finish() runs unconditionally — even on a visual mismatch
      // we want the exit-status assertion to fire so regressions in
      // tack's quit path are caught.
      let exit = live.finish();
      log::info!("tack scenario {} clean exit: {exit:?}", spec.id);

      if let Err(msg) = visual_result {
          panic!("tack visual regression ({golden_name}): {msg}");
      }
  }
  ```

  **Key change vs. the earlier draft:** `run_tack_scenario_golden` no longer takes `gpu`/`pipelines`/`renderer` as parameters — it acquires them internally via `headless_env()`. Per-test functions become one-liners (`run_tack_scenario_golden(&TACK_COLOR, 80, 24)`). This is strictly better than the old "pass the env through 3 layers of test wrappers" design because:
  1. The skip gate is a single function; no 6-site scatter.
  2. The test functions can't accidentally forget to check `headless_env()`.
  3. Parallel test invocations each get their own adapter — matches how vttest tests work (`let Some((gpu, pipelines, mut renderer)) = headless_env() else ...` inside each `#[test]`).
  4. If `headless_env()` ever becomes cacheable (thread-local), the single acquisition site makes that refactor trivial.

- [x] **OSC round-trip anchor warning (DOCUMENTATION ONLY here; verification CHECK lives in 07.2).** Section 06 extended `PtyResponder` with `ColorRequest` / `ClipboardLoad` / `ClipboardStore` handling. The `wait_for` anchor logic in `Section 04` `ScenarioRunner` waits 300ms for the `send()` echo then adds a 200ms quiet period after the anchor text lands. For most tack scenarios that's enough — but `tack_color` in particular may trigger OSC 10/11/4 color queries that round-trip through the responder and arrive AFTER the "stable" anchor. If `tack_color`'s `ready_anchor` matches text that's present BEFORE the OSC responses land, the golden captures a half-updated screen. The actual VERIFY step (render a second pass and diff) is in 07.2 — it requires having generated the golden first, which is a 07.2 task, not a 07.1 one.

- [x] Wire `mod tack;` into `oriterm/src/gpu/visual_regression/mod.rs` as a sibling of the existing `mod vttest;` and the new `mod frame_input_helper;` declarations. **No additional `#[cfg]` gating needed**: the `visual_regression` module itself is declared in `oriterm/src/gpu/mod.rs:79-80` under `#[cfg(all(test, feature = "gpu-tests"))]`, so everything under it — including the new `tack/` submodule — is already only compiled under `cargo test --features gpu-tests`. This is exactly how `vttest/` is wired (plain `mod vttest;` in `visual_regression/mod.rs:34`, no per-submodule cfg). The dev-dependency on `oriterm_test_support` also works because the entire tree is `cfg(test)`.

**Borrow-lifetime sanity check.** The flow `frame_input(&live.session, cell) → render_to_pixels(...) → live.golden_name() → live.finish()` is borrow-checker clean:
- `frame_input` borrows `&live.session` immutably. It calls `session.term().renderable_content()` which returns an OWNED `RenderableContent` (see `oriterm_core/src/term/snapshot.rs:33` — no lifetime parameter on the return type). The borrow of `live.session` ends at the semicolon ending the `let input = ...` statement.
- `render_to_pixels` takes `&FrameInput`, not `&live`. No live borrow.
- `live.golden_name()` takes `&self` — fine, no conflicting borrows.
- `live.finish()` consumes `live` — the final move. Nothing else references `live` after.

---

## 07.2 tack_color golden (size matrix)

**File(s):** `oriterm/src/gpu/visual_regression/tack/mod.rs` (test functions)

The color screen is the highest-value GPU test: it directly validates `setaf`/`setab` rendering, named colors, and the 256-color block. Run at three sizes to match the Section 05 size matrix.

- [x] Add to `tack/mod.rs`:
  ```rust
  // The const ScenarioSpec for the color screen lives in
  // oriterm_test_support::tack_framework::scenarios::color (defined
  // in Section 05). Both this GPU test and the text test in
  // oriterm_core/tests/tack/test_menu/color.rs reference the SAME
  // const — single source of truth for "how do you reach the color
  // screen and what does the parser extract".
  use oriterm_test_support::tack_framework::scenarios::color::TACK_COLOR;

  // TACK_COLOR's screen_id is `"tack_color"`, so the golden PNG name
  // resolved by `run_tack_scenario_golden` via `live.golden_name()`
  // is `tack_color_<cols>x<rows>` → `tack_color_80x24.png` etc.
  //
  // The skip gate (tack + tic + headless_env) lives inside
  // `run_tack_scenario_golden` — these test wrappers do NOT call
  // `tack_gpu_available()` or `headless_env()` directly.

  #[test]
  fn tack_golden_color_80x24() {
      run_tack_scenario_golden(&TACK_COLOR, 80, 24);
  }

  #[test]
  fn tack_golden_color_97x33() {
      run_tack_scenario_golden(&TACK_COLOR, 97, 33);
  }

  #[test]
  fn tack_golden_color_120x40() {
      run_tack_scenario_golden(&TACK_COLOR, 120, 40);
  }
  ```

  **Catalog placement (from Section 05):** all const ScenarioSpec values live in `oriterm_test_support::tack_framework::scenarios::*` (one submodule per screen family). Both text tests and GPU tests reference the same const — no duplication.

- [x] **TDD ordering for golden generation (do this IN ORDER):**
  1. **Write the three `#[test] fn tack_golden_color_*` functions FIRST.** No golden PNG exists yet on disk.
  2. **First run:** `timeout 150 cargo test -p oriterm --features gpu-tests -- tack_golden_color_80x24`. `compare_with_reference` hits the "missing reference" branch (`mod.rs:205-217`), saves the rendered frame as `tack_color_80x24.png`, and returns `Ok(())` with the eprintln `reference saved: ...`. The test passes — but no comparison happened yet.
  3. **Inspect the saved PNG visually.** Open `oriterm/tests/references/tack_color_80x24.png` in an image viewer. Verify it shows tack's color screen (named-color rows, RGB blocks, 256-color grid). If it shows half-rendered output, a wrong screen, or stray cursor artifacts — the `ready_anchor` in `TACK_COLOR` is firing too early. Fix the spec, delete the PNG, repeat from step 2.
  4. **Second run (the SEMANTIC PIN):** re-run the same test. Now `compare_with_reference` hits the comparison branch (`mod.rs:219+`) and asserts pixel equality. This run is the actual regression test — it ONLY passes if the GPU produces the SAME pixels as the committed golden. **This is the failing-test-first discipline applied to golden tests:** the first run creates the artifact, the second run is the test. Both must be green.
  5. **Repeat steps 2-4 for 97x33 and 120x40.**
  6. **`ORITERM_UPDATE_GOLDEN=1` is for REGENERATION, not initial creation.** Use it ONLY when an intentional rendering change requires updating an EXISTING golden. The first-run "missing reference" path is the canonical creation flow; do not use the regeneration env var on day one — the in-source CI guard from 07.5b will treat it as suspicious if `CI=true` is also set.

- [x] Verify the goldens are committed under `oriterm/tests/references/`:
  - `tack_color_80x24.png`
  - `tack_color_97x33.png`
  - `tack_color_120x40.png`

- [x] Run all 3 tack_color_* tests once more under both debug AND release profiles to verify cross-profile pixel parity:
  ```
  timeout 150 cargo test -p oriterm --features gpu-tests -- tack_golden_color
  timeout 150 cargo test -p oriterm --release --features gpu-tests -- tack_golden_color
  ```
  Both must pass — debug+release parity is required (per CLAUDE.md TDD discipline). If the release profile produces different pixels, that's a real bug in the optimizer or font rasterization path — file via `/add-bug` and treat as blocker, do NOT add per-profile goldens.

- [x] **OSC round-trip anchor verification (`tack_color` only).** The 07.1 documentation warns that `tack_color`'s OSC 10/11/4 color queries round-trip through `PtyResponder` and may arrive AFTER the `ready_anchor` text lands. Now that `tack_color_80x24.png` exists, verify the anchor isn't too early:
  1. Run `tack_golden_color_80x24` once to generate `tack_color_80x24.png`.
  2. Add a one-shot debug helper at the bottom of `tack/mod.rs` (NOT a `#[test]` — a temporary scratch helper, deleted after verification): re-run `ScenarioRunner::run_with_session_at(&TACK_COLOR, 80, 24)`, render via `frame_input` + `render_to_pixels`, sleep 100ms, render a SECOND pass without re-spawning tack, and `pixel_diff` the two frames.
  3. If `mismatches == 0` → anchor is stable, OSC round-trip completed before the captured frame. Delete the scratch helper, check this box.
  4. If `mismatches > 0` → anchor fired too early; the 200ms quiet period in `wait_for` is racing with the OSC reply pipeline. Fix by changing `TACK_COLOR.ready_anchor` (in `crates/oriterm_test_support/src/tack_framework/scenarios/color/mod.rs`) to a text that only appears AFTER the OSC reply lands (e.g., a cap label whose value comes from a color-reply read), regenerate the 3 color goldens, and re-verify.
  5. The scratch helper MUST be deleted before commit — it's a one-time verification, not a permanent test. Adding a permanent OSC-stability test would belong in Section 04 (the `wait_for` plumbing layer), not here.

---

## 07.3 tack_graphic_rendition golden

**File(s):** `oriterm/src/gpu/visual_regression/tack/mod.rs` (test functions)

The graphic rendition screen is the second-highest GPU test value: bold/dim/italic/underline/reverse/blink labels are drawn IN their respective styles. Visual regressions in glyph shaping, italic skew, or underline pixel placement only show up here.

- [x] Add to `tack/mod.rs`:
  ```rust
  use oriterm_test_support::tack_framework::scenarios::graphic_rendition::TACK_GRAPHIC_RENDITION_SGR;

  // Golden name derived from TACK_GRAPHIC_RENDITION_SGR.screen_id
  // (`"tack_graphic_rendition"`) + 80x24 → `tack_graphic_rendition_sgr_80x24.png`.
  #[test]
  fn tack_golden_graphic_rendition_80x24() {
      run_tack_scenario_golden(&TACK_GRAPHIC_RENDITION_SGR, 80, 24);
  }
  ```

- [x] Apply the **TDD ordering from 07.2** (write test → first run creates `tack_graphic_rendition_sgr_80x24.png` via the missing-reference branch → visual inspection → second run is the pixel-equality regression test). Then run under both debug and release profiles. Commit `oriterm/tests/references/tack_graphic_rendition_sgr_80x24.png`.

---

## 07.4 tack_character_sets golden

**File(s):** `oriterm/src/gpu/visual_regression/tack/mod.rs` (test functions)

DEC line-drawing characters are a GPU-test specialty: text snapshots can verify the chars are present but can't verify they render at the right pixel positions (line-drawing chars must connect cell-to-cell or borders look broken).

- [x] Add to `tack/mod.rs`:
  ```rust
  use oriterm_test_support::tack_framework::scenarios::character_sets::TACK_TOOLS_G0_DEC_GRAPHICS;

  // Golden name derived from TACK_TOOLS_G0_DEC_GRAPHICS.screen_id
  // (`"tack_character_sets"`) + 80x24 → `tack_character_sets_80x24.png`.
  // The const lives at `tack_framework::scenarios::character_sets`
  // (single-word module name — Section 06 Agent-2 review pinned this
  // naming convention to match the existing 04/05 scenarios).
  #[test]
  fn tack_golden_character_sets_80x24() {
      run_tack_scenario_golden(&TACK_TOOLS_G0_DEC_GRAPHICS, 80, 24);
  }
  ```

- [x] Apply the **TDD ordering from 07.2** (write test → first run creates `tack_character_sets_80x24.png` via the missing-reference branch → visual inspection → second run is the pixel-equality regression test). Then run under both debug and release profiles. Commit `oriterm/tests/references/tack_character_sets_80x24.png`. Visual inspection focus: DEC line-drawing chars must connect cell-to-cell — borders form a continuous box, not a dotted-line approximation.

- [x] **TPR checkpoint** — `/tpr-review` covering 07.0–07.4 (the `frame_input_helper` extraction, the GPU bridge, and all five new goldens — color x3, graphic_rendition, character_sets). Catches: `TerminfoEnv` lifetime bugs (the `LiveSession._terminfo` field dropping early if a future refactor moves `live.session`), wrong palette constants in `frame_input_helper::frame_input`, missing `gpu-tests` feature gate, `ORITERM_UPDATE_GOLDEN` leaking goldens into CI, golden generation that wasn't actually committed, scenario const path drift between text and GPU consumers, borrow-checker pitfalls if the implementer refactors `run_tack_scenario_golden` to early-return the `(gpu, pipelines, renderer)` tuple.

---

## 07.4b tack_modes golden (SGR styling on cap labels)

**File(s):** `oriterm/src/gpu/visual_regression/tack/mod.rs` (test function)

The modes screen in tack draws each supported capability LABEL in a styled cell (bold/reverse/underlined depending on state), then places the cap name next to it. The text snapshot in Section 05 catches missing labels; only a pixel comparison catches cases where the bold label renders without its bold-weight glyphs, or where the reverse-video swatch has the wrong background alignment. The modes screen is visually dense and is a strong candidate for GPU coverage.

- [x] Add to `tack/mod.rs`:
  ```rust
  use oriterm_test_support::tack_framework::scenarios::modes::TACK_MODES_AM;

  // Reuse TACK_MODES_AM — it navigates to the same modes screen
  // Section 05 text scenarios snapshot. Golden name is
  // `<screen_id>_<cols>x<rows>` → `tack_modes_80x24.png`.
  // The insta text snapshot (`oriterm_core/tests/tack/test_menu/snapshots/...tack_modes_80x24.snap`)
  // and this PNG golden (`oriterm/tests/references/tack_modes_80x24.png`)
  // share the dedupable name stem but live in different artifact
  // trees — no collision.
  #[test]
  fn tack_golden_modes_80x24() {
      run_tack_scenario_golden(&TACK_MODES_AM, 80, 24);
  }
  ```

- [x] Apply the **TDD ordering from 07.2** (write test → first run creates `tack_modes_80x24.png` via the missing-reference branch → visual inspection → second run is the pixel-equality regression test). Then run under both debug and release profiles. Commit `oriterm/tests/references/tack_modes_80x24.png`. Visual inspection focus: each mode label must render in its declared SGR style (bold-weight glyphs visible as thicker strokes; reverse-video swatch must paint the background block with the right alignment to the cap label cell).

---

## 07.5a Determinism, tolerance, cross-adapter verification

**Depends on:** 07.2, 07.3, 07.4, 07.4b ALL complete (all 6 tack PNG goldens must be committed before this subsection can run — the verification reruns the FULL `tack_golden` filter which currently includes 0 tests until 07.2-07.4b have landed).

**File(s):** None (verification only — no source code or CI config edits in this subsection).

GPU tests are subject to subtle non-determinism: different GPU adapters produce slightly different anti-aliased pixels, font hinting can vary frame-to-frame, the existing vttest goldens cope via `PIXEL_TOLERANCE = 2` (per-channel) combined with `MAX_MISMATCH_PERCENT = 0.5` (at most 0.5% of pixels may differ). Verify the new tack goldens behave the same way.

**Scope of the tolerance (critical, do NOT misread).** `PIXEL_TOLERANCE` + `MAX_MISMATCH_PERCENT` absorbs **sparse anti-aliasing noise** — a handful of pixels differing by 1–2 channel values along glyph edges, driver-to-driver floating-point rounding in blend operations. It does NOT absorb **whole-glyph shifts**: if a different GPU adapter or a different font rasterizer moves a glyph by one subpixel column, the mismatch percentage jumps well past 0.5% and the test fails. **Pixel-exact goldens are committed from the developer's local WSL2/Linux environment using the embedded font (`FontSet::embedded()` → swash+skrifa rasterization at `TEST_FONT_SIZE_PT=12.0`, `TEST_DPI=96.0`, `HintingMode::Full`).** CI must run on an adapter-class where those rasterization paths produce byte-identical output.

- [x] **Prerequisite confirmation.** Verify all 6 tack PNG goldens exist on disk before starting verification:
  ```
  ls oriterm/tests/references/tack_color_80x24.png \
     oriterm/tests/references/tack_color_97x33.png \
     oriterm/tests/references/tack_color_120x40.png \
     oriterm/tests/references/tack_graphic_rendition_sgr_80x24.png \
     oriterm/tests/references/tack_character_sets_80x24.png \
     oriterm/tests/references/tack_modes_80x24.png
  ```
  All 6 must exist. If any are missing, complete the corresponding 07.2/07.3/07.4/07.4b subsection FIRST. Do NOT proceed.

- [x] **Determinism matrix (rerun × thread-count × profile):** Run all 6 tack_golden_* tests across the full 10×2×2 = 40-invocation matrix:
  - **10 reruns** × **2 thread-count modes (`--test-threads=1` and `--test-threads=4`)** × **2 profiles (debug and release)** = 40 total invocations.
  - The matrix exercises: same-adapter determinism (rerun axis), parallel safety of `headless_env()` per-test acquisition (thread-count axis), and debug+release codegen parity (profile axis).
  - All 40 invocations must produce 6 passing tests. ANY single failure is a real bug (file via `/add-bug`, do NOT bump tolerances).
  ```
  for profile in "" "--release"; do
    for threads in 1 4; do
      for i in $(seq 1 10); do
        timeout 150 cargo test -p oriterm --features gpu-tests $profile -- tack_golden --test-threads=$threads || { echo "FAIL profile=$profile threads=$threads run=$i"; exit 1; }
      done
    done
  done
  ```
  Loop must complete with no `FAIL` line.

- [x] Check the project's `PIXEL_TOLERANCE` constant in `oriterm/src/gpu/visual_regression/mod.rs` (line 49: `pub(super) const PIXEL_TOLERANCE: u8 = 2;`) and `MAX_MISMATCH_PERCENT` (line 53: `pub(super) const MAX_MISMATCH_PERCENT: f64 = 0.5;`). The tack goldens should pass at the SAME tolerances as the vttest goldens — no per-family override. If they require a higher tolerance to be deterministic on the developer's local adapter, that's a real bug in the rendering pipeline (file via `/add-bug` and treat as blocker, do NOT bump the constants).

- [x] Check golden file sizes: each tack PNG should be small (a few KB) — much smaller than the vttest goldens which capture more complex screens. If a tack golden is mysteriously large (>100KB), inspect it visually for stray content (e.g., partial alt-screen exit sequences leaking into the captured frame).

- [x] **Cross-adapter drift (scope limitation, document explicitly in 07.R if hit in practice).** GPU tests require a working GPU adapter with the same rasterization characteristics as the committed goldens. On WSL2 Linux with the user's setup (per memory: "GPU works in WSL"), the developer runs `lavapipe`/`dzn`/`d3d12` depending on the wgpu backend selection. On macOS the tests run via Metal. On Windows the tests run via DirectX. **These are NOT byte-identical across adapter classes.** The `PIXEL_TOLERANCE`+`MAX_MISMATCH_PERCENT` tolerance is wide enough for sparse AA noise on the SAME adapter but narrow enough that a whole-glyph shift from a different rasterizer will fail. **Consequence:**
  - Goldens are committed from WSL2/Linux on the developer's embedded-font rasterization path.
  - CI must either (a) run on the same adapter-class or (b) run with `ORITERM_SKIP_GPU_GOLDENS=1` (or equivalent) and SKIP the tack + vttest goldens entirely. A third option is per-platform golden trees (`oriterm/tests/references/<platform>/tack_*.png`) but that's explicitly OUT OF SCOPE for Section 07 — file via `/add-bug` if it becomes necessary.
  - If a future contributor hits cross-adapter failures, the response is NEVER "bump PIXEL_TOLERANCE" — it's either "skip the GPU goldens on this CI runner" or "add per-platform goldens as a separate plan section".

**07.5a → 07.5b ordering.** 07.5a validates the 6 goldens are clean BEFORE 07.5b adds the in-source CI guard. This ordering matters because 07.5b adds a regression test that will run as part of `cargo test -p oriterm` — if 07.5a hasn't established that the goldens are stable, a flaky golden could mask a real regression in the guard test. Run 07.5a to green completion, then start 07.5b on a separate commit.

---

## 07.5b CI guard implementation (in-source panic + regression test)

**Depends on:** 07.5a complete (all 6 goldens proven stable across reruns and parallelism).

**File(s):** `oriterm/src/gpu/visual_regression/mod.rs` (MODIFIED — add panic guard at top of `compare_with_reference`); `oriterm/src/gpu/visual_regression/meta_tests.rs` (MODIFIED — add `compare_with_reference_ci_guard_fires` regression test).

**Workflow scope (explicit decision):** Section 07 does NOT add or modify any `.github/workflows/*.yml` file. As of 2026-04-08 there is NO existing `gpu-tests` job in the workflow tree (`.github/workflows/{ci,nightly,auto-release}.yml` — verified by `grep gpu-tests` returning zero matches). Section 07's CI guard is therefore an **in-source guard ONLY**: a hard `panic!` inside `compare_with_reference` that fires if both `CI` and `ORITERM_UPDATE_GOLDEN` are set, regardless of which workflow runs the test. Any future plan that adds GPU CI is responsible for adding the workflow-level unset (`env: { ORITERM_UPDATE_GOLDEN: "" }`) — that handoff is encoded as a cross-plan blocker note below. Section 07 does NOT take ownership of new workflow plumbing because that would expand scope into runner-class selection, adapter-class pinning, and GPU runner sourcing — all of which deserve their own dedicated plan section.

<!-- blocks: any-future-gpu-ci-plan — when a future plan adds a `gpu-tests` job to `.github/workflows/*.yml`, that plan MUST include a step that explicitly unsets `ORITERM_UPDATE_GOLDEN` (`env: { ORITERM_UPDATE_GOLDEN: "" }`) on the test step. Section 07's in-source panic is the primary defense; the workflow-level unset is belt-and-suspenders. Without the workflow-level guard, a CI environment that already has `ORITERM_UPDATE_GOLDEN` set in its job env (e.g., debugging a workflow regen locally and forgetting to unset it) would still hit the in-source panic — but the panic is recoverable noise, while the workflow-level unset prevents the panic from ever firing. Both layers are required for full defense-in-depth. -->

**CI hazard background:** The env var `ORITERM_UPDATE_GOLDEN` regenerates goldens and returns `Ok(())` silently — a leaked env var in a CI runner overwrites committed references with whatever the CI renders. `compare_with_reference` (`oriterm/src/gpu/visual_regression/mod.rs:188`) short-circuits on `ORITERM_UPDATE_GOLDEN=1` with zero assertion, so a regeneration-under-CI produces a green build with wrong goldens. The in-source guard makes this failure mode loud.

- [x] **[WASTE/STYLE]** `oriterm/src/gpu/visual_regression/mod.rs:11` — when adding the in-source panic guard, also update the module-level doc comment line `//! 4. ORITERM_UPDATE_GOLDEN=1: overwrites references with current output.` to mention the CI guard. Append a fifth line: `//! 5. CI guard: when both CI and ORITERM_UPDATE_GOLDEN are set, compare_with_reference panics hard to prevent silent golden overwrite in CI runners.` Without this update, the doc comment is misleading — it implies regeneration always works, but post-07.5b it panics under CI. Touched files get hygiene-cleaned, per the broken-window policy.

- [x] Add an in-source panic guard at the top of `compare_with_reference` in `oriterm/src/gpu/visual_regression/mod.rs`. Insert it as the FIRST statement of the function body (before the `let ref_dir = reference_dir();` line on the current line 179):
  ```rust
  // CI hazard guard: regeneration mode MUST NOT run inside CI, or
  // committed goldens will be silently overwritten by whatever the
  // CI adapter renders. Section 07.5b blocker — in-source defense.
  if std::env::var("CI").is_ok() && std::env::var("ORITERM_UPDATE_GOLDEN").is_ok() {
      panic!(
          "ORITERM_UPDATE_GOLDEN is set inside a CI environment (CI=set). \
           Regeneration mode would silently overwrite committed goldens. \
           Unset ORITERM_UPDATE_GOLDEN in your CI config."
      );
  }
  ```
  This is a HARD panic (not `debug_assert!`) because the failure mode — silent golden overwrite — produces a green build with wrong goldens, which is strictly worse than a loud panic. It fires on any CI runner that honors the standard `CI=true` env var (GitHub Actions, GitLab CI, CircleCI, Azure Pipelines, Buildkite). `debug_assert!` would silently no-op in `cargo test --release`, defeating the purpose.

- [x] **Pre-implementation pollution audit (LOAD-BEARING — verify BEFORE writing the regression test).** Confirm which workspace files currently read `CI` or `ORITERM_UPDATE_GOLDEN`:
  ```
  rg -l 'env::var\("CI"\)|env::var\("ORITERM_UPDATE_GOLDEN"\)' --type rust
  ```
  As of 2026-04-08 this returns:
  - `oriterm/src/gpu/visual_regression/mod.rs` (the function under guard — line 188)
  - `oriterm/src/gpu/visual_regression/text_blink_tests.rs` (line 100 — text-blink visual goldens use the SAME env var to opt into regeneration mode; **this is a third reader the prior agents missed**)

  The existing `update_golden_overwrites_reference` test in `meta_tests.rs:184` already documents the risk: `// We can't set the env var here without affecting other tests`. That comment is still correct after Section 07 lands. Therefore the regression test below MUST use `#[ignore]` mode from day one — it cannot run as part of the default `cargo test` invocation, or `text_blink_tests.rs::*` will see `ORITERM_UPDATE_GOLDEN=1` leak in under parallel scheduling and silently regenerate the text-blink goldens.

- [x] Add a regression test in `oriterm/src/gpu/visual_regression/meta_tests.rs` (NOT in a non-existent `tests.rs`). The compare-framework tests already live in `meta_tests.rs` (verified: lines 7-8 import `compare_with_reference`, `pixel_diff`, etc.; existing tests at lines 117-216 cover the missing-golden and update-golden paths). Add the new regression test as a sibling of `update_golden_overwrites_reference` (line 184). **Mark it `#[ignore]` to prevent env-var pollution of `text_blink_tests.rs`** (see the audit checkbox above):
  ```rust
  /// Regression pin for the Section 07.5b in-source CI guard.
  ///
  /// `#[ignore]` is MANDATORY: this test mutates `CI` and
  /// `ORITERM_UPDATE_GOLDEN` env vars via `unsafe { set_var }`,
  /// and `text_blink_tests.rs:100` reads `ORITERM_UPDATE_GOLDEN` —
  /// running under default `cargo test` would leak the var into
  /// a parallel sibling and silently regenerate the text-blink
  /// goldens. This test must be invoked explicitly via:
  ///   `cargo test -p oriterm --features gpu-tests 'compare_with_reference_ci_guard_fires' -- --ignored --test-threads=1`
  ///
  /// The trade-off: the test does NOT run on default `cargo test`
  /// invocations. The `07.N` checklist therefore includes a dedicated
  /// command line to run it once per CI cycle on the developer machine
  /// (not in CI itself, since the in-source guard already protects CI).
  #[test]
  #[ignore = "mutates CI/ORITERM_UPDATE_GOLDEN env vars; would pollute text_blink_tests.rs under parallel execution. Run explicitly with --ignored."]
  #[should_panic(expected = "ORITERM_UPDATE_GOLDEN is set inside a CI environment")]
  fn compare_with_reference_ci_guard_fires() {
      // Set BOTH env vars and verify compare_with_reference panics
      // hard. This is the regression-pin for the 07.5b in-source CI
      // guard — if a future refactor removes the panic check, this
      // test fails immediately instead of silently regressing the
      // golden-overwrite hazard.
      //
      // SAFETY of `set_var` in 2024 edition: invoked via `--ignored`,
      // so this test runs in isolation. The `#[ignore]` attribute is
      // load-bearing for safety, not just hygiene — without it the
      // env-var write races with `text_blink_tests.rs:100`.
      unsafe {
          std::env::set_var("CI", "true");
          std::env::set_var("ORITERM_UPDATE_GOLDEN", "1");
      }

      // Trivial 1x1 pixel — the panic must fire BEFORE any disk I/O.
      let pixels: Vec<u8> = vec![0, 0, 0, 255];
      let _ = compare_with_reference("_meta_test_ci_guard", &pixels, 1, 1);
      // Unreachable — the panic above must fire.
      // Note: we deliberately do NOT remove_var on the way out — the
      // panic propagates before any cleanup runs. Since `--ignored`
      // mode runs this test in isolation, leaked env vars cannot
      // pollute siblings.
  }
  ```
  Note: `unsafe { std::env::set_var(...) }` is required in Rust 2024 edition (`oriterm/Cargo.toml` inherits `edition.workspace = true` from the workspace `Cargo.toml` which pins `edition = "2024"`).

- [x] **Verify the `#[ignore]` regression test runs and panics as expected.** Run it explicitly:
  ```
  timeout 150 cargo test -p oriterm --features gpu-tests 'gpu::visual_regression::meta_tests::compare_with_reference_ci_guard_fires' -- --ignored
  ```
  Must report `1 passed` (the `#[should_panic]` matched). If the panic message doesn't match, the assertion text in `compare_with_reference` and the `expected =` string have drifted — update both to match.

- [x] **Confirm default `cargo test` still passes** WITHOUT running the ignored regression test:
  ```
  timeout 150 cargo test -p oriterm --features gpu-tests -- visual_regression::meta_tests
  ```
  All non-ignored meta_tests pass; the regression test is reported as `ignored`. Run with both `--test-threads=1` and `--test-threads=4` — both must be green AND the text-blink goldens (`visual_regression::text_blink_tests::*`) must remain unchanged on disk (verify with `git status oriterm/tests/references/text_blink_*.png` — should report no modifications).

- [x] `timeout 150 cargo test -p oriterm --features gpu-tests 'gpu::visual_regression::meta_tests::compare_with_reference_ci_guard_fires' -- --ignored` — passes (panic fires, `should_panic` matches the message). The `--ignored` flag is REQUIRED — without it cargo skips the test silently and the verification is a no-op.

- [x] `timeout 150 cargo test -p oriterm --features gpu-tests` — full GPU-feature test pass, no flakes. The ignored regression test reports as `1 ignored` and is NOT executed in this run.

---

## 07.R Third Party Review Findings

- [x] `[TPR-07-006][low]` `plans/tack-conformance/section-07-gpu-golden-images.md:645` — Stale non-runnable CI-guard test invocation in plan text.
  Resolved: Fixed on 2026-04-09. Corrected remaining stale invocations at plan lines 645 and 763.

- [x] `[TPR-07-007][low]` `oriterm/src/gpu/visual_regression/tack/mod.rs:13` — Module-level rustdoc skip-cases list was stale after ScenarioRunner::available() fix.
  Resolved: Fixed on 2026-04-09. Updated `//! Skip cases` list to document version-gate skip via `ScenarioRunner::available()`.

- [x] `[TPR-07-005][medium]` `oriterm/src/gpu/visual_regression/tack/mod.rs:64` — `run_tack_scenario_golden` bypasses the canonical tack version gate, so Section 07 hard-fails on unsupported tack versions instead of skipping cleanly.
  Resolved: Fixed on 2026-04-09. Replaced hand-rolled `tack_available()` + `tic_available()` with `ScenarioRunner::available()` which includes the canonical `tack_version_supported()` gate. Removed unused `tack_available`/`tic_available` imports.

- [x] `[TPR-07-001][medium]` `plans/tack-conformance/section-07-gpu-golden-images.md:4` — Section 07 is marked complete before its own closeout gates are done.
  Resolved: Fixed on 2026-04-09. Codex reverted section/overview/index status to `in-progress`. Section will be marked `complete` only after TPR passes clean.

- [x] `[TPR-07-002][low]` `plans/tack-conformance/section-07-gpu-golden-images.md:749` — The checked cross-test pollution audit claim does not match the documented command's real output.
  Resolved: Fixed on 2026-04-09. Corrected 07.N checklist text from "EXACTLY THREE files" to "EXACTLY TWO files" — `meta_tests.rs` uses `set_var` (write), not `env::var` (read), so it correctly does not match the reader-audit pattern.

- [x] `[TPR-07-003][low]` `plans/tack-conformance/section-07-gpu-golden-images.md:682` — The checked CI-guard verification command is not runnable as written.
  Resolved: Fixed on 2026-04-09. Corrected all documented invocations to use correct Cargo arg ordering: filter before `--`, `--ignored` after `--`. Fixed in plan lines 684, 694, 755, and in `meta_tests.rs:224-225` source comment.

- [x] `[TPR-07-004][medium]` `plans/tack-conformance/section-07-gpu-golden-images.md:763` — The completion checklist still claims Section 07 closeout is done even though the section remains open.
  Resolved: Fixed on 2026-04-09. Unchecked premature plan-sync items in 07.N. Plan sync is now deferred until TPR/hygiene review gates pass clean.

---

## 07.N Completion Checklist

**Structural gates:**
- [x] `oriterm/src/gpu/visual_regression/frame_input_helper.rs` exists (07.0) — single canonical `PtySession -> FrameInput` builder
- [x] `vttest/render.rs::frame_input` body is DELETED (re-exports from `frame_input_helper` or calls through it) — no duplicate copy remains
- [x] `oriterm/src/gpu/visual_regression/tack/mod.rs` exists, <500 lines
- [x] `mod frame_input_helper;` added as a sibling of `mod vttest;` / `mod tack;` in `visual_regression/mod.rs`
- [x] `mod tack;` added to `oriterm/src/gpu/visual_regression/mod.rs` as a plain sibling of `mod vttest;` — NO per-submodule `#[cfg]`. The parent `visual_regression` module itself is gated at `oriterm/src/gpu/mod.rs:79` under `#[cfg(all(test, feature = "gpu-tests"))]`, so the new submodule inherits that gate automatically. This matches `mod vttest;` on line 34 exactly.

**Contract gates:**
- [x] `LiveSession` wrapper holds the `TerminfoEnv` to keep it alive for tack's lifetime (defined by Section 04 — Section 07 just consumes it). Implementation never destructures `live` or moves `live.session` out.
- [x] `run_tack_scenario_golden(spec, cols, rows)` is the single canonical entry point — no `gpu`/`pipelines`/`renderer` parameters; the function owns `headless_env()` acquisition internally
- [x] `run_tack_scenario_golden(...)` owns the single consolidated skip gate (tack + tic + `headless_env`). No per-test function calls `tack_gpu_available()` or `headless_env()` directly.
- [x] `run_tack_scenario_golden(...)` calls `live.finish()` BEFORE returning (M5 cleanup contract from Section 04). The `finish` call is positioned AFTER the `compare_with_reference` capture so the visual diff is recorded before the exit-status assertion runs
- [x] `run_tack_scenario_golden(...)` derives the golden file name from `live.golden_name()` (the SSOT helper from Section 04), NOT a hand-passed `&str` parameter and NOT a `format!("{}_{}x{}", ...)` rebuild at the call site
- [x] `frame_input_helper::frame_input(...)` is the single FrameInput builder for both vttest and tack — no local `build_frame_input` copy exists in `tack/mod.rs`
- [x] No `FinishOnDrop` RAII guard exists around `LiveSession` — the current `compare → finish → log` ordering is the canonical pattern (see 07.1 justification).

**Artifact gates:**
- [x] 6 PNG goldens committed under `oriterm/tests/references/`:
  - tack_color_80x24.png
  - tack_color_97x33.png
  - tack_color_120x40.png
  - tack_graphic_rendition_sgr_80x24.png
  - tack_character_sets_80x24.png
  - tack_modes_80x24.png

**Test gates (07.5a determinism / verification):**
- [x] All 6 tack_golden_* tests pass: `timeout 150 cargo test -p oriterm --features gpu-tests -- tack_golden`
- [x] **40-invocation determinism matrix passes (10 reruns × 2 thread-modes × 2 profiles).** See 07.5a for the full bash loop. Same-adapter determinism is necessary but NOT sufficient for cross-adapter — see 07.5a for the WSL2-only policy.
- [x] Debug + release parity: tack goldens produce identical pixels in both `cargo test --features gpu-tests` and `cargo test --release --features gpu-tests`
- [x] PIXEL_TOLERANCE and MAX_MISMATCH_PERCENT unchanged (tack goldens use the same tolerances as vttest goldens — no per-family bumps)
- [x] vttest goldens (`timeout 150 cargo test -p oriterm --features gpu-tests -- vttest_golden`) still pass after `frame_input_helper` extraction (07.0) — byte-identical output pre/post extraction. Verified in BOTH debug and release profiles.

**CI hazard gates (07.5b blocker — in-source-only scope, see 07.5b for the workflow scope decision):**
- [x] **In-source guard (canonical, ONLY guard owned by Section 07):** `oriterm/src/gpu/visual_regression/mod.rs::compare_with_reference` starts with a hard `panic!` when BOTH `CI` and `ORITERM_UPDATE_GOLDEN` are set (see 07.5b for the exact snippet). This is the sole defense Section 07 adds — fires on every CI runner that sets `CI=true` (GitHub Actions, GitLab, CircleCI, Buildkite, Azure). `debug_assert!` is NOT sufficient: release CI builds would silently skip the check.
- [x] **NO workflow-level edits in Section 07.** As of 2026-04-08 there is NO `gpu-tests` job in `.github/workflows/{ci,nightly,auto-release}.yml` (verified by `rg gpu-tests .github/workflows`). Section 07 does NOT add or modify any `.github/workflows/*.yml` file. The cross-plan handoff is encoded as `<!-- blocks: any-future-gpu-ci-plan -->` inside 07.5b — any future plan that adds GPU CI is responsible for the workflow-level `env: { ORITERM_UPDATE_GOLDEN: "" }` step.
- [x] A regression test (`compare_with_reference_ci_guard_fires`) is in `oriterm/src/gpu/visual_regression/meta_tests.rs` as a sibling of the existing compare-framework tests (NOT in a non-existent `tests.rs`). It uses `#[should_panic(expected = "...")]` AND `#[ignore]` (load-bearing — see next gate) and forces both env vars to verify the in-source panic fires. Prevents a future refactor from removing the guard silently.
- [x] **`#[ignore]` is mandatory on the regression test** because `oriterm/src/gpu/visual_regression/text_blink_tests.rs:100` ALSO reads `ORITERM_UPDATE_GOLDEN` — running the regression test under default `cargo test` would leak the env var into parallel `text_blink_tests` execution and silently regenerate text-blink goldens. The `#[ignore]` attribute is the canonical isolation mechanism (run via `cargo test -p oriterm --features gpu-tests 'compare_with_reference_ci_guard_fires' -- --ignored --test-threads=1`).
- [x] Default `cargo test -p oriterm --features gpu-tests` (without `--ignored`) runs green at BOTH `--test-threads=1` and `--test-threads=4`. The regression test reports as `ignored` and does NOT execute. The text-blink goldens (`oriterm/tests/references/text_blink_*.png`) remain byte-identical post-test (verify via `git status oriterm/tests/references/text_blink_*.png` — must show no modifications).
- [x] Explicit verification of the regression test: `timeout 150 cargo test -p oriterm --features gpu-tests 'gpu::visual_regression::meta_tests::compare_with_reference_ci_guard_fires' -- --ignored` reports `1 passed`. This is a one-time developer-machine check, not a CI gate.
- [x] Cross-test pollution audit: `rg -l 'env::var\("CI"\)|env::var\("ORITERM_UPDATE_GOLDEN"\)' --type rust` returns EXACTLY TWO files that READ the env vars: `oriterm/src/gpu/visual_regression/mod.rs` (the function under guard) and `oriterm/src/gpu/visual_regression/text_blink_tests.rs` (the pre-existing reader at line 100). `meta_tests.rs` uses `set_var` (write), not `env::var` (read), so it does not match the pattern — this is correct because the audit targets readers, not writers. If the audit returns ANY file beyond these two, STOP and re-evaluate the `#[ignore]` strategy — there may be an additional leak vector.

**Cross-adapter drift gates (07.5a blocker — this is the other load-bearing non-obvious rule):**
- [x] All 6 tack goldens are generated on WSL2/Linux from the developer's embedded-font rasterization path — NEVER from macOS Metal or Windows DirectX, because those produce byte-divergent AA that jumps past `MAX_MISMATCH_PERCENT`. This is a HARD rule: if a contributor generates goldens on a non-WSL2 adapter, the committed PNGs will be green on their machine and red on the developer's. Enforce via the commit message (`tack: regenerate goldens (wsl2 lavapipe)` — pattern to match) and via the section's `reviewed: true` gate at PR review.
- [x] A `NOTES.md` or equivalent is NOT created. The cross-adapter policy lives inside this section file (07.5a) — committing it to a separate doc file is scattered knowledge. Future contributors who hit `MAX_MISMATCH_PERCENT` failures are routed to Section 07.5a via the panic message from `compare_with_reference`.
**Standard gates:**
- [x] `./build-all.sh` green
- [x] `./clippy-all.sh` green
- [x] `timeout 150 ./test-all.sh` green
- [x] All TPR checkpoint findings resolved (see `07.R`)

**Plan sync:**
- [x] Section frontmatter `status` → `complete`
- [x] Section frontmatter `reviewed` → `true` (flipped during multi-agent review, before implementation)
- [x] `00-overview.md` Quick Reference table updated
- [x] `00-overview.md` Mission Success Criteria #10 ticked
- [x] `index.md` Section 07 updated (status flips from `In Progress` → `Complete`)
- [x] Section 04, 05, 06 still passing — no framework changes needed since they already reference `oriterm_test_support::tack_framework::*` from the start
- [x] Section 09 unblocked: Section 09's `depends_on` frontmatter lists `"07"` — verified.

**Final review gates:**
- [x] `/tpr-review` final pass clean — 5 iterations, 7 findings (TPR-07-001 through TPR-07-007), all resolved. Code fixes: version gate (TPR-07-005), doc comments (TPR-07-003/007). Plan text fixes: premature completion (TPR-07-001/004), audit text (TPR-07-002), invocation syntax (TPR-07-003/006).
- [x] `/impl-hygiene-review last commit` final pass clean (after TPR) — specifically verifies the `frame_input_helper` extraction removed the `LEAK:algorithmic-duplication` finding and no `build_frame_input` copy snuck back in

**Exit Criteria:** `timeout 150 cargo test -p oriterm --features gpu-tests -- tack_golden` runs all 6 tack GPU goldens (3 color sizes + 1 graphic rendition + 1 character sets + 1 modes) deterministically. Pixel comparison passes at the existing PIXEL_TOLERANCE. Goldens are committed under `oriterm/tests/references/tack_*.png`. The text scenarios from Sections 04/05/06 still pass — they reference the same const ScenarioSpec values via `oriterm_test_support::tack_framework::scenarios::*`. Section 07 closes the visual regression gap for tack scenarios.
