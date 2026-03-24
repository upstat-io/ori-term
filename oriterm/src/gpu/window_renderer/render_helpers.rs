//! Render-phase helpers: GPU buffer uploads and draw-pass recording.
//!
//! Extracted from [`render.rs`] to keep each file under the 500-line limit.
//! All methods here are private helpers on [`WindowRenderer`], called by the
//! entry-point render methods in [`render.rs`].

use std::time::Instant;

use wgpu::RenderPass;

use super::super::pipeline::IMAGE_INSTANCE_STRIDE;
use super::WindowRenderer;
use super::helpers::{record_draw, record_draw_range, upload_buffer};
use crate::gpu::pipelines::GpuPipelines;

impl WindowRenderer {
    // Buffer uploads

    /// Upload all instance buffers to the GPU.
    ///
    /// Logs total bytes and wall time at `debug!` level for performance
    /// profiling (Section 23.4).
    pub(super) fn upload_instance_buffers(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let start = Instant::now();
        let mut total_bytes: usize = 0;

        macro_rules! upload {
            ($buf:ident, $writer:ident, $label:literal) => {
                let data = self.prepared.$writer.as_bytes();
                total_bytes += data.len();
                upload_buffer(device, queue, &mut self.$buf, data, $label);
            };
        }
        upload!(bg_buffer, backgrounds, "bg_instance_buffer");
        upload!(fg_buffer, glyphs, "fg_instance_buffer");
        upload!(
            subpixel_fg_buffer,
            subpixel_glyphs,
            "subpixel_fg_instance_buffer"
        );
        upload!(color_fg_buffer, color_glyphs, "color_fg_instance_buffer");
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
            "upload_instance_buffers: {total_bytes} bytes ({:.1} KB) in {:.3}ms",
            total_bytes as f64 / 1024.0,
            elapsed.as_secs_f64() * 1000.0,
        );
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

    // Draw pass recording

    /// Record cached-content draw passes.
    ///
    /// Two tiers in painter's order:
    /// - Terminal: cell backgrounds, images below text, glyphs, images above
    /// - Chrome: UI rects + chrome text (tab bar, search bar)
    pub(super) fn record_cached_content_passes<'a>(
        pipelines: &'a GpuPipelines,
        renderer: &'a Self,
        pass: &mut RenderPass<'a>,
    ) {
        let bg = renderer.uniform_buffer.bind_group();
        let mono = Some(renderer.atlas_bind_group.bind_group());
        let sub = Some(renderer.subpixel_atlas_bind_group.bind_group());
        let color = Some(renderer.color_atlas_bind_group.bind_group());
        let p = &renderer.prepared;

        // Terminal tier: backgrounds.
        record_draw(
            pass,
            &pipelines.bg_pipeline,
            bg,
            None,
            renderer.bg_buffer.as_ref(),
            p.backgrounds.len() as u32,
        );

        // Images below text (z_index < 0).
        Self::record_image_draws(
            pipelines,
            renderer,
            pass,
            &p.image_quads_below,
            0, // buffer offset: below quads come first
        );

        // Terminal tier: text glyphs.
        record_draw(
            pass,
            &pipelines.fg_pipeline,
            bg,
            mono,
            renderer.fg_buffer.as_ref(),
            p.glyphs.len() as u32,
        );
        record_draw(
            pass,
            &pipelines.subpixel_fg_pipeline,
            bg,
            sub,
            renderer.subpixel_fg_buffer.as_ref(),
            p.subpixel_glyphs.len() as u32,
        );
        record_draw(
            pass,
            &pipelines.color_fg_pipeline,
            bg,
            color,
            renderer.color_fg_buffer.as_ref(),
            p.color_glyphs.len() as u32,
        );

        // Images above text (z_index >= 0).
        Self::record_image_draws(
            pipelines,
            renderer,
            pass,
            &p.image_quads_above,
            p.image_quads_below.len(), // buffer offset: above quads start after below
        );

        // Chrome tier — per-instance clip rects handle clipping in the shader.
        record_draw(
            pass,
            &pipelines.ui_rect_pipeline,
            bg,
            None,
            renderer.ui_rect_buffer.as_ref(),
            p.ui_rects.len() as u32,
        );
        record_draw(
            pass,
            &pipelines.fg_pipeline,
            bg,
            mono,
            renderer.ui_fg_buffer.as_ref(),
            p.ui_glyphs.len() as u32,
        );
        record_draw(
            pass,
            &pipelines.subpixel_fg_pipeline,
            bg,
            sub,
            renderer.ui_subpixel_fg_buffer.as_ref(),
            p.ui_subpixel_glyphs.len() as u32,
        );
        record_draw(
            pass,
            &pipelines.color_fg_pipeline,
            bg,
            color,
            renderer.ui_color_fg_buffer.as_ref(),
            p.ui_color_glyphs.len() as u32,
        );
    }

    /// Record overlay draw passes only.
    pub(super) fn record_overlay_pass<'a>(
        pipelines: &'a GpuPipelines,
        renderer: &'a Self,
        pass: &mut RenderPass<'a>,
    ) {
        let bg = renderer.uniform_buffer.bind_group();
        let mono = Some(renderer.atlas_bind_group.bind_group());
        let sub = Some(renderer.subpixel_atlas_bind_group.bind_group());
        let color = Some(renderer.color_atlas_bind_group.bind_group());
        let p = &renderer.prepared;

        for range in &p.overlay_draw_ranges {
            record_draw_range(
                pass,
                &pipelines.ui_rect_pipeline,
                bg,
                None,
                renderer.overlay_rect_buffer.as_ref(),
                range.rects.0,
                range.rects.1,
            );
            record_draw_range(
                pass,
                &pipelines.fg_pipeline,
                bg,
                mono,
                renderer.overlay_fg_buffer.as_ref(),
                range.mono.0,
                range.mono.1,
            );
            record_draw_range(
                pass,
                &pipelines.subpixel_fg_pipeline,
                bg,
                sub,
                renderer.overlay_subpixel_fg_buffer.as_ref(),
                range.subpixel.0,
                range.subpixel.1,
            );
            record_draw_range(
                pass,
                &pipelines.color_fg_pipeline,
                bg,
                color,
                renderer.overlay_color_fg_buffer.as_ref(),
                range.color.0,
                range.color.1,
            );
        }
    }

    /// Record cursor draw pass only.
    pub(super) fn record_cursor_pass<'a>(
        pipelines: &'a GpuPipelines,
        renderer: &'a Self,
        pass: &mut RenderPass<'a>,
    ) {
        record_draw(
            pass,
            &pipelines.bg_pipeline,
            renderer.uniform_buffer.bind_group(),
            None,
            renderer.cursor_buffer.as_ref(),
            renderer.prepared.cursors.len() as u32,
        );
    }

    /// Record per-image draw calls for a set of image quads.
    ///
    /// Each image requires its own draw call because each has a unique
    /// texture bind group. `buffer_offset` is the starting index into the
    /// shared image instance buffer.
    fn record_image_draws<'a>(
        pipelines: &'a GpuPipelines,
        renderer: &'a Self,
        pass: &mut RenderPass<'a>,
        quads: &[super::super::prepared_frame::ImageQuad],
        buffer_offset: usize,
    ) {
        if quads.is_empty() {
            return;
        }
        let Some(buf) = renderer.image_instance_buffer.as_ref() else {
            return;
        };

        let stride = IMAGE_INSTANCE_STRIDE;

        pass.set_pipeline(&pipelines.image_pipeline);
        pass.set_bind_group(0, renderer.uniform_buffer.bind_group(), &[]);

        for (i, quad) in quads.iter().enumerate() {
            let Some(bind_group) = renderer.image_texture_cache.get_bind_group(quad.image_id)
            else {
                continue;
            };

            let byte_offset = ((buffer_offset + i) as u64) * stride;
            pass.set_bind_group(1, bind_group, &[]);
            pass.set_vertex_buffer(0, buf.slice(byte_offset..byte_offset + stride));
            pass.draw(0..4, 0..1);
        }
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
