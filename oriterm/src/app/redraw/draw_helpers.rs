//! Draw helper methods for tab bar, overlays, and widget pipeline phases.
//!
//! Extracted from `mod.rs` to keep the module under the 500-line limit.

use std::collections::HashMap;
use std::time::Instant;

use oriterm_ui::animation::FrameRequestFlags;
use oriterm_ui::draw::{DamageTracker, Scene, build_scene};
use oriterm_ui::geometry::Rect;
use oriterm_ui::interaction::InteractionManager;
use oriterm_ui::invalidation::DirtyKind;
use oriterm_ui::layout::compute_layout;
use oriterm_ui::overlay::OverlayManager;
use oriterm_ui::pipeline::collect_layout_bounds;
use oriterm_ui::theme::UiTheme;
use oriterm_ui::widget_id::WidgetId;
use oriterm_ui::widgets::status_bar::StatusBarData;
use oriterm_ui::widgets::tab_bar::TabBarWidget;
use oriterm_ui::widgets::{DrawCtx, LayoutCtx, Widget};
use oriterm_ui::window_root::WindowRoot;

use crate::app::App;
use crate::app::widget_pipeline;
use crate::font::{CachedTextMeasurer, TextShapeCache};
use crate::gpu::state::GpuState;
use crate::gpu::window_renderer::WindowRenderer;

impl App {
    /// Draw the tab bar (unified chrome bar).
    ///
    /// Tab bar coordinates are in logical pixels, positioned at y=0.
    /// Uses [`append_ui_scene_with_text`](crate::gpu::WindowRenderer::append_ui_scene_with_text)
    /// because tab titles are rendered as shaped text.
    ///
    /// Returns `true` if the tab bar has running animations (e.g. bell pulse).
    #[expect(
        clippy::too_many_arguments,
        reason = "tab bar drawing: widget, renderer, scene, bounds, scale, GPU, theme, cache, interaction, frame_requests, damage"
    )]
    pub(in crate::app::redraw) fn draw_tab_bar(
        tab_bar: Option<&TabBarWidget>,
        renderer: &mut WindowRenderer,
        scene: &mut Scene,
        bounds: Rect,
        scale: f32,
        gpu: &GpuState,
        theme: &UiTheme,
        text_cache: &TextShapeCache,
        interaction: &InteractionManager,
        frame_requests: &FrameRequestFlags,
        damage_tracker: &mut DamageTracker,
    ) -> bool {
        let Some(tab_bar) = tab_bar else {
            return false;
        };
        if tab_bar.tab_count() == 0 {
            return false;
        }

        let measurer = CachedTextMeasurer::new(renderer.ui_measurer(scale), text_cache, scale);
        let icons = renderer.resolved_icons();

        let mut ctx = DrawCtx {
            measurer: &measurer,
            scene,
            bounds,
            now: Instant::now(),
            theme,
            icons: Some(icons),
            interaction: Some(interaction),
            widget_id: None,
            frame_requests: Some(frame_requests),
        };
        build_scene(tab_bar, &mut ctx);
        damage_tracker.compute_damage(scene);
        log::debug!(
            "damage: has_damage={}, dirty_regions={}",
            damage_tracker.has_damage(),
            damage_tracker.dirty_regions().len()
        );
        let animating = frame_requests.anim_frame_requested();

        // Tab bar contains text — use text-aware conversion to rasterize
        // tab title glyphs into the chrome tier.
        renderer.append_ui_scene_with_text(scene, scale, 1.0, gpu);

        // Dragged tab overlay: render in the overlay tier (draws 10-13) so it
        // paints ON TOP of all chrome text. Without this, regular tab text from
        // the chrome tier (draw 7) would show through the dragged tab's bg.
        if tab_bar.has_drag_overlay() {
            scene.clear();
            let measurer = CachedTextMeasurer::new(renderer.ui_measurer(scale), text_cache, scale);
            let icons = renderer.resolved_icons();
            let mut overlay_ctx = DrawCtx {
                measurer: &measurer,
                scene,
                bounds,
                now: Instant::now(),
                theme,
                icons: Some(icons),
                interaction: Some(interaction),
                widget_id: None,
                frame_requests: Some(frame_requests),
            };
            tab_bar.draw_drag_overlay(&mut overlay_ctx);
            renderer.append_overlay_scene_with_text(scene, scale, 1.0, gpu);
        }

        animating
    }

    /// Draw overlays (active + dismissing) with per-overlay compositor opacity.
    ///
    /// Each overlay is drawn individually so its compositor layer opacity
    /// can be applied independently (e.g. during simultaneous fade-in/fade-out).
    /// Modal dim rects are emitted before their content overlay.
    ///
    /// Returns `true` if overlays have running animations (fade-in/fade-out).
    #[expect(
        clippy::too_many_arguments,
        reason = "overlay drawing: manager, renderer, scene, viewport, scale, GPU, tree, theme, cache, interaction, frame_requests"
    )]
    pub(in crate::app::redraw) fn draw_overlays(
        overlays: &mut OverlayManager,
        renderer: &mut WindowRenderer,
        scene: &mut Scene,
        logical_size: (f32, f32),
        scale: f32,
        gpu: &GpuState,
        tree: &oriterm_ui::compositor::layer_tree::LayerTree,
        theme: &UiTheme,
        text_cache: &TextShapeCache,
        interaction: &InteractionManager,
        frame_requests: &FrameRequestFlags,
    ) -> bool {
        let count = overlays.draw_count();
        if count == 0 {
            return false;
        }

        let bounds = Rect::new(0.0, 0.0, logical_size.0, logical_size.1);
        let mut animating = false;

        // Layout + draw phase: measurer borrows renderer immutably, then
        // drops before the mutable append_ui_scene_with_text call.
        // We collect (opacity) per overlay, then append after the borrow ends.
        {
            let measurer = CachedTextMeasurer::new(renderer.ui_measurer(scale), text_cache, scale);
            overlays.layout_overlays(&measurer, theme);
        }

        for i in 0..count {
            scene.clear();
            // Re-create measurer per iteration — cheap (no allocation), and
            // the immutable borrow drops before the mutable append below.
            let measurer = CachedTextMeasurer::new(renderer.ui_measurer(scale), text_cache, scale);
            let icons = renderer.resolved_icons();
            let mut ctx = DrawCtx {
                measurer: &measurer,
                scene,
                bounds,
                now: Instant::now(),
                theme,
                icons: Some(icons),
                interaction: Some(interaction),
                widget_id: None,
                frame_requests: Some(frame_requests),
            };
            let opacity = overlays.draw_overlay_at(i, &mut ctx, tree);

            // If opacity is < 1.0 an animation is running.
            if opacity < 1.0 - f32::EPSILON {
                animating = true;
            }

            // measurer (immutable borrow on renderer) is dropped here by NLL.
            // Overlays write to the overlay tier (draws 10-13) so their
            // backgrounds render ON TOP of chrome text (draws 7-9).
            renderer.append_overlay_scene_with_text(scene, scale, opacity, gpu);
        }

        animating || frame_requests.anim_frame_requested()
    }

    /// Draw the status bar at the bottom of the window.
    ///
    /// The status bar is non-interactive — no hover, focus, or animation
    /// state. It renders terminal metadata (shell name, pane count, grid
    /// dimensions, encoding, term type) into the chrome scene.
    #[expect(
        clippy::too_many_arguments,
        reason = "status bar drawing: widget, renderer, scene, bounds, scale, GPU, theme, cache"
    )]
    pub(in crate::app::redraw) fn draw_status_bar(
        status_bar: &oriterm_ui::widgets::status_bar::StatusBarWidget,
        renderer: &mut WindowRenderer,
        scene: &mut Scene,
        bounds: Rect,
        scale: f32,
        gpu: &GpuState,
        theme: &UiTheme,
        text_cache: &TextShapeCache,
    ) {
        let measurer = CachedTextMeasurer::new(renderer.ui_measurer(scale), text_cache, scale);
        scene.clear();
        let mut ctx = DrawCtx {
            measurer: &measurer,
            scene,
            bounds,
            now: Instant::now(),
            theme,
            icons: None,
            interaction: None,
            widget_id: None,
            frame_requests: None,
        };
        status_bar.paint(&mut ctx);
        renderer.append_ui_scene_with_text(scene, scale, 1.0, gpu);
    }
}

/// Computes prepaint layout bounds for a tab bar widget.
///
/// Runs the layout solver on the tab bar at its known position (y=0, full
/// logical width) and collects per-widget bounds into a `HashMap`. The
/// resulting map is passed to `prepaint_widget_tree` so that
/// `PrepaintCtx::bounds` reflects real screen positions.
#[expect(
    clippy::too_many_arguments,
    reason = "prepaint bounds: tab bar, renderer, cache, theme, scale, width"
)]
pub(in crate::app::redraw) fn collect_tab_bar_prepaint_bounds(
    tab_bar: &TabBarWidget,
    renderer: &WindowRenderer,
    text_cache: &TextShapeCache,
    theme: &UiTheme,
    scale: f32,
    tab_bar_bounds: Rect,
) -> HashMap<WidgetId, Rect> {
    let tab_bar_rect = tab_bar_bounds;
    let measurer = CachedTextMeasurer::new(renderer.ui_measurer(scale), text_cache, scale);
    let layout_ctx = LayoutCtx {
        measurer: &measurer,
        theme,
    };
    let mut bounds = HashMap::new();
    let tab_layout = compute_layout(&Widget::layout(tab_bar, &layout_ctx), tab_bar_rect);
    collect_layout_bounds(&tab_layout, &mut bounds);
    bounds
}

/// Run widget prepare and prepaint if the tree has pending dirty state.
///
/// Shared by both single-pane and multi-pane redraw paths. Drains
/// lifecycle events, checks dirty level, and if `>= Prepaint`, runs
/// the full prepare → prepaint pipeline on the tab bar and overlay
/// widget trees.
#[expect(
    clippy::too_many_arguments,
    reason = "phase gating: root, tab_bar, bounds, renderer, text_cache, theme, scale, stale"
)]
pub(super) fn phase_gate_widgets(
    root: &mut WindowRoot,
    tab_bar: &mut TabBarWidget,
    tab_bar_phys_rect: Rect,
    renderer: &WindowRenderer,
    text_cache: &TextShapeCache,
    ui_theme: &UiTheme,
    scale: f32,
    ui_stale: bool,
) {
    let now = Instant::now();
    let lifecycle_events = root.interaction_mut().drain_events();
    let widget_dirty = {
        let mut d = root.invalidation().max_dirty_kind();
        if !lifecycle_events.is_empty() {
            d = d.merge(DirtyKind::Prepaint);
        }
        if ui_stale {
            d = d.merge(DirtyKind::Prepaint);
        }
        d
    };
    root.frame_requests_mut().reset();

    log::debug!("phase gating: widget_dirty={widget_dirty:?}");

    if widget_dirty >= DirtyKind::Prepaint {
        let (interaction, invalidation, flags) =
            root.interaction_invalidation_and_frame_requests_mut();
        widget_pipeline::prepare_widget_tree(
            tab_bar,
            interaction,
            Some(invalidation),
            &lifecycle_events,
            None,
            Some(flags),
            now,
        );
        root.prepare_overlay_widgets(&lifecycle_events, now);

        let prepaint_tab_bounds = Rect::new(
            tab_bar_phys_rect.x() / scale,
            tab_bar_phys_rect.y() / scale,
            tab_bar_phys_rect.width() / scale,
            tab_bar_phys_rect.height() / scale,
        );
        let prepaint_bounds = collect_tab_bar_prepaint_bounds(
            tab_bar,
            renderer,
            text_cache,
            ui_theme,
            scale,
            prepaint_tab_bounds,
        );
        let (interaction, flags) = root.interaction_and_frame_requests();
        let invalidation = root.invalidation();
        widget_pipeline::prepaint_widget_tree(
            tab_bar,
            &prepaint_bounds,
            Some(interaction),
            ui_theme,
            now,
            Some(flags),
            Some(invalidation),
        );
        root.prepaint_overlay_widgets(&prepaint_bounds, ui_theme, now);
    }
}

/// Build status bar data from pane count and grid dimensions.
pub(super) fn status_bar_data(pane_count: usize, cols: usize, rows: usize) -> StatusBarData {
    StatusBarData {
        shell_name: "shell".into(),
        pane_count: format!(
            "{pane_count} pane{}",
            if pane_count == 1 { "" } else { "s" }
        ),
        grid_size: format!("{cols}\u{00d7}{rows}"),
        encoding: "UTF-8".into(),
        term_type: "xterm-256color".into(),
    }
}
