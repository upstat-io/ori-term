//! Kitty graphics `a=p` (place) action + placement construction helper.

use crate::effect::sink::EffectSink;
use crate::grid::StableRowIndex;
use crate::image::kitty::KittyCommand;
use crate::image::{ImageId, ImagePlacement, PlacementSizing};
use crate::index::Column;
use crate::term::Term;

use super::KittyReplyContext;

impl<S: EffectSink> Term<S> {
    /// Place a previously uploaded image.
    pub(super) fn kitty_place(&mut self, cmd: &KittyCommand) {
        let ctx = KittyReplyContext::from_cmd(cmd);

        let Some(image_id) = cmd.image_id else {
            self.kitty_respond(&ctx, "ENOENT");
            return;
        };

        if self.image_cache().get_no_touch(ImageId(image_id)).is_none() {
            self.kitty_respond(&ctx, "ENOENT");
            return;
        }

        // U=1: placement deferred to unicode placeholder chars in cells.
        // Record the anchor so LRU eviction does not drop the image while
        // the program writes its `U+10EEEE` cells. When `c=N,r=M` are
        // present, also record the placeholder display grid so the GPU
        // emit path can compute per-cell UV slices (§13.6.1 multi-cell
        // UV slicing).
        if cmd.unicode_placeholder {
            let id = ImageId::from_raw(image_id);
            let grid = match (cmd.display_cols, cmd.display_rows) {
                (Some(cols), Some(rows)) => Some((cols, rows)),
                _ => None,
            };
            self.image_cache_mut()
                .anchor_placeholder_with_grid(id, grid);
        } else {
            self.kitty_create_placement(image_id, cmd);
        }
        self.kitty_respond(&ctx, "OK");
    }

    /// Create a placement at the current cursor position.
    pub(super) fn kitty_create_placement(&mut self, image_id: u32, cmd: &KittyCommand) {
        let grid = self.grid();
        let cursor = grid.cursor();
        let col = cursor.col().0;
        let line = cursor.line();
        let stable_row = StableRowIndex::from_visible(grid, line);

        let img = self.image_cache().get_no_touch(ImageId(image_id));
        let (img_w, img_h) = img.map_or((0, 0), |i| (i.width, i.height));

        let cell_w = self.cell_pixel_width.max(1) as u32;
        let cell_h = self.cell_pixel_height.max(1) as u32;

        // Explicit c=/r= → cell-count sizing (scales with cell dimensions).
        // Otherwise → fixed-pixel sizing (image keeps its pixel dimensions).
        let explicit_cells = cmd.display_cols.is_some() || cmd.display_rows.is_some();

        let cols = cmd
            .display_cols
            .unwrap_or_else(|| if img_w > 0 { img_w.div_ceil(cell_w) } else { 1 })
            as usize;
        let rows = cmd
            .display_rows
            .unwrap_or_else(|| if img_h > 0 { img_h.div_ceil(cell_h) } else { 1 })
            as usize;

        let sizing = if explicit_cells {
            PlacementSizing::CellCount
        } else {
            PlacementSizing::FixedPixels {
                width: cols as u32 * cell_w,
                height: rows as u32 * cell_h,
            }
        };

        let placement = ImagePlacement {
            image_id: ImageId(image_id),
            placement_id: cmd.placement_id,
            source_x: cmd.source_x,
            source_y: cmd.source_y,
            source_w: cmd.source_width,
            source_h: cmd.source_height,
            cell_col: col,
            cell_row: stable_row,
            cols,
            rows,
            z_index: cmd.z_index,
            cell_x_offset: cmd.cell_x_offset as u16,
            cell_y_offset: cmd.cell_y_offset as u16,
            sizing,
        };

        self.image_cache_mut().place(placement);

        if !cmd.no_cursor_move {
            let grid = self.grid_mut();
            for _ in 0..rows.saturating_sub(1) {
                grid.linefeed();
            }
            // Per kitty graphics protocol: after `a=p` / `a=T`, the cursor
            // moves PAST the placed image — by `cols` columns and
            // `rows - 1` linefeeds — to mimic text-write behavior.
            // Without the horizontal advance, the cursor stays on the
            // placement's origin cell, and the cursor block in the
            // overlay pass overdraws the image (the §13.6.1-discovered
            // symptom: cache view has image pixels but final readback
            // shows cursor color).
            //
            // Clamp to `grid.cols()` (NOT `cols() - 1`) so the cursor
            // can enter the wrap-pending state used by
            // `handler/helpers.rs:339-370` (`cursor_was_past_edge` /
            // `set_col(Column(cols))` for the LINE_WRAP wrap-target
            // case). Text written after an image that fills to the
            // right edge then wraps to the next line correctly,
            // instead of overwriting the rightmost image column.
            let max_col = grid.cols();
            let current_col = grid.cursor().col().0;
            let new_col = current_col.saturating_add(cols).min(max_col);
            grid.cursor_mut().set_col(Column(new_col));
        }
    }
}
