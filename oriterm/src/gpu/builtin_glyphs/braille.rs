//! Braille pattern rendering (U+2800–U+28FF).
//!
//! Each braille character is a bitmask over an 8-dot grid (2 columns × 4 rows).
//! The character value encodes which dots are active.

use super::Canvas;

/// Standard braille dot positions: `(column, row, bit_index)`.
///
/// Bit layout matches Unicode:
/// - bits 0–2: left column, rows 0–2
/// - bits 3–5: right column, rows 0–2
/// - bit 6: left column, row 3
/// - bit 7: right column, row 3
const POSITIONS: [(usize, usize, u32); 8] = [
    (0, 0, 0),
    (0, 1, 1),
    (0, 2, 2),
    (1, 0, 3),
    (1, 1, 4),
    (1, 2, 5),
    (0, 3, 6),
    (1, 3, 7),
];

/// Draw a braille pattern onto the canvas. Returns `true` if handled.
pub(super) fn draw_braille(canvas: &mut Canvas, ch: char) -> bool {
    let bits = ch as u32 - 0x2800;
    if bits == 0 {
        return true; // Empty braille — no dots to draw.
    }

    let w = canvas.width() as f32;
    let h = canvas.height() as f32;
    let dot_w = (w / 5.0).round().max(2.0);
    let dot_h = (h / 10.0).round().max(2.0);

    for (col, row, bit) in POSITIONS {
        if bits & (1 << bit) != 0 {
            let dx = w * (0.25 + col as f32 * 0.5) - dot_w / 2.0;
            let dy = h * ((row as f32 + 0.5) / 4.0) - dot_h / 2.0;
            canvas.fill_rect(dx, dy, dot_w, dot_h, 255);
        }
    }

    true
}
