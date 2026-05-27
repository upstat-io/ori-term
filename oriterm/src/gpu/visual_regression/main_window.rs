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
use crate::font::{FontSet, HintingMode, TextShapeCache};
use crate::gpu::frame_input::{FrameInput, ViewportSize};
use crate::gpu::pipelines::GpuPipelines;
use crate::gpu::scene_convert::color_to_rgb;
use crate::gpu::state::GpuState;
use crate::gpu::window_renderer::WindowRenderer;

use super::{HeadlessEnvConfig, UiFontConfig, compare_with_reference, headless_env_with};

/// Headless environment with both terminal and UI fonts for composed rendering.
fn headless_composed_env() -> Option<(GpuState, GpuPipelines, WindowRenderer)> {
    headless_env_with(&HeadlessEnvConfig {
        ui: Some(UiFontConfig {
            font_set: FontSet::ui_embedded(),
            hinting: HintingMode::Full,
        }),
        resolve_icons_scale: Some(1.0),
        ..Default::default()
    })
}

/// Headless environment at 192 DPI for high-DPI composed rendering tests.
fn headless_composed_env_192dpi() -> Option<(GpuState, GpuPipelines, WindowRenderer)> {
    headless_env_with(&HeadlessEnvConfig {
        dpi: 192.0,
        ui: Some(UiFontConfig {
            font_set: FontSet::ui_embedded(),
            hinting: HintingMode::Full,
        }),
        resolve_icons_scale: Some(2.0),
        ..Default::default()
    })
}

/// Composed-frame scene description for [`render_main_window`]: tab/status
/// content, grid text, viewport dimensions, scale, and chrome visibility.
struct MainWindowScene<'a> {
    tabs: &'a [TabEntry],
    active_tab: usize,
    status_data: StatusBarData,
    grid_text: &'a str,
    width: u32,
    height: u32,
    scale: f32,
    show_status_bar: bool,
    show_tab_bar: bool,
    show_border: bool,
}

/// Render a composed main window frame: tab bar + grid + status bar + border.
///
/// Returns RGBA pixel buffer at the given width and height.
fn render_main_window(
    gpu: &GpuState,
    pipelines: &GpuPipelines,
    renderer: &mut WindowRenderer,
    scene: MainWindowScene<'_>,
) -> Vec<u8> {
    let MainWindowScene {
        tabs,
        active_tab,
        status_data,
        grid_text,
        width,
        height,
        scale,
        show_status_bar,
        show_tab_bar,
        show_border,
    } = scene;
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
        crate::app::ChromeLayout {
            tab_bar_hidden: !show_tab_bar,
            tab_bar_height: tab_bar_h,
            status_bar_height: sb_h,
            border_inset,
        },
    );

    // Build grid content.
    let mut input = FrameInput::test_grid(wl.cols, wl.rows, grid_text);
    input.viewport = ViewportSize::new(width, height);
    input.cell_size = cell;
    input.content.cursor.visible = false;

    // Prepare grid (fills instance buffers, clears, begins atlas frame).
    let origin = (wl.grid_rect.x(), wl.grid_rect.y());
    renderer.prepare(
        &input,
        gpu,
        pipelines,
        crate::gpu::PrepareRequest {
            origin: origin,
            cursor_opacity: 1.0,
            content_changed: true,
        },
    );

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
        let border_width = crate::gpu::window_renderer::physical_border_width(scale);
        renderer.append_window_border(width, height, border_color, border_width);
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
        MainWindowScene {
            tabs: &tabs,
            active_tab: 0,
            status_data: test_status_data(),
            grid_text: &text,
            width,
            height,
            scale: 1.0,
            show_status_bar: true,
            show_tab_bar: true,
            show_border: true,
        },
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
        MainWindowScene {
            tabs: &tabs,
            active_tab: 0,
            status_data: StatusBarData {
                shell_name: "zsh".into(),
                pane_count: "3 panes".into(),
                grid_size: "80\u{00d7}24".into(),
                encoding: "UTF-8".into(),
                term_type: "xterm-256color".into(),
            },
            grid_text: &text,
            width,
            height,
            scale: 1.0,
            show_status_bar: true,
            show_tab_bar: true,
            show_border: true,
        },
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
        MainWindowScene {
            tabs: &tabs,
            active_tab: 0,
            status_data: test_status_data(),
            grid_text: &text,
            width,
            height,
            scale: 2.0,
            show_status_bar: true,
            show_tab_bar: true,
            show_border: true,
        },
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
        MainWindowScene {
            tabs: &tabs,
            active_tab: 0,
            status_data: StatusBarData::default(),
            grid_text: &text,
            width,
            height,
            scale: 1.0,
            show_status_bar: false,
            show_tab_bar: true,
            show_border: true,
        },
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
        MainWindowScene {
            tabs: &[],
            active_tab: 0,
            status_data: test_status_data(),
            grid_text: &text,
            width,
            height,
            scale: 1.0,
            show_status_bar: true,
            show_tab_bar: false,
            show_border: true,
        },
    );

    if let Err(msg) =
        compare_with_reference("main_window_hidden_tab_bar_96dpi", &pixels, width, height)
    {
        panic!("{msg}");
    }
}
