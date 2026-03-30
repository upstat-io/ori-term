//! GPU texture creation and glyph upload for the glyph atlas.

use wgpu::{
    Device, Extent3d, Queue, Texture, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension,
};

use crate::font::RasterizedGlyph;

/// Create a texture array with the given number of layers.
pub(super) fn create_texture_array(
    device: &Device,
    size: u32,
    layers: u32,
    format: TextureFormat,
) -> (Texture, TextureView) {
    let label = match format {
        TextureFormat::R8Unorm => "glyph_atlas_array",
        _ => "rgba_glyph_atlas_array",
    };
    let texture = device.create_texture(&TextureDescriptor {
        label: Some(label),
        size: Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: layers,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format,
        // COPY_SRC needed for grow-on-demand: existing layers are copied
        // to a new larger texture when the atlas grows.
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::COPY_SRC,
        view_formats: &[],
    });

    let view = texture.create_view(&TextureViewDescriptor {
        dimension: Some(TextureViewDimension::D2Array),
        ..Default::default()
    });

    (texture, view)
}

/// Upload a glyph bitmap to a position on a texture array layer, zeroing
/// the `GLYPH_PADDING` gutter on the right and bottom edges.
///
/// The allocator reserves `w + padding` × `h + padding` for each glyph.
/// This function writes the glyph body, then zeros the right strip
/// `(x + w, y)` → `(x + w + padding, y + h)` and the bottom strip
/// `(x, y + h)` → `(x + w + padding, y + h + padding)`.  This prevents
/// the bilinear sampler from interpolating stale texels into glyph edges.
///
/// `padding_zeros` is a pre-allocated zero buffer sliced for each strip.
#[expect(
    clippy::too_many_arguments,
    reason = "GPU texture upload: resource refs, destination coords, glyph data, padding"
)]
pub(super) fn upload_glyph(
    queue: &Queue,
    texture: &Texture,
    page_idx: u32,
    x: u32,
    y: u32,
    glyph: &RasterizedGlyph,
    padding: u32,
    padding_zeros: &[u8],
) {
    let bpp = glyph.format.bytes_per_pixel();

    // Upload the glyph body.
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d { x, y, z: page_idx },
            aspect: wgpu::TextureAspect::All,
        },
        &glyph.bitmap,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(glyph.width * bpp),
            rows_per_image: None,
        },
        Extent3d {
            width: glyph.width,
            height: glyph.height,
            depth_or_array_layers: 1,
        },
    );

    if padding == 0 {
        return;
    }

    // Zero the right padding strip: padding × height pixels.
    let right_bytes = (padding * glyph.height * bpp) as usize;
    if right_bytes > 0 && right_bytes <= padding_zeros.len() {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: x + glyph.width,
                    y,
                    z: page_idx,
                },
                aspect: wgpu::TextureAspect::All,
            },
            &padding_zeros[..right_bytes],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padding * bpp),
                rows_per_image: None,
            },
            Extent3d {
                width: padding,
                height: glyph.height,
                depth_or_array_layers: 1,
            },
        );
    }

    // Zero the bottom padding strip: (width + padding) × padding pixels.
    let bottom_w = glyph.width + padding;
    let bottom_bytes = (bottom_w * padding * bpp) as usize;
    if bottom_bytes > 0 && bottom_bytes <= padding_zeros.len() {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x,
                    y: y + glyph.height,
                    z: page_idx,
                },
                aspect: wgpu::TextureAspect::All,
            },
            &padding_zeros[..bottom_bytes],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bottom_w * bpp),
                rows_per_image: None,
            },
            Extent3d {
                width: bottom_w,
                height: padding,
                depth_or_array_layers: 1,
            },
        );
    }
}
