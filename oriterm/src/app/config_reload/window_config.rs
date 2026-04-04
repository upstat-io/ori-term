//! Window transparency, blur, decoration, and tab bar config changes.

use super::super::App;
use crate::config::Config;

impl App {
    /// Detect and apply window transparency/blur changes.
    ///
    /// Iterates ALL windows — each must receive the updated transparency
    /// settings based on its focus state, not just the focused one.
    pub(in crate::app) fn apply_window_changes(&mut self, new: &Config) {
        let opacity_changed =
            (new.window.effective_opacity() - self.config.window.effective_opacity()).abs()
                > f32::EPSILON;
        let unfocused_opacity_changed = (new.window.effective_unfocused_opacity()
            - self.config.window.effective_unfocused_opacity())
        .abs()
            > f32::EPSILON;
        let blur_changed = new.window.blur != self.config.window.blur;

        if opacity_changed || unfocused_opacity_changed || blur_changed {
            for ctx in self.windows.values() {
                let focused = ctx.window.window().has_focus();
                let opacity = if focused {
                    new.window.effective_opacity()
                } else {
                    new.window.effective_unfocused_opacity()
                };
                let blur = new.window.blur && opacity < 1.0;
                ctx.window.set_transparency(opacity, blur);
            }

            log::info!(
                "config reload: opacity={:.2}, unfocused_opacity={:.2}, blur={}",
                new.window.effective_opacity(),
                new.window.effective_unfocused_opacity(),
                new.window.blur
            );
        }

        // Decoration mode changed — apply to existing windows where possible.
        // winit's `set_decorations(bool)` handles the decorated/frameless toggle.
        // macOS-specific modes (TransparentTitlebar, Buttonless) require window
        // recreation and are logged as requiring restart.
        let decorations_changed = new.window.decorations != self.config.window.decorations;
        if decorations_changed {
            let mode = super::super::init::decoration_to_mode(new.window.decorations);
            let winit_decorated = oriterm_ui::window::resolve_winit_decorations(mode);
            for ctx in self.windows.values() {
                ctx.window.window().set_decorations(winit_decorated);
            }
            log::info!(
                "config reload: decorations={:?} (winit decorated={})",
                new.window.decorations,
                winit_decorated,
            );
            // macOS titlebar transparency and button visibility are set at
            // window creation time and cannot be changed at runtime via winit.
            #[cfg(target_os = "macos")]
            {
                let old_mode =
                    super::super::init::decoration_to_mode(self.config.window.decorations);
                if old_mode.macos_requires_restart(mode) {
                    log::warn!("config reload: macOS titlebar mode change requires app restart");
                }
            }
        }

        // Tab bar position or style changed — relayout all windows since
        // chrome height changes when the tab bar is hidden/shown.
        let position_changed = new.window.tab_bar_position != self.config.window.tab_bar_position;
        let style_changed = new.window.tab_bar_style != self.config.window.tab_bar_style;
        if position_changed || style_changed {
            // Apply new metrics to all tab bar widgets before relayout.
            let metrics = super::super::init::metrics_from_style(new.window.tab_bar_style);
            if style_changed {
                for ctx in self.windows.values_mut() {
                    ctx.tab_bar.set_metrics(metrics);
                }
            }
            // Publish the effective chrome height to macOS — 0.0 when
            // Hidden so fullscreen notification callbacks center traffic
            // lights correctly. Must run on both position and style
            // changes (not just style) so switching to Hidden publishes 0.
            #[cfg(target_os = "macos")]
            {
                let hidden = new.window.tab_bar_position == crate::config::TabBarPosition::Hidden;
                let effective_h = if hidden { 0.0 } else { metrics.height };
                crate::window_manager::platform::macos::set_tab_bar_height(effective_h);
            }
            let window_sizes: Vec<_> = self
                .windows
                .iter()
                .map(|(&id, ctx)| {
                    let (w, h) = ctx.window.size_px();
                    (id, w, h)
                })
                .collect();
            for (winit_id, w, h) in window_sizes {
                self.sync_grid_layout(winit_id, w, h);
                self.refresh_platform_rects(winit_id);
            }
            log::info!(
                "config reload: tab_bar_position={:?}, tab_bar_style={:?}",
                new.window.tab_bar_position,
                new.window.tab_bar_style
            );
        }
    }
}
