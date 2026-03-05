//! GPU render phase: upload instance buffers, record draw passes, submit.

use wgpu::{
    Color, CommandEncoderDescriptor, LoadOp, Operations, RenderPassColorAttachment,
    RenderPassDescriptor, StoreOp, TextureView, TextureViewDescriptor,
};

use super::super::state::GpuState;
use super::helpers::{record_draw, upload_buffer};
use super::{SurfaceError, WindowRenderer};
use crate::gpu::pipelines::GpuPipelines;

impl WindowRenderer {
    /// Upload the stored prepared frame to the GPU and execute draw calls.
    ///
    /// Reads from `self.prepared` (filled by [`prepare`](Self::prepare)).
    /// Accepts any `TextureView` as target — works for both surfaces and
    /// offscreen render targets (tab previews, headless testing).
    pub fn render_frame(&mut self, gpu: &GpuState, pipelines: &GpuPipelines, target: &TextureView) {
        let device = &gpu.device;
        let queue = &gpu.queue;
        let vp = self.prepared.viewport;

        // Update screen_size uniform.
        self.uniform_buffer
            .write_screen_size(queue, vp.width as f32, vp.height as f32);

        // Upload instance data to GPU buffers.
        self.upload_instance_buffers(device, queue);

        // Encode render commands.
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("frame_encoder"),
        });

        let clear = Color {
            r: self.prepared.clear_color[0],
            g: self.prepared.clear_color[1],
            b: self.prepared.clear_color[2],
            a: self.prepared.clear_color[3],
        };

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

            Self::record_draw_passes(pipelines, self, &mut pass);
        }

        queue.submit(std::iter::once(encoder.finish()));
    }

    /// Upload all instance buffers to the GPU.
    fn upload_instance_buffers(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        upload_buffer(
            device,
            queue,
            &mut self.bg_buffer,
            self.prepared.backgrounds.as_bytes(),
            "bg_instance_buffer",
        );
        upload_buffer(
            device,
            queue,
            &mut self.fg_buffer,
            self.prepared.glyphs.as_bytes(),
            "fg_instance_buffer",
        );
        upload_buffer(
            device,
            queue,
            &mut self.subpixel_fg_buffer,
            self.prepared.subpixel_glyphs.as_bytes(),
            "subpixel_fg_instance_buffer",
        );
        upload_buffer(
            device,
            queue,
            &mut self.color_fg_buffer,
            self.prepared.color_glyphs.as_bytes(),
            "color_fg_instance_buffer",
        );
        upload_buffer(
            device,
            queue,
            &mut self.cursor_buffer,
            self.prepared.cursors.as_bytes(),
            "cursor_instance_buffer",
        );
        upload_buffer(
            device,
            queue,
            &mut self.ui_rect_buffer,
            self.prepared.ui_rects.as_bytes(),
            "ui_rect_instance_buffer",
        );
        upload_buffer(
            device,
            queue,
            &mut self.ui_fg_buffer,
            self.prepared.ui_glyphs.as_bytes(),
            "ui_fg_instance_buffer",
        );
        upload_buffer(
            device,
            queue,
            &mut self.ui_subpixel_fg_buffer,
            self.prepared.ui_subpixel_glyphs.as_bytes(),
            "ui_subpixel_fg_instance_buffer",
        );
        upload_buffer(
            device,
            queue,
            &mut self.ui_color_fg_buffer,
            self.prepared.ui_color_glyphs.as_bytes(),
            "ui_color_fg_instance_buffer",
        );
        upload_buffer(
            device,
            queue,
            &mut self.overlay_rect_buffer,
            self.prepared.overlay_rects.as_bytes(),
            "overlay_rect_instance_buffer",
        );
        upload_buffer(
            device,
            queue,
            &mut self.overlay_fg_buffer,
            self.prepared.overlay_glyphs.as_bytes(),
            "overlay_fg_instance_buffer",
        );
        upload_buffer(
            device,
            queue,
            &mut self.overlay_subpixel_fg_buffer,
            self.prepared.overlay_subpixel_glyphs.as_bytes(),
            "overlay_subpixel_fg_instance_buffer",
        );
        upload_buffer(
            device,
            queue,
            &mut self.overlay_color_fg_buffer,
            self.prepared.overlay_color_glyphs.as_bytes(),
            "overlay_color_fg_instance_buffer",
        );
    }

    /// Record the thirteen draw passes into the render pass.
    ///
    /// Three tiers in painter's order:
    /// - Terminal (draws 1–5): cell backgrounds, glyphs, cursors
    /// - Chrome (draws 6–9): UI rects + chrome text (tab bar, search bar)
    /// - Overlay (draws 10–13): overlay rects + overlay text (context menus)
    #[expect(
        clippy::too_many_lines,
        reason = "GPU draw dispatch table: 13 sequential record_draw calls across 3 tiers"
    )]
    fn record_draw_passes<'a>(
        pipelines: &'a GpuPipelines,
        renderer: &'a Self,
        pass: &mut wgpu::RenderPass<'a>,
    ) {
        let bg = renderer.uniform_buffer.bind_group();
        let mono = Some(renderer.atlas_bind_group.bind_group());
        let sub = Some(renderer.subpixel_atlas_bind_group.bind_group());
        let color = Some(renderer.color_atlas_bind_group.bind_group());
        let p = &renderer.prepared;

        // Terminal tier (draws 1–5).
        record_draw(
            pass,
            &pipelines.bg_pipeline,
            bg,
            None,
            renderer.bg_buffer.as_ref(),
            p.backgrounds.len() as u32,
        );
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
        record_draw(
            pass,
            &pipelines.bg_pipeline,
            bg,
            None,
            renderer.cursor_buffer.as_ref(),
            p.cursors.len() as u32,
        );

        // Chrome tier (draws 6–9).
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

        // Overlay tier (draws 10–13).
        record_draw(
            pass,
            &pipelines.ui_rect_pipeline,
            bg,
            None,
            renderer.overlay_rect_buffer.as_ref(),
            p.overlay_rects.len() as u32,
        );
        record_draw(
            pass,
            &pipelines.fg_pipeline,
            bg,
            mono,
            renderer.overlay_fg_buffer.as_ref(),
            p.overlay_glyphs.len() as u32,
        );
        record_draw(
            pass,
            &pipelines.subpixel_fg_pipeline,
            bg,
            sub,
            renderer.overlay_subpixel_fg_buffer.as_ref(),
            p.overlay_subpixel_glyphs.len() as u32,
        );
        record_draw(
            pass,
            &pipelines.color_fg_pipeline,
            bg,
            color,
            renderer.overlay_color_fg_buffer.as_ref(),
            p.overlay_color_glyphs.len() as u32,
        );
    }

    /// Acquire a surface texture, render the stored prepared frame, and present.
    ///
    /// Handles surface errors: `Lost`/`Outdated` → caller should reconfigure,
    /// `OutOfMemory` → propagated, `Timeout` → propagated.
    pub fn render_to_surface(
        &mut self,
        gpu: &GpuState,
        pipelines: &GpuPipelines,
        surface: &wgpu::Surface<'_>,
    ) -> Result<(), SurfaceError> {
        let output = surface.get_current_texture().map_err(|e| match e {
            wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated => SurfaceError::Lost,
            wgpu::SurfaceError::OutOfMemory => SurfaceError::OutOfMemory,
            wgpu::SurfaceError::Timeout => SurfaceError::Timeout,
            wgpu::SurfaceError::Other => SurfaceError::Other,
        })?;

        let view = output.texture.create_view(&TextureViewDescriptor {
            format: Some(gpu.render_format()),
            ..Default::default()
        });

        self.render_frame(gpu, pipelines, &view);
        output.present();
        Ok(())
    }
}
