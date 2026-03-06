//! Rendering snapshot extraction from terminal state.
//!
//! Extracted from `term/mod.rs` to keep the main file under the 500-line
//! limit. These methods build `RenderableContent` and manage damage state.

use crate::event::EventListener;
use crate::grid::CursorShape;
use crate::index::Column;

use super::Term;
use super::mode::TermMode;
use super::renderable::{self, RenderableCell, RenderableContent, RenderableCursor, TermDamage};

impl<T: EventListener> Term<T> {
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
        let palette = &self.palette;

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

            for col_idx in 0..cols {
                let col = Column(col_idx);
                let cell = &row[col];

                let fg = renderable::resolve_fg(cell.fg, cell.flags, palette);
                let bg = renderable::resolve_bg(cell.bg, palette);
                let (fg, bg) = renderable::apply_inverse(fg, bg, cell.flags);

                let (underline_color, has_hyperlink, zerowidth) = match cell.extra.as_ref() {
                    Some(e) => (
                        e.underline_color.map(|c| palette.resolve(c)),
                        e.hyperlink.is_some(),
                        e.zerowidth.clone(),
                    ),
                    None => (None, false, Vec::new()),
                };

                out.cells.push(RenderableCell {
                    line: vis_line,
                    column: col,
                    ch: cell.ch,
                    fg,
                    bg,
                    flags: cell.flags,
                    underline_color,
                    has_hyperlink,
                    zerowidth,
                });
            }
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

        out.all_dirty = renderable::collect_damage(grid, lines, cols, &mut out.damage);
        out.display_offset = offset;
        let base_abs = grid.scrollback().len().saturating_sub(offset);
        out.stable_row_base = grid.total_evicted() as u64 + base_abs as u64;
        out.mode = self.mode;
    }

    /// Drain damage from the active grid.
    ///
    /// Returns a [`TermDamage`] iterator that yields dirty lines and clears
    /// marks as it goes. Check [`TermDamage::is_all_dirty`] first — when true,
    /// repaint everything and drop the iterator (which clears remaining marks).
    pub fn damage(&mut self) -> TermDamage<'_> {
        let grid = self.grid_mut();
        let cols = grid.cols();
        let all_dirty = grid.dirty().is_all_dirty();
        TermDamage::new(grid.dirty_mut().drain(), cols, all_dirty)
    }

    /// Clear all damage marks without reading them.
    ///
    /// Called when the renderer wants to discard pending damage (e.g. after
    /// a full repaint that doesn't need per-line tracking).
    pub fn reset_damage(&mut self) {
        self.grid_mut().dirty_mut().drain().for_each(drop);
    }
}
