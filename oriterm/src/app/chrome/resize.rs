//! Window resize handling.
//!
//! Extracted from `chrome/mod.rs` to keep file sizes under the 500-line limit.

use winit::window::WindowId;

use super::compute_window_layout;
use crate::app::App;

impl App {
    /// Update window resize increments from current cell metrics.
    ///
    /// Called after any change that affects cell dimensions (font size,
    /// DPI, font family) so the window snaps to cell boundaries.
    pub(in crate::app) fn update_resize_increments(&self, winit_id: WindowId) {
        let Some(ctx) = self.windows.get(&winit_id) else {
            return;
        };
        let Some(renderer) = ctx.renderer.as_ref() else {
            return;
        };
        let cell = renderer.cell_metrics();

        // Push cell size and grid padding to the Win32 subclass for
        // WM_SIZING snap-to-grid. On frameless CSD windows, winit's
        // set_resize_increments is ignored by the OS, so WM_SIZING is the
        // only way to snap resize boundaries. Cell metrics are already in
        // physical pixels (font loaded at DPI). The padding ensures the
        // snapped width accounts for the grid origin offset.
        #[cfg(target_os = "windows")]
        {
            let scale = ctx.window.scale_factor().factor() as f32;
            let pad = (super::GRID_PADDING * scale).round();
            oriterm_ui::platform_windows::set_cell_size(
                ctx.window.window(),
                cell.width,
                cell.height,
                pad,
            );
        }

        if !self.config.window.resize_increments {
            return;
        }
        let inc =
            winit::dpi::PhysicalSize::new(cell.width.round() as u32, cell.height.round() as u32);
        ctx.window.window().set_resize_increments(Some(inc));
    }

    /// Recompute grid layout from current cell metrics and viewport size.
    ///
    /// Reads cell metrics from the renderer, chrome height (caption + tab bar)
    /// from widgets, and updates the terminal grid widget, tab grid, PTY
    /// dimensions, and resize increments. Called after any change to font,
    /// DPI, or window size.
    ///
    /// `winit_id` identifies which window to recompute. Widget updates and
    /// cache invalidation target only this window.
    pub(in crate::app) fn sync_grid_layout(
        &mut self,
        winit_id: WindowId,
        viewport_w: u32,
        viewport_h: u32,
    ) {
        let Some(ctx) = self.windows.get(&winit_id) else {
            return;
        };
        let Some(renderer) = ctx.renderer.as_ref() else {
            return;
        };
        let cell = renderer.cell_metrics();
        let scale = ctx.window.scale_factor().factor() as f32;
        let hidden = self.config.window.tab_bar_position == crate::config::TabBarPosition::Hidden;
        let tb_h = ctx.tab_bar.metrics().height;
        let sb_h = if self.config.window.show_status_bar {
            oriterm_ui::widgets::status_bar::STATUS_BAR_HEIGHT
        } else {
            0.0
        };
        let border_inset = if ctx.window.is_maximized() || ctx.window.is_fullscreen() {
            0.0
        } else {
            2.0
        };
        let wl = compute_window_layout(
            viewport_w,
            viewport_h,
            &cell,
            scale,
            hidden,
            tb_h,
            sb_h,
            border_inset,
        );

        // Reborrow mutably now that immutable reads are done.
        let ctx = self.windows.get_mut(&winit_id).expect("checked above");
        ctx.terminal_grid.set_cell_metrics(cell.width, cell.height);
        ctx.terminal_grid.set_grid_size(wl.cols, wl.rows);
        ctx.terminal_grid.set_bounds(wl.grid_rect);
        ctx.tab_bar_phys_rect = wl.tab_bar_rect;
        ctx.status_bar_phys_rect = wl.status_bar_rect;
        let (cols, rows) = (wl.cols, wl.rows);

        // Resize the active pane in this specific window (not the globally
        // focused one). Multi-pane layouts are recomputed by resize_all_panes.
        if let Some(pane_id) = self.active_pane_id_for_window(winit_id) {
            if let Some(mux) = self.mux.as_mut() {
                mux.resize_pane_grid(pane_id, rows as u16, cols as u16);
            }
        }
        self.resize_all_panes();
        if let Some(ctx) = self.windows.get_mut(&winit_id) {
            ctx.cached_dividers = None;
        }

        self.update_resize_increments(winit_id);
    }

    /// Handle window resize: reconfigure surface, update chrome layout,
    /// resize grid and PTY.
    ///
    /// `winit_id` identifies which window was resized. All operations
    /// (surface reconfigure, widget layout, grid recomputation) target
    /// only this window.
    ///
    /// Bails when the window is minimized. On Windows, the minimize
    /// animation fires `Resized` with a small non-zero size (e.g. 199×34)
    /// that produces a degenerate grid (15×1). Without this guard the PTY
    /// receives that tiny resize, the shell reflows/clears, and content is
    /// lost. On restore a fresh `Resized` fires with the real dimensions.
    pub(in crate::app) fn handle_resize(
        &mut self,
        winit_id: WindowId,
        size: winit::dpi::PhysicalSize<u32>,
    ) {
        // macOS: process fullscreen events eagerly during resize. macOS fires
        // resize events during fullscreen transitions BEFORE capturing the "to"
        // state for the animation. Processing here ensures the tab bar inset
        // is correct in the animation snapshot.
        #[cfg(target_os = "macos")]
        self.process_fullscreen_events();

        // On Windows, detect DPI changes from WM_DPICHANGED. The snap
        // subclass proc consumes the message before winit sees it, so
        // ScaleFactorChanged never fires — the resize handler is the
        // only reliable place to detect the change.
        #[cfg(target_os = "windows")]
        {
            let dpi_changed = self.windows.get_mut(&winit_id).and_then(|ctx| {
                let new_scale = oriterm_ui::platform_windows::get_current_dpi(ctx.window.window())?;
                ctx.window
                    .update_scale_factor(new_scale)
                    .then_some(new_scale)
            });
            if let Some(new_scale) = dpi_changed {
                self.handle_dpi_change(winit_id, new_scale);
                // Update chrome metrics for the new physical DPI.
                if let Some(ctx) = self.windows.get(&winit_id) {
                    let s = new_scale as f32;
                    let tab_bar_h = if self.config.window.tab_bar_position
                        == crate::config::TabBarPosition::Hidden
                    {
                        0.0
                    } else {
                        ctx.tab_bar.metrics().height
                    };
                    super::refresh_chrome(
                        ctx.window.window(),
                        &ctx.tab_bar.interactive_rects(),
                        tab_bar_h,
                        s,
                        true,
                    );
                }
            }
        }

        // Skip resize while minimized. On Windows the minimize animation
        // fires Resized with a small non-zero size (e.g. 199×34) that
        // computes a degenerate grid (15×1). Sending that to the PTY makes
        // the shell reflow/clear, destroying content. The restore event
        // delivers the real dimensions.
        let minimized = self
            .windows
            .get(&winit_id)
            .and_then(|ctx| ctx.window.window().is_minimized())
            .unwrap_or(false);
        if minimized {
            return;
        }

        // Window size changed — cached tab width is invalid.
        self.release_tab_width_lock();

        // Resize GPU surface (scoped to release borrows before sync_grid_layout).
        {
            let Some(gpu) = &self.gpu else { return };
            let Some(ctx) = self.windows.get_mut(&winit_id) else {
                return;
            };
            ctx.window.resize_surface(size.width, size.height, gpu);
        }

        // Update chrome and tab bar layout for new window width.
        if let Some(ctx) = self.windows.get_mut(&winit_id) {
            let scale = ctx.window.scale_factor().factor() as f32;
            let logical_w = size.width as f32 / scale;
            ctx.tab_bar.set_window_width(logical_w);
            ctx.status_bar.set_window_width(logical_w);

            // macOS tab bar inset (traffic light space) is managed by
            // fullscreen transition notifications in macos.rs, not here.
            // The willEnter/willExit observers update the inset before the
            // macOS animation starts, avoiding the visual glitch of traffic
            // lights overlapping tab text during the transition.
        }

        // Update overlay manager viewport for dialog placement.
        if let Some(ctx) = self.windows.get_mut(&winit_id) {
            let scale = ctx.window.scale_factor().factor() as f32;
            let logical_w = size.width as f32 / scale;
            let logical_h = size.height as f32 / scale;
            ctx.root
                .overlays_mut()
                .set_viewport(oriterm_ui::geometry::Rect::new(
                    0.0, 0.0, logical_w, logical_h,
                ));
        }

        // Recompute grid dimensions, resize terminal + PTY + increments.
        self.sync_grid_layout(winit_id, size.width, size.height);

        self.refresh_platform_rects(winit_id);

        if let Some(ctx) = self.windows.get_mut(&winit_id) {
            ctx.url_cache.invalidate();
            ctx.hovered_url = None; // Segments contain stale absolute rows.
            ctx.root.invalidation_mut().invalidate_all();
            ctx.root.damage_mut().reset();
            ctx.root.mark_dirty();
        }
    }

    /// Refresh platform hit test rects from the tab bar's interactive rects.
    ///
    /// Must be called after any tab mutation (add, remove, reorder, tear-off)
    /// so the OS hit test layer matches the current tab bar layout.
    /// Routes through [`NativeChromeOps`] — no-op on non-Windows platforms.
    pub(in crate::app) fn refresh_platform_rects(&self, winit_id: WindowId) {
        let Some(ctx) = self.windows.get(&winit_id) else {
            return;
        };
        let scale = ctx.window.scale_factor().factor() as f32;
        let tab_bar_h =
            if self.config.window.tab_bar_position == crate::config::TabBarPosition::Hidden {
                0.0
            } else {
                ctx.tab_bar.metrics().height
            };
        super::refresh_chrome(
            ctx.window.window(),
            &ctx.tab_bar.interactive_rects(),
            tab_bar_h,
            scale,
            true,
        );
    }
}
