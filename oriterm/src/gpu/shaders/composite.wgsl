// Layer composition shader: textured quads with per-layer transform and opacity.
//
// Each layer is rendered as a quad defined by its bounds, with a 2D affine
// transform applied. The layer's offscreen texture is sampled and multiplied
// by opacity (premultiplied alpha). Back-to-front draw order with
// PREMUL_ALPHA_BLEND gives correct transparency compositing.

struct ScreenUniform {
    screen_size: vec2<f32>,
    _pad: vec2<f32>,
}

struct LayerUniform {
    // 2D affine transform as mat3x3.
    // Column 0: (a, b, 0), Column 1: (c, d, 0), Column 2: (tx, ty, 1).
    transform: mat3x3<f32>,
    // Layer bounds in screen pixels: (x, y, width, height).
    bounds: vec4<f32>,
    // Layer opacity (0.0 = transparent, 1.0 = opaque).
    opacity: f32,
    _pad: vec3<f32>,
}

@group(0) @binding(0)
var<uniform> screen: ScreenUniform;

@group(1) @binding(0)
var<uniform> layer: LayerUniform;

@group(2) @binding(0)
var layer_texture: texture_2d<f32>;

@group(2) @binding(1)
var layer_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // TriangleStrip corners: 0=TL, 1=TR, 2=BL, 3=BR.
    let corner = vec2<f32>(
        f32(vertex_index & 1u),
        f32((vertex_index >> 1u) & 1u),
    );

    // Local position within layer bounds (pixels).
    let local = vec2<f32>(
        layer.bounds.x + layer.bounds.z * corner.x,
        layer.bounds.y + layer.bounds.w * corner.y,
    );

    // Apply 2D affine transform.
    let transformed = (layer.transform * vec3<f32>(local, 1.0)).xy;

    // Pixel to NDC.
    let ndc = vec2<f32>(
        transformed.x / screen.screen_size.x * 2.0 - 1.0,
        1.0 - transformed.y / screen.screen_size.y * 2.0,
    );

    var out: VertexOutput;
    out.position = vec4<f32>(ndc, 0.0, 1.0);
    out.uv = corner;
    return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(layer_texture, layer_sampler, input.uv);
    // Apply layer opacity (premultiplied alpha).
    return texel * layer.opacity;
}
