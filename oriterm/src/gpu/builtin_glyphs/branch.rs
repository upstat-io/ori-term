//! Branch drawing characters (U+F5D0–U+F60D).
//!
//! Kitty/Ghostty PUA characters for git-like graph visualization. Includes
//! horizontal/vertical lines, corner arcs, fading lines, and branch node
//! circles with directional connectors.

use super::Canvas;

/// Bundled cell geometry for branch drawing.
struct Ctx {
    w: f32,
    h: f32,
    cx: f32,
    cy: f32,
    thick: f32,
}

impl Ctx {
    fn new(canvas: &Canvas) -> Self {
        let w = canvas.width() as f32;
        let h = canvas.height() as f32;
        Self {
            w,
            h,
            cx: (w / 2.0).floor(),
            cy: (h / 2.0).floor(),
            thick: 1.0f32.max((w / 8.0).round()),
        }
    }
}

/// Draw a branch drawing character. Returns `true` if handled.
pub(super) fn draw_branch(canvas: &mut Canvas, ch: char) -> bool {
    let g = Ctx::new(canvas);

    match ch {
        '\u{F5D0}' => hline(canvas, &g),
        '\u{F5D1}' => vline(canvas, &g),
        '\u{F5D2}'..='\u{F5D5}' => draw_fading(canvas, &g, ch),
        '\u{F5D6}'..='\u{F5D9}' => draw_solo_arc(canvas, &g, ch),
        '\u{F5DA}'..='\u{F5ED}' => draw_line_arc_combo(canvas, &g, ch),
        '\u{F5EE}'..='\u{F60D}' => draw_branch_node(canvas, &g, ch),
        _ => return false,
    }
    true
}

#[derive(Clone, Copy)]
enum Corner {
    TL,
    TR,
    BL,
    BR,
}

/// Horizontal line through center.
fn hline(canvas: &mut Canvas, g: &Ctx) {
    let ht = g.thick / 2.0;
    canvas.fill_rect(0.0, g.cy - ht, g.w, g.thick, 255);
}

/// Vertical line through center.
fn vline(canvas: &mut Canvas, g: &Ctx) {
    let ht = g.thick / 2.0;
    canvas.fill_rect(g.cx - ht, 0.0, g.thick, g.h, 255);
}

/// Draw fading line characters (U+F5D2–F5D5).
fn draw_fading(canvas: &mut Canvas, g: &Ctx, ch: char) {
    let ht = g.thick / 2.0;
    let steps = 16u32;

    let (horizontal, positive) = match ch {
        '\u{F5D2}' => (true, true),   // Right
        '\u{F5D3}' => (true, false),  // Left
        '\u{F5D4}' => (false, true),  // Down
        '\u{F5D5}' => (false, false), // Up
        _ => return,
    };

    for s in 0..steps {
        let frac0 = s as f32 / steps as f32;
        let frac1 = (s + 1) as f32 / steps as f32;
        let alpha = (255.0 * (1.0 - frac0)).round() as u8;

        if horizontal {
            let len = if positive { g.w - g.cx } else { g.cx };
            let (x0, x1) = if positive {
                (g.cx + len * frac0, g.cx + len * frac1)
            } else {
                (g.cx - len * frac1, g.cx - len * frac0)
            };
            canvas.fill_rect(x0, g.cy - ht, x1 - x0, g.thick, alpha);
        } else {
            let len = if positive { g.h - g.cy } else { g.cy };
            let (y0, y1) = if positive {
                (g.cy + len * frac0, g.cy + len * frac1)
            } else {
                (g.cy - len * frac1, g.cy - len * frac0)
            };
            canvas.fill_rect(g.cx - ht, y0, g.thick, y1 - y0, alpha);
        }
    }
}

/// Draw a solo arc character (U+F5D6–F5D9).
fn draw_solo_arc(canvas: &mut Canvas, g: &Ctx, ch: char) {
    let corner = match ch {
        '\u{F5D6}' => Corner::BR,
        '\u{F5D7}' => Corner::BL,
        '\u{F5D8}' => Corner::TR,
        '\u{F5D9}' => Corner::TL,
        _ => return,
    };
    arc(canvas, g, corner);
}

/// Draw line + arc combination characters (U+F5DA–F5ED).
fn draw_line_arc_combo(canvas: &mut Canvas, g: &Ctx, ch: char) {
    match ch {
        '\u{F5DA}' => {
            vline(canvas, g);
            arc(canvas, g, Corner::TR);
        }
        '\u{F5DB}' => {
            vline(canvas, g);
            arc(canvas, g, Corner::BR);
        }
        '\u{F5DC}' => {
            arc(canvas, g, Corner::TR);
            arc(canvas, g, Corner::BR);
        }
        '\u{F5DD}' => {
            vline(canvas, g);
            arc(canvas, g, Corner::TL);
        }
        '\u{F5DE}' => {
            vline(canvas, g);
            arc(canvas, g, Corner::BL);
        }
        '\u{F5DF}' => {
            arc(canvas, g, Corner::TL);
            arc(canvas, g, Corner::BL);
        }
        '\u{F5E0}' => {
            arc(canvas, g, Corner::BL);
            hline(canvas, g);
        }
        '\u{F5E1}' => {
            arc(canvas, g, Corner::BR);
            hline(canvas, g);
        }
        '\u{F5E2}' => {
            arc(canvas, g, Corner::BR);
            arc(canvas, g, Corner::BL);
        }
        '\u{F5E3}' => {
            arc(canvas, g, Corner::TL);
            hline(canvas, g);
        }
        '\u{F5E4}' => {
            arc(canvas, g, Corner::TR);
            hline(canvas, g);
        }
        '\u{F5E5}' => {
            arc(canvas, g, Corner::TR);
            arc(canvas, g, Corner::TL);
        }
        '\u{F5E6}' => {
            vline(canvas, g);
            arc(canvas, g, Corner::TL);
            arc(canvas, g, Corner::TR);
        }
        '\u{F5E7}' => {
            vline(canvas, g);
            arc(canvas, g, Corner::BL);
            arc(canvas, g, Corner::BR);
        }
        '\u{F5E8}' => {
            hline(canvas, g);
            arc(canvas, g, Corner::BL);
            arc(canvas, g, Corner::TL);
        }
        '\u{F5E9}' => {
            hline(canvas, g);
            arc(canvas, g, Corner::TR);
            arc(canvas, g, Corner::BR);
        }
        '\u{F5EA}' => {
            vline(canvas, g);
            arc(canvas, g, Corner::TL);
            arc(canvas, g, Corner::BR);
        }
        '\u{F5EB}' => {
            vline(canvas, g);
            arc(canvas, g, Corner::TR);
            arc(canvas, g, Corner::BL);
        }
        '\u{F5EC}' => {
            hline(canvas, g);
            arc(canvas, g, Corner::TL);
            arc(canvas, g, Corner::BR);
        }
        '\u{F5ED}' => {
            hline(canvas, g);
            arc(canvas, g, Corner::TR);
            arc(canvas, g, Corner::BL);
        }
        _ => {}
    }
}

/// Draw a quarter-circle arc from cell center toward a corner.
///
/// Uses polyline approximation with 16 segments for smooth anti-aliased arcs.
fn arc(canvas: &mut Canvas, g: &Ctx, corner: Corner) {
    let segments = 16;
    let half_pi = std::f32::consts::FRAC_PI_2;

    let (dx, dy) = match corner {
        Corner::TL => (-g.cx, -g.cy),
        Corner::TR => (g.w - g.cx, -g.cy),
        Corner::BL => (-g.cx, g.h - g.cy),
        Corner::BR => (g.w - g.cx, g.h - g.cy),
    };

    let rx = dx.abs();
    let ry = dy.abs();
    let sx = if dx < 0.0 { -1.0 } else { 1.0 };
    let sy = if dy < 0.0 { -1.0 } else { 1.0 };

    for i in 0..segments {
        let t0 = (i as f32 / segments as f32) * half_pi;
        let t1 = ((i + 1) as f32 / segments as f32) * half_pi;

        let x0 = g.cx + sx * rx * t0.sin();
        let y0 = g.cy + sy * ry * (1.0 - t0.cos());
        let x1 = g.cx + sx * rx * t1.sin();
        let y1 = g.cy + sy * ry * (1.0 - t1.cos());

        canvas.fill_line(x0, y0, x1, y1, g.thick);
    }
}

/// Draw a branch node circle with optional directional connectors.
fn draw_branch_node(canvas: &mut Canvas, g: &Ctx, ch: char) {
    let idx = ch as u32 - 0xF5EE;
    let filled = idx.is_multiple_of(2);
    let (up, right, down, left) = branch_node_flags(idx);

    let radius = (g.w.min(g.h) / 4.0).round().max(2.0);
    let ht = g.thick / 2.0;

    if up {
        canvas.fill_rect(g.cx - ht, 0.0, g.thick, g.cy - radius, 255);
    }
    if down {
        canvas.fill_rect(g.cx - ht, g.cy + radius, g.thick, g.h - g.cy - radius, 255);
    }
    if left {
        canvas.fill_rect(0.0, g.cy - ht, g.cx - radius, g.thick, 255);
    }
    if right {
        canvas.fill_rect(g.cx + radius, g.cy - ht, g.w - g.cx - radius, g.thick, 255);
    }

    if filled {
        fill_circle(canvas, g.cx, g.cy, radius);
    } else {
        stroke_circle(canvas, g.cx, g.cy, radius, g.thick);
    }
}

/// Decode branch node connector flags from codepoint index.
fn branch_node_flags(idx: u32) -> (bool, bool, bool, bool) {
    #[rustfmt::skip]
    const FLAGS: [(bool, bool, bool, bool); 16] = [
        (false, false, false, false), // none
        (false, true, false, false),  // R
        (false, false, false, true),  // L
        (false, true, false, true),   // LR
        (false, false, true, false),  // D
        (true, false, false, false),  // U
        (true, false, true, false),   // UD
        (false, true, true, false),   // RD
        (true, true, false, false),   // RU
        (false, false, true, true),   // LD
        (true, false, false, true),   // LU
        (true, true, true, false),    // RUD
        (true, false, true, true),    // LUD
        (false, true, true, true),    // LRD
        (true, true, false, true),    // LRU
        (true, true, true, true),     // LRUD
    ];
    FLAGS[(idx / 2) as usize]
}

/// Fill a circle centered at (cx, cy) with the given radius.
fn fill_circle(canvas: &mut Canvas, cx: f32, cy: f32, radius: f32) {
    let y_min = (cy - radius - 1.0).floor().max(0.0) as u32;
    let y_max = ((cy + radius + 1.0).ceil() as u32).min(canvas.height());
    let x_min = (cx - radius - 1.0).floor().max(0.0) as u32;
    let x_max = ((cx + radius + 1.0).ceil() as u32).min(canvas.width());

    for py in y_min..y_max {
        for px in x_min..x_max {
            let dx = px as f32 + 0.5 - cx;
            let dy = py as f32 + 0.5 - cy;
            let dist = dx.hypot(dy) - radius;
            let alpha = sdf_alpha(dist);
            if alpha > 0 {
                canvas.blend_pixel(px as i32, py as i32, alpha);
            }
        }
    }
}

/// Stroke a circle outline centered at (cx, cy).
fn stroke_circle(canvas: &mut Canvas, cx: f32, cy: f32, radius: f32, thick: f32) {
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
            let alpha = sdf_alpha(ring_dist);
            if alpha > 0 {
                canvas.blend_pixel(px as i32, py as i32, alpha);
            }
        }
    }
}

/// Convert signed distance to pixel alpha (1px anti-alias zone).
fn sdf_alpha(dist: f32) -> u8 {
    if dist <= -0.5 {
        255
    } else if dist < 0.5 {
        ((0.5 - dist) * 255.0) as u8
    } else {
        0
    }
}
