//! Brush and paint conversion for COLR v1 compositing.
//!
//! Converts resolved brushes (solid, linear/radial/sweep gradients) to
//! tiny-skia paint objects, and maps COLR composite modes to tiny-skia
//! blend modes. Pure conversion — no pixmap or mask mutation.

use skrifa::color::{CompositeMode, Extend, Transform as ColrTransform};

use super::super::{ClipBox, ResolvedBrush, ResolvedColorStop, Rgba};
use super::{ComposeCtx, to_bx, to_by};

/// Convert a resolved brush to a tiny-skia `Paint`.
pub(super) fn make_paint(
    brush: &ResolvedBrush,
    ctx: &ComposeCtx<'_>,
    brush_xf: Option<&ColrTransform>,
) -> Option<tiny_skia::Paint<'static>> {
    let t = if let Some(bt) = brush_xf {
        *ctx.transform() * *bt
    } else {
        *ctx.transform()
    };

    let shader = match brush {
        ResolvedBrush::Solid(rgba) => tiny_skia::Shader::SolidColor(rgba_to_color(rgba)),
        ResolvedBrush::LinearGradient {
            p0,
            p1,
            stops,
            extend,
        } => {
            let start = pt(p0[0], p0[1], ctx.scale(), ctx.clip(), &t);
            let end = pt(p1[0], p1[1], ctx.scale(), ctx.clip(), &t);
            let gs = to_grad_stops(stops)?;
            tiny_skia::LinearGradient::new(
                start,
                end,
                gs,
                to_spread(*extend),
                tiny_skia::Transform::identity(),
            )?
        }
        ResolvedBrush::RadialGradient {
            c0,
            r0,
            c1,
            r1,
            stops,
            extend,
        } => {
            // tiny-skia RadialGradient: (focal_point, center, radius).
            // Map COLR two-circle model: c0/r0 = focal, c1/r1 = enclosing.
            // tiny-skia always uses a point focal — log when r0 > 0.
            if *r0 > 0.0 {
                log::trace!("radial gradient start radius {r0} approximated as point focal");
            }
            let focal = pt(c0[0], c0[1], ctx.scale(), ctx.clip(), &t);
            let center = pt(c1[0], c1[1], ctx.scale(), ctx.clip(), &t);
            let gs = to_grad_stops(stops)?;
            tiny_skia::RadialGradient::new(
                focal,
                center,
                r1 * ctx.scale(),
                gs,
                to_spread(*extend),
                tiny_skia::Transform::identity(),
            )?
        }
        ResolvedBrush::SweepGradient { .. } => {
            // Sweep gradients are handled by fill_sweep_direct() in the
            // compositor, not through the shader system. Return None here
            // to signal the caller to use the direct path.
            return None;
        }
    };

    Some(tiny_skia::Paint {
        anti_alias: true,
        shader,
        ..tiny_skia::Paint::default()
    })
}

/// Transform a font-unit point to a tiny-skia Point in bitmap coordinates.
fn pt(fx: f32, fy: f32, scale: f32, clip: &ClipBox, t: &ColrTransform) -> tiny_skia::Point {
    tiny_skia::Point::from_xy(to_bx(fx, fy, scale, clip, t), to_by(fx, fy, scale, clip, t))
}

/// Un-premultiply and convert to tiny-skia `Color`.
///
/// Clamps un-premultiplied components to `0.0..=1.0` because
/// `tiny_skia::Color::from_rgba` rejects out-of-range values entirely
/// (returns `None`), and floating-point arithmetic can produce values
/// like `1.0000001` from valid premultiplied input.
pub(super) fn rgba_to_color(c: &Rgba) -> tiny_skia::Color {
    if c.a > 0.0 {
        let inv = 1.0 / c.a;
        tiny_skia::Color::from_rgba(
            (c.r * inv).clamp(0.0, 1.0),
            (c.g * inv).clamp(0.0, 1.0),
            (c.b * inv).clamp(0.0, 1.0),
            c.a.clamp(0.0, 1.0),
        )
        .unwrap_or(tiny_skia::Color::TRANSPARENT)
    } else {
        tiny_skia::Color::TRANSPARENT
    }
}

/// Convert resolved gradient stops to tiny-skia stops.
fn to_grad_stops(stops: &[ResolvedColorStop]) -> Option<Vec<tiny_skia::GradientStop>> {
    let v: Vec<_> = stops
        .iter()
        .map(|s| tiny_skia::GradientStop::new(s.offset.clamp(0.0, 1.0), rgba_to_color(&s.color)))
        .collect();
    // tiny-skia requires at least 2 stops.
    if v.len() >= 2 { Some(v) } else { None }
}

/// Fill a pixmap with a sweep (conic) gradient directly, pixel by pixel.
///
/// Sweep gradients map angle from `center` to stop offsets. COLR defines
/// `start_angle` and `end_angle` in degrees (counter-clockwise from
/// 3-o'clock). Respects the optional mask.
#[expect(
    clippy::many_single_char_names,
    reason = "RGBA components and loop vars"
)]
#[expect(
    clippy::match_same_arms,
    reason = "extend modes listed explicitly for clarity"
)]
pub(super) fn fill_sweep_direct(
    pixmap: &mut tiny_skia::Pixmap,
    cx: f32,
    cy: f32,
    start_angle: f32,
    end_angle: f32,
    stops: &[ResolvedColorStop],
    extend: Extend,
    mask: Option<&tiny_skia::Mask>,
) {
    if stops.is_empty() {
        return;
    }
    let width = pixmap.width();
    let height = pixmap.height();
    let start_rad = start_angle.to_radians();
    let end_rad = end_angle.to_radians();
    let arc = end_rad - start_rad;

    for y in 0..height {
        for x in 0..width {
            // Check mask.
            if let Some(m) = mask {
                let mi = (y * width + x) as usize;
                if mi < m.data().len() && m.data()[mi] == 0 {
                    continue;
                }
            }

            let dx = x as f32 - cx;
            let dy = cy - y as f32; // Y-up in font coordinate space.
            let angle = dy.atan2(dx); // -PI..PI, 0 = right (3 o'clock).

            let t = if arc.abs() < 1e-6 {
                0.0
            } else {
                let raw = (angle - start_rad) / arc;
                match extend {
                    Extend::Pad => raw.clamp(0.0, 1.0),
                    Extend::Repeat => raw.rem_euclid(1.0),
                    Extend::Reflect => {
                        let t2 = raw.rem_euclid(2.0);
                        if t2 > 1.0 { 2.0 - t2 } else { t2 }
                    }
                    _ => raw.clamp(0.0, 1.0),
                }
            };

            let color = sample_stops(stops, t);
            // Premultiply and write.
            let a = (color.a * 255.0) as u8;
            let r = (color.r * color.a * 255.0) as u8;
            let g = (color.g * color.a * 255.0) as u8;
            let b = (color.b * color.a * 255.0) as u8;

            // Apply mask alpha.
            let (r, g, b, a) = if let Some(m) = mask {
                let mi = (y * width + x) as usize;
                let ma = if mi < m.data().len() {
                    m.data()[mi]
                } else {
                    255
                };
                (
                    ((r as u16 * ma as u16) / 255) as u8,
                    ((g as u16 * ma as u16) / 255) as u8,
                    ((b as u16 * ma as u16) / 255) as u8,
                    ((a as u16 * ma as u16) / 255) as u8,
                )
            } else {
                (r, g, b, a)
            };

            let idx = ((y * width + x) * 4) as usize;
            let data = pixmap.data_mut();
            // Composite over existing content (SrcOver).
            let dst_r = data[idx];
            let dst_g = data[idx + 1];
            let dst_b = data[idx + 2];
            let dst_a = data[idx + 3];
            let inv_a = 255 - a;
            data[idx] = r.saturating_add(((dst_r as u16 * inv_a as u16) / 255) as u8);
            data[idx + 1] = g.saturating_add(((dst_g as u16 * inv_a as u16) / 255) as u8);
            data[idx + 2] = b.saturating_add(((dst_b as u16 * inv_a as u16) / 255) as u8);
            data[idx + 3] = a.saturating_add(((dst_a as u16 * inv_a as u16) / 255) as u8);
        }
    }
}

/// Sample a color from gradient stops at position `t` (0..1).
fn sample_stops(stops: &[ResolvedColorStop], t: f32) -> Rgba {
    if stops.len() == 1 || t <= stops[0].offset {
        return stops[0].color;
    }
    let last = stops.len() - 1;
    if t >= stops[last].offset {
        return stops[last].color;
    }
    for w in stops.windows(2) {
        if t <= w[1].offset {
            let range = w[1].offset - w[0].offset;
            let frac = if range > 1e-6 {
                (t - w[0].offset) / range
            } else {
                0.0
            };
            let a = &w[0].color;
            let b = &w[1].color;
            return Rgba {
                r: a.r + (b.r - a.r) * frac,
                g: a.g + (b.g - a.g) * frac,
                b: a.b + (b.b - a.b) * frac,
                a: a.a + (b.a - a.a) * frac,
            };
        }
    }
    stops[last].color
}

/// Map COLR extend mode to tiny-skia spread mode.
fn to_spread(extend: Extend) -> tiny_skia::SpreadMode {
    match extend {
        Extend::Repeat => tiny_skia::SpreadMode::Repeat,
        Extend::Reflect => tiny_skia::SpreadMode::Reflect,
        _ => tiny_skia::SpreadMode::Pad,
    }
}

/// Map COLR composite mode to tiny-skia blend mode.
pub(super) fn to_blend_mode(mode: CompositeMode) -> tiny_skia::BlendMode {
    match mode {
        CompositeMode::Clear => tiny_skia::BlendMode::Clear,
        CompositeMode::Src => tiny_skia::BlendMode::Source,
        CompositeMode::Dest => tiny_skia::BlendMode::Destination,
        CompositeMode::DestOver => tiny_skia::BlendMode::DestinationOver,
        CompositeMode::SrcIn => tiny_skia::BlendMode::SourceIn,
        CompositeMode::DestIn => tiny_skia::BlendMode::DestinationIn,
        CompositeMode::SrcOut => tiny_skia::BlendMode::SourceOut,
        CompositeMode::DestOut => tiny_skia::BlendMode::DestinationOut,
        CompositeMode::SrcAtop => tiny_skia::BlendMode::SourceAtop,
        CompositeMode::DestAtop => tiny_skia::BlendMode::DestinationAtop,
        CompositeMode::Xor => tiny_skia::BlendMode::Xor,
        CompositeMode::Plus => tiny_skia::BlendMode::Plus,
        CompositeMode::Screen => tiny_skia::BlendMode::Screen,
        CompositeMode::Overlay => tiny_skia::BlendMode::Overlay,
        CompositeMode::Darken => tiny_skia::BlendMode::Darken,
        CompositeMode::Lighten => tiny_skia::BlendMode::Lighten,
        CompositeMode::ColorDodge => tiny_skia::BlendMode::ColorDodge,
        CompositeMode::ColorBurn => tiny_skia::BlendMode::ColorBurn,
        CompositeMode::HardLight => tiny_skia::BlendMode::HardLight,
        CompositeMode::SoftLight => tiny_skia::BlendMode::SoftLight,
        CompositeMode::Difference => tiny_skia::BlendMode::Difference,
        CompositeMode::Exclusion => tiny_skia::BlendMode::Exclusion,
        CompositeMode::Multiply => tiny_skia::BlendMode::Multiply,
        CompositeMode::HslHue => tiny_skia::BlendMode::Hue,
        CompositeMode::HslSaturation => tiny_skia::BlendMode::Saturation,
        CompositeMode::HslColor => tiny_skia::BlendMode::Color,
        CompositeMode::HslLuminosity => tiny_skia::BlendMode::Luminosity,
        _ => tiny_skia::BlendMode::SourceOver,
    }
}
