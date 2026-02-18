//! Text decoration rendering: underlines (single, double, curly, dotted, dashed)
//! and strikethrough.
//!
//! Each decoration is emitted as one or more solid-color rectangles into the
//! background instance buffer. The caller resolves underline color (SGR 58
//! override or foreground fallback) and passes it in.

use oriterm_core::{CellFlags, Rgb};

use crate::gpu::instance_writer::InstanceWriter;

/// Emit underline and strikethrough decorations for a single cell.
///
/// Fast-path: returns immediately when no decoration flags are set.
/// Underlines and strikethrough are independent — both can coexist on
/// the same cell.
pub(super) fn draw_decorations(
    backgrounds: &mut InstanceWriter,
    flags: CellFlags,
    underline_color: Option<Rgb>,
    fg: Rgb,
    x: f32,
    y: f32,
    cell_width: f32,
    cell_height: f32,
) {
    let has_underline = flags.intersects(CellFlags::ALL_UNDERLINES);
    let has_strikethrough = flags.contains(CellFlags::STRIKETHROUGH);

    if !has_underline && !has_strikethrough {
        return;
    }

    if has_underline {
        let color = underline_color.unwrap_or(fg);
        let underline_y = y + cell_height - 2.0;
        draw_underline(backgrounds, flags, color, x, underline_y, cell_width);
    }

    if has_strikethrough {
        let strike_y = y + cell_height / 2.0;
        backgrounds.push_rect(x, strike_y, cell_width, 1.0, fg, 1.0);
    }
}

/// Dispatch to the appropriate underline style.
///
/// Priority matches the old implementation: curly > double > dotted > dashed > single.
fn draw_underline(
    bg: &mut InstanceWriter,
    flags: CellFlags,
    color: Rgb,
    x: f32,
    y: f32,
    w: f32,
) {
    if flags.contains(CellFlags::CURLY_UNDERLINE) {
        draw_curly_underline(bg, color, x, y, w);
    } else if flags.contains(CellFlags::DOUBLE_UNDERLINE) {
        draw_double_underline(bg, color, x, y, w);
    } else if flags.contains(CellFlags::DOTTED_UNDERLINE) {
        draw_dotted_underline(bg, color, x, y, w);
    } else if flags.contains(CellFlags::DASHED_UNDERLINE) {
        draw_dashed_underline(bg, color, x, y, w);
    } else {
        // Single underline (plain UNDERLINE flag).
        bg.push_rect(x, y, w, 1.0, color, 1.0);
    }
}

/// Curly underline: per-pixel sine wave, amplitude 2px, period = cell width.
fn draw_curly_underline(bg: &mut InstanceWriter, color: Rgb, x: f32, y: f32, w: f32) {
    let steps = w as usize;
    for dx in 0..steps {
        let phase = (dx as f32 / w) * std::f32::consts::TAU;
        let offset = (phase.sin() * 2.0).round();
        bg.push_rect(x + dx as f32, y + offset, 1.0, 1.0, color, 1.0);
    }
}

/// Double underline: two 1px lines, 2px apart.
fn draw_double_underline(bg: &mut InstanceWriter, color: Rgb, x: f32, y: f32, w: f32) {
    bg.push_rect(x, y, w, 1.0, color, 1.0);
    bg.push_rect(x, y - 2.0, w, 1.0, color, 1.0);
}

/// Dotted underline: 1px on, 1px off.
fn draw_dotted_underline(bg: &mut InstanceWriter, color: Rgb, x: f32, y: f32, w: f32) {
    let steps = w as usize;
    for dx in (0..steps).step_by(2) {
        bg.push_rect(x + dx as f32, y, 1.0, 1.0, color, 1.0);
    }
}

/// Dashed underline: 3px on, 2px off (pattern period = 5).
fn draw_dashed_underline(bg: &mut InstanceWriter, color: Rgb, x: f32, y: f32, w: f32) {
    let steps = w as usize;
    for dx in 0..steps {
        if dx % 5 < 3 {
            bg.push_rect(x + dx as f32, y, 1.0, 1.0, color, 1.0);
        }
    }
}
