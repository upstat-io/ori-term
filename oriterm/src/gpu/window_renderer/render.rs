//! GPU render phase: upload instance buffers, record draw passes, submit.
//!
//! Two render paths:
//! - **Full render** ([`render_frame`]): all draw calls to a single target.
//!   Used for offscreen rendering (tab previews, visual regression tests).
//! - **Cached render** ([`render_to_surface`]): splits content from cursor.
//!   On content-change frames, everything except the cursor is rendered to an
//!   offscreen cache texture. On every frame (including cursor-blink-only),
//!   the cache is copied to the surface and only the cursor is drawn on top.
//!   This avoids the full GPU submission on idle blink frames. UI-only dialog
//!   windows bypass the cache and render straight to the surface.

use std::time::Instant;

#[cfg(all(test, feature = "gpu-tests"))]
use wgpu::TextureView;
use wgpu::{
    Color, CommandEncoderDescriptor, Extent3d, LoadOp, Operations, RenderPassColorAttachment,
    RenderPassDescriptor, StoreOp, TextureViewDescriptor,
};

use super::super::pipeline::IMAGE_INSTANCE_STRIDE;
use super::super::state::GpuState;
use super::helpers::{record_draw, record_draw_range, upload_buffer};
use super::{SurfaceError, WindowRenderer};
use crate::gpu::pipelines::GpuPipelines;

impl WindowRenderer {
    /// Upload the stored prepared frame to the GPU and execute draw calls.
    ///
    /// Reads from `self.prepared` (filled by [`prepare`](Self::prepare)).
    /// Accepts any `TextureView` as target — works for both surfaces and
    /// offscreen render targets (tab previews, headless testing).
    #[cfg(all(test, feature = "gpu-tests"))]
    pub fn render_frame(&mut self, gpu: &GpuState, pipelines: &GpuPipelines, target: &TextureView) {
        let device = &gpu.device;
        let queue = &gpu.queue;
        let vp = self.prepared.viewport;

        // Rebuild atlas bind groups if any atlas texture grew since last render.
        self.rebuild_stale_atlas_bind_groups(device, &pipelines.atlas_layout);

        // Update screen_size uniform.
        self.uniform_buffer
            .write_screen_size(queue, vp.width as f32, vp.height as f32);

        // Upload instance data to GPU buffers.
        self.upload_instance_buffers(device, queue);

        // Upload image instance data (shared buffer for all image quads).
        self.upload_image_instances(device, queue);

        // Encode render commands.
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("frame_encoder"),
        });

        let clear = self.clear_color();

        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("terminal_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(clear),
                        store: StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                ..Default::default()
            });

            Self::record_cached_content_passes(pipelines, self, &mut pass);
            Self::record_overlay_pass(pipelines, self, &mut pass);
            Self::record_cursor_pass(pipelines, self, &mut pass);
        }

        queue.submit(std::iter::once(encoder.finish()));

        // Debug: log draw call count per frame.
        log::debug!("frame draw calls: {}", self.prepared.count_draw_calls());
    }

    /// Acquire a surface texture, render with content caching, and present.
    ///
    /// Terminal windows use the cached path: when `content_changed` is `true`,
    /// all non-cursor content is rendered to an offscreen cache texture, then
    /// copied to the surface before drawing overlays/cursor. When `false`
    /// (cursor-blink-only), the cached texture is reused.
    ///
    /// UI-only dialog windows skip the offscreen cache entirely and render
    /// directly to the surface every frame.
    ///
    /// Handles surface errors: `Lost`/`Outdated` → caller should reconfigure,
    /// `OutOfMemory` → propagated, `Timeout` → propagated.
    pub fn render_to_surface(
        &mut self,
        gpu: &GpuState,
        pipelines: &GpuPipelines,
        surface: &wgpu::Surface<'_>,
        content_changed: bool,
    ) -> Result<(), SurfaceError> {
        let output = surface.get_current_texture().map_err(|e| match e {
            wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated => SurfaceError::Lost,
            wgpu::SurfaceError::OutOfMemory => SurfaceError::OutOfMemory,
            wgpu::SurfaceError::Timeout => SurfaceError::Timeout,
            wgpu::SurfaceError::Other => SurfaceError::Other,
        })?;

        let device = &gpu.device;
        let queue = &gpu.queue;

        // Rebuild atlas bind groups if any atlas texture grew since last render.
        self.rebuild_stale_atlas_bind_groups(device, &pipelines.atlas_layout);

        // Update screen_size uniform.
        let vp = self.prepared.viewport;
        self.uniform_buffer
            .write_screen_size(queue, vp.width as f32, vp.height as f32);

        if self.is_ui_only() {
            self.render_ui_only(gpu, pipelines, &output);
        } else {
            self.render_cached(gpu, pipelines, &output, content_changed);
        }

        output.present();
        log::debug!("frame draw calls: {}", self.prepared.count_draw_calls());
        Ok(())
    }

    /// UI-only path: render everything directly to the surface in a single pass.
    fn render_ui_only(
        &mut self,
        gpu: &GpuState,
        pipelines: &GpuPipelines,
        output: &wgpu::SurfaceTexture,
    ) {
        let device = &gpu.device;
        let queue = &gpu.queue;
        self.upload_instance_buffers(device, queue);
        self.upload_image_instances(device, queue);

        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("ui_only_frame_encoder"),
        });
        let surface_view = output.texture.create_view(&TextureViewDescriptor {
            format: Some(gpu.render_format()),
            ..Default::default()
        });
        let clear = self.clear_color();
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("ui_only_surface_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(clear),
                        store: StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                ..Default::default()
            });
            Self::record_cached_content_passes(pipelines, self, &mut pass);
            Self::record_overlay_pass(pipelines, self, &mut pass);
            Self::record_cursor_pass(pipelines, self, &mut pass);
        }
        queue.submit(std::iter::once(encoder.finish()));
    }

    /// Cached path: render content to an offscreen texture, copy to surface,
    /// then draw overlays and cursor on top.
    fn render_cached(
        &mut self,
        gpu: &GpuState,
        pipelines: &GpuPipelines,
        output: &wgpu::SurfaceTexture,
        content_changed: bool,
    ) {
        let vp = self.prepared.viewport;
        let device = &gpu.device;
        let queue = &gpu.queue;

        // Ensure the content cache texture exists and matches the viewport.
        self.ensure_content_cache(device, vp.width, vp.height, gpu);

        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("frame_encoder"),
        });

        if content_changed {
            // Upload all instance data for the content render.
            self.upload_instance_buffers(device, queue);
            self.upload_image_instances(device, queue);

            // Render everything except cursor to the content cache texture.
            let cache_view = self
                .content_cache_view
                .as_ref()
                .expect("cache just ensured");
            let clear = self.clear_color();
            {
                let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("content_cache_pass"),
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: cache_view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(clear),
                            store: StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    ..Default::default()
                });
                Self::record_cached_content_passes(pipelines, self, &mut pass);
            }
        } else {
            // Cached content is still valid: only transient overlay/cursor
            // tiers need fresh buffers this frame.
            self.upload_overlay_and_cursor_buffers(device, queue);
        }

        // Copy cached content to surface texture.
        let cache_tex = self.content_cache.as_ref().expect("cache ensured");
        encoder.copy_texture_to_texture(
            cache_tex.as_image_copy(),
            output.texture.as_image_copy(),
            Extent3d {
                width: vp.width,
                height: vp.height,
                depth_or_array_layers: 1,
            },
        );

        // Draw overlays and cursor on top of the copied content.
        let surface_view = output.texture.create_view(&TextureViewDescriptor {
            format: Some(gpu.render_format()),
            ..Default::default()
        });
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("overlay_cursor_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                ..Default::default()
            });
            Self::record_overlay_pass(pipelines, self, &mut pass);
            Self::record_cursor_pass(pipelines, self, &mut pass);
        }

        queue.submit(std::iter::once(encoder.finish()));
    }

    // ── Content cache management ──

    /// Ensure the offscreen content cache texture matches the viewport size.
    fn ensure_content_cache(&mut self, device: &wgpu::Device, w: u32, h: u32, gpu: &GpuState) {
        if self.content_cache_size == (w, h) && self.content_cache.is_some() {
            return;
        }

        // Drop old cache first so RSS delta reflects net allocation.
        let rss_before = crate::platform::memory::rss_bytes();
        self.content_cache = None;
        self.content_cache_view = None;

        let surface_format = gpu.surface_format();
        let render_format = gpu.render_format();
        let extra_formats = [render_format];
        let view_formats = if render_format == surface_format {
            &[]
        } else {
            extra_formats.as_slice()
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("content_cache"),
            size: Extent3d {
                width: w.max(1),
                height: h.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: surface_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats,
        });
        let view = texture.create_view(&TextureViewDescriptor {
            format: Some(render_format),
            ..Default::default()
        });

        self.content_cache = Some(texture);
        self.content_cache_view = Some(view);
        self.content_cache_size = (w, h);

        let theoretical_bytes = w.max(1) as usize * h.max(1) as usize * 4;
        let theoretical_mb = theoretical_bytes as f64 / 1_048_576.0;
        log::debug!("content cache texture created: {w}x{h} ({theoretical_mb:.1} MB theoretical)");

        // Measure RSS impact of the GPU texture allocation.
        // GPU-only textures (RENDER_ATTACHMENT | COPY_SRC) typically live in
        // VRAM and should not affect process RSS. On shared-memory GPUs
        // (integrated, llvmpipe) they may be mapped into process address space.
        if let (Some(before), Some(after)) = (rss_before, crate::platform::memory::rss_bytes()) {
            let delta = after as isize - before as isize;
            let delta_mb = delta as f64 / 1_048_576.0;
            log::info!(
                "content cache: {w}x{h} = {theoretical_mb:.1} MB theoretical, \
                 RSS delta: {delta_mb:+.1} MB ({} -> {})",
                crate::platform::memory::format_bytes(before),
                crate::platform::memory::format_bytes(after),
            );
        }
    }

    // ── Buffer uploads ──

    /// Upload all instance buffers to the GPU.
    ///
    /// Logs total bytes and wall time at `debug!` level for performance
    /// profiling (Section 23.4).
    fn upload_instance_buffers(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
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
    fn upload_overlay_and_cursor_buffers(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
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
    fn upload_image_instances(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
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

    // ── Draw pass recording ──

    /// Record cached-content draw passes.
    ///
    /// Two tiers in painter's order:
    /// - Terminal: cell backgrounds, images below text, glyphs, images above
    /// - Chrome: UI rects + chrome text (tab bar, search bar)
    fn record_cached_content_passes<'a>(
        pipelines: &'a GpuPipelines,
        renderer: &'a Self,
        pass: &mut wgpu::RenderPass<'a>,
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
    fn record_overlay_pass<'a>(
        pipelines: &'a GpuPipelines,
        renderer: &'a Self,
        pass: &mut wgpu::RenderPass<'a>,
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
    fn record_cursor_pass<'a>(
        pipelines: &'a GpuPipelines,
        renderer: &'a Self,
        pass: &mut wgpu::RenderPass<'a>,
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
        pass: &mut wgpu::RenderPass<'a>,
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
    fn clear_color(&self) -> Color {
        Color {
            r: self.prepared.clear_color[0],
            g: self.prepared.clear_color[1],
            b: self.prepared.clear_color[2],
            a: self.prepared.clear_color[3],
        }
    }
}
