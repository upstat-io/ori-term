//! Glyph atlas: guillotine-packed texture array for GPU glyph rendering.
//!
//! [`GlyphAtlas`] manages a grow-on-demand `Texture2DArray` (2048×2048,
//! starting with 1 layer and growing up to [`MAX_PAGES`]) using guillotine
//! bin packing for mixed glyph sizes. Pages are evicted via LRU when all
//! are full. Glyphs are inserted once and looked up by [`RasterKey`] on
//! subsequent frames.
//!
//! Three atlas instances are used at runtime:
//! - **Monochrome** (`R8Unorm`): standard glyph alpha masks.
//! - **Subpixel** (`Rgba8Unorm`): LCD subpixel coverage masks (RGB/BGR).
//! - **Color** (`Rgba8Unorm`): color emoji and bitmap glyphs.
//!
//! Atlases that are not immediately needed (e.g., color atlas before any
//! emoji, or the inactive mono/subpixel atlas) are created in **lazy mode**
//! via [`GlyphAtlas::new_lazy`]: a 1×1 placeholder texture that consumes
//! negligible GPU memory. On first [`insert`](GlyphAtlas::insert), the
//! placeholder is replaced with the full 2048² texture (materialization).
//!
//! When a page fills and a new layer is needed, the atlas grows by creating
//! a new texture with one additional layer, copying existing layers via
//! `CommandEncoder::copy_texture_to_texture()`, and incrementing a
//! [`generation`](GlyphAtlas::generation) counter. Callers check the
//! generation to detect stale bind groups.

mod rect_packer;
mod texture;

use std::collections::HashMap;

use wgpu::{
    CommandEncoderDescriptor, Device, Extent3d, Queue, Texture, TextureFormat, TextureView,
};

use self::rect_packer::RectPacker;
use self::texture::{create_texture_array, upload_glyph};
use crate::font::{GlyphFormat, RasterKey, RasterizedGlyph};

/// Atlas page dimension (width = height).
const PAGE_SIZE: u32 = 2048;

/// Maximum number of texture array layers.
const MAX_PAGES: u32 = 4;

/// Padding between glyphs to prevent texture filtering artifacts.
const GLYPH_PADDING: u32 = 1;

/// Per-page packing state and LRU metadata.
struct AtlasPage {
    packer: RectPacker,
    last_used_frame: u64,
    glyph_count: u32,
}

/// Which atlas an entry resides in.
///
/// Determines pipeline routing during the prepare phase: each kind maps
/// to a different fragment shader and blend mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtlasKind {
    /// Monochrome glyph: `R8Unorm` atlas, tinted by `fg_color`.
    Mono,
    /// LCD subpixel glyph: `Rgba8Unorm` atlas, per-channel `mix(bg, fg, mask)`.
    Subpixel,
    /// Color bitmap: `Rgba8Unorm` atlas, rendered as-is (no tinting).
    Color,
}

/// Location and metrics of a cached glyph in the atlas.
#[derive(Debug, Clone, Copy)]
pub struct AtlasEntry {
    /// Page index (texture array layer).
    pub page: u32,
    /// Normalized U coordinate of left edge (0.0–1.0).
    pub uv_x: f32,
    /// Normalized V coordinate of top edge (0.0–1.0).
    pub uv_y: f32,
    /// Normalized width (0.0–1.0).
    pub uv_w: f32,
    /// Normalized height (0.0–1.0).
    pub uv_h: f32,
    /// Bitmap width in pixels.
    pub width: u32,
    /// Bitmap height in pixels.
    pub height: u32,
    /// Horizontal bearing (pixels from glyph origin to left edge).
    pub bearing_x: i32,
    /// Vertical bearing (pixels from baseline to top edge; positive = above).
    pub bearing_y: i32,
    /// Which atlas this entry resides in (determines pipeline routing).
    pub kind: AtlasKind,
}

impl AtlasEntry {
    /// Whether this entry lives in the color (RGBA) atlas.
    ///
    /// Convenience for code that only needs the mono/non-mono distinction.
    #[allow(dead_code, reason = "convenience accessor for future use")]
    pub fn is_color(&self) -> bool {
        matches!(self.kind, AtlasKind::Color)
    }
}

/// Texture atlas for glyph bitmaps using guillotine packing on a `Texture2DArray`.
///
/// Manages a grow-on-demand texture array starting with 1 layer and growing
/// up to [`MAX_PAGES`] layers. The texture format is determined at
/// construction: `R8Unorm` for monochrome glyphs, `Rgba8Unorm` for color
/// emoji. Glyphs are packed using guillotine best-short-side-fit, uploaded
/// via `queue.write_texture`, and cached by [`RasterKey`] for O(1) lookup.
/// When all pages are full, the least-recently-used page is evicted.
///
/// When the atlas grows (new layer allocated), [`generation`](Self::generation)
/// increments so callers can detect stale bind groups.
pub struct GlyphAtlas {
    /// Current `Texture2DArray` (grows on demand).
    texture: Texture,
    /// `D2Array` view over all current layers.
    view: TextureView,
    /// Per-page packing state + LRU metadata.
    pages: Vec<AtlasPage>,
    /// Glyph cache: `RasterKey` → atlas entry.
    cache: HashMap<RasterKey, AtlasEntry>,
    page_size: u32,
    max_pages: u32,
    /// Number of layers in the current GPU texture.
    texture_layers: u32,
    /// Monotonically increasing frame counter for LRU tracking.
    frame_counter: u64,
    /// Incremented when the texture is replaced (grow or recreate).
    generation: u64,
    /// Pixel format of this atlas texture.
    format: GlyphFormat,
    /// wgpu texture format (cached from construction).
    tex_format: TextureFormat,
    /// Whether this atlas is in lazy mode (1×1 placeholder, not yet materialized).
    ///
    /// Set by [`new_lazy`](Self::new_lazy), cleared by [`materialize`](Self::materialize)
    /// on first [`insert`](Self::insert). Saves ~4–16 MB GPU memory per atlas
    /// that is never used (e.g., color atlas when no emoji are rendered).
    lazy: bool,
}

impl GlyphAtlas {
    /// Create a new atlas with a 1-layer texture array and one active page.
    ///
    /// The texture starts with a single layer and grows on demand up to
    /// [`MAX_PAGES`] layers. This saves ~108 MB GPU memory per window for
    /// typical ASCII terminal usage.
    ///
    /// `format` determines the texture format:
    /// - [`GlyphFormat::Alpha`] → `R8Unorm` (1 byte/pixel).
    /// - [`GlyphFormat::SubpixelRgb`] / [`GlyphFormat::SubpixelBgr`] → `Rgba8Unorm` (4 bytes/pixel).
    /// - [`GlyphFormat::Color`] → `Rgba8Unorm` (4 bytes/pixel).
    pub fn new(device: &Device, format: GlyphFormat) -> Self {
        let tex_format = match format {
            GlyphFormat::Alpha => TextureFormat::R8Unorm,
            GlyphFormat::Color => TextureFormat::Rgba8UnormSrgb,
            _ => TextureFormat::Rgba8Unorm, // subpixel masks are linear
        };
        // Start with 1 layer; grow on demand when pages overflow.
        let (texture, view) = create_texture_array(device, PAGE_SIZE, 1, tex_format);

        Self {
            texture,
            view,
            pages: vec![AtlasPage {
                packer: RectPacker::new(PAGE_SIZE, PAGE_SIZE),
                last_used_frame: 0,
                glyph_count: 0,
            }],
            cache: HashMap::new(),
            page_size: PAGE_SIZE,
            max_pages: MAX_PAGES,
            texture_layers: 1,
            frame_counter: 0,
            generation: 0,
            format,
            tex_format,
            lazy: false,
        }
    }

    /// Create a lazy atlas with a 1×1 placeholder texture.
    ///
    /// The full 2048² texture is allocated on the first [`insert`](Self::insert)
    /// call (materialization). Until then, the atlas consumes negligible GPU
    /// memory — saving ~4 MB (`R8Unorm`) or ~16 MB (`Rgba8Unorm`) per atlas.
    ///
    /// Use for atlases that may never be needed (e.g., color atlas when no
    /// emoji are rendered, or the inactive mono/subpixel atlas).
    pub fn new_lazy(device: &Device, format: GlyphFormat) -> Self {
        let tex_format = match format {
            GlyphFormat::Alpha => TextureFormat::R8Unorm,
            GlyphFormat::Color => TextureFormat::Rgba8UnormSrgb,
            _ => TextureFormat::Rgba8Unorm,
        };
        // 1×1 placeholder — satisfies bind group layout without allocating a full page.
        let (texture, view) = create_texture_array(device, 1, 1, tex_format);

        Self {
            texture,
            view,
            pages: Vec::new(),
            cache: HashMap::new(),
            page_size: PAGE_SIZE,
            max_pages: MAX_PAGES,
            texture_layers: 0,
            frame_counter: 0,
            generation: 0,
            format,
            tex_format,
            lazy: true,
        }
    }

    /// Increment the frame counter for LRU tracking.
    ///
    /// Call at the start of each frame before any glyph lookups or inserts.
    pub fn begin_frame(&mut self) {
        self.frame_counter += 1;
    }

    /// Look up a previously inserted glyph.
    ///
    /// For LRU correctness, callers with `&mut` access should also call
    /// [`touch_page`](Self::touch_page) with the entry's page index.
    pub fn lookup(&self, key: RasterKey) -> Option<&AtlasEntry> {
        self.cache.get(&key)
    }

    /// Look up a glyph and touch its page for LRU tracking in one call.
    ///
    /// Combines [`lookup`](Self::lookup) and [`touch_page`](Self::touch_page)
    /// atomically so callers can't forget to update LRU on cache hits.
    pub fn lookup_touch(&mut self, key: RasterKey) -> Option<AtlasEntry> {
        let entry = self.cache.get(&key).copied()?;
        if let Some(p) = self.pages.get_mut(entry.page as usize) {
            p.last_used_frame = self.frame_counter;
        }
        Some(entry)
    }

    /// Mark a page as used this frame for LRU tracking.
    ///
    /// Call after [`lookup`](Self::lookup) when you have mutable access to
    /// ensure recently-used pages are not evicted.
    pub fn touch_page(&mut self, page: u32) {
        if let Some(p) = self.pages.get_mut(page as usize) {
            p.last_used_frame = self.frame_counter;
        }
    }

    /// Insert a rasterized glyph into the atlas.
    ///
    /// Finds space via guillotine packing, uploads the bitmap to the GPU, and
    /// caches the entry. Returns `None` for zero-size glyphs (e.g. space)
    /// or glyphs too large for an atlas page.
    ///
    /// `device` is needed for grow-on-demand: when all existing pages are
    /// full and fewer than [`MAX_PAGES`] exist, the texture array is grown
    /// by one layer (creating a new texture and copying existing layers).
    pub fn insert(
        &mut self,
        key: RasterKey,
        glyph: &RasterizedGlyph,
        device: &Device,
        queue: &Queue,
    ) -> Option<AtlasEntry> {
        if let Some(&entry) = self.cache.get(&key) {
            return Some(entry);
        }

        if glyph.width == 0 || glyph.height == 0 {
            return None;
        }

        // Materialize lazy atlas on first real insert.
        if self.lazy {
            self.materialize(device);
        }

        let max_dim = self.page_size.saturating_sub(GLYPH_PADDING);
        if glyph.width > max_dim || glyph.height > max_dim {
            log::warn!(
                "glyph too large for atlas: {}×{} exceeds page size {}",
                glyph.width,
                glyph.height,
                self.page_size,
            );
            return None;
        }

        let (page_idx, x, y) = self.find_space(glyph.width, glyph.height);

        // Grow the GPU texture if find_space added a page beyond the
        // current texture layer count.
        if self.pages.len() as u32 > self.texture_layers {
            self.grow_texture(device, queue);
        }

        upload_glyph(queue, &self.texture, page_idx, x, y, glyph);

        let page = &mut self.pages[page_idx as usize];
        page.last_used_frame = self.frame_counter;
        page.glyph_count += 1;

        // Log when page utilization exceeds 80%.
        let total_pixels = u64::from(self.page_size) * u64::from(self.page_size);
        let free_pixels = page.packer.free_area();
        let used_fraction = 1.0 - free_pixels as f64 / total_pixels as f64;
        if used_fraction > 0.8 {
            log::debug!(
                "atlas page {page_idx} at {:.0}% utilization ({} glyphs)",
                used_fraction * 100.0,
                page.glyph_count,
            );
        }

        let kind = match self.format {
            GlyphFormat::Color => AtlasKind::Color,
            GlyphFormat::SubpixelRgb | GlyphFormat::SubpixelBgr => AtlasKind::Subpixel,
            GlyphFormat::Alpha => AtlasKind::Mono,
        };
        let ps = self.page_size as f32;
        let entry = AtlasEntry {
            page: page_idx,
            uv_x: x as f32 / ps,
            uv_y: y as f32 / ps,
            uv_w: glyph.width as f32 / ps,
            uv_h: glyph.height as f32 / ps,
            width: glyph.width,
            height: glyph.height,
            bearing_x: glyph.bearing_x,
            bearing_y: glyph.bearing_y,
            kind,
        };

        self.cache.insert(key, entry);
        Some(entry)
    }

    /// Look up or insert a glyph in one call.
    ///
    /// If the key is already cached, returns the entry (touching LRU).
    /// Otherwise, calls `rasterize` to produce the glyph and inserts it.
    /// Callers that maintain a separate empty-key set should check it
    /// before calling this method.
    ///
    /// This unifies the lookup-rasterize-insert pattern used by
    /// [`ensure_glyphs_cached`](crate::gpu::window_renderer::helpers::ensure_glyphs_cached).
    #[allow(dead_code, reason = "convenience API for later integration")]
    pub fn get_or_insert(
        &mut self,
        key: RasterKey,
        rasterize: impl FnOnce() -> Option<RasterizedGlyph>,
        device: &Device,
        queue: &Queue,
    ) -> Option<AtlasEntry> {
        // Cache hit — touch page and return.
        if let Some(entry) = self.cache.get(&key).copied() {
            self.touch_page(entry.page);
            return Some(entry);
        }

        // Cache miss — rasterize and insert.
        let glyph = rasterize()?;
        self.insert(key, &glyph, device, queue)
    }

    /// `Texture2DArray` view for atlas bind group creation.
    pub fn view(&self) -> &TextureView {
        &self.view
    }

    /// Number of cached glyph entries.
    #[allow(dead_code, reason = "used in tests and diagnostics")]
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Whether the cache is empty.
    #[allow(dead_code, reason = "used in tests and diagnostics")]
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Number of active atlas pages.
    #[allow(dead_code, reason = "used in tests and diagnostics")]
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Current frame counter value.
    #[allow(dead_code, reason = "used in tests and diagnostics")]
    pub fn frame_counter(&self) -> u64 {
        self.frame_counter
    }

    /// Texture generation counter.
    ///
    /// Incremented when the texture is replaced (grow-on-demand). Callers
    /// compare against their last-seen generation to detect stale bind
    /// groups that reference a now-destroyed `TextureView`.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Clear all cached glyphs and reset packing state.
    ///
    /// Keeps the texture array but resets to one active page. Called on font
    /// size change when all cached glyphs become invalid.
    pub fn clear(&mut self) {
        self.cache.clear();
        for page in &mut self.pages {
            page.packer.reset();
            page.glyph_count = 0;
        }
        self.pages.truncate(1);
    }

    /// Whether this atlas is in lazy mode (1×1 placeholder).
    #[allow(dead_code, reason = "used in tests and diagnostics")]
    pub fn is_lazy(&self) -> bool {
        self.lazy
    }

    // ── Private helpers ──

    /// Replace the 1×1 placeholder with the full-size texture and first page.
    ///
    /// Called once on the first [`insert`](Self::insert). Bumps the generation
    /// counter so callers rebuild stale bind groups.
    fn materialize(&mut self, device: &Device) {
        debug_assert!(self.lazy, "materialize called on non-lazy atlas");
        log::debug!(
            "atlas materializing: {:?} → {PAGE_SIZE}² texture",
            self.tex_format
        );

        let (texture, view) = create_texture_array(device, PAGE_SIZE, 1, self.tex_format);
        self.texture = texture;
        self.view = view;
        self.texture_layers = 1;
        self.pages.push(AtlasPage {
            packer: RectPacker::new(PAGE_SIZE, PAGE_SIZE),
            last_used_frame: self.frame_counter,
            glyph_count: 0,
        });
        self.generation += 1;
        self.lazy = false;
    }

    /// Grow the GPU texture to match the current page count.
    ///
    /// Creates a new `Texture2DArray` with `self.pages.len()` layers,
    /// copies all existing layers from the old texture, and replaces
    /// `self.texture` and `self.view`. Increments `self.generation` so
    /// callers can detect stale bind groups.
    fn grow_texture(&mut self, device: &Device, queue: &Queue) {
        let new_layers = self.pages.len() as u32;
        let old_layers = self.texture_layers;
        debug_assert!(new_layers > old_layers);

        log::debug!(
            "atlas growing: {old_layers} → {new_layers} layers ({:?})",
            self.tex_format,
        );

        let (new_texture, new_view) =
            create_texture_array(device, self.page_size, new_layers, self.tex_format);

        // Copy existing layers from old texture to new texture.
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("atlas_grow_copy"),
        });
        for layer in 0..old_layers {
            encoder.copy_texture_to_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: layer,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyTextureInfo {
                    texture: &new_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: layer,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                Extent3d {
                    width: self.page_size,
                    height: self.page_size,
                    depth_or_array_layers: 1,
                },
            );
        }
        queue.submit(std::iter::once(encoder.finish()));

        self.texture = new_texture;
        self.view = new_view;
        self.texture_layers = new_layers;
        self.generation += 1;
    }

    /// Find space for a glyph, returning `(page_idx, x, y)`.
    ///
    /// Tries each existing page's guillotine packer. If all are full and
    /// fewer than `max_pages` exist, adds a new page. If at the page limit,
    /// evicts the least-recently-used page.
    fn find_space(&mut self, w: u32, h: u32) -> (u32, u32, u32) {
        let padded_w = w + GLYPH_PADDING;
        let padded_h = h + GLYPH_PADDING;

        // Try existing pages.
        for (i, page) in self.pages.iter_mut().enumerate() {
            if let Some((x, y)) = page.packer.pack(padded_w, padded_h) {
                return (i as u32, x, y);
            }
        }

        // All pages full — add a new one if under the limit.
        if (self.pages.len() as u32) < self.max_pages {
            let page_idx = self.pages.len();
            self.pages.push(AtlasPage {
                packer: RectPacker::new(self.page_size, self.page_size),
                last_used_frame: self.frame_counter,
                glyph_count: 0,
            });

            let (x, y) = self.pages[page_idx]
                .packer
                .pack(padded_w, padded_h)
                .expect("fresh page must fit glyph within page_size bounds");

            return (page_idx as u32, x, y);
        }

        // At max pages — LRU eviction.
        let evicted = self.find_lru_page();
        self.evict_page(evicted);

        let (x, y) = self.pages[evicted]
            .packer
            .pack(padded_w, padded_h)
            .expect("freshly evicted page must fit glyph");

        (evicted as u32, x, y)
    }

    /// Find the page index with the smallest `last_used_frame`.
    fn find_lru_page(&self) -> usize {
        self.pages
            .iter()
            .enumerate()
            .min_by_key(|(_, p)| p.last_used_frame)
            .map(|(i, _)| i)
            .expect("at least one page exists")
    }

    /// Evict a page: reset its packer and remove all cache entries on it.
    fn evict_page(&mut self, page_idx: usize) {
        self.pages[page_idx].packer.reset();
        self.pages[page_idx].glyph_count = 0;
        self.pages[page_idx].last_used_frame = self.frame_counter;
        self.cache.retain(|_, e| e.page as usize != page_idx);
    }
}

#[cfg(test)]
mod tests;
