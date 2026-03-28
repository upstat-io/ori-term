//! Golden tests for tab bar emoji icon rendering.
//!
//! Verifies that emoji icons from OSC 0/1 actually rasterize visible pixels
//! through the terminal font's color emoji fallback chain.

#![cfg(all(test, feature = "gpu-tests"))]

use std::time::Instant;

use oriterm_ui::draw::Scene;
use oriterm_ui::geometry::Rect;
use oriterm_ui::theme::UiTheme;
use oriterm_ui::widgets::Widget;
use oriterm_ui::widgets::tab_bar::TabBarWidget;
use oriterm_ui::widgets::tab_bar::widget::{TabEntry, TabIcon};

use crate::font::shaper::CachedTextMeasurer;
use crate::font::{FontCollection, FontSet, GlyphFormat, HintingMode, TextShapeCache};
use crate::gpu::pipelines::GpuPipelines;
use crate::gpu::state::GpuState;
use crate::gpu::window_renderer::WindowRenderer;

const WIDTH: u32 = 400;
const HEIGHT: u32 = 50;

/// Headless environment with terminal font (emoji fallback) + UI font.
fn headless_tab_bar_env() -> Option<(GpuState, GpuPipelines, WindowRenderer)> {
    let gpu = GpuState::new_headless().ok()?;
    let pipelines = GpuPipelines::new(&gpu);
    let font_collection = FontCollection::new(
        FontSet::embedded(),
        12.0,
        96.0,
        GlyphFormat::Alpha,
        400,
        HintingMode::Full,
    )
    .ok()?;
    let ui_font_sizes = crate::font::ui_font_sizes::UiFontSizes::new(
        FontSet::ui_embedded(),
        96.0,
        GlyphFormat::Alpha,
        HintingMode::Full,
        400,
        &crate::font::ui_font_sizes::PRELOAD_SIZES,
    )
    .ok()?;
    let mut renderer = WindowRenderer::new(&gpu, &pipelines, font_collection, Some(ui_font_sizes));
    renderer.resolve_icons(&gpu, 1.0);
    Some((gpu, pipelines, renderer))
}

/// Paint a tab bar and render to pixels.
fn render_tab_bar(
    gpu: &GpuState,
    pipelines: &GpuPipelines,
    renderer: &mut WindowRenderer,
    entries: Vec<TabEntry>,
) -> Vec<u8> {
    let theme = UiTheme::dark();
    let mut tab_bar = TabBarWidget::with_theme(WIDTH as f32, &theme);
    tab_bar.set_tabs(entries);
    tab_bar.set_active_index(0);

    let measurer = renderer.ui_measurer(1.0);
    let text_cache = TextShapeCache::new();
    let cached = CachedTextMeasurer::new(measurer, &text_cache, 1.0);
    let icons = renderer.resolved_icons();

    let mut scene = Scene::new();
    let mut ctx = oriterm_ui::widgets::DrawCtx {
        scene: &mut scene,
        theme: &theme,
        measurer: &cached,
        icons: Some(icons),
        bounds: Rect::new(0.0, 0.0, WIDTH as f32, 46.0),
        now: Instant::now(),
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    tab_bar.paint(&mut ctx);

    let bg = oriterm_core::Rgb {
        r: 30,
        g: 30,
        b: 30,
    };
    renderer.prepare_ui_frame(WIDTH, HEIGHT, bg, 1.0);
    renderer.resolve_icons(gpu, 1.0);
    renderer.append_ui_scene_with_text(&scene, 1.0, 1.0, gpu);
    let target = gpu.create_render_target(WIDTH, HEIGHT);
    renderer.render_frame(gpu, pipelines, target.view());
    gpu.read_render_target(&target)
        .expect("pixel readback should succeed")
}

/// Count pixels that have color saturation (not just white/gray on dark bg).
///
/// Color emoji have hue (green 🐍, orange 🔥). Regular mono text is
/// white/gray (R≈G≈B). A pixel has saturation when its max channel
/// differs from its min channel significantly.
fn count_saturated_pixels(pixels: &[u8], threshold: u8) -> usize {
    pixels
        .chunks(4)
        .filter(|px| {
            let r = px[0];
            let g = px[1];
            let b = px[2];
            let max = r.max(g).max(b);
            let min = r.min(g).min(b);
            // Must be above background and have color variation.
            max > 40 && (max - min) > threshold
        })
        .count()
}

#[test]
fn emoji_icon_produces_saturated_color_pixels() {
    let Some((gpu, pipelines, mut renderer)) = headless_tab_bar_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    // Tab with ONLY emoji icon, empty title — any color pixels must be from the emoji.
    let with_emoji = render_tab_bar(
        &gpu,
        &pipelines,
        &mut renderer,
        vec![TabEntry::new("").with_icon(Some(TabIcon::Emoji("🐍".to_owned())))],
    );

    // Tab with NO icon, empty title — baseline (should have zero saturation).
    let without_emoji = render_tab_bar(&gpu, &pipelines, &mut renderer, vec![TabEntry::new("")]);

    let sat_with = count_saturated_pixels(&with_emoji, 20);
    let sat_without = count_saturated_pixels(&without_emoji, 20);

    // The emoji tab must have MORE saturated pixels than the empty tab.
    // Color emoji produce green/brown/etc pixels that have hue.
    // If both are zero, the emoji didn't rasterize as a color glyph.
    assert!(
        sat_with > sat_without + 10,
        "Emoji 🐍 must produce color (saturated) pixels in the tab bar. \
         with_emoji={sat_with} saturated pixels, without_emoji={sat_without}. \
         Expected at least 10 more saturated pixels with emoji. \
         This means the emoji is NOT rendering through the color emoji font."
    );
}
