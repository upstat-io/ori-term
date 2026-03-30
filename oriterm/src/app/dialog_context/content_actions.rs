//! Dialog content action dispatch.
//!
//! Handles widget actions emitted by dialog content (settings panel,
//! confirmation dialog): Save, Cancel, OK, dropdown open, selection,
//! toggle, and keyboard routing to confirmation dialog widgets.

use oriterm_ui::widgets::settings_panel::SettingsPanel;
use oriterm_ui::widgets::sidebar_nav::FooterTarget;
use oriterm_ui::widgets::{Widget, WidgetAction};
use winit::window::WindowId;

use crate::app::settings_overlay;
use crate::app::widget_pipeline::collect_focusable_ids;
use crate::event::ConfirmationKind;

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
            WidgetAction::FooterAction(ref target) => {
                self.handle_footer_action(window_id, target);
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
            | WidgetAction::WindowClose
            | WidgetAction::SettingsUnsaved(_)
            | WidgetAction::PageDirty { .. }
            | WidgetAction::TabTitleChanged { .. } => {}
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
    ///
    /// Rebuilds the entire form panel from the default config so all widgets
    /// (dropdowns, toggles, sliders, number inputs) reflect the new values.
    fn reset_dialog_settings(&mut self, window_id: WindowId) {
        let ui_theme = self.ui_theme;
        let Some(ctx) = self.dialogs.get_mut(&window_id) else {
            return;
        };
        let DialogContent::Settings {
            pending_config,
            original_config,
            ids,
            panel,
            active_page,
        } = &mut ctx.content
        else {
            return;
        };
        log::info!("settings dialog: resetting to defaults");
        **pending_config = Config::default();
        // Keep `original_config` pinned to the persisted on-disk config so dirty
        // detection correctly treats reset-to-defaults as an unsaved change when
        // the persisted config differs from defaults (TPR-12-011).

        // Rebuild the form widgets so they reflect the default config values.
        // Preserve the current page so the user stays where they were.
        let (content, new_ids, footer_ids) = settings_overlay::form_builder::build_settings_dialog(
            pending_config,
            &ui_theme,
            *active_page,
            ctx.scale_factor.factor(),
            f64::from(pending_config.window.effective_opacity()),
            None,
        );
        // Deregister old panel to avoid leaking InteractionManager state (TPR-11-009).
        crate::app::widget_pipeline::deregister_widget_tree(
            &mut **panel,
            ctx.root.interaction_mut(),
        );
        **ids = new_ids;
        **panel = SettingsPanel::embedded(content, footer_ids);
        // Register new tree; WidgetAdded events delivered next frame (TPR-04-003).
        crate::app::widget_pipeline::register_widget_tree(&mut **panel, ctx.root.interaction_mut());

        // Rebuild key contexts so keymap scope gating covers the new widgets.
        ctx.root.key_contexts_mut().clear();
        oriterm_ui::action::collect_key_contexts(&mut **panel, ctx.root.key_contexts_mut());

        // Rebuild focus order and sync InteractionManager if the previously
        // focused widget disappeared from the rebuilt tree.
        let mut focusable = Vec::new();
        collect_focusable_ids(&mut **panel, &mut focusable);
        ctx.root.sync_focus_order(focusable);

        // Rebuild parent map for dirty-ancestor tracking (TPR-04-002).
        if let Some(r) = ctx.renderer.as_ref() {
            let s = ctx.scale_factor.factor() as f32;
            ctx.root
                .interaction_mut()
                .set_parent_map(super::content_parent_map(
                    &**panel,
                    r,
                    s,
                    &ctx.text_cache,
                    &ctx.surface_config,
                    &ui_theme,
                ));
        }

        // Recompute hot path to preserve hover on surviving widgets (TPR-04-007).
        super::recompute_dialog_hot_path(
            &mut ctx.root,
            &**panel,
            ctx.last_cursor_pos,
            ctx.renderer.as_ref(),
            ctx.scale_factor.factor() as f32,
            &ctx.text_cache,
            &ctx.surface_config,
            &ui_theme,
        );

        // Invalidate all caches.
        ctx.cached_layout = None;

        let dirty = *pending_config != *original_config;
        let title = if dirty {
            "Settings \u{2022}"
        } else {
            "Settings"
        };
        ctx.window.set_title(title);

        // Update per-page dirty dots after reset.
        panel.accept_action(&WidgetAction::SettingsUnsaved(dirty));
        let page_dirty = settings_overlay::per_page_dirty(pending_config, original_config);
        for (page, &is_dirty) in page_dirty.iter().enumerate() {
            panel.accept_action(&WidgetAction::PageDirty {
                page,
                dirty: is_dirty,
            });
        }

        ctx.request_urgent_redraw();
    }

    /// Dispatch a settings widget action to update the pending config.
    ///
    /// Always propagates to the panel via `accept_action`, even when the config
    /// is unchanged — widgets may need to update visuals (e.g. sidebar page
    /// switching, dropdown selection display). After config changes, compares
    /// pending vs original to track dirty state.
    pub(super) fn dispatch_dialog_settings_action(
        &mut self,
        window_id: WindowId,
        action: &WidgetAction,
    ) {
        let ui_theme = self.ui_theme;
        let Some(ctx) = self.dialogs.get_mut(&window_id) else {
            return;
        };
        let DialogContent::Settings {
            ids,
            pending_config,
            panel,
            original_config,
            active_page,
        } = &mut ctx.content
        else {
            return;
        };

        let config_changed =
            settings_overlay::action_handler::handle_settings_action(action, ids, pending_config);
        if config_changed {
            log::info!("settings dialog: pending config updated (deferred until Save)");
            // Update dirty indicator — shows in chrome title bar, taskbar, and footer.
            let dirty = **pending_config != **original_config;
            let title = if dirty {
                "Settings \u{2022}"
            } else {
                "Settings"
            };
            ctx.window.set_title(title);
            panel.accept_action(&WidgetAction::SettingsUnsaved(dirty));

            // Per-page dirty state drives the sidebar modified dots.
            let page_dirty = settings_overlay::per_page_dirty(pending_config, original_config);
            for (page, &is_dirty) in page_dirty.iter().enumerate() {
                panel.accept_action(&WidgetAction::PageDirty {
                    page,
                    dirty: is_dirty,
                });
            }
        }

        // Always propagate — widgets update visuals (page switch, selection).
        let widget_handled = panel.accept_action(action);

        // Track page switches for reset preservation.
        // Only sidebar nav selections represent actual page switches — not
        // scheme card, cursor picker, or dropdown selections (TPR-11-001).
        if let WidgetAction::Selected { id, index } = action {
            if widget_handled && *id == ids.sidebar_id && *index < 8 {
                *active_page = *index;

                // Page switch: register the new page's widgets and rebuild
                // focus/keymap state so keyboard navigation targets the
                // correct page. WidgetAdded events stay pending for
                // delivery on the next render frame (TPR-04-003).
                crate::app::widget_pipeline::register_widget_tree(
                    &mut **panel,
                    ctx.root.interaction_mut(),
                );
                // GC stale registrations from the old page (TPR-11-009).
                let root_id = ctx.root.widget().id();
                let mut valid = vec![root_id];
                crate::app::widget_pipeline::collect_all_widget_ids(&mut **panel, &mut valid);
                let stale = ctx.root.interaction_mut().gc_stale_widgets(&valid);
                ctx.root.mark_widgets_prepaint_dirty(&stale);

                ctx.root.key_contexts_mut().clear();
                oriterm_ui::action::collect_key_contexts(&mut **panel, ctx.root.key_contexts_mut());

                let mut focusable = Vec::new();
                collect_focusable_ids(&mut **panel, &mut focusable);
                ctx.root.sync_focus_order(focusable);

                // Rebuild parent map for dirty-ancestor tracking (TPR-04-002).
                if let Some(r) = ctx.renderer.as_ref() {
                    let s = ctx.scale_factor.factor() as f32;
                    let pm = super::content_parent_map(
                        &**panel,
                        r,
                        s,
                        &ctx.text_cache,
                        &ctx.surface_config,
                        &ui_theme,
                    );
                    ctx.root.interaction_mut().set_parent_map(pm);
                }

                // Recompute hot path to preserve hover on surviving widgets (TPR-04-007).
                super::recompute_dialog_hot_path(
                    &mut ctx.root,
                    &**panel,
                    ctx.last_cursor_pos,
                    ctx.renderer.as_ref(),
                    ctx.scale_factor.factor() as f32,
                    &ctx.text_cache,
                    &ctx.surface_config,
                    &ui_theme,
                );
            }
        }

        if config_changed || widget_handled {
            // Invalidate both layout caches: the dialog context's cached
            // layout tree AND the panel's internal paint-time cache. Page
            // switching changes which widgets appear in the layout tree, so
            // stale cached layouts would cause hit testing against the old
            // page's widgets — making the new page's controls unresponsive.
            ctx.cached_layout = None;
            panel.invalidate_cache();
            ctx.request_urgent_redraw();
        }
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

    /// Handle a sidebar footer action (config path click, update link click).
    fn handle_footer_action(&self, window_id: WindowId, target: &FooterTarget) {
        let Some(ctx) = self.dialogs.get(&window_id) else {
            return;
        };
        match target {
            FooterTarget::ConfigPath => {
                let DialogContent::Settings { .. } = &ctx.content else {
                    return;
                };
                let path = crate::config::config_path();
                log::info!("footer: opening config file: {}", path.display());
                if let Err(e) = open::that(&path) {
                    log::warn!("footer: failed to open config file: {e}");
                }
            }
            FooterTarget::UpdateLink(url) => {
                if let Some(url) = url {
                    log::info!("footer: opening update URL: {url}");
                    if let Err(e) = open::that(url) {
                        log::warn!("footer: failed to open update URL: {e}");
                    }
                } else {
                    log::info!("footer: update link clicked (no URL configured)");
                }
            }
        }
    }
}
