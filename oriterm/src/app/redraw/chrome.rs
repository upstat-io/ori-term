//! Chrome rendering pipeline: tab bar, overlays, search bar, status bar,
//! window border.
//!
//! Extracted from the single-pane and multi-pane redraw paths to eliminate
//! ~100 lines of algorithmic duplication. Both paths call [`render_chrome`]
//! after pane extraction and preparation are complete.

use oriterm_ui::geometry::Rect;
use oriterm_ui::theme::UiTheme;

use super::draw_helpers;
use crate::app::App;
use crate::app::window_context::WindowContext;
use crate::config::{Config, TabBarPosition};
use crate::gpu::FrameSearch;
use crate::gpu::state::GpuState;

/// Parameters that vary between the single-pane and multi-pane chrome
/// rendering pipelines. Focused-pane chrome state (`content_cols`,
/// `content_rows`, `search`) is passed explicitly to avoid reading the
/// per-pane scratch buffer `ctx.frame`, which holds the last-iterated
/// pane's state in multi-pane mode.
pub(in crate::app::redraw) struct ChromeParams<'a> {
    /// Number of panes (1 for single-pane, `layouts.len()` for multi-pane).
    pub pane_count: usize,
    /// Focused pane's column count, for status-bar text rendering.
    pub content_cols: usize,
    /// Focused pane's row count, for status-bar text rendering.
    pub content_rows: usize,
    /// Focused pane's search state, for search-bar overlay.
    pub search: Option<&'a FrameSearch>,
}

/// Render chrome (tab bar, overlays, search bar, status bar, window border)
/// and compute whether a full content render is needed.
///
/// Shared by both the single-pane and multi-pane redraw paths. Called after
/// all pane extraction and preparation is complete. The renderer is
/// re-borrowed from `ctx.renderer` so the caller's prior borrow must have
/// ended (NLL handles this automatically).
///
/// Search state and grid dimensions come from `params` (the focused pane's
/// data, provided explicitly by the caller). `ctx.frame` is NOT consulted
/// for chrome-relevant fields because in multi-pane mode it is a per-pane
/// scratch buffer holding the last-iterated pane's state.
///
/// Returns `true` if `render_to_surface` should do a full content render.
#[expect(
    clippy::too_many_lines,
    reason = "linear chrome pipeline: phase gate → tab bar → overlays → search → status → border"
)]
pub(in crate::app::redraw) fn render_chrome(
    ctx: &mut WindowContext,
    config: &Config,
    ui_theme: &UiTheme,
    gpu: &GpuState,
    params: &ChromeParams<'_>,
) -> bool {
    let renderer = ctx.renderer.as_mut().expect("renderer checked by caller");
    let (w, h) = ctx.window.size_px();
    let scale = ctx.window.scale_factor().factor() as f32;

    // Phase gating: prepare + prepaint widget trees if dirty.
    draw_helpers::phase_gate_widgets(
        &mut ctx.root,
        &mut ctx.tab_bar,
        ctx.tab_bar_phys_rect,
        renderer,
        &ctx.text_cache,
        ui_theme,
        scale,
        ctx.ui_stale,
    );

    // Draw tab bar (unified chrome bar).
    let tab_bar_hidden = config.window.tab_bar_position == TabBarPosition::Hidden;
    let logical_w = (w as f32 / scale).round() as u32;
    let (interaction, flags, damage) = ctx.root.interaction_frame_requests_and_damage_mut();
    let tab_bar_ref = (!tab_bar_hidden).then_some(&ctx.tab_bar);
    let tb_phys = ctx.tab_bar_phys_rect;
    let tab_bar_bounds = Rect::new(
        tb_phys.x() / scale,
        tb_phys.y() / scale,
        tb_phys.width() / scale,
        tb_phys.height() / scale,
    );
    let tab_bar_animating = App::draw_tab_bar(
        tab_bar_ref,
        renderer,
        &mut ctx.chrome_scene,
        tab_bar_bounds,
        scale,
        gpu,
        ui_theme,
        &ctx.text_cache,
        interaction,
        flags,
        damage,
    );
    if tab_bar_animating {
        ctx.root.mark_dirty();
    }

    // Draw overlays with per-overlay compositor opacity.
    let logical_size = (logical_w as f32, h as f32 / scale);
    let (overlays, layer_tree, interaction, flags) = ctx
        .root
        .overlays_layer_tree_interaction_and_frame_requests();
    let overlays_animating = App::draw_overlays(
        overlays,
        renderer,
        &mut ctx.chrome_scene,
        logical_size,
        scale,
        gpu,
        layer_tree,
        ui_theme,
        &ctx.text_cache,
        interaction,
        flags,
    );
    if overlays_animating {
        ctx.root.mark_dirty();
    }

    // Draw search bar overlay when search is active.
    if let Some(search) = params.search {
        let chrome_h = if tab_bar_hidden {
            0.0
        } else {
            ctx.tab_bar.metrics().height
        };
        App::draw_search_bar(
            search,
            renderer,
            &mut ctx.chrome_scene,
            &mut ctx.search_bar_buf,
            logical_w as f32,
            chrome_h,
            scale,
            gpu,
            &ctx.text_cache,
        );
    }

    // Update and draw status bar at the bottom of the window.
    if config.window.show_status_bar && config.window.tab_bar_position != TabBarPosition::Bottom {
        ctx.status_bar.set_data(draw_helpers::status_bar_data(
            params.pane_count,
            params.content_cols,
            params.content_rows,
        ));
        let phys = ctx.status_bar_phys_rect;
        let sb_bounds = Rect::new(
            phys.x() / scale,
            phys.y() / scale,
            phys.width() / scale,
            phys.height() / scale,
        );
        App::draw_status_bar(
            &ctx.status_bar,
            renderer,
            &mut ctx.chrome_scene,
            sb_bounds,
            scale,
            gpu,
            ui_theme,
            &ctx.text_cache,
        );
    }

    let needs_full_render = renderer.cache_invalidated_this_frame() || ctx.ui_stale;

    // Overlay tiers render above the cached content every frame, so
    // only chrome animations keep the content cache stale.
    ctx.ui_stale = tab_bar_animating;

    // Window border: 2px border-strong frame, skipped when maximized/fullscreen.
    // macOS: the compositor provides a native window shadow — no border needed.
    #[cfg(not(target_os = "macos"))]
    if !ctx.window.is_maximized() && !ctx.window.is_fullscreen() {
        let border_color = crate::gpu::scene_convert::color_to_rgb(ui_theme.border_strong);
        let border_width = crate::gpu::window_renderer::physical_border_width(scale);
        renderer.append_window_border(w, h, border_color, border_width);
    }

    needs_full_render
}
