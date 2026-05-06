//! Frame shaping — converts terminal grid cells into glyph runs.
//!
//! Extracted from `helpers/mod.rs` to keep the file under 500 lines
//! (BUG-06-015).

use crate::gpu::frame_input::FrameInput;
use super::ShapingScratch;
use crate::font::{
    FontCollection, build_col_glyph_map, prepare_line, shape_prepared_runs, size_key,
};

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
