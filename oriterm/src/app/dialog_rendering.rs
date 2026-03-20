//! Dialog window rendering.
//!
//! Draws chrome (title bar) and content (settings form or confirmation
//! dialog) to the dialog window's GPU surface. Overlay popups (dropdown
//! lists) are drawn on top.

use std::time::Instant;

use winit::window::WindowId;

use oriterm_ui::draw::build_scene;
use oriterm_ui::geometry::Rect;
use oriterm_ui::invalidation::DirtyKind;
use oriterm_ui::widgets::DrawCtx;

use super::App;
use super::widget_pipeline::{prepaint_widget_tree, prepare_widget_tree};
use crate::font::{CachedTextMeasurer, UiFontMeasurer};

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
        ctx.root.set_urgent_redraw(false);
        if !ctx.has_surface_area() {
            return;
        }
        let w = ctx.surface_config.width;
        let h = ctx.surface_config.height;
        let scale = ctx.scale_factor.factor() as f32;

        // Prepare the UI-only frame.
        {
            let renderer = ctx.renderer.as_mut().expect("checked above");
            let c = ui_theme.bg_primary;
            let bg = oriterm_core::Rgb {
                r: (c.r * 255.0) as u8,
                g: (c.g * 255.0) as u8,
                b: (c.b * 255.0) as u8,
            };
            renderer.prepare_ui_frame(w, h, bg, 1.0);
            renderer.resolve_icons(gpu, scale);
        }

        let logical_w = w as f32 / scale;
        let logical_h = h as f32 / scale;

        Self::compose_dialog_widgets(ctx, &ui_theme, scale, logical_w, logical_h, gpu);

        // Draw overlay popups (dropdown lists) on top.
        // Re-borrow renderer inside the helper to avoid split-borrow conflict.
        Self::render_dialog_overlays(ctx, scale, &ui_theme, gpu);

        // Re-borrow renderer for final surface render.
        let renderer = ctx.renderer.as_mut().expect("checked above");
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

        // Track animation state for phase gating on the next frame.
        ctx.ui_stale = ctx.root.frame_requests().anim_frame_requested();
        if ctx.ui_stale {
            ctx.root.mark_dirty();
            ctx.window.request_redraw();
        }
    }

    /// Pre-paint mutation + scene composition for chrome and content.
    ///
    /// Delivers lifecycle events, updates visual state animators, then
    /// draws chrome and content widgets to the draw list.
    #[expect(
        clippy::too_many_arguments,
        reason = "extracted helper: ctx, theme, scale, dimensions, gpu"
    )]
    fn compose_dialog_widgets(
        ctx: &mut super::dialog_context::DialogWindowContext,
        ui_theme: &oriterm_ui::theme::UiTheme,
        scale: f32,
        logical_w: f32,
        logical_h: f32,
        gpu: &crate::gpu::state::GpuState,
    ) {
        let renderer = ctx.renderer.as_mut().expect("caller checked renderer");
        let chrome_h = ctx.chrome.caption_height();
        ctx.scene.clear();
        let measurer = CachedTextMeasurer::new(
            UiFontMeasurer::new(renderer.active_ui_collection(), scale),
            &ctx.text_cache,
            scale,
        );
        let icons = renderer.resolved_icons();

        // Phase gating: compute dirty level from lifecycle events,
        // animation state, and invalidation tracker.
        let now = Instant::now();
        let lifecycle_events = ctx.root.interaction_mut().drain_events();
        let widget_dirty = {
            let mut d = ctx.root.invalidation().max_dirty_kind();
            if !lifecycle_events.is_empty() {
                d = d.merge(DirtyKind::Prepaint);
            }
            if ctx.ui_stale {
                d = d.merge(DirtyKind::Prepaint);
            }
            d
        };
        *ctx.root.frame_requests_mut() = oriterm_ui::animation::FrameRequestFlags::new();

        if widget_dirty >= DirtyKind::Prepaint {
            let (interaction, frame_requests) = ctx.root.interaction_mut_and_frame_requests();
            prepare_widget_tree(
                &mut ctx.chrome,
                interaction,
                &lifecycle_events,
                None,
                Some(frame_requests),
                now,
            );
            let (interaction, frame_requests) = ctx.root.interaction_mut_and_frame_requests();
            prepare_widget_tree(
                ctx.content.content_widget_mut(),
                interaction,
                &lifecycle_events,
                None,
                Some(frame_requests),
                now,
            );

            // Prepaint: resolve visual state into widget fields.
            let prepaint_bounds = std::collections::HashMap::new();
            let (interaction, frame_requests) = ctx.root.interaction_and_frame_requests();
            prepaint_widget_tree(
                &mut ctx.chrome,
                &prepaint_bounds,
                Some(interaction),
                ui_theme,
                now,
                Some(frame_requests),
            );
            let (interaction, frame_requests) = ctx.root.interaction_and_frame_requests();
            prepaint_widget_tree(
                ctx.content.content_widget_mut(),
                &prepaint_bounds,
                Some(interaction),
                ui_theme,
                now,
                Some(frame_requests),
            );
        }

        // Draw the chrome title bar via build_scene.
        let chrome_bounds = Rect::new(0.0, 0.0, logical_w, chrome_h);
        {
            let mut draw_ctx = DrawCtx {
                measurer: &measurer,
                scene: &mut ctx.scene,
                bounds: chrome_bounds,
                now: Instant::now(),
                theme: ui_theme,
                icons: Some(icons),
                interaction: None,
                widget_id: None,
                frame_requests: None,
            };
            build_scene(&ctx.chrome, &mut draw_ctx);
        }

        // Draw the dialog content below the chrome.
        let content_bounds = Rect::new(0.0, chrome_h, logical_w, logical_h - chrome_h);
        {
            let mut draw_ctx = DrawCtx {
                measurer: &measurer,
                scene: &mut ctx.scene,
                bounds: content_bounds,
                now: Instant::now(),
                theme: ui_theme,
                icons: Some(icons),
                interaction: None,
                widget_id: None,
                frame_requests: None,
            };
            build_scene(ctx.content.content_widget(), &mut draw_ctx);
        }

        // Compute per-widget damage for future partial repaint support.
        ctx.root.damage_mut().compute_damage(&ctx.scene);

        // Convert scene to GPU instances.
        renderer.append_ui_scene_with_text(&ctx.scene, scale, 1.0, gpu);
    }

    /// Draw overlay popups (dropdown lists) on top of dialog content.
    fn render_dialog_overlays(
        ctx: &mut super::dialog_context::DialogWindowContext,
        scale: f32,
        ui_theme: &oriterm_ui::theme::UiTheme,
        gpu: &crate::gpu::state::GpuState,
    ) {
        let overlay_count = ctx.root.overlay_draw_count();
        if overlay_count == 0 {
            return;
        }

        let w = ctx.surface_config.width as f32 / scale;
        let h = ctx.surface_config.height as f32 / scale;
        let overlay_bounds = Rect::new(0.0, 0.0, w, h);

        let renderer = ctx.renderer.as_mut().expect("caller verified renderer");
        {
            let measurer = CachedTextMeasurer::new(
                UiFontMeasurer::new(renderer.active_ui_collection(), scale),
                &ctx.text_cache,
                scale,
            );
            ctx.root.overlays_mut().layout_overlays(&measurer, ui_theme);
        }

        for i in 0..overlay_count {
            ctx.scene.clear();
            let measurer = CachedTextMeasurer::new(
                UiFontMeasurer::new(renderer.active_ui_collection(), scale),
                &ctx.text_cache,
                scale,
            );
            let icons = renderer.resolved_icons();
            let mut overlay_draw_ctx = DrawCtx {
                measurer: &measurer,
                scene: &mut ctx.scene,
                bounds: overlay_bounds,
                now: Instant::now(),
                theme: ui_theme,
                icons: Some(icons),
                interaction: None,
                widget_id: None,
                frame_requests: None,
            };
            let opacity = ctx.root.draw_overlay_at(i, &mut overlay_draw_ctx);
            renderer.append_overlay_scene_with_text(&ctx.scene, scale, opacity, gpu);
        }
    }
}
