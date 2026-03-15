//! Dialog content action dispatch.
//!
//! Handles widget actions emitted by dialog content (settings panel,
//! confirmation dialog): Save, Cancel, OK, dropdown open, selection,
//! toggle. Also manages dropdown popup overlays within dialog windows
//! and keyboard routing to confirmation dialog widgets.

use std::time::Instant;

use oriterm_ui::geometry::Rect;
use oriterm_ui::input::{
    EventResponse, HoverEvent, Key as UiKey, KeyEvent as UiKeyEvent, Modifiers as UiModifiers,
};
use oriterm_ui::overlay::OverlayEventResult;
use oriterm_ui::widgets::{EventCtx, Widget, WidgetAction};
use winit::keyboard::{Key, NamedKey};
use winit::window::WindowId;

use crate::app::settings_overlay;
use crate::event::ConfirmationKind;
use crate::font::{CachedTextMeasurer, UiFontMeasurer};

use super::DialogContent;
use crate::app::App;

impl App {
    /// Process a `WidgetAction` emitted by dialog content widgets.
    ///
    /// Routes settings-specific actions (Save, Cancel, dropdown open,
    /// toggle, selection) and confirmation actions (OK, Cancel) to their
    /// handlers.
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
            WidgetAction::Clicked(_) => {
                // OK button clicked in a confirmation dialog.
                self.execute_confirmation(window_id);
            }
            WidgetAction::DismissOverlay(_) => {
                // Cancel button clicked in a confirmation dialog.
                self.close_dialog(window_id);
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
            let DialogContent::Settings { pending_config, .. } = &ctx.content else {
                return;
            };
            (**pending_config).clone()
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
        } = &mut ctx.content
        else {
            return;
        };

        if settings_overlay::action_handler::handle_settings_action(action, ids, pending_config) {
            log::info!("settings dialog: pending config updated (deferred until Save)");
            // Propagate selection back to widget so it updates its display.
            panel.accept_action(action);
            ctx.request_urgent_redraw();
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
        ctx.overlays.replace_popup(
            Box::new(widget),
            anchor,
            Placement::BelowFlush,
            &mut ctx.layer_tree,
            &mut ctx.layer_animator,
            now,
        );
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
            ctx.request_urgent_redraw();
        }
        self.pending_dropdown_id = None;
    }

    /// Execute the confirmation action and close the dialog.
    fn execute_confirmation(&mut self, window_id: WindowId) {
        // Extract the confirmation kind before closing (close drops the ctx).
        let kind = {
            let Some(ctx) = self.dialogs.get_mut(&window_id) else {
                return;
            };
            match &mut ctx.content {
                DialogContent::Confirmation { kind, .. } => {
                    // Take ownership by swapping with a dummy.
                    // We use a Paste with empty text as the dummy since we're
                    // about to close the dialog anyway.
                    std::mem::replace(
                        kind,
                        ConfirmationKind::Paste {
                            text: String::new(),
                        },
                    )
                }
                DialogContent::Settings { .. } => return,
            }
        };

        match kind {
            ConfirmationKind::Paste { text } => {
                log::info!("confirmation: pasting {} bytes", text.len());
                self.write_paste_to_pty(&text);
            }
        }

        self.close_dialog(window_id);
    }

    /// Try routing a key event to the dialog content widget.
    ///
    /// Returns the emitted `WidgetAction` if the content handled the key.
    /// Only confirmation dialogs handle key events (Tab/Enter/Space for
    /// button focus cycling and activation).
    pub(in crate::app) fn try_dialog_content_key(
        &mut self,
        window_id: WindowId,
        event: &winit::event::KeyEvent,
    ) -> Option<WidgetAction> {
        let ui_key = match &event.logical_key {
            Key::Named(NamedKey::Tab) => UiKey::Tab,
            Key::Named(NamedKey::Enter) => UiKey::Enter,
            Key::Named(NamedKey::Space) => UiKey::Space,
            _ => return None,
        };

        let ctx = self.dialogs.get_mut(&window_id)?;
        if !matches!(ctx.content, DialogContent::Confirmation { .. }) {
            return None;
        }

        let ui_event = UiKeyEvent {
            key: ui_key,
            modifiers: UiModifiers::NONE,
        };
        let renderer = ctx.renderer.as_ref()?;
        let scale = ctx.scale_factor.factor() as f32;
        let measurer = CachedTextMeasurer::new(
            UiFontMeasurer::new(renderer.active_ui_collection(), scale),
            &ctx.text_cache,
            scale,
        );
        let chrome_h = ctx.chrome.caption_height();
        let w = ctx.surface_config.width as f32 / scale;
        let h = ctx.surface_config.height as f32 / scale;
        let content_bounds = Rect::new(0.0, chrome_h, w, h - chrome_h);
        let event_ctx = EventCtx {
            measurer: &measurer,
            bounds: content_bounds,
            is_focused: true,
            focused_widget: None,
            theme: &self.ui_theme,
            interaction: None,
            widget_id: None,
        };
        let resp = ctx
            .content
            .content_widget_mut()
            .handle_key(ui_event, &event_ctx);
        if matches!(
            resp.response,
            EventResponse::RequestPaint | EventResponse::RequestLayout
        ) {
            ctx.request_urgent_redraw();
        }
        resp.action
    }

    /// Clear hover state for chrome and content.
    pub(in crate::app) fn clear_dialog_hover(&mut self, window_id: WindowId) {
        let ui_theme = self.ui_theme;
        let Some(ctx) = self.dialogs.get_mut(&window_id) else {
            return;
        };
        let Some(renderer) = ctx.renderer.as_ref() else {
            return;
        };
        let scale = ctx.scale_factor.factor() as f32;
        let measurer = CachedTextMeasurer::new(
            UiFontMeasurer::new(renderer.active_ui_collection(), scale),
            &ctx.text_cache,
            scale,
        );
        let mut needs_redraw = false;

        // Chrome hover clear.
        let event_ctx = EventCtx {
            measurer: &measurer,
            bounds: Rect::default(),
            is_focused: false,
            focused_widget: None,
            theme: &ui_theme,
            interaction: None,
            widget_id: None,
        };
        let resp = ctx.chrome.handle_hover(HoverEvent::Leave, &event_ctx);
        if matches!(
            resp.response,
            EventResponse::RequestPaint | EventResponse::RequestLayout
        ) {
            needs_redraw = true;
        }

        // Content hover clear.
        let w = ctx.surface_config.width as f32 / scale;
        let h = ctx.surface_config.height as f32 / scale;
        let chrome_h = ctx.chrome.caption_height();
        let content_bounds = Rect::new(0.0, chrome_h, w, h - chrome_h);
        let event_ctx = EventCtx {
            measurer: &measurer,
            bounds: content_bounds,
            is_focused: false,
            focused_widget: None,
            theme: &ui_theme,
            interaction: None,
            widget_id: None,
        };
        let resp = ctx
            .content
            .content_widget_mut()
            .handle_hover(HoverEvent::Leave, &event_ctx);
        if matches!(
            resp.response,
            EventResponse::RequestPaint | EventResponse::RequestLayout
        ) {
            needs_redraw = true;
        }

        if needs_redraw {
            ctx.request_urgent_redraw();
        }
    }
}
