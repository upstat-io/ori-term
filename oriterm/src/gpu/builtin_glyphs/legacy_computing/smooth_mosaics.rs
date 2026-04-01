//! Smooth mosaic rendering (U+1FB3C–U+1FB67).
//!
//! 44 characters rendered on a 3-column x 4-row grid with rectangular and
//! diagonal fills. Each pattern is a hand-authored lookup table entry
//! matching the Unicode chart shapes.

use super::super::Canvas;

/// Draw a smooth mosaic character using the lookup table.
pub(super) fn draw(canvas: &mut Canvas, ch: char) {
    let idx = (ch as u32 - 0x1FB3C) as usize;
    let pattern = &SMOOTH_MOSAICS[idx];
    let w = canvas.width() as f32;
    let h = canvas.height() as f32;
    let cw = w / 3.0;
    let rh = h / 4.0;

    for (row_idx, row) in pattern.iter().enumerate() {
        let y = (rh * row_idx as f32).round();
        let y1 = (rh * (row_idx + 1) as f32).round();
        let row_h = y1 - y;

        for (col_idx, fill) in [row[0], row[1], row[2]].iter().enumerate() {
            let x = (cw * col_idx as f32).round();
            let x1 = (cw * (col_idx + 1) as f32).round();
            let cell_w = x1 - x;

            match fill {
                F::E => {}
                F::X => canvas.fill_rect(x, y, cell_w, row_h, 255),
                F::BL => fill_triangle_bl(canvas, x, y, cell_w, row_h),
                F::TR => fill_triangle_tr(canvas, x, y, cell_w, row_h),
                F::TL => fill_triangle_tl(canvas, x, y, cell_w, row_h),
                F::BR => fill_triangle_br(canvas, x, y, cell_w, row_h),
            }
        }
    }
}

/// Cell fill type for smooth mosaic sub-cells.
#[derive(Clone, Copy)]
enum F {
    /// Empty.
    E,
    /// Full.
    X,
    /// Diagonal triangle: bottom-left anchor.
    BL,
    /// Diagonal triangle: top-right anchor.
    TR,
    /// Diagonal triangle: top-left anchor.
    TL,
    /// Diagonal triangle: bottom-right anchor.
    BR,
}

/// Smooth mosaic lookup table indexed from U+1FB3C.
///
/// Each entry is 4 rows x 3 columns. Row 0 is top, column 0 is left.
#[rustfmt::skip]
const SMOOTH_MOSAICS: [[[F; 3]; 4]; 44] = [
    // U+1FB3C
    [[F::E, F::E, F::E], [F::E, F::E, F::E], [F::X, F::E, F::E], [F::X, F::X, F::E]],
    // U+1FB3D
    [[F::E, F::E, F::E], [F::E, F::E, F::E], [F::X, F::TR, F::E], [F::X, F::X, F::X]],
    // U+1FB3E
    [[F::E, F::E, F::E], [F::X, F::E, F::E], [F::X, F::TR, F::E], [F::X, F::X, F::E]],
    // U+1FB3F
    [[F::E, F::E, F::E], [F::X, F::E, F::E], [F::X, F::X, F::E], [F::X, F::X, F::X]],
    // U+1FB40
    [[F::X, F::E, F::E], [F::X, F::E, F::E], [F::X, F::X, F::E], [F::X, F::X, F::E]],
    // U+1FB41
    [[F::BR, F::X, F::X], [F::X, F::X, F::X], [F::X, F::X, F::X], [F::X, F::X, F::X]],
    // U+1FB42
    [[F::E, F::BR, F::X], [F::X, F::X, F::X], [F::X, F::X, F::X], [F::X, F::X, F::X]],
    // U+1FB43
    [[F::E, F::X, F::X], [F::E, F::X, F::X], [F::X, F::X, F::X], [F::X, F::X, F::X]],
    // U+1FB44
    [[F::E, F::E, F::X], [F::E, F::X, F::X], [F::X, F::X, F::X], [F::X, F::X, F::X]],
    // U+1FB45
    [[F::E, F::X, F::X], [F::E, F::X, F::X], [F::E, F::X, F::X], [F::X, F::X, F::X]],
    // U+1FB46
    [[F::E, F::E, F::E], [F::E, F::BR, F::X], [F::X, F::X, F::X], [F::X, F::X, F::X]],
    // U+1FB47
    [[F::E, F::E, F::E], [F::E, F::E, F::E], [F::E, F::E, F::X], [F::E, F::X, F::X]],
    // U+1FB48
    [[F::E, F::E, F::E], [F::E, F::E, F::E], [F::E, F::BR, F::X], [F::X, F::X, F::X]],
    // U+1FB49
    [[F::E, F::E, F::E], [F::E, F::E, F::X], [F::E, F::BR, F::X], [F::E, F::X, F::X]],
    // U+1FB4A
    [[F::E, F::E, F::E], [F::E, F::E, F::X], [F::E, F::X, F::X], [F::X, F::X, F::X]],
    // U+1FB4B
    [[F::E, F::E, F::X], [F::E, F::E, F::X], [F::E, F::X, F::X], [F::E, F::X, F::X]],
    // U+1FB4C
    [[F::X, F::X, F::BL], [F::X, F::X, F::X], [F::X, F::X, F::X], [F::X, F::X, F::X]],
    // U+1FB4D
    [[F::X, F::BL, F::E], [F::X, F::X, F::X], [F::X, F::X, F::X], [F::X, F::X, F::X]],
    // U+1FB4E
    [[F::X, F::X, F::E], [F::X, F::X, F::E], [F::X, F::X, F::X], [F::X, F::X, F::X]],
    // U+1FB4F
    [[F::X, F::E, F::E], [F::X, F::X, F::E], [F::X, F::X, F::X], [F::X, F::X, F::X]],
    // U+1FB50
    [[F::X, F::X, F::E], [F::X, F::X, F::E], [F::X, F::X, F::E], [F::X, F::X, F::X]],
    // U+1FB51
    [[F::E, F::E, F::E], [F::X, F::BL, F::E], [F::X, F::X, F::X], [F::X, F::X, F::X]],
    // U+1FB52
    [[F::X, F::X, F::X], [F::X, F::X, F::X], [F::X, F::X, F::X], [F::TR, F::X, F::X]],
    // U+1FB53
    [[F::X, F::X, F::X], [F::X, F::X, F::X], [F::X, F::X, F::X], [F::E, F::TR, F::X]],
    // U+1FB54
    [[F::X, F::X, F::X], [F::X, F::X, F::X], [F::E, F::X, F::X], [F::E, F::X, F::X]],
    // U+1FB55
    [[F::X, F::X, F::X], [F::X, F::X, F::X], [F::E, F::X, F::X], [F::E, F::E, F::X]],
    // U+1FB56
    [[F::X, F::X, F::X], [F::E, F::X, F::X], [F::E, F::X, F::X], [F::E, F::X, F::X]],
    // U+1FB57
    [[F::X, F::X, F::X], [F::X, F::X, F::X], [F::X, F::TR, F::E], [F::E, F::E, F::E]],
    // U+1FB58
    [[F::X, F::X, F::X], [F::X, F::X, F::X], [F::X, F::X, F::E], [F::X, F::E, F::E]],
    // U+1FB59
    [[F::X, F::X, F::X], [F::X, F::X, F::E], [F::X, F::X, F::E], [F::X, F::X, F::E]],
    // U+1FB5A
    [[F::X, F::X, F::X], [F::X, F::X, F::X], [F::X, F::X, F::E], [F::X, F::X, F::E]],
    // U+1FB5B
    [[F::X, F::X, F::X], [F::X, F::X, F::E], [F::X, F::X, F::E], [F::X, F::E, F::E]],
    // U+1FB5C
    [[F::X, F::X, F::X], [F::X, F::X, F::X], [F::E, F::TR, F::X], [F::E, F::E, F::E]],
    // U+1FB5D
    [[F::X, F::X, F::X], [F::X, F::X, F::X], [F::X, F::X, F::X], [F::X, F::X, F::TL]],
    // U+1FB5E
    [[F::X, F::X, F::X], [F::X, F::X, F::X], [F::X, F::X, F::X], [F::X, F::TL, F::E]],
    // U+1FB5F
    [[F::X, F::X, F::X], [F::X, F::X, F::X], [F::X, F::X, F::E], [F::X, F::X, F::E]],
    // U+1FB60
    [[F::X, F::X, F::X], [F::X, F::X, F::X], [F::X, F::E, F::E], [F::X, F::X, F::E]],
    // U+1FB61
    [[F::X, F::X, F::X], [F::X, F::X, F::E], [F::X, F::X, F::E], [F::X, F::X, F::E]],
    // U+1FB62
    [[F::X, F::X, F::X], [F::X, F::X, F::X], [F::X, F::TL, F::E], [F::E, F::E, F::E]],
    // U+1FB63
    [[F::X, F::X, F::X], [F::X, F::X, F::X], [F::X, F::X, F::X], [F::E, F::X, F::TL]],
    // U+1FB64
    [[F::X, F::X, F::X], [F::X, F::X, F::X], [F::X, F::X, F::X], [F::E, F::TL, F::E]],
    // U+1FB65
    [[F::X, F::X, F::X], [F::X, F::X, F::X], [F::E, F::X, F::X], [F::E, F::E, F::X]],
    // U+1FB66
    [[F::X, F::X, F::X], [F::X, F::X, F::X], [F::E, F::E, F::X], [F::E, F::X, F::X]],
    // U+1FB67
    [[F::X, F::X, F::X], [F::X, F::X, F::E], [F::X, F::X, F::E], [F::X, F::X, F::X]],
];

// -- Triangle fill helpers --

/// Fill bottom-left triangle of a cell rect.
fn fill_triangle_bl(canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32) {
    let rows = h.ceil() as u32;
    for r in 0..rows {
        let frac = (r as f32 + 0.5) / h;
        let fill_w = (w * frac).round();
        canvas.fill_rect(x, y + h - r as f32 - 1.0, fill_w, 1.0, 255);
    }
}

/// Fill top-right triangle of a cell rect.
fn fill_triangle_tr(canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32) {
    let rows = h.ceil() as u32;
    for r in 0..rows {
        let frac = (r as f32 + 0.5) / h;
        let fill_w = (w * frac).round();
        canvas.fill_rect(x + w - fill_w, y + r as f32, fill_w, 1.0, 255);
    }
}

/// Fill top-left triangle of a cell rect.
fn fill_triangle_tl(canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32) {
    let rows = h.ceil() as u32;
    for r in 0..rows {
        let frac = (r as f32 + 0.5) / h;
        let fill_w = (w * frac).round();
        canvas.fill_rect(x, y + r as f32, fill_w, 1.0, 255);
    }
}

/// Fill bottom-right triangle of a cell rect.
fn fill_triangle_br(canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32) {
    let rows = h.ceil() as u32;
    for r in 0..rows {
        let frac = (r as f32 + 0.5) / h;
        let fill_w = (w * frac).round();
        canvas.fill_rect(x + w - fill_w, y + h - r as f32 - 1.0, fill_w, 1.0, 255);
    }
}
