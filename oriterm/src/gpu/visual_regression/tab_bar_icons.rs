//! Golden tests for tab bar emoji icon rendering.
//!
//! Verifies that emoji icons from OSC 0/1 render visibly in the tab bar
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

/// Create a headless environment with terminal font (has emoji fallback).
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

/// Render a tab bar Scene to pixels.
fn render_tab_bar_to_pixels(
    gpu: &GpuState,
    pipelines: &GpuPipelines,
    renderer: &mut WindowRenderer,
    scene: &Scene,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let bg = oriterm_core::Rgb {
        r: 30,
        g: 30,
        b: 30,
    };
    renderer.prepare_ui_frame(width, height, bg, 1.0);
    renderer.resolve_icons(gpu, 1.0);
    renderer.append_ui_scene_with_text(scene, 1.0, 1.0, gpu);
    let target = gpu.create_render_target(width, height);
    renderer.render_frame(gpu, pipelines, target.view());
    gpu.read_render_target(&target)
        .expect("pixel readback should succeed")
}

/// Build and paint a tab bar with emoji icons into a Scene.
fn paint_tab_bar_with_emoji(renderer: &WindowRenderer) -> Scene {
    let theme = UiTheme::dark();
    let mut tab_bar = TabBarWidget::with_theme(400.0, &theme);
    tab_bar.set_tabs(vec![
        TabEntry::new("Python").with_icon(Some(TabIcon::Emoji("🐍".to_owned()))),
        TabEntry::new("Fire").with_icon(Some(TabIcon::Emoji("🔥".to_owned()))),
        TabEntry::new("No Icon"),
    ]);
    tab_bar.set_active_index(0);

    let measurer = renderer.ui_measurer(1.0);
    let text_cache = TextShapeCache::new();
    let cached = CachedTextMeasurer::new(measurer, &text_cache, 1.0);

    let icons = renderer.resolved_icons();
    let mut scene = Scene::new();
    let now = Instant::now();
    let mut ctx = oriterm_ui::widgets::DrawCtx {
        scene: &mut scene,
        theme: &theme,
        measurer: &cached,
        icons: Some(icons),
        bounds: Rect::new(0.0, 0.0, 400.0, 46.0),
        now,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };

    // Paint via Widget trait (available from oriterm crate).
    tab_bar.paint(&mut ctx);
    scene
}

/// Check if a rectangular region has any non-background pixels.
fn region_has_content(
    pixels: &[u8],
    img_width: u32,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    threshold: u8,
) -> bool {
    for row in y..y.saturating_add(h) {
        for col in x..x.saturating_add(w) {
            let idx = ((row * img_width + col) * 4) as usize;
            if idx + 2 >= pixels.len() {
                continue;
            }
            let dr = pixels[idx].abs_diff(30);
            let dg = pixels[idx + 1].abs_diff(30);
            let db = pixels[idx + 2].abs_diff(30);
            if dr > threshold || dg > threshold || db > threshold {
                return true;
            }
        }
    }
    false
}

#[test]
fn tab_bar_emoji_icon_renders_visible_pixels() {
    let Some((gpu, pipelines, mut renderer)) = headless_tab_bar_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let scene = paint_tab_bar_with_emoji(&renderer);
    let width = 400;
    let height = 50;
    let pixels = render_tab_bar_to_pixels(&gpu, &pipelines, &mut renderer, &scene, width, height);

    // The emoji icon should render in the first tab's icon area.
    // Tab starts at TAB_LEFT_MARGIN (16px), icon at +TAB_PADDING (8px).
    // Check a generous region covering the icon area of tab 1.
    let has_icon = region_has_content(&pixels, width, 16, 2, 36, 44, 10);

    assert!(
        has_icon,
        "Emoji icon 🐍 should render visible pixels in the tab bar icon area. \
         The terminal font's color emoji fallback chain must be used for \
         tab icon shaping and rasterization."
    );
}
