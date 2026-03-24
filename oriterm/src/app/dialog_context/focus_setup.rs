//! Dialog focus infrastructure initialization.
//!
//! Registers content widgets with `InteractionManager`, builds the parent
//! map from layout, sets tab order, and focuses the default widget.

use oriterm_ui::geometry::Rect;
use oriterm_ui::interaction::build_parent_map;
use oriterm_ui::layout::compute_layout;
use oriterm_ui::widgets::LayoutCtx;
use winit::window::WindowId;

use crate::app::widget_pipeline::collect_focusable_ids;
use crate::font::CachedTextMeasurer;

use crate::app::App;

impl App {
    /// Initialize focus infrastructure for a dialog's content widgets.
    ///
    /// Registers all content widgets with `InteractionManager`, builds the
    /// parent map from the layout tree, sets focus order, and focuses the
    /// default button (if a confirmation dialog).
    pub(in crate::app) fn setup_dialog_focus(&mut self, window_id: WindowId) {
        let ui_theme = self.ui_theme;
        let Some(ctx) = self.dialogs.get_mut(&window_id) else {
            return;
        };
        let Some(renderer) = ctx.renderer.as_ref() else {
            return;
        };
        let scale = ctx.scale_factor.factor() as f32;
        let measurer = CachedTextMeasurer::new(renderer.ui_measurer(scale), &ctx.text_cache, scale);

        // Register all widgets (chrome + content) with InteractionManager.
        // WidgetAdded events stay pending and are delivered on the first
        // compose_dialog_widgets() frame via prepare_widget_tree (TPR-04-003).
        crate::app::widget_pipeline::register_widget_tree(
            &mut ctx.chrome,
            ctx.root.interaction_mut(),
        );
        crate::app::widget_pipeline::register_widget_tree(
            ctx.content.content_widget_mut(),
            ctx.root.interaction_mut(),
        );

        // Collect key contexts for keymap scope gating.
        ctx.root.key_contexts_mut().clear();
        oriterm_ui::action::collect_key_contexts(&mut ctx.chrome, ctx.root.key_contexts_mut());
        oriterm_ui::action::collect_key_contexts(
            ctx.content.content_widget_mut(),
            ctx.root.key_contexts_mut(),
        );

        // Compute layout and build parent map.
        let chrome_h = ctx.chrome.caption_height();
        let w = ctx.surface_config.width as f32 / scale;
        let h = ctx.surface_config.height as f32 / scale;
        let layout_ctx = LayoutCtx {
            measurer: &measurer,
            theme: &ui_theme,
        };
        let layout_box = ctx.content.content_widget().layout(&layout_ctx);
        let local_viewport = Rect::new(0.0, 0.0, w, h - chrome_h);
        let layout_node = compute_layout(&layout_box, local_viewport);
        let parent_map = build_parent_map(&layout_node);
        ctx.root.interaction_mut().set_parent_map(parent_map);

        // Collect focusable widgets and set tab order.
        let mut focusable = Vec::new();
        collect_focusable_ids(ctx.content.content_widget_mut(), &mut focusable);

        // Set initial focus on the first focusable widget (typically OK button).
        let initial_focus = focusable.first().copied();
        ctx.root.focus_mut().set_focus_order(focusable);
        if let Some(id) = initial_focus {
            // FocusChanged events stay pending for delivery on the first
            // compose_dialog_widgets() frame (TPR-04-003).
            let changed = {
                let (interaction, focus) = ctx.root.interaction_and_focus_mut();
                interaction.request_focus(id, focus)
            };
            ctx.root.mark_widgets_prepaint_dirty(&changed);
        }
    }
}
