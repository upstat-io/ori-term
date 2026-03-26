//! Overlay and context menu event dispatch.

use std::time::Instant;

use oriterm_ui::geometry::Rect;
use oriterm_ui::overlay::{OverlayEventResult, Placement};
use oriterm_ui::widget_id::WidgetId;
use oriterm_ui::widgets::WidgetAction;
use oriterm_ui::widgets::menu::{MenuEntry, MenuStyle, MenuWidget};

use crate::config::Config;

use super::super::{App, context_menu};

impl App {
    /// Clear all transient popup overlays in a terminal window.
    pub(in crate::app) fn clear_window_popups(&mut self, window_id: winit::window::WindowId) {
        let mut removed = 0;
        if let Some(ctx) = self.windows.get_mut(&window_id) {
            removed = ctx.root.clear_popups();
            if removed > 0 {
                ctx.context_menu = None;
                ctx.root.mark_dirty();
                ctx.root.set_urgent_redraw(true);
            }
        }
        if removed > 0 {
            self.pending_dropdown_id = None;
        }
    }

    /// Process the result of routing an event through the overlay manager.
    pub(in crate::app) fn handle_overlay_result(&mut self, result: OverlayEventResult) {
        match result {
            OverlayEventResult::Delivered { response, .. } => {
                log::debug!("overlay Delivered: action={:?}", response.action);

                if response.handled {
                    if let Some(ctx) = self.focused_ctx_mut() {
                        ctx.root.set_urgent_redraw(true);
                    }
                }

                let Some(action) = response.action else {
                    if response.handled {
                        if let Some(ctx) = self.focused_ctx_mut() {
                            ctx.root.mark_dirty();
                        }
                    }
                    return;
                };

                match action {
                    WidgetAction::DismissOverlay(_) => {
                        self.dismiss_topmost_overlay();
                    }
                    WidgetAction::MoveOverlay { delta_x, delta_y } => {
                        if let Some(ctx) = self.focused_ctx_mut() {
                            ctx.root.overlays_mut().offset_topmost(delta_x, delta_y);
                            ctx.root.mark_dirty();
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
                    // All other actions: mark dirty if the event was handled (visual feedback).
                    _ => {
                        if response.handled {
                            if let Some(ctx) = self.focused_ctx_mut() {
                                ctx.root.mark_dirty();
                                ctx.root.set_urgent_redraw(true);
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
                        ctx.root.mark_dirty();
                        ctx.root.set_urgent_redraw(true);
                    }
                } else {
                    // Top-level overlay dismissed (Escape, click-outside).
                    if let Some(ctx) = self.focused_ctx_mut() {
                        ctx.context_menu = None;
                        ctx.root.mark_dirty();
                        ctx.root.set_urgent_redraw(true);
                    }
                }
            }
            OverlayEventResult::Blocked => {
                log::debug!("overlay Blocked");
            }
            OverlayEventResult::PassThrough => {}
        }
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
            ctx.root.mark_dirty();
        }
    }

    /// Dismiss the topmost overlay.
    fn dismiss_topmost_overlay(&mut self) {
        let now = Instant::now();
        if let Some(ctx) = self.focused_ctx_mut() {
            ctx.root.dismiss_topmost(now);
            ctx.root.mark_dirty();
            ctx.root.set_urgent_redraw(true);
        }
        if self.pending_dropdown_id.is_some() {
            // Only the dropdown popup was dismissed.
            self.pending_dropdown_id = None;
        } else {
            // Top-level overlay dismissed.
            if let Some(ctx) = self.focused_ctx_mut() {
                ctx.context_menu = None;
            }
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
            ctx.root.mark_dirty();
        }
    }

    /// Clear context menu state and dismiss its overlay.
    fn dismiss_context_menu(&mut self) {
        if let Some(ctx) = self.focused_ctx_mut() {
            ctx.context_menu = None;
            ctx.root.clear_popups();
            ctx.root.mark_dirty();
            ctx.root.set_urgent_redraw(true);
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
        style.shadow_color = self.ui_theme.shadow;
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
            ctx.root
                .replace_popup(Box::new(widget), anchor, Placement::BelowFlush, now);
            ctx.root.mark_dirty();
            ctx.root.set_urgent_redraw(true);
        }
    }

    /// Handle selection from a dropdown popup.
    ///
    /// Dismisses the popup overlay and propagates the selection to the
    /// overlay beneath so widgets can update their display.
    fn dispatch_dropdown_selection(&mut self, index: usize) {
        let dropdown_id = self.pending_dropdown_id.take();
        let Some(id) = dropdown_id else {
            return;
        };

        // Dismiss the popup overlay (topmost).
        let now = Instant::now();
        if let Some(ctx) = self.focused_ctx_mut() {
            ctx.root.dismiss_topmost(now);
            ctx.root.mark_dirty();
            ctx.root.set_urgent_redraw(true);
        }

        // Propagate the selection to the overlay beneath.
        let action = WidgetAction::Selected { id, index };
        if let Some(ctx) = self.focused_ctx_mut() {
            ctx.root.overlays_mut().accept_action_topmost(&action);
            ctx.root.mark_dirty();
        }
    }
}
