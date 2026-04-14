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
use crate::gpu::visual_regression::GoldenLaneConfig;
use crate::gpu::window_renderer::WindowRenderer;

use super::observers;

/// Pixel data from the most recent `render_frame_cached()` call,
/// carried between the TextureRender and GoldenImage rungs.
struct RenderedPixelData {
    pixels: Vec<u8>,
    width: u32,
    height: u32,
}

/// Visual verification chain harness (rungs 1-8).
///
/// Wraps `SpecHarness` (headless, rungs 1-4) and adds GPU observation
/// for rungs 5-8. Holds the deterministic GPU environment created by
/// `headless_env_with_pinned_software_rasterizer()` and the
/// `GoldenLaneConfig` that controls hinting, subpixel positioning,
/// and tolerance parameters.
pub struct VisualSpecHarness {
    core: SpecHarness,
    gpu: GpuState,
    pipelines: GpuPipelines,
    renderer: WindowRenderer,
    config: GoldenLaneConfig,
    /// Pixel data from the TextureRender rung, consumed by GoldenImage.
    last_rendered: Option<RenderedPixelData>,
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
    /// the deterministic golden lane defaults.
    ///
    /// Returns `None` if no software rasterizer is available.
    pub fn new() -> Option<Self> {
        Self::with_size(24, 80)
    }

    /// Create a new visual harness with custom terminal dimensions
    /// using the deterministic golden lane.
    ///
    /// Returns `None` if no software rasterizer is available.
    pub fn with_size(lines: usize, cols: usize) -> Option<Self> {
        Self::with_config(GoldenLaneConfig {
            viewport_rows: lines as u32,
            viewport_cols: cols as u32,
            ..GoldenLaneConfig::SPEC_DEFAULT
        })
    }

    /// Create a new visual harness with explicit golden lane configuration.
    ///
    /// Uses `headless_env_with_pinned_software_rasterizer()` to pin
    /// the software rasterizer, hinting mode, and glyph format from
    /// `config`. Returns `None` if the software rasterizer is unavailable.
    pub fn with_config(config: GoldenLaneConfig) -> Option<Self> {
        let (gpu, pipelines, renderer) =
            super::super::headless_env_with_pinned_software_rasterizer(&config)?;
        let core =
            SpecHarness::with_size(config.viewport_rows as usize, config.viewport_cols as usize);
        Some(Self {
            core,
            gpu,
            pipelines,
            renderer,
            config,
            last_rendered: None,
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
        self.last_rendered = None;

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
                RungName::TextureRender => {
                    let _input = frame_input
                        .as_ref()
                        .expect("TextureRender rung requires FrameInput rung to have run first");
                    // Render via the production cached path.
                    let vp = self.renderer.prepared.viewport;
                    let output = self.renderer.render_frame_cached(
                        &self.gpu,
                        &self.pipelines,
                        vp.width,
                        vp.height,
                        true,
                    );
                    let pixels = self
                        .gpu
                        .read_render_target(&output)
                        .expect("pixel readback should succeed");
                    // Stash pixels for the GoldenImage rung, then borrow.
                    self.last_rendered = Some(RenderedPixelData {
                        pixels,
                        width: vp.width,
                        height: vp.height,
                    });
                    let data = self.last_rendered.as_ref().unwrap();
                    let rendered = observers::RenderedPixels {
                        pixels: &data.pixels,
                        width: data.width,
                        height: data.height,
                    };
                    match &scenario.expectations.texture {
                        Some(e) => observers::observe_texture_render(&rendered, e),
                        None => RungResult::pass(rung),
                    }
                }
                RungName::GoldenImage => {
                    let data = self
                        .last_rendered
                        .as_ref()
                        .expect("GoldenImage rung requires TextureRender rung to have run first");
                    let rendered = observers::RenderedPixels {
                        pixels: &data.pixels,
                        width: data.width,
                        height: data.height,
                    };
                    match &scenario.expectations.golden {
                        Some(e) => observers::observe_golden_image(
                            &rendered,
                            scenario.catalog_row_id,
                            e,
                            self.config(),
                        ),
                        None => RungResult::pass(rung),
                    }
                }
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
    ///
    /// `subpixel_positioning` is read from `self.config` — spec-conformance
    /// goldens disable it for deterministic pixel output.
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
            subpixel_positioning: self.config.subpixel_positioning,
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

    /// Borrow the golden lane config.
    pub fn config(&self) -> &GoldenLaneConfig {
        &self.config
    }
}
