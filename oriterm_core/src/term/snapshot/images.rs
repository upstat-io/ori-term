//! Image placement + pixel-data extraction for the rendering snapshot.
//!
//! Split from `snapshot/mod.rs` to keep both files under the 500-line limit.
//! Owns the viewport image-placement walk and the decoded-RGBA collection;
//! the cell/palette/damage extraction stays in `snapshot/mod.rs`.

use std::collections::HashSet;

use crate::effect::sink::EffectSink;
use crate::image::ImageId;
use crate::term::Term;
use crate::term::renderable::{RenderableContent, RenderableImageData, RenderablePlacement};

/// Read-only inputs for [`Term::extract_images`].
#[derive(Clone, Copy)]
struct ImageExtractCtx<'a> {
    /// Image cache to read placements + pixel data from.
    cache: &'a crate::image::ImageCache,
    /// Stable row index of the viewport's top row.
    stable_row_base: u64,
    /// Number of viewport lines visible.
    viewport_lines: usize,
    /// Cell pixel width.
    cell_w: u16,
    /// Cell pixel height.
    cell_h: u16,
}

/// Mutable output sinks for [`Term::extract_images`].
struct ImageExtractSink<'a> {
    /// Collected viewport placements.
    images: &'a mut Vec<RenderablePlacement>,
    /// Decoded RGBA data for referenced images.
    image_data: &'a mut Vec<RenderableImageData>,
    /// Set of image ids referenced this frame.
    seen_ids: &'a mut HashSet<ImageId>,
}

impl<S: EffectSink> Term<S> {
    /// Extract image placements visible in the viewport and propagate dirty.
    pub(super) fn fill_image_snapshot(&self, out: &mut RenderableContent) {
        Self::extract_images(
            ImageExtractCtx {
                cache: self.image_cache(),
                stable_row_base: out.stable_row_base,
                viewport_lines: out.lines,
                cell_w: self.cell_pixel_width,
                cell_h: self.cell_pixel_height,
            },
            ImageExtractSink {
                images: &mut out.images,
                image_data: &mut out.image_data,
                seen_ids: &mut out.seen_image_ids,
            },
        );

        // Propagate image dirty flag. When images changed, force a full
        // viewport repaint since image mutations don't set per-line grid
        // dirty flags. The dirty flag is cleared by `reset_damage()`.
        out.images_dirty = self.image_cache().is_dirty();
        if out.images_dirty {
            out.all_dirty = true;
        }
    }

    /// Extract visible image placements and their pixel data.
    ///
    /// Converts `ImagePlacement` cell coordinates to viewport pixel positions
    /// and collects the decoded RGBA data for GPU texture upload.
    fn extract_images(ctx: ImageExtractCtx<'_>, sink: ImageExtractSink<'_>) {
        let ImageExtractCtx {
            cache,
            stable_row_base,
            viewport_lines,
            cell_w,
            cell_h,
        } = ctx;
        let ImageExtractSink {
            images,
            image_data,
            seen_ids,
        } = sink;
        images.clear();
        image_data.clear();
        seen_ids.clear();

        if cache.placement_count() == 0 {
            return;
        }

        let top = crate::grid::StableRowIndex(stable_row_base);
        let bottom =
            crate::grid::StableRowIndex(stable_row_base + viewport_lines.saturating_sub(1) as u64);

        let cw = f32::from(cell_w);
        let ch = f32::from(cell_h);

        for p in cache.viewport_placements(top, bottom) {
            // Skip placements whose image data is no longer cached.
            let Some(img) = cache.get_no_touch(p.image_id) else {
                continue;
            };

            // Signed offset: images starting above the viewport have negative Y,
            // so their visible bottom portion renders correctly. The GPU clips
            // fragments outside the framebuffer (implicit viewport scissor).
            let row_offset = p.cell_row.0 as i64 - stable_row_base as i64;
            let vp_x = p.cell_col as f32 * cw + f32::from(p.cell_x_offset);
            let vp_y = row_offset as f32 * ch + f32::from(p.cell_y_offset);

            let (disp_w, disp_h) = match p.sizing {
                crate::image::PlacementSizing::CellCount => {
                    (p.cols as f32 * cw, p.rows as f32 * ch)
                }
                crate::image::PlacementSizing::FixedPixels { width, height } => {
                    (width as f32, height as f32)
                }
            };

            let (src_x, src_y, src_w, src_h) = uv_source_rect(p, img);

            images.push(RenderablePlacement {
                image_id: p.image_id,
                viewport_x: vp_x,
                viewport_y: vp_y,
                display_width: disp_w,
                display_height: disp_h,
                source_x: src_x,
                source_y: src_y,
                source_w: src_w,
                source_h: src_h,
                z_index: p.z_index,
                opacity: 1.0,
            });

            seen_ids.insert(p.image_id);
        }

        // Collect pixel data for referenced images.
        for &id in seen_ids.iter() {
            if let Some(img) = cache.get_no_touch(id) {
                image_data.push(cached_image_data(id, img));
            }
        }
    }

    /// Append image data for any placeholder-cell `image_id` not already
    /// covered by [`extract_images`]. Called after the cell walk so the
    /// GPU can sample the texture for kitty unicode-placeholder cells.
    pub(super) fn fill_placeholder_image_data(&self, out: &mut RenderableContent) {
        let cache = self.image_cache();
        for pc in &out.placeholder_cells {
            if out.seen_image_ids.contains(&pc.image_id) {
                continue;
            }
            if let Some(img) = cache.get_no_touch(pc.image_id) {
                out.image_data.push(cached_image_data(pc.image_id, img));
                out.seen_image_ids.insert(pc.image_id);
            }
        }
    }
}

/// Normalized UV source rect (0..1) for a placement within its image.
///
/// Falls back to the full image (`0, 0, 1, 1`) when the image reports zero
/// dimensions, matching the GPU sampler's whole-texture default.
fn uv_source_rect(
    p: &crate::image::ImagePlacement,
    img: &crate::image::ImageData,
) -> (f32, f32, f32, f32) {
    let iw = img.width as f32;
    let ih = img.height as f32;
    if iw <= 0.0 || ih <= 0.0 {
        return (0.0, 0.0, 1.0, 1.0);
    }
    let sx = p.source_x as f32 / iw;
    let sy = p.source_y as f32 / ih;
    let sw = if p.source_w > 0 {
        p.source_w as f32 / iw
    } else {
        1.0 - sx
    };
    let sh = if p.source_h > 0 {
        p.source_h as f32 / ih
    } else {
        1.0 - sy
    };
    (sx, sy, sw, sh)
}

/// Build a [`RenderableImageData`] snapshot entry from a cached image.
///
/// Single construction site for the cache-image → snapshot-entry mapping,
/// shared by [`Term::extract_images`] and [`Term::fill_placeholder_image_data`].
fn cached_image_data(id: ImageId, img: &crate::image::ImageData) -> RenderableImageData {
    RenderableImageData {
        id,
        data: img.data.clone(),
        width: img.width,
        height: img.height,
        pixel_generation: img.pixel_generation,
    }
}
