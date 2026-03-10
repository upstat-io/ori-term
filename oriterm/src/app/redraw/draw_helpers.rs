//! Draw helper methods for tab bar and overlays.
//!
//! Extracted from `mod.rs` to keep the module under the 500-line limit.

use std::cell::Cell;
use std::time::Instant;

use oriterm_ui::draw::DrawList;
use oriterm_ui::overlay::OverlayManager;
use oriterm_ui::theme::UiTheme;
use oriterm_ui::widgets::{DrawCtx, Widget};

use super::super::App;
use crate::font::UiFontMeasurer;
use crate::gpu::state::GpuState;

impl App {
    /// Draw the tab bar (unified chrome bar).
    ///
    /// Tab bar coordinates are in logical pixels, positioned at y=0.
    /// Uses [`append_ui_draw_list_with_text`](crate::gpu::WindowRenderer::append_ui_draw_list_with_text)
    /// because tab titles are rendered as shaped text.
    ///
    /// Returns `true` if the tab bar has running animations (e.g. bell pulse).
    #[expect(
        clippy::too_many_arguments,
        reason = "tab bar drawing: widget, renderer, draw list, viewport, scale, GPU, theme"
    )]
    pub(in crate::app::redraw) fn draw_tab_bar(
        tab_bar: Option<&oriterm_ui::widgets::tab_bar::TabBarWidget>,
        renderer: &mut crate::gpu::WindowRenderer,
        draw_list: &mut DrawList,
        logical_width: f32,
        scale: f32,
        gpu: &GpuState,
        theme: &UiTheme,
    ) -> bool {
        let Some(tab_bar) = tab_bar else {
            return false;
        };
        if tab_bar.tab_count() == 0 {
            return false;
        }

        let tab_bar_h = oriterm_ui::widgets::tab_bar::constants::TAB_BAR_HEIGHT;
        let bounds = oriterm_ui::geometry::Rect::new(0.0, 0.0, logical_width, tab_bar_h);

        draw_list.clear();
        let animations_running = Cell::new(false);
        let measurer = UiFontMeasurer::new(renderer.active_ui_collection(), scale);
        let icons = renderer.resolved_icons();

        let mut ctx = DrawCtx {
            measurer: &measurer,
            draw_list,
            bounds,
            focused_widget: None,
            now: Instant::now(),
            animations_running: &animations_running,
            theme,
            icons: Some(icons),
        };
        tab_bar.draw(&mut ctx);
        let animating = animations_running.get();

        // Tab bar contains text — use text-aware conversion to rasterize
        // tab title glyphs into the chrome tier.
        renderer.append_ui_draw_list_with_text(draw_list, scale, 1.0, gpu);

        // Dragged tab overlay: render in the overlay tier (draws 10–13) so it
        // paints ON TOP of all chrome text. Without this, regular tab text from
        // the chrome tier (draw 7) would show through the dragged tab's bg.
        if tab_bar.has_drag_overlay() {
            draw_list.clear();
            let measurer = UiFontMeasurer::new(renderer.active_ui_collection(), scale);
            let icons = renderer.resolved_icons();
            let mut overlay_ctx = DrawCtx {
                measurer: &measurer,
                draw_list,
                bounds,
                focused_widget: None,
                now: Instant::now(),
                animations_running: &animations_running,
                theme,
                icons: Some(icons),
            };
            tab_bar.draw_drag_overlay(&mut overlay_ctx);
            renderer.append_overlay_draw_list_with_text(draw_list, scale, 1.0, gpu);
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
        reason = "overlay drawing: manager, renderer, draw list, viewport, scale, GPU, tree, theme"
    )]
    pub(in crate::app::redraw) fn draw_overlays(
        overlays: &mut OverlayManager,
        renderer: &mut crate::gpu::WindowRenderer,
        draw_list: &mut DrawList,
        logical_size: (f32, f32),
        scale: f32,
        gpu: &GpuState,
        tree: &oriterm_ui::compositor::layer_tree::LayerTree,
        theme: &UiTheme,
    ) -> bool {
        let count = overlays.draw_count();
        if count == 0 {
            return false;
        }

        let bounds = oriterm_ui::geometry::Rect::new(0.0, 0.0, logical_size.0, logical_size.1);
        let animations_running = Cell::new(false);
        let mut animating = false;

        // Layout + draw phase: measurer borrows renderer immutably, then
        // drops before the mutable append_ui_draw_list_with_text call.
        // We collect (opacity) per overlay, then append after the borrow ends.
        {
            let measurer = UiFontMeasurer::new(renderer.active_ui_collection(), scale);
            overlays.layout_overlays(&measurer, theme);
        }

        for i in 0..count {
            draw_list.clear();
            // Re-create measurer per iteration — cheap (no allocation), and
            // the immutable borrow drops before the mutable append below.
            let measurer = UiFontMeasurer::new(renderer.active_ui_collection(), scale);
            let icons = renderer.resolved_icons();
            let mut ctx = DrawCtx {
                measurer: &measurer,
                draw_list,
                bounds,
                focused_widget: None,
                now: Instant::now(),
                animations_running: &animations_running,
                theme,
                icons: Some(icons),
            };
            let opacity = overlays.draw_overlay_at(i, &mut ctx, tree);

            // If opacity is < 1.0 an animation is running.
            if opacity < 1.0 - f32::EPSILON {
                animating = true;
            }

            // measurer (immutable borrow on renderer) is dropped here by NLL.
            // Overlays write to the overlay tier (draws 10–13) so their
            // backgrounds render ON TOP of chrome text (draws 7–9).
            renderer.append_overlay_draw_list_with_text(draw_list, scale, opacity, gpu);
        }

        animating || animations_running.get()
    }
}
