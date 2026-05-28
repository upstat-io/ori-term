// Subpixel foreground shader: LCD per-channel alpha blending.
//
// Samples an Rgba8Unorm atlas where R/G/B channels contain independent
// subpixel coverage masks (from swash Format::Subpixel). Each color
// channel is blended independently: mix(bg, fg, mask_channel). This
// achieves ~3x effective horizontal resolution on LCD displays.
// Per-instance clip rect discards out-of-bounds fragments.

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
    @location(1) bg_color: vec4<f32>,
    @location(2) tex_coord: vec2<f32>,
    @location(3) @interpolate(flat) atlas_page: u32,
    @location(4) clip_min: vec2<f32>,
    @location(5) clip_max: vec2<f32>,
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

    // UV from atlas: origin + extent * corner.
    let tex_coord = instance.uv.xy + instance.uv.zw * corner;

    var out: VertexOutput;
    out.position = vec4<f32>(ndc, 0.0, 1.0);
    out.fg_color = instance.fg_color;
    out.bg_color = instance.bg_color;
    out.tex_coord = tex_coord;
    out.atlas_page = instance.atlas_page;
    out.clip_min = instance.clip.xy;
    out.clip_max = instance.clip.xy + instance.clip.zw;
    return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Per-instance clip rect test.
    let frag_pos = input.position.xy;
    if frag_pos.x < input.clip_min.x || frag_pos.x > input.clip_max.x
        || frag_pos.y < input.clip_min.y || frag_pos.y > input.clip_max.y {
        discard;
    }

    // Sample per-channel coverage mask from the subpixel atlas.
    // R/G/B contain independent subpixel coverage (0.0-1.0).
    let mask = textureSample(subpixel_atlas_texture, subpixel_atlas_sampler, input.tex_coord, input.atlas_page);

    let fg = input.fg_color;
    let bg = input.bg_color;

    // Known background — true per-channel LCD compositing.
    //
    // fg.a carries the dim factor (1.0 = normal, ~0.6 = dimmed pane).
    // Scale coverage by dim so dimmed text appears lighter. When dim=0,
    // all mask channels become 0, the zero-coverage guard fires, and the
    // pixel passes through transparent — fully dimmed text is invisible.
    //
    // BlendState is PREMUL_ALPHA_BLEND: src*1 + dst*(1-src_alpha). The cell
    // bg is drawn FIRST as a separate bg quad (record_passes.rs), so the
    // framebuffer already holds bg-over-dest BEFORE this glyph draw. The
    // branch therefore splits on bg.a to avoid double-applying that bg:
    //
    // - OPAQUE bg (bg.a >= 0.999): composite glyph over bg and OVERWRITE
    //   (out_a=1.0). The opaque result REPLACES the framebuffer, so the bg
    //   quad already underneath is harmlessly overwritten — true per-channel
    //   LCD AA, no double-apply. At bg.a=1.0 this is byte-identical to
    //   mix(bg,fg,cov) with alpha 1.0.
    // - TRANSLUCENT bg (0.001 < bg.a < 0.999): emit GLYPH-ONLY premultiplied
    //   — do NOT re-include bg, because the bg is ALREADY in the framebuffer
    //   via the bg quad. Blending glyph-only over that framebuffer yields
    //   glyph-over-bg-over-dest at the correct brightness. Tradeoff: a single
    //   premultiplied draw carries only a SCALAR src_alpha through
    //   PREMUL_ALPHA_BLEND, so per-channel LCD coverage cannot survive the
    //   blend — subpixel AA degrades toward grayscale-equivalent on
    //   translucent cells on this non-dual path. True per-channel AA over a
    //   translucent bg requires DUAL-SOURCE blending (subpixel_fg_dual.wgsl,
    //   used on dual-source-capable hardware — final = src0*src1 +
    //   dst*(1-src1), per-channel; alacritty's SRC1_COLOR approach).
    //   (Decision 09 sub-question 2.)
    if bg.a > 0.001 {
        let dim = fg.a;
        let cov_r = mask.r * dim;
        let cov_g = mask.g * dim;
        let cov_b = mask.b * dim;
        let coverage = max(cov_r, max(cov_g, cov_b));
        if coverage < 0.001 {
            return vec4<f32>(0.0, 0.0, 0.0, 0.0);
        }
        if bg.a >= 0.999 {
            // Opaque bg: composite-and-overwrite (true per-channel LCD AA).
            let out_r = fg.r * cov_r + bg.r * (1.0 - cov_r);
            let out_g = fg.g * cov_g + bg.g * (1.0 - cov_g);
            let out_b = fg.b * cov_b + bg.b * (1.0 - cov_b);
            return vec4<f32>(out_r, out_g, out_b, 1.0);
        }
        // Translucent bg: glyph-only premultiplied (bg comes from the bg quad
        // already in the framebuffer; AA degrades to grayscale-equivalent).
        let out_r = fg.r * cov_r;
        let out_g = fg.g * cov_g;
        let out_b = fg.b * cov_b;
        return vec4<f32>(out_r, out_g, out_b, coverage);
    }

    // Unknown background — grayscale alpha fallback.
    // Without a real background hint, preserving independent RGB coverage
    // produces visible color fringing on non-default cell backgrounds.
    // Collapse the subpixel mask to a single coverage value so the glyph
    // blends like standard grayscale text over whatever background is
    // already in the framebuffer.
    //
    // BlendState: premultiplied vec4(fg.rgb*a, a) blends correctly over
    // whatever is in the framebuffer.
    let coverage = max(mask.r, max(mask.g, mask.b));
    let a = coverage * fg.a;
    return vec4<f32>(fg.rgb * a, a);
}
