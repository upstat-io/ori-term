//! Configuration hot-reload — applies config changes without restart.
//!
//! When the config file watcher detects changes, [`App::apply_config_reload`]
//! loads the new config, computes deltas, and applies only what changed:
//! fonts, colors, cursor style, window, behavior, bell, keybindings.

mod color_config;
mod font_config;

#[cfg(test)]
pub(crate) use color_config::apply_color_overrides;
pub(crate) use color_config::build_palette_from_config;
pub(crate) use font_config::{
    apply_font_config, apply_font_config_to_ui_sizes, resolve_atlas_filtering, resolve_hinting,
    resolve_subpixel_mode, resolve_subpixel_positioning,
};

use super::{App, DEFAULT_DPI};
use crate::config::{Config, FontConfig};
use crate::font::{FontByteCache, FontCollection, FontSet};
use crate::keybindings;

/// Minimum font size in points.
const MIN_FONT_SIZE: f32 = 4.0;

/// Maximum font size in points.
const MAX_FONT_SIZE: f32 = 72.0;

/// Compute the new font size after applying `delta`, clamped to valid range.
///
/// Returns `None` if the result is the same as `current` (no-op).
fn compute_zoomed_size(current: f32, delta: f32) -> Option<f32> {
    let new_size = (current + delta).clamp(MIN_FONT_SIZE, MAX_FONT_SIZE);
    if (new_size - current).abs() < f32::EPSILON {
        None
    } else {
        Some(new_size)
    }
}

/// Determine whether `current` font size should be reset to `configured`.
///
/// Returns `Some(configured)` if different from `current`, `None` if already matching.
fn compute_reset_size(current: f32, configured: f32) -> Option<f32> {
    if (current - configured).abs() < f32::EPSILON {
        None
    } else {
        Some(configured)
    }
}

#[cfg(test)]
mod tests;

use font_config::rebuild_ui_font_sizes;

impl App {
    /// Apply a reloaded configuration to the running application.
    ///
    /// Reloads the config file, compares against the current config, and
    /// applies only the fields that changed. On parse error, logs a warning
    /// and keeps the previous config.
    pub(super) fn apply_config_reload(&mut self) {
        let new_config = match Config::try_load() {
            Ok(c) => c,
            Err(e) => {
                log::warn!("config reload: {e}");
                return;
            }
        };

        self.apply_font_changes(&new_config);
        self.apply_color_changes(&new_config);
        self.apply_cursor_changes(&new_config);
        self.apply_window_changes(&new_config);
        self.apply_behavior_changes(&new_config);
        self.apply_image_changes(&new_config);
        self.apply_keybinding_changes(&new_config);

        // Bell config is read from self.config at usage sites, so
        // storing the new config is sufficient. Log if it changed.
        if new_config.bell != self.config.bell {
            log::info!("config reload: bell settings updated");
        }

        // Store the new config as current.
        self.config = new_config;

        // Update UI chrome theme if the config override changed.
        let new_theme = super::resolve_ui_theme(&self.config);
        if new_theme != self.ui_theme {
            self.ui_theme = new_theme;
            for ctx in self.windows.values_mut() {
                ctx.tab_bar.apply_theme(&self.ui_theme);
                ctx.status_bar.apply_theme(&self.ui_theme);
            }
        }

        // Invalidate pane render cache and mark dirty for redraw.
        for ctx in self.windows.values_mut() {
            ctx.pane_cache.invalidate_all();
            ctx.root.invalidation_mut().invalidate_all();
            ctx.root.damage_mut().reset();
            ctx.root.mark_dirty();
            ctx.ui_stale = true;
        }

        log::info!("config reload: applied successfully");
    }

    /// Detect and apply font changes (family, size, weight, features, fallback,
    /// hinting, subpixel mode, variations, codepoint map).
    ///
    /// Iterates ALL windows — each may have a different DPI scale factor,
    /// so each gets its own `FontCollection` at the correct physical DPI.
    pub(in crate::app) fn apply_font_changes(&mut self, new: &Config) {
        let old = &self.config.font;
        // Opacity crossing the 1.0 threshold changes auto-detected subpixel
        // mode (subpixel disabled for transparent backgrounds). Treat this as
        // a font change so the glyph atlas is rebuilt with the correct format.
        let opacity_affects_subpixel = (new.window.effective_opacity() < 1.0)
            != (self.config.window.effective_opacity() < 1.0);

        let font_changed = (new.font.size - old.size).abs() > f32::EPSILON
            || (new.font.line_height - old.line_height).abs() > f32::EPSILON
            || new.font.family != old.family
            || new.font.weight != old.weight
            || new.font.features != old.features
            || new.font.fallback != old.fallback
            || new.font.hinting != old.hinting
            || new.font.subpixel_mode != old.subpixel_mode
            || new.font.variations != old.variations
            || new.font.codepoint_map != old.codepoint_map
            || opacity_affects_subpixel;

        if !font_changed {
            return;
        }

        let weight = new.font.effective_weight();
        let mut cache = FontByteCache::new();
        let mut font_set =
            match FontSet::load_cached(new.font.family.as_deref(), weight, &mut cache) {
                Ok(fs) => fs,
                Err(e) => {
                    log::warn!("config reload: font load failed: {e}");
                    return;
                }
            };

        // Prepend user-configured fallback fonts before system fallbacks.
        let user_fb_families: Vec<&str> = new
            .font
            .fallback
            .iter()
            .map(|f| f.family.as_str())
            .collect();
        let fallback_map = font_set.prepend_user_fallbacks(&user_fb_families, &mut cache);

        // Update cached font set on App for future window creation.
        self.font_set = Some(font_set.clone());
        self.user_fallback_map.clone_from(&fallback_map);

        let (Some(gpu), Some(pipelines)) = (&self.gpu, self.pipelines.as_ref()) else {
            return;
        };

        // Iterate ALL windows — each may have a different DPI.
        for ctx in self.windows.values_mut() {
            let Some(renderer) = ctx.renderer.as_mut() else {
                continue;
            };
            let scale = ctx.window.scale_factor().factor();
            let hinting = resolve_hinting(&new.font, scale);
            let opacity = f64::from(new.window.effective_opacity());
            let format = resolve_subpixel_mode(&new.font, scale, opacity).glyph_format();
            let physical_dpi = DEFAULT_DPI * scale as f32;

            let fc = match FontCollection::new(
                font_set.clone(),
                new.font.size,
                physical_dpi,
                format,
                weight,
                hinting,
            ) {
                Ok(mut fc) => {
                    apply_font_config(&mut fc, &new.font, &fallback_map);
                    fc
                }
                Err(e) => {
                    log::warn!("config reload: font collection failed for window: {e}");
                    continue;
                }
            };

            let cell = fc.cell_metrics();
            log::info!(
                "config reload: font size={:.1}, cell={}x{}",
                new.font.size,
                cell.width,
                cell.height,
            );

            // UI registry always uses base weight 400 (Regular) — individual
            // UI text elements drive their weight via TextStyle.weight through
            // resolve_ui_weight(), not through the collection-level weight.
            rebuild_ui_font_sizes(
                renderer,
                &font_set,
                physical_dpi,
                format,
                hinting,
                400,
                &new.font,
                &fallback_map,
            );
            renderer.replace_font_collection(fc, gpu);
            renderer.set_subpixel_positioning(resolve_subpixel_positioning(&new.font, scale));
            let af = resolve_atlas_filtering(&new.font, scale);
            renderer.set_atlas_filtering(af, gpu, &pipelines.atlas_layout);
            ctx.text_cache.clear();
        }

        // Grid dimensions, terminal widget, PTY, and resize increments all
        // depend on cell metrics — sync_grid_layout handles them together.
        // Collect window IDs + sizes first (can't call &mut self methods
        // while iterating self.windows).
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
        }
    }

    /// Detect and apply color config changes.
    ///
    /// Resolves the effective theme (honoring config override), resolves the
    /// color scheme, builds the palette, applies overrides, and marks all lines
    /// dirty so colors are re-resolved.
    pub(in crate::app) fn apply_color_changes(&mut self, new: &Config) {
        if new.colors == self.config.colors {
            return;
        }

        // Resolve theme: config override takes priority over system detection.
        let theme = new
            .colors
            .resolve_theme(crate::platform::theme::system_theme);
        let palette = build_palette_from_config(&new.colors, theme);

        // Apply to all panes, not just the active one.
        let Some(mux) = self.mux.as_mut() else { return };
        for pane_id in mux.pane_ids() {
            mux.set_pane_theme(pane_id, theme, palette.clone());
        }

        log::info!("config reload: colors updated (theme={theme:?})");
    }

    /// Detect and apply cursor style and blink interval changes.
    pub(in crate::app) fn apply_cursor_changes(&mut self, new: &Config) {
        if new.terminal.cursor_style != self.config.terminal.cursor_style {
            let shape = new.terminal.cursor_style.to_shape();
            if let Some(mux) = self.mux.as_mut() {
                for pane_id in mux.pane_ids() {
                    mux.set_cursor_shape(pane_id, shape);
                }
            }
        }

        if new.terminal.cursor_blink_interval_ms != self.config.terminal.cursor_blink_interval_ms {
            let interval = std::time::Duration::from_millis(new.terminal.cursor_blink_interval_ms);
            self.cursor_blink.set_interval(interval);
            log::info!(
                "config reload: cursor blink interval={}ms",
                new.terminal.cursor_blink_interval_ms
            );
        }
    }

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
            let mode = super::init::decoration_to_mode(new.window.decorations);
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
                let old_mode = super::init::decoration_to_mode(self.config.window.decorations);
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
            let metrics = super::init::metrics_from_style(new.window.tab_bar_style);
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

    /// Detect and apply behavior config changes.
    ///
    /// Behavior flags are read from `self.config` at usage sites, so
    /// storing the new config is sufficient. If `bold_is_bright` changed,
    /// marks all panes dirty since existing cells may render differently.
    pub(in crate::app) fn apply_behavior_changes(&mut self, new: &Config) {
        if new.behavior.bold_is_bright != self.config.behavior.bold_is_bright {
            let enabled = new.behavior.bold_is_bright;
            if let Some(mux) = self.mux.as_mut() {
                for pane_id in mux.pane_ids() {
                    mux.set_bold_is_bright(pane_id, enabled);
                    mux.mark_all_dirty(pane_id);
                }
            }
            log::info!("config reload: bold_is_bright={enabled}");
        }
    }

    /// Detect and apply image protocol config changes.
    ///
    /// Updates CPU-side limits on the mux backend and GPU texture cache
    /// limits on each window renderer.
    pub(in crate::app) fn apply_image_changes(&mut self, new: &Config) {
        let changed = new.terminal.image_config() != self.config.terminal.image_config()
            || new.terminal.image_gpu_memory_limit != self.config.terminal.image_gpu_memory_limit;

        if !changed {
            return;
        }

        // CPU-side: propagate enable/disable + memory limits to all panes.
        if let Some(mux) = self.mux.as_mut() {
            for pane_id in mux.pane_ids() {
                mux.set_image_config(pane_id, new.terminal.image_config());
            }
        }

        // GPU-side: update texture cache limit on each window renderer.
        for ctx in self.windows.values_mut() {
            if let Some(renderer) = ctx.renderer.as_mut() {
                renderer.set_image_gpu_memory_limit(new.terminal.image_gpu_memory_limit);
            }
        }

        log::info!(
            "config reload: image protocol={}, mem={}MB, gpu={}MB, max_single={}MB, animation={}",
            new.terminal.image_protocol,
            new.terminal.image_memory_limit / 1_000_000,
            new.terminal.image_gpu_memory_limit / 1_000_000,
            new.terminal.image_max_single_size / 1_000_000,
            new.terminal.image_animation,
        );
    }

    /// Rebuild keybinding table from new config.
    pub(in crate::app) fn apply_keybinding_changes(&mut self, new: &Config) {
        self.bindings = keybindings::merge_bindings(&new.keybind);
    }

    /// Adjust font size by `delta` points (positive = larger, negative = smaller).
    ///
    /// Clamps to [4.0, 72.0] pt and triggers the full font reload pipeline.
    pub(in crate::app) fn zoom_font_size(&mut self, delta: f32) {
        let Some(new_size) = compute_zoomed_size(self.config.font.size, delta) else {
            return;
        };
        let mut new_config = self.config.clone();
        new_config.font.size = new_size;
        self.apply_font_changes(&new_config);
        self.config.font.size = new_size;
        for ctx in self.windows.values_mut() {
            ctx.root.mark_dirty();
        }
        log::info!("zoom: font size {new_size:.1}pt");
    }

    /// Reset font size to the user's configured value (from config file).
    ///
    /// Falls back to the built-in default (11.0 pt) if the config file
    /// cannot be loaded.
    pub(in crate::app) fn reset_font_size(&mut self) {
        let configured_size =
            Config::try_load().map_or_else(|_| FontConfig::default().size, |c| c.font.size);
        let Some(target) = compute_reset_size(self.config.font.size, configured_size) else {
            return;
        };
        let mut new_config = self.config.clone();
        new_config.font.size = target;
        self.apply_font_changes(&new_config);
        self.config.font.size = target;
        for ctx in self.windows.values_mut() {
            ctx.root.mark_dirty();
        }
        log::info!("zoom: reset to {configured_size:.1}pt");
    }
}
