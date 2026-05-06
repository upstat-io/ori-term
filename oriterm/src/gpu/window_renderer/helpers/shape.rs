//! Frame shaping — converts terminal grid cells into glyph runs.
//!
//! Extracted from `helpers/mod.rs` to keep the file under 500 lines
//! (BUG-06-015).

use crate::gpu::frame_input::FrameInput;
use crate::gpu::prepare::ShapedFrame;
use crate::gpu::maybe_shrink_vec;
use crate::font::{
    FontCollection, build_col_glyph_map, prepare_line, shape_prepared_runs, size_key,
};

/// Per-frame scratch buffers reused across frames to avoid per-frame allocation.
pub(crate) struct ShapingScratch {
    /// The shaped frame populated during Prepare.
    pub(crate)frame: ShapedFrame,
    pub(crate)runs: Vec<crate::font::ShapingRun>,
    pub(crate)glyphs: Vec<oriterm_ui::text::ShapedGlyph>,
    pub(crate)col_starts: Vec<usize>,
    /// Column-to-glyph map for the current row.
    pub(crate)col_map: Vec<Option<usize>>,
    /// Rustybuzz buffer reused across frames to avoid per-frame allocation.
    pub(crate)unicode_buffer: Option<rustybuzz::UnicodeBuffer>,
    /// Rustybuzz Face objects reused across frames.
    ///
    /// Stored with `'static` lifetime because `ShapingScratch` has no lifetime
    /// parameter. Filled via [`FontCollection::fill_shaping_faces`] which
    /// transmutes the actual `'a` borrow to `'static`. This is sound because
    /// the Vec is cleared before every fill and only accessed while
    /// `FontCollection` is borrowed (within `shape_frame`).
    pub(crate)faces_buf: Vec<Option<rustybuzz::Face<'static>>>,
}

impl ShapingScratch {
    pub(crate)fn new() -> Self {
        Self {
            frame: ShapedFrame::new(0, 0),
            runs: Vec::new(),
            glyphs: Vec::new(),
            col_starts: Vec::new(),
            col_map: Vec::new(),
            unicode_buffer: None,
            faces_buf: Vec::new(),
        }
    }

    /// Shrink per-row scratch buffers if capacity vastly exceeds usage.
    pub(crate)fn maybe_shrink(&mut self) {
        self.frame.maybe_shrink();
        maybe_shrink_vec(&mut self.runs);
        maybe_shrink_vec(&mut self.glyphs);
        maybe_shrink_vec(&mut self.col_starts);
        maybe_shrink_vec(&mut self.col_map);
        maybe_shrink_vec(&mut self.faces_buf);
    }
}

pub(crate) fn shape_frame(
    input: &FrameInput,
    fonts: &FontCollection,
    scratch: &mut ShapingScratch,
) {
    let cols = input.columns();
    let size_q6 = size_key(fonts.size_px());
    let hinted = fonts.hinting_mode().hint_flag();
    scratch.frame.clear(cols, size_q6, hinted);
    if cols == 0 {
        return;
    }
    let rows = input.rows().min(input.content.cells.len() / cols);
    fonts.fill_shaping_faces(&mut scratch.faces_buf);

    for row_idx in 0..rows {
        let start = row_idx * cols;
        let end = start + cols;
        let row_cells = &input.content.cells[start..end];

        prepare_line(row_cells, cols, fonts, &mut scratch.runs);
        shape_prepared_runs(
            &scratch.runs,
            &scratch.faces_buf,
            fonts,
            &mut scratch.glyphs,
            &mut scratch.col_starts,
            &mut scratch.unicode_buffer,
        );
        build_col_glyph_map(&scratch.col_starts, cols, &mut scratch.col_map);
        scratch
            .frame
            .push_row(&scratch.glyphs, &scratch.col_starts, &scratch.col_map);
    }
}
