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

/// Upload a glyph bitmap to a position on a texture array layer.
///
/// Handles both `R8Unorm` (1 byte/pixel) and `Rgba8Unorm` (4 bytes/pixel)
/// textures based on the glyph's format.
#[expect(
    clippy::too_many_arguments,
    reason = "GPU texture upload: resource refs, destination coords, glyph data"
)]
pub(super) fn upload_glyph(
    queue: &Queue,
    texture: &Texture,
    page_idx: u32,
    x: u32,
    y: u32,
    glyph: &RasterizedGlyph,
) {
    let bpp = glyph.format.bytes_per_pixel();
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
}
