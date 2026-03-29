//! UI rect pipeline: dedicated 144-byte instance layout and render pipeline.

use wgpu::{
    BindGroupLayout, ColorTargetState, ColorWrites, FragmentState, MultisampleState,
    PipelineLayoutDescriptor, RenderPipeline, RenderPipelineDescriptor, VertexAttribute,
    VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
};

use super::{PREMUL_ALPHA_BLEND, QUAD_PRIMITIVE};
use crate::gpu::state::GpuState;
use crate::gpu::ui_rect_writer::UI_RECT_INSTANCE_SIZE;

/// Embedded WGSL source for the UI rect shader.
const UI_RECT_SHADER_SRC: &str = include_str!("../shaders/ui_rect.wgsl");

/// UI rect instance stride in bytes.
pub const UI_RECT_STRIDE: u64 = UI_RECT_INSTANCE_SIZE as u64;

/// Vertex attributes for the 144-byte UI rect instance record.
///
/// Maps to the `InstanceInput` struct in `ui_rect.wgsl`. Fully independent
/// from the terminal instance format (`INSTANCE_ATTRS`).
pub const UI_RECT_ATTRS: [VertexAttribute; 10] = [
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
    // location 2: clip (vec4<f32>) at offset 16.
    VertexAttribute {
        format: VertexFormat::Float32x4,
        offset: 16,
        shader_location: 2,
    },
    // location 3: fill_color (vec4<f32>) at offset 32.
    VertexAttribute {
        format: VertexFormat::Float32x4,
        offset: 32,
        shader_location: 3,
    },
    // location 4: border_widths (vec4<f32>) at offset 48.
    VertexAttribute {
        format: VertexFormat::Float32x4,
        offset: 48,
        shader_location: 4,
    },
    // location 5: corner_radii (vec4<f32>) at offset 64.
    VertexAttribute {
        format: VertexFormat::Float32x4,
        offset: 64,
        shader_location: 5,
    },
    // location 6: border_top (vec4<f32>) at offset 80.
    VertexAttribute {
        format: VertexFormat::Float32x4,
        offset: 80,
        shader_location: 6,
    },
    // location 7: border_right (vec4<f32>) at offset 96.
    VertexAttribute {
        format: VertexFormat::Float32x4,
        offset: 96,
        shader_location: 7,
    },
    // location 8: border_bottom (vec4<f32>) at offset 112.
    VertexAttribute {
        format: VertexFormat::Float32x4,
        offset: 112,
        shader_location: 8,
    },
    // location 9: border_left (vec4<f32>) at offset 128.
    VertexAttribute {
        format: VertexFormat::Float32x4,
        offset: 128,
        shader_location: 9,
    },
];

/// Returns the instance buffer layout for the UI rect pipeline.
pub fn ui_rect_buffer_layout() -> VertexBufferLayout<'static> {
    VertexBufferLayout {
        array_stride: UI_RECT_STRIDE,
        step_mode: VertexStepMode::Instance,
        attributes: &UI_RECT_ATTRS,
    }
}

/// Create the UI rect render pipeline.
///
/// Uses only bind group 0 (uniforms). Renders SDF rounded rectangles with
/// per-side borders via the `ui_rect.wgsl` shader. Premultiplied alpha blend.
pub fn create_ui_rect_pipeline(gpu: &GpuState, uniform_layout: &BindGroupLayout) -> RenderPipeline {
    let shader = gpu
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ui_rect_shader"),
            source: wgpu::ShaderSource::Wgsl(UI_RECT_SHADER_SRC.into()),
        });

    let pipeline_layout = gpu
        .device
        .create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("ui_rect_pipeline_layout"),
            bind_group_layouts: &[uniform_layout],
            ..Default::default()
        });

    gpu.device
        .create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("ui_rect_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[ui_rect_buffer_layout()],
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
