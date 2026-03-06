//! iTerm2 image protocol handler.
//!
//! Handles OSC 1337 File= sequences: parses args, decodes image data,
//! stores in `ImageCache`, and creates a placement at cursor.

use std::sync::Arc;

use log::warn;

use crate::event::EventListener;
use crate::grid::StableRowIndex;
use crate::image::iterm2::{Iterm2Image, SizeSpec, parse_iterm2_file};
use crate::image::{ImageData, ImageFormat, ImageId, ImagePlacement, ImageSource, decode_to_rgba};
use crate::term::Term;

/// Parameters for resolving iTerm2 display dimensions.
struct DisplaySizeParams {
    w_spec: SizeSpec,
    h_spec: SizeSpec,
    img_w: u32,
    img_h: u32,
    cell_w: u32,
    cell_h: u32,
    term_w: u32,
    term_h: u32,
    preserve_aspect: bool,
}

impl<T: EventListener> Term<T> {
    /// Parse and execute an iTerm2 File= image command.
    pub(in crate::term::handler) fn handle_iterm2_file(&mut self, params: &[&[u8]]) {
        let image = match parse_iterm2_file(params) {
            Ok(img) => img,
            Err(e) => {
                warn!("iTerm2 image parse error: {e}");
                return;
            }
        };

        // Non-inline images are downloads — not displayed.
        if !image.inline {
            return;
        }

        let max_bytes = self.image_cache().max_single_image_bytes();
        if image.data.len() > max_bytes {
            warn!(
                "iTerm2 image exceeds max size ({} > {max_bytes})",
                image.data.len()
            );
            return;
        }

        // Decode image to RGBA pixels.
        let (rgba, img_w, img_h) = match decode_to_rgba(&image.data) {
            Ok(result) => result,
            Err(e) => {
                warn!("iTerm2 image decode failed: {e}");
                return;
            }
        };

        // Store image in cache.
        let id = self.image_cache_mut().next_image_id();
        let img_data = ImageData {
            id,
            width: img_w,
            height: img_h,
            data: Arc::new(rgba),
            format: ImageFormat::Rgba,
            source: ImageSource::Direct,
            last_accessed: 0,
        };

        if let Err(e) = self.image_cache_mut().store(img_data) {
            warn!("iTerm2 image store failed: {e}");
            return;
        }

        self.iterm2_create_placement(id, img_w, img_h, &image);
    }

    /// Create an image placement at the current cursor position.
    fn iterm2_create_placement(
        &mut self,
        id: ImageId,
        img_w: u32,
        img_h: u32,
        image: &Iterm2Image,
    ) {
        let cell_w = self.cell_pixel_width.max(1) as u32;
        let cell_h = self.cell_pixel_height.max(1) as u32;
        let term_cols = self.grid().cols();
        let term_lines = self.grid().lines();

        let (display_w, display_h) = resolve_display_size(&DisplaySizeParams {
            w_spec: image.width,
            h_spec: image.height,
            img_w,
            img_h,
            cell_w,
            cell_h,
            term_w: term_cols as u32 * cell_w,
            term_h: term_lines as u32 * cell_h,
            preserve_aspect: image.preserve_aspect_ratio,
        });

        // Clamp to terminal width.
        let display_w = display_w.min(term_cols as u32 * cell_w);

        let cols = display_w.div_ceil(cell_w) as usize;
        let rows = display_h.div_ceil(cell_h) as usize;

        let grid = self.grid();
        let col = grid.cursor().col().0;
        let line = grid.cursor().line();
        let scrollback_len = grid.scrollback().len();
        let display_offset = grid.display_offset();
        let abs_row = scrollback_len.saturating_sub(display_offset) + line;
        let stable_row = StableRowIndex(abs_row as u64);

        let placement = ImagePlacement {
            image_id: id,
            placement_id: None,
            source_x: 0,
            source_y: 0,
            source_w: img_w,
            source_h: img_h,
            cell_col: col,
            cell_row: stable_row,
            cols,
            rows,
            z_index: 0,
            cell_x_offset: 0,
            cell_y_offset: 0,
        };

        self.image_cache_mut().place(placement);

        // Cursor advances below image.
        let prev = self.grid().total_evicted();
        let grid = self.grid_mut();
        for _ in 0..rows.saturating_sub(1) {
            grid.linefeed();
        }
        self.prune_images_if_evicted(prev);
    }
}

/// Resolve display pixel size from iTerm2 size specs.
///
/// Handles `Auto`, `Cells`, `Pixels`, and `Percent` modes for both width
/// and height, with optional aspect ratio preservation.
fn resolve_display_size(p: &DisplaySizeParams) -> (u32, u32) {
    let raw_w = resolve_one_dimension(p.w_spec, p.img_w, p.cell_w, p.term_w);
    let raw_h = resolve_one_dimension(p.h_spec, p.img_h, p.cell_h, p.term_h);

    if !p.preserve_aspect {
        return (raw_w.max(1), raw_h.max(1));
    }

    // Preserve aspect ratio when one or both dimensions are auto.
    let w_is_auto = p.w_spec == SizeSpec::Auto;
    let h_is_auto = p.h_spec == SizeSpec::Auto;

    match (w_is_auto, h_is_auto) {
        // Both auto: use native size, clamped to terminal width.
        (true, true) => {
            let w = p.img_w.min(p.term_w).max(1);
            let h = if p.img_w > 0 {
                (u64::from(p.img_h) * u64::from(w) / u64::from(p.img_w).max(1)) as u32
            } else {
                p.img_h
            };
            (w.max(1), h.max(1))
        }
        // Width explicit, height auto: scale height to match.
        (false, true) => {
            let w = raw_w.max(1);
            let h = if p.img_w > 0 {
                (u64::from(p.img_h) * u64::from(w) / u64::from(p.img_w).max(1)) as u32
            } else {
                p.img_h
            };
            (w, h.max(1))
        }
        // Height explicit, width auto: scale width to match.
        (true, false) => {
            let h = raw_h.max(1);
            let w = if p.img_h > 0 {
                (u64::from(p.img_w) * u64::from(h) / u64::from(p.img_h).max(1)) as u32
            } else {
                p.img_w
            };
            (w.max(1), h)
        }
        // Both explicit: use as-is (aspect not preserved when both specified).
        (false, false) => (raw_w.max(1), raw_h.max(1)),
    }
}

/// Resolve a single dimension from a `SizeSpec`.
fn resolve_one_dimension(spec: SizeSpec, native: u32, cell_size: u32, term_size: u32) -> u32 {
    match spec {
        SizeSpec::Auto => native,
        SizeSpec::Cells(n) => n * cell_size,
        SizeSpec::Pixels(n) => n,
        SizeSpec::Percent(pct) => term_size * pct / 100,
    }
}
