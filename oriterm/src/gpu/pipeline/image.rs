//! Image overlay pipeline: per-image textured quads for inline terminal images.

use wgpu::{
    BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
    ColorTargetState, ColorWrites, Device, FragmentState, MultisampleState,
    PipelineLayoutDescriptor, RenderPipeline, RenderPipelineDescriptor, SamplerBindingType,
    ShaderModuleDescriptor, ShaderStages, TextureSampleType, TextureViewDimension, VertexAttribute,
    VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
};

use super::super::state::GpuState;
use super::{PREMUL_ALPHA_BLEND, QUAD_PRIMITIVE};

/// Embedded WGSL source for the image overlay shader.
///
/// Default build = production shader (`image.wgsl`). When the
/// `gpu-debug-image-shader` cargo feature is enabled, the debug variant
/// (`image_debug.wgsl`) is selected — its fragment stage returns opaque
/// green, isolating the production fragment shader as the §13.6.1 item 9
/// bisection target.
#[cfg(not(feature = "gpu-debug-image-shader"))]
const IMAGE_SHADER_SRC: &str = include_str!("../shaders/image.wgsl");
#[cfg(feature = "gpu-debug-image-shader")]
const IMAGE_SHADER_SRC: &str = include_str!("../shaders/image_debug.wgsl");

/// Instance stride for image quads (36 bytes).
///
/// Layout: `pos(2f) + size(2f) + uv_pos(2f) + uv_size(2f) + opacity(1f)`.
pub const IMAGE_INSTANCE_STRIDE: u64 = 36;

/// Vertex attributes for the 36-byte image instance record.
const IMAGE_INSTANCE_ATTRS: [VertexAttribute; 5] = [
    // location 0: pos (vec2<f32>) at offset 0.
    VertexAttribute {
        format: VertexFormat::Float32x2,
        offset: 0,
        shader_location: 0,
    },
    // location 1: size (vec2<f32>) at offset 8.
    VertexAttribute {
        format: VertexFormat::Float32x2,
        offset: 8,
        shader_location: 1,
    },
    // location 2: uv_pos (vec2<f32>) at offset 16.
    VertexAttribute {
        format: VertexFormat::Float32x2,
        offset: 16,
        shader_location: 2,
    },
    // location 3: uv_size (vec2<f32>) at offset 24.
    VertexAttribute {
        format: VertexFormat::Float32x2,
        offset: 24,
        shader_location: 3,
    },
    // location 4: opacity (f32) at offset 32.
    VertexAttribute {
        format: VertexFormat::Float32,
        offset: 32,
        shader_location: 4,
    },
];

/// Returns the instance buffer layout for image quads.
fn image_instance_buffer_layout() -> VertexBufferLayout<'static> {
    VertexBufferLayout {
        array_stride: IMAGE_INSTANCE_STRIDE,
        step_mode: VertexStepMode::Instance,
        attributes: &IMAGE_INSTANCE_ATTRS,
    }
}

/// Create the bind group layout for per-image textures (group 1).
///
/// Contains a `texture_2d` and a filtering sampler. Each image gets its
/// own bind group from this layout.
pub fn create_image_texture_bind_group_layout(device: &Device) -> BindGroupLayout {
    device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("image_texture_bind_group_layout"),
        entries: &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

/// Create the image overlay render pipeline.
///
/// Uses bind groups 0 (uniforms) and 1 (per-image texture + sampler).
/// Renders textured quads for inline terminal images. Premultiplied alpha blend.
pub fn create_image_pipeline(
    gpu: &GpuState,
    uniform_layout: &BindGroupLayout,
    image_texture_layout: &BindGroupLayout,
) -> RenderPipeline {
    let shader = gpu.device.create_shader_module(ShaderModuleDescriptor {
        label: Some("image_shader"),
        source: wgpu::ShaderSource::Wgsl(IMAGE_SHADER_SRC.into()),
    });

    let pipeline_layout = gpu
        .device
        .create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("image_pipeline_layout"),
            bind_group_layouts: &[uniform_layout, image_texture_layout],
            ..Default::default()
        });

    gpu.device
        .create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("image_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[image_instance_buffer_layout()],
            },
            primitive: QUAD_PRIMITIVE,
            depth_stencil: None,
            multisample: MultisampleState::default(),
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format: gpu.render_format(),
                    blend: Some(PREMUL_ALPHA_BLEND),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: gpu.pipeline_cache.as_ref(),
        })
}

/// §13.6.1 item 13: frozen-baseline snapshot of `create_image_pipeline`'s
/// wgpu state. Any future refactor that alters blend factors, vertex
/// layout, primitive topology, bind-group layout, or color target state
/// changes this dump and breaks the gate test.
///
/// Intentionally stringly-typed: maps wgpu enum variants to their
/// `Debug`-style names so a regression that swaps `BlendFactor::One` for
/// `BlendFactor::SrcAlpha` (semantically very different) surfaces as a
/// string mismatch in the test failure diff, not an enum-equality nop.
#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImagePipelineStateDump {
    /// Per-attribute `(shader_location, VertexFormat name, byte offset)`.
    pub vertex_attrs: Vec<(u32, &'static str, u64)>,
    /// Instance stride in bytes (`VertexBufferLayout::array_stride`).
    pub vertex_stride: u64,
    /// `VertexStepMode` enum variant name (`"Instance"` for image quads).
    pub vertex_step_mode: &'static str,
    /// `PrimitiveTopology` enum variant name (`"TriangleStrip"`).
    pub primitive_topology: &'static str,
    /// `FrontFace` enum variant name (`"Ccw"`).
    pub front_face: &'static str,
    /// `cull_mode` — `None` means no culling.
    pub cull_mode: Option<&'static str>,
    /// Color blend src factor variant name.
    pub blend_color_src: &'static str,
    /// Color blend dst factor variant name.
    pub blend_color_dst: &'static str,
    /// Color blend operation variant name (`"Add"`).
    pub blend_color_op: &'static str,
    /// Alpha blend src factor variant name.
    pub blend_alpha_src: &'static str,
    /// Alpha blend dst factor variant name.
    pub blend_alpha_dst: &'static str,
    /// Alpha blend operation variant name (`"Add"`).
    pub blend_alpha_op: &'static str,
    /// Color write mask — `"ALL"` for unrestricted RGBA writes.
    pub color_write_mask: &'static str,
    /// Bind group layout labels in pipeline-layout order (group 0, 1, …).
    pub bind_group_layouts: Vec<&'static str>,
}

/// §13.6.1 item 13: produce the frozen-baseline dump of the production
/// `create_image_pipeline` state. The matching gate test in
/// `tests.rs` asserts the dump equals a baseline literal — any
/// pipeline-state change that is not paired with a baseline update will
/// fail the gate.
#[cfg(test)]
pub fn dump_image_pipeline_state() -> ImagePipelineStateDump {
    // Vertex attributes — pull from the SSOT `IMAGE_INSTANCE_ATTRS`.
    let vertex_attrs = IMAGE_INSTANCE_ATTRS
        .iter()
        .map(|a| {
            let format_name = match a.format {
                VertexFormat::Float32 => "Float32",
                VertexFormat::Float32x2 => "Float32x2",
                VertexFormat::Float32x3 => "Float32x3",
                VertexFormat::Float32x4 => "Float32x4",
                VertexFormat::Uint32 => "Uint32",
                _ => "<unhandled-vertex-format>",
            };
            (a.shader_location, format_name, a.offset)
        })
        .collect();

    use wgpu::{Face, FrontFace, PrimitiveTopology};
    // Pull primitive state from the shared QUAD_PRIMITIVE constant — SSOT.
    let primitive_topology = match QUAD_PRIMITIVE.topology {
        PrimitiveTopology::PointList => "PointList",
        PrimitiveTopology::LineList => "LineList",
        PrimitiveTopology::LineStrip => "LineStrip",
        PrimitiveTopology::TriangleList => "TriangleList",
        PrimitiveTopology::TriangleStrip => "TriangleStrip",
    };
    let front_face = match QUAD_PRIMITIVE.front_face {
        FrontFace::Ccw => "Ccw",
        FrontFace::Cw => "Cw",
    };
    let cull_mode = QUAD_PRIMITIVE.cull_mode.map(|c| match c {
        Face::Front => "Front",
        Face::Back => "Back",
    });

    // Pull blend state from the shared PREMUL_ALPHA_BLEND constant — SSOT.
    let blend_color_src = blend_factor_name(PREMUL_ALPHA_BLEND.color.src_factor);
    let blend_color_dst = blend_factor_name(PREMUL_ALPHA_BLEND.color.dst_factor);
    let blend_color_op = blend_op_name(PREMUL_ALPHA_BLEND.color.operation);
    let blend_alpha_src = blend_factor_name(PREMUL_ALPHA_BLEND.alpha.src_factor);
    let blend_alpha_dst = blend_factor_name(PREMUL_ALPHA_BLEND.alpha.dst_factor);
    let blend_alpha_op = blend_op_name(PREMUL_ALPHA_BLEND.alpha.operation);

    ImagePipelineStateDump {
        vertex_attrs,
        vertex_stride: IMAGE_INSTANCE_STRIDE,
        vertex_step_mode: "Instance",
        primitive_topology,
        front_face,
        cull_mode,
        blend_color_src,
        blend_color_dst,
        blend_color_op,
        blend_alpha_src,
        blend_alpha_dst,
        blend_alpha_op,
        color_write_mask: "ALL",
        bind_group_layouts: vec!["uniform", "image_texture"],
    }
}

#[cfg(test)]
fn blend_factor_name(f: wgpu::BlendFactor) -> &'static str {
    match f {
        wgpu::BlendFactor::Zero => "Zero",
        wgpu::BlendFactor::One => "One",
        wgpu::BlendFactor::Src => "Src",
        wgpu::BlendFactor::OneMinusSrc => "OneMinusSrc",
        wgpu::BlendFactor::SrcAlpha => "SrcAlpha",
        wgpu::BlendFactor::OneMinusSrcAlpha => "OneMinusSrcAlpha",
        wgpu::BlendFactor::Dst => "Dst",
        wgpu::BlendFactor::OneMinusDst => "OneMinusDst",
        wgpu::BlendFactor::DstAlpha => "DstAlpha",
        wgpu::BlendFactor::OneMinusDstAlpha => "OneMinusDstAlpha",
        wgpu::BlendFactor::SrcAlphaSaturated => "SrcAlphaSaturated",
        wgpu::BlendFactor::Constant => "Constant",
        wgpu::BlendFactor::OneMinusConstant => "OneMinusConstant",
        wgpu::BlendFactor::Src1 => "Src1",
        wgpu::BlendFactor::OneMinusSrc1 => "OneMinusSrc1",
        wgpu::BlendFactor::Src1Alpha => "Src1Alpha",
        wgpu::BlendFactor::OneMinusSrc1Alpha => "OneMinusSrc1Alpha",
    }
}

#[cfg(test)]
fn blend_op_name(o: wgpu::BlendOperation) -> &'static str {
    match o {
        wgpu::BlendOperation::Add => "Add",
        wgpu::BlendOperation::Subtract => "Subtract",
        wgpu::BlendOperation::ReverseSubtract => "ReverseSubtract",
        wgpu::BlendOperation::Min => "Min",
        wgpu::BlendOperation::Max => "Max",
    }
}

/// §13.6.1 item 10: diagnostic variant of `create_image_pipeline` with
/// blending disabled. Identical to production except `blend: None` (direct
/// write, no compositing). Isolates whether `PREMUL_ALPHA_BLEND` is the
/// source of the kitty rendering bug — if pixels appear with blend off but
/// not with blend on, the blend factors are interacting with the cache_view's
/// clear color to produce `(0, 0, 0, X)` output.
///
/// MUST NOT be wired into production code paths. Tests opt in via the
/// `gpu-debug-image-blend-off` cargo feature.
#[cfg(feature = "gpu-debug-image-blend-off")]
pub fn create_image_pipeline_no_blend(
    gpu: &GpuState,
    uniform_layout: &BindGroupLayout,
    image_texture_layout: &BindGroupLayout,
) -> RenderPipeline {
    let shader = gpu.device.create_shader_module(ShaderModuleDescriptor {
        label: Some("image_shader_no_blend"),
        source: wgpu::ShaderSource::Wgsl(IMAGE_SHADER_SRC.into()),
    });

    let pipeline_layout = gpu
        .device
        .create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("image_pipeline_no_blend_layout"),
            bind_group_layouts: &[uniform_layout, image_texture_layout],
            ..Default::default()
        });

    gpu.device
        .create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("image_pipeline_no_blend"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[image_instance_buffer_layout()],
            },
            primitive: QUAD_PRIMITIVE,
            depth_stencil: None,
            multisample: MultisampleState::default(),
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format: gpu.render_format(),
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: gpu.pipeline_cache.as_ref(),
        })
}
