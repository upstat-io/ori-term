//! Overlay and context menu event dispatch.

use std::time::Instant;

use oriterm_ui::geometry::Rect;
use oriterm_ui::overlay::{OverlayEventResult, Placement};
use oriterm_ui::widget_id::WidgetId;
use oriterm_ui::widgets::WidgetAction;
use oriterm_ui::widgets::menu::{MenuEntry, MenuStyle, MenuWidget};

use crate::config::Config;

use super::super::{App, context_menu, settings_overlay};

impl App {
    /// Process the result of routing an event through the overlay manager.
    pub(in crate::app) fn handle_overlay_result(&mut self, result: OverlayEventResult) {
        match result {
            OverlayEventResult::Delivered { response, .. } => {
                log::debug!("overlay Delivered: action={:?}", response.action);

                let Some(action) = response.action else {
                    if response.response.is_handled() {
                        if let Some(ctx) = self.focused_ctx_mut() {
                            ctx.dirty = true;
                        }
                    }
                    return;
                };

                // Settings panel actions: try matching against known control IDs.
                if self.try_dispatch_settings_action(&action) {
                    return;
                }

                // Non-settings actions.
                match action {
                    WidgetAction::SaveSettings => {
                        self.save_settings();
                    }
                    WidgetAction::CancelSettings => {
                        self.cancel_settings();
                    }
                    WidgetAction::DismissOverlay(_) => {
                        self.dismiss_topmost_overlay();
                    }
                    WidgetAction::MoveOverlay { delta_x, delta_y } => {
                        if let Some(ctx) = self.focused_ctx_mut() {
                            ctx.overlays.offset_topmost(delta_x, delta_y);
                            ctx.dirty = true;
                        }
                    }
                    WidgetAction::OpenDropdown {
                        id,
                        options,
                        selected,
                        anchor,
                    } => {
                        self.open_dropdown_popup(id, options, selected, anchor);
                    }
                    WidgetAction::Selected { index, .. } if self.pending_dropdown_id.is_some() => {
                        self.dispatch_dropdown_selection(index);
                    }
                    WidgetAction::Selected { index, .. } => {
                        log::info!("overlay Selected: index={index}");
                        self.dispatch_context_action(index);
                    }
                    _ => {
                        if response.response.is_handled() {
                            if let Some(ctx) = self.focused_ctx_mut() {
                                ctx.dirty = true;
                            }
                        }
                    }
                }
            }
            OverlayEventResult::Dismissed(_id) => {
                log::info!("overlay Dismissed");
                if self.pending_dropdown_id.is_some() {
                    // A dropdown popup was dismissed — clear only popup state.
                    // The settings panel beneath remains functional.
                    self.pending_dropdown_id = None;
                    if let Some(ctx) = self.focused_ctx_mut() {
                        ctx.dirty = true;
                    }
                } else {
                    // Top-level overlay dismissed (Escape, click-outside).
                    // Discard any pending settings changes.
                    self.settings_pending = None;
                    if let Some(ctx) = self.focused_ctx_mut() {
                        ctx.context_menu = None;
                        ctx.dirty = true;
                    }
                    self.settings_ids = None;
                }
            }
            OverlayEventResult::Blocked => {
                log::debug!("overlay Blocked");
            }
            OverlayEventResult::PassThrough => {}
        }
    }

    /// Try dispatching a widget action as a settings control change.
    ///
    /// Mutates the pending config copy — does NOT touch `self.config`.
    /// Changes only take effect when the user clicks Save.
    fn try_dispatch_settings_action(&mut self, action: &WidgetAction) -> bool {
        let Some(ids) = &self.settings_ids else {
            return false;
        };
        let Some(pending) = self.settings_pending.as_mut() else {
            return false;
        };
        if !settings_overlay::action_handler::handle_settings_action(action, ids, pending) {
            return false;
        }

        log::info!("settings: pending config updated (deferred until Save)");
        if let Some(ctx) = self.focused_ctx_mut() {
            ctx.dirty = true;
        }
        true
    }

    /// Save settings: apply pending config, persist to disk, and dismiss.
    fn save_settings(&mut self) {
        if let Some(pending) = self.settings_pending.take() {
            log::info!("settings: applying and saving to disk");
            let old_config = std::mem::replace(&mut self.config, pending);
            self.apply_settings_change(old_config);
            self.config.save();
        }
        self.dismiss_topmost_overlay();
    }

    /// Cancel settings: discard pending changes and dismiss.
    fn cancel_settings(&mut self) {
        self.settings_pending = None;
        self.dismiss_topmost_overlay();
    }

    /// Apply config changes after Save commits the pending config.
    ///
    /// The `apply_*_changes` methods compare a "new" config against
    /// `self.config` (the "old" state). We temporarily swap the old config
    /// back so comparison-based methods detect the delta correctly.
    pub(in crate::app) fn apply_settings_change(&mut self, old_config: Config) {
        let new_config = std::mem::replace(&mut self.config, old_config);

        self.apply_font_changes(&new_config);
        self.apply_color_changes(&new_config);
        self.apply_cursor_changes(&new_config);
        self.apply_window_changes(&new_config);

        // Restore the new config.
        self.config = new_config;

        // Update UI theme.
        let new_theme = super::super::resolve_ui_theme(&self.config);
        if new_theme != self.ui_theme {
            self.ui_theme = new_theme;
            for ctx in self.windows.values_mut() {
                ctx.tab_bar.apply_theme(&self.ui_theme);
            }
        }

        // Invalidate render caches and mark dirty.
        for ctx in self.windows.values_mut() {
            ctx.pane_cache.invalidate_all();
            ctx.dirty = true;
        }
    }

    /// Dismiss the topmost overlay.
    fn dismiss_topmost_overlay(&mut self) {
        let now = Instant::now();
        if let Some(ctx) = self.focused_ctx_mut() {
            ctx.overlays
                .begin_dismiss_topmost(&mut ctx.layer_tree, &mut ctx.layer_animator, now);
            ctx.dirty = true;
        }
        if self.pending_dropdown_id.is_some() {
            // Only the dropdown popup was dismissed.
            self.pending_dropdown_id = None;
        } else {
            // Top-level overlay dismissed.
            if let Some(ctx) = self.focused_ctx_mut() {
                ctx.context_menu = None;
            }
            self.settings_ids = None;
        }
    }

    /// Dispatch a context menu selection by index.
    fn dispatch_context_action(&mut self, index: usize) {
        // Resolve the action from the context menu state.
        let action = self
            .focused_ctx()
            .and_then(|ctx| ctx.context_menu.as_ref())
            .and_then(|cm| cm.resolve(index))
            .cloned();

        // Dismiss the menu overlay.
        self.dismiss_context_menu();

        let Some(action) = action else {
            return;
        };

        match action {
            context_menu::ContextAction::Settings => {
                log::info!("settings action dispatched, sending OpenSettings event");
                self.event_proxy.send(crate::event::TermEvent::OpenSettings);
            }
            context_menu::ContextAction::About => {
                log::info!("about action dispatched");
                // TODO: open About dialog
            }
            context_menu::ContextAction::CloseTab(idx) => {
                self.close_tab_at_index(idx);
            }
            context_menu::ContextAction::DuplicateTab(_idx) => {
                if let Some(win_id) = self.active_window {
                    self.new_tab_in_window(win_id);
                }
            }
            context_menu::ContextAction::MoveToNewWindow(tab_id) => {
                self.move_tab_to_new_window_deferred(tab_id);
            }
            context_menu::ContextAction::Copy => {
                self.copy_selection();
            }
            context_menu::ContextAction::Paste => {
                self.paste_from_clipboard();
            }
            context_menu::ContextAction::SelectAll => {
                self.select_all_in_pane();
            }
            context_menu::ContextAction::NewTab => {
                if let Some(win_id) = self.active_window {
                    self.new_tab_in_window(win_id);
                }
            }
        }

        if let Some(ctx) = self.focused_ctx_mut() {
            ctx.dirty = true;
        }
    }

    /// Clear context menu state and dismiss its overlay.
    fn dismiss_context_menu(&mut self) {
        if let Some(ctx) = self.focused_ctx_mut() {
            ctx.context_menu = None;
            let now = Instant::now();
            ctx.overlays
                .begin_dismiss_topmost(&mut ctx.layer_tree, &mut ctx.layer_animator, now);
            ctx.dirty = true;
        }
    }

    /// Open a dropdown popup overlay below the trigger widget.
    fn open_dropdown_popup(
        &mut self,
        dropdown_id: WidgetId,
        options: Vec<String>,
        selected: usize,
        anchor: Rect,
    ) {
        let entries: Vec<MenuEntry> = options
            .into_iter()
            .map(|label| MenuEntry::Item { label })
            .collect();

        // Dropdown list style: flush below trigger, matching width, scrollable.
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
            // Pre-scroll to show the selected item.
            widget.ensure_visible(selected);
        }

        let now = Instant::now();
        self.pending_dropdown_id = Some(dropdown_id);

        if let Some(ctx) = self.focused_ctx_mut() {
            ctx.overlays.push_overlay(
                Box::new(widget),
                anchor,
                Placement::BelowFlush,
                &mut ctx.layer_tree,
                &mut ctx.layer_animator,
                now,
            );
            ctx.dirty = true;
            ctx.ui_stale = true;
        }
    }

    /// Handle selection from a dropdown popup.
    ///
    /// Routes the selection as a `WidgetAction::Selected` with the
    /// dropdown's widget ID so the settings action handler can match it.
    fn dispatch_dropdown_selection(&mut self, index: usize) {
        let dropdown_id = self.pending_dropdown_id.take();
        let Some(id) = dropdown_id else {
            return;
        };

        // Dismiss the popup overlay (topmost).
        let now = Instant::now();
        if let Some(ctx) = self.focused_ctx_mut() {
            ctx.overlays
                .begin_dismiss_topmost(&mut ctx.layer_tree, &mut ctx.layer_animator, now);
            ctx.dirty = true;
        }

        // Route the selection through the settings action handler.
        let action = WidgetAction::Selected { id, index };
        if self.try_dispatch_settings_action(&action) {
            // Propagate back to the dropdown widget so it updates its display.
            if let Some(ctx) = self.focused_ctx_mut() {
                ctx.overlays.accept_action_topmost(&action);
                ctx.dirty = true;
            }
            return;
        }

        log::info!("dropdown selection: id={id:?}, index={index}, no handler matched");
    }
}
