//! Per-frame render dispatch: iterate dirty windows and dialogs.
//!
//! Extracted from `event_loop.rs` to keep that file under the 500-line limit.
//! Called once per frame from `about_to_wait` when at least one window is dirty
//! and the frame budget has elapsed.
//!
//! This is the canonical SSOT dispatch point — the render gate (5.16.2)
//! consults [`gate_outcome`] *before* walking `windows` + `dialogs`. Both
//! single-pane and multi-pane render paths flow through this function, so the
//! gate covers every entry point with a single guard (no duplicate guards
//! inside multi-pane helpers — algorithmic-DRY per
//! ).

use super::App;
use super::perf_stats::FramePhases;
use crate::gpu::recovery::{RenderOutcome, gate_outcome};

impl App {
    /// Render all dirty terminal and dialog windows.
    ///
    /// Temporarily swaps `focused_window_id`/`active_window` to target each
    /// dirty window, then restores the original focus. Returns a
    /// [`RenderOutcome`] so the gate decision is observable in tests and the
    /// caller (the event loop's `about_to_wait`) can record perf counters
    /// for gated frames.
    pub(super) fn render_dirty_windows(&mut self) -> RenderOutcome {
        // I1 — render gate. Consult `gpu_health` first; when `Recovering`
        // or `Unavailable`, return early WITHOUT clearing dirty flags and
        // WITHOUT invoking any `WindowRenderer` method. Windows stay dirty
        // so the next successful frame after recovery is a full repaint.
        if let Some(gated) = gate_outcome(&self.gpu_health) {
            log::trace!(
                "render gate: {gated:?} (gpu_health.epoch={:?})",
                self.gpu_health.epoch(),
            );
            return gated;
        }

        // No dirty windows — nothing to render. Distinct from `Submitted`
        // so the caller can record "frame ran but produced no GPU work"
        // separately from "frame ran and submitted commands".
        if !self.is_any_window_dirty() {
            return RenderOutcome::Skipped;
        }

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

        RenderOutcome::Submitted
    }

    /// Returns `true` if any terminal or dialog window needs rendering.
    pub(super) fn is_any_window_dirty(&self) -> bool {
        self.windows.values().any(|c| c.root.is_dirty())
            || self.dialogs.values().any(|c| c.root.is_dirty())
    }

    /// Returns `true` if any dirty window has requested an urgent redraw.
    ///
    /// Urgent redraws bypass the frame budget gate (e.g., user-initiated
    /// actions that should be visible immediately).
    pub(super) fn is_any_urgent_redraw(&self) -> bool {
        self.windows
            .values()
            .any(|c| c.root.is_dirty() && c.root.is_urgent_redraw())
            || self
                .dialogs
                .values()
                .any(|c| c.root.is_dirty() && c.root.is_urgent_redraw())
    }

    /// Returns `true` if any window has active compositor animations.
    pub(super) fn has_active_animations(&self) -> bool {
        self.windows
            .values()
            .any(|c| c.root.layer_animator().is_any_animating())
            || self
                .dialogs
                .values()
                .any(|c| c.root.layer_animator().is_any_animating())
    }
}
