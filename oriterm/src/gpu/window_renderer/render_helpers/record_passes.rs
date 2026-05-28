//! Draw-pass recording helpers on [`WindowRenderer`].
//!
//! Extracted from `render_helpers/mod.rs` to keep each file under the
//! 500-line limit. These methods record cached-content, overlay, cursor,
//! and per-image draw calls into a `wgpu::RenderPass`.

use wgpu::RenderPass;

use super::super::super::pipeline::IMAGE_INSTANCE_STRIDE;
use super::super::WindowRenderer;
use super::super::helpers::{DrawResources, record_draw, record_draw_range};
use crate::gpu::pipelines::GpuPipelines;

impl WindowRenderer {
    /// Record cached-content draw passes.
    ///
    /// Two tiers in painter's order:
    /// - Terminal: cell backgrounds, images below text, glyphs, images above
    /// - Chrome: UI rects + chrome text (tab bar, search bar)
    pub(in crate::gpu::window_renderer) fn record_cached_content_passes<'a>(
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
            DrawResources {
                pipeline: &pipelines.bg_pipeline,
                uniform_bg: bg,
                atlas_bg: None,
                buffer: renderer.bg_buffer.as_ref(),
            },
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
            DrawResources {
                pipeline: &pipelines.fg_pipeline,
                uniform_bg: bg,
                atlas_bg: mono,
                buffer: renderer.fg_buffer.as_ref(),
            },
            p.glyphs.len() as u32,
        );
        record_draw(
            pass,
            DrawResources {
                pipeline: &pipelines.subpixel_fg_pipeline,
                uniform_bg: bg,
                atlas_bg: sub,
                buffer: renderer.subpixel_fg_buffer.as_ref(),
            },
            p.subpixel_glyphs.len() as u32,
        );
        record_draw(
            pass,
            DrawResources {
                pipeline: &pipelines.color_fg_pipeline,
                uniform_bg: bg,
                atlas_bg: color,
                buffer: renderer.color_fg_buffer.as_ref(),
            },
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
            DrawResources {
                pipeline: &pipelines.ui_rect_pipeline,
                uniform_bg: bg,
                atlas_bg: None,
                buffer: renderer.ui_rect_buffer.as_ref(),
            },
            p.ui_rects.len() as u32,
        );
        record_draw(
            pass,
            DrawResources {
                pipeline: &pipelines.fg_pipeline,
                uniform_bg: bg,
                atlas_bg: mono,
                buffer: renderer.ui_fg_buffer.as_ref(),
            },
            p.ui_glyphs.len() as u32,
        );
        record_draw(
            pass,
            DrawResources {
                pipeline: &pipelines.subpixel_fg_pipeline,
                uniform_bg: bg,
                atlas_bg: sub,
                buffer: renderer.ui_subpixel_fg_buffer.as_ref(),
            },
            p.ui_subpixel_glyphs.len() as u32,
        );
        record_draw(
            pass,
            DrawResources {
                pipeline: &pipelines.color_fg_pipeline,
                uniform_bg: bg,
                atlas_bg: color,
                buffer: renderer.ui_color_fg_buffer.as_ref(),
            },
            p.ui_color_glyphs.len() as u32,
        );
    }

    /// Record overlay draw passes only.
    pub(in crate::gpu::window_renderer) fn record_overlay_pass<'a>(
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
                DrawResources {
                    pipeline: &pipelines.ui_rect_pipeline,
                    uniform_bg: bg,
                    atlas_bg: None,
                    buffer: renderer.overlay_rect_buffer.as_ref(),
                },
                range.rects.0,
                range.rects.1,
            );
            record_draw_range(
                pass,
                DrawResources {
                    pipeline: &pipelines.fg_pipeline,
                    uniform_bg: bg,
                    atlas_bg: mono,
                    buffer: renderer.overlay_fg_buffer.as_ref(),
                },
                range.mono.0,
                range.mono.1,
            );
            record_draw_range(
                pass,
                DrawResources {
                    pipeline: &pipelines.subpixel_fg_pipeline,
                    uniform_bg: bg,
                    atlas_bg: sub,
                    buffer: renderer.overlay_subpixel_fg_buffer.as_ref(),
                },
                range.subpixel.0,
                range.subpixel.1,
            );
            record_draw_range(
                pass,
                DrawResources {
                    pipeline: &pipelines.color_fg_pipeline,
                    uniform_bg: bg,
                    atlas_bg: color,
                    buffer: renderer.overlay_color_fg_buffer.as_ref(),
                },
                range.color.0,
                range.color.1,
            );
        }
    }

    /// Record cursor draw pass only.
    pub(in crate::gpu::window_renderer) fn record_cursor_pass<'a>(
        pipelines: &'a GpuPipelines,
        renderer: &'a Self,
        pass: &mut RenderPass<'a>,
    ) {
        record_draw(
            pass,
            DrawResources {
                pipeline: &pipelines.bg_pipeline,
                uniform_bg: renderer.uniform_buffer.bind_group(),
                atlas_bg: None,
                buffer: renderer.cursor_buffer.as_ref(),
            },
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
        quads: &[super::super::super::prepared_frame::ImageQuad],
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
}
