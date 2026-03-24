//! Text and icon draw command conversion to GPU glyph instances.
//!
//! Extracted from the parent module to keep `mod.rs` under the 500-line limit.

use oriterm_ui::color::Color;
use oriterm_ui::geometry::{Point, Rect};
use oriterm_ui::text::ShapedText;

use crate::font::{FaceIdx, FontRealm, RasterKey, SyntheticFlags, subpx_bin, subpx_offset};
use crate::gpu::atlas::{AtlasEntry, AtlasKind};
use crate::gpu::instance_writer::ScreenRect;

use super::TextContext;

/// Convert a text draw command into glyph instances.
///
/// The text position is in logical pixels (from widget layout). Glyph
/// advances, offsets, bearings, and bitmap dimensions are in physical pixels
/// (from the font collection loaded at physical DPI). We scale the position
/// to physical at the start, then work entirely in physical pixel space —
/// no scaling of glyph bitmap dimensions, which would cause blurriness.
#[expect(
    clippy::too_many_arguments,
    reason = "text conversion: position, shaped, color, bg_hint, text context, scale, opacity, clip"
)]
pub(super) fn convert_text(
    position: Point,
    shaped: &ShapedText,
    color: Color,
    bg_hint: Option<Color>,
    ctx: &mut TextContext<'_>,
    scale: f32,
    opacity: f32,
    clip: [f32; 4],
) {
    let fg = color_to_rgb(color);
    let subpixel_bg = bg_hint.map(color_to_rgb);
    let alpha = color.a * opacity;
    let baseline = shaped.baseline;

    // Convert logical position to physical. All subsequent values
    // (advances, offsets, bearings, bitmap dims) are already physical.
    //
    // Round base_y to an integer pixel boundary. baseline and bearing_y
    // are already integers, so rounding base_y ensures every glyph's
    // screen rect has integer Y coordinates. Without this, fractional
    // positions (e.g. from centering a dialog in the viewport) cause the
    // bilinear atlas sampler to interpolate the bottom row of glyph pixels
    // with transparent atlas padding, producing a "cut off at bottom" artifact.
    let mut cursor_x = position.x * scale;
    let base_y = (position.y * scale).round();

    for glyph in &shaped.glyphs {
        let advance = glyph.x_advance;

        // Skip advance-only glyphs (spaces: glyph_id=0).
        if glyph.glyph_id == 0 {
            cursor_x += advance;
            continue;
        }

        let subpx = subpx_bin(cursor_x + glyph.x_offset);
        let key = RasterKey {
            glyph_id: glyph.glyph_id,
            face_idx: FaceIdx(glyph.face_index),
            weight: shaped.weight,
            size_q6: shaped.size_q6,
            synthetic: SyntheticFlags::from_bits_truncate(glyph.synthetic),
            hinted: ctx.hinted,
            subpx_x: subpx,
            font_realm: FontRealm::Ui,
        };

        if let Some(entry) = ctx.atlas.lookup_key(key) {
            emit_text_glyph(
                cursor_x,
                base_y,
                baseline,
                glyph,
                entry,
                fg,
                subpixel_bg,
                alpha,
                subpx,
                ctx,
                clip,
            );
        }

        cursor_x += advance;
    }
}

/// Emit a single text glyph instance, routing by atlas kind.
///
/// All coordinates are in physical pixels — no scale factor needed. The
/// glyph bitmap dimensions come directly from the atlas entry (rasterized
/// at the font's physical pixel size).
#[expect(
    clippy::too_many_arguments,
    reason = "text glyph instance: position, glyph data, atlas entry, color, bg, clip"
)]
fn emit_text_glyph(
    cursor_x: f32,
    base_y: f32,
    baseline: f32,
    glyph: &oriterm_ui::text::ShapedGlyph,
    entry: &AtlasEntry,
    fg: oriterm_core::Rgb,
    subpixel_bg: Option<oriterm_core::Rgb>,
    alpha: f32,
    subpx: u8,
    ctx: &mut TextContext<'_>,
    clip: [f32; 4],
) {
    let absorbed = subpx_offset(subpx);
    let gx = cursor_x + glyph.x_offset - absorbed + entry.bearing_x as f32;
    let gy = base_y + baseline - entry.bearing_y as f32 - glyph.y_offset;
    let uv = [entry.uv_x, entry.uv_y, entry.uv_w, entry.uv_h];
    // All values are physical pixels — no scaling needed.
    let rect = ScreenRect {
        x: gx,
        y: gy,
        w: entry.width as f32,
        h: entry.height as f32,
    };

    match entry.kind {
        AtlasKind::Subpixel => {
            if let Some(bg) = subpixel_bg {
                // Known background — per-channel compositing in the shader.
                ctx.subpixel_writer
                    .push_glyph_with_bg(rect, uv, fg, bg, alpha, entry.page, clip);
            } else {
                // No background hint — fall back to alpha blending.
                ctx.subpixel_writer
                    .push_glyph(rect, uv, fg, alpha, entry.page, clip);
            }
        }
        AtlasKind::Mono => {
            ctx.mono_writer
                .push_glyph(rect, uv, fg, alpha, entry.page, clip);
        }
        AtlasKind::Color => {
            ctx.color_writer
                .push_glyph(rect, uv, fg, alpha, entry.page, clip);
        }
    }
}

/// Convert an icon draw command into a mono glyph instance.
///
/// The icon bitmap lives in the mono atlas and is tinted to `color`
/// by the `fg.wgsl` shader (same as monochrome text glyphs).
#[expect(
    clippy::too_many_arguments,
    reason = "icon conversion: rect, atlas_page, uv, color, text context, scale, opacity, clip"
)]
pub(super) fn convert_icon(
    rect: Rect,
    atlas_page: u32,
    uv: [f32; 4],
    color: Color,
    ctx: &mut TextContext<'_>,
    scale: f32,
    opacity: f32,
    clip: [f32; 4],
) {
    let fg = color_to_rgb(color);
    let alpha = color.a * opacity;

    // Snap icon quad to integer physical pixels so each texel maps 1:1 to a
    // screen pixel. Without this, bilinear atlas sampling blurs the icon.
    let x = (rect.x() * scale).round();
    let y = (rect.y() * scale).round();
    let w = (rect.width() * scale).round();
    let h = (rect.height() * scale).round();
    let screen = ScreenRect { x, y, w, h };
    ctx.mono_writer
        .push_glyph(screen, uv, fg, alpha, atlas_page, clip);
}

/// Convert an [`oriterm_ui::color::Color`] (f32 RGBA) to [`oriterm_core::Rgb`] (u8 RGB).
pub(super) fn color_to_rgb(c: Color) -> oriterm_core::Rgb {
    oriterm_core::Rgb {
        r: (c.r * 255.0).round() as u8,
        g: (c.g * 255.0).round() as u8,
        b: (c.b * 255.0).round() as u8,
    }
}
