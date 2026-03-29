// UI rect shader: SDF rounded rectangles with per-side borders.
//
// Renders axis-aligned rounded rectangles using signed distance fields.
// Supports per-side border widths, per-side border colors, and four
// independent corner radii. Premultiplied alpha output matches
// PREMUL_ALPHA_BLEND. Per-instance clip rect discards out-of-bounds fragments.

struct Uniform {
    screen_size: vec2<f32>,
    _pad: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniform;

struct InstanceInput {
    @location(0) pos: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) clip: vec4<f32>,
    @location(3) fill_color: vec4<f32>,
    @location(4) border_widths: vec4<f32>,   // [top, right, bottom, left]
    @location(5) corner_radii: vec4<f32>,    // [tl, tr, br, bl]
    @location(6) border_top: vec4<f32>,      // RGBA linear
    @location(7) border_right: vec4<f32>,
    @location(8) border_bottom: vec4<f32>,
    @location(9) border_left: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) fill_color: vec4<f32>,
    @location(1) local_pos: vec2<f32>,
    @location(2) half_size: vec2<f32>,
    @location(3) corner_radii: vec4<f32>,
    @location(4) border_widths: vec4<f32>,
    @location(5) border_top: vec4<f32>,
    @location(6) border_right: vec4<f32>,
    @location(7) border_bottom: vec4<f32>,
    @location(8) border_left: vec4<f32>,
    @location(9) clip_min: vec2<f32>,
    @location(10) clip_max: vec2<f32>,
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
    out.fill_color = instance.fill_color;
    out.local_pos = local;
    out.half_size = half;
    out.corner_radii = instance.corner_radii;
    out.border_widths = instance.border_widths;
    out.border_top = instance.border_top;
    out.border_right = instance.border_right;
    out.border_bottom = instance.border_bottom;
    out.border_left = instance.border_left;
    out.clip_min = instance.clip.xy;
    out.clip_max = instance.clip.xy + instance.clip.zw;
    return out;
}

// Four-corner rounded-rect SDF.
// Selects the radius for the quadrant the fragment is in, then evaluates
// the quarter-circle SDF for that corner.
fn sd_rounded_rect(p: vec2<f32>, half_size: vec2<f32>, radii: vec4<f32>) -> f32 {
    // Select radius by quadrant: TL, TR, BR, BL.
    let r = select(
        select(radii.w, radii.z, p.x > 0.0),  // bottom: BL or BR
        select(radii.x, radii.y, p.x > 0.0),  // top: TL or TR
        p.y < 0.0
    );
    let cr = min(r, min(half_size.x, half_size.y));
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

    let d_outer = sd_rounded_rect(input.local_pos, input.half_size, input.corner_radii);

    // Anti-aliased outer edge: 1px smoothstep.
    let outer_alpha = 1.0 - smoothstep(-0.5, 0.5, d_outer);

    if outer_alpha <= 0.0 {
        discard;
    }

    let bw = input.border_widths; // [top, right, bottom, left]

    // Check if any border is visible.
    let has_border = bw.x > 0.0 || bw.y > 0.0 || bw.z > 0.0 || bw.w > 0.0;

    var color: vec4<f32>;

    if has_border {
        // Uniform fast path: all four sides same width and color.
        let is_uniform = bw.x == bw.y && bw.y == bw.z && bw.z == bw.w
            && all(input.border_top == input.border_right)
            && all(input.border_right == input.border_bottom)
            && all(input.border_bottom == input.border_left);

        // Inner rect: asymmetric inset by per-side widths.
        let inner_half = input.half_size - vec2<f32>(
            (bw.y + bw.w) * 0.5,  // (right + left) / 2
            (bw.x + bw.z) * 0.5   // (top + bottom) / 2
        );
        let center_offset = vec2<f32>(
            (bw.w - bw.y) * 0.5,  // (left - right) / 2
            (bw.x - bw.z) * 0.5   // (top - bottom) / 2
        );

        // Inner corner radii: shrink by max of adjacent border widths (CSS spec).
        let ir = vec4<f32>(
            max(input.corner_radii.x - max(bw.x, bw.w), 0.0),  // TL: top, left
            max(input.corner_radii.y - max(bw.x, bw.y), 0.0),  // TR: top, right
            max(input.corner_radii.z - max(bw.z, bw.y), 0.0),  // BR: bottom, right
            max(input.corner_radii.w - max(bw.z, bw.w), 0.0),  // BL: bottom, left
        );

        let inner_pos = input.local_pos - center_offset;
        let d_inner = sd_rounded_rect(inner_pos, max(inner_half, vec2<f32>(0.0)), ir);
        let inner_alpha = 1.0 - smoothstep(-0.5, 0.5, d_inner);

        if is_uniform {
            // Uniform: single border color, skip side ownership.
            color = mix(input.border_top, input.fill_color, inner_alpha);
        } else {
            // Per-side: determine border color via diagonal corner ownership.
            let border_color = select_border_color(
                input.local_pos,
                input.half_size,
                bw,
                input.border_top,
                input.border_right,
                input.border_bottom,
                input.border_left,
            );
            color = mix(border_color, input.fill_color, inner_alpha);
        }
    } else {
        color = input.fill_color;
    }

    // Apply outer edge anti-aliasing and premultiply.
    let a = color.a * outer_alpha;
    return vec4<f32>(color.rgb * a, a);
}

// Determine which border side owns a fragment in the border ring.
//
// Straight edge regions pick the nearest side. Corner regions are split
// along the diagonal from the outer corner to the inner corner — the
// CSS 2.1 spec's "angled" join rule. When two adjacent sides have
// different widths (e.g. top=2, left=4), the diagonal tilts so the
// wider side claims more of the corner, proportional to the width ratio.
fn select_border_color(
    p: vec2<f32>,
    half_size: vec2<f32>,
    bw: vec4<f32>,           // border widths [top, right, bottom, left]
    top: vec4<f32>,
    right: vec4<f32>,
    bottom: vec4<f32>,
    left: vec4<f32>,
) -> vec4<f32> {
    // Distance from each edge (positive = inside, toward center).
    let dt = p.y + half_size.y;  // from top
    let db = half_size.y - p.y;  // from bottom
    let dl = p.x + half_size.x;  // from left
    let dr = half_size.x - p.x;  // from right

    // Determine which corner quadrant the fragment is in, and the two
    // adjacent border widths for that corner's diagonal.
    let in_top = dt < db;
    let in_left = dl < dr;

    // Horizontal and vertical border widths for this corner.
    let h_bw = select(bw.y, bw.w, in_left);   // right or left width
    let v_bw = select(bw.z, bw.x, in_top);     // bottom or top width

    // Distance from the nearest horizontal and vertical edges.
    let near_h = select(dr, dl, in_left);
    let near_v = select(db, dt, in_top);

    // CSS diagonal ownership: a fragment belongs to the horizontal side
    // (top/bottom) when its position is above the diagonal from the outer
    // corner to the inner corner. The diagonal slope = v_bw / h_bw.
    //
    // For a corner at the outer edge, the diagonal line satisfies:
    //   near_v / v_bw < near_h / h_bw  →  horizontal (top/bottom) wins
    //   near_v / v_bw > near_h / h_bw  →  vertical (left/right) wins
    //
    // Cross-multiply to avoid division (both widths >= 0):
    //   near_v * h_bw < near_h * v_bw  →  horizontal wins
    let h_wins = near_v * h_bw < near_h * v_bw;

    // When a side has zero width, it shouldn't own fragments.
    // Fall back to the other side if one width is zero.
    let force_v = v_bw <= 0.0 && h_bw > 0.0;
    let force_h = h_bw <= 0.0 && v_bw > 0.0;

    let use_horizontal = (h_wins || force_h) && !force_v;

    if use_horizontal {
        return select(bottom, top, in_top);
    }
    return select(right, left, in_left);
}
