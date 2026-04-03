//! Composed golden tests for the full main window chrome.
//!
//! Renders tab bar + terminal grid + status bar + window border into a single
//! frame, verifying that the complete window composition matches reference PNGs.

#![cfg(all(test, feature = "gpu-tests"))]

use std::time::Instant;

use oriterm_ui::draw::Scene;
use oriterm_ui::geometry::Rect;
use oriterm_ui::theme::UiTheme;
use oriterm_ui::widgets::Widget;
use oriterm_ui::widgets::status_bar::{STATUS_BAR_HEIGHT, StatusBarData, StatusBarWidget};
use oriterm_ui::widgets::tab_bar::{TabBarWidget, TabEntry};

use crate::app::compute_window_layout;
use crate::font::shaper::CachedTextMeasurer;
use crate::font::ui_font_sizes::{PRELOAD_SIZES, UiFontSizes};
use crate::font::{FontCollection, FontSet, GlyphFormat, HintingMode, TextShapeCache};
use crate::gpu::frame_input::{FrameInput, ViewportSize};
use crate::gpu::pipelines::GpuPipelines;
use crate::gpu::scene_convert::color_to_rgb;
use crate::gpu::state::GpuState;
use crate::gpu::window_renderer::WindowRenderer;

use super::compare_with_reference;

/// Headless environment with both terminal and UI fonts for composed rendering.
fn headless_composed_env() -> Option<(GpuState, GpuPipelines, WindowRenderer)> {
    let gpu = GpuState::new_headless().ok()?;
    let pipelines = GpuPipelines::new(&gpu);
    let font_collection = FontCollection::new(
        FontSet::embedded(),
        12.0,
        96.0,
        GlyphFormat::Alpha,
        400,
        550,
        HintingMode::Full,
    )
    .ok()?;
    let ui_font_sizes = UiFontSizes::new(
        FontSet::ui_embedded(),
        96.0,
        GlyphFormat::Alpha,
        HintingMode::Full,
        400,
        550,
        &PRELOAD_SIZES,
    )
    .ok()?;
    let mut renderer = WindowRenderer::new(&gpu, &pipelines, font_collection, Some(ui_font_sizes));
    renderer.resolve_icons(&gpu, 1.0);
    Some((gpu, pipelines, renderer))
}

/// Headless environment at 192 DPI for high-DPI composed rendering tests.
fn headless_composed_env_192dpi() -> Option<(GpuState, GpuPipelines, WindowRenderer)> {
    let gpu = GpuState::new_headless().ok()?;
    let pipelines = GpuPipelines::new(&gpu);
    let font_collection = FontCollection::new(
        FontSet::embedded(),
        12.0,
        192.0,
        GlyphFormat::Alpha,
        400,
        550,
        HintingMode::Full,
    )
    .ok()?;
    let ui_font_sizes = UiFontSizes::new(
        FontSet::ui_embedded(),
        192.0,
        GlyphFormat::Alpha,
        HintingMode::Full,
        400,
        550,
        &PRELOAD_SIZES,
    )
    .ok()?;
    let mut renderer = WindowRenderer::new(&gpu, &pipelines, font_collection, Some(ui_font_sizes));
    renderer.resolve_icons(&gpu, 2.0);
    Some((gpu, pipelines, renderer))
}

/// Render a composed main window frame: tab bar + grid + status bar + border.
///
/// Returns RGBA pixel buffer at the given width and height.
#[expect(
    clippy::too_many_arguments,
    reason = "composed rendering: GPU, pipelines, renderer, tabs, status, grid text, size, scale, options"
)]
fn render_main_window(
    gpu: &GpuState,
    pipelines: &GpuPipelines,
    renderer: &mut WindowRenderer,
    tabs: &[TabEntry],
    active_tab: usize,
    status_data: StatusBarData,
    grid_text: &str,
    width: u32,
    height: u32,
    scale: f32,
    show_status_bar: bool,
    show_tab_bar: bool,
    show_border: bool,
) -> Vec<u8> {
    let theme = UiTheme::dark();
    let cell = renderer.cell_metrics();
    let tab_bar_h = if show_tab_bar { 36.0 } else { 0.0 };
    let sb_h = if show_status_bar {
        STATUS_BAR_HEIGHT
    } else {
        0.0
    };
    let border_inset = if show_border { 2.0 } else { 0.0 };

    let wl = compute_window_layout(
        width,
        height,
        &cell,
        scale,
        !show_tab_bar,
        tab_bar_h,
        sb_h,
        border_inset,
    );

    // Build grid content.
    let mut input = FrameInput::test_grid(wl.cols, wl.rows, grid_text);
    input.viewport = ViewportSize::new(width, height);
    input.cell_size = cell;
    input.content.cursor.visible = false;

    // Prepare grid (fills instance buffers, clears, begins atlas frame).
    let origin = (wl.grid_rect.x(), wl.grid_rect.y());
    renderer.prepare(&input, gpu, pipelines, origin, 1.0, true);

    let text_cache = TextShapeCache::new();

    // Paint tab bar.
    if show_tab_bar && !tabs.is_empty() {
        let mut tab_bar = TabBarWidget::with_theme(wl.tab_bar_rect.width() / scale, &theme);
        tab_bar.set_tabs(tabs.to_vec());
        tab_bar.set_active_index(active_tab);

        let measurer = renderer.ui_measurer(scale);
        let cached = CachedTextMeasurer::new(measurer, &text_cache, scale);
        let icons = renderer.resolved_icons();
        let bounds = Rect::new(
            wl.tab_bar_rect.x() / scale,
            wl.tab_bar_rect.y() / scale,
            wl.tab_bar_rect.width() / scale,
            wl.tab_bar_rect.height() / scale,
        );
        let mut scene = Scene::new();
        let mut ctx = oriterm_ui::widgets::DrawCtx {
            scene: &mut scene,
            theme: &theme,
            measurer: &cached,
            icons: Some(icons),
            bounds,
            now: Instant::now(),
            interaction: None,
            widget_id: None,
            frame_requests: None,
        };
        tab_bar.paint(&mut ctx);
        renderer.append_ui_scene_with_text(&scene, scale, 1.0, gpu);
    }

    // Paint status bar.
    if show_status_bar {
        let mut status_bar = StatusBarWidget::new(wl.status_bar_rect.width() / scale, &theme);
        status_bar.set_data(status_data);

        let measurer = renderer.ui_measurer(scale);
        let cached = CachedTextMeasurer::new(measurer, &text_cache, scale);
        let bounds = Rect::new(
            wl.status_bar_rect.x() / scale,
            wl.status_bar_rect.y() / scale,
            wl.status_bar_rect.width() / scale,
            wl.status_bar_rect.height() / scale,
        );
        let mut scene = Scene::new();
        let mut ctx = oriterm_ui::widgets::DrawCtx {
            scene: &mut scene,
            theme: &theme,
            measurer: &cached,
            icons: None,
            bounds,
            now: Instant::now(),
            interaction: None,
            widget_id: None,
            frame_requests: None,
        };
        status_bar.paint(&mut ctx);
        renderer.append_ui_scene_with_text(&scene, scale, 1.0, gpu);
    }

    // Paint window border.
    if show_border {
        let border_color = color_to_rgb(theme.border_strong);
        renderer.append_window_border(width, height, border_color, (2.0 * scale).round());
    }

    // Render and readback.
    let target = gpu.create_render_target(width, height);
    renderer.render_frame(gpu, pipelines, target.view());
    gpu.read_render_target(&target)
        .expect("pixel readback should succeed")
}

/// Standard status bar data for tests.
fn test_status_data() -> StatusBarData {
    StatusBarData {
        shell_name: "zsh".into(),
        pane_count: "1 pane".into(),
        grid_size: "80\u{00d7}24".into(),
        encoding: "UTF-8".into(),
        term_type: "xterm-256color".into(),
    }
}

/// Simple test grid text (printable ASCII pattern).
fn test_grid_text(cols: usize, rows: usize) -> String {
    (0..(cols * rows))
        .map(|i| {
            let ch = b' ' + (i % 95) as u8;
            ch as char
        })
        .collect()
}

// -- Tests --

#[test]
fn main_window_single_pane_96dpi() {
    let Some((gpu, pipelines, mut renderer)) = headless_composed_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let width = 800u32;
    let height = 600u32;
    let text = test_grid_text(80, 30);
    let tabs = vec![TabEntry::new("zsh")];

    let pixels = render_main_window(
        &gpu,
        &pipelines,
        &mut renderer,
        &tabs,
        0,
        test_status_data(),
        &text,
        width,
        height,
        1.0,
        true,
        true,
        true,
    );

    if let Err(msg) =
        compare_with_reference("main_window_single_pane_96dpi", &pixels, width, height)
    {
        panic!("{msg}");
    }
}

#[test]
fn main_window_3tabs_96dpi() {
    let Some((gpu, pipelines, mut renderer)) = headless_composed_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let width = 800u32;
    let height = 600u32;
    let text = test_grid_text(80, 30);
    let tabs = vec![
        TabEntry::new("zsh"),
        TabEntry::new("nvim").with_modified(true),
        TabEntry::new("htop"),
    ];

    let pixels = render_main_window(
        &gpu,
        &pipelines,
        &mut renderer,
        &tabs,
        0,
        StatusBarData {
            shell_name: "zsh".into(),
            pane_count: "3 panes".into(),
            grid_size: "80\u{00d7}24".into(),
            encoding: "UTF-8".into(),
            term_type: "xterm-256color".into(),
        },
        &text,
        width,
        height,
        1.0,
        true,
        true,
        true,
    );

    if let Err(msg) = compare_with_reference("main_window_3tabs_96dpi", &pixels, width, height) {
        panic!("{msg}");
    }
}

#[test]
fn main_window_192dpi() {
    let Some((gpu, pipelines, mut renderer)) = headless_composed_env_192dpi() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let width = 1600u32;
    let height = 1200u32;
    let text = test_grid_text(80, 30);
    let tabs = vec![TabEntry::new("zsh")];

    let pixels = render_main_window(
        &gpu,
        &pipelines,
        &mut renderer,
        &tabs,
        0,
        test_status_data(),
        &text,
        width,
        height,
        2.0,
        true,
        true,
        true,
    );

    if let Err(msg) = compare_with_reference("main_window_192dpi", &pixels, width, height) {
        panic!("{msg}");
    }
}

#[test]
fn main_window_no_status_bar_96dpi() {
    let Some((gpu, pipelines, mut renderer)) = headless_composed_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let width = 800u32;
    let height = 600u32;
    let text = test_grid_text(80, 30);
    let tabs = vec![TabEntry::new("zsh")];

    let pixels = render_main_window(
        &gpu,
        &pipelines,
        &mut renderer,
        &tabs,
        0,
        StatusBarData::default(),
        &text,
        width,
        height,
        1.0,
        false,
        true,
        true,
    );

    if let Err(msg) =
        compare_with_reference("main_window_no_status_bar_96dpi", &pixels, width, height)
    {
        panic!("{msg}");
    }
}

#[test]
fn main_window_hidden_tab_bar_96dpi() {
    let Some((gpu, pipelines, mut renderer)) = headless_composed_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let width = 800u32;
    let height = 600u32;
    let text = test_grid_text(80, 30);

    let pixels = render_main_window(
        &gpu,
        &pipelines,
        &mut renderer,
        &[],
        0,
        test_status_data(),
        &text,
        width,
        height,
        1.0,
        true,
        false,
        true,
    );

    if let Err(msg) =
        compare_with_reference("main_window_hidden_tab_bar_96dpi", &pixels, width, height)
    {
        panic!("{msg}");
    }
}
