//! Minimal-quad-draw reproducer pilot — §13.6.1 item 11.
//!
//! Bypasses `WindowRenderer::prepare` entirely. Constructs a 1×1 red RGBA
//! texture, the production image bind-group layout, a single hand-encoded
//! `ImageQuad` instance buffer, and a single render pass writing to a 16×16
//! render target. Reads back the rendered pixels and asserts at least one
//! red pixel exists. Isolates pipeline-state correctness (shader + blend +
//! vertex layout + bind group wiring) from all `WindowRenderer` orchestration
//! — content cache, multi-pane composition, scroll, damage tracking, snapshot
//! plumbing.
//!
//! If this pilot produces red pixels while the production kitty render
//! pipeline produces no visible image, the bug is above the pipeline-state
//! boundary: orchestration, frame plumbing, or quad-emission upstream of
//! the GPU pass. If this pilot ALSO produces no red pixels, the bug is in
//! pipeline state itself: shader, blend factors, vertex layout, or bind
//! group binding indices.
//!
//! `#[ignore]`-gated diagnostic probe (matches the §13.6.1 instrumentation
//! pattern established by `probe_kitty_pipeline_stages`). Run manually via:
//!
//! ```text
//! cargo test -p oriterm --lib --features gpu-tests \
//!     -- --ignored minimal_image_quad
//! ```

#![cfg(all(test, feature = "gpu-tests"))]

use crate::gpu::pipeline::{
    create_image_pipeline, create_image_texture_bind_group_layout,
    create_uniform_bind_group_layout,
};
use crate::gpu::state::GpuState;

/// Render target side length (16×16 = 1024-byte RGBA, well above the
/// `COPY_BYTES_PER_ROW_ALIGNMENT = 256` floor).
const TARGET_DIM: u32 = 16;

/// Image quad pixel extent at top-left of the render target.
const QUAD_DIM: f32 = 8.0;

/// Pack a `[f32; N]` to little-endian bytes for vertex/uniform upload.
fn f32_slice_to_bytes<const N: usize>(values: [f32; N]) -> Vec<u8> {
    let mut out = Vec::with_capacity(N * 4);
    for v in values {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

#[test]
#[ignore = "BUG-08-043: diagnostic probe — kitty image pipeline bisection (minimal quad); run via cargo test -p oriterm --lib --features gpu-tests -- --ignored minimal_image_quad"]
fn minimal_image_quad_reproducer_produces_red_pixel() {
    let Ok(gpu) = GpuState::new_headless() else {
        eprintln!("SKIP: no GPU adapter available (headless init failed)");
        return;
    };

    let device = gpu.device_for_test();
    let queue = gpu.queue_for_test();

    // --- 1. Build pipeline + layouts directly (no GpuPipelines / WindowRenderer).
    let uniform_layout = create_uniform_bind_group_layout(device);
    let image_layout = create_image_texture_bind_group_layout(device);
    let pipeline = create_image_pipeline(&gpu, &uniform_layout, &image_layout);

    // --- 2. Uniform buffer: vec2<f32> screen_size + vec2<f32> _pad = 16 bytes.
    let uniform_bytes =
        f32_slice_to_bytes([TARGET_DIM as f32, TARGET_DIM as f32, 0.0, 0.0]);
    let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("minimal_quad_uniform"),
        size: uniform_bytes.len() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&uniform_buf, 0, &uniform_bytes);
    let uniform_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("minimal_quad_uniform_bg"),
        layout: &uniform_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buf.as_entire_binding(),
        }],
    });

    // --- 3. 1×1 RGBA red texture, uploaded via queue.write_texture.
    let red_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("minimal_quad_red_texture"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let red_bytes: [u8; 4] = [0xFF, 0x00, 0x00, 0xFF];
    queue.write_texture(
        red_texture.as_image_copy(),
        &red_bytes,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4),
            rows_per_image: Some(1),
        },
        wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    let red_view = red_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("minimal_quad_sampler"),
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    let image_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("minimal_quad_image_bg"),
        layout: &image_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&red_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });

    // --- 4. Instance buffer: one ImageQuad record (36 bytes per IMAGE_INSTANCE_ATTRS).
    //   pos(2f) + size(2f) + uv_pos(2f) + uv_size(2f) + opacity(1f) = 36 bytes.
    let instance_bytes = f32_slice_to_bytes([
        0.0, 0.0, // pos: top-left in pixels
        QUAD_DIM, QUAD_DIM, // size: 8×8 px
        0.0, 0.0, // uv_pos
        1.0, 1.0, // uv_size (full texture)
        1.0, // opacity
    ]);
    let instance_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("minimal_quad_instance"),
        size: instance_bytes.len() as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&instance_buf, 0, &instance_bytes);

    // --- 5. Render target (16×16, RENDER_ATTACHMENT + COPY_SRC).
    let target = gpu.create_render_target(TARGET_DIM, TARGET_DIM);

    // --- 6. Encode + submit: clear to black, draw 1 image quad (4 vertices, 1 instance).
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("minimal_quad_encoder"),
    });
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("minimal_quad_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target.view(),
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &uniform_bg, &[]);
        pass.set_bind_group(1, &image_bg, &[]);
        pass.set_vertex_buffer(0, instance_buf.slice(..));
        pass.draw(0..4, 0..1);
    }
    queue.submit(std::iter::once(encoder.finish()));

    // --- 7. Readback + assert.
    let pixels = gpu
        .read_render_target(&target)
        .expect("minimal_image_quad: pixel readback should succeed");
    assert_eq!(
        pixels.len(),
        (TARGET_DIM * TARGET_DIM * 4) as usize,
        "minimal_image_quad: readback length must equal width * height * 4"
    );

    // Scan for any red-ish pixel (r > 100, g < 50, b < 50). Tolerates the
    // sRGB-vs-linear distinction in the framebuffer format — pure red goes
    // through textureSample without modification because the shader copies
    // tex.rgb through with opacity = 1 and tex.a = 1 (no premultiplied
    // attenuation against an alpha < 1 surface).
    let mut red_count = 0usize;
    let mut first_red: Option<(u32, u32, u8, u8, u8, u8)> = None;
    for y in 0..TARGET_DIM {
        for x in 0..TARGET_DIM {
            let off = ((y * TARGET_DIM + x) * 4) as usize;
            let r = pixels[off];
            let g = pixels[off + 1];
            let b = pixels[off + 2];
            let a = pixels[off + 3];
            if r > 100 && g < 50 && b < 50 {
                red_count += 1;
                if first_red.is_none() {
                    first_red = Some((x, y, r, g, b, a));
                }
            }
        }
    }

    assert!(
        red_count >= 1,
        "minimal_image_quad: expected at least 1 red pixel in {TARGET_DIM}x{TARGET_DIM} \
         render output (8x8 image quad at top-left), got 0; \
         indicates pipeline-state defect (shader, blend, vertex layout, or bind \
         group wiring) — orchestration is bypassed in this pilot, so any failure \
         here is squarely in pipeline state per §13.6.1 item 11"
    );
    eprintln!(
        "[minimal_image_quad] {red_count} red pixels (of {} sampled); first at {:?}",
        TARGET_DIM * TARGET_DIM,
        first_red,
    );
}
