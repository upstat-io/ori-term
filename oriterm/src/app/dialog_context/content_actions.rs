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
                self.cancel_dialog_settings(window_id);
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

    /// Persist the already-applied settings to disk and close the dialog.
    ///
    /// Live preview means `self.config` already reflects the pending changes
    /// (applied in `dispatch_dialog_settings_action`). Save just persists.
    fn save_dialog_settings(&mut self, window_id: WindowId) {
        log::info!("settings dialog: saving to disk");
        self.config.save();
        self.close_dialog(window_id);
    }

    /// Reset the pending settings config to defaults.
    ///
    /// Rebuilds the entire form panel from the default config so all widgets
    /// (dropdowns, toggles, sliders, number inputs) reflect the new values.
    fn reset_dialog_settings(&mut self, window_id: WindowId) {
        let ui_theme = self.ui_theme;
        // Scope the dialog borrow so we can call apply_settings_change after.
        let reset_config = {
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

            let (content, new_ids, footer_ids) =
                settings_overlay::form_builder::build_settings_dialog(
                    pending_config,
                    &ui_theme,
                    *active_page,
                    ctx.scale_factor.factor(),
                    f64::from(pending_config.window.effective_opacity()),
                    None,
                );
            crate::app::widget_pipeline::deregister_widget_tree(
                &mut **panel,
                ctx.root.interaction_mut(),
            );
            **ids = new_ids;
            **panel = SettingsPanel::embedded(content, footer_ids);
            super::rebuild_dialog_page_state(
                &mut ctx.root,
                &mut **panel,
                ctx.renderer.as_ref(),
                ctx.scale_factor,
                &ctx.text_cache,
                &ctx.surface_config,
                ctx.last_cursor_pos,
                &ui_theme,
            );
            ctx.cached_layout = None;

            let dirty = *pending_config != *original_config;
            ctx.window.set_title(if dirty {
                "Settings \u{2022}"
            } else {
                "Settings"
            });
            panel.accept_action(&WidgetAction::SettingsUnsaved(dirty));
            let page_dirty = settings_overlay::per_page_dirty(pending_config, original_config);
            for (page, &is_dirty) in page_dirty.iter().enumerate() {
                panel.accept_action(&WidgetAction::PageDirty {
                    page,
                    dirty: is_dirty,
                });
            }

            let cfg = (**pending_config).clone();
            ctx.request_urgent_redraw();
            cfg
        };

        // Live-apply the reset config (dialog borrow released).
        let old = std::mem::replace(&mut self.config, reset_config);
        self.apply_settings_change(old);
    }

    /// Revert live-previewed changes and close the dialog.
    ///
    /// Restores `self.config` to the `original_config` snapshot taken when
    /// the dialog opened, then applies the delta so terminal windows revert.
    fn cancel_dialog_settings(&mut self, window_id: WindowId) {
        let original = {
            let Some(ctx) = self.dialogs.get_mut(&window_id) else {
                self.close_dialog(window_id);
                return;
            };
            let DialogContent::Settings {
                original_config, ..
            } = &ctx.content
            else {
                self.close_dialog(window_id);
                return;
            };
            (**original_config).clone()
        };

        // Revert: swap original back in, apply delta to undo live preview.
        if self.config != original {
            log::info!("settings dialog: reverting live-previewed changes");
            let previewed = std::mem::replace(&mut self.config, original);
            self.apply_settings_change(previewed);
        }

        self.close_dialog(window_id);
    }

    /// Dispatch a settings widget action to update the pending config.
    ///
    /// Changes are applied live (immediate preview in terminal windows).
    /// Save persists to disk; Cancel reverts to the original config.
    pub(super) fn dispatch_dialog_settings_action(
        &mut self,
        window_id: WindowId,
        action: &WidgetAction,
    ) {
        // Snapshot for live-apply: clone the pending config after mutation,
        // then apply outside the dialog borrow scope.
        let live_apply_pending = {
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

            let config_changed = settings_overlay::action_handler::handle_settings_action(
                action,
                ids,
                pending_config,
            );
            if config_changed {
                // Update dirty indicator.
                let dirty = **pending_config != **original_config;
                let title = if dirty {
                    "Settings \u{2022}"
                } else {
                    "Settings"
                };
                ctx.window.set_title(title);
                panel.accept_action(&WidgetAction::SettingsUnsaved(dirty));

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
            if let WidgetAction::Selected { id, index } = action {
                if widget_handled && *id == ids.sidebar_id && *index < 8 {
                    *active_page = *index;
                    super::rebuild_dialog_page_state(
                        &mut ctx.root,
                        &mut **panel,
                        ctx.renderer.as_ref(),
                        ctx.scale_factor,
                        &ctx.text_cache,
                        &ctx.surface_config,
                        ctx.last_cursor_pos,
                        &ui_theme,
                    );
                }
            }

            // Clone before releasing the destructured borrow on ctx.content.
            let apply = if config_changed {
                Some((**pending_config).clone())
            } else {
                None
            };

            if config_changed || widget_handled {
                ctx.cached_layout = None;
                panel.invalidate_cache();
                ctx.request_urgent_redraw();
            }

            apply
        };

        // Live-apply: swap pending config into self.config so terminal
        // windows immediately reflect the change. The dialog borrow is
        // released, so we can call apply_settings_change freely.
        if let Some(pending) = live_apply_pending {
            let old = std::mem::replace(&mut self.config, pending);
            self.apply_settings_change(old);
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
