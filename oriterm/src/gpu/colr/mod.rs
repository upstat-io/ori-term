//! COLR v1 CPU compositing: rasterizes paint commands into RGBA bitmaps.
//!
//! Takes the [`PaintCommand`] list from the [`PaintCollector`] and composites
//! each layer on the CPU into a premultiplied RGBA bitmap. The result is
//! inserted into the color atlas as a [`RasterizedGlyph`] with
//! [`GlyphFormat::Color`], indistinguishable from sbix/CBDT color emoji.
//!
//! This approach handles the vast majority of real-world COLR v1 emoji (Segoe
//! UI Emoji, Noto Color Emoji v2) which consist of `FillGlyph` + `Solid` brush
//! layers. Gradient fills are composited per-pixel on the CPU.

mod compose;

use swash::scale::ScaleContext;

use crate::font::collection::colr_v1::ClipBox;
use crate::font::collection::colr_v1::rasterize::collect_colr_v1;
use crate::font::collection::{FaceData, font_ref};
use crate::font::{GlyphFormat, RasterizedGlyph};

/// Try to rasterize a COLR v1 glyph via CPU compositing.
///
/// Returns a [`RasterizedGlyph`] with `GlyphFormat::Color` if the glyph has
/// COLR v1 data, or `None` to fall back to the normal swash path.
///
/// The glyph outline for each layer is rasterized via swash (alpha mask only),
/// then composited with the resolved brush color onto the output RGBA buffer.
pub(crate) fn try_rasterize_colr_v1(
    fd: &FaceData,
    glyph_id: u16,
    size_px: f32,
    variations: &[(&str, f32)],
    ctx: &mut ScaleContext,
) -> Option<RasterizedGlyph> {
    let colr = collect_colr_v1(&fd.bytes, fd.face_index, glyph_id, size_px)?;
    log::info!(
        "COLR glyph {glyph_id}: {} commands, clip_box={:?}",
        colr.commands.len(),
        colr.clip_box
    );

    // Determine output dimensions from clip box.
    let clip = colr.clip_box.unwrap_or_else(|| {
        // Fallback: estimate from font metrics.
        estimate_clip_box(fd, glyph_id, size_px)
    });

    let width = clip.width().ceil() as u32;
    let height = clip.height().ceil() as u32;
    if width == 0 || height == 0 {
        return None;
    }

    // RGBA buffer (premultiplied, initially transparent).
    let mut bitmap = vec![0u8; (width * height * 4) as usize];

    // Composite each paint command.
    compose::composite_commands(
        &colr.commands,
        &mut bitmap,
        width,
        height,
        clip,
        fd,
        size_px,
        variations,
        ctx,
    );

    // Bearing: offset from glyph origin to top-left of bitmap.
    let bearing_x = clip.x_min.floor() as i32;
    let bearing_y = clip.y_max.ceil() as i32;

    Some(RasterizedGlyph {
        width,
        height,
        bearing_x,
        bearing_y,
        advance: 0.0,
        format: GlyphFormat::Color,
        bitmap,
    })
}

/// Estimate a clip box from font metrics when no COLR v1 clip box is defined.
///
/// Uses the glyph advance width and font ascent/descent as a rough bounding
/// box. Most COLR v1 fonts define clip boxes, so this is a rare fallback.
fn estimate_clip_box(fd: &FaceData, glyph_id: u16, size_px: f32) -> ClipBox {
    let fr = font_ref(fd);
    let metrics = fr.metrics(&[]).scale(size_px);
    let glyph_metrics = fr.glyph_metrics(&[]).scale(size_px);
    let advance = glyph_metrics.advance_width(glyph_id);
    ClipBox {
        x_min: 0.0,
        y_min: -metrics.descent.abs(),
        x_max: advance,
        y_max: metrics.ascent,
    }
}

#[cfg(test)]
mod tests;
