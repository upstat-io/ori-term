//! Dialog overlay (dropdown popup) management.
//!
//! Handles opening, dismissing, and routing events for dropdown popups
//! within dialog windows. Dropdown selections are routed back through
//! the settings action handler.

use std::time::Instant;

use oriterm_ui::geometry::Rect;
use oriterm_ui::overlay::OverlayEventResult;
use oriterm_ui::widgets::WidgetAction;
use winit::window::WindowId;

use crate::app::App;

impl App {
    /// Open a dropdown popup within a dialog window's overlay manager.
    #[expect(
        clippy::too_many_arguments,
        reason = "forwarding OpenDropdown fields + window ID"
    )]
    pub(in crate::app) fn open_dialog_dropdown(
        &mut self,
        window_id: WindowId,
        dropdown_id: oriterm_ui::widget_id::WidgetId,
        options: Vec<String>,
        selected: usize,
        anchor: Rect,
    ) {
        use oriterm_ui::overlay::Placement;
        use oriterm_ui::widgets::menu::{MenuEntry, MenuStyle, MenuWidget};

        let entries: Vec<MenuEntry> = options
            .into_iter()
            .map(|label| MenuEntry::Item { label })
            .collect();

        let mut style = MenuStyle::from_theme(&self.ui_theme);
        style.min_width = anchor.width();
        style.extra_width = 24.0;
        style.shadow_color = self.ui_theme.shadow;
        style.max_height = Some(300.0);
        style.selected_bg = self.ui_theme.accent.with_alpha(0.12);

        let mut widget = MenuWidget::new(entries).with_style(style);
        if selected < widget.entries().len() {
            widget = widget.with_selected_index(selected);
            widget.ensure_visible(selected);
        }

        let now = Instant::now();

        // Store the dropdown ID so we can route the selection back.
        self.pending_dropdown_id = Some(dropdown_id);

        let Some(ctx) = self.dialogs.get_mut(&window_id) else {
            return;
        };
        ctx.root
            .replace_popup(Box::new(widget), anchor, Placement::BelowFlush, now);
        ctx.request_urgent_redraw();
    }

    /// Process an overlay event result from a dialog window.
    pub(in crate::app) fn handle_dialog_overlay_result(
        &mut self,
        window_id: WindowId,
        result: OverlayEventResult,
    ) {
        match result {
            OverlayEventResult::Delivered { response, .. } => {
                if let Some(WidgetAction::Selected { index, .. }) = response.action {
                    // Route selection through settings action handler.
                    if let Some(dropdown_id) = self.pending_dropdown_id.take() {
                        // Dismiss the popup overlay.
                        self.dismiss_dialog_overlay(window_id);

                        let action = WidgetAction::Selected {
                            id: dropdown_id,
                            index,
                        };
                        self.dispatch_dialog_settings_action(window_id, &action);
                    }
                }
            }
            OverlayEventResult::Dismissed(_) => {
                self.pending_dropdown_id = None;
            }
            OverlayEventResult::Blocked | OverlayEventResult::PassThrough => {}
        }
    }

    /// Dismiss the topmost overlay in a dialog window.
    pub(in crate::app) fn dismiss_dialog_overlay(&mut self, window_id: WindowId) {
        let now = Instant::now();
        if let Some(ctx) = self.dialogs.get_mut(&window_id) {
            ctx.root.dismiss_topmost(now);
            ctx.request_urgent_redraw();
        }
        self.pending_dropdown_id = None;
    }
}
