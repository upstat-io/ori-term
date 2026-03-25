//! Vector icon rasterization via `tiny_skia`.
//!
//! Converts normalized [`IconPath`] definitions into alpha-only bitmaps
//! at a requested pixel size, suitable for upload to the monochrome glyph
//! atlas. Color is applied at draw time by the shader's `fg_color` attribute.

mod cache;

use oriterm_ui::icons::{IconPath, IconStyle, PathCommand};

pub(crate) use cache::IconCache;

/// Rasterize an icon path into an alpha-only bitmap.
///
/// `size_px` is the bitmap dimension in **physical pixels**. `scale` is the
/// display scale factor (e.g. 1.5 for 150% DPI) — used to convert
/// [`IconStyle::Stroke`] widths from logical to physical pixels.
///
/// Returns `size_px × size_px` bytes of R8 alpha data (0 = transparent,
/// 255 = opaque). The icon is rendered white-on-transparent and the alpha
/// channel is extracted.
pub(crate) fn rasterize_icon(icon: &IconPath, size_px: u32, scale: f32) -> Vec<u8> {
    rasterize_commands(icon.commands, icon.style, size_px, scale)
}

/// Rasterize path commands into an alpha-only bitmap.
///
/// Same as [`rasterize_icon`] but accepts commands and style separately,
/// allowing rasterization of dynamically-generated path data (e.g. from
/// SVG import) without requiring a `&'static` lifetime.
pub(crate) fn rasterize_commands(
    commands: &[PathCommand],
    style: IconStyle,
    size_px: u32,
    scale: f32,
) -> Vec<u8> {
    if size_px == 0 {
        return Vec::new();
    }

    let w = size_px;
    let h = size_px;
    let mut pixmap =
        tiny_skia::Pixmap::new(w, h).expect("icon size_px must be > 0 and <= i32::MAX");

    let px = size_px as f32;
    let path = build_path(commands, px);

    let paint = {
        let mut p = tiny_skia::Paint::default();
        p.set_color_rgba8(255, 255, 255, 255);
        p.anti_alias = true;
        p
    };

    match style {
        IconStyle::Stroke(logical_width) => {
            // Stroke width is in logical pixels → multiply by scale for physical.
            let stroke = tiny_skia::Stroke {
                width: logical_width * scale,
                line_cap: tiny_skia::LineCap::Round,
                line_join: tiny_skia::LineJoin::Round,
                ..tiny_skia::Stroke::default()
            };
            pixmap.stroke_path(
                &path,
                &paint,
                &stroke,
                tiny_skia::Transform::identity(),
                None,
            );
        }
        IconStyle::Fill => {
            pixmap.fill_path(
                &path,
                &paint,
                tiny_skia::FillRule::Winding,
                tiny_skia::Transform::identity(),
                None,
            );
        }
    }

    // Extract alpha channel (every 4th byte starting at offset 3 in RGBA).
    pixmap.data().iter().skip(3).step_by(4).copied().collect()
}

/// Build a `tiny_skia::Path` from normalized path commands scaled to pixel size.
fn build_path(commands: &[PathCommand], scale: f32) -> tiny_skia::Path {
    let mut builder = tiny_skia::PathBuilder::new();
    for cmd in commands {
        match *cmd {
            PathCommand::MoveTo(x, y) => builder.move_to(x * scale, y * scale),
            PathCommand::LineTo(x, y) => builder.line_to(x * scale, y * scale),
            PathCommand::CubicTo(cx1, cy1, cx2, cy2, x, y) => {
                builder.cubic_to(
                    cx1 * scale,
                    cy1 * scale,
                    cx2 * scale,
                    cy2 * scale,
                    x * scale,
                    y * scale,
                );
            }
            PathCommand::Close => builder.close(),
        }
    }
    builder.finish().unwrap_or_else(|| {
        // Empty path fallback — shouldn't happen with validated icons.
        let mut b = tiny_skia::PathBuilder::new();
        b.move_to(0.0, 0.0);
        b.finish().expect("trivial path must build")
    })
}

#[cfg(test)]
mod tests;
