//! Per-frame render dispatch: iterate dirty windows and dialogs.
//!
//! Extracted from `event_loop.rs` to keep that file under the 500-line limit.
//! Called once per frame from `about_to_wait` when at least one window is dirty
//! and the frame budget has elapsed.

use super::App;
use super::perf_stats::FramePhases;

impl App {
    /// Render all dirty terminal and dialog windows.
    ///
    /// Temporarily swaps `focused_window_id`/`active_window` to target each
    /// dirty window, then restores the original focus.
    pub(super) fn render_dirty_windows(&mut self) {
        let frame_start = std::time::Instant::now();
        let mut phases = FramePhases::default();

        self.scratch_dirty_windows.clear();
        self.scratch_dirty_windows.extend(
            self.windows
                .iter()
                .filter(|(_, ctx)| ctx.root.is_dirty())
                .map(|(&id, _)| id),
        );

        let saved_focused = self.focused_window_id;
        let saved_active = self.active_window;

        for i in 0..self.scratch_dirty_windows.len() {
            let wid = self.scratch_dirty_windows[i];
            if let Some(ctx) = self.windows.get_mut(&wid) {
                ctx.root.clear_dirty();
            }
            let mux_wid = self
                .windows
                .get(&wid)
                .map(|ctx| ctx.window.session_window_id());
            self.focused_window_id = Some(wid);
            self.active_window = mux_wid;
            let win_phases = self.handle_redraw();
            phases.accumulate(&win_phases);
            // Clear invalidation after render so build_scene sees dirty widgets.
            if let Some(ctx) = self.windows.get_mut(&wid) {
                ctx.root.invalidation_mut().clear();
            }
        }

        self.focused_window_id = saved_focused;
        self.active_window = saved_active;

        // Render dirty dialog windows (reuse the same scratch buffer).
        // NOTE: inner loop parallels the windows loop above — both follow
        // collect-dirty → clear-dirty → dispatch → clear-invalidation.
        // Mirror structural changes across both loops.
        self.scratch_dirty_windows.clear();
        self.scratch_dirty_windows.extend(
            self.dialogs
                .iter()
                .filter(|(_, ctx)| ctx.root.is_dirty())
                .map(|(&id, _)| id),
        );
        for i in 0..self.scratch_dirty_windows.len() {
            let wid = self.scratch_dirty_windows[i];
            if let Some(ctx) = self.dialogs.get_mut(&wid) {
                ctx.root.clear_dirty();
            }
            self.render_dialog(wid);
            // Clear invalidation after render so build_scene sees dirty widgets.
            if let Some(ctx) = self.dialogs.get_mut(&wid) {
                ctx.root.invalidation_mut().clear();
            }
        }

        self.last_render = std::time::Instant::now();
        self.perf.record_render(frame_start.elapsed(), &phases);

        // Post-render: shrink grow-only buffers if capacity vastly exceeds usage.
        for ctx in self.windows.values_mut() {
            if let Some(renderer) = ctx.renderer.as_mut() {
                renderer.maybe_shrink_buffers();
            }
            ctx.chrome_scene.maybe_shrink();
            ctx.root.damage_mut().maybe_shrink();
        }
        for ctx in self.dialogs.values_mut() {
            if let Some(renderer) = ctx.renderer.as_mut() {
                renderer.maybe_shrink_buffers();
            }
            ctx.scene.maybe_shrink();
            ctx.root.damage_mut().maybe_shrink();
        }
        if let Some(mux) = self.mux.as_mut() {
            mux.maybe_shrink_renderable_caches();
        }
    }

    /// Returns `true` if any terminal or dialog window needs rendering.
    pub(super) fn is_any_window_dirty(&self) -> bool {
        self.windows.values().any(|c| c.root.is_dirty())
            || self.dialogs.values().any(|c| c.root.is_dirty())
    }
}
