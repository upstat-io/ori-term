//! Geometric Shapes (U+25A0–U+25FF) — subset.
//!
//! Pixel-perfect rendering for commonly used geometric shapes: squares,
//! triangles, diamonds, circles, and corner triangles.
//!
//! **Sizing rule**: font glyphs are designed on a square em-square. Terminal
//! cells are taller than wide (~2:1). ALL shapes use the cell *width* as
//! the base for a square bounding box, centered vertically in the cell.
//! Empirical measurement of monospace fonts shows normal shapes at ~0.92
//! of cell width; small variants at ~0.50.

use super::Canvas;

/// Square bounding box for centered shapes, derived from cell dimensions.
///
/// The box side length equals `cell_width`; it is vertically centered in
/// the taller cell. All centered shapes (squares, triangles, diamonds)
/// draw within this box so they maintain 1:1 aspect ratio.
struct SqBox {
    w: f32,
    oy: f32,
    s: f32,
}

impl SqBox {
    fn new(w: f32, h: f32) -> Self {
        Self {
            w,
            oy: ((h - w) / 2.0).round(),
            s: w,
        }
    }

    /// Compute offset + size for a shape at `frac` of the bounding box.
    fn inset(&self, frac: f32) -> (f32, f32, f32) {
        let sz = (self.s * frac).round();
        let x = ((self.w - sz) / 2.0).round();
        let y = self.oy + ((self.s - sz) / 2.0).round();
        (x, y, sz)
    }
}

/// Draw a geometric shape. Returns `true` if handled.
pub(super) fn draw_geometric(canvas: &mut Canvas, ch: char) -> bool {
    let w = canvas.width() as f32;
    let h = canvas.height() as f32;
    let b = SqBox::new(w, h);

    // Sizing: ~0.92 of cell width for normal shapes, ~0.50 for small variants.
    // Empirical from DejaVu Sans Mono, Fira Code, JetBrains Mono measurements.
    match ch {
        // Filled squares.
        '\u{25A0}' => sq_fill(canvas, &b, 0.92, 255),
        '\u{25AA}' => sq_fill(canvas, &b, 0.50, 255),
        '\u{25FC}' => sq_fill(canvas, &b, 0.75, 255),
        '\u{25FE}' => sq_fill(canvas, &b, 0.60, 255),
        // Outlined squares (25A2 = rounded, approximated as square).
        '\u{25A1}' | '\u{25A2}' => sq_outline(canvas, &b, 0.92),
        '\u{25AB}' => sq_outline(canvas, &b, 0.50),
        '\u{25FB}' => sq_outline(canvas, &b, 0.75),
        '\u{25FD}' => sq_outline(canvas, &b, 0.60),
        '\u{25A3}' => {
            sq_outline(canvas, &b, 0.92);
            sq_fill(canvas, &b, 0.50, 255);
        }
        // Triangles.
        '\u{25B2}' => tri_fill(canvas, &b, 0.92, Dir4::Up, 255),
        '\u{25B3}' => tri_outline(canvas, &b, 0.92, Dir4::Up),
        '\u{25B4}' => tri_fill(canvas, &b, 0.55, Dir4::Up, 255),
        '\u{25B5}' => tri_outline(canvas, &b, 0.55, Dir4::Up),
        '\u{25B6}' | '\u{25BA}' => tri_fill(canvas, &b, 0.92, Dir4::Right, 255),
        '\u{25B7}' | '\u{25BB}' => tri_outline(canvas, &b, 0.92, Dir4::Right),
        '\u{25B8}' => tri_fill(canvas, &b, 0.55, Dir4::Right, 255),
        '\u{25B9}' => tri_outline(canvas, &b, 0.55, Dir4::Right),
        '\u{25BC}' => tri_fill(canvas, &b, 0.92, Dir4::Down, 255),
        '\u{25BD}' => tri_outline(canvas, &b, 0.92, Dir4::Down),
        '\u{25BE}' => tri_fill(canvas, &b, 0.55, Dir4::Down, 255),
        '\u{25BF}' => tri_outline(canvas, &b, 0.55, Dir4::Down),
        '\u{25C0}' | '\u{25C4}' => tri_fill(canvas, &b, 0.92, Dir4::Left, 255),
        '\u{25C1}' | '\u{25C5}' => tri_outline(canvas, &b, 0.92, Dir4::Left),
        '\u{25C2}' => tri_fill(canvas, &b, 0.55, Dir4::Left, 255),
        '\u{25C3}' => tri_outline(canvas, &b, 0.55, Dir4::Left),
        // Diamonds.
        '\u{25C6}' => diamond_fill(canvas, &b, 0.92, 255),
        '\u{25C7}' | '\u{25CA}' => diamond_outline(canvas, &b, 0.92),
        '\u{25C8}' => {
            diamond_outline(canvas, &b, 0.92);
            diamond_fill(canvas, &b, 0.45, 255);
        }
        // Circles (radius from cell width for square proportions).
        '\u{25CB}' => circle_stroke(canvas, w, h, 0.85),
        '\u{25CF}' => circle_fill(canvas, w, h, 0.85, 255),
        '\u{25CE}' => {
            circle_stroke(canvas, w, h, 0.85);
            circle_fill(canvas, w, h, 0.45, 255);
        }
        '\u{25C9}' => {
            circle_fill(canvas, w, h, 0.85, 255);
            circle_fill(canvas, w, h, 0.40, 0);
        }
        '\u{25EF}' => circle_stroke(canvas, w, h, 0.95),
        // Half circles.
        '\u{25D0}' => half_circle(canvas, w, h, Dir4::Left),
        '\u{25D1}' => half_circle(canvas, w, h, Dir4::Right),
        '\u{25D2}' => half_circle(canvas, w, h, Dir4::Down),
        '\u{25D3}' => half_circle(canvas, w, h, Dir4::Up),
        // Corner triangles — inside square bounding box (not full cell).
        '\u{25E2}' => corner_tri(canvas, &b, Corner::BR, 255),
        '\u{25E3}' => corner_tri(canvas, &b, Corner::BL, 255),
        '\u{25E4}' => corner_tri(canvas, &b, Corner::TL, 255),
        '\u{25E5}' => corner_tri(canvas, &b, Corner::TR, 255),
        '\u{25F8}' => corner_tri_outline(canvas, &b, Corner::TL),
        '\u{25F9}' => corner_tri_outline(canvas, &b, Corner::TR),
        '\u{25FA}' => corner_tri_outline(canvas, &b, Corner::BL),
        '\u{25FF}' => corner_tri_outline(canvas, &b, Corner::BR),
        _ => return false,
    }
    true
}

/// Line thickness for outlines.
fn thick(w: f32) -> f32 {
    1.0f32.max((w / 12.0).round())
}

// -- Squares --

fn sq_fill(canvas: &mut Canvas, b: &SqBox, frac: f32, alpha: u8) {
    let (x, y, sz) = b.inset(frac);
    canvas.fill_rect(x, y, sz, sz, alpha);
}

fn sq_outline(canvas: &mut Canvas, b: &SqBox, frac: f32) {
    let t = thick(b.w);
    let (x, y, sz) = b.inset(frac);
    canvas.fill_rect(x, y, sz, t, 255);
    canvas.fill_rect(x, y + sz - t, sz, t, 255);
    canvas.fill_rect(x, y, t, sz, 255);
    canvas.fill_rect(x + sz - t, y, t, sz, 255);
}

// -- Triangles --

#[derive(Clone, Copy)]
enum Dir4 {
    Up,
    Down,
    Left,
    Right,
}

fn tri_fill(canvas: &mut Canvas, b: &SqBox, frac: f32, dir: Dir4, alpha: u8) {
    let (ox, oy, sz) = b.inset(frac);
    let n = sz.ceil() as u32;
    for i in 0..n {
        let progress = (i as f32 + 0.5) / sz;
        let span = (sz * progress).round();
        match dir {
            Dir4::Up => {
                canvas.fill_rect(
                    ox + (sz - span) / 2.0,
                    oy + sz - i as f32 - 1.0,
                    span,
                    1.0,
                    alpha,
                );
            }
            Dir4::Down => {
                canvas.fill_rect(ox + (sz - span) / 2.0, oy + i as f32, span, 1.0, alpha);
            }
            Dir4::Right => {
                canvas.fill_rect(ox + i as f32, oy + (sz - span) / 2.0, 1.0, span, alpha);
            }
            Dir4::Left => {
                canvas.fill_rect(
                    ox + sz - i as f32 - 1.0,
                    oy + (sz - span) / 2.0,
                    1.0,
                    span,
                    alpha,
                );
            }
        }
    }
}

fn tri_outline(canvas: &mut Canvas, b: &SqBox, frac: f32, dir: Dir4) {
    let t = thick(b.w);
    let (ox, oy, sz) = b.inset(frac);
    let cx = ox + sz / 2.0;
    let cy = oy + sz / 2.0;
    match dir {
        Dir4::Up => {
            canvas.fill_line(cx, oy, ox, oy + sz, t);
            canvas.fill_line(cx, oy, ox + sz, oy + sz, t);
            canvas.fill_rect(ox, oy + sz - t, sz, t, 255);
        }
        Dir4::Down => {
            canvas.fill_rect(ox, oy, sz, t, 255);
            canvas.fill_line(ox, oy, cx, oy + sz, t);
            canvas.fill_line(ox + sz, oy, cx, oy + sz, t);
        }
        Dir4::Right => {
            canvas.fill_rect(ox, oy, t, sz, 255);
            canvas.fill_line(ox, oy, ox + sz, cy, t);
            canvas.fill_line(ox, oy + sz, ox + sz, cy, t);
        }
        Dir4::Left => {
            canvas.fill_rect(ox + sz - t, oy, t, sz, 255);
            canvas.fill_line(ox + sz, oy, ox, cy, t);
            canvas.fill_line(ox + sz, oy + sz, ox, cy, t);
        }
    }
}

// -- Diamonds --

fn diamond_fill(canvas: &mut Canvas, b: &SqBox, frac: f32, alpha: u8) {
    let (ox, oy, sz) = b.inset(frac);
    let n = sz.ceil() as u32;
    for r in 0..n {
        let progress = (r as f32 + 0.5) / sz;
        let rw = if progress < 0.5 {
            (sz * progress * 2.0).round()
        } else {
            (sz * (1.0 - progress) * 2.0).round()
        };
        canvas.fill_rect(ox + (sz - rw) / 2.0, oy + r as f32, rw, 1.0, alpha);
    }
}

fn diamond_outline(canvas: &mut Canvas, b: &SqBox, frac: f32) {
    let t = thick(b.w);
    let cx = b.w / 2.0;
    let cy = b.oy + b.s / 2.0;
    let hs = (b.s * frac / 2.0).round();
    canvas.fill_line(cx, cy - hs, cx + hs, cy, t);
    canvas.fill_line(cx + hs, cy, cx, cy + hs, t);
    canvas.fill_line(cx, cy + hs, cx - hs, cy, t);
    canvas.fill_line(cx - hs, cy, cx, cy - hs, t);
}

// -- Circles --

fn circle_fill(canvas: &mut Canvas, w: f32, h: f32, frac: f32, alpha: u8) {
    let cx = w / 2.0;
    let cy = h / 2.0;
    let radius = (w * frac / 2.0).round();
    let y0 = (cy - radius - 1.0).floor().max(0.0) as u32;
    let y1 = ((cy + radius + 1.0).ceil() as u32).min(canvas.height());
    let x0 = (cx - radius - 1.0).floor().max(0.0) as u32;
    let x1 = ((cx + radius + 1.0).ceil() as u32).min(canvas.width());
    for py in y0..y1 {
        for px in x0..x1 {
            let dx = px as f32 + 0.5 - cx;
            let dy = py as f32 + 0.5 - cy;
            let a = sdf_alpha(dx.hypot(dy) - radius, alpha);
            if a > 0 {
                canvas.blend_pixel(px as i32, py as i32, a);
            }
        }
    }
}

fn circle_stroke(canvas: &mut Canvas, w: f32, h: f32, frac: f32) {
    let cx = w / 2.0;
    let cy = h / 2.0;
    let radius = (w * frac / 2.0).round();
    let t = thick(w);
    let ht = t / 2.0;
    let inner = radius - ht;
    let outer = radius + ht;
    let y0 = (cy - outer - 1.0).floor().max(0.0) as u32;
    let y1 = ((cy + outer + 1.0).ceil() as u32).min(canvas.height());
    let x0 = (cx - outer - 1.0).floor().max(0.0) as u32;
    let x1 = ((cx + outer + 1.0).ceil() as u32).min(canvas.width());
    for py in y0..y1 {
        for px in x0..x1 {
            let dx = px as f32 + 0.5 - cx;
            let dy = py as f32 + 0.5 - cy;
            let dist = dx.hypot(dy);
            let ring = if dist < inner {
                inner - dist
            } else if dist > outer {
                dist - outer
            } else {
                -1.0
            };
            let a = sdf_alpha(ring, 255);
            if a > 0 {
                canvas.blend_pixel(px as i32, py as i32, a);
            }
        }
    }
}

fn half_circle(canvas: &mut Canvas, w: f32, h: f32, side: Dir4) {
    let cx = w / 2.0;
    let cy = h / 2.0;
    let radius = (w * 0.85 / 2.0).round();
    circle_stroke(canvas, w, h, 0.85);
    for py in 0..canvas.height() {
        for px in 0..canvas.width() {
            let dx = px as f32 + 0.5 - cx;
            let dy = py as f32 + 0.5 - cy;
            let inside = match side {
                Dir4::Left => dx < 0.0,
                Dir4::Right => dx > 0.0,
                Dir4::Up => dy < 0.0,
                Dir4::Down => dy > 0.0,
            };
            if inside && dx.hypot(dy) < radius - 0.5 {
                canvas.blend_pixel(px as i32, py as i32, 255);
            }
        }
    }
}

// -- Corner triangles (fill entire cell) --

#[derive(Clone, Copy)]
enum Corner {
    TL,
    TR,
    BL,
    BR,
}

fn corner_tri(canvas: &mut Canvas, b: &SqBox, c: Corner, alpha: u8) {
    let (ox, oy, sz) = b.inset(1.0);
    let rows = sz.ceil() as u32;
    for r in 0..rows {
        let f = (r as f32 + 0.5) / sz;
        let rw = (sz * f).round();
        match c {
            Corner::BL => canvas.fill_rect(ox, oy + r as f32, rw, 1.0, alpha),
            Corner::BR => canvas.fill_rect(ox + sz - rw, oy + r as f32, rw, 1.0, alpha),
            Corner::TL => canvas.fill_rect(ox, oy + sz - r as f32 - 1.0, rw, 1.0, alpha),
            Corner::TR => {
                canvas.fill_rect(ox + sz - rw, oy + sz - r as f32 - 1.0, rw, 1.0, alpha);
            }
        }
    }
}

fn corner_tri_outline(canvas: &mut Canvas, b: &SqBox, c: Corner) {
    let t = thick(b.w);
    let (ox, oy, sz) = b.inset(1.0);
    let x1 = ox + sz;
    let y1 = oy + sz;
    match c {
        Corner::TL => {
            canvas.fill_line(ox, oy, ox, y1, t);
            canvas.fill_line(ox, oy, x1, oy, t);
            canvas.fill_line(ox, y1, x1, oy, t);
        }
        Corner::TR => {
            canvas.fill_line(ox, oy, x1, oy, t);
            canvas.fill_line(x1, oy, x1, y1, t);
            canvas.fill_line(ox, oy, x1, y1, t);
        }
        Corner::BL => {
            canvas.fill_line(ox, oy, ox, y1, t);
            canvas.fill_line(ox, y1, x1, y1, t);
            canvas.fill_line(ox, oy, x1, y1, t);
        }
        Corner::BR => {
            canvas.fill_line(ox, y1, x1, y1, t);
            canvas.fill_line(x1, oy, x1, y1, t);
            canvas.fill_line(ox, y1, x1, oy, t);
        }
    }
}

fn sdf_alpha(dist: f32, max_alpha: u8) -> u8 {
    if dist <= -0.5 {
        max_alpha
    } else if dist < 0.5 {
        ((0.5 - dist) * max_alpha as f32) as u8
    } else {
        0
    }
}
