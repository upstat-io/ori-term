//! Box drawing character rendering (U+2500–U+257F).
//!
//! Each character is decomposed into up to four segments from the cell center.
//! A 128-entry lookup table encodes `[left, right, up, down]` weights per char.
//! Rounded corners fall back to right-angle segments; diagonals use anti-aliased
//! line rendering via the canvas SDF path.

use super::Canvas;

/// Segment weight for box drawing lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum Weight {
    None = 0,
    Light = 1,
    Heavy = 2,
    Double = 3,
}

impl Weight {
    fn from_byte(b: u8) -> Self {
        match b {
            1 => Self::Light,
            2 => Self::Heavy,
            3 => Self::Double,
            _ => Self::None,
        }
    }

    fn is_some(self) -> bool {
        !matches!(self, Self::None)
    }
}

/// Draw a box drawing character onto the canvas. Returns `true` if handled.
pub(super) fn draw_box(canvas: &mut Canvas, ch: char) -> bool {
    let idx = ch as u32 - 0x2500;

    // Rounded corners (U+256D–U+2570): render as right-angle segments.
    if (0x6D..=0x70).contains(&idx) {
        return draw_rounded_corner(canvas, ch);
    }

    // Diagonals (U+2571–U+2573): anti-aliased lines.
    if (0x71..=0x73).contains(&idx) {
        return draw_diagonal(canvas, ch);
    }

    let [left, right, up, down] = box_segments(ch);
    if !left.is_some() && !right.is_some() && !up.is_some() && !down.is_some() {
        return false;
    }

    let w = canvas.width() as f32;
    let h = canvas.height() as f32;
    let cx = (w / 2.0).floor();
    let cy = (h / 2.0).floor();
    let thin = 1.0f32.max((w / 8.0).round());
    let thick = (thin * 3.0).min(w / 2.0);

    draw_h_segment(canvas, left, cx, 0.0, cy, thin, thick);
    draw_h_segment(canvas, right, w, cx, cy, thin, thick);
    draw_v_segment(canvas, up, cy, 0.0, cx, thin, thick);
    draw_v_segment(canvas, down, h, cy, cx, thin, thick);

    true
}

/// Draw a horizontal segment from `from_x` to `to_x` at vertical center `cy`.
#[expect(
    clippy::too_many_arguments,
    reason = "line drawing primitives: canvas, weight, endpoints, thickness"
)]
fn draw_h_segment(
    canvas: &mut Canvas,
    weight: Weight,
    to_x: f32,
    from_x: f32,
    cy: f32,
    thin: f32,
    thick: f32,
) {
    let lx = from_x.min(to_x);
    let rx = from_x.max(to_x);
    let seg_w = rx - lx;
    if seg_w <= 0.0 {
        return;
    }
    match weight {
        Weight::None => {}
        Weight::Light => {
            canvas.fill_rect(lx, cy - (thin / 2.0).floor(), seg_w, thin, 255);
        }
        Weight::Heavy => {
            canvas.fill_rect(lx, cy - (thick / 2.0).floor(), seg_w, thick, 255);
        }
        Weight::Double => {
            let gap = (thin * 2.0).max(2.0);
            canvas.fill_rect(lx, cy - (gap / 2.0).floor() - thin, seg_w, thin, 255);
            canvas.fill_rect(lx, cy + (gap / 2.0).ceil(), seg_w, thin, 255);
        }
    }
}

/// Draw a vertical segment from `from_y` to `to_y` at horizontal center `cx`.
#[expect(
    clippy::too_many_arguments,
    reason = "line drawing primitives: canvas, weight, endpoints, thickness"
)]
fn draw_v_segment(
    canvas: &mut Canvas,
    weight: Weight,
    to_y: f32,
    from_y: f32,
    cx: f32,
    thin: f32,
    thick: f32,
) {
    let ty = from_y.min(to_y);
    let by = from_y.max(to_y);
    let seg_h = by - ty;
    if seg_h <= 0.0 {
        return;
    }
    match weight {
        Weight::None => {}
        Weight::Light => {
            canvas.fill_rect(cx - (thin / 2.0).floor(), ty, thin, seg_h, 255);
        }
        Weight::Heavy => {
            canvas.fill_rect(cx - (thick / 2.0).floor(), ty, thick, seg_h, 255);
        }
        Weight::Double => {
            let gap = (thin * 2.0).max(2.0);
            canvas.fill_rect(cx - (gap / 2.0).floor() - thin, ty, thin, seg_h, 255);
            canvas.fill_rect(cx + (gap / 2.0).ceil(), ty, thin, seg_h, 255);
        }
    }
}

/// Decode the segment table for a box drawing character.
fn box_segments(ch: char) -> [Weight; 4] {
    let idx = (ch as u32 - 0x2500) as usize;
    if idx >= BOX_DRAWING_TABLE.len() {
        return [Weight::None; 4];
    }
    let row = BOX_DRAWING_TABLE[idx];
    [
        Weight::from_byte(row[0]),
        Weight::from_byte(row[1]),
        Weight::from_byte(row[2]),
        Weight::from_byte(row[3]),
    ]
}

/// Draw rounded corners (U+256D–U+2570) as right-angle segments.
fn draw_rounded_corner(canvas: &mut Canvas, ch: char) -> bool {
    let w = canvas.width() as f32;
    let h = canvas.height() as f32;
    let thin = 1.0f32.max((w / 8.0).round());
    let thick = thin * 3.0;
    let cx = (w / 2.0).floor();
    let cy = (h / 2.0).floor();

    match ch {
        '\u{256D}' => {
            draw_h_segment(canvas, Weight::Light, w, cx, cy, thin, thick);
            draw_v_segment(canvas, Weight::Light, h, cy, cx, thin, thick);
        }
        '\u{256E}' => {
            draw_h_segment(canvas, Weight::Light, cx, 0.0, cy, thin, thick);
            draw_v_segment(canvas, Weight::Light, h, cy, cx, thin, thick);
        }
        '\u{256F}' => {
            draw_h_segment(canvas, Weight::Light, cx, 0.0, cy, thin, thick);
            draw_v_segment(canvas, Weight::Light, cy, 0.0, cx, thin, thick);
        }
        '\u{2570}' => {
            draw_h_segment(canvas, Weight::Light, w, cx, cy, thin, thick);
            draw_v_segment(canvas, Weight::Light, cy, 0.0, cx, thin, thick);
        }
        _ => return false,
    }
    true
}

/// Draw diagonal lines (U+2571–U+2573) with anti-aliased rendering.
fn draw_diagonal(canvas: &mut Canvas, ch: char) -> bool {
    let w = canvas.width() as f32;
    let h = canvas.height() as f32;
    let thin = 1.0f32.max((w / 8.0).round());

    match ch {
        '\u{2571}' => {
            // ╱ upper right to lower left.
            canvas.fill_line(w, 0.0, 0.0, h, thin);
        }
        '\u{2572}' => {
            // ╲ upper left to lower right.
            canvas.fill_line(0.0, 0.0, w, h, thin);
        }
        '\u{2573}' => {
            // ╳ diagonal cross (both diagonals).
            canvas.fill_line(w, 0.0, 0.0, h, thin);
            canvas.fill_line(0.0, 0.0, w, h, thin);
        }
        _ => return false,
    }
    true
}

// Table: [left, right, up, down] for U+2500..U+257F (128 entries).
// 0 = none, 1 = light, 2 = heavy, 3 = double.
#[rustfmt::skip]
const BOX_DRAWING_TABLE: [[u8; 4]; 128] = [
    // U+2500–U+250F
    [1,1,0,0], [2,2,0,0], [0,0,1,1], [0,0,2,2],
    [1,1,0,0], [2,2,0,0], [0,0,1,1], [0,0,2,2],
    [1,1,0,0], [2,2,0,0], [0,0,1,1], [0,0,2,2],
    [0,1,0,1], [0,2,0,1], [0,1,0,2], [0,2,0,2],
    // U+2510–U+251F
    [1,0,0,1], [2,0,0,1], [1,0,0,2], [2,0,0,2],
    [0,1,1,0], [0,2,1,0], [0,1,2,0], [0,2,2,0],
    [1,0,1,0], [2,0,1,0], [1,0,2,0], [2,0,2,0],
    [0,1,1,1], [0,2,1,1], [0,1,2,1], [0,1,1,2],
    // U+2520–U+252F
    [0,1,2,2], [0,2,2,1], [0,2,1,2], [0,2,2,2],
    [1,0,1,1], [2,0,1,1], [1,0,2,1], [1,0,1,2],
    [1,0,2,2], [2,0,2,1], [2,0,1,2], [2,0,2,2],
    [1,1,0,1], [2,1,0,1], [1,2,0,1], [2,2,0,1],
    // U+2530–U+253F
    [1,1,0,2], [2,1,0,2], [1,2,0,2], [2,2,0,2],
    [1,1,1,0], [2,1,1,0], [1,2,1,0], [2,2,1,0],
    [1,1,2,0], [2,1,2,0], [1,2,2,0], [2,2,2,0],
    [1,1,1,1], [2,1,1,1], [1,2,1,1], [2,2,1,1],
    // U+2540–U+254F
    [1,1,2,1], [1,1,1,2], [1,1,2,2], [2,1,2,1],
    [1,2,2,1], [2,1,1,2], [1,2,1,2], [2,2,2,1],
    [2,2,1,2], [2,1,2,2], [1,2,2,2], [2,2,2,2],
    [1,1,0,0], [2,2,0,0], [0,0,1,1], [0,0,2,2],
    // U+2550–U+255F
    [3,3,0,0], [0,0,3,3], [0,1,0,3], [0,3,0,1],
    [0,3,0,3], [1,0,0,3], [3,0,0,1], [3,0,0,3],
    [0,1,3,0], [0,3,1,0], [0,3,3,0], [1,0,3,0],
    [3,0,1,0], [3,0,3,0], [0,1,3,3], [0,3,1,1],
    // U+2560–U+256F
    [0,3,3,3], [1,0,3,3], [3,0,1,1], [3,0,3,3],
    [1,1,0,3], [3,3,0,1], [3,3,0,3], [1,1,3,0],
    [3,3,1,0], [3,3,3,0], [1,1,3,3], [3,3,1,1],
    [3,3,3,3], [0,0,0,0], [0,0,0,0], [0,0,0,0],
    // U+2570–U+257F
    [0,0,0,0], [0,0,0,0], [0,0,0,0], [0,0,0,0],
    [1,0,0,0], [0,0,1,0], [0,1,0,0], [0,0,0,1],
    [2,0,0,0], [0,0,2,0], [0,2,0,0], [0,0,0,2],
    [1,2,0,0], [0,0,1,2], [2,1,0,0], [0,0,2,1],
];
