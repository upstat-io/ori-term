//! Box drawing character rendering (U+2500–U+257F).
//!
//! Each character is decomposed into up to four segments from the cell center.
//! A 128-entry lookup table encodes `[left, right, up, down]` weights per char.
//! Rounded corners fall back to right-angle segments; diagonals use anti-aliased
//! line rendering via the canvas SDF path.

use super::{Canvas, LineF, RectF};

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

    draw_h_segment(
        canvas,
        left,
        SegmentSpec {
            to: cx,
            from: 0.0,
            center: cy,
            thin,
            thick,
        },
    );
    draw_h_segment(
        canvas,
        right,
        SegmentSpec {
            to: w,
            from: cx,
            center: cy,
            thin,
            thick,
        },
    );
    draw_v_segment(
        canvas,
        up,
        SegmentSpec {
            to: cy,
            from: 0.0,
            center: cx,
            thin,
            thick,
        },
    );
    draw_v_segment(
        canvas,
        down,
        SegmentSpec {
            to: h,
            from: cy,
            center: cx,
            thin,
            thick,
        },
    );

    true
}

/// Geometry + stroke weights for one box-drawing segment.
///
/// `from`/`to` are the segment endpoints along its axis (x for horizontal,
/// y for vertical); `center` is the cross-axis center (cy for horizontal,
/// cx for vertical). `thin`/`thick` are the light/heavy stroke widths.
#[derive(Clone, Copy)]
struct SegmentSpec {
    /// Segment end coordinate along its axis.
    to: f32,
    /// Segment start coordinate along its axis.
    from: f32,
    /// Cross-axis center coordinate.
    center: f32,
    /// Light stroke width.
    thin: f32,
    /// Heavy stroke width.
    thick: f32,
}

/// Draw a horizontal segment from `from` to `to` at vertical center.
fn draw_h_segment(canvas: &mut Canvas, weight: Weight, spec: SegmentSpec) {
    let SegmentSpec {
        to: to_x,
        from: from_x,
        center: cy,
        thin,
        thick,
    } = spec;
    let lx = from_x.min(to_x);
    let rx = from_x.max(to_x);
    let seg_w = rx - lx;
    if seg_w <= 0.0 {
        return;
    }
    match weight {
        Weight::None => {}
        Weight::Light => {
            canvas.fill_rect(RectF::new(lx, cy - (thin / 2.0).floor(), seg_w, thin), 255);
        }
        Weight::Heavy => {
            canvas.fill_rect(
                RectF::new(lx, cy - (thick / 2.0).floor(), seg_w, thick),
                255,
            );
        }
        Weight::Double => {
            let gap = (thin * 2.0).max(2.0);
            canvas.fill_rect(
                RectF::new(lx, cy - (gap / 2.0).floor() - thin, seg_w, thin),
                255,
            );
            canvas.fill_rect(RectF::new(lx, cy + (gap / 2.0).ceil(), seg_w, thin), 255);
        }
    }
}

/// Draw a vertical segment from `from` to `to` at horizontal center.
fn draw_v_segment(canvas: &mut Canvas, weight: Weight, spec: SegmentSpec) {
    let SegmentSpec {
        to: to_y,
        from: from_y,
        center: cx,
        thin,
        thick,
    } = spec;
    let ty = from_y.min(to_y);
    let by = from_y.max(to_y);
    let seg_h = by - ty;
    if seg_h <= 0.0 {
        return;
    }
    match weight {
        Weight::None => {}
        Weight::Light => {
            canvas.fill_rect(RectF::new(cx - (thin / 2.0).floor(), ty, thin, seg_h), 255);
        }
        Weight::Heavy => {
            canvas.fill_rect(
                RectF::new(cx - (thick / 2.0).floor(), ty, thick, seg_h),
                255,
            );
        }
        Weight::Double => {
            let gap = (thin * 2.0).max(2.0);
            canvas.fill_rect(
                RectF::new(cx - (gap / 2.0).floor() - thin, ty, thin, seg_h),
                255,
            );
            canvas.fill_rect(RectF::new(cx + (gap / 2.0).ceil(), ty, thin, seg_h), 255);
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

    let h_to_right = SegmentSpec {
        to: w,
        from: cx,
        center: cy,
        thin,
        thick,
    };
    let h_to_left = SegmentSpec {
        to: cx,
        from: 0.0,
        center: cy,
        thin,
        thick,
    };
    let v_to_bottom = SegmentSpec {
        to: h,
        from: cy,
        center: cx,
        thin,
        thick,
    };
    let v_to_top = SegmentSpec {
        to: cy,
        from: 0.0,
        center: cx,
        thin,
        thick,
    };

    match ch {
        '\u{256D}' => {
            draw_h_segment(canvas, Weight::Light, h_to_right);
            draw_v_segment(canvas, Weight::Light, v_to_bottom);
        }
        '\u{256E}' => {
            draw_h_segment(canvas, Weight::Light, h_to_left);
            draw_v_segment(canvas, Weight::Light, v_to_bottom);
        }
        '\u{256F}' => {
            draw_h_segment(canvas, Weight::Light, h_to_left);
            draw_v_segment(canvas, Weight::Light, v_to_top);
        }
        '\u{2570}' => {
            draw_h_segment(canvas, Weight::Light, h_to_right);
            draw_v_segment(canvas, Weight::Light, v_to_top);
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
            canvas.fill_line(LineF::new(w, 0.0, 0.0, h), thin);
        }
        '\u{2572}' => {
            // ╲ upper left to lower right.
            canvas.fill_line(LineF::new(0.0, 0.0, w, h), thin);
        }
        '\u{2573}' => {
            // ╳ diagonal cross (both diagonals).
            canvas.fill_line(LineF::new(w, 0.0, 0.0, h), thin);
            canvas.fill_line(LineF::new(0.0, 0.0, w, h), thin);
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
