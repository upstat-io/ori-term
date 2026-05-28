//! Chrome draw methods on [`App`]: tab bar, overlays, status bar.
//!
//! Extracted from `draw_helpers/mod.rs` to keep each file under the 500-line
//! limit. Each method builds a [`DrawCtx`], paints a chrome widget tree into
//! the scene, then appends the rasterized result via the renderer.

use std::time::Instant;

use oriterm_ui::animation::FrameRequestFlags;
use oriterm_ui::draw::{DamageTracker, Scene, build_scene};
use oriterm_ui::geometry::Rect;
use oriterm_ui::interaction::InteractionManager;
use oriterm_ui::overlay::OverlayManager;
use oriterm_ui::theme::UiTheme;
use oriterm_ui::widgets::tab_bar::TabBarWidget;
use oriterm_ui::widgets::{DrawCtx, Widget};

use crate::app::App;
use crate::font::{CachedTextMeasurer, TextShapeCache};
use crate::gpu::state::GpuState;
use crate::gpu::window_renderer::WindowRenderer;

/// Shared rendering handles for the chrome draw helpers.
///
/// Bundles the renderer, output scene, GPU state, theme, text cache, scale,
/// and target bounds — the invariant set threaded through `draw_tab_bar`,
/// `draw_overlays`, and `draw_status_bar`. Per-call widget state
/// (interaction, frame requests, damage) stays a separate argument because
/// those borrow from a different owner (`WindowRoot`).
pub(in crate::app::redraw) struct ChromeDraw<'a> {
    /// Window renderer (text append + icon resolution).
    pub renderer: &'a mut WindowRenderer,
    /// Output scene for this chrome element.
    pub scene: &'a mut Scene,
    /// GPU state for text rasterization.
    pub gpu: &'a GpuState,
    /// UI theme.
    pub theme: &'a UiTheme,
    /// Shaped-text cache.
    pub text_cache: &'a TextShapeCache,
    /// Logical-to-physical scale factor.
    pub scale: f32,
    /// Logical-pixel bounds for this chrome element.
    pub bounds: Rect,
}

impl App {
    /// Draw the tab bar (unified chrome bar).
    ///
    /// Tab bar coordinates are in logical pixels, positioned at y=0.
    /// Uses [`append_ui_scene_with_text`](crate::gpu::WindowRenderer::append_ui_scene_with_text)
    /// because tab titles are rendered as shaped text.
    ///
    /// Returns `true` if the tab bar has running animations (e.g. bell pulse).
    pub(in crate::app::redraw) fn draw_tab_bar(
        tab_bar: Option<&TabBarWidget>,
        draw: ChromeDraw<'_>,
        interaction: &InteractionManager,
        frame_requests: &FrameRequestFlags,
        damage_tracker: &mut DamageTracker,
    ) -> bool {
        let ChromeDraw {
            renderer,
            scene,
            gpu,
            theme,
            text_cache,
            scale,
            bounds,
        } = draw;
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
    pub(in crate::app::redraw) fn draw_overlays(
        overlays: &mut OverlayManager,
        draw: ChromeDraw<'_>,
        tree: &oriterm_ui::compositor::layer_tree::LayerTree,
        interaction: &InteractionManager,
        frame_requests: &FrameRequestFlags,
    ) -> bool {
        let ChromeDraw {
            renderer,
            scene,
            gpu,
            theme,
            text_cache,
            scale,
            bounds,
        } = draw;
        let count = overlays.draw_count();
        if count == 0 {
            return false;
        }

        let mut animating = false;

        // Layout + draw phase: measurer borrows renderer immutably, then
        // drops before the mutable append_ui_scene_with_text call.
        // Opacity is collected per overlay and appended after the borrow ends.
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
    pub(in crate::app::redraw) fn draw_status_bar(
        status_bar: &oriterm_ui::widgets::status_bar::StatusBarWidget,
        draw: ChromeDraw<'_>,
    ) {
        let ChromeDraw {
            renderer,
            scene,
            gpu,
            theme,
            text_cache,
            scale,
            bounds,
        } = draw;
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
