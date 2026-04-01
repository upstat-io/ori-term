//! Geometric Shapes (U+25A0–U+25FF) — subset.
//!
//! Pixel-perfect rendering for commonly used geometric shapes: squares,
//! triangles, diamonds, circles, and corner triangles. Follows Ghostty's
//! `geometric_shapes.zig` for the corner triangle subset, extends with
//! additional shapes used by TUI frameworks.

use super::Canvas;

/// Draw a geometric shape. Returns `true` if handled.
pub(super) fn draw_geometric(canvas: &mut Canvas, ch: char) -> bool {
    let w = canvas.width() as f32;
    let h = canvas.height() as f32;

    // Sizing: match font glyph proportions. "Normal" shapes ~55% of cell,
    // "small" shapes ~30%, "large circle" ~70%. Corner triangles fill the cell.
    match ch {
        // Filled squares.
        '\u{25A0}' => centered_rect(canvas, w, h, 0.55, 255),
        '\u{25AA}' => centered_rect(canvas, w, h, 0.3, 255),
        '\u{25FC}' => centered_rect(canvas, w, h, 0.45, 255),
        '\u{25FE}' => centered_rect(canvas, w, h, 0.35, 255),
        // Outlined squares (25A2 = rounded, approximated as square).
        '\u{25A1}' | '\u{25A2}' => outlined_rect(canvas, w, h, 0.55),
        '\u{25AB}' => outlined_rect(canvas, w, h, 0.3),
        '\u{25FB}' => outlined_rect(canvas, w, h, 0.45),
        '\u{25FD}' => outlined_rect(canvas, w, h, 0.35),
        '\u{25A3}' => {
            // White square containing black small square.
            outlined_rect(canvas, w, h, 0.55);
            centered_rect(canvas, w, h, 0.25, 255);
        }
        // Triangles (pointing up, down, left, right — filled and outlined).
        '\u{25B2}' => fill_triangle_up(canvas, w, h, 0.6, 255),
        '\u{25B3}' => outline_triangle_up(canvas, w, h, 0.6),
        '\u{25B4}' => fill_triangle_up(canvas, w, h, 0.35, 255),
        '\u{25B5}' => outline_triangle_up(canvas, w, h, 0.35),
        '\u{25B6}' | '\u{25BA}' => fill_triangle_right(canvas, w, h, 0.6, 255),
        '\u{25B7}' | '\u{25BB}' => outline_triangle_right(canvas, w, h, 0.6),
        '\u{25B8}' => fill_triangle_right(canvas, w, h, 0.35, 255),
        '\u{25B9}' => outline_triangle_right(canvas, w, h, 0.35),
        '\u{25BC}' => fill_triangle_down(canvas, w, h, 0.6, 255),
        '\u{25BD}' => outline_triangle_down(canvas, w, h, 0.6),
        '\u{25BE}' => fill_triangle_down(canvas, w, h, 0.35, 255),
        '\u{25BF}' => outline_triangle_down(canvas, w, h, 0.35),
        '\u{25C0}' | '\u{25C4}' => fill_triangle_left(canvas, w, h, 0.6, 255),
        '\u{25C1}' | '\u{25C5}' => outline_triangle_left(canvas, w, h, 0.6),
        '\u{25C2}' => fill_triangle_left(canvas, w, h, 0.35, 255),
        '\u{25C3}' => outline_triangle_left(canvas, w, h, 0.35),
        // Diamonds.
        '\u{25C6}' => fill_diamond(canvas, w, h, 0.55, 255),
        '\u{25C7}' | '\u{25CA}' => outline_diamond(canvas, w, h, 0.55),
        '\u{25C8}' => {
            outline_diamond(canvas, w, h, 0.55);
            fill_diamond(canvas, w, h, 0.25, 255);
        }
        // Circles.
        '\u{25CB}' => stroke_circle(canvas, w, h, 0.5),
        '\u{25CF}' => fill_circle(canvas, w, h, 0.5, 255),
        '\u{25CE}' => {
            // Bullseye: outer ring + inner filled.
            stroke_circle(canvas, w, h, 0.5);
            fill_circle(canvas, w, h, 0.25, 255);
        }
        '\u{25C9}' => {
            // Fisheye: outer filled + inner white.
            fill_circle(canvas, w, h, 0.5, 255);
            fill_circle(canvas, w, h, 0.2, 0);
        }
        '\u{25EF}' => stroke_circle(canvas, w, h, 0.7), // Large circle
        // Half circles (filled).
        '\u{25D0}' => half_circle_left(canvas, w, h),
        '\u{25D1}' => half_circle_right(canvas, w, h),
        '\u{25D2}' => half_circle_bottom(canvas, w, h),
        '\u{25D3}' => half_circle_top(canvas, w, h),
        // Corner triangles (Ghostty subset).
        '\u{25E2}' => corner_triangle(canvas, w, h, CornerTri::BR, 255),
        '\u{25E3}' => corner_triangle(canvas, w, h, CornerTri::BL, 255),
        '\u{25E4}' => corner_triangle(canvas, w, h, CornerTri::TL, 255),
        '\u{25E5}' => corner_triangle(canvas, w, h, CornerTri::TR, 255),
        // Corner triangle outlines.
        '\u{25F8}' => corner_triangle_outline(canvas, w, h, CornerTri::TL),
        '\u{25F9}' => corner_triangle_outline(canvas, w, h, CornerTri::TR),
        '\u{25FA}' => corner_triangle_outline(canvas, w, h, CornerTri::BL),
        '\u{25FF}' => corner_triangle_outline(canvas, w, h, CornerTri::BR),
        _ => return false,
    }
    true
}

// -- Centered rectangles --

/// Draw a filled centered rectangle scaled by `frac` of cell size.
fn centered_rect(canvas: &mut Canvas, w: f32, h: f32, frac: f32, alpha: u8) {
    let rw = (w * frac).round();
    let rh = (h * frac).round();
    let x = ((w - rw) / 2.0).round();
    let y = ((h - rh) / 2.0).round();
    canvas.fill_rect(x, y, rw, rh, alpha);
}

/// Draw an outlined centered rectangle.
fn outlined_rect(canvas: &mut Canvas, w: f32, h: f32, frac: f32) {
    let thick = 1.0f32.max((w / 10.0).round());
    let rw = (w * frac).round();
    let rh = (h * frac).round();
    let x = ((w - rw) / 2.0).round();
    let y = ((h - rh) / 2.0).round();
    canvas.fill_rect(x, y, rw, thick, 255);
    canvas.fill_rect(x, y + rh - thick, rw, thick, 255);
    canvas.fill_rect(x, y, thick, rh, 255);
    canvas.fill_rect(x + rw - thick, y, thick, rh, 255);
}

// -- Triangles --

/// Fill upward-pointing triangle.
fn fill_triangle_up(canvas: &mut Canvas, w: f32, h: f32, frac: f32, alpha: u8) {
    let th = (h * frac).round();
    let tw = (w * frac).round();
    let ox = (w - tw) / 2.0;
    let oy = (h - th) / 2.0;
    let rows = th.ceil() as u32;
    for r in 0..rows {
        let progress = (r as f32 + 0.5) / th;
        let row_w = (tw * progress).round();
        let rx = ox + (tw - row_w) / 2.0;
        canvas.fill_rect(rx, oy + th - r as f32 - 1.0, row_w, 1.0, alpha);
    }
}

/// Outline upward-pointing triangle.
fn outline_triangle_up(canvas: &mut Canvas, w: f32, h: f32, frac: f32) {
    let thick = 1.0f32.max((w / 10.0).round());
    let th = (h * frac).round();
    let tw = (w * frac).round();
    let cx = w / 2.0;
    let oy = (h - th) / 2.0;
    let ox = (w - tw) / 2.0;
    // Left edge, right edge, bottom edge.
    canvas.fill_line(cx, oy, ox, oy + th, thick);
    canvas.fill_line(cx, oy, ox + tw, oy + th, thick);
    canvas.fill_rect(ox, oy + th - thick, tw, thick, 255);
}

/// Fill downward-pointing triangle.
fn fill_triangle_down(canvas: &mut Canvas, w: f32, h: f32, frac: f32, alpha: u8) {
    let th = (h * frac).round();
    let tw = (w * frac).round();
    let ox = (w - tw) / 2.0;
    let oy = (h - th) / 2.0;
    let rows = th.ceil() as u32;
    for r in 0..rows {
        let progress = (r as f32 + 0.5) / th;
        let row_w = (tw * progress).round();
        let rx = ox + (tw - row_w) / 2.0;
        canvas.fill_rect(rx, oy + r as f32, row_w, 1.0, alpha);
    }
}

/// Outline downward-pointing triangle.
fn outline_triangle_down(canvas: &mut Canvas, w: f32, h: f32, frac: f32) {
    let thick = 1.0f32.max((w / 10.0).round());
    let th = (h * frac).round();
    let tw = (w * frac).round();
    let cx = w / 2.0;
    let oy = (h - th) / 2.0;
    let ox = (w - tw) / 2.0;
    canvas.fill_rect(ox, oy, tw, thick, 255);
    canvas.fill_line(ox, oy, cx, oy + th, thick);
    canvas.fill_line(ox + tw, oy, cx, oy + th, thick);
}

/// Fill right-pointing triangle.
fn fill_triangle_right(canvas: &mut Canvas, w: f32, h: f32, frac: f32, alpha: u8) {
    let th = (h * frac).round();
    let tw = (w * frac).round();
    let ox = (w - tw) / 2.0;
    let oy = (h - th) / 2.0;
    let cols = tw.ceil() as u32;
    for c in 0..cols {
        let progress = (c as f32 + 0.5) / tw;
        let col_h = (th * progress).round();
        let ry = oy + (th - col_h) / 2.0;
        canvas.fill_rect(ox + c as f32, ry, 1.0, col_h, alpha);
    }
}

/// Outline right-pointing triangle.
fn outline_triangle_right(canvas: &mut Canvas, w: f32, h: f32, frac: f32) {
    let thick = 1.0f32.max((w / 10.0).round());
    let th = (h * frac).round();
    let tw = (w * frac).round();
    let ox = (w - tw) / 2.0;
    let oy = (h - th) / 2.0;
    let cy = h / 2.0;
    canvas.fill_rect(ox, oy, thick, th, 255);
    canvas.fill_line(ox, oy, ox + tw, cy, thick);
    canvas.fill_line(ox, oy + th, ox + tw, cy, thick);
}

/// Fill left-pointing triangle.
fn fill_triangle_left(canvas: &mut Canvas, w: f32, h: f32, frac: f32, alpha: u8) {
    let th = (h * frac).round();
    let tw = (w * frac).round();
    let ox = (w - tw) / 2.0;
    let oy = (h - th) / 2.0;
    let cols = tw.ceil() as u32;
    for c in 0..cols {
        let progress = (c as f32 + 0.5) / tw;
        let col_h = (th * progress).round();
        let ry = oy + (th - col_h) / 2.0;
        canvas.fill_rect(ox + tw - c as f32 - 1.0, ry, 1.0, col_h, alpha);
    }
}

/// Outline left-pointing triangle.
fn outline_triangle_left(canvas: &mut Canvas, w: f32, h: f32, frac: f32) {
    let thick = 1.0f32.max((w / 10.0).round());
    let th = (h * frac).round();
    let tw = (w * frac).round();
    let ox = (w - tw) / 2.0;
    let oy = (h - th) / 2.0;
    let cy = h / 2.0;
    canvas.fill_rect(ox + tw - thick, oy, thick, th, 255);
    canvas.fill_line(ox + tw, oy, ox, cy, thick);
    canvas.fill_line(ox + tw, oy + th, ox, cy, thick);
}

// -- Diamonds --

/// Fill a centered diamond.
fn fill_diamond(canvas: &mut Canvas, w: f32, h: f32, frac: f32, alpha: u8) {
    let dw = (w * frac).round();
    let dh = (h * frac).round();
    let ox = (w - dw) / 2.0;
    let oy = (h - dh) / 2.0;
    let rows = dh.ceil() as u32;
    for r in 0..rows {
        let progress = (r as f32 + 0.5) / dh;
        let row_w = if progress < 0.5 {
            (dw * progress * 2.0).round()
        } else {
            (dw * (1.0 - progress) * 2.0).round()
        };
        let rx = ox + (dw - row_w) / 2.0;
        canvas.fill_rect(rx, oy + r as f32, row_w, 1.0, alpha);
    }
}

/// Outline a centered diamond.
fn outline_diamond(canvas: &mut Canvas, w: f32, h: f32, frac: f32) {
    let thick = 1.0f32.max((w / 10.0).round());
    let dw = (w * frac).round();
    let dh = (h * frac).round();
    let cx = w / 2.0;
    let cy = h / 2.0;
    let hdw = dw / 2.0;
    let hdh = dh / 2.0;
    canvas.fill_line(cx, cy - hdh, cx + hdw, cy, thick);
    canvas.fill_line(cx + hdw, cy, cx, cy + hdh, thick);
    canvas.fill_line(cx, cy + hdh, cx - hdw, cy, thick);
    canvas.fill_line(cx - hdw, cy, cx, cy - hdh, thick);
}

// -- Circles --

/// Fill a centered circle.
fn fill_circle(canvas: &mut Canvas, w: f32, h: f32, frac: f32, alpha: u8) {
    let cx = w / 2.0;
    let cy = h / 2.0;
    let radius = (w.min(h) * frac / 2.0).round();

    let y_min = (cy - radius - 1.0).floor().max(0.0) as u32;
    let y_max = ((cy + radius + 1.0).ceil() as u32).min(canvas.height());
    let x_min = (cx - radius - 1.0).floor().max(0.0) as u32;
    let x_max = ((cx + radius + 1.0).ceil() as u32).min(canvas.width());

    for py in y_min..y_max {
        for px in x_min..x_max {
            let dx = px as f32 + 0.5 - cx;
            let dy = py as f32 + 0.5 - cy;
            let dist = dx.hypot(dy) - radius;
            let a = sdf_alpha(dist, alpha);
            if a > 0 {
                canvas.blend_pixel(px as i32, py as i32, a);
            }
        }
    }
}

/// Stroke a centered circle outline.
fn stroke_circle(canvas: &mut Canvas, w: f32, h: f32, frac: f32) {
    let cx = w / 2.0;
    let cy = h / 2.0;
    let radius = (w.min(h) * frac / 2.0).round();
    let thick = 1.0f32.max((w / 10.0).round());
    let ht = thick / 2.0;
    let inner = radius - ht;
    let outer = radius + ht;

    let y_min = (cy - outer - 1.0).floor().max(0.0) as u32;
    let y_max = ((cy + outer + 1.0).ceil() as u32).min(canvas.height());
    let x_min = (cx - outer - 1.0).floor().max(0.0) as u32;
    let x_max = ((cx + outer + 1.0).ceil() as u32).min(canvas.width());

    for py in y_min..y_max {
        for px in x_min..x_max {
            let dx = px as f32 + 0.5 - cx;
            let dy = py as f32 + 0.5 - cy;
            let dist = dx.hypot(dy);
            let ring_dist = if dist < inner {
                inner - dist
            } else if dist > outer {
                dist - outer
            } else {
                -1.0
            };
            let a = sdf_alpha(ring_dist, 255);
            if a > 0 {
                canvas.blend_pixel(px as i32, py as i32, a);
            }
        }
    }
}

// -- Half circles --

/// Left-half filled circle.
fn half_circle_left(canvas: &mut Canvas, w: f32, h: f32) {
    let cx = w / 2.0;
    let cy = h / 2.0;
    let radius = (w.min(h) * 0.5 / 2.0).round();
    stroke_circle(canvas, w, h, 0.5);
    // Fill left half.
    for py in 0..canvas.height() {
        for px in 0..canvas.width() {
            let dx = px as f32 + 0.5 - cx;
            let dy = py as f32 + 0.5 - cy;
            if dx < 0.0 && dx.hypot(dy) < radius - 0.5 {
                canvas.blend_pixel(px as i32, py as i32, 255);
            }
        }
    }
}

/// Right-half filled circle.
fn half_circle_right(canvas: &mut Canvas, w: f32, h: f32) {
    let cx = w / 2.0;
    let cy = h / 2.0;
    let radius = (w.min(h) * 0.5 / 2.0).round();
    stroke_circle(canvas, w, h, 0.5);
    for py in 0..canvas.height() {
        for px in 0..canvas.width() {
            let dx = px as f32 + 0.5 - cx;
            let dy = py as f32 + 0.5 - cy;
            if dx > 0.0 && dx.hypot(dy) < radius - 0.5 {
                canvas.blend_pixel(px as i32, py as i32, 255);
            }
        }
    }
}

/// Bottom-half filled circle.
fn half_circle_bottom(canvas: &mut Canvas, w: f32, h: f32) {
    let cx = w / 2.0;
    let cy = h / 2.0;
    let radius = (w.min(h) * 0.5 / 2.0).round();
    stroke_circle(canvas, w, h, 0.5);
    for py in 0..canvas.height() {
        for px in 0..canvas.width() {
            let dx = px as f32 + 0.5 - cx;
            let dy = py as f32 + 0.5 - cy;
            if dy > 0.0 && dx.hypot(dy) < radius - 0.5 {
                canvas.blend_pixel(px as i32, py as i32, 255);
            }
        }
    }
}

/// Top-half filled circle.
fn half_circle_top(canvas: &mut Canvas, w: f32, h: f32) {
    let cx = w / 2.0;
    let cy = h / 2.0;
    let radius = (w.min(h) * 0.5 / 2.0).round();
    stroke_circle(canvas, w, h, 0.5);
    for py in 0..canvas.height() {
        for px in 0..canvas.width() {
            let dx = px as f32 + 0.5 - cx;
            let dy = py as f32 + 0.5 - cy;
            if dy < 0.0 && dx.hypot(dy) < radius - 0.5 {
                canvas.blend_pixel(px as i32, py as i32, 255);
            }
        }
    }
}

// -- Corner triangles --

#[derive(Clone, Copy)]
enum CornerTri {
    TL,
    TR,
    BL,
    BR,
}

/// Fill a full-cell corner triangle.
fn corner_triangle(canvas: &mut Canvas, w: f32, h: f32, corner: CornerTri, alpha: u8) {
    let rows = h.ceil() as u32;
    for r in 0..rows {
        let frac = (r as f32 + 0.5) / h;
        let row_w = (w * frac).round();
        match corner {
            CornerTri::BL => canvas.fill_rect(0.0, r as f32, row_w, 1.0, alpha),
            CornerTri::BR => canvas.fill_rect(w - row_w, r as f32, row_w, 1.0, alpha),
            CornerTri::TL => {
                canvas.fill_rect(0.0, h - r as f32 - 1.0, row_w, 1.0, alpha);
            }
            CornerTri::TR => {
                canvas.fill_rect(w - row_w, h - r as f32 - 1.0, row_w, 1.0, alpha);
            }
        }
    }
}

/// Outline a full-cell corner triangle.
fn corner_triangle_outline(canvas: &mut Canvas, w: f32, h: f32, corner: CornerTri) {
    let thick = 1.0f32.max((w / 10.0).round());
    match corner {
        CornerTri::TL => {
            canvas.fill_line(0.0, 0.0, 0.0, h, thick);
            canvas.fill_line(0.0, 0.0, w, 0.0, thick);
            canvas.fill_line(0.0, h, w, 0.0, thick);
        }
        CornerTri::TR => {
            canvas.fill_line(0.0, 0.0, w, 0.0, thick);
            canvas.fill_line(w, 0.0, w, h, thick);
            canvas.fill_line(0.0, 0.0, w, h, thick);
        }
        CornerTri::BL => {
            canvas.fill_line(0.0, 0.0, 0.0, h, thick);
            canvas.fill_line(0.0, h, w, h, thick);
            canvas.fill_line(0.0, 0.0, w, h, thick);
        }
        CornerTri::BR => {
            canvas.fill_line(0.0, h, w, h, thick);
            canvas.fill_line(w, 0.0, w, h, thick);
            canvas.fill_line(0.0, h, w, 0.0, thick);
        }
    }
}

/// Convert signed distance to pixel alpha (1px anti-alias zone).
fn sdf_alpha(dist: f32, max_alpha: u8) -> u8 {
    if dist <= -0.5 {
        max_alpha
    } else if dist < 0.5 {
        ((0.5 - dist) * max_alpha as f32) as u8
    } else {
        0
    }
}
