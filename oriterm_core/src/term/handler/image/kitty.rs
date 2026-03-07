//! Kitty graphics protocol handler.
//!
//! Handles APC `G` commands: transmit, place, delete, query.
//! Animation (`a=f`, `a=a`) is in sibling `kitty_animation.rs`.

use std::sync::Arc;

use log::{debug, warn};

use crate::event::{Event, EventListener};
use crate::grid::StableRowIndex;
use crate::image::kitty::{
    KittyAction, KittyCommand, KittyTransmission, LoadingImage, parse_kitty_command,
};
use crate::image::{
    ImageData, ImageId, ImagePlacement, ImageSource, PlacementSizing, decode_to_rgba, rgb_to_rgba,
};
use crate::term::Term;

/// Parameters for storing an image via Kitty protocol.
pub(super) struct KittyStoreParams {
    pub(super) image_id: u32,
    pub(super) payload: Vec<u8>,
    pub(super) format: u32,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) transmission: KittyTransmission,
}

impl<T: EventListener> Term<T> {
    /// Parse and execute a Kitty graphics command.
    pub(super) fn handle_kitty_graphics(&mut self, data: &[u8]) {
        if !self.image_protocol_enabled {
            return;
        }
        let cmd = match parse_kitty_command(data) {
            Ok(cmd) => cmd,
            Err(e) => {
                warn!("kitty graphics parse error: {e}");
                return;
            }
        };

        match cmd.action {
            KittyAction::Query => self.kitty_query(&cmd),
            KittyAction::Transmit => self.kitty_transmit(cmd),
            KittyAction::TransmitAndPlace => self.kitty_transmit_and_place(cmd),
            KittyAction::Place => self.kitty_place(&cmd),
            KittyAction::Delete => self.kitty_delete(&cmd),
            KittyAction::Frame => self.kitty_frame(cmd),
            KittyAction::Animate => self.kitty_animate(&cmd),
        }
    }

    /// Query: send OK response without modifying state.
    fn kitty_query(&self, cmd: &KittyCommand) {
        let id = cmd.image_id.unwrap_or(0);
        self.kitty_respond(id, cmd.quiet, "OK");
    }

    /// Transmit: upload image data (possibly chunked).
    fn kitty_transmit(&mut self, cmd: KittyCommand) {
        if cmd.more_data {
            self.kitty_accumulate_chunk(cmd);
            return;
        }

        let params = self.kitty_finalize_payload(&cmd);
        let image_id = params.image_id;

        if let Err(msg) = self.kitty_store_image(params) {
            warn!("kitty transmit failed: {msg}");
            self.kitty_respond(image_id, cmd.quiet, &msg);
        } else {
            self.kitty_respond(image_id, cmd.quiet, "OK");
        }
    }

    /// Transmit and place in one step.
    fn kitty_transmit_and_place(&mut self, cmd: KittyCommand) {
        if cmd.more_data {
            self.kitty_accumulate_chunk(cmd);
            return;
        }

        let params = self.kitty_finalize_payload(&cmd);
        let image_id = params.image_id;

        if let Err(msg) = self.kitty_store_image(params) {
            warn!("kitty transmit+place failed: {msg}");
            self.kitty_respond(image_id, cmd.quiet, &msg);
            return;
        }

        // U=1: image stored but placement deferred to unicode placeholder
        // chars (U+10EEEE) that the program writes into cells.
        if !cmd.unicode_placeholder {
            self.kitty_create_placement(image_id, &cmd);
        }
        self.kitty_respond(image_id, cmd.quiet, "OK");
    }

    /// Finalize payload from accumulated chunks or single command.
    pub(super) fn kitty_finalize_payload(&mut self, cmd: &KittyCommand) -> KittyStoreParams {
        let (payload, format, width, height, transmission) =
            if let Some(mut loading) = self.loading_image.take() {
                loading.payload.extend_from_slice(&cmd.payload);
                (
                    loading.payload,
                    loading.format,
                    loading.width,
                    loading.height,
                    loading.transmission,
                )
            } else {
                (
                    cmd.payload.clone(),
                    cmd.format,
                    cmd.source_width,
                    cmd.source_height,
                    cmd.transmission,
                )
            };

        let image_id = cmd
            .image_id
            .unwrap_or_else(|| self.image_cache_mut().next_image_id().0);

        KittyStoreParams {
            image_id,
            payload,
            format,
            width,
            height,
            transmission,
        }
    }

    /// Place a previously uploaded image.
    fn kitty_place(&mut self, cmd: &KittyCommand) {
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

    /// Delete images/placements based on the delete specifier.
    fn kitty_delete(&mut self, cmd: &KittyCommand) {
        let spec = cmd.delete_specifier.unwrap_or(b'a');

        // Extract cursor position before borrowing cache.
        let grid = self.grid();
        let cursor = grid.cursor();
        let cursor_col = cursor.col().0;
        let cursor_line = cursor.line();
        let scrollback_len = grid.scrollback().len();
        let display_offset = grid.display_offset();
        let abs_row = scrollback_len.saturating_sub(display_offset) + cursor_line;
        let cursor_row = StableRowIndex(abs_row as u64);

        let cache = self.image_cache_mut();

        match spec {
            // Lowercase: delete placements only.
            // Uppercase: delete image data + all placements.
            b'a' | b'A' => cache.clear(),
            b'i' => {
                if let Some(id) = cmd.image_id {
                    cache.remove_placements_for_image(ImageId(id));
                }
            }
            b'I' => {
                if let Some(id) = cmd.image_id {
                    cache.remove_image(ImageId(id));
                }
            }
            b'p' => {
                if let Some(pid) = cmd.placement_id {
                    if let Some(id) = cmd.image_id {
                        cache.remove_placement(ImageId(id), pid);
                    }
                }
            }
            b'P' => {
                if let Some(pid) = cmd.placement_id {
                    if let Some(id) = cmd.image_id {
                        cache.remove_placement(ImageId(id), pid);
                        cache.remove_image(ImageId(id));
                    }
                }
            }
            b'c' => cache.remove_placements_at_column(cursor_col),
            b'C' => {
                cache.remove_placements_at_column(cursor_col);
                cache.remove_orphans();
            }
            b'x' => {
                let col = cmd.source_x as usize;
                cache.remove_placements_at_column(col);
            }
            b'X' => {
                let col = cmd.source_x as usize;
                cache.remove_placements_at_column(col);
                cache.remove_orphans();
            }
            b'y' => {
                let row = StableRowIndex(u64::from(cmd.source_y));
                cache.remove_placements_at_row(row);
            }
            b'Y' => {
                let row = StableRowIndex(u64::from(cmd.source_y));
                cache.remove_placements_at_row(row);
                cache.remove_orphans();
            }
            b'z' => {
                let z = cmd.z_index;
                cache.remove_placements_by_z_index(z);
            }
            b'Z' => {
                let z = cmd.z_index;
                cache.remove_placements_by_z_index(z);
                cache.remove_orphans();
            }
            b'r' | b'R' => {
                cache.remove_by_position(cursor_col, cursor_row);
                if spec == b'R' {
                    cache.remove_orphans();
                }
            }
            b'n' => debug!("kitty delete d=n not yet implemented"),
            b'N' => debug!("kitty delete d=N not yet implemented"),
            _ => debug!("kitty delete specifier {:?} not implemented", spec as char),
        }
    }

    /// Accumulate a chunk for multi-part transmission.
    pub(super) fn kitty_accumulate_chunk(&mut self, cmd: KittyCommand) {
        let max_bytes = self.image_cache().max_single_image_bytes();

        if let Some(ref mut loading) = self.loading_image {
            loading.payload.extend_from_slice(&cmd.payload);
            if loading.payload.len() > max_bytes {
                warn!("kitty chunked transfer exceeds max size, discarding");
                self.loading_image = None;
            }
        } else {
            let image_id = cmd
                .image_id
                .unwrap_or_else(|| self.image_cache_mut().next_image_id().0);
            self.loading_image = Some(LoadingImage {
                image_id,
                image_number: cmd.image_number,
                payload: cmd.payload,
                format: cmd.format,
                width: cmd.source_width,
                height: cmd.source_height,
                compression: cmd.compression,
                transmission: cmd.transmission,
            });
        }
    }

    /// Decode and store image data in the cache.
    fn kitty_store_image(&mut self, p: KittyStoreParams) -> Result<(), String> {
        let (pixel_data, source) = match p.transmission {
            KittyTransmission::Direct => (p.payload, ImageSource::Direct),
            KittyTransmission::File | KittyTransmission::TempFile => {
                return self.kitty_store_from_file(&p);
            }
            KittyTransmission::SharedMemory => {
                return Err("EINVAL: shared memory transmission not yet supported".to_string());
            }
        };

        let (rgba_data, w, h) = Self::kitty_decode_pixels(pixel_data, p.format, p.width, p.height)?;

        let img = ImageData {
            id: ImageId(p.image_id),
            width: w,
            height: h,
            data: Arc::new(rgba_data),
            format: crate::image::ImageFormat::Rgba,
            source,
            last_accessed: 0,
        };

        self.image_cache_mut()
            .store(img)
            .map_err(|e| format!("ENOMEM: {e}"))?;

        Ok(())
    }

    /// Decode pixel data from format code to RGBA.
    pub(super) fn kitty_decode_pixels(
        payload: Vec<u8>,
        format: u32,
        width: u32,
        height: u32,
    ) -> Result<(Vec<u8>, u32, u32), String> {
        match format {
            32 => {
                if width == 0 || height == 0 {
                    return Err("EINVAL: missing s= or v= for raw RGBA".to_string());
                }
                let expected = (width as usize) * (height as usize) * 4;
                if payload.len() != expected {
                    return Err(format!(
                        "EINVAL: RGBA payload size {} != expected {expected}",
                        payload.len(),
                    ));
                }
                Ok((payload, width, height))
            }
            24 => {
                if width == 0 || height == 0 {
                    return Err("EINVAL: missing s= or v= for raw RGB".to_string());
                }
                rgb_to_rgba(&payload)
                    .map(|rgba| (rgba, width, height))
                    .ok_or_else(|| "EINVAL: RGB payload length not a multiple of 3".to_string())
            }
            100 => decode_to_rgba(&payload).map_err(|e| format!("EINVAL: PNG decode failed: {e}")),
            _ => Err(format!("EINVAL: unsupported format f={format}")),
        }
    }

    /// Store image from a file path (t=f or t=t transmission).
    fn kitty_store_from_file(&mut self, p: &KittyStoreParams) -> Result<(), String> {
        let path_str = std::str::from_utf8(&p.payload)
            .map_err(|_e| "EINVAL: file path is not valid UTF-8".to_string())?;

        let path = std::path::Path::new(path_str);

        if path_str.contains("..") {
            return Err("EINVAL: path traversal not allowed".to_string());
        }

        let max_bytes = self.image_cache().max_single_image_bytes();
        let file_data =
            std::fs::read(path).map_err(|e| format!("EIO: failed to read file: {e}"))?;

        if file_data.len() > max_bytes {
            if p.transmission == KittyTransmission::TempFile {
                let _ = std::fs::remove_file(path);
            }
            return Err("ENOMEM: file exceeds max image size".to_string());
        }

        if p.transmission == KittyTransmission::TempFile {
            let _ = std::fs::remove_file(path);
        }

        let source = ImageSource::File(path.to_path_buf());

        let (rgba_data, w, h) = if p.format == 24 || p.format == 32 {
            Self::kitty_decode_pixels(file_data, p.format, p.width, p.height)?
        } else {
            decode_to_rgba(&file_data).map_err(|e| format!("EINVAL: image decode failed: {e}"))?
        };

        let img = ImageData {
            id: ImageId(p.image_id),
            width: w,
            height: h,
            data: Arc::new(rgba_data),
            format: crate::image::ImageFormat::Rgba,
            source,
            last_accessed: 0,
        };

        self.image_cache_mut()
            .store(img)
            .map_err(|e| format!("ENOMEM: {e}"))?;

        Ok(())
    }

    /// Create a placement at the current cursor position.
    fn kitty_create_placement(&mut self, image_id: u32, cmd: &KittyCommand) {
        let grid = self.grid();
        let cursor = grid.cursor();
        let col = cursor.col().0;
        let line = cursor.line();
        let scrollback_len = grid.scrollback().len();
        let display_offset = grid.display_offset();

        let abs_row = scrollback_len.saturating_sub(display_offset) + line;
        let stable_row = StableRowIndex(abs_row as u64);

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

    /// Send a Kitty graphics response.
    pub(super) fn kitty_respond(&self, image_id: u32, quiet: u8, msg: &str) {
        if quiet >= 2 {
            return;
        }
        if quiet >= 1 && msg == "OK" {
            return;
        }

        let response = format!("\x1b_Gi={image_id};{msg}\x1b\\");
        self.event_listener.send_event(Event::PtyWrite(response));
    }
}
