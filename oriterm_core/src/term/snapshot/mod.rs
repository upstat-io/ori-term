//! Rendering snapshot extraction from terminal state.
//!
//! Extracted from `term/mod.rs` to keep the main file under the 500-line
//! limit. These methods build `RenderableContent` and manage damage state.
//! Image placement + pixel-data extraction lives in the `images` submodule.

mod images;

use std::collections::HashSet;

use vte::ansi::Color;

use crate::cell::Cell;
use crate::color::palette::Palette;
use crate::effect::sink::EffectSink;
use crate::grid::CursorShape;
use crate::image::{ImageId, KITTY_PLACEHOLDER};
use crate::index::Column;

use super::handler::image::kitty::placeholder::{IncompletePlacement, ResolvedPlaceholder};

use super::Term;
use super::mode::TermMode;
use super::renderable::{
    self, RenderableCell, RenderableContent, RenderableCursor, RenderablePlaceholderCell,
    TermDamage,
};

/// Per-row inputs for [`Term::fill_row_cells`].
#[derive(Clone, Copy)]
struct RowFill<'a> {
    /// The grid row whose cells are being snapshotted.
    row: &'a crate::grid::Row,
    /// Viewport line index this row maps to.
    vis_line: usize,
    /// Number of columns to walk.
    cols: usize,
    /// Palette used to resolve indexed colors.
    palette: &'a Palette,
}

/// Read-only inputs for [`Term::resolve_placeholder_cell`].
#[derive(Clone, Copy)]
struct PlaceholderInput<'a> {
    /// The cell being snapshotted (provides `ch` + `fg`).
    cell: &'a Cell,
    /// Viewport line index this cell maps to.
    vis_line: usize,
    /// Column index of the cell.
    col: Column,
    /// Combining-mark / zero-width chars carrying the placeholder diacritics.
    zerowidth: &'a [char],
    /// Raw underline color (kitty placement-id source), pre-palette resolution.
    underline_color_raw: Option<Color>,
}

impl<S: EffectSink> Term<S> {
    /// Extract a complete rendering snapshot.
    ///
    /// Convenience wrapper that allocates a fresh [`RenderableContent`] and
    /// fills it. For hot-path rendering, prefer [`renderable_content_into`]
    /// with a reused buffer to avoid per-frame allocation.
    ///
    /// This is a pure read — dirty state is **not** cleared. Callers must
    /// drain dirty state separately via `grid_mut().dirty_mut().drain()`
    /// after consuming the snapshot.
    ///
    /// [`renderable_content_into`]: Self::renderable_content_into
    pub fn renderable_content(&self) -> RenderableContent {
        let grid = self.grid();
        let mut out = RenderableContent {
            cells: Vec::with_capacity(grid.lines() * grid.cols()),
            cursor: RenderableCursor {
                line: 0,
                column: Column(0),
                shape: CursorShape::default(),
                visible: false,
            },
            display_offset: 0,
            stable_row_base: 0,
            mode: TermMode::empty(),
            all_dirty: false,
            damage: Vec::new(),
            images: Vec::new(),
            image_data: Vec::new(),
            images_dirty: false,
            placeholder_cells: Vec::new(),
            cols: 0,
            lines: 0,
            scrollback_len: 0,
            palette_snapshot: Vec::new(),
            search_active: false,
            search_query: String::new(),
            search_matches: Vec::new(),
            search_focused: None,
            search_total_matches: 0,
            mouse_cursor_icon: None,
            seen_image_ids: HashSet::new(),
        };
        self.renderable_content_into(&mut out);
        out
    }

    /// Fill an existing [`RenderableContent`] with the current terminal state.
    ///
    /// Clears `out` and refills it, reusing the underlying `Vec` allocations.
    /// The renderer should keep a single `RenderableContent` and pass it each
    /// frame to avoid the ~`lines * cols * 56` byte allocation that
    /// [`renderable_content`] performs.
    ///
    /// This is a pure read — dirty state is **not** cleared. Callers must
    /// drain dirty state separately via `grid_mut().dirty_mut().drain()`
    /// after consuming the snapshot.
    ///
    /// [`renderable_content`]: Self::renderable_content
    pub fn renderable_content_into(&self, out: &mut RenderableContent) {
        out.cells.clear();
        out.damage.clear();
        out.placeholder_cells.clear();

        let grid = self.grid();
        let raw_offset = grid.display_offset();
        debug_assert!(
            raw_offset <= grid.scrollback().len(),
            "display_offset ({raw_offset}) must be <= scrollback.len() ({})",
            grid.scrollback().len(),
        );
        let offset = raw_offset.min(grid.scrollback().len());
        let lines = grid.lines();
        let cols = grid.cols();
        let reverse_video = self.mode.contains(TermMode::REVERSE_VIDEO);

        // DECSCNM (mode 5): swap default fg/bg for cell resolution and palette
        // snapshot. Clone the palette only when reverse video is active (rare).
        let swapped;
        let palette = if reverse_video {
            swapped = {
                let mut p = self.palette.clone();
                p.swap_fg_bg();
                p
            };
            &swapped
        } else {
            &self.palette
        };

        for vis_line in 0..lines {
            // Top `offset` lines come from scrollback; the rest from the grid.
            let row = if vis_line < offset {
                let sb_idx = offset - 1 - vis_line;
                match grid.scrollback().get(sb_idx) {
                    Some(row) => row,
                    None => continue,
                }
            } else {
                let grid_line = vis_line - offset;
                &grid[crate::index::Line(grid_line as i32)]
            };

            self.fill_row_cells(
                out,
                RowFill {
                    row,
                    vis_line,
                    cols,
                    palette,
                },
            );
        }

        // Cursor is visible when SHOW_CURSOR is set and we're at the live view.
        let cursor_visible = self.mode.contains(TermMode::SHOW_CURSOR)
            && offset == 0
            && self.cursor_shape != CursorShape::Hidden;

        out.cursor = RenderableCursor {
            line: grid.cursor().line(),
            column: grid.cursor().col(),
            shape: self.cursor_shape,
            visible: cursor_visible,
        };

        out.all_dirty = renderable::collect_damage(grid, lines, &mut out.damage);
        out.display_offset = offset;
        let base_abs = grid.scrollback().len().saturating_sub(offset);
        out.stable_row_base = grid.total_evicted() as u64 + base_abs as u64;
        out.mode = self.mode;
        out.cols = cols;
        out.lines = lines;
        out.scrollback_len = grid.scrollback().len();
        out.mouse_cursor_icon = self.mouse_cursor_icon;

        Self::fill_palette_snapshot(palette, out);
        self.fill_image_snapshot(out);
        self.fill_placeholder_image_data(out);
    }

    /// Append the snapshot cells (and any kitty placeholder-cell entries)
    /// for one grid row. Continuation state for unicode placeholders is
    /// scoped to a single row, so callers re-seed `prev_placeholder` per
    /// invocation (this method owns that local state).
    fn fill_row_cells(&self, out: &mut RenderableContent, ctx: RowFill<'_>) {
        let RowFill {
            row,
            vis_line,
            cols,
            palette,
        } = ctx;
        let mut prev_placeholder: Option<ResolvedPlaceholder> = None;

        for col_idx in 0..cols {
            let col = Column(col_idx);
            let cell = &row[col];

            let fg = renderable::resolve_fg(cell.fg, cell.flags, palette, self.bold_is_bright);
            let bg = renderable::resolve_bg(cell.bg, palette);
            let renderable::ChannelColors {
                fg,
                bg,
                fg_alpha,
                bg_alpha,
            } = renderable::apply_inverse(
                renderable::ChannelColors {
                    fg,
                    bg,
                    fg_alpha: cell.fg_alpha(),
                    bg_alpha: cell.bg_alpha(),
                },
                cell.flags,
            );

            let (underline_color_raw, has_hyperlink, hyperlink_uri, zerowidth) =
                match cell.extra.as_ref() {
                    Some(e) => (
                        e.underline_color,
                        e.hyperlink.is_some(),
                        e.hyperlink.as_ref().map(|h| h.uri.clone()),
                        e.zerowidth.clone(),
                    ),
                    None => (None, false, None, Vec::new()),
                };

            let underline_color = underline_color_raw.map(|c| palette.resolve(c));

            // Double-exposure clamp: a placeholder image quad suppresses the
            // U+10EEEE glyph (a space is substituted) so the fallback glyph
            // does not render on top of the image, while bg / flags / selection
            // coverage stay intact.
            let suppress_glyph = self.resolve_placeholder_cell(
                PlaceholderInput {
                    cell,
                    vis_line,
                    col,
                    zerowidth: &zerowidth,
                    underline_color_raw,
                },
                &mut prev_placeholder,
                out,
            );

            out.cells.push(RenderableCell {
                line: vis_line,
                column: col,
                ch: if suppress_glyph { ' ' } else { cell.ch },
                fg,
                bg,
                flags: cell.flags,
                underline_color,
                fg_alpha,
                bg_alpha,
                underline_alpha: cell.underline_alpha(),
                has_hyperlink,
                hyperlink_uri,
                zerowidth,
            });
        }
    }

    /// Resolve a kitty unicode-placeholder cell (`U+10EEEE`) into an image-quad
    /// placement appended to `out.placeholder_cells`. Returns `true` when the
    /// cell's glyph must be suppressed (an image quad took its place).
    ///
    /// `prev_placeholder` carries row-scoped continuation state across cells so
    /// a multi-cell placeholder run resolves against its anchor.
    fn resolve_placeholder_cell(
        &self,
        input: PlaceholderInput<'_>,
        prev_placeholder: &mut Option<ResolvedPlaceholder>,
        out: &mut RenderableContent,
    ) -> bool {
        let PlaceholderInput {
            cell,
            vis_line,
            col,
            zerowidth,
            underline_color_raw,
        } = input;

        if cell.ch != KITTY_PLACEHOLDER {
            *prev_placeholder = None;
            return false;
        }

        let incomplete = IncompletePlacement::decode(zerowidth, cell.fg, underline_color_raw);
        let resolved = incomplete.resolve_with_continuation(prev_placeholder.as_ref());
        // Only emit when the resolved image_id is non-zero AND the image is
        // still cached. A bare U+10EEEE with no fg + no diacritic must render as
        // a glyph, not an image quad.
        let suppress_glyph = if resolved.image_id != 0
            && self
                .image_cache()
                .get_no_touch(ImageId::from_raw(resolved.image_id))
                .is_some()
        {
            let image_id = ImageId::from_raw(resolved.image_id);
            // (1, 1) is the implicit default — single-cell placement renders the
            // full image. A recorded multi-cell grid tells the GPU emit path to
            // slice the source.
            let (placement_cols, placement_rows) = self
                .image_cache()
                .placeholder_anchor_grid_for(image_id)
                .unwrap_or((1, 1));
            out.placeholder_cells.push(RenderablePlaceholderCell {
                line: vis_line,
                column: col,
                image_id,
                image_row: resolved.image_row,
                image_col: resolved.image_col,
                placement_id: resolved.placement_id,
                placement_cols,
                placement_rows,
            });
            true
        } else {
            false
        };
        *prev_placeholder = Some(resolved);
        suppress_glyph
    }

    /// Write 270 pre-resolved RGB entries from the palette into the snapshot.
    fn fill_palette_snapshot(palette: &Palette, out: &mut RenderableContent) {
        out.palette_snapshot.clear();
        out.palette_snapshot
            .reserve(270usize.saturating_sub(out.palette_snapshot.capacity()));
        for i in 0..270 {
            let rgb = palette.color(i);
            out.palette_snapshot.push([rgb.r, rgb.g, rgb.b]);
        }
    }

    /// Drain damage from the active grid.
    ///
    /// Returns a [`TermDamage`] iterator that yields dirty lines and clears
    /// marks as it goes. Check [`TermDamage::is_all_dirty`] first — when true,
    /// repaint everything and drop the iterator (which clears remaining marks).
    /// Also clears the image cache dirty flag.
    pub fn damage(&mut self) -> TermDamage<'_> {
        self.image_cache_mut().take_dirty();
        let grid = self.grid_mut();
        let all_dirty = grid.dirty().is_all_dirty();
        TermDamage::new(grid.dirty_mut().drain(), all_dirty)
    }

    /// Clear all damage marks without reading them.
    ///
    /// Called when the renderer wants to discard pending damage (e.g. after
    /// a full repaint that doesn't need per-line tracking). Also clears the
    /// image cache dirty flag.
    pub fn reset_damage(&mut self) {
        self.grid_mut().dirty_mut().drain().for_each(drop);
        self.image_cache_mut().take_dirty();
    }
}

#[cfg(test)]
mod tests;
