//! Icon atlas cache: maps `(IconId, size_px)` to atlas entries.
//!
//! Icons are rasterized once per size and cached in the monochrome glyph
//! atlas. The cache is invalidated on DPI change (which changes `size_px`).

use std::collections::HashMap;

use wgpu::Queue;

use oriterm_ui::icons::IconId;

use super::rasterize_icon;
use crate::font::{FaceIdx, GlyphFormat, RasterKey, RasterizedGlyph, SyntheticFlags};
use crate::gpu::atlas::{AtlasEntry, GlyphAtlas};

/// Reserved face index for icon atlas entries.
///
/// Real font face indices are small (0–~10). Using `u16::MAX` guarantees
/// no collision with glyph raster keys.
const ICON_FACE_IDX: FaceIdx = FaceIdx(u16::MAX);

/// Cache key: icon identity + target pixel size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct CacheKey {
    id: IconId,
    size_px: u32,
}

/// Caches rasterized icon bitmaps in the monochrome glyph atlas.
///
/// Each icon is rasterized at a specific pixel size (determined by DPI)
/// and uploaded to the mono atlas as an `R8Unorm` alpha mask. Subsequent
/// lookups return the cached [`AtlasEntry`] without re-rasterization.
pub(crate) struct IconCache {
    entries: HashMap<CacheKey, AtlasEntry>,
}

impl IconCache {
    /// Create an empty icon cache.
    pub(crate) fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Look up an icon in the cache, rasterizing and uploading if not present.
    ///
    /// `size_px` is the physical pixel size. `scale` is the display scale
    /// factor, forwarded to [`rasterize_icon`] for stroke width conversion.
    ///
    /// Returns `None` if the atlas cannot accommodate the icon (extremely
    /// unlikely for small icon bitmaps).
    #[expect(
        clippy::too_many_arguments,
        reason = "icon cache lookup: id, size, scale, atlas, queue are all required context"
    )]
    pub(crate) fn get_or_insert(
        &mut self,
        id: IconId,
        size_px: u32,
        scale: f32,
        atlas: &mut GlyphAtlas,
        queue: &Queue,
    ) -> Option<AtlasEntry> {
        let key = CacheKey { id, size_px };

        if let Some(&entry) = self.entries.get(&key) {
            atlas.touch_page(entry.page);
            return Some(entry);
        }

        // Rasterize the icon to an alpha-only bitmap.
        let icon_path = id.path();
        let alpha_data = rasterize_icon(icon_path, size_px, scale);
        if alpha_data.is_empty() {
            return None;
        }

        // Wrap as a RasterizedGlyph for atlas insertion.
        let glyph = RasterizedGlyph {
            width: size_px,
            height: size_px,
            bearing_x: 0,
            bearing_y: size_px as i32,
            advance: 0.0,
            format: GlyphFormat::Alpha,
            bitmap: alpha_data,
        };

        // Synthetic RasterKey using reserved face index + icon variant as glyph_id.
        let raster_key = RasterKey {
            glyph_id: id as u16,
            face_idx: ICON_FACE_IDX,
            size_q6: (size_px as f32 * 64.0).round() as u32,
            synthetic: SyntheticFlags::NONE,
            hinted: false,
            subpx_x: 0,
            font_realm: crate::font::FontRealm::Ui,
        };

        let entry = atlas.insert(raster_key, &glyph, queue)?;
        self.entries.insert(key, entry);
        Some(entry)
    }

    /// Discard all cached entries.
    ///
    /// Call on DPI change (the atlas is also cleared, so our entries
    /// become stale). Icons will be re-rasterized at the new size on
    /// the next frame.
    pub(crate) fn clear(&mut self) {
        self.entries.clear();
    }

    /// Number of cached icon entries.
    #[allow(dead_code, reason = "used in tests and diagnostics")]
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }
}
