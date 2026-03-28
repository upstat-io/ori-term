//! Dialog window rendering.
//!
//! Draws content (settings form or confirmation dialog) to the dialog
//! window's GPU surface. The dialog is fully frameless — content fills
//! the entire window with no chrome title bar. Overlay popups (dropdown
//! lists) are drawn on top.

use std::time::Instant;

use winit::window::WindowId;

use oriterm_ui::geometry::Rect;
use oriterm_ui::invalidation::DirtyKind;
use oriterm_ui::layout::compute_layout;
use oriterm_ui::pipeline::collect_layout_bounds;
use oriterm_ui::widgets::{DrawCtx, LayoutCtx, Widget};

use super::App;
use super::widget_pipeline::{prepaint_widget_tree, prepare_widget_tree};
use crate::font::CachedTextMeasurer;

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
        // Clear per-frame invalidation so stale dirty marks don't accumulate.
        ctx.root.invalidation_mut().clear();
    }

    /// Pre-paint mutation + scene composition for dialog content.
    ///
    /// Delivers lifecycle events, updates visual state animators, then
    /// draws content widgets to the draw list. The dialog is fully frameless
    /// — content fills the entire window with no chrome title bar.
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
        ctx.scene.clear();
        let measurer = CachedTextMeasurer::new(renderer.ui_measurer(scale), &ctx.text_cache, scale);
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
            {
                let use_tracker = !ctx.ui_stale;
                let (interaction, invalidation, flags) =
                    ctx.root.interaction_invalidation_and_frame_requests_mut();
                let tracker_content = if use_tracker {
                    Some(invalidation)
                } else {
                    None
                };
                prepare_widget_tree(
                    ctx.content.content_widget_mut(),
                    interaction,
                    tracker_content,
                    &lifecycle_events,
                    None,
                    Some(flags),
                    now,
                );
            }
            // Prepaint: resolve visual state into widget fields.
            // Compute layout bounds so PrepaintCtx::bounds reflects real
            // screen positions (not Rect::default()).
            let prepaint_bounds = collect_dialog_prepaint_bounds(
                ctx.content.content_widget(),
                &measurer,
                ui_theme,
                logical_w,
                logical_h,
            );
            let (interaction, flags) = ctx.root.interaction_and_frame_requests();
            // When animating (ui_stale), use full walk (None) so all widgets
            // get prepaint/tick. The invalidation tracker was cleared after
            // the previous frame — selective walks would skip everything.
            let invalidation_tracker = if ctx.ui_stale {
                None
            } else {
                Some(ctx.root.invalidation())
            };
            prepaint_widget_tree(
                ctx.content.content_widget_mut(),
                &prepaint_bounds,
                Some(interaction),
                ui_theme,
                now,
                Some(flags),
                invalidation_tracker,
            );
        }

        // Draw the dialog content filling the full window.
        // Paint directly (not via build_scene, which clears the scene).
        // The scene was already cleared at the top of this function.
        //
        // A separate FrameRequestFlags collects paint-phase anim_frame
        // requests (e.g. toggle slide animation) and is merged back into
        // the root flags afterward.
        let paint_flags = oriterm_ui::animation::FrameRequestFlags::new();
        let interaction = ctx.root.interaction();
        let content_bounds = Rect::new(0.0, 0.0, logical_w, logical_h);
        {
            let mut draw_ctx = DrawCtx {
                measurer: &measurer,
                scene: &mut ctx.scene,
                bounds: content_bounds,
                now: Instant::now(),
                theme: ui_theme,
                icons: Some(icons),
                interaction: Some(interaction),
                widget_id: None,
                frame_requests: Some(&paint_flags),
            };
            ctx.content.content_widget().paint(&mut draw_ctx);
        }

        // Merge paint-phase anim_frame requests into the root flags so
        // render_dialog() sees them and schedules the next frame.
        if paint_flags.anim_frame_requested() {
            ctx.root.frame_requests().request_anim_frame();
        }
        if paint_flags.paint_requested() {
            ctx.root.frame_requests().request_paint();
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
            let measurer =
                CachedTextMeasurer::new(renderer.ui_measurer(scale), &ctx.text_cache, scale);
            ctx.root.overlays_mut().layout_overlays(&measurer, ui_theme);
        }

        let overlay_flags = oriterm_ui::animation::FrameRequestFlags::new();
        let interaction = ctx.root.interaction();
        for i in 0..overlay_count {
            ctx.scene.clear();
            let measurer =
                CachedTextMeasurer::new(renderer.ui_measurer(scale), &ctx.text_cache, scale);
            let icons = renderer.resolved_icons();
            let mut overlay_draw_ctx = DrawCtx {
                measurer: &measurer,
                scene: &mut ctx.scene,
                bounds: overlay_bounds,
                now: Instant::now(),
                theme: ui_theme,
                icons: Some(icons),
                interaction: Some(interaction),
                widget_id: None,
                frame_requests: Some(&overlay_flags),
            };
            let opacity = ctx.root.draw_overlay_at(i, &mut overlay_draw_ctx);
            renderer.append_overlay_scene_with_text(&ctx.scene, scale, opacity, gpu);
        }
        if overlay_flags.anim_frame_requested() {
            ctx.root.frame_requests().request_anim_frame();
        }
    }
}

/// Collects prepaint layout bounds for dialog content widgets.
///
/// Runs the layout solver on content at the full window bounds and
/// collects per-widget bounds into a `HashMap` so that
/// `PrepaintCtx::bounds` reflects real screen positions.
fn collect_dialog_prepaint_bounds(
    content: &dyn Widget,
    measurer: &dyn oriterm_ui::widgets::TextMeasurer,
    ui_theme: &oriterm_ui::theme::UiTheme,
    logical_w: f32,
    logical_h: f32,
) -> std::collections::HashMap<oriterm_ui::widget_id::WidgetId, Rect> {
    let content_bounds = Rect::new(0.0, 0.0, logical_w, logical_h);
    let layout_ctx = LayoutCtx {
        measurer,
        theme: ui_theme,
    };
    let mut bounds = std::collections::HashMap::new();
    let content_layout = compute_layout(&content.layout(&layout_ctx), content_bounds);
    collect_layout_bounds(&content_layout, &mut bounds);
    bounds
}
