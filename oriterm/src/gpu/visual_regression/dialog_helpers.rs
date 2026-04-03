//! Dialog-specific headless rendering helpers.
//!
//! Provides `headless_dialog_env()` (UI-only renderer with embedded IBM
//! Plex Mono) and `render_dialog_to_pixels()` (scene → pixel readback)
//! for settings-dialog golden tests.

use oriterm_core::Rgb;
use oriterm_ui::draw::Scene;

use crate::font::ui_font_sizes::PRELOAD_SIZES;
use crate::font::ui_font_sizes::UiFontSizes;
use crate::font::{FontSet, GlyphFormat, HintingMode};
use crate::gpu::pipelines::GpuPipelines;
use crate::gpu::state::GpuState;
use crate::gpu::window_renderer::WindowRenderer;

/// Dark theme background (`bg_primary` = #1a1b1e).
const DIALOG_BG: Rgb = Rgb {
    r: 26,
    g: 27,
    b: 30,
};

/// Default font weight for UI text.
const UI_FONT_WEIGHT: u16 = 400;

/// Attempt to create a headless UI-only rendering environment at 96 DPI.
///
/// Uses `FontSet::ui_embedded()` (IBM Plex Mono) for deterministic output.
/// Returns `None` if no GPU adapter is available.
pub(super) fn headless_dialog_env() -> Option<(GpuState, GpuPipelines, WindowRenderer)> {
    headless_dialog_env_with_dpi(96.0)
}

/// Headless UI-only rendering environment with configurable DPI.
pub(super) fn headless_dialog_env_with_dpi(
    dpi: f32,
) -> Option<(GpuState, GpuPipelines, WindowRenderer)> {
    let gpu = GpuState::new_headless().ok()?;
    let pipelines = GpuPipelines::new(&gpu);
    let ui_font_sizes = UiFontSizes::new(
        FontSet::ui_embedded(),
        dpi,
        GlyphFormat::Alpha,
        HintingMode::Full,
        UI_FONT_WEIGHT,
        550,
        PRELOAD_SIZES,
    )
    .ok()?;
    let scale = dpi / 96.0;
    let mut renderer = WindowRenderer::new_ui_only(&gpu, &pipelines, ui_font_sizes);
    renderer.resolve_icons(&gpu, scale);
    Some((gpu, pipelines, renderer))
}

/// Render a dialog `Scene` to RGBA pixels via the headless UI-only pipeline.
///
/// Mirrors the production path in `dialog_rendering.rs`: `prepare_ui_frame`
/// → `resolve_icons` → `append_ui_scene_with_text` → `render_frame` →
/// pixel readback.
pub(super) fn render_dialog_to_pixels(
    gpu: &GpuState,
    pipelines: &GpuPipelines,
    renderer: &mut WindowRenderer,
    scene: &Scene,
    width: u32,
    height: u32,
    scale: f32,
) -> Vec<u8> {
    renderer.prepare_ui_frame(width, height, DIALOG_BG, 1.0);
    renderer.resolve_icons(gpu, scale);
    renderer.append_ui_scene_with_text(scene, scale, 1.0, gpu);
    let target = gpu.create_render_target(width, height);
    renderer.render_frame(gpu, pipelines, target.view());
    gpu.read_render_target(&target)
        .expect("pixel readback should succeed")
}
