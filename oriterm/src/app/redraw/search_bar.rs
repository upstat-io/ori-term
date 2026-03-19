//! Search bar overlay rendering.

use std::fmt::Write as _;

use oriterm_ui::draw::Scene;
use oriterm_ui::geometry::Point;
use oriterm_ui::widgets::status_badge::StatusBadge;

use super::App;
use crate::font::{CachedTextMeasurer, TextShapeCache, UiFontMeasurer};
use crate::gpu::FrameSearch;
use crate::gpu::state::GpuState;

impl App {
    /// Draw the search bar overlay above the grid area.
    ///
    /// Shows the current query and match count ("N of M") as a floating
    /// [`StatusBadge`]. Coordinates are in logical pixels; `scale` converts
    /// to physical pixels for the GPU pipeline.
    #[expect(
        clippy::too_many_arguments,
        reason = "search bar drawing: search state, renderer, scene, buffer, viewport, caption, scale, GPU, cache"
    )]
    pub(in crate::app::redraw) fn draw_search_bar(
        search: &FrameSearch,
        renderer: &mut crate::gpu::WindowRenderer,
        scene: &mut Scene,
        buf: &mut String,
        logical_width: f32,
        caption_h: f32,
        scale: f32,
        gpu: &GpuState,
        text_cache: &TextShapeCache,
    ) {
        buf.clear();
        let query = search.query();
        if query.is_empty() {
            buf.push_str("Search: ");
        } else if search.match_count() == 0 {
            let _ = write!(buf, "Search: {query}  No matches");
        } else {
            let _ = write!(
                buf,
                "Search: {query}  {} of {}",
                search.focused_display(),
                search.match_count()
            );
        }

        let badge = StatusBadge::new(buf);

        // Shape text and measure badge (immutable borrow on renderer ends
        // after shape — NLL lets the mutable append follow).
        let max_text_w = logical_width * 0.4;
        let measurer = CachedTextMeasurer::new(
            UiFontMeasurer::new(renderer.active_ui_collection(), scale),
            text_cache,
            scale,
        );
        let (w, _h) = badge.measure(&measurer, max_text_w);

        // Position: top-right of grid area, inset from edges.
        let margin = 8.0;
        let pos = Point::new(logical_width - w - margin, caption_h + margin);

        scene.clear();
        let _ = badge.draw(scene, &measurer, pos, max_text_w);

        renderer.append_ui_scene_with_text(scene, scale, 1.0, gpu);
    }
}
