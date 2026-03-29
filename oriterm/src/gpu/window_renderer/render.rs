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

#[cfg(all(test, feature = "gpu-tests"))]
use wgpu::TextureView;
use wgpu::{
    CommandEncoderDescriptor, Extent3d, LoadOp, Operations, RenderPassColorAttachment,
    RenderPassDescriptor, StoreOp, TextureViewDescriptor,
};

use super::super::state::GpuState;
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
}
