// UI rect shader: SDF rounded rectangles with optional border.
//
// Renders axis-aligned rounded rectangles using a signed distance field.
// Fill color comes from bg_color, border color from fg_color. Corner radius
// and border width are passed via instance attributes at offsets 72 and 76.
// Premultiplied alpha output matches PREMUL_ALPHA_BLEND.
// Per-instance clip rect discards out-of-bounds fragments.

struct Uniform {
    screen_size: vec2<f32>,
    _pad: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniform;

struct InstanceInput {
    @location(0) pos: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) uv: vec4<f32>,
    @location(3) fg_color: vec4<f32>,       // border color
    @location(4) bg_color: vec4<f32>,       // fill color
    @location(5) kind: u32,
    @location(6) atlas_page: u32,
    @location(7) corner_radius: f32,
    @location(8) border_width: f32,
    @location(9) clip: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) fill_color: vec4<f32>,
    @location(1) border_color: vec4<f32>,
    @location(2) local_pos: vec2<f32>,      // pixel offset from rect center
    @location(3) half_size: vec2<f32>,       // half-extents of the rect
    @location(4) corner_radius: f32,
    @location(5) border_width: f32,
    @location(6) clip_min: vec2<f32>,
    @location(7) clip_max: vec2<f32>,
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    instance: InstanceInput,
) -> VertexOutput {
    // TriangleStrip corners: 0=TL, 1=TR, 2=BL, 3=BR.
    let corner = vec2<f32>(
        f32(vertex_index & 1u),
        f32((vertex_index >> 1u) & 1u),
    );

    let px = instance.pos + instance.size * corner;

    // Pixel to NDC.
    let ndc = vec2<f32>(
        px.x / uniforms.screen_size.x * 2.0 - 1.0,
        1.0 - px.y / uniforms.screen_size.y * 2.0,
    );

    // Local position relative to rect center (for SDF evaluation).
    let half = instance.size * 0.5;
    let local = (corner - 0.5) * instance.size;

    var out: VertexOutput;
    out.position = vec4<f32>(ndc, 0.0, 1.0);
    out.fill_color = instance.bg_color;
    out.border_color = instance.fg_color;
    out.local_pos = local;
    out.half_size = half;
    out.corner_radius = instance.corner_radius;
    out.border_width = instance.border_width;
    out.clip_min = instance.clip.xy;
    out.clip_max = instance.clip.xy + instance.clip.zw;
    return out;
}

// Signed distance to a rounded box (Inigo Quilez).
// Returns negative inside, positive outside. `r` is the corner radius.
fn sd_rounded_box(p: vec2<f32>, half_size: vec2<f32>, r: f32) -> f32 {
    // Clamp radius to half the smallest dimension.
    let max_r = min(half_size.x, half_size.y);
    let cr = min(r, max_r);
    let q = abs(p) - half_size + cr;
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - cr;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Per-instance clip rect test.
    let frag_pos = input.position.xy;
    if frag_pos.x < input.clip_min.x || frag_pos.x > input.clip_max.x
        || frag_pos.y < input.clip_min.y || frag_pos.y > input.clip_max.y {
        discard;
    }

    let d_outer = sd_rounded_box(input.local_pos, input.half_size, input.corner_radius);

    // Anti-aliased outer edge: 1px smoothstep.
    let outer_alpha = 1.0 - smoothstep(-0.5, 0.5, d_outer);

    if outer_alpha <= 0.0 {
        discard;
    }

    var color: vec4<f32>;

    if input.border_width > 0.0 {
        // Inner SDF for border: inset by border_width.
        let inner_radius = max(input.corner_radius - input.border_width, 0.0);
        let inner_half = input.half_size - input.border_width;
        let d_inner = sd_rounded_box(input.local_pos, inner_half, inner_radius);

        // Anti-aliased inner edge.
        let inner_alpha = 1.0 - smoothstep(-0.5, 0.5, d_inner);

        // Mix: border in the ring between outer and inner, fill inside inner.
        let fill = input.fill_color;
        let bord = input.border_color;
        color = mix(bord, fill, inner_alpha);
    } else {
        color = input.fill_color;
    }

    // Apply outer edge anti-aliasing and premultiply.
    let a = color.a * outer_alpha;
    return vec4<f32>(color.rgb * a, a);
}
