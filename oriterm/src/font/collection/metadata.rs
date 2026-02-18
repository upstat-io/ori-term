//! Fallback metadata, cap-height normalization, and OpenType feature helpers.
//!
//! Extracted from `collection/mod.rs` to keep the main module under the
//! 500-line limit. All items are internal to the collection module.

use super::super::FaceIdx;

/// Minimum font size in pixels (prevents degenerate scaling).
pub(super) const MIN_FONT_SIZE: f32 = 2.0;

/// Maximum font size in pixels (prevents absurd scaling).
pub(super) const MAX_FONT_SIZE: f32 = 200.0;

/// Per-fallback metadata for cap-height normalization and feature overrides.
///
/// Each entry in `fallback_meta` corresponds 1:1 to the matching entry in
/// `fallbacks`. System-discovered fallbacks get auto-computed `scale_factor`
/// with default features; user-configured fallbacks can override features
/// and add a `size_offset`.
pub(super) struct FallbackMeta {
    /// Cap-height normalization: `primary_cap_height / fallback_cap_height`.
    ///
    /// Ensures glyphs from different fonts appear at visually consistent sizes.
    /// A value of 1.0 means the fallback already matches the primary.
    pub scale_factor: f32,
    /// User-configured size adjustment in points (0.0 if unset).
    pub size_offset: f32,
    /// Per-fallback OpenType feature overrides.
    ///
    /// When `Some`, these features replace collection-wide defaults for this
    /// fallback. When `None`, collection defaults apply.
    pub features: Option<Vec<rustybuzz::Feature>>,
}

/// Default OpenType features: standard ligatures + contextual alternates.
///
/// These are the features most users expect from a terminal font.
pub(super) fn default_features() -> Vec<rustybuzz::Feature> {
    parse_features(&["liga", "calt"])
}

/// Parse feature tag strings into rustybuzz features.
///
/// Each string follows rustybuzz's `Feature::from_str` format:
/// - `"liga"` — enable standard ligatures
/// - `"-liga"` — disable standard ligatures
/// - `"+dlig"` — enable discretionary ligatures
/// - `"kern=0"` — disable kerning
///
/// Invalid tags are logged and skipped.
pub(super) fn parse_features(tags: &[&str]) -> Vec<rustybuzz::Feature> {
    tags.iter()
        .filter_map(|tag| match tag.parse::<rustybuzz::Feature>() {
            Ok(f) => Some(f),
            Err(e) => {
                log::warn!("font: invalid OpenType feature '{tag}': {e}");
                None
            }
        })
        .collect()
}

/// Compute effective font size for a face index with cap-height normalization.
///
/// Primary faces return `base_size` unchanged. Fallback faces are scaled by
/// their cap-height ratio: `base_size * scale_factor + size_offset`, clamped
/// to `[MIN_FONT_SIZE, MAX_FONT_SIZE]`.
pub(super) fn effective_size_for(
    face_idx: FaceIdx,
    base_size: f32,
    fallback_meta: &[FallbackMeta],
) -> f32 {
    if let Some(fb_i) = face_idx.fallback_index() {
        if let Some(meta) = fallback_meta.get(fb_i) {
            return (base_size * meta.scale_factor + meta.size_offset)
                .clamp(MIN_FONT_SIZE, MAX_FONT_SIZE);
        }
    }
    base_size
}

/// Compute the `wght` variation value for a face index.
///
/// Primary faces use the configured weight (Regular/Italic) or bold-derived
/// weight (Bold/BoldItalic). Fallback faces return `None`.
pub(super) fn weight_variation(face_idx: FaceIdx, weight: u16) -> Option<f32> {
    if face_idx.is_fallback() {
        return None;
    }
    let i = face_idx.as_usize();
    let w = if i == 1 || i == 3 {
        (weight + 300).min(900)
    } else {
        weight
    };
    Some(w as f32)
}
