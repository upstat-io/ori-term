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
//!   - tack not installed, tic not installed, or tack version
//!     unsupported (via `ScenarioRunner::available()`)
//!   - GPU adapter unavailable (no compatible wgpu backend)
//!
//! All three skip conditions are consolidated into
//! [`run_tack_scenario_golden`] — per-test functions do NOT scatter
//! `headless_env()` probes.

use oriterm_test_support::tack_framework::scenarios::character_sets::TACK_TOOLS_G0_DEC_GRAPHICS;
use oriterm_test_support::tack_framework::scenarios::color::TACK_COLOR;
use oriterm_test_support::tack_framework::scenarios::graphic_rendition::TACK_GRAPHIC_RENDITION_SGR;
use oriterm_test_support::tack_framework::scenarios::modes::TACK_MODES_AM;
use oriterm_test_support::tack_framework::{ScenarioRunner, ScenarioSpec};

use super::frame_input_helper::frame_input;
use super::{compare_with_reference, headless_env, render_to_pixels};

/// Run a tack scenario through the GPU pipeline and assert the
/// rendered framebuffer matches a committed PNG golden.
///
/// This function owns the ENTIRE skip gate:
///   - `ScenarioRunner::available()` — else `eprintln` + return
///     (includes `tack_available()`, `tic_available()`, and
///     `tack_version_supported()` via the canonical gate)
///   - `headless_env()` — else `eprintln` + return
///
/// Per-test wrappers MUST NOT call `headless_env()` directly.
/// Centralizing the gate here eliminates the 6-site scatter that
/// would otherwise spread GPU-adapter skip checks across every
/// `#[test]` function.
///
/// Flow on the hot path (all gates satisfied):
///   1. `ScenarioRunner` spawns tack, navigates to `spec.ready_anchor`.
///   2. Borrow `&live.session` to build a `FrameInput` via the shared
///      `frame_input_helper::frame_input` (Section 07.0).
///   3. `render_to_pixels` produces a framebuffer.
///   4. `live.golden_name()` produces the PNG filename (SSOT helper
///      from Section 04 — never rebuild `format!` at this site).
///   5. `compare_with_reference` diffs pixels vs. the committed PNG.
///   6. `live.finish()` consumes `live`, quits tack, asserts
///      `exit.success()`. This runs AFTER the compare so the visual
///      diff is logged before the exit-status assertion.
///   7. If the visual diff failed, panic with the golden name.
///
/// **Do NOT wrap `live` in a `FinishOnDrop` RAII guard.** `PtySession`
/// already has a `Drop` impl that kills + reaps the child, so the
/// panic path still cleans up FDs; the only thing lost on panic is
/// the exit-status assertion. A `Drop`-based `finish` would trigger
/// Rust's panic-during-drop abort semantics on the double-failure path.
fn run_tack_scenario_golden(spec: &ScenarioSpec, cols: u16, rows: u16) {
    if !ScenarioRunner::available() {
        eprintln!("skipped: tack/tic not installed or tack version unsupported");
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
    // stay alive until live.finish() runs.
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

// Color screen: highest GPU test value (setaf/setab rendering)

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

// Graphic rendition: SGR styles (bold/dim/italic/underline)

#[test]
fn tack_golden_graphic_rendition_80x24() {
    run_tack_scenario_golden(&TACK_GRAPHIC_RENDITION_SGR, 80, 24);
}

// Character sets: DEC line-drawing (G0/G1/GL/GR banks)

#[test]
fn tack_golden_character_sets_80x24() {
    run_tack_scenario_golden(&TACK_TOOLS_G0_DEC_GRAPHICS, 80, 24);
}

// Modes: SGR-styled cap labels

#[test]
fn tack_golden_modes_80x24() {
    run_tack_scenario_golden(&TACK_MODES_AM, 80, 24);
}
