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
use oriterm_ui::widgets::tab_bar::constants::TAB_BAR_HEIGHT;
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
        550,
        HintingMode::Full,
    )
    .ok()?;
    let ui_font_sizes = crate::font::ui_font_sizes::UiFontSizes::new(
        FontSet::ui_embedded(),
        96.0,
        GlyphFormat::Alpha,
        HintingMode::Full,
        400,
        550,
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
        bounds: Rect::new(0.0, 0.0, WIDTH as f32, TAB_BAR_HEIGHT),
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
        550,
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

/// Detect clipping on any edge of rendered color emoji.
///
/// Color emoji are pre-dithered — every outermost edge should fade to the
/// background via anti-aliasing. A hard cut (many opaque/saturated pixels
/// on the bounding edge) means the rendering pipeline clips the glyph.
///
/// Tests the FULL GPU pipeline (scene → clip rects → shaders → pixel output)
/// not just the raw rasterized bitmap.
#[test]
fn emoji_not_clipped_in_rendered_output() {
    let Some((gpu, pipelines, mut renderer)) = headless_tab_bar_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    // Render each emoji alone so we can isolate its bounding box.
    for (label, emoji) in [("smiley", "😀"), ("snake", "🐍"), ("fire", "🔥")] {
        let pixels = render_tab_bar(
            &gpu,
            &pipelines,
            &mut renderer,
            vec![TabEntry::new("").with_icon(Some(TabIcon::Emoji(emoji.to_owned())))],
        );

        // Find the emoji's bounding box by looking for saturated (colorful) pixels.
        // Skip the top accent bar (2px) and bottom border area (2px) since those
        // are tab bar decorations that contain colored pixels unrelated to emoji.
        let accent_h = TAB_BAR_HEIGHT.ceil() as usize;
        let bbox = find_color_bbox_inset(&pixels, WIDTH as usize, HEIGHT as usize, 3, accent_h);
        let Some((min_x, min_y, max_x, max_y)) = bbox else {
            panic!("{label} ({emoji}): no color pixels found in rendered output");
        };
        let ew = max_x - min_x + 1;
        let eh = max_y - min_y + 1;
        assert!(
            ew > 4 && eh > 4,
            "{label}: emoji region too small: {ew}x{eh}"
        );

        // Check all 4 edges for hard clip lines.
        let edges = [
            (
                "top",
                scan_edge(&pixels, WIDTH as usize, min_x, max_x, min_y, true),
            ),
            (
                "bottom",
                scan_edge(&pixels, WIDTH as usize, min_x, max_x, max_y, true),
            ),
            (
                "left",
                scan_edge(&pixels, WIDTH as usize, min_y, max_y, min_x, false),
            ),
            (
                "right",
                scan_edge(&pixels, WIDTH as usize, min_y, max_y, max_x, false),
            ),
        ];

        eprintln!("{label} ({emoji}) bbox: ({min_x},{min_y})-({max_x},{max_y}) = {ew}x{eh}");
        for (edge_name, fraction) in &edges {
            eprintln!("  {edge_name}: {:.0}%", fraction * 100.0);
        }
        for (edge_name, fraction) in &edges {
            assert!(
                *fraction < 0.9,
                "{label} ({emoji}): {edge_name} edge is {:.0}% hard — clip detected. \
                 The rendering pipeline is truncating the emoji on the {edge_name}.",
                fraction * 100.0,
            );
        }
    }
}

/// Find bounding box of color (saturated) pixels with y insets.
///
/// Skips the top `y_top` and bottom rows past `y_bottom` to exclude
/// tab bar decorations (accent bar, bottom border) from emoji detection.
fn find_color_bbox_inset(
    pixels: &[u8],
    w: usize,
    h: usize,
    y_top: usize,
    y_bottom: usize,
) -> Option<(usize, usize, usize, usize)> {
    let mut min_x = w;
    let mut max_x = 0;
    let mut min_y = h;
    let mut max_y = 0;
    let mut found = false;
    for y in y_top..y_bottom.min(h) {
        for x in 0..w {
            let idx = (y * w + x) * 4;
            if idx + 3 >= pixels.len() {
                continue;
            }
            let r = pixels[idx];
            let g = pixels[idx + 1];
            let b = pixels[idx + 2];
            let max_ch = r.max(g).max(b);
            let min_ch = r.min(g).min(b);
            // Color saturation: difference between brightest and dimmest channel.
            // Gray text (R≈G≈B) and dark background have saturation < 30.
            if max_ch.saturating_sub(min_ch) > 30 && max_ch > 50 {
                found = true;
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }
        }
    }
    if found {
        Some((min_x, min_y, max_x, max_y))
    } else {
        None
    }
}

/// Measure what fraction of an edge has hard (non-anti-aliased) content.
///
/// A pixel counts as "hard" if its alpha is > 200 AND it has color saturation > 20.
/// If `horizontal`, scans x in `start..=end` at row `fixed`.
/// If not horizontal, scans y in `start..=end` at column `fixed`.
fn scan_edge(
    pixels: &[u8],
    img_w: usize,
    start: usize,
    end: usize,
    fixed: usize,
    horizontal: bool,
) -> f32 {
    let mut hard = 0u32;
    let mut total = 0u32;
    for i in start..=end {
        let (x, y) = if horizontal { (i, fixed) } else { (fixed, i) };
        let idx = (y * img_w + x) * 4;
        if idx + 3 >= pixels.len() {
            continue;
        }
        total += 1;
        let r = pixels[idx];
        let g = pixels[idx + 1];
        let b = pixels[idx + 2];
        let a = pixels[idx + 3];
        let max_ch = r.max(g).max(b);
        let min_ch = r.min(g).min(b);
        let sat = max_ch.saturating_sub(min_ch);
        // Hard pixel: high alpha AND colorful (not just gray anti-alias fringe).
        if a > 200 && sat > 20 {
            hard += 1;
        }
    }
    if total == 0 {
        return 0.0;
    }
    hard as f32 / total as f32
}
