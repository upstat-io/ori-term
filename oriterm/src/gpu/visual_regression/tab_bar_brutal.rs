//! Golden tests for the brutal tab bar design.
//!
//! Verifies flat tabs, accent bar, bottom border, modified dot, separators,
//! close button opacity, and action button borders against reference PNGs.

#![cfg(all(test, feature = "gpu-tests"))]

use std::time::Instant;

use oriterm_ui::draw::Scene;
use oriterm_ui::geometry::Rect;
use oriterm_ui::theme::UiTheme;
use oriterm_ui::widgets::Widget;
use oriterm_ui::widgets::tab_bar::constants::TAB_BAR_HEIGHT;
use oriterm_ui::widgets::tab_bar::{TabBarHit, TabBarWidget, TabEntry};

use crate::font::shaper::CachedTextMeasurer;
use crate::font::ui_font_sizes::{PRELOAD_SIZES, UiFontSizes};
use crate::font::{FontCollection, FontSet, GlyphFormat, HintingMode, TextShapeCache};
use crate::gpu::pipelines::GpuPipelines;
use crate::gpu::state::GpuState;
use crate::gpu::window_renderer::WindowRenderer;

use super::compare_with_reference;

const WIDTH: u32 = 600;

/// Headless environment with UI font (no emoji needed for brutal tests).
fn headless_brutal_env() -> Option<(GpuState, GpuPipelines, WindowRenderer)> {
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

/// Paint a tab bar with given entries and hover state, render to pixels.
fn render_tab_bar_brutal(
    gpu: &GpuState,
    pipelines: &GpuPipelines,
    renderer: &mut WindowRenderer,
    entries: Vec<TabEntry>,
    active_index: usize,
    hover_hit: TabBarHit,
) -> Vec<u8> {
    let theme = UiTheme::dark();
    let mut tab_bar = TabBarWidget::with_theme(WIDTH as f32, &theme);
    tab_bar.set_tabs(entries);
    tab_bar.set_active_index(active_index);
    tab_bar.set_hover_hit(hover_hit);

    let height = TAB_BAR_HEIGHT.ceil() as u32;

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

/// 3 tabs (1 active, 1 modified, 1 plain) at 96 DPI.
///
/// Verifies: flat tabs, accent bar on active, bottom border, modified dot,
/// separators, no rounded corners.
#[test]
fn tab_bar_brutal_3tabs_96dpi() {
    let Some((gpu, pipelines, mut renderer)) = headless_brutal_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let height = TAB_BAR_HEIGHT.ceil() as u32;
    let pixels = render_tab_bar_brutal(
        &gpu,
        &pipelines,
        &mut renderer,
        vec![
            TabEntry::new("zsh"),
            TabEntry::new("nvim config.toml").with_modified(true),
            TabEntry::new("cargo build"),
        ],
        0,
        TabBarHit::None,
    );

    if let Err(msg) = compare_with_reference("tab_bar_brutal_3tabs_96dpi", &pixels, WIDTH, height) {
        panic!("{msg}");
    }
}

/// Active tab with close button at default 0.6 opacity (no hover).
#[test]
fn tab_bar_brutal_active_close_default_96dpi() {
    let Some((gpu, pipelines, mut renderer)) = headless_brutal_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let height = TAB_BAR_HEIGHT.ceil() as u32;
    let pixels = render_tab_bar_brutal(
        &gpu,
        &pipelines,
        &mut renderer,
        vec![TabEntry::new("zsh"), TabEntry::new("htop")],
        0,
        TabBarHit::None,
    );

    if let Err(msg) = compare_with_reference(
        "tab_bar_brutal_active_close_default_96dpi",
        &pixels,
        WIDTH,
        height,
    ) {
        panic!("{msg}");
    }
}

/// Active tab with close button hovered (1.0 opacity + danger highlight).
#[test]
fn tab_bar_brutal_hover_close_96dpi() {
    let Some((gpu, pipelines, mut renderer)) = headless_brutal_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let height = TAB_BAR_HEIGHT.ceil() as u32;
    let pixels = render_tab_bar_brutal(
        &gpu,
        &pipelines,
        &mut renderer,
        vec![TabEntry::new("zsh"), TabEntry::new("htop")],
        0,
        TabBarHit::CloseTab(0),
    );

    if let Err(msg) =
        compare_with_reference("tab_bar_brutal_hover_close_96dpi", &pixels, WIDTH, height)
    {
        panic!("{msg}");
    }
}

/// Tab bar with new-tab and dropdown buttons visible.
///
/// Verifies: border-left on action buttons, correct icon placement at 36px.
#[test]
fn tab_bar_brutal_actions_96dpi() {
    let Some((gpu, pipelines, mut renderer)) = headless_brutal_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let height = TAB_BAR_HEIGHT.ceil() as u32;
    let pixels = render_tab_bar_brutal(
        &gpu,
        &pipelines,
        &mut renderer,
        vec![TabEntry::new("zsh")],
        0,
        TabBarHit::None,
    );

    if let Err(msg) = compare_with_reference("tab_bar_brutal_actions_96dpi", &pixels, WIDTH, height)
    {
        panic!("{msg}");
    }
}

/// Single tab (no separators, active by default).
///
/// Edge case: accent bar and bottom bleed render correctly with only one tab.
#[test]
fn tab_bar_brutal_single_tab_96dpi() {
    let Some((gpu, pipelines, mut renderer)) = headless_brutal_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let height = TAB_BAR_HEIGHT.ceil() as u32;
    let pixels = render_tab_bar_brutal(
        &gpu,
        &pipelines,
        &mut renderer,
        vec![TabEntry::new("zsh")],
        0,
        TabBarHit::None,
    );

    if let Err(msg) =
        compare_with_reference("tab_bar_brutal_single_tab_96dpi", &pixels, WIDTH, height)
    {
        panic!("{msg}");
    }
}
