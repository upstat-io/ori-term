//! Glyph rasterization for terminal grid and UI text paths.
//!
//! Extracted from `collection/mod.rs` to keep the main module under the
//! 500-line limit. Both `rasterize()` (terminal grid) and
//! `rasterize_with_weight()` (UI text) live here.

use super::colr_v1::rasterize::try_rasterize_colr_v1;
use super::face::rasterize_from_face;
use super::metadata::{effective_size_for, face_variations, face_variations_for_ui_weight};
use super::{FontCollection, RasterizedGlyph};
use crate::font::{FaceIdx, RasterKey};

impl FontCollection {
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
        let face_vars = face_variations(key.face_idx, key.synthetic, self.weight, &fd.axes);
        let effective_synthetic = key.synthetic - face_vars.suppress_synthetic;
        let subpx_x_offset = super::super::subpx_offset(key.subpx_x);

        // COLRv1 compositor first — uses the correct COLR clip box for canvas
        // sizing, preventing bottom/right edge clipping (BUG-04-001). Falls
        // through to swash for non-COLR glyphs or if compositing fails.
        let glyph = try_rasterize_colr_v1(fd, key.glyph_id, size).or_else(|| {
            rasterize_from_face(
                fd,
                key.glyph_id,
                size,
                &face_vars.settings,
                effective_synthetic,
                self.metrics.height,
                self.format,
                self.hinting.hint_flag(),
                subpx_x_offset,
                &mut self.scale_context,
            )
        })?;

        self.glyph_cache.insert(key, glyph);
        self.glyph_cache.get(&key)
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

        let glyph = try_rasterize_colr_v1(fd, key.glyph_id, size).or_else(|| {
            rasterize_from_face(
                fd,
                key.glyph_id,
                size,
                &face_vars.settings,
                effective_synthetic,
                self.metrics.height,
                self.format,
                self.hinting.hint_flag(),
                subpx_x_offset,
                &mut self.scale_context,
            )
        })?;

        self.glyph_cache.insert(key, glyph);
        self.glyph_cache.get(&key)
    }
}
