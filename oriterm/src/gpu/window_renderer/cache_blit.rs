//! Cache-to-output blit: copies the offscreen content cache to a destination
//! texture, conditionally pre-clearing the destination when it exceeds the
//! prepared viewport so the region outside the cache extent is initialized
//! to `clear_color()` rather than undefined GPU memory.
//!
//! See: bug-tracker/plans/completed/BUG-06-052/ — resize-grow uncovered-region pin.
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
}
