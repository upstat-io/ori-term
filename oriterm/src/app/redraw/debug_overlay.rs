//! Debug performance overlay rendering.
//!
//! Shows FPS, dirty row count, total instance count, and atlas utilization
//! as a floating badge in the bottom-left corner. Toggled via
//! `Ctrl+Shift+F12` (action: `ToggleDebugOverlay`).

use std::fmt::Write as _;

use oriterm_ui::draw::Scene;
use oriterm_ui::geometry::Point;
use oriterm_ui::widgets::status_badge::StatusBadge;

use super::App;
use crate::font::{CachedTextMeasurer, TextShapeCache};
use crate::gpu::WindowRenderer;
use crate::gpu::state::GpuState;

/// Collected stats for the current frame.
pub(in crate::app) struct DebugStats {
    /// Smoothed frames per second.
    pub fps: f32,
    /// Number of dirty rows this frame.
    pub dirty_rows: usize,
    /// Total visible rows.
    pub total_rows: usize,
    /// Total GPU instances across all buffers.
    pub instances: usize,
    /// Number of draw calls.
    pub draw_calls: u32,
    /// Mono atlas: (glyphs cached, pages active).
    pub mono_atlas: (usize, usize),
    /// Subpixel atlas: (glyphs cached, pages active).
    pub subpixel_atlas: (usize, usize),
    /// Color atlas: (glyphs cached, pages active).
    pub color_atlas: (usize, usize),
}

impl App {
    /// Draw the debug performance overlay in the bottom-left corner.
    #[expect(
        clippy::too_many_arguments,
        reason = "debug overlay drawing: stats, renderer, scene, buffer, viewport, scale, GPU, cache"
    )]
    pub(in crate::app::redraw) fn draw_debug_overlay(
        stats: &DebugStats,
        renderer: &mut WindowRenderer,
        scene: &mut Scene,
        buf: &mut String,
        logical_w: f32,
        logical_h: f32,
        scale: f32,
        gpu: &GpuState,
        text_cache: &TextShapeCache,
    ) {
        buf.clear();
        let dirty_pct = if stats.total_rows > 0 {
            (stats.dirty_rows as f32 / stats.total_rows as f32) * 100.0
        } else {
            0.0
        };

        let _ = write!(
            buf,
            "FPS: {:.0}  Dirty: {}/{} ({:.0}%)  Inst: {}  Draws: {}  \
             Atlas: M:{}/{}p S:{}/{}p C:{}/{}p",
            stats.fps,
            stats.dirty_rows,
            stats.total_rows,
            dirty_pct,
            stats.instances,
            stats.draw_calls,
            stats.mono_atlas.0,
            stats.mono_atlas.1,
            stats.subpixel_atlas.0,
            stats.subpixel_atlas.1,
            stats.color_atlas.0,
            stats.color_atlas.1,
        );

        let badge = StatusBadge::new(buf);

        let max_text_w = logical_w * 0.8;
        let measurer = CachedTextMeasurer::new(renderer.ui_measurer(scale), text_cache, scale);
        let (_w, h) = badge.measure(&measurer, max_text_w);

        // Position: bottom-left, inset from edges.
        let margin = 8.0;
        let pos = Point::new(margin, logical_h - h - margin);

        scene.clear();
        let _ = badge.draw(scene, &measurer, pos, max_text_w);

        renderer.append_ui_scene_with_text(scene, scale, 1.0, gpu);
    }
}
