//! CPU compositing of COLR v1 paint commands into RGBA bitmaps.
//!
//! Rasterizes each glyph outline via swash and composites the resulting
//! alpha mask with the resolved brush color onto the output buffer using
//! `SrcOver` blending.

use swash::FontRef;
use swash::scale::{Render, ScaleContext, Source};
use swash::zeno::Format;

use super::{ClipBox, PaintCommand, ResolvedBrush, ResolvedColorStop, Rgba};
use crate::font::collection::{FaceData, font_ref};

/// Composite all paint commands onto the RGBA bitmap.
///
/// Commands are replayed in order. `FillGlyph` commands rasterize the
/// glyph outline as an alpha mask and composite with the brush color.
/// Transform and clip commands modify the rasterization context.
#[expect(
    clippy::too_many_arguments,
    reason = "compositing context: bitmap, dimensions, font data, scale context"
)]
pub(super) fn composite_commands(
    commands: &[PaintCommand],
    bitmap: &mut [u8],
    width: u32,
    height: u32,
    clip: ClipBox,
    fd: &FaceData,
    size_px: f32,
    variations: &[(&str, f32)],
    ctx: &mut ScaleContext,
) {
    for cmd in commands {
        match cmd {
            PaintCommand::FillGlyph {
                glyph_id, brush, ..
            } => {
                let gid = glyph_id.to_u32() as u16;
                composite_fill_glyph(
                    bitmap, width, height, clip, fd, gid, size_px, variations, brush, ctx,
                );
            }
            PaintCommand::Fill(brush) => {
                // Fill the entire clip area with the brush (no glyph mask).
                fill_rect(bitmap, width, height, brush);
            }
            // Transform, clip, and layer commands are not yet handled in the
            // CPU path. Most emoji only use FillGlyph + Solid, so this covers
            // the common case. Complex compositions will render with slight
            // artifacts until GPU render-to-texture is added.
            PaintCommand::PushTransform(_)
            | PaintCommand::PopTransform
            | PaintCommand::PushClipGlyph(_)
            | PaintCommand::PushClipBox(_)
            | PaintCommand::PopClip
            | PaintCommand::PushLayer(_)
            | PaintCommand::PopLayer => {}
        }
    }
}

/// Rasterize a glyph outline and composite with the brush onto the bitmap.
#[expect(
    clippy::too_many_arguments,
    reason = "rasterization parameters for a single glyph layer"
)]
fn composite_fill_glyph(
    bitmap: &mut [u8],
    bmp_w: u32,
    bmp_h: u32,
    clip: ClipBox,
    fd: &FaceData,
    glyph_id: u16,
    size_px: f32,
    variations: &[(&str, f32)],
    brush: &ResolvedBrush,
    ctx: &mut ScaleContext,
) {
    let fr = font_ref(fd);

    // Rasterize just the outline (no color sources).
    let mask = rasterize_outline(&fr, glyph_id, size_px, variations, ctx);
    let Some(mask) = mask else { return };

    // Position the mask within the bitmap based on bearings and clip origin.
    let mask_x = mask.placement.left as f32 - clip.x_min;
    let mask_y = clip.y_max - mask.placement.top as f32;

    let mx = mask_x.round() as i32;
    let my = mask_y.round() as i32;

    // Composite each mask pixel with the brush color.
    for row in 0..mask.placement.height {
        for col in 0..mask.placement.width {
            let bx = mx + col as i32;
            let by = my + row as i32;
            if bx < 0 || by < 0 || bx >= bmp_w as i32 || by >= bmp_h as i32 {
                continue;
            }
            let mask_idx = (row * mask.placement.width + col) as usize;
            let alpha = mask.data[mask_idx] as f32 / 255.0;
            if alpha < 1.0 / 255.0 {
                continue;
            }

            let color = brush_color_at(brush, bx as f32, by as f32);
            let src = Rgba {
                r: color.r * alpha,
                g: color.g * alpha,
                b: color.b * alpha,
                a: color.a * alpha,
            };

            blend_src_over(bitmap, bmp_w, bx as u32, by as u32, src);
        }
    }
}

/// Fill the entire bitmap with a brush (no glyph mask).
fn fill_rect(bitmap: &mut [u8], width: u32, height: u32, brush: &ResolvedBrush) {
    for y in 0..height {
        for x in 0..width {
            let color = brush_color_at(brush, x as f32, y as f32);
            blend_src_over(bitmap, width, x, y, color);
        }
    }
}

/// Rasterize a glyph outline as an alpha mask via swash.
fn rasterize_outline(
    fr: &FontRef<'_>,
    glyph_id: u16,
    size_px: f32,
    variations: &[(&str, f32)],
    ctx: &mut ScaleContext,
) -> Option<swash::scale::image::Image> {
    let builder = ctx.builder(*fr).size(size_px).hint(true);
    let mut scaler = if variations.is_empty() {
        builder.build()
    } else {
        builder.variations(variations).build()
    };

    let mut render = Render::new(&[Source::Outline]);
    render.format(Format::Alpha);
    render.render(&mut scaler, glyph_id)
}

/// Get the brush color at a given pixel position.
///
/// For solid brushes, returns the single color. For gradients, evaluates
/// the gradient at the given position.
fn brush_color_at(brush: &ResolvedBrush, px: f32, py: f32) -> Rgba {
    match brush {
        ResolvedBrush::Solid(color) => *color,
        ResolvedBrush::LinearGradient {
            p0,
            p1,
            stops,
            extend,
            ..
        } => {
            let param = linear_gradient_t(*p0, *p1, px, py);
            let param = apply_extend(param, *extend);
            sample_stops(stops, param)
        }
        ResolvedBrush::RadialGradient {
            c0,
            r0,
            c1,
            r1,
            stops,
            extend,
        } => {
            let param = radial_gradient_t(*c0, *r0, *c1, *r1, px, py);
            let param = apply_extend(param, *extend);
            sample_stops(stops, param)
        }
        ResolvedBrush::SweepGradient {
            center,
            start_angle,
            end_angle,
            stops,
            extend,
        } => {
            let param = sweep_gradient_t(*center, *start_angle, *end_angle, px, py);
            let param = apply_extend(param, *extend);
            sample_stops(stops, param)
        }
    }
}

/// `SrcOver` blend: composite `src` onto `dst` in the bitmap.
fn blend_src_over(bitmap: &mut [u8], width: u32, bx: u32, by: u32, src: Rgba) {
    let idx = ((by * width + bx) * 4) as usize;
    let dst_r = bitmap[idx] as f32 / 255.0;
    let dst_g = bitmap[idx + 1] as f32 / 255.0;
    let dst_b = bitmap[idx + 2] as f32 / 255.0;
    let dst_a = bitmap[idx + 3] as f32 / 255.0;

    let inv_sa = 1.0 - src.a;
    let out_r = src.r + dst_r * inv_sa;
    let out_g = src.g + dst_g * inv_sa;
    let out_b = src.b + dst_b * inv_sa;
    let out_a = src.a + dst_a * inv_sa;

    bitmap[idx] = (out_r * 255.0).round().clamp(0.0, 255.0) as u8;
    bitmap[idx + 1] = (out_g * 255.0).round().clamp(0.0, 255.0) as u8;
    bitmap[idx + 2] = (out_b * 255.0).round().clamp(0.0, 255.0) as u8;
    bitmap[idx + 3] = (out_a * 255.0).round().clamp(0.0, 255.0) as u8;
}

// ── Gradient evaluation ──

/// Linear gradient parameter at point `(px, py)`.
fn linear_gradient_t(p0: [f32; 2], p1: [f32; 2], px: f32, py: f32) -> f32 {
    let dx = p1[0] - p0[0];
    let dy = p1[1] - p0[1];
    let len_sq = dx * dx + dy * dy;
    if len_sq < 0.0001 {
        return 0.0;
    }
    ((px - p0[0]) * dx + (py - p0[1]) * dy) / len_sq
}

/// Radial gradient parameter at point `(px, py)`.
#[expect(
    clippy::too_many_arguments,
    reason = "radial gradient defined by two circles (center + radius each) plus eval point"
)]
fn radial_gradient_t(
    center0: [f32; 2],
    radius0: f32,
    center1: [f32; 2],
    radius1: f32,
    px: f32,
    py: f32,
) -> f32 {
    let dcx = center1[0] - center0[0];
    let dcy = center1[1] - center0[1];
    let dr = radius1 - radius0;
    let dpx = px - center0[0];
    let dpy = py - center0[1];

    let coeff_a = dcx * dcx + dcy * dcy - dr * dr;
    let coeff_b = 2.0 * (dpx * dcx + dpy * dcy - radius0 * dr);
    let coeff_c = dpx * dpx + dpy * dpy - radius0 * radius0;

    let disc = coeff_b * coeff_b - 4.0 * coeff_a * coeff_c;
    if disc < 0.0 {
        return 0.0;
    }
    let sqrt_disc = disc.sqrt();
    if coeff_a.abs() < 0.0001 {
        if coeff_b.abs() < 0.0001 {
            return 0.0;
        }
        return -coeff_c / coeff_b;
    }
    let t1 = (-coeff_b + sqrt_disc) / (2.0 * coeff_a);
    let t2 = (-coeff_b - sqrt_disc) / (2.0 * coeff_a);
    let t_max = t1.max(t2);
    let t_min = t1.min(t2);
    if radius0 + t_max * dr >= 0.0 {
        t_max
    } else {
        t_min
    }
}

/// Sweep gradient parameter at point `(px, py)`.
fn sweep_gradient_t(center: [f32; 2], start_angle: f32, end_angle: f32, px: f32, py: f32) -> f32 {
    let dx = px - center[0];
    let dy = -(py - center[1]); // flip Y for screen coords
    let mut angle = dy.atan2(dx).to_degrees();
    if angle < 0.0 {
        angle += 360.0;
    }
    let range = end_angle - start_angle;
    if range.abs() < 0.0001 {
        return 0.0;
    }
    (angle - start_angle) / range
}

/// Apply extend mode (pad, repeat, reflect) to a gradient parameter.
fn apply_extend(t: f32, extend: skrifa::color::Extend) -> f32 {
    use skrifa::color::Extend;
    match extend {
        Extend::Repeat => t - t.floor(),
        Extend::Reflect => {
            let period = t - 2.0 * (t * 0.5).floor();
            if period > 1.0 { 2.0 - period } else { period }
        }
        // Pad + any future variants added to this non-exhaustive enum.
        _ => t.clamp(0.0, 1.0),
    }
}

/// Sample the color stop array at parameter t.
fn sample_stops(stops: &[ResolvedColorStop], t: f32) -> Rgba {
    if stops.is_empty() {
        return Rgba {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        };
    }
    if stops.len() == 1 || t <= stops[0].offset {
        return stops[0].color;
    }
    for window in stops.windows(2) {
        let prev = &window[0];
        let curr = &window[1];
        if t <= curr.offset {
            let range = curr.offset - prev.offset;
            let frac = if range > 0.0001 {
                (t - prev.offset) / range
            } else {
                0.0
            };
            return lerp_rgba(prev.color, curr.color, frac);
        }
    }
    stops[stops.len() - 1].color
}

/// Linear interpolation between two RGBA colors.
fn lerp_rgba(a: Rgba, b: Rgba, t: f32) -> Rgba {
    Rgba {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a + (b.a - a.a) * t,
    }
}

#[cfg(test)]
mod tests;
