//! Sixel graphics protocol handler.
//!
//! Handles DCS sixel sequences: accumulates pixel data via
//! `sixel_start`/`sixel_put`/`sixel_end`, then stores the decoded
//! image in `ImageCache` and places it at the cursor.

use std::sync::Arc;

use log::warn;

use crate::event::EventListener;
use crate::grid::StableRowIndex;
use crate::image::sixel::SixelParser;
use crate::image::{ImageData, ImageFormat, ImageId, ImagePlacement, ImageSource};
use crate::term::{Term, TermMode};

impl<T: EventListener> Term<T> {
    /// Begin a sixel sequence: create parser from DCS params.
    pub(in crate::term::handler) fn handle_sixel_start(&mut self, params: &[u16]) {
        self.sixel_parser = Some(SixelParser::new(params));
    }

    /// Feed one byte to the active sixel parser.
    pub(in crate::term::handler) fn handle_sixel_put(&mut self, byte: u8) {
        if let Some(ref mut parser) = self.sixel_parser {
            parser.feed(byte);
        }
    }

    /// Finalize sixel image: decode, store in cache, place at cursor.
    pub(in crate::term::handler) fn handle_sixel_end(&mut self) {
        let Some(parser) = self.sixel_parser.take() else {
            return;
        };

        let (rgba, w, h) = match parser.finish() {
            Ok(result) => result,
            Err(e) => {
                warn!("sixel decode failed: {e}");
                return;
            }
        };

        // Store image in cache.
        let id = self.image_cache_mut().next_image_id();
        let img = ImageData {
            id,
            width: w,
            height: h,
            data: Arc::new(rgba),
            format: ImageFormat::Rgba,
            source: ImageSource::Direct,
            last_accessed: 0,
        };

        if let Err(e) = self.image_cache_mut().store(img) {
            warn!("sixel image store failed: {e}");
            return;
        }

        self.sixel_create_placement(id, w, h);
    }

    /// Create an image placement for a sixel image at the current cursor.
    fn sixel_create_placement(&mut self, id: ImageId, w: u32, h: u32) {
        let grid = self.grid();
        let col = grid.cursor().col().0;
        let line = grid.cursor().line();
        let scrollback_len = grid.scrollback().len();
        let display_offset = grid.display_offset();
        let abs_row = scrollback_len.saturating_sub(display_offset) + line;
        let stable_row = StableRowIndex(abs_row as u64);

        let cell_w = self.cell_pixel_width.max(1) as u32;
        let cell_h = self.cell_pixel_height.max(1) as u32;
        let cols = w.div_ceil(cell_w) as usize;
        let rows = h.div_ceil(cell_h) as usize;

        let placement = ImagePlacement {
            image_id: id,
            placement_id: None,
            source_x: 0,
            source_y: 0,
            source_w: w,
            source_h: h,
            cell_col: col,
            cell_row: stable_row,
            cols,
            rows,
            z_index: 0,
            cell_x_offset: 0,
            cell_y_offset: 0,
        };

        self.image_cache_mut().place(placement);

        // Advance cursor based on sixel modes.
        let sixel_scrolling = self.mode.contains(TermMode::SIXEL_SCROLLING);
        let cursor_right = self.mode.contains(TermMode::SIXEL_CURSOR_RIGHT);

        if cursor_right {
            let new_col = col + cols;
            let max_col = self.grid().cols().saturating_sub(1);
            self.grid_mut()
                .move_to_column(crate::index::Column(new_col.min(max_col)));
        } else if sixel_scrolling {
            let prev = self.grid().total_evicted();
            let grid = self.grid_mut();
            for _ in 0..rows.saturating_sub(1) {
                grid.linefeed();
            }
            self.prune_images_if_evicted(prev);
        } else {
            // DECSDM off: cursor stays at original position.
        }
    }
}
