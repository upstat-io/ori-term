//! Visual verification chain harness wrapping `SpecHarness`.
//!
//! `VisualSpecHarness` composes the headless `SpecHarness` (rungs 1-4)
//! with GPU infrastructure for visual rungs 5-8. It builds `FrameInput`
//! from the terminal state, calls `WindowRenderer::prepare()`, and
//! inspects the resulting `PreparedFrame` for GPU instance observation.
//!
//! Uses the same golden-test palette constants as
//! `visual_regression::frame_input_helper` (SSOT — the constants are
//! identical, not duplicated, because this harness builds from `Term`
//! directly rather than from `PtySession`).

use oriterm_core::{Rgb, TermMode};

use oriterm_test_support::spec_chain::{RungName, RungResult, SpecHarness, SpecScenario};

use crate::gpu::frame_input::{FrameInput, FramePalette, ViewportSize};
use crate::gpu::pipelines::GpuPipelines;
use crate::gpu::state::GpuState;
use crate::gpu::window_renderer::WindowRenderer;

use super::observers;

/// Visual verification chain harness (rungs 1-8).
///
/// Wraps `SpecHarness` (headless, rungs 1-4) and adds GPU observation
/// for rungs 5-8. Holds the headless GPU environment created by
/// `headless_env_with_hinting()`.
pub struct VisualSpecHarness {
    core: SpecHarness,
    gpu: GpuState,
    pipelines: GpuPipelines,
    renderer: WindowRenderer,
}

/// Standard golden-test palette foreground.
const PALETTE_FG: Rgb = Rgb {
    r: 211,
    g: 215,
    b: 207,
};

/// Standard golden-test palette background.
///
/// Must differ from cell bg `(0,0,0)` so the prepare phase emits bg quads.
const PALETTE_BG: Rgb = Rgb { r: 1, g: 1, b: 1 };

impl VisualSpecHarness {
    /// Create a new visual harness with default terminal (24×80) and
    /// headless GPU environment.
    ///
    /// Returns `None` if no GPU adapter is available (CI without GPU).
    pub fn new() -> Option<Self> {
        Self::with_size(24, 80)
    }

    /// Create a new visual harness with custom terminal dimensions.
    ///
    /// Returns `None` if no GPU adapter is available.
    pub fn with_size(lines: usize, cols: usize) -> Option<Self> {
        let (gpu, pipelines, renderer) = super::super::headless_env()?;
        let core = SpecHarness::with_size(lines, cols);
        Some(Self {
            core,
            gpu,
            pipelines,
            renderer,
        })
    }

    /// Run a scenario through every applicable rung (1-8), stopping at
    /// the first failure.
    ///
    /// Rungs 1-4 are delegated to the inner `SpecHarness` observers.
    /// Rungs 5-6 use GPU-aware observers. Rungs 7-8 are stubs until
    /// Section 04.4 (texture-render + golden-image, after Section 05).
    pub fn run_visual_scenario(&mut self, scenario: &SpecScenario) -> Vec<RungResult> {
        self.core.prepare_scenario(scenario);

        let mut results = Vec::new();
        let mut frame_input: Option<FrameInput> = None;

        for &rung in scenario.applicable_rungs() {
            let result = match rung {
                RungName::FrameInput => {
                    let input = self.build_frame_input();
                    let r = match &scenario.expectations.frame_input {
                        Some(e) => observers::observe_frame_input(&input, e),
                        None => RungResult::pass(rung),
                    };
                    frame_input = Some(input);
                    r
                }
                RungName::GpuInstance => {
                    let input = frame_input.as_ref().expect(
                        "GpuInstance rung requires FrameInput rung to have run first \
                         (rung chain ordering guarantees this)",
                    );
                    self.renderer
                        .prepare(input, &self.gpu, &self.pipelines, (0.0, 0.0), 1.0, true);
                    match &scenario.expectations.gpu_instance {
                        Some(e) => observers::observe_gpu_instance(&self.renderer.prepared, e),
                        None => RungResult::pass(rung),
                    }
                }
                // Rungs 7-8: stubs until 04.4 (after Section 05).
                RungName::TextureRender | RungName::GoldenImage => RungResult::pass(rung),
                // Rungs 1-4: delegate to core observers.
                _ => self.core.observe_rung(rung, &scenario.expectations),
            };

            let failed = !result.passed;
            results.push(result);
            if failed {
                break;
            }
        }

        results
    }

    /// Build a `FrameInput` from the current terminal state.
    ///
    /// Uses the same palette constants as `frame_input_helper::frame_input()`
    /// (canonical home for `PtySession` → `FrameInput`). This method is
    /// the canonical home for `Term` → `FrameInput` in harness context.
    fn build_frame_input(&self) -> FrameInput {
        let term = self.core.term();
        let content = term.renderable_content();
        let cell = self.renderer.cell_metrics();

        let cols = content.cols;
        let rows = content.lines;
        let w = (cell.width * cols as f32).ceil() as u32;
        let h = (cell.height * rows as f32).ceil() as u32;

        let reverse_video = content.mode.contains(TermMode::REVERSE_VIDEO);
        let (fg, bg) = if reverse_video {
            (PALETTE_BG, PALETTE_FG)
        } else {
            (PALETTE_FG, PALETTE_BG)
        };

        FrameInput {
            content,
            viewport: ViewportSize::new(w, h),
            cell_size: cell,
            content_cols: cols,
            content_rows: rows,
            palette: FramePalette {
                background: bg,
                foreground: fg,
                cursor_color: Rgb {
                    r: 255,
                    g: 255,
                    b: 255,
                },
                opacity: 1.0,
                selection_fg: None,
                selection_bg: None,
            },
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

    /// Borrow the inner `SpecHarness`.
    pub fn core(&self) -> &SpecHarness {
        &self.core
    }

    /// Borrow the inner `SpecHarness` mutably.
    pub fn core_mut(&mut self) -> &mut SpecHarness {
        &mut self.core
    }
}
