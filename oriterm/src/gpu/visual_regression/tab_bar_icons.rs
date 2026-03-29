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
use crate::font::{FontCollection, FontSet, GlyphFormat, GlyphStyle, HintingMode, TextShapeCache};
use crate::gpu::pipelines::GpuPipelines;
use crate::gpu::state::GpuState;
use crate::gpu::window_renderer::WindowRenderer;

use super::compare_with_reference;

const WIDTH: u32 = 600;
const HEIGHT: u32 = 60;

/// Path to Segoe UI Emoji on Windows (accessible from WSL via /mnt/c).
const SEGOE_EMOJI_PATH: &str = "/mnt/c/Windows/Fonts/seguiemj.ttf";

/// Build a FontSet with the best available emoji fallback.
///
/// Prefers Segoe UI Emoji (COLRv1, Windows-native quality) if accessible
/// from WSL. Falls back to NotoEmoji-Regular (embedded, monochrome outlines).
fn font_set_with_best_emoji() -> FontSet {
    FontSet::embedded().with_system_emoji_fallback(SEGOE_EMOJI_PATH)
}

/// Headless environment with best available emoji font.
fn headless_tab_bar_env() -> Option<(GpuState, GpuPipelines, WindowRenderer)> {
    let gpu = GpuState::new_headless().ok()?;
    let pipelines = GpuPipelines::new(&gpu);
    let font_collection = FontCollection::new(
        font_set_with_best_emoji(),
        14.0,
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

/// Verify emoji resolves to a fallback face.
#[test]
fn embedded_font_resolves_emoji_glyph() {
    let fc = FontCollection::new(
        font_set_with_best_emoji(),
        14.0,
        96.0,
        GlyphFormat::Alpha,
        400,
        HintingMode::Full,
    )
    .expect("collection should build");
    let resolved = fc.resolve_prefer_emoji('😀', GlyphStyle::Regular);
    assert_ne!(
        resolved.glyph_id, 0,
        "😀 must resolve to a non-zero glyph_id via the emoji fallback font."
    );
    assert!(
        resolved.face_idx.is_fallback(),
        "emoji should resolve via fallback face (got face_idx={:?})",
        resolved.face_idx,
    );
}

/// Golden test: render emoji tabs and compare against reference PNG.
#[test]
fn tab_bar_emoji_golden() {
    let Some((gpu, pipelines, mut renderer)) = headless_tab_bar_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let pixels = render_tab_bar(
        &gpu,
        &pipelines,
        &mut renderer,
        vec![
            TabEntry::new("Snake").with_icon(Some(TabIcon::Emoji("🐍".to_owned()))),
            TabEntry::new("Fire").with_icon(Some(TabIcon::Emoji("🔥".to_owned()))),
            TabEntry::new("Smile").with_icon(Some(TabIcon::Emoji("😀".to_owned()))),
        ],
    );

    if let Err(msg) = compare_with_reference("tab_bar_emoji", &pixels, WIDTH, HEIGHT) {
        panic!("{msg}");
    }
}

/// Differential test: emoji icon must produce different pixels than no icon.
#[test]
fn emoji_icon_produces_visible_pixels() {
    let Some((gpu, pipelines, mut renderer)) = headless_tab_bar_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let with_emoji = render_tab_bar(
        &gpu,
        &pipelines,
        &mut renderer,
        vec![TabEntry::new("").with_icon(Some(TabIcon::Emoji("😀".to_owned())))],
    );

    let without_emoji = render_tab_bar(&gpu, &pipelines, &mut renderer, vec![TabEntry::new("")]);

    let diff_count = with_emoji
        .chunks(4)
        .zip(without_emoji.chunks(4))
        .filter(|(a, b)| {
            a[0].abs_diff(b[0]) > 2 || a[1].abs_diff(b[1]) > 2 || a[2].abs_diff(b[2]) > 2
        })
        .count();

    assert!(
        diff_count > 20,
        "Emoji 😀 must produce visible glyph pixels in the tab bar. \
         Only {diff_count} pixels differed. The emoji fallback font must be \
         loaded and the glyph must rasterize through the terminal font collection."
    );
}
