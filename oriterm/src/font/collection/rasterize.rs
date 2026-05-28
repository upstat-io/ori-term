//! Glyph rasterization for terminal grid and UI text paths.
//!
//! Extracted from `collection/mod.rs` to keep the main module under the
//! 500-line limit. Both `rasterize()` (terminal grid) and
//! `rasterize_with_weight()` (UI text) live here.
//!
//! Alpha correction: `GlyphFormat::Alpha` (R8) coverage values receive a
//! gamma-aware boost via [`apply_alpha_correction`] to compensate for the
//! visual weight loss that occurs when raw coverage masks are composited
//! in linear space with sRGB output. Without this, grayscale text appears
//! ~100 CSS weight units lighter than DirectWrite/browser rendering at
//! the same font weight.
//!
//! `GlyphFormat::SubpixelRgb` / `SubpixelBgr` bitmaps are NOT boosted —
//! each of the R / G / B bytes is an independent per-channel LCD coverage
//! value (zeno `Format::Subpixel` rasterizes each channel at a different
//! subpixel X offset). Applying a concave gamma curve per-byte magnifies
//! per-channel asymmetry into saturated color fringes and over-thickens
//! strokes. Gamma compensation for subpixel, if needed, belongs at the
//! shader blend step (already linear-space in oriterm; see
//! `oriterm/src/gpu/instance_writer/mod.rs` `rgb_to_floats`), not at the
//! rasterizer. `GlyphFormat::Color` is never boosted (premultiplied RGBA).

use super::colr_v1::rasterize::try_rasterize_colr_v1;
use super::face::{RasterSpec, rasterize_from_face};
use super::metadata::{effective_size_for, face_variations, face_variations_for_ui_weight};
use super::{FontCollection, GlyphFormat, RasterizedGlyph};
use crate::font::{FaceIdx, RasterKey};

/// Default text gamma for glyph alpha correction.
///
/// Matches DirectWrite's default gamma (1.8). Corrects the visual weight
/// loss that occurs when swash's raw coverage masks are composited in
/// linear space with sRGB output.
///
/// 1.0 = no correction. Higher = heavier text.
pub(super) const TEXT_GAMMA: f32 = 1.8;

/// Build a 256-entry lookup table for `pow(alpha/255, 1/gamma) * 255`.
///
/// Maps each byte value `[0, 255]` to its gamma-corrected equivalent.
/// 0 maps to 0, 255 maps to 255. Intermediate values are boosted,
/// with the strongest effect on low-coverage pixels (thin strokes).
///
/// Examples at gamma 1.8:
/// - 26 (10%) → 44 (17%) — thin anti-aliased edge boosted 70%
/// - 77 (30%) → 105 (41%) — medium coverage boosted 37%
/// - 128 (50%) → 153 (60%) — half coverage boosted 20%
/// - 230 (90%) → 240 (94%) — near-opaque barely affected
pub(super) fn build_gamma_lut(gamma: f32) -> [u8; 256] {
    let mut lut = [0u8; 256];
    if (gamma - 1.0).abs() < f32::EPSILON {
        for (i, entry) in lut.iter_mut().enumerate() {
            *entry = i as u8;
        }
        return lut;
    }
    let inv_gamma = 1.0 / gamma;
    for i in 0..=255u16 {
        let a = i as f32 / 255.0;
        lut[i as usize] = (a.powf(inv_gamma) * 255.0 + 0.5) as u8;
    }
    lut
}

/// Apply gamma-aware alpha correction to 8-bit monochrome glyph coverage.
///
/// Transforms each byte through the pre-built LUT: `byte = lut[byte]`.
/// Applied to `GlyphFormat::Alpha` (`R8Unorm`) bitmaps only.
///
/// Must NOT be applied to `SubpixelRgb` / `SubpixelBgr` — each R / G / B
/// byte is an independent per-channel LCD coverage value, and the concave
/// gamma curve magnifies per-channel asymmetry into saturated color
/// fringes (). Must NOT be applied to `Color` — premultiplied
/// RGBA would corrupt.
///
/// The `debug_assert!` below is the runtime guard that catches any caller
/// slipping past the format-check at the call sites. Together with the
/// per-site guards in `rasterize` and `rasterize_with_weight` it makes the
/// Alpha-only invariant explicit at every layer — any future path that
/// forgets the guard fires this panic in debug builds.
fn apply_alpha_correction(glyph: &mut RasterizedGlyph, lut: &[u8; 256]) {
    debug_assert_eq!(
        glyph.format,
        GlyphFormat::Alpha,
        "apply_alpha_correction called on non-Alpha glyph ({:?}); per-byte gamma \
         boost corrupts subpixel per-channel coverage and premultiplied color data",
        glyph.format,
    );
    for byte in &mut glyph.bitmap {
        *byte = lut[*byte as usize];
    }
}

impl FontCollection {
    /// Apply the per-format gamma correction and insert the glyph into the cache.
    ///
    /// Canonical home for the `GlyphFormat::Alpha`-only gamma rule ().
    /// Called by both `rasterize` and `rasterize_with_weight` after the raw
    /// rasterizer produces the bitmap — keeping the correction guard in a single
    /// place so any future protocol change lands once, not twice.
    ///
    /// Only `GlyphFormat::Alpha` (R8 coverage) receives the boost; `SubpixelRgb`,
    /// `SubpixelBgr`, and `Color` pass through unchanged — see the module-level
    /// doc for the rationale.
    fn insert_corrected(
        &mut self,
        key: RasterKey,
        mut glyph: RasterizedGlyph,
    ) -> Option<&RasterizedGlyph> {
        if glyph.format == GlyphFormat::Alpha {
            apply_alpha_correction(&mut glyph, &self.gamma_lut);
        }
        self.cache_insert(key, glyph);
        self.glyph_cache.get(&key)
    }

    /// Rasterize a glyph and cache the result.
    ///
    /// Returns `None` for empty glyphs (e.g. space) or unsupported formats.
    /// Subsequent calls with the same key return the cached bitmap.
    pub fn rasterize(&mut self, key: RasterKey) -> Option<&RasterizedGlyph> {
        // Built-in glyphs are rasterized by `builtin_glyphs::ensure_cached`,
        // not through font faces. Guard against the sentinel index to prevent
        // an out-of-bounds panic on `self.primary[65535]`.
        if key.face_idx == FaceIdx::BUILTIN {
            return None;
        }

        // NLL limitation: `if let Some(g) = get() { return Some(g); }` ties the
        // immutable borrow to the return lifetime, blocking `insert` on the miss
        // path (E0502). Two lookups are the idiomatic workaround until Polonius.
        if self.glyph_cache.contains_key(&key) {
            return self.glyph_cache.get(&key);
        }

        // Inline face lookup for disjoint borrows with scale_context.
        let fd = if let Some(fb_i) = key.face_idx.fallback_index() {
            self.fallbacks.get(fb_i)?
        } else {
            self.primary[key.face_idx.as_usize()].as_ref()?
        };
        let size = effective_size_for(key.face_idx, self.size_px, &self.fallback_meta);
        let face_vars = face_variations(
            key.face_idx,
            key.synthetic,
            self.weight,
            self.bold_weight,
            &fd.axes,
        );
        let effective_synthetic = key.synthetic - face_vars.suppress_synthetic;
        let subpx_x_offset = super::super::subpx_offset(key.subpx_x);

        // COLRv1 compositor first — uses the correct COLR clip box for canvas
        // sizing, preventing bottom/right edge clipping. Falls
        // through to swash for non-COLR glyphs or if compositing fails.
        let gid_u16 = key.glyph_id as u16;
        let glyph = try_rasterize_colr_v1(fd, gid_u16, size).or_else(|| {
            rasterize_from_face(
                fd,
                RasterSpec {
                    glyph_id: gid_u16,
                    size_px: size,
                    variations: &face_vars.settings,
                    synthetic: effective_synthetic,
                    cell_height: self.metrics.height,
                    format: self.format,
                    hinted: self.hinting.hint_flag(),
                    subpx_x_offset,
                },
                &mut self.scale_context,
            )
        })?;

        self.insert_corrected(key, glyph)
    }

    /// Rasterize a glyph using a specific requested weight.
    ///
    /// UI-text counterpart to [`rasterize`] — uses `requested_weight` instead
    /// of `self.weight` when computing variation axes. Terminal grid code
    /// continues using [`rasterize`].
    pub fn rasterize_with_weight(
        &mut self,
        key: RasterKey,
        requested_weight: u16,
    ) -> Option<&RasterizedGlyph> {
        if key.face_idx == FaceIdx::BUILTIN {
            return None;
        }

        if self.glyph_cache.contains_key(&key) {
            return self.glyph_cache.get(&key);
        }

        // For medium-weight requests on the Regular slot, prefer the Medium
        // face so rasterization matches the shaping substitution.
        let use_medium = key.face_idx == FaceIdx::REGULAR
            && (500..700).contains(&requested_weight)
            && self.medium.is_some();
        let fd = if let Some(fb_i) = key.face_idx.fallback_index() {
            self.fallbacks.get(fb_i)?
        } else if use_medium {
            self.medium.as_ref()?
        } else {
            self.primary[key.face_idx.as_usize()].as_ref()?
        };
        let size = effective_size_for(key.face_idx, self.size_px, &self.fallback_meta);
        let face_vars = face_variations_for_ui_weight(key.synthetic, requested_weight, &fd.axes);
        let effective_synthetic = key.synthetic - face_vars.suppress_synthetic;
        let subpx_x_offset = super::super::subpx_offset(key.subpx_x);

        let gid_u16 = key.glyph_id as u16;
        let glyph = try_rasterize_colr_v1(fd, gid_u16, size).or_else(|| {
            rasterize_from_face(
                fd,
                RasterSpec {
                    glyph_id: gid_u16,
                    size_px: size,
                    variations: &face_vars.settings,
                    synthetic: effective_synthetic,
                    cell_height: self.metrics.height,
                    format: self.format,
                    hinted: self.hinting.hint_flag(),
                    subpx_x_offset,
                },
                &mut self.scale_context,
            )
        })?;

        self.insert_corrected(key, glyph)
    }
}
