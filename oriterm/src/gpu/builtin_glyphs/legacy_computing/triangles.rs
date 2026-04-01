//! Edge triangle and combined triangle rendering (U+1FB68–U+1FB6F, U+1FB9A–U+1FB9F).
//!
//! Edge triangles fill one half of the cell diagonally from an edge midpoint.
//! Combined triangles compose two edge triangles or corner shade fills.

use super::super::Canvas;

/// Draw edge triangle characters (U+1FB68–U+1FB6F).
///
/// U+1FB68–1FB6B: inverted edge triangles (filled complement).
/// U+1FB6C–1FB6F: standard edge triangles.
pub(super) fn draw_edge_triangle(canvas: &mut Canvas, ch: char) {
    let w = canvas.width() as f32;
    let h = canvas.height() as f32;

    match ch {
        '\u{1FB68}' => fill_edge_complement(canvas, Edge::Left),
        '\u{1FB69}' => fill_edge_complement(canvas, Edge::Top),
        '\u{1FB6A}' => fill_edge_complement(canvas, Edge::Right),
        '\u{1FB6B}' => fill_edge_complement(canvas, Edge::Bottom),
        '\u{1FB6C}' => fill_edge(canvas, Edge::Left, w, h),
        '\u{1FB6D}' => fill_edge(canvas, Edge::Top, w, h),
        '\u{1FB6E}' => fill_edge(canvas, Edge::Right, w, h),
        '\u{1FB6F}' => fill_edge(canvas, Edge::Bottom, w, h),
        _ => {}
    }
}

/// Draw combined triangle and corner shade characters (U+1FB9A–U+1FB9F).
pub(super) fn draw_combined(canvas: &mut Canvas, ch: char) {
    let w = canvas.width() as f32;
    let h = canvas.height() as f32;

    match ch {
        // Upper + lower edge triangles (bowtie vertical).
        '\u{1FB9A}' => {
            fill_edge(canvas, Edge::Top, w, h);
            fill_edge(canvas, Edge::Bottom, w, h);
        }
        // Left + right edge triangles (bowtie horizontal).
        '\u{1FB9B}' => {
            fill_edge(canvas, Edge::Left, w, h);
            fill_edge(canvas, Edge::Right, w, h);
        }
        // Corner triangle shades (medium shade).
        '\u{1FB9C}' => fill_corner(canvas, 0.0, 0.0, w, h),
        '\u{1FB9D}' => fill_corner(canvas, w, 0.0, -w, h),
        '\u{1FB9E}' => fill_corner(canvas, w, h, -w, -h),
        '\u{1FB9F}' => fill_corner(canvas, 0.0, h, w, -h),
        _ => {}
    }
}

#[derive(Clone, Copy)]
enum Edge {
    Left,
    Top,
    Right,
    Bottom,
}

/// Fill an edge-aligned triangle (point at midpoint of opposite edge).
fn fill_edge(canvas: &mut Canvas, edge: Edge, w: f32, h: f32) {
    match edge {
        Edge::Left => {
            let rows = h.ceil() as u32;
            for r in 0..rows {
                let mid = h / 2.0;
                let dist = ((r as f32 + 0.5) - mid).abs() / mid;
                let fw = (w * (1.0 - dist)).round();
                canvas.fill_rect(0.0, r as f32, fw, 1.0, 255);
            }
        }
        Edge::Right => {
            let rows = h.ceil() as u32;
            for r in 0..rows {
                let mid = h / 2.0;
                let dist = ((r as f32 + 0.5) - mid).abs() / mid;
                let fw = (w * (1.0 - dist)).round();
                canvas.fill_rect(w - fw, r as f32, fw, 1.0, 255);
            }
        }
        Edge::Top => {
            let cols = w.ceil() as u32;
            for c in 0..cols {
                let mid = w / 2.0;
                let dist = ((c as f32 + 0.5) - mid).abs() / mid;
                let fh = (h * (1.0 - dist)).round();
                canvas.fill_rect(c as f32, 0.0, 1.0, fh, 255);
            }
        }
        Edge::Bottom => {
            let cols = w.ceil() as u32;
            for c in 0..cols {
                let mid = w / 2.0;
                let dist = ((c as f32 + 0.5) - mid).abs() / mid;
                let fh = (h * (1.0 - dist)).round();
                canvas.fill_rect(c as f32, h - fh, 1.0, fh, 255);
            }
        }
    }
}

/// Fill complement of an edge triangle (everything except the triangle).
fn fill_edge_complement(canvas: &mut Canvas, edge: Edge) {
    let w = canvas.width() as f32;
    let h = canvas.height() as f32;

    // Fill entire cell, then clear the triangle area.
    canvas.fill_rect(0.0, 0.0, w, h, 255);

    match edge {
        Edge::Left => {
            let rows = h.ceil() as u32;
            for r in 0..rows {
                let mid = h / 2.0;
                let dist = ((r as f32 + 0.5) - mid).abs() / mid;
                let cw = (w * (1.0 - dist)).round();
                canvas.fill_rect(0.0, r as f32, cw, 1.0, 0);
            }
        }
        Edge::Right => {
            let rows = h.ceil() as u32;
            for r in 0..rows {
                let mid = h / 2.0;
                let dist = ((r as f32 + 0.5) - mid).abs() / mid;
                let cw = (w * (1.0 - dist)).round();
                canvas.fill_rect(w - cw, r as f32, cw, 1.0, 0);
            }
        }
        Edge::Top => {
            let cols = w.ceil() as u32;
            for c in 0..cols {
                let mid = w / 2.0;
                let dist = ((c as f32 + 0.5) - mid).abs() / mid;
                let ch = (h * (1.0 - dist)).round();
                canvas.fill_rect(c as f32, 0.0, 1.0, ch, 0);
            }
        }
        Edge::Bottom => {
            let cols = w.ceil() as u32;
            for c in 0..cols {
                let mid = w / 2.0;
                let dist = ((c as f32 + 0.5) - mid).abs() / mid;
                let ch = (h * (1.0 - dist)).round();
                canvas.fill_rect(c as f32, h - ch, 1.0, ch, 0);
            }
        }
    }
}

/// Fill a corner triangle with medium shade (128 alpha).
fn fill_corner(canvas: &mut Canvas, ox: f32, oy: f32, dw: f32, dh: f32) {
    let rows = dh.abs().ceil() as u32;
    let sign_h = if dh >= 0.0 { 1.0 } else { -1.0 };
    let sign_w = if dw >= 0.0 { 1.0 } else { -1.0 };

    for r in 0..rows {
        let frac = (r as f32 + 0.5) / dh.abs();
        let fw = (dw.abs() * (1.0 - frac)).round();
        let y = oy + sign_h * r as f32;
        let x = if sign_w >= 0.0 { ox } else { ox - fw };
        canvas.fill_rect(x, y, fw, 1.0, 128);
    }
}
