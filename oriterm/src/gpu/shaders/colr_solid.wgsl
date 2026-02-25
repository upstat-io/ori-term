// COLR v1 solid fill shader: composites a glyph alpha mask with a solid color.
//
// Used during render-to-texture COLR v1 compositing. Each layer's glyph outline
// is rasterized to an alpha mask (R8Unorm), then this shader fills the masked
// region with a solid premultiplied RGBA color.

struct FillUniforms {
    color: vec4<f32>,
}

@group(0) @binding(0)
var alpha_texture: texture_2d<f32>;

@group(0) @binding(1)
var alpha_sampler: sampler;

@group(1) @binding(0)
var<uniform> fill: FillUniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    // Fullscreen triangle strip: 0=TL, 1=TR, 2=BL, 3=BR.
    let uv = vec2<f32>(f32(vi & 1u), f32((vi >> 1u) & 1u));
    let pos = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    var out: VertexOutput;
    out.position = vec4<f32>(pos, 0.0, 1.0);
    out.uv = uv;
    return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let alpha = textureSample(alpha_texture, alpha_sampler, input.uv).r;
    // Premultiplied output: color already premultiplied, scale by mask alpha.
    return fill.color * alpha;
}
