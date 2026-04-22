//! Kitty graphics `a=p` (place) action + placement construction helper.

use crate::effect::sink::EffectSink;
use crate::grid::StableRowIndex;
use crate::image::kitty::KittyCommand;
use crate::image::{ImageId, ImagePlacement, PlacementSizing};
use crate::term::Term;

impl<S: EffectSink> Term<S> {
    /// Place a previously uploaded image.
    pub(super) fn kitty_place(&mut self, cmd: &KittyCommand) {
        let Some(image_id) = cmd.image_id else {
            self.kitty_respond(0, cmd.quiet, "ENOENT");
            return;
        };

        if self.image_cache().get_no_touch(ImageId(image_id)).is_none() {
            self.kitty_respond(image_id, cmd.quiet, "ENOENT");
            return;
        }

        // U=1: placement deferred to unicode placeholder chars in cells.
        if !cmd.unicode_placeholder {
            self.kitty_create_placement(image_id, cmd);
        }
        self.kitty_respond(image_id, cmd.quiet, "OK");
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
        }
    }
}
