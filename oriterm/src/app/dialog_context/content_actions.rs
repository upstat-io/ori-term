//! Dialog content action dispatch.
//!
//! Handles widget actions emitted by dialog content (settings panel,
//! confirmation dialog): Save, Cancel, OK, dropdown open, selection,
//! toggle. Also manages dropdown popup overlays within dialog windows
//! and keyboard routing to confirmation dialog widgets.

use std::time::Instant;

use oriterm_ui::controllers::ControllerRequests;
use oriterm_ui::geometry::Rect;
use oriterm_ui::interaction::build_parent_map;
use oriterm_ui::layout::compute_layout;
use oriterm_ui::overlay::OverlayEventResult;
use oriterm_ui::widgets::{LayoutCtx, Widget, WidgetAction};
use winit::window::WindowId;

use crate::app::settings_overlay;
use crate::app::widget_pipeline::{apply_dispatch_requests, collect_focusable_ids};
use crate::event::ConfirmationKind;
use crate::font::{CachedTextMeasurer, UiFontMeasurer};

use super::DialogContent;
use crate::app::App;
use crate::config::Config;

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
            WidgetAction::Toggled { .. }
            | WidgetAction::Selected { .. }
            | WidgetAction::ValueChanged { .. }
            | WidgetAction::TextChanged { .. } => {
                self.dispatch_dialog_settings_action(window_id, &action);
            }
            WidgetAction::ResetDefaults => {
                self.reset_dialog_settings(window_id);
            }
            WidgetAction::Clicked(_) => {
                // OK button clicked in a confirmation dialog.
                self.execute_confirmation(window_id);
            }
            WidgetAction::DismissOverlay(_) => {
                // Cancel button clicked in a confirmation dialog.
                self.close_dialog(window_id);
            }
            // Controller-emitted actions that don't apply to dialog content.
            WidgetAction::DoubleClicked(_)
            | WidgetAction::TripleClicked(_)
            | WidgetAction::DragStart { .. }
            | WidgetAction::DragUpdate { .. }
            | WidgetAction::DragEnd { .. }
            | WidgetAction::ScrollBy { .. }
            | WidgetAction::MoveOverlay { .. }
            | WidgetAction::WindowMinimize
            | WidgetAction::WindowMaximize
            | WidgetAction::WindowClose => {}
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

    /// Reset the pending settings config to defaults.
    fn reset_dialog_settings(&mut self, window_id: WindowId) {
        let Some(ctx) = self.dialogs.get_mut(&window_id) else {
            return;
        };
        let DialogContent::Settings {
            pending_config,
            original_config,
            ..
        } = &mut ctx.content
        else {
            return;
        };
        log::info!("settings dialog: resetting to defaults");
        **pending_config = Config::default();
        let dirty = **pending_config != **original_config;
        let title = if dirty {
            "Settings \u{2022}"
        } else {
            "Settings"
        };
        ctx.window.set_title(title);
        ctx.root.mark_dirty();
    }

    /// Dispatch a settings widget action to update the pending config.
    ///
    /// Always propagates to the panel via `accept_action`, even when the config
    /// is unchanged — widgets may need to update visuals (e.g. sidebar page
    /// switching, dropdown selection display). After config changes, compares
    /// pending vs original to track dirty state.
    fn dispatch_dialog_settings_action(&mut self, window_id: WindowId, action: &WidgetAction) {
        let Some(ctx) = self.dialogs.get_mut(&window_id) else {
            return;
        };
        let DialogContent::Settings {
            ids,
            pending_config,
            panel,
            original_config,
        } = &mut ctx.content
        else {
            return;
        };

        let config_changed =
            settings_overlay::action_handler::handle_settings_action(action, ids, pending_config);
        if config_changed {
            log::info!("settings dialog: pending config updated (deferred until Save)");
            // Update dirty indicator — shows in taskbar/alt-tab title.
            let dirty = **pending_config != **original_config;
            let title = if dirty {
                "Settings \u{2022}"
            } else {
                "Settings"
            };
            ctx.window.set_title(title);
        }

        // Always propagate — widgets update visuals (page switch, selection).
        let widget_handled = panel.accept_action(action);
        if config_changed || widget_handled {
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

    /// Check if a dialog window has an active overlay (dropdown popup).
    pub(in crate::app) fn dialog_has_overlay(&self, window_id: WindowId) -> bool {
        self.dialogs
            .get(&window_id)
            .is_some_and(|ctx| ctx.root.has_overlays())
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

    /// Route a key event through the controller pipeline to dialog content.
    ///
    /// Converts the winit key event to an `InputEvent`, computes the layout
    /// tree for parent map and focus path, then dispatches via
    /// `deliver_event_to_tree`. Returns the first emitted `WidgetAction`.
    pub(in crate::app) fn dispatch_dialog_content_key(
        &mut self,
        window_id: WindowId,
        event: &winit::event::KeyEvent,
    ) -> Option<WidgetAction> {
        let input_event = super::key_conversion::winit_key_to_input_event(event, self.modifiers)?;

        let ui_theme = self.ui_theme;
        let ctx = self.dialogs.get_mut(&window_id)?;
        let renderer = ctx.renderer.as_ref()?;
        let scale = ctx.scale_factor.factor() as f32;
        let measurer = CachedTextMeasurer::new(
            UiFontMeasurer::new(renderer.active_ui_collection(), scale),
            &ctx.text_cache,
            scale,
        );

        // Compute layout for parent map (needed by focus_ancestor_path).
        let chrome_h = ctx.chrome.caption_height();
        let w = ctx.surface_config.width as f32 / scale;
        let h = ctx.surface_config.height as f32 / scale;
        let content_bounds = Rect::new(0.0, chrome_h, w, h - chrome_h);
        let layout_ctx = LayoutCtx {
            measurer: &measurer,
            theme: &ui_theme,
        };
        let layout_box = ctx.content.content_widget().layout(&layout_ctx);
        let local_viewport = Rect::new(0.0, 0.0, content_bounds.width(), content_bounds.height());
        let layout_node = compute_layout(&layout_box, local_viewport);

        // Update parent map and focus order from the current layout.
        let parent_map = build_parent_map(&layout_node);
        ctx.root.interaction_mut().set_parent_map(parent_map);

        let mut focusable = Vec::new();
        collect_focusable_ids(ctx.content.content_widget_mut(), &mut focusable);
        ctx.root.focus_mut().set_focus_order(focusable);

        #[cfg(debug_assertions)]
        let layout_ids = {
            let mut ids = std::collections::HashSet::new();
            oriterm_ui::pipeline::collect_layout_widget_ids(&layout_node, &mut ids);
            ids
        };

        // Build focus path for keyboard routing.
        let focus_path = ctx.root.interaction().focus_ancestor_path();
        let active = ctx.root.interaction().active_widget();
        let now = Instant::now();

        // Dispatch via keymap (for KeyDown) or fall through to controllers.
        let result = super::keymap_dispatch::dispatch_dialog_key_event(
            &input_event,
            ctx,
            &focus_path,
            active,
            content_bounds,
            &layout_node,
            now,
            #[cfg(debug_assertions)]
            &layout_ids,
        );

        // Apply interaction state changes (focus cycling, active).
        let (interaction, focus) = ctx.root.interaction_and_focus_mut();
        apply_dispatch_requests(result.requests, result.source, interaction, focus);

        if result.requests.contains(ControllerRequests::PAINT) {
            ctx.request_urgent_redraw();
        }

        // Transform Clicked(id) through the content widget's on_action
        // (e.g., SettingsPanel maps Clicked(save_id) → SaveSettings).
        result.actions.into_iter().next().map(|a| {
            if let WidgetAction::Clicked(id) = a {
                ctx.content
                    .content_widget_mut()
                    .on_action(WidgetAction::Clicked(id), content_bounds)
                    .unwrap_or(WidgetAction::Clicked(id))
            } else {
                a
            }
        })
    }

    /// Clear hover state for chrome and content.
    ///
    /// Clears the `InteractionManager`'s hot path (empty = no widget under cursor).
    /// The next `prepare_widget_tree` will deliver `HotChanged(false)` lifecycle
    /// events and the `VisualStateAnimator` transitions back to normal.
    pub(in crate::app) fn clear_dialog_hover(&mut self, window_id: WindowId) {
        let Some(ctx) = self.dialogs.get_mut(&window_id) else {
            return;
        };
        ctx.root.interaction_mut().update_hot_path(&[]);
        ctx.request_urgent_redraw();
    }

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
        let measurer = CachedTextMeasurer::new(
            UiFontMeasurer::new(renderer.active_ui_collection(), scale),
            &ctx.text_cache,
            scale,
        );

        // Register all widgets (chrome + content) with InteractionManager.
        crate::app::widget_pipeline::register_widget_tree(
            &mut ctx.chrome,
            ctx.root.interaction_mut(),
        );
        crate::app::widget_pipeline::register_widget_tree(
            ctx.content.content_widget_mut(),
            ctx.root.interaction_mut(),
        );
        // Drain registration lifecycle events (WidgetAdded).
        let _ = ctx.root.interaction_mut().drain_events();

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
            let (interaction, focus) = ctx.root.interaction_and_focus_mut();
            interaction.request_focus(id, focus);
            // Drain focus lifecycle events.
            let _ = interaction.drain_events();
        }
    }
}
