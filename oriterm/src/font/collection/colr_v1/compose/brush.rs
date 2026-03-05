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
        ctx.xf * *bt
    } else {
        ctx.xf
    };

    let shader = match brush {
        ResolvedBrush::Solid(rgba) => tiny_skia::Shader::SolidColor(rgba_to_color(rgba)),
        ResolvedBrush::LinearGradient {
            p0,
            p1,
            stops,
            extend,
        } => {
            let start = pt(p0[0], p0[1], ctx.scale, ctx.clip, &t);
            let end = pt(p1[0], p1[1], ctx.scale, ctx.clip, &t);
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
            let focal = pt(c0[0], c0[1], ctx.scale, ctx.clip, &t);
            let center = pt(c1[0], c1[1], ctx.scale, ctx.clip, &t);
            let gs = to_grad_stops(stops)?;
            tiny_skia::RadialGradient::new(
                focal,
                center,
                r1 * ctx.scale,
                gs,
                to_spread(*extend),
                tiny_skia::Transform::identity(),
            )?
        }
        ResolvedBrush::SweepGradient {
            center,
            start_angle,
            end_angle,
            stops,
            extend,
        } => {
            // tiny-skia 0.11 has no SweepGradient. Sample the midpoint color
            // of the gradient as an approximation.
            log::trace!(
                "sweep gradient ({start_angle}°..{end_angle}°) at ({},{}) \
                 extend={extend:?}: approximated as solid",
                center[0],
                center[1],
            );
            let color = sweep_midpoint_color(stops);
            tiny_skia::Shader::SolidColor(color)
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

/// Sample the midpoint color of gradient stops for sweep gradient fallback.
fn sweep_midpoint_color(stops: &[ResolvedColorStop]) -> tiny_skia::Color {
    if stops.is_empty() {
        return tiny_skia::Color::TRANSPARENT;
    }
    if stops.len() == 1 {
        return rgba_to_color(&stops[0].color);
    }
    // Find the pair of stops around t=0.5 and interpolate.
    for w in stops.windows(2) {
        if 0.5 <= w[1].offset {
            let range = w[1].offset - w[0].offset;
            let frac = if range > 0.0001 {
                (0.5 - w[0].offset) / range
            } else {
                0.0
            };
            let a = &w[0].color;
            let b = &w[1].color;
            let mid = Rgba {
                r: a.r + (b.r - a.r) * frac,
                g: a.g + (b.g - a.g) * frac,
                b: a.b + (b.b - a.b) * frac,
                a: a.a + (b.a - a.a) * frac,
            };
            return rgba_to_color(&mid);
        }
    }
    rgba_to_color(&stops.last().unwrap().color)
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
