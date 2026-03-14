//! Settings overlay — modal settings panel pushed into the overlay manager.
//!
//! Replaces the separate settings window (`settings_ui`) with a modal overlay
//! within the active terminal window.

pub(in crate::app) mod action_handler;
pub(in crate::app) mod form_builder;

use std::time::Instant;

use oriterm_ui::overlay::Placement;
use oriterm_ui::widgets::settings_panel::SettingsPanel;

pub(in crate::app) use form_builder::SettingsIds;

use super::App;

impl App {
    /// Opens the settings panel as a centered modal overlay in the focused window.
    ///
    /// Bails if no focused window exists or if a modal is already open.
    /// Retained for overlay-based settings fallback (e.g. if dialog creation fails).
    #[allow(dead_code, reason = "retained for overlay fallback path")]
    pub(in crate::app) fn open_settings_overlay(&mut self) {
        // Check guard: bail if no window or modal already open.
        let has_modal = self
            .focused_ctx()
            .is_some_and(|ctx| ctx.overlays.has_modal());
        if has_modal || self.focused_ctx().is_none() {
            return;
        }

        // Create a working copy of the config for pending edits.
        self.settings_pending = Some(self.config.clone());

        // Build form from current config (no borrow on self.windows).
        let (mut form, ids) = form_builder::build_settings_form(&self.config);
        self.settings_ids = Some(ids);

        // Compute aligned label column widths before first layout/draw.
        if let Some(ctx) = self.focused_ctx() {
            let scale = ctx.window.scale_factor().factor() as f32;
            if let Some(renderer) = ctx.renderer.as_ref() {
                let measurer = crate::font::CachedTextMeasurer::new(
                    crate::font::UiFontMeasurer::new(renderer.active_ui_collection(), scale),
                    &ctx.text_cache,
                    scale,
                );
                form.compute_label_widths(&measurer, &self.ui_theme);
            }
        }

        let panel = SettingsPanel::new(form);

        // Now take the mutable borrow for overlay push.
        let Some(ctx) = self.focused_ctx_mut() else {
            return;
        };
        let viewport = ctx.overlays.viewport();
        let now = Instant::now();
        ctx.overlays.push_modal(
            Box::new(panel),
            viewport,
            Placement::Center,
            &mut ctx.layer_tree,
            &mut ctx.layer_animator,
            now,
        );
        ctx.dirty = true;
        ctx.ui_stale = true;
    }
}
