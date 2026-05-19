// Debug variant of `image.wgsl` for §13.6.1 item 9.
//
// Vertex shader is a verbatim copy of the production shader — vertex math
// is shared between debug and production. Only the fragment shader differs:
// returns opaque green regardless of the sampled texture. Proves the
// fragment stage runs and the pipeline's color attachment is wired. If
// green pixels appear at the kitty placement when this shader is active,
// the production bug is in `image.wgsl`'s `textureSample` / RGB×alpha×opacity
// arithmetic (NOT in pipeline-state, vertex-shader, or quad-emission code).
//
// Activated only under the `gpu-debug-image-shader` cargo feature; never
// part of any default build.

struct Uniform {
    screen_size: vec2<f32>,
    _pad: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniform;

@group(1) @binding(0)
var image_texture: texture_2d<f32>;

@group(1) @binding(1)
var image_sampler: sampler;

struct InstanceInput {
    @location(0) pos: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) uv_pos: vec2<f32>,
    @location(3) uv_size: vec2<f32>,
    @location(4) opacity: f32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
    @location(1) opacity: f32,
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    instance: InstanceInput,
) -> VertexOutput {
    let corner = vec2<f32>(
        f32(vertex_index & 1u),
        f32((vertex_index >> 1u) & 1u),
    );

    let px = instance.pos + instance.size * corner;

    let ndc = vec2<f32>(
        px.x / uniforms.screen_size.x * 2.0 - 1.0,
        1.0 - px.y / uniforms.screen_size.y * 2.0,
    );

    let tex_coord = instance.uv_pos + instance.uv_size * corner;

    var out: VertexOutput;
    out.position = vec4<f32>(ndc, 0.0, 1.0);
    out.tex_coord = tex_coord;
    out.opacity = instance.opacity;
    return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.0, 1.0, 0.0, 1.0);
}
