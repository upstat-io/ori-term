//! Golden tests for the status bar widget.
//!
//! Verifies background, top border, left/right aligned text items, accent
//! coloring, gap spacing, and empty-field handling against reference PNGs.

#![cfg(all(test, feature = "gpu-tests"))]

use std::time::Instant;

use oriterm_ui::draw::Scene;
use oriterm_ui::geometry::Rect;
use oriterm_ui::theme::UiTheme;
use oriterm_ui::widgets::Widget;
use oriterm_ui::widgets::status_bar::{STATUS_BAR_HEIGHT, StatusBarData, StatusBarWidget};

use crate::font::shaper::CachedTextMeasurer;
use crate::font::ui_font_sizes::{PRELOAD_SIZES, UiFontSizes};
use crate::font::{FontCollection, FontSet, GlyphFormat, HintingMode, TextShapeCache};
use crate::gpu::pipelines::GpuPipelines;
use crate::gpu::state::GpuState;
use crate::gpu::window_renderer::WindowRenderer;

use super::compare_with_reference;

const WIDTH: u32 = 800;

/// Headless environment with UI font for status bar rendering.
fn headless_status_bar_env() -> Option<(GpuState, GpuPipelines, WindowRenderer)> {
    let gpu = GpuState::new_headless().ok()?;
    let pipelines = GpuPipelines::new(&gpu);
    let font_collection = FontCollection::new(
        FontSet::embedded(),
        14.0,
        96.0,
        GlyphFormat::Alpha,
        400,
        HintingMode::Full,
    )
    .ok()?;
    let ui_font_sizes = UiFontSizes::new(
        FontSet::ui_embedded(),
        96.0,
        GlyphFormat::Alpha,
        HintingMode::Full,
        400,
        &PRELOAD_SIZES,
    )
    .ok()?;
    let mut renderer = WindowRenderer::new(&gpu, &pipelines, font_collection, Some(ui_font_sizes));
    renderer.resolve_icons(&gpu, 1.0);
    Some((gpu, pipelines, renderer))
}

/// Paint a status bar with given data, render to pixels.
fn render_status_bar(
    gpu: &GpuState,
    pipelines: &GpuPipelines,
    renderer: &mut WindowRenderer,
    data: StatusBarData,
) -> Vec<u8> {
    let theme = UiTheme::dark();
    let mut widget = StatusBarWidget::new(WIDTH as f32, &theme);
    widget.set_data(data);

    let height = STATUS_BAR_HEIGHT.ceil() as u32;

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
        bounds: Rect::new(0.0, 0.0, WIDTH as f32, STATUS_BAR_HEIGHT),
        now: Instant::now(),
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    widget.paint(&mut ctx);

    let bg = oriterm_core::Rgb {
        r: 14,
        g: 14,
        b: 18,
    };
    renderer.prepare_ui_frame(WIDTH, height, bg, 1.0);
    renderer.resolve_icons(gpu, 1.0);
    renderer.append_ui_scene_with_text(&scene, 1.0, 1.0, gpu);
    let target = gpu.create_render_target(WIDTH, height);
    renderer.render_frame(gpu, pipelines, target.view());
    gpu.read_render_target(&target)
        .expect("pixel readback should succeed")
}

/// Full data: shell, panes, grid size, encoding, term type.
///
/// Verifies: background, top border, accent items (zsh, xterm-256color),
/// faint items (3 panes, 120x30, UTF-8), left/right alignment, gap spacing.
#[test]
fn status_bar_full_data_96dpi() {
    let Some((gpu, pipelines, mut renderer)) = headless_status_bar_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let height = STATUS_BAR_HEIGHT.ceil() as u32;
    let pixels = render_status_bar(
        &gpu,
        &pipelines,
        &mut renderer,
        StatusBarData {
            shell_name: "zsh".into(),
            pane_count: "3 panes".into(),
            grid_size: "120\u{00d7}30".into(),
            encoding: "UTF-8".into(),
            term_type: "xterm-256color".into(),
        },
    );

    if let Err(msg) = compare_with_reference("status_bar_full_data_96dpi", &pixels, WIDTH, height) {
        panic!("{msg}");
    }
}

/// Single pane: "1 pane" (singular) with different text length.
#[test]
fn status_bar_single_pane_96dpi() {
    let Some((gpu, pipelines, mut renderer)) = headless_status_bar_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let height = STATUS_BAR_HEIGHT.ceil() as u32;
    let pixels = render_status_bar(
        &gpu,
        &pipelines,
        &mut renderer,
        StatusBarData {
            shell_name: "bash".into(),
            pane_count: "1 pane".into(),
            grid_size: "80\u{00d7}24".into(),
            encoding: "UTF-8".into(),
            term_type: "xterm-256color".into(),
        },
    );

    if let Err(msg) = compare_with_reference("status_bar_single_pane_96dpi", &pixels, WIDTH, height)
    {
        panic!("{msg}");
    }
}

/// Empty items: some fields are empty, verifying no spurious gaps or artifacts.
#[test]
fn status_bar_empty_items_96dpi() {
    let Some((gpu, pipelines, mut renderer)) = headless_status_bar_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let height = STATUS_BAR_HEIGHT.ceil() as u32;
    let pixels = render_status_bar(
        &gpu,
        &pipelines,
        &mut renderer,
        StatusBarData {
            shell_name: "zsh".into(),
            pane_count: String::new(),
            grid_size: "120\u{00d7}30".into(),
            encoding: String::new(),
            term_type: String::new(),
        },
    );

    if let Err(msg) = compare_with_reference("status_bar_empty_items_96dpi", &pixels, WIDTH, height)
    {
        panic!("{msg}");
    }
}
