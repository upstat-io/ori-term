// COLR v1 gradient fill shader: composites a glyph alpha mask with a gradient.
//
// Supports linear, radial, and sweep gradients via a `gradient_type` uniform.
// Color stops are passed as a uniform array (max 16 stops). The gradient is
// computed in font-unit UV space, then masked by the alpha texture.

const MAX_STOPS: u32 = 16u;
const GRAD_LINEAR: u32 = 0u;
const GRAD_RADIAL: u32 = 1u;
const GRAD_SWEEP: u32 = 2u;

// Extend modes matching skrifa::color::Extend.
const EXTEND_PAD: u32 = 0u;
const EXTEND_REPEAT: u32 = 1u;
const EXTEND_REFLECT: u32 = 2u;

struct GradientStop {
    color: vec4<f32>,
    offset: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
}

struct GradientUniforms {
    // Gradient geometry (interpretation depends on gradient_type).
    // Linear: p0 = point0.xy, p1 = point1.xy
    // Radial: p0 = center0.xy, p1.x = radius0, p1.y = radius1, p1.zw = center1.xy
    // Sweep: p0 = center.xy, p1.x = start_angle, p1.y = end_angle
    point0: vec4<f32>,
    point1: vec4<f32>,
    gradient_type: u32,
    extend_mode: u32,
    stop_count: u32,
    _pad: u32,
    stops: array<GradientStop, 16>,
}

@group(0) @binding(0)
var alpha_texture: texture_2d<f32>;

@group(0) @binding(1)
var alpha_sampler: sampler;

@group(1) @binding(0)
var<uniform> grad: GradientUniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    let uv = vec2<f32>(f32(vi & 1u), f32((vi >> 1u) & 1u));
    let pos = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    var out: VertexOutput;
    out.position = vec4<f32>(pos, 0.0, 1.0);
    out.uv = uv;
    return out;
}

/// Apply extend mode to a parameter t.
fn apply_extend(t: f32, mode: u32) -> f32 {
    if mode == EXTEND_PAD {
        return clamp(t, 0.0, 1.0);
    }
    if mode == EXTEND_REPEAT {
        return t - floor(t);
    }
    // EXTEND_REFLECT
    let period = t - 2.0 * floor(t * 0.5);
    if period > 1.0 {
        return 2.0 - period;
    }
    return period;
}

/// Sample the gradient color at parameter t using the color stop array.
fn sample_gradient(t: f32) -> vec4<f32> {
    let tc = apply_extend(t, grad.extend_mode);

    // Before first stop.
    if tc <= grad.stops[0].offset || grad.stop_count == 1u {
        return grad.stops[0].color;
    }

    // Between stops — linear interpolation.
    for (var i = 1u; i < grad.stop_count; i = i + 1u) {
        if tc <= grad.stops[i].offset {
            let prev = grad.stops[i - 1u];
            let curr = grad.stops[i];
            let f = (tc - prev.offset) / max(curr.offset - prev.offset, 0.0001);
            return mix(prev.color, curr.color, f);
        }
    }

    // After last stop.
    return grad.stops[grad.stop_count - 1u].color;
}

/// Compute the gradient parameter t for linear gradient.
fn linear_t(pos: vec2<f32>) -> f32 {
    let p0 = grad.point0.xy;
    let p1 = grad.point0.zw;
    let d = p1 - p0;
    let len_sq = dot(d, d);
    if len_sq < 0.0001 {
        return 0.0;
    }
    return dot(pos - p0, d) / len_sq;
}

/// Compute the gradient parameter t for radial gradient.
fn radial_t(pos: vec2<f32>) -> f32 {
    let c0 = grad.point0.xy;
    let r0 = grad.point0.z;
    let c1 = grad.point0.zw;
    // point1.xy holds (r0, r1) for radial.
    let r0_val = grad.point1.x;
    let r1_val = grad.point1.y;
    let c1_actual = grad.point1.zw;

    let dc = c1_actual - c0;
    let dr = r1_val - r0_val;
    let dp = pos - c0;

    let a = dot(dc, dc) - dr * dr;
    let b = 2.0 * (dot(dp, dc) - r0_val * dr);
    let c = dot(dp, dp) - r0_val * r0_val;

    let disc = b * b - 4.0 * a * c;
    if disc < 0.0 {
        return 0.0;
    }

    let sqrt_disc = sqrt(disc);
    // Use the larger root for the outer circle.
    if abs(a) < 0.0001 {
        if abs(b) < 0.0001 {
            return 0.0;
        }
        return -c / b;
    }
    let t1 = (-b + sqrt_disc) / (2.0 * a);
    let t2 = (-b - sqrt_disc) / (2.0 * a);
    // Pick the largest t where the interpolated radius is non-negative.
    let t_max = max(t1, t2);
    let t_min = min(t1, t2);
    if r0_val + t_max * dr >= 0.0 {
        return t_max;
    }
    return t_min;
}

/// Compute the gradient parameter t for sweep gradient.
fn sweep_t(pos: vec2<f32>) -> f32 {
    let center = grad.point0.xy;
    let start_angle = grad.point1.x;
    let end_angle = grad.point1.y;

    let dp = pos - center;
    // atan2 returns [-PI, PI], convert to degrees [0, 360).
    var angle = degrees(atan2(-dp.y, dp.x));
    if angle < 0.0 {
        angle = angle + 360.0;
    }

    let range = end_angle - start_angle;
    if abs(range) < 0.0001 {
        return 0.0;
    }
    return (angle - start_angle) / range;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let alpha = textureSample(alpha_texture, alpha_sampler, input.uv).r;

    var t: f32;
    if grad.gradient_type == GRAD_LINEAR {
        t = linear_t(input.uv);
    } else if grad.gradient_type == GRAD_RADIAL {
        t = radial_t(input.uv);
    } else {
        t = sweep_t(input.uv);
    }

    let color = sample_gradient(t);
    return color * alpha;
}
