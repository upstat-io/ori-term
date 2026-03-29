//! COLR v1 detection, paint collection, and CPU rasterization.
//!
//! Detects COLR v1 glyphs via skrifa, collects paint commands, and composites
//! them on the CPU via tiny-skia into premultiplied RGBA bitmaps. The result
//! is inserted into the glyph cache as a [`RasterizedGlyph`] with
//! [`GlyphFormat::Color`], indistinguishable from sbix/CBDT color emoji.

use skrifa::instance::Size;
use skrifa::{FontRef, MetadataProvider};

use super::{ClipBox, ColrV1Glyph, PaintCollector, load_palette};
use crate::font::GlyphFormat;
use crate::font::collection::{FaceData, RasterizedGlyph, font_ref};

/// Check whether a font face contains a COLR color glyph (v0 or v1).
#[allow(
    dead_code,
    reason = "diagnostic predicate for tests and future per-glyph detection"
)]
pub(crate) fn has_colr(font_bytes: &[u8], face_index: u32, glyph_id: u16) -> bool {
    let Some(font) = FontRef::from_index(font_bytes, face_index).ok() else {
        return false;
    };
    font.color_glyphs()
        .get(skrifa::GlyphId::new(u32::from(glyph_id)))
        .is_some()
}

/// Collect the COLR paint tree for a glyph (v0 or v1).
///
/// Returns `None` if the glyph has no COLR data or if paint collection
/// fails. The returned [`ColrV1Glyph`] contains the paint commands and an
/// optional clip box (in font units, unscaled).
///
/// `size_px` is used to compute the clip box in pixel coordinates. The paint
/// commands themselves operate in font units — the compositor applies a
/// `size_px / upem` scaling transform during rendering.
/// Get just the COLR clip box for a glyph without collecting paint commands.
///
/// Returns the padded clip box in pixel coordinates, or `None` if the glyph
/// has no COLR data. Used to determine the correct canvas size for swash's
/// color rendering (swash clips to outline bounds, not COLR bounds).
pub(crate) fn colr_clip_box(fd: &FaceData, glyph_id: u16, size_px: f32) -> Option<ClipBox> {
    let font = FontRef::from_index(&fd.bytes, fd.face_index).ok()?;
    let gid = skrifa::GlyphId::new(u32::from(glyph_id));
    let color_glyph = font.color_glyphs().get(gid)?;
    let size = Size::new(size_px);
    let bb = color_glyph.bounding_box(skrifa::instance::LocationRef::default(), size)?;
    // Pad by 10% to capture any overflow beyond the declared bounds.
    let pad = (bb.y_max - bb.y_min) * 0.1;
    Some(ClipBox {
        x_min: bb.x_min - pad,
        y_min: bb.y_min - pad,
        x_max: bb.x_max + pad,
        y_max: bb.y_max + pad,
    })
}

pub(crate) fn collect_colr_v1(
    font_bytes: &[u8],
    face_index: u32,
    glyph_id: u16,
    size_px: f32,
) -> Option<ColrV1Glyph> {
    let font = FontRef::from_index(font_bytes, face_index).ok()?;
    let gid = skrifa::GlyphId::new(u32::from(glyph_id));
    // Try v1 first (has clip boxes, richer paint trees), fall back to v0.
    let color_glyph = font.color_glyphs().get(gid)?;

    let size = Size::new(size_px);

    // Get the clip box (scaled to pixels) if available.
    let clip_box = color_glyph
        .bounding_box(skrifa::instance::LocationRef::default(), size)
        .map(|bb| ClipBox {
            x_min: bb.x_min,
            y_min: bb.y_min,
            x_max: bb.x_max,
            y_max: bb.y_max,
        });

    // Collect paint commands.
    let palette = load_palette(&font);
    let mut collector = PaintCollector::new(palette);
    if color_glyph
        .paint(skrifa::instance::LocationRef::default(), &mut collector)
        .is_err()
    {
        log::warn!("COLR v1 paint collection failed for glyph {glyph_id}");
        return None;
    }

    let commands = collector.into_commands();
    if commands.is_empty() {
        return None;
    }

    Some(ColrV1Glyph { commands, clip_box })
}

/// Try to rasterize a COLR v1 glyph via CPU compositing.
///
/// Returns a [`RasterizedGlyph`] with `GlyphFormat::Color` if the glyph has
/// COLR data, or `None` to fall back to the normal swash path.
///
/// Glyph outlines are extracted via skrifa and composited with tiny-skia,
/// which provides proper path clipping, gradient fills, and layer compositing.
#[cfg(any())] // Compositor disabled — only colr_clip_box is used.
pub(crate) fn try_rasterize_colr_v1(
    fd: &FaceData,
    glyph_id: u16,
    size_px: f32,
) -> Option<RasterizedGlyph> {
    let colr = collect_colr_v1(&fd.bytes, fd.face_index, glyph_id, size_px)?;
    log::debug!(
        "COLR glyph {glyph_id}: {} commands, clip_box={:?}",
        colr.commands.len(),
        colr.clip_box
    );

    // Determine output dimensions from clip box, with 1px padding on all
    // sides to prevent COLR paint layer overflow from being clipped.
    let raw_clip = colr
        .clip_box
        .unwrap_or_else(|| estimate_clip_box(fd, glyph_id, size_px));
    // Pad clip box by 10% of the glyph height on each side to ensure
    // COLR paint layers that overflow the declared bounds render fully.
    let pad = raw_clip.height() * 0.1;
    let clip = ClipBox {
        x_min: raw_clip.x_min - pad,
        y_min: raw_clip.y_min - pad,
        x_max: raw_clip.x_max + pad,
        y_max: raw_clip.y_max + pad,
    };

    let width = clip.width().ceil() as u32;
    let height = clip.height().ceil() as u32;
    if width == 0 || height == 0 {
        return None;
    }

    // RGBA buffer (premultiplied, initially transparent).
    let mut bitmap = vec![0u8; (width * height * 4) as usize];

    // Composite paint commands via tiny-skia.
    super::compose::composite_commands(
        &colr.commands,
        &mut bitmap,
        width,
        height,
        clip,
        fd,
        size_px,
    );

    // If the compositor produced a blank bitmap (all bytes zero), fall
    // through to swash. Swash can still render the glyph via COLR v0
    // BaseGlyph records (backwards-compatible) or other color sources
    // (CBDT/sbix).
    if bitmap.iter().all(|&b| b == 0) {
        log::debug!("COLR glyph {glyph_id}: blank bitmap, falling through to swash");
        return None;
    }

    // Bearing: offset from glyph origin to top-left of bitmap.
    let bearing_x = clip.x_min.floor() as i32;
    let bearing_y = clip.y_max.ceil() as i32;

    // Advance width from font metrics. COLR v1 is used for color emoji fonts
    // which don't have weight variation axes, so empty variations is correct.
    let fr = font_ref(fd);
    let advance = fr.glyph_metrics(&[]).scale(size_px).advance_width(glyph_id);

    Some(RasterizedGlyph {
        width,
        height,
        bearing_x,
        bearing_y,
        advance,
        format: GlyphFormat::Color,
        bitmap,
    })
}

/// Estimate a clip box from font metrics when no COLR v1 clip box is defined.
///
/// Uses the glyph advance width and font ascent/descent as a rough bounding
/// box. Most COLR v1 fonts define clip boxes, so this is a rare fallback.
/// Empty variations: color emoji fonts don't have weight variation axes.
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
