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
        ResolvedBrush::RadialGradient { r0, .. } if *r0 > 0.0 => {
            // Two-circle radial gradients (r0 > 0) are handled by
            // fill_radial_direct() — tiny-skia only supports point-focal.
            return None;
        }
        ResolvedBrush::RadialGradient {
            c0,
            c1,
            r1,
            stops,
            extend,
            ..
        } => {
            // Point-focal radial gradient (r0 == 0): tiny-skia handles this.
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
/// 3-o'clock). Pixel angles are normalized to `[0°, 360°)` to avoid the
/// `atan2` discontinuity at ±180°. Respects the optional mask.
///
/// Color stops are already premultiplied RGBA — no additional alpha
/// multiplication is applied when writing pixels.
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
    // Work in degrees throughout to match COLR spec convention.
    // Do NOT normalize start/end — COLR 1.9.1 distinguishes 330°→400°
    // from 330°→40°.
    let arc = end_angle - start_angle;

    for y in 0..height {
        for x in 0..width {
            // Check mask (early exit for fully masked pixels).
            if let Some(m) = mask {
                let mi = (y * width + x) as usize;
                if mi < m.data().len() && m.data()[mi] == 0 {
                    continue;
                }
            }

            let dx = x as f32 - cx;
            let dy = cy - y as f32; // Y-up in font coordinate space.
            // Normalize pixel angle to [0°, 360°) — avoids the atan2
            // discontinuity at ±180° that caused gradient seam artifacts.
            let angle = dy.atan2(dx).to_degrees().rem_euclid(360.0);

            let t = if arc.abs() < 1e-6 {
                0.0
            } else {
                // Compute angular distance from start in the sweep direction.
                // For CCW (arc > 0): diff in [0, 360).
                // For CW (arc < 0): diff in (-360, 0].
                let diff = if arc > 0.0 {
                    (angle - start_angle).rem_euclid(360.0)
                } else {
                    (angle - start_angle).rem_euclid(360.0) - 360.0
                };
                let raw = diff / arc;
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
            // Color stops are already premultiplied — convert directly to u8.
            let a = (color.a * 255.0) as u8;
            let r = (color.r * 255.0) as u8;
            let g = (color.g * 255.0) as u8;
            let b = (color.b * 255.0) as u8;

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

/// Fill a pixmap with a two-circle radial gradient directly, pixel by pixel.
///
/// Implements the COLR v1 / CSS two-circle radial gradient model where the
/// gradient is defined by two circles `(c0, r0)` and `(c1, r1)`. For each
/// pixel, solves for the parameter `t` such that the pixel lies on the circle
/// `center(t) = lerp(c0, c1, t)` with `radius(t) = lerp(r0, r1, t)`.
///
/// All center coordinates are in bitmap pixel space (already transformed by
/// the caller). Radii are scaled by `scale` (`size_px` / upem).
#[expect(
    clippy::many_single_char_names,
    reason = "RGBA components, quadratic coefficients, and loop vars"
)]
#[expect(
    clippy::match_same_arms,
    reason = "extend modes listed explicitly for clarity"
)]
pub(super) fn fill_radial_direct(
    pixmap: &mut tiny_skia::Pixmap,
    cx0: f32,
    cy0: f32,
    r0: f32,
    cx1: f32,
    cy1: f32,
    r1: f32,
    stops: &[ResolvedColorStop],
    extend: Extend,
    mask: Option<&tiny_skia::Mask>,
) {
    if stops.is_empty() {
        return;
    }
    let width = pixmap.width();
    let height = pixmap.height();
    let dx = cx1 - cx0;
    let dy = cy1 - cy0;
    let dr = r1 - r0;
    // Quadratic coefficient A is constant for all pixels.
    let a_coeff = dx * dx + dy * dy - dr * dr;

    for y in 0..height {
        for x in 0..width {
            if let Some(m) = mask {
                let mi = (y * width + x) as usize;
                if mi < m.data().len() && m.data()[mi] == 0 {
                    continue;
                }
            }

            let px = x as f32;
            // Bitmap Y is top-down, but our centers are already in bitmap
            // space (Y-flipped by the caller via to_bx/to_by), so use py
            // directly without flipping.
            let py = y as f32;
            let sx = px - cx0;
            let sy = py - cy0;

            let b_coeff = -2.0 * (sx * dx + sy * dy + r0 * dr);
            let c_coeff = sx * sx + sy * sy - r0 * r0;

            // Solve At² + Bt + C = 0 for t, pick the largest t with r(t) > 0.
            let t_val = solve_radial_t(a_coeff, b_coeff, c_coeff, r0, dr);
            let Some(raw_t) = t_val else {
                continue;
            };

            let t = match extend {
                Extend::Pad => raw_t.clamp(0.0, 1.0),
                Extend::Repeat => raw_t.rem_euclid(1.0),
                Extend::Reflect => {
                    let u = raw_t.rem_euclid(2.0);
                    if u > 1.0 { 2.0 - u } else { u }
                }
                _ => raw_t.clamp(0.0, 1.0),
            };

            let color = sample_stops(stops, t);
            // Color stops are already premultiplied — convert directly to u8.
            let a = (color.a * 255.0) as u8;
            let r = (color.r * 255.0) as u8;
            let g = (color.g * 255.0) as u8;
            let b = (color.b * 255.0) as u8;

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

/// Solve the two-circle radial gradient quadratic for the largest valid `t`.
///
/// Returns the largest `t` where `r(t) = r0 + dr * t > 0`, or `None` if
/// no valid solution exists.
fn solve_radial_t(a: f32, b: f32, c: f32, r0: f32, dr: f32) -> Option<f32> {
    if a.abs() < 1e-10 {
        // Linear case: Bt + C = 0.
        if b.abs() < 1e-10 {
            return None;
        }
        let t = -c / b;
        return if r0 + dr * t > 0.0 { Some(t) } else { None };
    }

    let disc = b * b - 4.0 * a * c;
    if disc < 0.0 {
        return None;
    }

    let sqrt_disc = disc.sqrt();
    let t1 = (-b + sqrt_disc) / (2.0 * a);
    let t2 = (-b - sqrt_disc) / (2.0 * a);

    // Pick the larger t with r(t) > 0 (per COLR spec: later circles win).
    let (hi, lo) = if t1 > t2 { (t1, t2) } else { (t2, t1) };
    if r0 + dr * hi > 0.0 {
        Some(hi)
    } else if r0 + dr * lo > 0.0 {
        Some(lo)
    } else {
        None
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
