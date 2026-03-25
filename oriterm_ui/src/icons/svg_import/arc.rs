//! SVG arc-to-cubic Bézier conversion (SVG spec Appendix F).

use std::f32::consts::PI;

use crate::icons::PathCommand;

/// Convert an SVG arc to cubic Bézier segments.
///
/// Uses the endpoint-to-center parameterization from SVG spec Appendix F,
/// then subdivides into ≤90° segments approximated as cubics.
#[expect(clippy::too_many_arguments, reason = "SVG arc has 7 parameters")]
pub fn arc_to_cubics(
    x1: f32,
    y1: f32,
    mut rx: f32,
    mut ry: f32,
    x_rotation_deg: f32,
    large_arc: bool,
    sweep: bool,
    x2: f32,
    y2: f32,
    vb: f32,
    cmds: &mut Vec<PathCommand>,
) {
    // Degenerate: same start/end.
    if (x1 - x2).abs() < 1e-6 && (y1 - y2).abs() < 1e-6 {
        return;
    }
    // Degenerate: zero radius → line.
    if rx.abs() < 1e-6 || ry.abs() < 1e-6 {
        cmds.push(PathCommand::LineTo(x2 / vb, y2 / vb));
        return;
    }

    rx = rx.abs();
    ry = ry.abs();

    let phi = x_rotation_deg * PI / 180.0;
    let cos_phi = phi.cos();
    let sin_phi = phi.sin();

    // Step 1: Compute (x1', y1') — rotated midpoint.
    let dx = (x1 - x2) / 2.0;
    let dy = (y1 - y2) / 2.0;
    let x1p = cos_phi * dx + sin_phi * dy;
    let y1p = -sin_phi * dx + cos_phi * dy;

    // Step 2: Scale radii if needed.
    let x1p2 = x1p * x1p;
    let y1p2 = y1p * y1p;
    let rx2 = rx * rx;
    let ry2 = ry * ry;

    let lambda = x1p2 / rx2 + y1p2 / ry2;
    if lambda > 1.0 {
        let sqrt_lambda = lambda.sqrt();
        rx *= sqrt_lambda;
        ry *= sqrt_lambda;
    }

    let rx2 = rx * rx;
    let ry2 = ry * ry;

    // Step 3: Compute center point.
    let numer = (rx2 * ry2 - rx2 * y1p2 - ry2 * x1p2).max(0.0);
    let denom = rx2 * y1p2 + ry2 * x1p2;
    let sq = if denom > 1e-10 {
        (numer / denom).sqrt()
    } else {
        0.0
    };

    let sign = if large_arc == sweep { -1.0 } else { 1.0 };
    let cxp = sign * sq * (rx * y1p / ry);
    let cyp = sign * sq * -(ry * x1p / rx);

    // Un-rotate center.
    let ccx = cos_phi * cxp - sin_phi * cyp + f32::midpoint(x1, x2);
    let ccy = sin_phi * cxp + cos_phi * cyp + f32::midpoint(y1, y2);

    // Step 4: Compute angles.
    let theta1 = vector_angle(1.0, 0.0, (x1p - cxp) / rx, (y1p - cyp) / ry);
    let mut dtheta = vector_angle(
        (x1p - cxp) / rx,
        (y1p - cyp) / ry,
        (-x1p - cxp) / rx,
        (-y1p - cyp) / ry,
    );

    if !sweep && dtheta > 0.0 {
        dtheta -= 2.0 * PI;
    } else if sweep && dtheta < 0.0 {
        dtheta += 2.0 * PI;
    } else {
        // Both conditions false: dtheta sign already matches sweep direction.
    }

    // Step 5: Subdivide into segments ≤ 90° and emit cubics.
    let n_segs = (dtheta.abs() / (PI / 2.0)).ceil() as usize;
    let seg_angle = dtheta / n_segs as f32;

    for seg in 0..n_segs {
        let t1 = theta1 + seg as f32 * seg_angle;
        let t2 = t1 + seg_angle;
        emit_segment(ccx, ccy, rx, ry, cos_phi, sin_phi, t1, t2, vb, cmds);
    }
}

/// Emit one cubic Bézier for an arc segment from angle `t1` to `t2`.
#[expect(clippy::too_many_arguments, reason = "arc segment geometry")]
fn emit_segment(
    cx: f32,
    cy: f32,
    rx: f32,
    ry: f32,
    cos_phi: f32,
    sin_phi: f32,
    t1: f32,
    t2: f32,
    vb: f32,
    cmds: &mut Vec<PathCommand>,
) {
    let alpha = 4.0 / 3.0 * ((t2 - t1) / 4.0).tan();

    let cos_t1 = t1.cos();
    let sin_t1 = t1.sin();
    let cos_t2 = t2.cos();
    let sin_t2 = t2.sin();

    let p1x = rx * cos_t1;
    let p1y = ry * sin_t1;
    let p2x = rx * cos_t2;
    let p2y = ry * sin_t2;

    let q1x = p1x - alpha * rx * sin_t1;
    let q1y = p1y + alpha * ry * cos_t1;
    let q2x = p2x + alpha * rx * sin_t2;
    let q2y = p2y - alpha * ry * cos_t2;

    let transform = |px: f32, py: f32| -> (f32, f32) {
        (
            (cos_phi * px - sin_phi * py + cx) / vb,
            (sin_phi * px + cos_phi * py + cy) / vb,
        )
    };

    let (cx1, cy1) = transform(q1x, q1y);
    let (cx2, cy2) = transform(q2x, q2y);
    let (ex, ey) = transform(p2x, p2y);

    cmds.push(PathCommand::CubicTo(cx1, cy1, cx2, cy2, ex, ey));
}

/// Angle between two vectors.
fn vector_angle(ux: f32, uy: f32, vx: f32, vy: f32) -> f32 {
    let dot = ux * vx + uy * vy;
    let cross = ux * vy - uy * vx;
    let len = ux.hypot(uy) * vx.hypot(vy);
    if len < 1e-10 {
        return 0.0;
    }
    let cos_a = (dot / len).clamp(-1.0, 1.0);
    let a = cos_a.acos();
    if cross < 0.0 { -a } else { a }
}
