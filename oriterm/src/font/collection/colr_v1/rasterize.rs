//! COLR v1 detection and CPU rasterization entry point.
//!
//! Detects COLR v1 glyphs via skrifa, collects paint commands, and provides
//! the entry point for GPU-composited color emoji rendering.

use skrifa::instance::Size;
use skrifa::{FontRef, MetadataProvider};

use super::{ClipBox, ColrV1Glyph, PaintCollector, load_palette};

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
/// commands themselves operate in font units — the caller applies a
/// `size_px / upem` scaling transform before GPU compositing.
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
