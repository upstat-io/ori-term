//! UI text shaping — free-form pixel positioning for non-grid text.
//!
//! Tab bar titles, search bar content, and overlays need text that isn't
//! tied to grid columns. This module provides [`shape_text_string`] to shape
//! arbitrary strings into [[`ShapedGlyph`]]s with `x_advance` positioning,
//! plus [`measure_text`], [`shape_text`], and [`truncate_with_ellipsis`] for layout.

use std::borrow::Cow;

use oriterm_ui::text::{ShapedGlyph, ShapedText, TextOverflow, TextStyle};

use crate::font::collection::FontCollection;
use crate::font::{FaceIdx, GlyphStyle, SyntheticFlags};

/// Shape a plain text string for UI rendering (tab titles, search bar, overlays).
///
/// Segments text into runs by font face, shapes each run through rustybuzz,
/// and emits [[`ShapedGlyph`]]s with pixel-based `x_advance` positioning.
/// Spaces produce advance-only glyphs (`glyph_id=0`) at cell width.
///
/// `glyph_style` selects the font weight (Regular, Bold, etc.).
/// Pass `buffer_slot` to persist the rustybuzz buffer across frames.
#[expect(
    clippy::string_slice,
    reason = "byte indices from char_indices() are always valid char boundaries"
)]
#[expect(
    clippy::too_many_arguments,
    reason = "glyph_style added for font weight selection; grouping into a struct would obscure the API"
)]
pub fn shape_text_string(
    text: &str,
    glyph_style: GlyphStyle,
    synthetic: SyntheticFlags,
    requested_weight: u16,
    faces: &[Option<rustybuzz::Face<'_>>],
    collection: &FontCollection,
    output: &mut Vec<ShapedGlyph>,
    buffer_slot: &mut Option<rustybuzz::UnicodeBuffer>,
) {
    output.clear();
    if text.is_empty() {
        return;
    }

    let mut buffer = buffer_slot.take().unwrap_or_default();

    let mut run_start: Option<usize> = None;
    let mut run_face = FaceIdx::REGULAR;

    for (byte_idx, ch) in text.char_indices() {
        // Resolve face for non-space characters. Spaces stay in the
        // current run so rustybuzz computes the font's actual space
        // advance (proportional, not monospace cell_w).
        let face_idx = if ch == ' ' {
            run_face
        } else if is_likely_emoji(ch) {
            collection.resolve_prefer_emoji(ch, glyph_style).face_idx
        } else {
            collection.resolve(ch, glyph_style).face_idx
        };

        if let Some(start) = run_start {
            if face_idx != run_face {
                // Face changed — flush current run.
                let run_syn = per_face_synthetic(synthetic, requested_weight, run_face, collection);
                buffer = shape_ui_run(
                    &text[start..byte_idx],
                    run_face,
                    run_syn,
                    faces,
                    collection,
                    output,
                    buffer,
                );
                run_start = Some(byte_idx);
                run_face = face_idx;
            }
        } else {
            run_start = Some(byte_idx);
            run_face = face_idx;
        }
    }

    // Flush last run.
    if let Some(start) = run_start {
        let run_syn = per_face_synthetic(synthetic, requested_weight, run_face, collection);
        buffer = shape_ui_run(
            &text[start..],
            run_face,
            run_syn,
            faces,
            collection,
            output,
            buffer,
        );
    }

    *buffer_slot = Some(buffer);
}

/// Shape text into a [`ShapedText`] block using the given style.
///
/// Higher-level API that handles text transform, font weight selection,
/// overflow (clip, ellipsis, wrap), letter spacing, and returns a complete
/// [`ShapedText`] with layout metrics.
///
/// Text transform is applied before overflow so case changes that alter
/// string length (e.g. `ß` → `SS`) are accounted for in truncation.
///
/// `max_width` limits the text width for overflow handling. Pass `f32::INFINITY`
/// for unconstrained shaping.
///
/// `phys_letter_spacing` is the inter-glyph spacing in physical pixels.
/// It is applied to glyph advances and accounted for during ellipsis
/// truncation so that spaced text does not overflow `max_width`.
pub fn shape_text(
    text: &str,
    style: &TextStyle,
    max_width: f32,
    phys_letter_spacing: f32,
    collection: &FontCollection,
) -> ShapedText {
    // Apply text transform before overflow — case changes can alter length.
    let transformed = style.text_transform.apply(text);

    let resolution = collection.resolve_ui_weight_info(style.weight.value());
    let glyph_style = if resolution.face_slot == 1 || resolution.needs_synthetic_bold {
        GlyphStyle::Bold
    } else {
        GlyphStyle::Regular
    };
    let requested_weight = style.weight.value();
    let synthetic = if resolution.needs_synthetic_bold {
        SyntheticFlags::BOLD
    } else {
        SyntheticFlags::NONE
    };

    let mut shaped = match style.overflow {
        TextOverflow::Ellipsis => {
            // Shape the full text first to get accurate width including letter
            // spacing. This avoids false truncation when ligatures or combining
            // marks reduce glyph count below character count — the char-based
            // approximation in truncate_with_ellipsis would overestimate width.
            let full = shape_to_shaped_text(
                &transformed,
                glyph_style,
                collection,
                requested_weight,
                synthetic,
            );
            let full_width = full.width
                + if phys_letter_spacing > 0.0 && !full.glyphs.is_empty() {
                    phys_letter_spacing * full.glyphs.len() as f32
                } else {
                    0.0
                };

            if full_width <= max_width {
                full
            } else {
                let truncated = truncate_with_ellipsis(
                    &transformed,
                    max_width,
                    phys_letter_spacing,
                    collection,
                );
                shape_to_shaped_text(
                    &truncated,
                    glyph_style,
                    collection,
                    requested_weight,
                    synthetic,
                )
            }
        }
        TextOverflow::Clip | TextOverflow::Wrap => shape_to_shaped_text(
            &transformed,
            glyph_style,
            collection,
            requested_weight,
            synthetic,
        ),
    };

    // Apply letter spacing to glyph advances so width is fully resolved.
    if phys_letter_spacing > 0.0 && !shaped.glyphs.is_empty() {
        for g in &mut shaped.glyphs {
            g.x_advance += phys_letter_spacing;
        }
        shaped.width += phys_letter_spacing * shaped.glyphs.len() as f32;
    }

    // Stamp font source so the glyph cache routes to the correct collection.
    shaped.font_source = style.font_source;

    shaped
}

/// Shape text into a [`ShapedText`] block with computed metrics.
///
/// Uses weight-aware shaping faces so the `wght` axis matches the requested
/// UI weight. This ensures layout metrics (advances, positioning) are computed
/// from the same weight used for atlas rasterization.
fn shape_to_shaped_text(
    text: &str,
    glyph_style: GlyphStyle,
    collection: &FontCollection,
    requested_weight: u16,
    synthetic: SyntheticFlags,
) -> ShapedText {
    let faces = collection.create_shaping_faces_for_weight(requested_weight, synthetic);
    let mut glyphs = Vec::new();
    let mut buffer_slot = None;
    shape_text_string(
        text,
        glyph_style,
        synthetic,
        requested_weight,
        &faces,
        collection,
        &mut glyphs,
        &mut buffer_slot,
    );

    let width: f32 = glyphs.iter().map(|g| g.x_advance).sum();
    let metrics = collection.cell_metrics();
    let size_q6 = super::super::collection::size_key(collection.size_px());

    ShapedText::new(
        glyphs,
        width,
        metrics.height,
        metrics.baseline,
        size_q6,
        requested_weight,
    )
}

/// Measure the total pixel width of a text string using unicode widths.
///
/// Uses `unicode_width * cell_width` for measurement, consistent with
/// [`truncate_with_ellipsis`]. Exact for monospace fonts.
#[cfg(test)]
pub fn measure_text(text: &str, collection: &FontCollection) -> f32 {
    let cell_w = collection.cell_metrics().width;
    text.chars()
        .map(|ch| unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0) as f32 * cell_w)
        .sum()
}

/// Truncate text with ellipsis if it exceeds `max_width` pixels.
///
/// Returns the original text unchanged if it fits. Otherwise, truncates at
/// a character boundary and appends `\u{2026}` (…). Uses cell-width-based
/// measurement which is exact for monospace fonts.
///
/// `phys_letter_spacing` is the inter-glyph spacing in physical pixels. When
/// non-zero, each character's effective width includes spacing so that the
/// truncated result stays within `max_width` after spacing is applied.
#[expect(
    clippy::string_slice,
    reason = "end_byte is accumulated from char_indices() offsets + len_utf8()"
)]
pub fn truncate_with_ellipsis<'a>(
    text: &'a str,
    max_width: f32,
    phys_letter_spacing: f32,
    collection: &FontCollection,
) -> Cow<'a, str> {
    let cell_w = collection.cell_metrics().width;

    // Sum unicode widths plus letter spacing for width approximation.
    // Count only visible characters (nonzero unicode width) for spacing to
    // match the shaping output where zero-width marks don't produce glyphs.
    let mut total_cells: usize = 0;
    let mut visible_count: usize = 0;
    for ch in text.chars() {
        let w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        total_cells += w;
        if w > 0 {
            visible_count += 1;
        }
    }
    let total_width = total_cells as f32 * cell_w + visible_count as f32 * phys_letter_spacing;
    if total_width <= max_width {
        return Cow::Borrowed(text);
    }

    // Ellipsis (U+2026) is width 1 in monospace, plus its own letter spacing.
    let ellipsis_width = cell_w + phys_letter_spacing;
    let budget = max_width - ellipsis_width;
    if budget <= 0.0 {
        return Cow::Owned(String::from("\u{2026}"));
    }

    let mut used = 0.0_f32;
    let mut end_byte = 0;
    for (byte_idx, ch) in text.char_indices() {
        let char_w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        let w = char_w as f32 * cell_w + if char_w > 0 { phys_letter_spacing } else { 0.0 };
        if used + w > budget {
            break;
        }
        used += w;
        end_byte = byte_idx + ch.len_utf8();
    }

    let mut result = String::with_capacity(end_byte + 3);
    result.push_str(&text[..end_byte]);
    result.push('\u{2026}');
    Cow::Owned(result)
}

/// Shape a single UI text run and append results.
///
/// Returns the cleared `UnicodeBuffer` for reuse by the next run. When no
/// face is available, emits advance-only glyphs based on unicode width.
#[expect(
    clippy::too_many_arguments,
    reason = "mirrors grid shape_run with separate text+face_idx instead of ShapingRun"
)]
fn shape_ui_run(
    text: &str,
    face_idx: FaceIdx,
    synthetic: SyntheticFlags,
    faces: &[Option<rustybuzz::Face<'_>>],
    collection: &FontCollection,
    output: &mut Vec<ShapedGlyph>,
    mut buffer: rustybuzz::UnicodeBuffer,
) -> rustybuzz::UnicodeBuffer {
    let syn_bits = synthetic.bits();
    let Some(face) = faces.get(face_idx.as_usize()).and_then(|f| f.as_ref()) else {
        // No rustybuzz face — try cmap + font metrics for each character.
        // This handles color emoji fonts that ttf-parser can't parse for
        // shaping but that swash can still rasterize via cmap lookup.
        let cell_w = collection.cell_metrics().width;
        for ch in text.chars() {
            if let Some((gid, advance)) = collection.cmap_glyph(ch, face_idx) {
                output.push(ShapedGlyph {
                    glyph_id: gid,
                    face_index: face_idx.0,
                    synthetic: syn_bits,
                    x_advance: advance,
                    x_offset: 0.0,
                    y_offset: 0.0,
                });
            } else {
                let w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
                if w == 0 {
                    continue;
                }
                output.push(ShapedGlyph {
                    glyph_id: 0,
                    face_index: face_idx.0,
                    synthetic: syn_bits,
                    x_advance: w as f32 * cell_w,
                    x_offset: 0.0,
                    y_offset: 0.0,
                });
            }
        }
        return buffer;
    };

    buffer.push_str(text);
    buffer.set_direction(rustybuzz::Direction::LeftToRight);

    let features = collection.features_for_face(face_idx);
    let glyph_buffer = rustybuzz::shape(face, features, buffer);
    let infos = glyph_buffer.glyph_infos();
    let positions = glyph_buffer.glyph_positions();

    let upem = face.units_per_em() as f32;
    let eff_size = collection.effective_size(face_idx);
    let scale = eff_size / upem;

    for (info, pos) in infos.iter().zip(positions.iter()) {
        output.push(ShapedGlyph {
            glyph_id: info.glyph_id as u16,
            face_index: face_idx.0,
            synthetic: syn_bits,
            x_advance: pos.x_advance as f32 * scale,
            x_offset: pos.x_offset as f32 * scale,
            y_offset: pos.y_offset as f32 * scale,
        });
    }

    glyph_buffer.clear()
}

/// Compute per-face synthetic flags for a UI text run.
///
/// When the primary font handles weight via its `wght` axis, the top-level
/// `synthetic` flags won't include `BOLD`. But a fallback face without a
/// `wght` axis needs synthetic bold to approximate the requested weight.
/// This function adds `BOLD` when the face can't express the weight natively.
pub(super) fn per_face_synthetic(
    base: SyntheticFlags,
    requested_weight: u16,
    face_idx: FaceIdx,
    collection: &FontCollection,
) -> SyntheticFlags {
    if requested_weight >= 700
        && !base.contains(SyntheticFlags::BOLD)
        && !collection.face_has_wght_axis(face_idx)
        && !face_idx.is_bold_primary()
    {
        base | SyntheticFlags::BOLD
    } else {
        base
    }
}

/// Whether a codepoint is likely emoji and should prefer emoji font resolution.
///
/// Delegates to [`oriterm_core::is_emoji_presentation`] for the base ranges,
/// then adds ZWJ and variation selectors used in emoji sequences.
fn is_likely_emoji(cp: char) -> bool {
    oriterm_core::is_emoji_presentation(cp) || matches!(cp, '\u{200D}' | '\u{FE0F}')
}
