//! Cache-to-output blit: copies the offscreen content cache to a destination
//! texture, conditionally pre-clearing the destination when it exceeds the
//! prepared viewport so the region outside the cache extent is initialized
//! to `clear_color()` rather than undefined GPU memory.
//!
//! Shared by `render_cached` (production swapchain path) and
//! `render_frame_cached` (test path). The signature uses only public wgpu
//! types so the helper has no `#[cfg(test)]` gating — the gate belongs to
//! the test-only caller, not the helper.

use wgpu::{
    CommandEncoder, Extent3d, LoadOp, Operations, RenderPassColorAttachment, RenderPassDescriptor,
    StoreOp, Texture, TextureFormat, TextureViewDescriptor,
};

use super::WindowRenderer;

impl WindowRenderer {
    /// Copy the offscreen content cache to `output_texture`, pre-clearing
    /// any region of the destination outside the prepared viewport.
    ///
    /// `output_texture` accepts both the swapchain's `SurfaceTexture.texture`
    /// (production `render_cached` path) and an offscreen `RenderTarget`'s
    /// underlying texture (test `render_frame_cached` path) — the helper
    /// uses only the `wgpu::Texture` public surface to keep both call sites
    /// behind a single SSOT.
    ///
    /// When the destination exceeds the prepared viewport on either axis
    /// (resize-grow path), open a no-draw render pass with
    /// `LoadOp::Clear(self.clear_color())` to initialize the full
    /// destination, then copy the upper-left `min(vp, dst)` sub-rect. When
    /// `dst == vp` (common path) and on shrink paths, skip the clear; the
    /// copy alone covers every destination texel that needs the cache.
    pub(super) fn copy_cache_to_output(
        &self,
        encoder: &mut CommandEncoder,
        output_texture: &Texture,
        render_format: TextureFormat,
    ) {
        let vp = self.prepared.viewport;
        let cache_tex = self
            .content_cache
            .as_ref()
            .expect("content cache ensured before blit");
        let dst = output_texture.size();

        // Zero-extent guard: minimized windows reconfigure the swapchain to
        // 0×0; a 0-extent render pass or `copy_texture_to_texture` raises a
        // wgpu validation panic. Skip the whole blit — there is no surface
        // to paint anyway. Symmetric guard against `vp = (0,0)` (degenerate
        // prepare-time viewport) is enforced below the clear pass via the
        // `copy_w == 0 || copy_h == 0` check.
        if dst.width == 0 || dst.height == 0 {
            return;
        }

        if dst.width > vp.width || dst.height > vp.height {
            let view = output_texture.create_view(&TextureViewDescriptor {
                format: Some(render_format),
                ..Default::default()
            });
            // `_clear_pass` borrows `view`. Drop order: `_clear_pass` drops
            // first (records the clear + store) before `view`. Safe — the
            // pass commits its work to the encoder before the borrow ends.
            let _clear_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("cache_blit_pre_clear"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(self.clear_color()),
                        store: StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                ..Default::default()
            });
        }

        let copy_w = vp.width.min(dst.width);
        let copy_h = vp.height.min(dst.height);

        // Skip the copy when either axis is zero — happens when
        // `prepared.viewport` is (0,0) (degenerate prepare-time state, e.g.
        // very early in window setup). The destination has already been
        // cleared above when `dst > vp`; nothing remains to copy.
        if copy_w == 0 || copy_h == 0 {
            return;
        }

        encoder.copy_texture_to_texture(
            cache_tex.as_image_copy(),
            output_texture.as_image_copy(),
            Extent3d {
                width: copy_w,
                height: copy_h,
                depth_or_array_layers: 1,
            },
        );
    }

    /// Test-only diagnostic (§13.6.1 item 5): run ONLY the
    /// `copy_cache_to_output` blit (Stage B). No image-draws, no render
    /// passes, no overlay pass.
    ///
    /// Pre-requirement: `ensure_content_cache` must have run once (drive
    /// a normal `render_frame_cached` first) so `self.content_cache` is
    /// allocated. Returns `None` if the cache has not been allocated.
    ///
    /// Creates a fresh `COPY_DST` render target of `(target_width,
    /// target_height)`, encodes a single command buffer containing only
    /// the `copy_cache_to_output` call, submits, and returns the target.
    /// The caller drives `queue.write_texture` against the cache texture
    /// BEFORE invoking this method to pre-load a known pixel region; the
    /// returned target is read back via `gpu.read_render_target` to prove
    /// the copy preserves the pre-loaded pixels.
    #[cfg(all(test, feature = "gpu-tests"))]
    pub(crate) fn copy_cache_to_output_for_test(
        &mut self,
        gpu: &super::super::state::GpuState,
        target_width: u32,
        target_height: u32,
    ) -> Option<crate::gpu::render_target::RenderTarget> {
        self.content_cache.as_ref()?;
        let device = gpu.device_for_test();
        let queue = gpu.queue_for_test();
        let output = gpu.create_copy_dst_target(target_width, target_height);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("copy_cache_to_output_for_test_encoder"),
        });
        self.copy_cache_to_output(&mut encoder, output.texture(), gpu.render_format());
        queue.submit(std::iter::once(encoder.finish()));
        Some(output)
    }
}
