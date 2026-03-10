//! Dialog content action dispatch.
//!
//! Handles widget actions emitted by dialog content (settings panel):
//! Save, Cancel, dropdown open, selection, toggle. Also manages
//! dropdown popup overlays within dialog windows.

use std::time::Instant;

use oriterm_ui::geometry::Rect;
use oriterm_ui::overlay::OverlayEventResult;
use oriterm_ui::widgets::{Widget, WidgetAction};
use winit::window::WindowId;

use crate::app::settings_overlay;

use super::super::App;
use super::DialogContent;

impl App {
    /// Process a `WidgetAction` emitted by dialog content widgets.
    ///
    /// Routes settings-specific actions (Save, Cancel, dropdown open,
    /// toggle, selection) to their handlers.
    pub(in crate::app) fn handle_dialog_content_action(
        &mut self,
        window_id: WindowId,
        action: WidgetAction,
    ) {
        match action {
            WidgetAction::SaveSettings => {
                self.save_dialog_settings(window_id);
            }
            WidgetAction::CancelSettings => {
                self.close_dialog(window_id);
            }
            WidgetAction::OpenDropdown {
                id,
                options,
                selected,
                anchor,
            } => {
                self.open_dialog_dropdown(window_id, id, options, selected, anchor);
            }
            WidgetAction::Toggled { .. } | WidgetAction::Selected { .. } => {
                self.dispatch_dialog_settings_action(window_id, &action);
            }
            _ => {}
        }
    }

    /// Apply pending settings config, persist to disk, and close the dialog.
    fn save_dialog_settings(&mut self, window_id: WindowId) {
        let pending = {
            let Some(ctx) = self.dialogs.get_mut(&window_id) else {
                return;
            };
            let DialogContent::Settings { pending_config, .. } = &ctx.content;
            pending_config.clone()
        };

        log::info!("settings dialog: applying and saving to disk");
        let old_config = std::mem::replace(&mut self.config, pending);
        self.apply_settings_change(old_config);
        self.config.save();

        self.close_dialog(window_id);
    }

    /// Dispatch a settings widget action to update the pending config.
    fn dispatch_dialog_settings_action(&mut self, window_id: WindowId, action: &WidgetAction) {
        let Some(ctx) = self.dialogs.get_mut(&window_id) else {
            return;
        };
        let DialogContent::Settings {
            ids,
            pending_config,
            panel,
            ..
        } = &mut ctx.content;

        if settings_overlay::action_handler::handle_settings_action(action, ids, pending_config) {
            log::info!("settings dialog: pending config updated (deferred until Save)");
            // Propagate selection back to widget so it updates its display.
            panel.accept_action(action);
            ctx.dirty = true;
        }
    }

    /// Open a dropdown popup within a dialog window's overlay manager.
    #[expect(
        clippy::too_many_arguments,
        reason = "forwarding OpenDropdown fields + window ID"
    )]
    fn open_dialog_dropdown(
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
        style.corner_radius = 4.0;
        style.shadow_color = self.ui_theme.shadow.with_alpha(0.15);
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
        ctx.overlays.push_overlay(
            Box::new(widget),
            anchor,
            Placement::BelowFlush,
            &mut ctx.layer_tree,
            &mut ctx.layer_animator,
            now,
        );
        ctx.dirty = true;
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

    /// Check if a dialog window has an active overlay (dropdown popup).
    pub(in crate::app) fn dialog_has_overlay(&self, window_id: WindowId) -> bool {
        self.dialogs
            .get(&window_id)
            .is_some_and(|ctx| !ctx.overlays.is_empty())
    }

    /// Dismiss the topmost overlay in a dialog window.
    pub(in crate::app) fn dismiss_dialog_overlay(&mut self, window_id: WindowId) {
        let now = Instant::now();
        if let Some(ctx) = self.dialogs.get_mut(&window_id) {
            ctx.overlays
                .begin_dismiss_topmost(&mut ctx.layer_tree, &mut ctx.layer_animator, now);
            ctx.dirty = true;
        }
        self.pending_dropdown_id = None;
    }
}
