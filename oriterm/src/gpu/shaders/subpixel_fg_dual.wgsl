enable dual_source_blending;

// Dual-source subpixel foreground shader: true per-channel LCD compositing.
//
// Uses WGSL `@blend_src` to output both the composited color and the
// per-channel coverage mask. The GPU hardware performs:
//   final = src0 * src1 + dst * (1 - src1)
// This achieves optically correct per-channel blending without requiring
// the CPU to pass background color as instance data.
//
// Requires the DUAL_SOURCE_BLENDING device feature.

struct Uniform {
    screen_size: vec2<f32>,
    _pad: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniform;

@group(1) @binding(0)
var subpixel_atlas_texture: texture_2d_array<f32>;

@group(1) @binding(1)
var subpixel_atlas_sampler: sampler;

struct InstanceInput {
    @location(0) pos: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) uv: vec4<f32>,
    @location(3) fg_color: vec4<f32>,
    @location(4) bg_color: vec4<f32>,
    @location(5) kind: u32,
    @location(6) atlas_page: u32,
    @location(7) clip: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) fg_color: vec4<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) @interpolate(flat) atlas_page: u32,
    @location(3) clip_min: vec2<f32>,
    @location(4) clip_max: vec2<f32>,
}

struct FragmentOutput {
    @location(0) @blend_src(0) color: vec4<f32>,
    @location(0) @blend_src(1) mask: vec4<f32>,
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

    let tex_coord = instance.uv.xy + instance.uv.zw * corner;

    var out: VertexOutput;
    out.position = vec4<f32>(ndc, 0.0, 1.0);
    out.fg_color = instance.fg_color;
    out.tex_coord = tex_coord;
    out.atlas_page = instance.atlas_page;
    out.clip_min = instance.clip.xy;
    out.clip_max = instance.clip.xy + instance.clip.zw;
    return out;
}

@fragment
fn fs_main(input: VertexOutput) -> FragmentOutput {
    // Per-instance clip rect test.
    let frag_pos = input.position.xy;
    if frag_pos.x < input.clip_min.x || frag_pos.x > input.clip_max.x
        || frag_pos.y < input.clip_min.y || frag_pos.y > input.clip_max.y {
        discard;
    }

    // Sample per-channel coverage mask from the subpixel atlas.
    let mask = textureSample(subpixel_atlas_texture, subpixel_atlas_sampler, input.tex_coord, input.atlas_page);

    let fg = input.fg_color;
    let dim = fg.a;

    // Scale mask by dim factor for dimmed panes.
    let scaled = vec4<f32>(mask.r * dim, mask.g * dim, mask.b * dim, max(mask.r, max(mask.g, mask.b)) * dim);

    // Output foreground color and per-channel coverage mask.
    // GPU blends: color * mask + framebuffer * (1 - mask)
    return FragmentOutput(
        vec4<f32>(fg.rgb, 1.0),
        scaled,
    );
}
