//! Dialog window rendering.
//!
//! Draws chrome (title bar) and content (settings form or confirmation
//! dialog) to the dialog window's GPU surface. Overlay popups (dropdown
//! lists) are drawn on top.

use std::cell::Cell;
use std::time::Instant;

use winit::window::WindowId;

use oriterm_ui::geometry::Rect;
use oriterm_ui::widgets::{DrawCtx, Widget};

use super::App;
use crate::font::UiFontMeasurer;

impl App {
    /// Render a dialog window's content to its GPU surface.
    pub(super) fn render_dialog(&mut self, winit_id: WindowId) {
        let Some(gpu) = self.gpu.as_ref() else { return };
        let Some(pipelines) = self.pipelines.as_ref() else {
            return;
        };
        let ui_theme = self.ui_theme;

        let Some(ctx) = self.dialogs.get_mut(&winit_id) else {
            return;
        };
        if !ctx.has_surface_area() {
            return;
        }
        let Some(renderer) = ctx.renderer.as_mut() else {
            return;
        };

        let w = ctx.surface_config.width;
        let h = ctx.surface_config.height;
        let scale = ctx.scale_factor.factor() as f32;

        // Prepare the UI-only frame.
        let c = ui_theme.bg_primary;
        let bg = oriterm_core::Rgb {
            r: (c.r * 255.0) as u8,
            g: (c.g * 255.0) as u8,
            b: (c.b * 255.0) as u8,
        };
        renderer.prepare_ui_frame(w, h, bg, 1.0);

        // Resolve icons.
        renderer.resolve_icons(gpu, scale);

        let logical_w = w as f32 / scale;
        let logical_h = h as f32 / scale;
        let chrome_h = ctx.chrome.caption_height();

        ctx.draw_list.clear();
        let animations_running = Cell::new(false);
        let measurer = UiFontMeasurer::new(renderer.active_ui_collection(), scale);
        let icons = renderer.resolved_icons();

        // Draw the chrome title bar.
        let chrome_bounds = Rect::new(0.0, 0.0, logical_w, chrome_h);
        {
            let mut draw_ctx = DrawCtx {
                measurer: &measurer,
                draw_list: &mut ctx.draw_list,
                bounds: chrome_bounds,
                focused_widget: None,
                now: Instant::now(),
                animations_running: &animations_running,
                theme: &ui_theme,
                icons: Some(icons),
            };
            ctx.chrome.draw(&mut draw_ctx);
        }

        // Draw the dialog content below the chrome.
        let content_bounds = Rect::new(0.0, chrome_h, logical_w, logical_h - chrome_h);
        {
            let mut draw_ctx = DrawCtx {
                measurer: &measurer,
                draw_list: &mut ctx.draw_list,
                bounds: content_bounds,
                focused_widget: None,
                now: Instant::now(),
                animations_running: &animations_running,
                theme: &ui_theme,
                icons: Some(icons),
            };
            ctx.content.content_widget().draw(&mut draw_ctx);
        }

        // Convert draw list to GPU instances.
        renderer.append_ui_draw_list_with_text(&ctx.draw_list, scale, 1.0, gpu);

        // Draw overlay popups (dropdown lists) on top.
        let overlay_count = ctx.overlays.draw_count();
        if overlay_count > 0 {
            let overlay_bounds = Rect::new(0.0, 0.0, logical_w, logical_h);
            {
                let measurer = UiFontMeasurer::new(renderer.active_ui_collection(), scale);
                ctx.overlays.layout_overlays(&measurer, &ui_theme);
            }
            for i in 0..overlay_count {
                ctx.draw_list.clear();
                let measurer = UiFontMeasurer::new(renderer.active_ui_collection(), scale);
                let icons = renderer.resolved_icons();
                let mut overlay_draw_ctx = DrawCtx {
                    measurer: &measurer,
                    draw_list: &mut ctx.draw_list,
                    bounds: overlay_bounds,
                    focused_widget: None,
                    now: Instant::now(),
                    animations_running: &animations_running,
                    theme: &ui_theme,
                    icons: Some(icons),
                };
                let opacity =
                    ctx.overlays
                        .draw_overlay_at(i, &mut overlay_draw_ctx, &ctx.layer_tree);
                renderer.append_overlay_draw_list_with_text(&ctx.draw_list, scale, opacity, gpu);
            }
        }

        // Render to surface. Dialogs always re-render content (no cursor blink).
        let result = renderer.render_to_surface(gpu, pipelines, &ctx.surface, true);
        match result {
            Ok(()) => {}
            Err(crate::gpu::SurfaceError::Lost) => {
                log::warn!("dialog surface lost, reconfiguring");
                ctx.resize_surface(w, h, gpu);
            }
            Err(e) => log::error!("dialog render error: {e}"),
        }

        // Widget animations (e.g. hover fade) need continued redraws.
        if animations_running.get() {
            ctx.dirty = true;
        }
    }
}
