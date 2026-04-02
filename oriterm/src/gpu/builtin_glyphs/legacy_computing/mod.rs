//! Symbols for Legacy Computing (U+1FB00–U+1FB9F).
//!
//! Sextants, smooth mosaics, fractional blocks, edge triangles, wedges,
//! shade patterns, and checkerboard fills. Follows Ghostty's
//! `symbols_for_legacy_computing.zig` as reference.

mod smooth_mosaics;
mod triangles;

use super::Canvas;

/// Draw a Symbols for Legacy Computing character. Returns `true` if handled.
pub(in crate::gpu::builtin_glyphs) fn draw(canvas: &mut Canvas, ch: char) -> bool {
    match ch {
        '\u{1FB00}'..='\u{1FB3B}' => draw_sextant(canvas, ch),
        '\u{1FB3C}'..='\u{1FB67}' => smooth_mosaics::draw(canvas, ch),
        '\u{1FB68}'..='\u{1FB6F}' => triangles::draw_edge_triangle(canvas, ch),
        '\u{1FB70}'..='\u{1FB75}' => draw_vertical_eighth(canvas, ch),
        '\u{1FB76}'..='\u{1FB7B}' => draw_horizontal_eighth(canvas, ch),
        '\u{1FB7C}'..='\u{1FB7F}' => draw_corner_eighth(canvas, ch),
        '\u{1FB80}'..='\u{1FB81}' => draw_compound_eighth(canvas, ch),
        '\u{1FB82}'..='\u{1FB86}' => draw_upper_block(canvas, ch),
        '\u{1FB87}'..='\u{1FB8B}' => draw_right_block(canvas, ch),
        '\u{1FB8C}'..='\u{1FB94}' => draw_shade_block(canvas, ch),
        '\u{1FB95}'..='\u{1FB97}' => draw_checkerboard(canvas, ch),
        '\u{1FB98}'..='\u{1FB99}' => draw_diagonal_fill(canvas, ch),
        '\u{1FB9A}'..='\u{1FB9F}' => triangles::draw_combined(canvas, ch),
        _ => return false,
    }
    true
}

// -- Sextants (U+1FB00–U+1FB3B) --

/// Draw a sextant character: 2-column x 3-row block grid.
///
/// The codepoint maps to a 6-bit bitmask via `idx + (idx / 0x14) + 1`
/// (Ghostty's formula). Bits: tl, tr, ml, mr, bl, br.
fn draw_sextant(canvas: &mut Canvas, ch: char) {
    let w = canvas.width() as f32;
    let h = canvas.height() as f32;
    let hw = (w / 2.0).round();
    let th = (h / 3.0).round();
    let th2 = (h * 2.0 / 3.0).round();

    let idx = ch as u32 - 0x1FB00;
    let bits = (idx + (idx / 0x14) + 1) as u8;

    // bit 0 = tl, bit 1 = tr, bit 2 = ml, bit 3 = mr, bit 4 = bl, bit 5 = br
    if bits & 0b00_0001 != 0 {
        canvas.fill_rect(0.0, 0.0, hw, th, 255);
    }
    if bits & 0b00_0010 != 0 {
        canvas.fill_rect(hw, 0.0, w - hw, th, 255);
    }
    if bits & 0b00_0100 != 0 {
        canvas.fill_rect(0.0, th, hw, th2 - th, 255);
    }
    if bits & 0b00_1000 != 0 {
        canvas.fill_rect(hw, th, w - hw, th2 - th, 255);
    }
    if bits & 0b01_0000 != 0 {
        canvas.fill_rect(0.0, th2, hw, h - th2, 255);
    }
    if bits & 0b10_0000 != 0 {
        canvas.fill_rect(hw, th2, w - hw, h - th2, 255);
    }
}

// -- Vertical one-eighth blocks (U+1FB70–U+1FB75) --

/// Draw vertical stripe blocks (2nd through 7th eighth, left to right).
fn draw_vertical_eighth(canvas: &mut Canvas, ch: char) {
    let w = canvas.width() as f32;
    let h = canvas.height() as f32;
    let n = (ch as u32 - 0x1FB70 + 1) as f32;
    let x = (w * n / 8.0).round();
    let x1 = (w * (n + 1.0) / 8.0).round();
    canvas.fill_rect(x, 0.0, x1 - x, h, 255);
}

// -- Horizontal one-eighth blocks (U+1FB76–U+1FB7B) --

/// Draw horizontal stripe blocks (2nd through 7th eighth, top to bottom).
fn draw_horizontal_eighth(canvas: &mut Canvas, ch: char) {
    let w = canvas.width() as f32;
    let h = canvas.height() as f32;
    let n = (ch as u32 - 0x1FB76 + 1) as f32;
    let y = (h * n / 8.0).round();
    let y1 = (h * (n + 1.0) / 8.0).round();
    canvas.fill_rect(0.0, y, w, y1 - y, 255);
}

// -- Corner one-eighth blocks (U+1FB7C–U+1FB7F) --

/// Draw L-shaped corner eighth blocks.
fn draw_corner_eighth(canvas: &mut Canvas, ch: char) {
    let w = canvas.width() as f32;
    let h = canvas.height() as f32;
    let ew = (w / 8.0).round();
    let eh = (h / 8.0).round();

    match ch {
        '\u{1FB7C}' => {
            canvas.fill_rect(0.0, 0.0, ew, h, 255);
            canvas.fill_rect(0.0, h - eh, w, eh, 255);
        }
        '\u{1FB7D}' => {
            canvas.fill_rect(0.0, 0.0, ew, h, 255);
            canvas.fill_rect(0.0, 0.0, w, eh, 255);
        }
        '\u{1FB7E}' => {
            canvas.fill_rect(w - ew, 0.0, ew, h, 255);
            canvas.fill_rect(0.0, 0.0, w, eh, 255);
        }
        '\u{1FB7F}' => {
            canvas.fill_rect(w - ew, 0.0, ew, h, 255);
            canvas.fill_rect(0.0, h - eh, w, eh, 255);
        }
        _ => {}
    }
}

// -- Compound eighth blocks (U+1FB80–U+1FB81) --

/// Draw compound horizontal eighth blocks.
fn draw_compound_eighth(canvas: &mut Canvas, ch: char) {
    let w = canvas.width() as f32;
    let h = canvas.height() as f32;
    let eh = (h / 8.0).round();

    match ch {
        // Upper and lower one-eighth block.
        '\u{1FB80}' => {
            canvas.fill_rect(0.0, 0.0, w, eh, 255);
            canvas.fill_rect(0.0, h - eh, w, eh, 255);
        }
        // Horizontal one-eighth blocks 1-3-5-8.
        '\u{1FB81}' => {
            for n in [0.0, 2.0, 4.0, 7.0] {
                let y = (h * n / 8.0).round();
                let y1 = (h * (n + 1.0) / 8.0).round();
                canvas.fill_rect(0.0, y, w, y1 - y, 255);
            }
        }
        _ => {}
    }
}

// -- Upper fractional blocks (U+1FB82–U+1FB86) --

/// Draw upper N-fraction blocks (1/4, 3/8, 5/8, 3/4, 7/8).
fn draw_upper_block(canvas: &mut Canvas, ch: char) {
    let w = canvas.width() as f32;
    let h = canvas.height() as f32;
    let frac = match ch {
        '\u{1FB82}' => 0.25,
        '\u{1FB83}' => 0.375,
        '\u{1FB84}' => 0.625,
        '\u{1FB85}' => 0.75,
        '\u{1FB86}' => 0.875,
        _ => return,
    };
    canvas.fill_rect(0.0, 0.0, w, (h * frac).round(), 255);
}

// -- Right fractional blocks (U+1FB87–U+1FB8B) --

/// Draw right N-fraction blocks (1/4, 3/8, 5/8, 3/4, 7/8).
fn draw_right_block(canvas: &mut Canvas, ch: char) {
    let w = canvas.width() as f32;
    let h = canvas.height() as f32;
    let frac = match ch {
        '\u{1FB87}' => 0.25,
        '\u{1FB88}' => 0.375,
        '\u{1FB89}' => 0.625,
        '\u{1FB8A}' => 0.75,
        '\u{1FB8B}' => 0.875,
        _ => return,
    };
    let bw = (w * frac).round();
    canvas.fill_rect(w - bw, 0.0, bw, h, 255);
}

// -- Shade blocks (U+1FB8C–U+1FB94) --

/// Draw medium-shade half blocks and combinations.
fn draw_shade_block(canvas: &mut Canvas, ch: char) {
    let w = canvas.width() as f32;
    let h = canvas.height() as f32;
    let hw = (w / 2.0).round();
    let hh = (h / 2.0).round();

    match ch {
        '\u{1FB8C}' => canvas.fill_rect(0.0, 0.0, hw, h, 128),
        '\u{1FB8D}' => canvas.fill_rect(hw, 0.0, w - hw, h, 128),
        '\u{1FB8E}' => canvas.fill_rect(0.0, 0.0, w, hh, 128),
        '\u{1FB8F}' => canvas.fill_rect(0.0, hh, w, h - hh, 128),
        '\u{1FB90}' => canvas.fill_rect(0.0, 0.0, w, h, 128),
        '\u{1FB91}' => {
            canvas.fill_rect(0.0, 0.0, w, h, 128);
            canvas.fill_rect(0.0, 0.0, w, hh, 255);
        }
        '\u{1FB92}' => {
            canvas.fill_rect(0.0, 0.0, w, h, 128);
            canvas.fill_rect(0.0, hh, w, h - hh, 255);
        }
        // U+1FB93 is unallocated (Unicode hole) — falls through to no-op.
        '\u{1FB94}' => {
            canvas.fill_rect(0.0, 0.0, w, h, 128);
            canvas.fill_rect(hw, 0.0, w - hw, h, 255);
        }
        _ => {}
    }
}

// -- Checkerboard fills (U+1FB95–U+1FB97) --

/// Draw checkerboard and horizontal stripe fills.
fn draw_checkerboard(canvas: &mut Canvas, ch: char) {
    let w = canvas.width();
    let h = canvas.height();

    match ch {
        '\u{1FB95}' => checker_fill(canvas, w, h, 0),
        '\u{1FB96}' => checker_fill(canvas, w, h, 1),
        '\u{1FB97}' => {
            let bar = (h as f32 / 4.0).round() as u32;
            if bar == 0 {
                return;
            }
            for row_start in (0..h).step_by((bar * 2) as usize) {
                canvas.fill_rect(0.0, row_start as f32, w as f32, bar as f32, 255);
            }
        }
        _ => {}
    }
}

/// Fill canvas with checkerboard pattern.
fn checker_fill(canvas: &mut Canvas, w: u32, h: u32, parity: u32) {
    let cell = (w.min(h) / 4).max(1);
    for py in 0..h {
        for px in 0..w {
            if (px / cell + py / cell + parity).is_multiple_of(2) {
                canvas.blend_pixel(px as i32, py as i32, 255);
            }
        }
    }
}

// -- Diagonal fills (U+1FB98–U+1FB99) --

/// Draw diagonal line fill patterns.
fn draw_diagonal_fill(canvas: &mut Canvas, ch: char) {
    let w = canvas.width() as f32;
    let h = canvas.height() as f32;
    let thickness = (w / 8.0).max(1.0);
    let count = (w / (2.0 * thickness)).ceil() as i32;

    for i in -count..=count * 2 {
        let offset = i as f32 * 2.0 * thickness;
        match ch {
            '\u{1FB98}' => canvas.fill_line(offset, 0.0, w + offset, h, thickness),
            '\u{1FB99}' => canvas.fill_line(w - offset, 0.0, -offset, h, thickness),
            _ => {}
        }
    }
}
