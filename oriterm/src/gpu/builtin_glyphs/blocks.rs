//! Block element rendering (U+2580–U+259F).
//!
//! Full blocks, fractional fills, shade patterns, and quadrant combinations.

use super::Canvas;

/// Draw a block element onto the canvas. Returns `true` if handled.
pub(super) fn draw_block(canvas: &mut Canvas, ch: char) -> bool {
    let w = canvas.width() as f32;
    let h = canvas.height() as f32;

    match ch {
        // Upper half block.
        '\u{2580}' => canvas.fill_rect(0.0, 0.0, w, (h / 2.0).round(), 255),
        // Lower N/8 blocks (U+2581–U+2587).
        '\u{2581}'..='\u{2587}' => {
            let eighths = (ch as u32 - 0x2580) as f32;
            let bh = (h * eighths / 8.0).round();
            canvas.fill_rect(0.0, h - bh, w, bh, 255);
        }
        // Full block.
        '\u{2588}' => canvas.fill_rect(0.0, 0.0, w, h, 255),
        // Left N/8 blocks (U+2589–U+258F): 7/8 down to 1/8.
        '\u{2589}'..='\u{258F}' => {
            let eighths = (0x2590 - ch as u32) as f32;
            canvas.fill_rect(0.0, 0.0, (w * eighths / 8.0).round(), h, 255);
        }
        // Right half.
        '\u{2590}' => {
            let hw = (w / 2.0).round();
            canvas.fill_rect(w - hw, 0.0, hw, h, 255);
        }
        // Shade blocks: 25%, 50%, 75%.
        '\u{2591}' => canvas.fill_rect(0.0, 0.0, w, h, 64),
        '\u{2592}' => canvas.fill_rect(0.0, 0.0, w, h, 128),
        '\u{2593}' => canvas.fill_rect(0.0, 0.0, w, h, 191),
        // Upper 1/8.
        '\u{2594}' => canvas.fill_rect(0.0, 0.0, w, (h / 8.0).round(), 255),
        // Right 1/8.
        '\u{2595}' => {
            let bw = (w / 8.0).round();
            canvas.fill_rect(w - bw, 0.0, bw, h, 255);
        }
        // Quadrant block elements (U+2596–U+259F).
        '\u{2596}'..='\u{259F}' => draw_quadrant(canvas, ch),
        _ => return false,
    }
    true
}

/// Draw a quadrant block element from a 4-bit bitmask.
///
/// Bit layout: `bit3 = TL, bit2 = TR, bit1 = BL, bit0 = BR`.
fn draw_quadrant(canvas: &mut Canvas, ch: char) {
    // Bitmask per quadrant, indexed from U+2596.
    #[rustfmt::skip]
    const QUADRANT_MASKS: [u8; 10] = [
        0b0010, // U+2596: lower left
        0b0001, // U+2597: lower right
        0b1000, // U+2598: upper left
        0b1011, // U+2599: upper left + lower left + lower right
        0b1001, // U+259A: upper left + lower right
        0b1110, // U+259B: upper left + upper right + lower left
        0b1101, // U+259C: upper left + upper right + lower right
        0b0100, // U+259D: upper right
        0b0110, // U+259E: upper right + lower left
        0b0111, // U+259F: upper right + lower left + lower right
    ];

    let w = canvas.width() as f32;
    let h = canvas.height() as f32;
    let idx = (ch as u32 - 0x2596) as usize;
    let mask = QUADRANT_MASKS[idx];
    let hw = (w / 2.0).round();
    let hh = (h / 2.0).round();

    if mask & 0b1000 != 0 {
        canvas.fill_rect(0.0, 0.0, hw, hh, 255);
    }
    if mask & 0b0100 != 0 {
        canvas.fill_rect(hw, 0.0, w - hw, hh, 255);
    }
    if mask & 0b0010 != 0 {
        canvas.fill_rect(0.0, hh, hw, h - hh, 255);
    }
    if mask & 0b0001 != 0 {
        canvas.fill_rect(hw, hh, w - hw, h - hh, 255);
    }
}
