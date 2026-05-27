//! Render-phase helpers: GPU buffer uploads and draw-pass recording.
//!
//! Extracted from [`render.rs`] to keep each file under the 500-line limit.
//! All methods here are private helpers on [`WindowRenderer`], called by the
//! entry-point render methods in [`render.rs`]. Draw-pass recording lives in
//! the [`record_passes`] submodule.

mod record_passes;

use std::time::Instant;

use super::super::pipeline::IMAGE_INSTANCE_STRIDE;
use super::WindowRenderer;
use super::helpers::{PartialUpload, upload_buffer, upload_buffer_partial};

impl WindowRenderer {
    // Buffer uploads

    /// Upload all instance buffers to the GPU.
    ///
    /// When the prepare phase used the incremental path, terminal-tier buffers
    /// (backgrounds, glyphs, subpixel, color) use partial uploads — only the
    /// bytes from the first dirty row onward are written to the GPU. Clean rows
    /// before that point already have correct data in the GPU buffer from the
    /// previous frame. Cursor, UI, and overlay tiers always do full uploads.
    ///
    /// Logs total bytes and wall time at `debug!` level for performance
    /// profiling (Section 23.4).
    pub(super) fn upload_instance_buffers(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let start = Instant::now();
        let mut total_bytes: usize = 0;

        // Terminal tier: use partial upload when the incremental path ran.
        let partial = self.first_dirty_byte_offsets();
        if let Some((bg_off, fg_off, sub_off, col_off)) = partial {
            macro_rules! upload_partial {
                ($buf:ident, $writer:ident, $offset:expr, $label:literal) => {
                    let data = self.prepared.$writer.as_bytes();
                    total_bytes += data.len() - $offset;
                    upload_buffer_partial(
                        device,
                        queue,
                        &mut self.$buf,
                        data,
                        PartialUpload {
                            offset: $offset,
                            label: $label,
                        },
                    );
                };
            }
            upload_partial!(bg_buffer, backgrounds, bg_off, "bg_instance_buffer");
            upload_partial!(fg_buffer, glyphs, fg_off, "fg_instance_buffer");
            upload_partial!(
                subpixel_fg_buffer,
                subpixel_glyphs,
                sub_off,
                "subpixel_fg_instance_buffer"
            );
            upload_partial!(
                color_fg_buffer,
                color_glyphs,
                col_off,
                "color_fg_instance_buffer"
            );
        } else {
            macro_rules! upload_full {
                ($buf:ident, $writer:ident, $label:literal) => {
                    let data = self.prepared.$writer.as_bytes();
                    total_bytes += data.len();
                    upload_buffer(device, queue, &mut self.$buf, data, $label);
                };
            }
            upload_full!(bg_buffer, backgrounds, "bg_instance_buffer");
            upload_full!(fg_buffer, glyphs, "fg_instance_buffer");
            upload_full!(
                subpixel_fg_buffer,
                subpixel_glyphs,
                "subpixel_fg_instance_buffer"
            );
            upload_full!(color_fg_buffer, color_glyphs, "color_fg_instance_buffer");
        }

        // Cursor, UI, and overlay tiers: always full upload.
        macro_rules! upload {
            ($buf:ident, $writer:ident, $label:literal) => {
                let data = self.prepared.$writer.as_bytes();
                total_bytes += data.len();
                upload_buffer(device, queue, &mut self.$buf, data, $label);
            };
        }
        upload!(cursor_buffer, cursors, "cursor_instance_buffer");
        upload!(ui_rect_buffer, ui_rects, "ui_rect_instance_buffer");
        upload!(ui_fg_buffer, ui_glyphs, "ui_fg_instance_buffer");
        upload!(
            ui_subpixel_fg_buffer,
            ui_subpixel_glyphs,
            "ui_subpixel_fg_instance_buffer"
        );
        upload!(
            ui_color_fg_buffer,
            ui_color_glyphs,
            "ui_color_fg_instance_buffer"
        );
        upload!(
            overlay_rect_buffer,
            overlay_rects,
            "overlay_rect_instance_buffer"
        );
        upload!(
            overlay_fg_buffer,
            overlay_glyphs,
            "overlay_fg_instance_buffer"
        );
        upload!(
            overlay_subpixel_fg_buffer,
            overlay_subpixel_glyphs,
            "overlay_subpixel_fg_instance_buffer"
        );
        upload!(
            overlay_color_fg_buffer,
            overlay_color_glyphs,
            "overlay_color_fg_instance_buffer"
        );

        let elapsed = start.elapsed();
        log::debug!(
            "upload_instance_buffers: {total_bytes} bytes ({:.1} KB) in {:.3}ms{}",
            total_bytes as f64 / 1024.0,
            elapsed.as_secs_f64() * 1000.0,
            if partial.is_some() { " [partial]" } else { "" },
        );
    }

    /// Compute per-buffer byte offsets for partial terminal-tier upload.
    ///
    /// Returns `(bg_offset, glyph_offset, subpixel_offset, color_offset)` —
    /// the byte offset in each buffer from which the GPU data diverges from
    /// the previous frame. Returns `None` if partial upload is not possible
    /// (full rebuild, no dirty info, or row count mismatch).
    fn first_dirty_byte_offsets(&self) -> Option<(usize, usize, usize, usize)> {
        let frame = &self.prepared;
        if !frame.was_incremental {
            return None;
        }
        let dirty = &frame.scratch_dirty;
        let ranges = &frame.row_ranges;
        if dirty.is_empty() || ranges.is_empty() {
            return None;
        }

        // Find the first dirty row.
        let first = dirty.iter().position(|&d| d)?;
        if first >= ranges.len() {
            return None;
        }

        let r = &ranges[first];
        Some((
            r.backgrounds.start,
            r.glyphs.start,
            r.subpixel_glyphs.start,
            r.color_glyphs.start,
        ))
    }

    /// Upload only the transient overlay and cursor buffers.
    ///
    /// Used when the cached terminal/chrome content is still valid and only
    /// the overlay or cursor layer needs to change.
    pub(super) fn upload_overlay_and_cursor_buffers(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        macro_rules! upload {
            ($buf:ident, $writer:ident, $label:literal) => {
                upload_buffer(
                    device,
                    queue,
                    &mut self.$buf,
                    self.prepared.$writer.as_bytes(),
                    $label,
                );
            };
        }

        upload!(cursor_buffer, cursors, "cursor_instance_buffer");
        upload!(
            overlay_rect_buffer,
            overlay_rects,
            "overlay_rect_instance_buffer"
        );
        upload!(
            overlay_fg_buffer,
            overlay_glyphs,
            "overlay_fg_instance_buffer"
        );
        upload!(
            overlay_subpixel_fg_buffer,
            overlay_subpixel_glyphs,
            "overlay_subpixel_fg_instance_buffer"
        );
        upload!(
            overlay_color_fg_buffer,
            overlay_color_glyphs,
            "overlay_color_fg_instance_buffer"
        );
    }

    /// Upload image quad instances to a shared GPU buffer.
    ///
    /// Each image quad is 36 bytes. All quads (below + above text) are packed
    /// into a single buffer. Individual draw calls index into this buffer
    /// with vertex buffer offsets.
    pub(super) fn upload_image_instances(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let below = &self.prepared.image_quads_below;
        let above = &self.prepared.image_quads_above;
        let total = below.len() + above.len();
        if total == 0 {
            return;
        }

        let stride = IMAGE_INSTANCE_STRIDE as usize;
        self.image_instance_data.clear();
        self.image_instance_data.reserve(total * stride);

        for quad in below.iter().chain(above.iter()) {
            self.image_instance_data
                .extend_from_slice(&quad.x.to_le_bytes());
            self.image_instance_data
                .extend_from_slice(&quad.y.to_le_bytes());
            self.image_instance_data
                .extend_from_slice(&quad.w.to_le_bytes());
            self.image_instance_data
                .extend_from_slice(&quad.h.to_le_bytes());
            self.image_instance_data
                .extend_from_slice(&quad.uv_x.to_le_bytes());
            self.image_instance_data
                .extend_from_slice(&quad.uv_y.to_le_bytes());
            self.image_instance_data
                .extend_from_slice(&quad.uv_w.to_le_bytes());
            self.image_instance_data
                .extend_from_slice(&quad.uv_h.to_le_bytes());
            self.image_instance_data
                .extend_from_slice(&quad.opacity.to_le_bytes());
        }

        upload_buffer(
            device,
            queue,
            &mut self.image_instance_buffer,
            &self.image_instance_data,
            "image_instance_buffer",
        );
    }

    /// Resolved clear color from the prepared frame.
    pub(super) fn clear_color(&self) -> wgpu::Color {
        wgpu::Color {
            r: self.prepared.clear_color[0],
            g: self.prepared.clear_color[1],
            b: self.prepared.clear_color[2],
            a: self.prepared.clear_color[3],
        }
    }
}
