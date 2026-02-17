//! Prepare phase: convert a [`FrameInput`] into GPU-ready instance buffers.
//!
//! [`prepare_frame`] is a pure CPU function — no wgpu types, no device, no
//! queue. Given a terminal snapshot and an atlas lookup, it produces a
//! [`PreparedFrame`] containing three [`InstanceWriter`] buffers (backgrounds,
//! glyphs, cursors) ready for GPU upload.
//!
//! The [`AtlasLookup`] trait abstracts glyph lookup for testability: production
//! wraps `FontCollection::resolve` + `GlyphAtlas::lookup`; tests use a simple
//! `HashMap`.

use oriterm_core::{CellFlags, CursorShape, Rgb};

use super::atlas::AtlasEntry;
use super::frame_input::FrameInput;
use super::prepared_frame::PreparedFrame;
use crate::font::GlyphStyle;

/// Abstracts glyph atlas lookup for testability.
///
/// Production (Section 5.10): wraps `FontCollection::resolve` +
/// `GlyphAtlas::lookup`. Tests: wraps a `HashMap<(char, GlyphStyle), AtlasEntry>`.
pub trait AtlasLookup {
    /// Look up a cached glyph entry by character and style.
    fn lookup(&self, ch: char, style: GlyphStyle) -> Option<&AtlasEntry>;
}

/// Convert cell flags to the corresponding glyph style.
pub(crate) fn glyph_style(flags: CellFlags) -> GlyphStyle {
    let bold = flags.contains(CellFlags::BOLD);
    let italic = flags.contains(CellFlags::ITALIC);
    match (bold, italic) {
        (true, true) => GlyphStyle::BoldItalic,
        (true, false) => GlyphStyle::Bold,
        (false, true) => GlyphStyle::Italic,
        (false, false) => GlyphStyle::Regular,
    }
}

/// Convert a [`FrameInput`] into a GPU-ready [`PreparedFrame`].
///
/// Allocates a new `PreparedFrame` with capacity for the grid dimensions,
/// then delegates to [`prepare_frame_into`]. For steady-state rendering,
/// prefer `prepare_frame_into` to reuse allocations across frames.
pub fn prepare_frame(input: &FrameInput, atlas: &dyn AtlasLookup) -> PreparedFrame {
    let cols = input.columns();
    let rows = input.rows();
    let opacity = f64::from(input.palette.opacity);
    let mut frame =
        PreparedFrame::with_capacity(input.viewport, cols, rows, input.palette.background, opacity);
    fill_frame(input, atlas, &mut frame);
    frame
}

/// Convert a [`FrameInput`] into a pre-existing [`PreparedFrame`], reusing
/// its buffer allocations.
///
/// Clears all instance buffers (retaining capacity), updates the clear
/// color, then fills backgrounds, glyphs, and cursors. This avoids the
/// ~307KB per-frame allocation that [`prepare_frame`] incurs for an 80x24
/// terminal.
pub fn prepare_frame_into(
    input: &FrameInput,
    atlas: &dyn AtlasLookup,
    out: &mut PreparedFrame,
) {
    out.clear();
    out.viewport = input.viewport;
    out.set_clear_color(input.palette.background, f64::from(input.palette.opacity));
    fill_frame(input, atlas, out);
}

/// Shared implementation: emit instances into `frame`.
///
/// Iterates every visible cell, emits background and glyph instances, then
/// builds cursor instances. The result is fully deterministic: same input +
/// same atlas = bitwise identical output.
fn fill_frame(input: &FrameInput, atlas: &dyn AtlasLookup, frame: &mut PreparedFrame) {
    let cw = input.cell_size.width;
    let ch = input.cell_size.height;
    let baseline = input.cell_size.baseline;

    for cell in &input.content.cells {
        // Wide char spacers are handled by the primary wide char cell.
        if cell.flags.contains(CellFlags::WIDE_CHAR_SPACER) {
            continue;
        }

        let col = cell.column.0;
        let row = cell.line;
        let x = col as f32 * cw;
        let y = row as f32 * ch;

        // Background: wide chars span 2 cell widths.
        let bg_w = if cell.flags.contains(CellFlags::WIDE_CHAR) {
            2.0 * cw
        } else {
            cw
        };
        frame.backgrounds.push_rect(x, y, bg_w, ch, cell.bg, 1.0);

        // Foreground glyph (skip spaces).
        if cell.ch != ' ' {
            let style = glyph_style(cell.flags);
            if let Some(entry) = atlas.lookup(cell.ch, style) {
                let glyph_x = x + entry.bearing_x as f32;
                let glyph_y = y + baseline - entry.bearing_y as f32;
                let uv = [entry.uv_x, entry.uv_y, entry.uv_w, entry.uv_h];
                frame.glyphs.push_glyph(
                    glyph_x,
                    glyph_y,
                    entry.width as f32,
                    entry.height as f32,
                    uv,
                    cell.fg,
                    1.0,
                );
            }
        }
    }

    // Cursor instances.
    let cursor = &input.content.cursor;
    if cursor.visible {
        build_cursor(
            frame,
            cursor.shape,
            cursor.column.0,
            cursor.line,
            cw,
            ch,
            input.palette.cursor_color,
        );
    }
}

/// Emit cursor instances into the prepared frame.
///
/// The cursor shape determines the geometry:
/// - `Block` — full cell rectangle.
/// - `Bar` — 2px-wide vertical line at the left edge.
/// - `Underline` — 2px-tall horizontal line at the bottom.
/// - `HollowBlock` — 4 thin outline rectangles (top, bottom, left, right).
/// - `Hidden` — no instances.
fn build_cursor(
    frame: &mut PreparedFrame,
    shape: CursorShape,
    col: usize,
    row: usize,
    cw: f32,
    ch: f32,
    color: Rgb,
) {
    let x = col as f32 * cw;
    let y = row as f32 * ch;
    let thickness = 2.0_f32;

    match shape {
        CursorShape::Block => {
            frame.cursors.push_cursor(x, y, cw, ch, color, 1.0);
        }
        CursorShape::Bar => {
            frame
                .cursors
                .push_cursor(x, y, thickness, ch, color, 1.0);
        }
        CursorShape::Underline => {
            frame
                .cursors
                .push_cursor(x, y + ch - thickness, cw, thickness, color, 1.0);
        }
        CursorShape::HollowBlock => {
            // Top edge.
            frame
                .cursors
                .push_cursor(x, y, cw, thickness, color, 1.0);
            // Bottom edge.
            frame
                .cursors
                .push_cursor(x, y + ch - thickness, cw, thickness, color, 1.0);
            // Left edge.
            frame
                .cursors
                .push_cursor(x, y, thickness, ch, color, 1.0);
            // Right edge.
            frame
                .cursors
                .push_cursor(x + cw - thickness, y, thickness, ch, color, 1.0);
        }
        CursorShape::Hidden => {}
    }
}

#[cfg(test)]
mod tests;
